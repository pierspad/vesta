//! Comandi Tauri per la trascrizione audio/video.
//!
//! Adapter sottile sopra [`whisper_common::pipeline`]: traduce fra il mondo
//! Tauri (AppHandle, stato gestito, eventi `transcribe-progress` /
//! `transcribe-segment`) e la pipeline headless media → SRT. Tutta la logica
//! (download modelli, conversione audio, backend locale/cloud, post-processing
//! e scrittura SRT) vive nella libreria, condivisa con la CLI `srt-transcribe`.

use std::path::Path;
use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use whisper_common::model::{list_models, uninstall_model, WhisperModelInfo};
use whisper_common::pipeline::{self, PipelineCallbacks, TranscriptionConfig};

use crate::state::AppTranscribeState;

/// Config della trascrizione: è direttamente quella della pipeline headless
/// (i nomi dei campi sono il contratto serde con il frontend).
pub type TranscribeConfig = TranscriptionConfig;

#[derive(Debug, Clone, Serialize)]
pub struct TranscribeResult {
    pub success: bool,
    pub message: String,
    pub output_path: Option<String>,
    pub subtitle_count: usize,
    pub detected_language: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscribeProgressEvent {
    pub stage: String,
    pub message: String,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscribeSegmentEvent {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

/// Costruisce le callback della pipeline che rilanciano gli eventi Tauri.
fn tauri_callbacks(app: &AppHandle) -> PipelineCallbacks {
    let app_progress = app.clone();
    let app_segment = app.clone();

    PipelineCallbacks {
        on_progress: Some(Arc::new(move |update: pipeline::ProgressUpdate| {
            let _ = app_progress.emit(
                "transcribe-progress",
                TranscribeProgressEvent {
                    stage: update.stage,
                    message: update.message,
                    percentage: update.percentage,
                },
            );
        })),
        on_segment: Some(Arc::new(move |start_ms, end_ms, text: &str| {
            let _ = app_segment.emit(
                "transcribe-segment",
                TranscribeSegmentEvent { start_ms, end_ms, text: text.to_string() },
            );
        })),
    }
}

// ─── Tauri Commands ──────────────────────────────────────────────────────────

/// Check what Whisper backends are available
#[tauri::command]
pub async fn transcribe_check_backends(app: AppHandle) -> Result<serde_json::Value, String> {
    let ffmpeg_cmd = crate::commands::flashcards::media::resolve_ffmpeg_path(Some(&app)).await;
    let ffmpeg_available = tokio::process::Command::new(&ffmpeg_cmd)
        .arg("-version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    // whisper-rs is always available (compiled natively)
    Ok(serde_json::json!({
        "ffmpeg": ffmpeg_available,
        "whisper_cpp": true,
        "python_whisper": false,
        "whisper_binary": "whisper-rs (native)",
        "any_whisper": true,
    }))
}

/// Get list of models with their download status
#[tauri::command]
pub async fn transcribe_list_models() -> Result<Vec<WhisperModelInfo>, String> {
    list_models().map_err(|e| e.to_string())
}

/// Download a specific Whisper model
#[tauri::command]
pub async fn transcribe_download_model(
    app: AppHandle,
    state: State<'_, AppTranscribeState>,
    model_id: String,
) -> Result<bool, String> {
    let cancel_token = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        let token = CancellationToken::new();
        s.cancellation_token = Some(token.clone());
        token
    };

    let app_progress = app.clone();
    let model_id_progress = model_id.clone();

    whisper_common::model::download_model(
        &model_id,
        move |percentage| {
            let _ = app_progress.emit(
                "transcribe-progress",
                TranscribeProgressEvent {
                    stage: "download".to_string(),
                    message: format!(
                        "Downloading model {model_id_progress} ({percentage}%)..."
                    ),
                    percentage: percentage as f64,
                },
            );
        },
        Some(&cancel_token),
    )
    .await
    .map(|_| true)
    .map_err(|e| e.to_string())
}

/// Uninstall (delete) a specific Whisper model
#[tauri::command]
pub async fn transcribe_uninstall_model(model_id: String) -> Result<bool, String> {
    uninstall_model(&model_id).map_err(|e| e.to_string())
}

/// Start transcription
#[tauri::command]
pub async fn transcribe_start(
    app: AppHandle,
    state: State<'_, AppTranscribeState>,
    config: TranscribeConfig,
) -> Result<TranscribeResult, String> {
    // Check if already transcribing
    let cancel_token = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        if s.is_transcribing {
            return Err("Transcription already in progress".to_string());
        }
        s.is_transcribing = true;
        let token = CancellationToken::new();
        s.cancellation_token = Some(token.clone());
        token
    };

    let _ = app.emit(
        "transcribe-progress",
        TranscribeProgressEvent {
            stage: "start".to_string(),
            message: "Starting transcription...".to_string(),
            percentage: 0.0,
        },
    );

    let ffmpeg_cmd = crate::commands::flashcards::media::resolve_ffmpeg_path(Some(&app)).await;
    let callbacks = tauri_callbacks(&app);

    let result =
        pipeline::transcribe_to_srt(&config, &ffmpeg_cmd, callbacks, &cancel_token).await;

    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.is_transcribing = false;
    }

    match result {
        Ok(outcome) => Ok(TranscribeResult {
            success: true,
            message: format!("Transcription completed: {} segments", outcome.subtitle_count),
            output_path: Some(outcome.output_path),
            subtitle_count: outcome.subtitle_count,
            detected_language: outcome.detected_language,
        }),
        Err(e) => Err(e.to_string()),
    }
}

/// Cancel ongoing transcription
#[tauri::command]
pub async fn transcribe_cancel(state: State<'_, AppTranscribeState>) -> Result<(), String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    if let Some(token) = s.cancellation_token.take() {
        token.cancel();
    }
    s.is_transcribing = false;
    Ok(())
}

/// Check if a file exists at the given path
#[tauri::command]
pub async fn transcribe_check_file_exists(path: String) -> bool {
    Path::new(&path).exists()
}
