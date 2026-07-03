//! App-level ffmpeg/ffprobe path resolution.
//!
//! The media *extraction* logic now lives in the `srt-flashcards` crate; this
//! small module only resolves which binary to invoke for the desktop app —
//! preferring the system `PATH`, then falling back to a build the app may have
//! downloaded into its local data directory. It is shared by the flashcards,
//! transcription and sync features.

use tauri::{AppHandle, Manager};

/// Resolve the ffmpeg executable (system `PATH`, else app-data download).
pub async fn resolve_ffmpeg_path(app: Option<&AppHandle>) -> String {
    resolve(app, "ffmpeg").await
}

/// Resolve the ffprobe executable (system `PATH`, else app-data download).
pub async fn resolve_ffprobe_path(app: Option<&AppHandle>) -> String {
    resolve(app, "ffprobe").await
}

async fn resolve(app: Option<&AppHandle>, name: &str) -> String {
    if srt_flashcards::check_ffmpeg(name).await {
        return name.to_string();
    }
    if let Some(app) = app {
        if let Ok(app_data) = app.path().app_local_data_dir() {
            let mut path = app_data.join("ffmpeg_bin").join(name);
            if cfg!(windows) {
                path.set_extension("exe");
            }
            if path.exists() {
                return path.to_string_lossy().to_string();
            }
        }
    }
    // Manual installs: a binary placed next to the Vesta executable.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let mut path = dir.join(name);
            if cfg!(windows) {
                path.set_extension("exe");
            }
            if path.exists() {
                return path.to_string_lossy().to_string();
            }
        }
    }
    name.to_string()
}
