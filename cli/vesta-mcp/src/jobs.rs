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

#[derive(Clone, Copy, Debug, Serialize, JsonSchema, PartialEq, Eq)]
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
    seq: u64,
    snapshot: JobSnapshot,
    cancel: CancellationToken,
}
#[derive(Default)]
struct State {
    next_seq: u64,
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
        if state.entries.len() >= HISTORY_LIMIT {
            let candidate = state
                .entries
                .iter()
                .filter(|(_, e)| !e.snapshot.status.active())
                .min_by_key(|(_, e)| e.seq)
                .or_else(|| state.entries.iter().min_by_key(|(_, e)| e.seq))
                .map(|(k, _)| k.clone());
            if let Some(evict_id) = candidate {
                state.entries.remove(&evict_id);
            }
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
        let seq = state.next_seq;
        state.next_seq += 1;
        state.entries.insert(
            id.clone(),
            Entry {
                seq,
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
                        clean_empty_output_dir(&entry.snapshot.output_dir);
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

fn clean_empty_output_dir(dir: &str) {
    let path = std::path::Path::new(dir);
    if let Ok(mut entries) = std::fs::read_dir(path)
        && entries.next().is_none()
    {
        let _ = std::fs::remove_dir(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifo_eviction_orders_by_sequence_not_alphabetical_id() {
        let jobs = Jobs::default();
        let mut state = jobs.lock();

        // Populate entries such that the first inserted entry ("vesta-zzz")
        // is alphabetically AFTER later entries ("vesta-001", etc.)
        for i in 0..HISTORY_LIMIT {
            let id = if i == 0 {
                "vesta-zzz".to_string()
            } else {
                format!("vesta-{:03}", i)
            };
            let seq = state.next_seq;
            state.next_seq += 1;
            state.entries.insert(
                id.clone(),
                Entry {
                    seq,
                    snapshot: JobSnapshot {
                        job_id: id,
                        operation: "test".into(),
                        status: Status::Succeeded,
                        output_dir: "/tmp".into(),
                        progress: None,
                        result: None,
                        error: None,
                    },
                    cancel: CancellationToken::new(),
                },
            );
        }

        assert_eq!(state.entries.len(), HISTORY_LIMIT);
        assert!(state.entries.contains_key("vesta-zzz"));
        assert!(state.entries.contains_key("vesta-001"));

        // Evict the entry with the smallest sequence number.
        let candidate = state
            .entries
            .iter()
            .filter(|(_, e)| !e.snapshot.status.active())
            .min_by_key(|(_, e)| e.seq)
            .or_else(|| state.entries.iter().min_by_key(|(_, e)| e.seq))
            .map(|(k, _)| k.clone());
        if let Some(evict_id) = candidate {
            state.entries.remove(&evict_id);
        }

        // Under alphabetical order, "vesta-001" would have been evicted.
        // Under FIFO order, "vesta-zzz" (seq 0) MUST be evicted!
        assert!(
            !state.entries.contains_key("vesta-zzz"),
            "oldest entry vesta-zzz must be evicted first"
        );
        assert!(
            state.entries.contains_key("vesta-001"),
            "vesta-001 must still exist"
        );
    }

    #[tokio::test]
    async fn start_work_evicts_in_fifo_order_when_limit_reached() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(vec![dir.path().to_path_buf()], dir.path().to_path_buf()).unwrap();
        let jobs = Jobs::default();

        // Pre-populate with HISTORY_LIMIT completed jobs
        {
            let mut state = jobs.lock();
            for i in 0..HISTORY_LIMIT {
                let id = if i == 0 {
                    "vesta-zzz-oldest".to_string()
                } else {
                    format!("vesta-aaa-{:03}", i)
                };
                let seq = state.next_seq;
                state.next_seq += 1;
                state.entries.insert(
                    id.clone(),
                    Entry {
                        seq,
                        snapshot: JobSnapshot {
                            job_id: id,
                            operation: "test".into(),
                            status: Status::Succeeded,
                            output_dir: dir.path().to_string_lossy().into_owned(),
                            progress: None,
                            result: None,
                            error: None,
                        },
                        cancel: CancellationToken::new(),
                    },
                );
            }
        }

        assert!(jobs.get("vesta-zzz-oldest").is_ok());
        assert!(jobs.get("vesta-aaa-001").is_ok());

        // Now start one more job via start_work
        let snap = jobs
            .start_work(&paths, "test", |_out, _cancel, _progress| async move {
                Ok(serde_json::json!({"done": true}))
            })
            .unwrap();

        // The oldest entry (vesta-zzz-oldest) should have been evicted, not vesta-aaa-001
        assert!(
            jobs.get("vesta-zzz-oldest").is_err(),
            "FIFO eviction must remove oldest entry vesta-zzz-oldest"
        );
        assert!(
            jobs.get("vesta-aaa-001").is_ok(),
            "newer entry vesta-aaa-001 must be retained"
        );
        assert!(jobs.get(&snap.job_id).is_ok());
    }

    #[tokio::test]
    async fn failed_job_cleans_empty_dir_but_retains_partial_checkpoints() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(vec![dir.path().to_path_buf()], dir.path().to_path_buf()).unwrap();
        let jobs = Jobs::default();

        // 1. Job fails without writing files -> directory removed
        let snap1 = jobs
            .start_work(
                &paths,
                "fail_empty",
                |_output, _cancel, _progress| async move { Err("immediate failure".to_string()) },
            )
            .unwrap();

        let out_dir1 = std::path::PathBuf::from(&snap1.output_dir);
        assert!(out_dir1.exists(), "directory initially created");

        // Wait for job to finish
        for _ in 0..100 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            if !jobs.get(&snap1.job_id).unwrap().status.active() {
                break;
            }
        }
        assert_eq!(jobs.get(&snap1.job_id).unwrap().status, Status::Failed);
        assert!(
            !out_dir1.exists(),
            "empty directory must be deleted on failure"
        );

        // 2. Job fails after writing a partial checkpoint -> directory retained
        let snap2 = jobs
            .start_work(
                &paths,
                "fail_partial",
                |output, _cancel, _progress| async move {
                    std::fs::write(output.join("checkpoint.json"), b"partial checkpoint").unwrap();
                    Err("failure after checkpoint".to_string())
                },
            )
            .unwrap();

        let out_dir2 = std::path::PathBuf::from(&snap2.output_dir);
        assert!(out_dir2.exists());

        // Wait for job to finish
        for _ in 0..100 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            if !jobs.get(&snap2.job_id).unwrap().status.active() {
                break;
            }
        }
        assert_eq!(jobs.get(&snap2.job_id).unwrap().status, Status::Failed);
        assert!(
            out_dir2.exists(),
            "non-empty directory must be retained on failure"
        );
        assert!(out_dir2.join("checkpoint.json").exists());
    }

    #[tokio::test]
    async fn cancelled_job_cleans_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(vec![dir.path().to_path_buf()], dir.path().to_path_buf()).unwrap();
        let jobs = Jobs::default();

        let snap = jobs
            .start_work(
                &paths,
                "cancel_empty",
                |_output, cancel, _progress| async move {
                    while !cancel.is_cancelled() {
                        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                    }
                    Err("cancelled".to_string())
                },
            )
            .unwrap();

        let out_dir = std::path::PathBuf::from(&snap.output_dir);
        assert!(out_dir.exists());

        jobs.cancel(&snap.job_id).unwrap();

        for _ in 0..100 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            if !jobs.get(&snap.job_id).unwrap().status.active() {
                break;
            }
        }
        assert_eq!(jobs.get(&snap.job_id).unwrap().status, Status::Cancelled);
        assert!(
            !out_dir.exists(),
            "empty directory must be deleted on cancel"
        );
    }
}
