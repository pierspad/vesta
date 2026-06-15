//! Tauri command wrappers around the [`srt_flashcards`] engine.
//!
//! These functions only translate between the Tauri world (app handle, managed
//! state, emitted events) and the pure engine API. No flashcard business logic
//! lives here.

use crate::state::AppFlashcardState;
use srt_flashcards::{
    AudioTrackInfo, FlashcardConfig, FlashcardProgressEvent, FlashcardResult, MediaTools,
    PreviewLine, SubFileInfo,
};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

use super::media::{resolve_ffmpeg_path, resolve_ffprobe_path};

async fn resolve_media_tools(app: &AppHandle) -> MediaTools {
    MediaTools {
        ffmpeg: resolve_ffmpeg_path(Some(app)).await,
        ffprobe: resolve_ffprobe_path(Some(app)).await,
    }
}

// ─── Commands ────────────────────────────────────────────────────────────────

/// Load a subtitle file and return summary info.
#[tauri::command]
pub async fn flashcard_load_subs(path: String) -> Result<SubFileInfo, String> {
    srt_flashcards::load_sub_file_info(&path)
}

/// Probe a media file and return its audio streams in ffmpeg `0:a:N` order.
#[tauri::command]
pub async fn flashcard_list_audio_tracks(
    app: AppHandle,
    path: String,
) -> Result<Vec<AudioTrackInfo>, String> {
    let ffprobe = resolve_ffprobe_path(Some(&app)).await;
    srt_flashcards::list_audio_tracks(&path, &ffprobe).await
}

/// Generate preview data: parse, match, filter, and return all lines.
#[tauri::command]
pub async fn flashcard_preview(config: FlashcardConfig) -> Result<Vec<PreviewLine>, String> {
    srt_flashcards::preview(&config)
}

/// Main generation command — runs the engine with parallel ffmpeg extraction,
/// forwarding progress to the frontend and honouring cancellation.
#[tauri::command]
pub async fn flashcard_generate(
    app: AppHandle,
    state: State<'_, AppFlashcardState>,
    config: FlashcardConfig,
) -> Result<FlashcardResult, String> {
    // Guard against concurrent runs and arm a fresh cancellation token.
    let cancel_token = {
        let mut fc_state = state.lock().map_err(|e| e.to_string())?;
        if fc_state.is_processing {
            return Err("Already processing flashcards".to_string());
        }
        fc_state.is_processing = true;
        let token = CancellationToken::new();
        fc_state.cancellation_token = Some(token.clone());
        token
    };

    let tools = resolve_media_tools(&app).await;

    let app_for_progress = app.clone();
    let progress = move |event: FlashcardProgressEvent| {
        let _ = app_for_progress.emit("flashcard-progress", event);
    };

    let result = srt_flashcards::generate(config, tools, cancel_token, &progress).await;

    if let Ok(mut fc_state) = state.lock() {
        fc_state.is_processing = false;
        fc_state.cancellation_token = None;
    }

    result
}

/// Cancel an in-flight flashcard generation.
#[tauri::command]
pub async fn flashcard_cancel(state: State<'_, AppFlashcardState>) -> Result<bool, String> {
    let mut fc_state = state.lock().map_err(|e| e.to_string())?;
    if let Some(ref token) = fc_state.cancellation_token {
        token.cancel();
    }
    fc_state.is_processing = false;
    fc_state.cancellation_token = None;
    Ok(true)
}

/// Check if ffmpeg is available (system `PATH` or downloaded into app data).
#[tauri::command]
pub async fn flashcard_check_deps(app: AppHandle) -> Result<bool, String> {
    if srt_flashcards::check_ffmpeg("ffmpeg").await {
        return Ok(true);
    }
    if let Ok(app_data) = app.path().app_local_data_dir() {
        let mut ffmpeg_path = app_data.join("ffmpeg_bin").join("ffmpeg");
        if cfg!(windows) {
            ffmpeg_path.set_extension("exe");
        }
        if ffmpeg_path.exists() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Download a static ffmpeg build into the app data directory.
#[tauri::command]
pub async fn flashcard_download_ffmpeg(app: AppHandle) -> Result<bool, String> {
    use ffmpeg_sidecar::download::{download_ffmpeg_package, ffmpeg_download_url, unpack_ffmpeg};

    let app_data = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let dest = app_data.join("ffmpeg_bin");
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;

    tokio::task::spawn_blocking(move || {
        let url = ffmpeg_download_url().map_err(|e| e.to_string())?;
        let archive = download_ffmpeg_package(url, &dest).map_err(|e| e.to_string())?;
        unpack_ffmpeg(&archive, &dest).map_err(|e| e.to_string())?;
        Ok(true)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Check if a directory exists.
#[tauri::command]
pub async fn flashcard_check_dir_exists(path: String) -> Result<bool, String> {
    Ok(std::path::Path::new(&path).is_dir())
}

/// Get the number of available CPU cores.
#[tauri::command]
pub async fn flashcard_get_cpu_count() -> Result<usize, String> {
    Ok(std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4))
}
