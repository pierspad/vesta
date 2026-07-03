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

/// Check if ffmpeg is available. Uses the exact same resolution logic as every
/// feature that later *invokes* ffmpeg (system `PATH`, app-data download,
/// executable directory) and verifies the binary actually runs — so the
/// "ffmpeg missing" banner can never disagree with the real state.
#[tauri::command]
pub async fn flashcard_check_deps(app: AppHandle) -> Result<bool, String> {
    let ffmpeg = resolve_ffmpeg_path(Some(&app)).await;
    Ok(srt_flashcards::check_ffmpeg(&ffmpeg).await)
}

/// Locate `name` (e.g. `ffmpeg`) anywhere below `dir` and move it to the
/// directory root. Different `ffmpeg_sidecar` releases (and archive layouts on
/// Windows) unpack binaries into nested folders like `ffmpeg-xxx/bin/`.
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
            if depth > 0 && path.is_dir() {
                if let Some(found) = find(&path, file_name, depth - 1) {
                    return Some(found);
                }
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

/// Download a static ffmpeg build into the app data directory.
///
/// After unpacking, the binaries are hoisted to the destination root and the
/// installation is verified by actually running `ffmpeg -version`: the command
/// only returns `Ok` when the downloaded ffmpeg is genuinely usable.
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
        let url = ffmpeg_download_url().map_err(|e| format!("Could not determine download URL: {e}"))?;
        let archive = download_ffmpeg_package(url, &dest_task)
            .map_err(|e| format!("Download failed: {e}"))?;
        unpack_ffmpeg(&archive, &dest_task).map_err(|e| format!("Unpack failed: {e}"))?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())??;

    hoist_binary(&dest, "ffmpeg")?;
    hoist_binary(&dest, "ffprobe")?;

    // Final verification: resolve the way every consumer does and run it.
    let ffmpeg = resolve_ffmpeg_path(Some(&app)).await;
    if !srt_flashcards::check_ffmpeg(&ffmpeg).await {
        return Err(
            "ffmpeg was downloaded but could not be executed. \
             You can install it manually and place it next to the Vesta executable."
                .to_string(),
        );
    }
    Ok(true)
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

/// Get total physical system memory in megabytes.
#[tauri::command]
pub async fn flashcard_get_total_memory_mb() -> Result<u64, String> {
    #[cfg(target_os = "linux")]
    {
        let content = std::fs::read_to_string("/proc/meminfo")
            .map_err(|e| format!("Failed to read /proc/meminfo: {}", e))?;
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                let kb: u64 = line
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(4 * 1024 * 1024);
                return Ok(kb / 1024);
            }
        }
        Ok(4096)
    }
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("sysctl")
            .arg("-n")
            .arg("hw.memsize")
            .output()
            .map_err(|e| format!("sysctl failed: {}", e))?;
        let bytes: u64 = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .unwrap_or(4 * 1024 * 1024 * 1024);
        Ok(bytes / (1024 * 1024))
    }
    #[cfg(target_os = "windows")]
    {
        use std::mem;
        #[repr(C)]
        struct MEMORYSTATUSEX {
            dw_length: u32,
            dw_memory_load: u32,
            ull_total_phys: u64,
            ull_avail_phys: u64,
            ull_total_page_file: u64,
            ull_avail_page_file: u64,
            ull_total_virtual: u64,
            ull_avail_virtual: u64,
            ull_avail_extended_virtual: u64,
        }
        extern "system" {
            fn GlobalMemoryStatusEx(lp_buffer: *mut MEMORYSTATUSEX) -> i32;
        }
        let mut status: MEMORYSTATUSEX = unsafe { mem::zeroed() };
        status.dw_length = mem::size_of::<MEMORYSTATUSEX>() as u32;
        unsafe { GlobalMemoryStatusEx(&mut status) };
        Ok(status.ull_total_phys / (1024 * 1024))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Ok(4096)
    }
}

/// Write a list of preview lines to a temporary subtitle file and return its path.
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
    
    // Format lines to SRT
    let mut content = String::new();
    let mut counter = 1;
    for line in lines {
        if !line.active {
            continue;
        }
        
        let text = if use_native {
            line.subs2_text.clone()
        } else {
            Some(line.subs1_text.clone())
        };
        
        if let Some(txt) = text {
            let start_time = format_ms(line.start_ms);
            let end_time = format_ms(line.end_ms);
            
            content.push_str(&format!("{}\n", counter));
            content.push_str(&format!("{} --> {}\n", start_time, end_time));
            content.push_str(&format!("{}\n\n", txt));
            counter += 1;
        }
    }
    
    std::fs::write(&temp_file_path, content).map_err(|e| e.to_string())?;
    
    Ok(temp_file_path.to_string_lossy().to_string())
}

fn format_ms(ms: i64) -> String {
    let h = ms / 3600000;
    let m = (ms % 3600000) / 60000;
    let s = (ms % 60000) / 1000;
    let mss = ms % 1000;
    format!("{:02}:{:02}:{:02},{:03}", h, m, s, mss)
}
