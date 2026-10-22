use crate::paths::Paths;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;
use srt_flashcards::{FlashcardConfig, MediaTools};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, MutexGuard},
};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const HISTORY_LIMIT: usize = 64;

#[derive(Clone, Copy, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
}
impl Status {
    fn active(self) -> bool {
        matches!(self, Self::Running | Self::Cancelling)
    }
}

#[derive(Clone, Serialize, JsonSchema)]
pub struct JobSnapshot {
    pub job_id: String,
    pub status: Status,
    pub operation: String,
    pub output_dir: String,
    pub progress: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<String>,
}
struct Entry {
    snapshot: JobSnapshot,
    cancel: CancellationToken,
}
#[derive(Default)]
struct State {
    entries: BTreeMap<String, Entry>,
    handles: Vec<JoinHandle<()>>,
    shutting_down: bool,
}

/// One processing job at a time, with bounded in-memory history and cooperative shutdown.
#[derive(Clone, Default)]
pub struct Jobs {
    state: Arc<Mutex<State>>,
}
impl Jobs {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn start(&self, mut config: FlashcardConfig, paths: &Paths) -> Result<JobSnapshot, String> {
        self.start_work(
            paths,
            "flashcards",
            move |output, cancel, progress| async move {
                config.output_dir = output.to_string_lossy().into_owned();
                let result = srt_flashcards::generate(
                    config,
                    MediaTools::default(),
                    cancel,
                    &move |event| {
                        if let Ok(value) = serde_json::to_value(event) {
                            progress(value);
                        }
                    },
                )
                .await?;
                if !result.success {
                    return Err(result.message);
                }
                serde_json::to_value(result)
                    .map_err(|_| "Cannot serialize generation result".into())
            },
        )
    }

    pub fn start_translation(
        &self,
        input: String,
        language: String,
        profile: crate::provider_config::TranslationProfile,
        paths: &Paths,
    ) -> Result<JobSnapshot, String> {
        self.start_work(paths, "translation", move |output, cancel, progress| async move {
            let subtitles = tokio::task::spawn_blocking(move || srt_parser::SrtParser::parse_file(input))
                .await.map_err(|_| "Cannot read subtitle input".to_string())?
                .map_err(|e| format!("Cannot parse SRT input: {e}"))?;
            if cancel.is_cancelled() { return Err("Translation cancelled".into()); }
            if subtitles.is_empty() { return Err("Input contains no subtitles".into()); }
            let source = subtitles.clone();
            let pool = srt_translate::build_pool(&profile.tiers)?;
            let path = output.join("translated.srt");
            let translated = srt_translate::translate_subtitles_tiered_cancellable(
                pool, subtitles, &language, profile.batch_size, profile.resume_overlap,
                profile.title_context.as_deref(), &path,
                move |event| progress(serde_json::json!({
                    "stage": "translate", "current_batch": event.current_batch,
                    "total_batches": event.total_batches, "eta_seconds": event.eta_seconds,
                    "batch_start": event.batch_start, "batch_end": event.batch_end,
                })), cancel,
            ).await.map_err(|e| e.to_string())?;
            let missing = srt_translate::get_missing_or_incorrect_subtitle_ids(&source, &translated);
            if !missing.is_empty() {
                return Err(format!("Translation incomplete: {} subtitles remain; any completed checkpoints are retained in the job output directory", missing.len()));
            }
            Ok(serde_json::json!({"output_path": path, "subtitle_count": translated.len()}))
        })
    }

    pub fn start_transcription(
        &self,
        input: String,
        language: Option<String>,
        profile: crate::provider_config::TranscriptionProfile,
        paths: &Paths,
    ) -> Result<JobSnapshot, String> {
        self.start_work(
            paths,
            "transcription",
            move |output, cancel, progress| async move {
                let config = profile.config(
                    input,
                    output
                        .join("transcribed.srt")
                        .to_string_lossy()
                        .into_owned(),
                    language,
                );
                if cancel.is_cancelled() {
                    return Err("Transcription cancelled".into());
                }
                let callbacks = srt_transcribe::pipeline::PipelineCallbacks {
                    on_progress: Some(Arc::new(move |event| {
                        if let Ok(value) = serde_json::to_value(event) {
                            progress(value);
                        }
                    })),
                    on_segment: None,
                };
                let result = srt_transcribe::pipeline::transcribe_to_srt_with_download_policy(
                    &config,
                    "ffmpeg",
                    callbacks,
                    &cancel,
                    profile.allow_model_download,
                )
                .await
                .map_err(|e| e.to_string())?;
                serde_json::to_value(result)
                    .map_err(|_| "Cannot serialize transcription result".into())
            },
        )
    }

    fn start_work<F, Fut>(
        &self,
        paths: &Paths,
        operation: &str,
        work: F,
    ) -> Result<JobSnapshot, String>
    where
        F: FnOnce(std::path::PathBuf, CancellationToken, Arc<dyn Fn(Value) + Send + Sync>) -> Fut
            + Send
            + 'static,
        Fut: std::future::Future<Output = Result<Value, String>> + Send + 'static,
    {
        let mut state = self.lock();
        if state.shutting_down {
            return Err("Server is shutting down".into());
        }
        if state.entries.values().any(|e| e.snapshot.status.active()) {
            return Err("A job is already active; wait for it or cancel it first".into());
        }
        if state.entries.len() >= HISTORY_LIMIT
            && let Some(id) = state.entries.keys().next().cloned()
        {
            state.entries.remove(&id);
        }
        let output = paths.new_output()?;
        let id = output
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("Invalid output directory")?
            .to_owned();
        let snapshot = JobSnapshot {
            job_id: id.clone(),
            operation: operation.to_owned(),
            status: Status::Running,
            output_dir: output.to_string_lossy().into_owned(),
            progress: None,
            result: None,
            error: None,
        };
        let cancel = CancellationToken::new();
        state.entries.insert(
            id.clone(),
            Entry {
                snapshot: snapshot.clone(),
                cancel: cancel.clone(),
            },
        );
        state.handles.retain(|handle| !handle.is_finished());
        let jobs = self.clone();
        state.handles.push(tokio::spawn(async move {
            let progress_jobs = jobs.clone();
            let progress_id = id.clone();
            let progress = Arc::new(move |value| {
                if let Some(entry) = progress_jobs.lock().entries.get_mut(&progress_id) {
                    entry.snapshot.progress = Some(value);
                }
            });
            let worker_cancel = cancel.clone();
            // A panicking engine must still leave a terminal, observable job state.
            let outcome =
                tokio::spawn(async move { work(output, worker_cancel, progress).await }).await;
            if let Some(entry) = jobs.lock().entries.get_mut(&id) {
                match outcome {
                    Ok(Ok(result)) => {
                        entry.snapshot.status = Status::Succeeded;
                        entry.snapshot.result = Some(result);
                    }
                    other => {
                        entry.snapshot.status = if cancel.is_cancelled() {
                            Status::Cancelled
                        } else {
                            Status::Failed
                        };
                        entry.snapshot.error = Some(match other {
                            Ok(Err(error)) => error,
                            Err(_) => "Processing task failed".into(),
                            Ok(Ok(_)) => unreachable!(),
                        });
                    }
                }
            }
        }));
        Ok(snapshot)
    }

    pub fn get(&self, id: &str) -> Result<JobSnapshot, String> {
        self.lock()
            .entries
            .get(id)
            .map(|entry| entry.snapshot.clone())
            .ok_or_else(|| "Unknown job ID (history lasts only for this server session)".into())
    }

    pub fn cancel(&self, id: &str) -> Result<JobSnapshot, String> {
        let mut state = self.lock();
        let entry = state.entries.get_mut(id).ok_or("Unknown job ID")?;
        if entry.snapshot.status.active() {
            entry.cancel.cancel();
            entry.snapshot.status = Status::Cancelling;
        }
        Ok(entry.snapshot.clone())
    }

    pub async fn shutdown(&self) {
        let handles = {
            let mut state = self.lock();
            state.shutting_down = true;
            for entry in state.entries.values() {
                entry.cancel.cancel();
            }
            std::mem::take(&mut state.handles)
        };
        for handle in handles {
            let _ = handle.await;
        }
    }
}
