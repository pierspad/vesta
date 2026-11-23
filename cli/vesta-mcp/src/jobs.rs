use crate::paths::Paths;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, MutexGuard},
};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use vesta_flashcards::{FlashcardConfig, MediaTools};

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

/// Target source for an OCR job: either an embedded bitmap track or a burned-in video ROI.
pub enum OcrTarget {
    BitmapTrack(u32),
    HardcodedRoi {
        roi: vesta_ocr::VideoRoi,
        sample_fps: f64,
    },
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
                let result = vesta_flashcards::generate(
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

    pub fn start_batch(
        &self,
        configs: Vec<FlashcardConfig>,
        merge_apkg: bool,
        paths: &Paths,
    ) -> Result<JobSnapshot, String> {
        self.start_work(
            paths,
            "batch_flashcards",
            move |output, cancel, progress| async move {
                let total_episodes = configs.len();
                let mut episode_results = Vec::with_capacity(total_episodes);
                let mut apkg_paths = Vec::new();
                let mut total_cards = 0usize;

                for (idx, mut config) in configs.into_iter().enumerate() {
                    if cancel.is_cancelled() {
                        return Err("Batch flashcard generation cancelled".into());
                    }
                    let ep_dir = output.join(format!("ep-{:03}", idx + 1));
                    let ep_dir_str = ep_dir.to_string_lossy().into_owned();
                    std::fs::create_dir_all(&ep_dir).map_err(|e| {
                        format!(
                            "Cannot create episode output directory '{}': {e}",
                            ep_dir.display()
                        )
                    })?;
                    config.output_dir = ep_dir_str.clone();
                    let deck_name = config.deck_name.clone();
                    let deck_name_progress = deck_name.clone();
                    let episode_number = config.episode_number;
                    let progress_ep = progress.clone();
                    let result = vesta_flashcards::generate(
                        config,
                        MediaTools::default(),
                        cancel.clone(),
                        &move |event| {
                            if let Ok(event_value) = serde_json::to_value(event) {
                                progress_ep(serde_json::json!({
                                    "stage": "batch",
                                    "episode_index": idx,
                                    "episode_number": episode_number,
                                    "total_episodes": total_episodes,
                                    "deck": &deck_name_progress,
                                    "episode_progress": event_value,
                                }));
                            }
                        },
                    )
                    .await;
                    let result = match result {
                        Ok(res) if res.success => res,
                        Ok(res) => {
                            clean_empty_output_dir(&ep_dir_str);
                            return Err(format!(
                                "Episode {} ({}) failed: {}",
                                idx + 1,
                                deck_name,
                                res.message
                            ));
                        }
                        Err(err) => {
                            clean_empty_output_dir(&ep_dir_str);
                            return Err(format!(
                                "Episode {} ({}) failed: {err}",
                                idx + 1,
                                deck_name
                            ));
                        }
                    };
                    total_cards = total_cards.saturating_add(result.cards_generated);
                    if let Some(ref apkg) = result.apkg_path {
                        apkg_paths.push(std::path::PathBuf::from(apkg));
                    }
                    episode_results.push(serde_json::json!({
                        "index": idx,
                        "episode_number": episode_number,
                        "deck": deck_name,
                        "cards_generated": result.cards_generated,
                        "audio_clips": result.audio_clips,
                        "snapshots": result.snapshots,
                        "video_clips": result.video_clips,
                        "tsv_path": result.tsv_path,
                        "apkg_path": result.apkg_path,
                        "output_size_bytes": result.output_size_bytes,
                    }));
                }

                let merged_apkg_path = if merge_apkg && apkg_paths.len() >= 2 {
                    if cancel.is_cancelled() {
                        return Err("Batch flashcard generation cancelled".into());
                    }
                    progress(serde_json::json!({
                        "stage": "merge",
                        "total_episodes": total_episodes,
                    }));
                    let merged_path = output.join("merged.apkg");
                    let merged_clone = merged_path.clone();
                    tokio::task::spawn_blocking(move || {
                        vesta_flashcards::merge_apkg(&apkg_paths, &merged_clone)
                    })
                    .await
                    .map_err(|e| format!("Batch merge task failed: {e}"))?
                    .map_err(|e| format!("Failed to merge batch APKG outputs: {e}"))?;
                    Some(merged_path)
                } else {
                    None
                };

                Ok(serde_json::json!({
                    "total_episodes": total_episodes,
                    "succeeded_episodes": episode_results.len(),
                    "cards_generated": total_cards,
                    "episodes": episode_results,
                    "merged_apkg_path": merged_apkg_path,
                }))
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
            let subtitles = tokio::task::spawn_blocking(move || vesta_parser::SrtParser::parse_file(input))
                .await.map_err(|_| "Cannot read subtitle input".to_string())?
                .map_err(|e| format!("Cannot parse SRT input: {e}"))?;
            if cancel.is_cancelled() { return Err("Translation cancelled".into()); }
            if subtitles.is_empty() { return Err("Input contains no subtitles".into()); }
            let source = subtitles.clone();
            let pool = vesta_translate::build_pool(&profile.tiers)?;
            let path = output.join("translated.srt");
            let translated = vesta_translate::translate_subtitles_tiered_cancellable(
                pool, subtitles, &language, profile.batch_size, profile.resume_overlap,
                profile.title_context.as_deref(), &path,
                move |event| progress(serde_json::json!({
                    "stage": "translate", "current_batch": event.current_batch,
                    "total_batches": event.total_batches, "eta_seconds": event.eta_seconds,
                    "batch_start": event.batch_start, "batch_end": event.batch_end,
                })), cancel,
            ).await.map_err(|e| e.to_string())?;
            let missing = vesta_translate::get_missing_or_incorrect_subtitle_ids(&source, &translated);
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
                let callbacks = vesta_transcribe::pipeline::PipelineCallbacks {
                    on_progress: Some(Arc::new(move |event| {
                        if let Ok(value) = serde_json::to_value(event) {
                            progress(value);
                        }
                    })),
                    on_segment: None,
                };
                let result = vesta_transcribe::pipeline::transcribe_to_srt_with_download_policy(
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

    pub fn start_ocr(
        &self,
        input: String,
        target: OcrTarget,
        language: Option<String>,
        profile: crate::provider_config::TranslationProfile,
        paths: &Paths,
    ) -> Result<JobSnapshot, String> {
        self.start_work(paths, "ocr", move |output, cancel, progress| async move {
            if cancel.is_cancelled() {
                return Err("OCR cancelled".into());
            }
            let input_path = std::path::PathBuf::from(input);
            let output_path = output.join("ocr.srt");
            let callbacks = vesta_ocr::OcrCallbacks {
                on_progress: Some(Arc::new(move |event| {
                    if let Ok(value) = serde_json::to_value(event) {
                        progress(value);
                    }
                })),
                on_segment: None,
            };
            let outcome = match target {
                OcrTarget::BitmapTrack(track_index) => {
                    let config = vesta_ocr::OcrConfig {
                        input_path,
                        output_path,
                        track_index,
                        language,
                        tiers: profile.tiers,
                        preview_limit: None,
                        ffmpeg_path: "ffmpeg".into(),
                        ffprobe_path: "ffprobe".into(),
                    };
                    vesta_ocr::ocr_bitmap_track_to_srt(&config, callbacks, &cancel).await
                }
                OcrTarget::HardcodedRoi { roi, sample_fps } => {
                    let config = vesta_ocr::HardcodedOcrConfig {
                        input_path,
                        output_path,
                        roi,
                        sample_fps,
                        language,
                        tiers: profile.tiers,
                        preview_limit: None,
                        ffmpeg_path: "ffmpeg".into(),
                        ffprobe_path: "ffprobe".into(),
                    };
                    vesta_ocr::ocr_hardcoded_to_srt(&config, callbacks, &cancel).await
                }
            }
            .map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "output_path": outcome.output_path,
                "subtitle_count": outcome.subtitle_count,
            }))
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn start_autosync(
        &self,
        input: String,
        media: String,
        language: Option<String>,
        quick: bool,
        max_offset_secs: u32,
        profile: crate::provider_config::TranscriptionProfile,
        paths: &Paths,
    ) -> Result<JobSnapshot, String> {
        self.start_work(
            paths,
            "autosync",
            move |output, cancel, progress| async move {
                let input_clone = input.clone();
                let media_clone = media.clone();
                let (mut engine, subtitles) = tokio::task::spawn_blocking(move || {
                    let mut engine = vesta_sync::SyncEngine::new(&input_clone)
                        .map_err(|e| format!("Cannot parse SRT input: {e}"))?;
                    engine.set_video_path(&media_clone);
                    let subtitles: Vec<vesta_autosync::SubtitleLine> = engine
                        .get_all_subtitles()
                        .iter()
                        .map(|sub| vesta_autosync::SubtitleLine {
                            id: sub.id,
                            start_ms: sub.start.milliseconds as i64,
                            text: sub.text.clone(),
                        })
                        .collect();
                    Ok::<_, String>((engine, subtitles))
                })
                .await
                .map_err(|_| "Cannot load subtitle input".to_string())??;

                if cancel.is_cancelled() {
                    return Err("Auto-sync cancelled".into());
                }
                if subtitles.is_empty() {
                    return Err("Input contains no subtitles".into());
                }

                let is_local =
                    ["local", "whisper", "local_whisper"].contains(&profile.provider.as_str());
                let (model_path, cloud) = if is_local {
                    let resolved = vesta_transcribe::model::model_file_path(&profile.model)
                        .map_err(|e| e.to_string())?;
                    let path = if resolved.is_file() {
                        resolved
                    } else if !profile.allow_model_download {
                        return Err("Local Whisper model is not installed; install it with vesta-cli transcribe download or explicitly enable allow_model_download in the server profile".into());
                    } else {
                        let progress_dl = progress.clone();
                        let model_id = profile.model.clone();
                        vesta_transcribe::model::download_model(
                            &profile.model,
                            move |pct| {
                                progress_dl(serde_json::json!({
                                    "stage": "download",
                                    "message": format!("Downloading model {model_id}"),
                                    "percentage": pct,
                                }));
                            },
                            Some(&cancel),
                        )
                        .await
                        .map_err(|e| e.to_string())?
                    };
                    (path, None)
                } else {
                    (
                        std::path::PathBuf::new(),
                        Some(vesta_transcribe::cloud::CloudConfig {
                            provider: profile.provider.clone(),
                            api_key: profile.api_key.unwrap_or_default(),
                            api_url: profile.api_url.clone(),
                            model: profile.model.clone(),
                            language: language.clone(),
                            translate_to_english: false,
                        }),
                    )
                };

                if cancel.is_cancelled() {
                    return Err("Auto-sync cancelled".into());
                }

                let mut config = vesta_autosync::AutoSyncConfig::new(media, model_path);
                config.language = language;
                config.quick = quick;
                config.use_gpu = Some(profile.use_gpu);
                config.max_offset_secs = Some(max_offset_secs);

                let progress_cb = progress.clone();
                let on_progress: vesta_autosync::ProgressCallback =
                    Arc::new(move |update: vesta_autosync::AutoSyncProgress| {
                        if let Ok(value) = serde_json::to_value(update) {
                            progress_cb(value);
                        }
                    });

                let outcome = vesta_autosync::run_auto_sync_with_cloud(
                    &config,
                    subtitles,
                    Some(on_progress),
                    &cancel,
                    cloud,
                )
                .await
                .map_err(|e| e.to_string())?;

                if outcome.cancelled || cancel.is_cancelled() {
                    return Err("Auto-sync cancelled".into());
                }
                if outcome.suggestions.is_empty() {
                    return Err("No confident subtitle matches found for auto-sync".into());
                }

                let output_path = output.join("synced.srt");
                let out_clone = output_path.clone();
                let suggestions = outcome.suggestions;
                let (anchors_created, average_offset_ms) = tokio::task::spawn_blocking(move || {
                    let created = engine.group_anchor_edits(|eng| -> Result<usize, String> {
                        let mut count = 0;
                        for suggestion in &suggestions {
                            eng.add_anchor(
                                suggestion.subtitle_id,
                                suggestion.corrected_time_ms,
                                false,
                            )
                            .map_err(|e| e.to_string())?;
                            count += 1;
                        }
                        Ok(count)
                    })?;
                    engine.save_synced_file(&out_clone).map_err(|e| e.to_string())?;
                    Ok::<_, String>((created, engine.get_average_offset().round() as i64))
                })
                .await
                .map_err(|_| "Failed to save synced subtitles".to_string())??;

                Ok(serde_json::json!({
                    "output_path": output_path,
                    "anchors_created": anchors_created,
                    "segments_analyzed": outcome.segments_analyzed,
                    "average_offset_ms": average_offset_ms,
                }))
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn start_refine(
        &self,
        input: String,
        output_ext: &'static str,
        prompt: String,
        batch_mode: bool,
        batch_size: usize,
        allow_partial: bool,
        profile: crate::provider_config::TranslationProfile,
        paths: &Paths,
    ) -> Result<JobSnapshot, String> {
        self.start_work(paths, "refine", move |output, cancel, progress| async move {
            let input_for_load = input.clone();
            let cards =
                tokio::task::spawn_blocking(move || vesta_refine::load_cards(&input_for_load))
                    .await
                    .map_err(|_| "Cannot read deck input".to_string())??;
            if cancel.is_cancelled() {
                return Err("Refinement cancelled".into());
            }
            if cards.is_empty() {
                return Err("Input contains no annotatable cards".into());
            }
            let pool = vesta_translate::build_pool(&profile.tiers)?;
            let total = cards.len();
            let updates = Arc::new(Mutex::new(Vec::<vesta_refine::RefineUpdate>::new()));
            let updates_cb = updates.clone();
            let done_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let failed_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let done_cb = done_count.clone();
            let failed_cb = failed_count.clone();

            let summary = vesta_refine::refine_cards_tiered(
                cards,
                vesta_refine::RefineRunConfig {
                    prompt,
                    batch_mode,
                    batch_size,
                },
                pool,
                move |event| match event {
                    vesta_refine::RefineEvent::CardDone {
                        id,
                        notes,
                        done,
                        total,
                    } => {
                        updates_cb
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .push(vesta_refine::RefineUpdate { id, notes });
                        done_cb.store(done, std::sync::atomic::Ordering::Relaxed);
                        let failed = failed_cb.load(std::sync::atomic::Ordering::Relaxed);
                        progress(serde_json::json!({
                            "stage": "refine",
                            "done": done,
                            "failed": failed,
                            "total": total,
                        }));
                    }
                    vesta_refine::RefineEvent::CardFailed { id, error } => {
                        let failed =
                            failed_cb.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                        let done = done_cb.load(std::sync::atomic::Ordering::Relaxed);
                        progress(serde_json::json!({
                            "stage": "refine",
                            "done": done,
                            "failed": failed,
                            "total": total,
                            "last_failed_id": id,
                            "last_error": error,
                        }));
                    }
                    vesta_refine::RefineEvent::Info { message } => {
                        progress(serde_json::json!({
                            "stage": "refine",
                            "message": message,
                            "total": total,
                        }));
                    }
                },
                cancel.clone(),
            )
            .await?;

            let complete = summary.failed == 0 && summary.remaining == 0 && !summary.pool_exhausted;
            let cancelled = summary.cancelled || cancel.is_cancelled();
            if cancelled {
                return Err("Refinement cancelled".into());
            }
            if summary.done == 0 || (!complete && !allow_partial) {
                return Err(format!(
                    "Refinement incomplete: {} done, {} failed, {} remaining (pool_exhausted: {})",
                    summary.done, summary.failed, summary.remaining, summary.pool_exhausted
                ));
            }
            let output_path = output.join(format!("refined.{output_ext}"));
            let output_str = output_path.to_string_lossy().into_owned();
            let updates_vec = std::mem::take(
                &mut *updates
                    .lock()
                    .map_err(|_| "Annotation update lock poisoned".to_string())?,
            );
            tokio::task::spawn_blocking(move || {
                vesta_refine::save_cards(&input, &output_str, updates_vec)
            })
            .await
            .map_err(|_| "Failed to save refined deck".to_string())??;
            Ok(serde_json::json!({
                "output_path": output_path,
                "done": summary.done,
                "failed": summary.failed,
                "remaining": summary.remaining,
                "pool_exhausted": summary.pool_exhausted,
                "cancelled": false,
            }))
        })
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

pub(crate) fn clean_empty_output_dir(dir: &str) {
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
