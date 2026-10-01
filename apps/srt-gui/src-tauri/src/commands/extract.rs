use super::flashcards::media::{resolve_ffmpeg_path, resolve_ffprobe_path};
use tauri::AppHandle;

#[tauri::command]
pub async fn embedded_subtitle_tracks(
    app: AppHandle,
    path: String,
) -> Result<Vec<srt_extract::embedded::EmbeddedSubtitleTrack>, String> {
    let ffprobe = resolve_ffprobe_path(Some(&app)).await;
    srt_extract::embedded::list_embedded_subtitles(&ffprobe, std::path::Path::new(&path))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn extract_embedded_subtitle(
    app: AppHandle,
    path: String,
    index: u32,
    output_path: String,
) -> Result<(), String> {
    let ffprobe = resolve_ffprobe_path(Some(&app)).await;
    let ffmpeg = resolve_ffmpeg_path(Some(&app)).await;
    srt_extract::embedded::extract_embedded_subtitle(
        &ffmpeg,
        &ffprobe,
        std::path::Path::new(&path),
        index,
        std::path::Path::new(&output_path),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_output_path(app: AppHandle, path: String, folder: bool) -> Result<(), String> {
    use tauri_plugin_shell::ShellExt;
    let path = std::path::PathBuf::from(path);
    let path = if folder {
        path.parent().ok_or("Output has no parent folder")?
    } else {
        path.as_path()
    };
    if !path.exists() {
        return Err("Output no longer exists".into());
    }
    #[allow(deprecated)]
    app.shell()
        .open(path.to_string_lossy(), None)
        .map_err(|e| e.to_string())
}

/// Preview uses a temporary extraction and caps returned text; no user output is written.
#[tauri::command]
pub async fn preview_embedded_subtitle(
    app: AppHandle,
    path: String,
    index: u32,
) -> Result<String, String> {
    let ffprobe = resolve_ffprobe_path(Some(&app)).await;
    let ffmpeg = resolve_ffmpeg_path(Some(&app)).await;
    srt_extract::embedded::preview_embedded_subtitle(
        &ffmpeg,
        &ffprobe,
        std::path::Path::new(&path),
        index,
    )
    .await
    .map_err(|error| error.to_string())
}
