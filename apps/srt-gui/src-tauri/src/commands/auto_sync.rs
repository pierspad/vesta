use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use srt_autosync::{AutoSyncConfig, AutoSyncProgress, SubtitleLine};

use crate::state::{AppSyncState, AppTranscribeState};

#[derive(Debug, Clone, Serialize)]
pub struct AutoSyncResult {
    pub success: bool,
    pub cancelled: bool,
    pub anchors_created: usize,
    pub segments_analyzed: usize,
    pub message: String,
}

struct AutoSyncGuard<'a>(&'a AppSyncState);

impl<'a> Drop for AutoSyncGuard<'a> {
    fn drop(&mut self) {
        if let Ok(mut ss) = self.0.lock() {
            ss.is_auto_syncing = false;
            ss.auto_sync_cancellation_token = None;
        }
    }
}

#[tauri::command]
pub async fn sync_cancel_auto_sync(sync_state: State<'_, AppSyncState>) -> Result<(), String> {
    let ss = sync_state.lock().map_err(|e| e.to_string())?;
    if let Some(token) = &ss.auto_sync_cancellation_token {
        token.cancel();
    }
    Ok(())
}

/// Auto-sync command: transcribes strategic audio segments and matches
/// them against loaded SRT subtitles to create anchor points automatically.
#[tauri::command]
pub async fn sync_auto_sync(
    app: AppHandle,
    sync_state: State<'_, AppSyncState>,
    transcribe_state: State<'_, AppTranscribeState>,
    model_id: String,
    language: Option<String>,
    quick: bool,
) -> Result<AutoSyncResult, String> {
    let token = {
        let mut ss = sync_state.lock().map_err(|e| e.to_string())?;
        if ss.is_auto_syncing {
            return Err("Auto-sync is already in progress".to_string());
        }

        let ts = transcribe_state.lock().map_err(|e| e.to_string())?;
        if ts.is_transcribing {
            return Err("A transcription is already in progress".to_string());
        }

        let token = CancellationToken::new();
        ss.is_auto_syncing = true;
        ss.auto_sync_cancellation_token = Some(token.clone());
        token
    };

    // Ensure state cleanup when function returns
    let _guard = AutoSyncGuard(&sync_state);

    // Get media path and subtitle info from sync engine
    let (media_path, subtitles) = {
        let ss = sync_state.lock().map_err(|e| e.to_string())?;
        let engine = ss.engine.as_ref().ok_or("No SRT file loaded for sync")?;

        let media = engine
            .get_video_path()
            .ok_or("No media file loaded. Load audio/video first.")?
            .to_string();

        let subs: Vec<SubtitleLine> = engine
            .get_all_subtitles()
            .iter()
            .map(|sub| SubtitleLine {
                id: sub.id,
                start_ms: sub.start.milliseconds as i64,
                text: sub.text.clone(),
            })
            .collect();

        (media, subs)
    };

    let model_path =
        srt_transcribe::model::model_file_path(&model_id).map_err(|e| e.to_string())?;

    let ffmpeg_cmd = crate::commands::flashcards::media::resolve_ffmpeg_path(Some(&app)).await;
    let ffprobe_cmd = crate::commands::flashcards::media::resolve_ffprobe_path(Some(&app)).await;

    let config = AutoSyncConfig {
        media_path,
        model_path,
        language,
        quick,
        ffmpeg_cmd,
        ffprobe_cmd,
    };

    // Re-emit engine progress as Tauri events.
    let app_progress = app.clone();
    let on_progress: srt_autosync::ProgressCallback = Arc::new(move |update: AutoSyncProgress| {
        let _ = app_progress.emit("sync-auto-progress", update);
    });

    let outcome = srt_autosync::run_auto_sync(&config, subtitles, Some(on_progress), &token)
        .await
        .map_err(|e| e.to_string())?;

    if outcome.cancelled {
        return Ok(AutoSyncResult {
            success: false,
            cancelled: true,
            anchors_created: 0,
            segments_analyzed: outcome.segments_analyzed,
            message: "Auto-sync was cancelled.".to_string(),
        });
    }

    // Apply suggestions to the engine, skipping those near existing anchors.
    let anchors_created = {
        let mut ss = sync_state.lock().map_err(|e| e.to_string())?;
        let engine = ss.engine.as_mut().ok_or("No SRT file loaded for sync")?;

        let existing_anchors = engine.get_anchors().to_vec();

        let mut count = 0;
        for s in &outcome.suggestions {
            let is_near_existing = existing_anchors.iter().any(|a| {
                (a.original_time_ms - s.original_start_ms).abs() < 10_000
                    || (a.corrected_time_ms - s.corrected_time_ms).abs() < 10_000
            });

            if !is_near_existing
                && engine
                    .add_anchor(s.subtitle_id, s.corrected_time_ms, false)
                    .is_ok()
            {
                count += 1;
            }
        }

        count
    };

    if anchors_created == 0 {
        let message = "No confident matches found. Try with a larger Whisper model or check that the SRT matches the audio.".to_string();
        let _ = app.emit(
            "sync-auto-progress",
            AutoSyncProgress {
                stage: "done".to_string(),
                message: message.clone(),
                percentage: 100.0,
                message_key: Some("sync.autoSyncProgress.noMatches".to_string()),
                params: None,
            },
        );
        return Ok(AutoSyncResult {
            success: false,
            cancelled: false,
            anchors_created: 0,
            segments_analyzed: outcome.segments_analyzed,
            message,
        });
    }

    let _ = app.emit(
        "sync-auto-progress",
        AutoSyncProgress {
            stage: "done".to_string(),
            message: format!(
                "Auto-sync complete! Created {anchors_created} new visual anchor points."
            ),
            percentage: 100.0,
            message_key: Some("sync.autoSyncProgress.done".to_string()),
            params: Some(std::collections::HashMap::from([(
                "count".to_string(),
                anchors_created.to_string(),
            )])),
        },
    );

    Ok(AutoSyncResult {
        success: true,
        cancelled: false,
        anchors_created,
        segments_analyzed: outcome.segments_analyzed,
        message: format!("Successfully created {anchors_created} new visual anchor points."),
    })
}
