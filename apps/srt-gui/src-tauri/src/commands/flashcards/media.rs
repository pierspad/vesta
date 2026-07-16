use tauri::{AppHandle, Manager};

pub async fn resolve_ffmpeg_path(app: Option<&AppHandle>) -> String {
    resolve(app, "ffmpeg").await
}

pub async fn resolve_ffprobe_path(app: Option<&AppHandle>) -> String {
    resolve(app, "ffprobe").await
}

async fn resolve(app: Option<&AppHandle>, name: &str) -> String {
    if srt_flashcards::check_ffmpeg(name).await {
        return name.to_string();
    }
    if let Some(app) = app
        && let Ok(app_data) = app.path().app_local_data_dir()
    {
        let mut path = app_data.join("ffmpeg_bin").join(name);
        if cfg!(windows) {
            path.set_extension("exe");
        }
        if path.exists() {
            return path.to_string_lossy().to_string();
        }
    }

    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let mut path = dir.join(name);
        if cfg!(windows) {
            path.set_extension("exe");
        }
        if path.exists() {
            return path.to_string_lossy().to_string();
        }
    }
    name.to_string()
}
