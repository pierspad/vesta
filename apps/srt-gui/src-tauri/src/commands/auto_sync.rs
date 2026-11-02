use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use srt_autosync::{AutoSyncConfig, AutoSyncProgress, ExistingAnchor, SubtitleLine};

use crate::state::{AppSyncState, AppTranscribeState, OperationGuard};

#[derive(Debug, Clone, Serialize)]
pub struct AutoSyncResult {
    pub success: bool,
    pub cancelled: bool,
    pub anchors_created: usize,
    pub segments_analyzed: usize,
    pub message: String,
}

#[tauri::command]
pub async fn sync_cancel_auto_sync(sync_state: State<'_, AppSyncState>) -> Result<(), String> {
    sync_state
        .lock()
        .map_err(|e| e.to_string())?
        .operation
        .cancel();
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
    cloud: Option<srt_transcribe::cloud::CloudConfig>,
) -> Result<AutoSyncResult, String> {
    // Reserve the shared transcription resource too; checking a flag alone
    // would allow a transcription to start immediately after the check.
    let transcribe_guard = OperationGuard::begin(
        &transcribe_state,
        "A transcription or model download is already in progress",
    )?;
    let guard = OperationGuard::with_token(
        &sync_state,
        "Auto-sync is already in progress",
        transcribe_guard.token(),
    )?;
    let token = guard.token();

    // Get media path, subtitle info, and existing ground-truth anchors from sync engine
    let (media_path, subtitles, existing_anchors) = {
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

        let anchors: Vec<ExistingAnchor> = engine
            .get_anchors()
            .iter()
            .filter(|anchor| anchor.is_manual)
            .map(|a| ExistingAnchor {
                original_time_ms: a.original_time_ms,
                target_time_ms: a.corrected_time_ms,
            })
            .collect();

        (media, subs, anchors)
    };

    if let Some(endpoint) = &cloud {
        if !srt_transcribe::pipeline::is_cloud_provider(Some(&endpoint.provider)) {
            return Err("Invalid external STT provider".into());
        }
        if endpoint.api_key.trim().is_empty()
            && !(["custom", "custom_whisper"].contains(&endpoint.provider.as_str())
                && endpoint
                    .api_url
                    .as_deref()
                    .is_some_and(|url| !url.trim().is_empty()))
        {
            return Err("Missing STT API credentials".into());
        }
    }
    let model_path = if cloud.is_some() {
        std::path::PathBuf::new()
    } else {
        srt_transcribe::model::model_file_path(&model_id).map_err(|e| e.to_string())?
    };

    let ffmpeg_cmd = crate::commands::media::resolve_ffmpeg_path(Some(&app)).await;
    let ffprobe_cmd = crate::commands::media::resolve_ffprobe_path(Some(&app)).await;

    let config = AutoSyncConfig {
        media_path,
        model_path,
        language,
        quick,
        ffmpeg_cmd,
        ffprobe_cmd,
        use_gpu: Some(true),
        max_offset_secs: Some(180),
        existing_anchors,
    };

    // Re-emit engine progress as Tauri events.
    let app_progress = app.clone();
    let on_progress: srt_autosync::ProgressCallback = Arc::new(move |update: AutoSyncProgress| {
        let _ = app_progress.emit("sync-auto-progress", update);
    });

    let outcome = srt_autosync::run_auto_sync_with_cloud(
        &config,
        subtitles,
        Some(on_progress),
        &token,
        cloud,
    )
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
        // The document cannot change while the guard is held. Check cancellation
        // under the same lock used by the cancel command before committing anchors.
        if token.is_cancelled() {
            return Ok(AutoSyncResult {
                success: false,
                cancelled: true,
                anchors_created: 0,
                segments_analyzed: outcome.segments_analyzed,
                message: "Auto-sync was cancelled.".into(),
            });
        }
        let engine = ss.engine.as_mut().ok_or("No SRT file loaded for sync")?;

        apply_suggestions(engine, &outcome.suggestions)
    };

    if anchors_created == 0 {
        let (message, key) = if !outcome.suggestions.is_empty() {
            (
                "Auto-sync completed. All suggested anchors are already aligned with existing anchors.".to_string(),
                "sync.autoSyncProgress.alreadySynced".to_string(),
            )
        } else {
            (
                "No confident matches found. Try with a larger STT model or check that the SRT matches the audio.".to_string(),
                "sync.autoSyncProgress.noMatches".to_string(),
            )
        };
        let _ = app.emit(
            "sync-auto-progress",
            AutoSyncProgress {
                stage: "done".to_string(),
                message: message.clone(),
                percentage: 100.0,
                message_key: Some(key),
                params: None,
            },
        );
        return Ok(AutoSyncResult {
            success: !outcome.suggestions.is_empty(),
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

/// Publish a replacement automatic result while retaining user-confirmed points.
fn apply_suggestions(
    engine: &mut srt_sync::SyncEngine,
    suggestions: &[srt_autosync::AnchorSuggestion],
) -> usize {
    // Keep the previous automatic result if no replacement was found.
    // Only user-confirmed anchors are fixed constraints for the next run.
    if !suggestions.is_empty() {
        let automatic_ids: Vec<_> = engine
            .get_anchors()
            .iter()
            .filter(|anchor| !anchor.is_manual)
            .map(|anchor| anchor.subtitle_index)
            .collect();
        for id in automatic_ids {
            engine.remove_anchor(id);
        }
    }
    let existing_anchors = engine.get_anchors().to_vec();
    let len_before = existing_anchors.len();

    for s in suggestions {
        let is_near_existing = existing_anchors.iter().any(|a| {
            (a.original_time_ms - s.original_start_ms).abs() < 15_000
                || (a.corrected_time_ms - s.corrected_time_ms).abs() < 15_000
        });

        if !is_near_existing {
            let _ = engine.add_anchor(s.subtitle_id, s.corrected_time_ms, false);
        }
    }

    engine.get_anchors().len().saturating_sub(len_before)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rerun_replaces_automatic_anchors_and_retains_manual_constraints() {
        let source = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(source.path(), "1\n00:01:00,000 --> 00:01:02,000\nFirst\n\n2\n00:03:00,000 --> 00:03:02,000\nMiddle\n\n3\n00:05:00,000 --> 00:05:02,000\nLast\n").unwrap();
        let mut engine = srt_sync::SyncEngine::new(source.path()).unwrap();
        engine.add_anchor(1, 65_000, true).unwrap();
        engine.add_anchor(3, 315_000, true).unwrap();
        engine.add_anchor(2, 190_000, false).unwrap();
        assert_eq!(apply_suggestions(&mut engine, &[]), 0);
        assert_eq!(engine.get_anchors().len(), 3);
        let suggestion = srt_autosync::AnchorSuggestion {
            subtitle_id: 2,
            original_start_ms: 180_000,
            corrected_time_ms: 192_000,
            similarity: 0.95,
            score: 0.95,
        };
        assert_eq!(apply_suggestions(&mut engine, &[suggestion]), 1);
        let anchors = engine.get_anchors();
        assert_eq!(anchors.len(), 3);
        assert!(anchors[0].is_manual);
        assert_eq!(anchors[0].corrected_time_ms, 65_000);
        assert_eq!(anchors[1].corrected_time_ms, 192_000);
        assert!(anchors[2].is_manual);
        assert_eq!(anchors[2].corrected_time_ms, 315_000);
    }
}
