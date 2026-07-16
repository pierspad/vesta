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

#[tauri::command]
pub async fn flashcard_load_subs(path: String) -> Result<SubFileInfo, String> {
    srt_flashcards::load_sub_file_info(&path)
}

#[tauri::command]
pub async fn flashcard_list_audio_tracks(
    app: AppHandle,
    path: String,
) -> Result<Vec<AudioTrackInfo>, String> {
    let ffprobe = resolve_ffprobe_path(Some(&app)).await;
    srt_flashcards::list_audio_tracks(&path, &ffprobe).await
}

#[tauri::command]
pub async fn flashcard_preview(config: FlashcardConfig) -> Result<Vec<PreviewLine>, String> {
    srt_flashcards::preview(&config)
}

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

#[tauri::command]
pub async fn flashcard_check_deps(app: AppHandle) -> Result<bool, String> {
    let ffmpeg = resolve_ffmpeg_path(Some(&app)).await;
    Ok(srt_flashcards::check_ffmpeg(&ffmpeg).await)
}

fn hoist_binary(dir: &std::path::Path, name: &str) -> Result<(), String> {
    let file_name = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };
    let target = dir.join(&file_name);
    if target.exists() {
        return Ok(());
    }

    fn find(dir: &std::path::Path, file_name: &str, depth: u8) -> Option<std::path::PathBuf> {
        let entries = std::fs::read_dir(dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.file_name().is_some_and(|n| n == file_name) {
                return Some(path);
            }
            if depth > 0
                && path.is_dir()
                && let Some(found) = find(&path, file_name, depth - 1)
            {
                return Some(found);
            }
        }
        None
    }

    if let Some(found) = find(dir, &file_name, 4) {
        std::fs::rename(&found, &target)
            .or_else(|_| std::fs::copy(&found, &target).map(|_| ()))
            .map_err(|e| format!("Failed to move {file_name} into place: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn flashcard_download_ffmpeg(app: AppHandle) -> Result<bool, String> {
    use ffmpeg_sidecar::download::{download_ffmpeg_package, ffmpeg_download_url, unpack_ffmpeg};

    let app_data = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let dest = app_data.join("ffmpeg_bin");
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;

    let dest_task = dest.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let url =
            ffmpeg_download_url().map_err(|e| format!("Could not determine download URL: {e}"))?;
        let archive = download_ffmpeg_package(url, &dest_task)
            .map_err(|e| format!("Download failed: {e}"))?;
        unpack_ffmpeg(&archive, &dest_task).map_err(|e| format!("Unpack failed: {e}"))?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;

    hoist_binary(&dest, "ffmpeg")?;
    hoist_binary(&dest, "ffprobe")?;

    let ffmpeg = resolve_ffmpeg_path(Some(&app)).await;
    if !srt_flashcards::check_ffmpeg(&ffmpeg).await {
        return Err("ffmpeg was downloaded but could not be executed. \
             You can install it manually and place it next to the Vesta executable."
            .to_string());
    }
    Ok(true)
}

#[tauri::command]
pub async fn flashcard_check_dir_exists(path: String) -> Result<bool, String> {
    Ok(std::path::Path::new(&path).is_dir())
}

#[tauri::command]
pub async fn flashcard_get_cpu_count() -> Result<usize, String> {
    Ok(std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4))
}

#[tauri::command]
pub async fn flashcard_get_total_memory_mb() -> Result<u64, String> {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let total_mb = sys.total_memory() / (1024 * 1024);

    Ok(if total_mb > 0 { total_mb } else { 4096 })
}

#[tauri::command]
pub async fn save_temp_subtitles(
    app: tauri::AppHandle,
    lines: Vec<PreviewLine>,
    use_native: bool,
) -> Result<String, String> {
    let app_data = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let temp_dir = app_data.join("temp_subs");
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let suffix = if use_native { "native" } else { "target" };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let temp_file_path = temp_dir.join(format!("temp_{}_{}.srt", suffix, now));

    let mut subtitles = std::collections::HashMap::new();
    let mut counter: u32 = 1;
    for line in lines {
        if !line.active {
            continue;
        }

        let text = if use_native {
            line.subs2_text.clone()
        } else {
            Some(line.subs1_text.clone())
        };

        let Some(text) = text else { continue };

        subtitles.insert(
            counter,
            srt_parser::Subtitle {
                id: counter,
                start: srt_parser::Timestamp {
                    milliseconds: line.start_ms.max(0) as u64,
                },
                end: srt_parser::Timestamp {
                    milliseconds: line.end_ms.max(0) as u64,
                },
                text,
            },
        );
        counter += 1;
    }

    srt_parser::SrtParser::save_file(&temp_file_path, &subtitles).map_err(|e| e.to_string())?;

    Ok(temp_file_path.to_string_lossy().to_string())
}
