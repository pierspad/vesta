use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use srt_condense::{CondenseConfig, CondenseResult};

use super::flashcards::media::resolve_ffmpeg_path;
use crate::state::AppCondenseState;

#[tauri::command]
pub async fn condense_start(
    app: AppHandle,
    config: CondenseConfig,
    state: State<'_, AppCondenseState>,
) -> Result<CondenseResult, String> {
    let token = CancellationToken::new();
    {
        let mut condense_state = state.lock().map_err(|e| e.to_string())?;
        if condense_state.is_running {
            return Err("ERR_ALREADY_RUNNING".to_string());
        }
        condense_state.is_running = true;
        condense_state.cancellation_token = Some(token.clone());
    }

    let ffmpeg = resolve_ffmpeg_path(Some(&app)).await;
    let progress_app = app.clone();
    let on_progress = move |p: srt_condense::CondenseProgress| {
        let _ = progress_app.emit("condense-progress", &p);
    };

    let result = srt_condense::condense(config, &ffmpeg, &on_progress, token).await;

    if let Ok(mut condense_state) = state.lock() {
        condense_state.is_running = false;
        condense_state.cancellation_token = None;
    }

    result
}

/// Cancella la condensazione in corso.
#[tauri::command]
pub async fn condense_cancel(state: State<'_, AppCondenseState>) -> Result<bool, String> {
    let condense_state = state.lock().map_err(|e| e.to_string())?;
    if let Some(token) = &condense_state.cancellation_token {
        token.cancel();
        Ok(true)
    } else {
        Ok(false)
    }
}

fn anki_url(url: Option<String>) -> String {
    url.filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| srt_ankiconnect::DEFAULT_URL.to_string())
}

#[tauri::command]
pub async fn ankiconnect_ping(url: Option<String>) -> Result<u32, String> {
    srt_ankiconnect::ping(&anki_url(url)).await
}

#[tauri::command]
pub async fn ankiconnect_deck_names(url: Option<String>) -> Result<Vec<String>, String> {
    srt_ankiconnect::deck_names(&anki_url(url)).await
}

#[tauri::command]
pub async fn ankiconnect_import_package(path: String, url: Option<String>) -> Result<bool, String> {
    srt_ankiconnect::import_package(&anki_url(url), &path).await?;
    Ok(true)
}
