use anyhow::{Context as _, Result};
use std::path::Path;
use tauri::{AppHandle, Manager};

// ─── FFmpeg Media Extraction ─────────────────────────────────────────────────

/// Check if ffmpeg is available in PATH
pub(crate) async fn check_ffmpeg() -> Result<bool> {
    let output = tokio::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .await;
    Ok(output.is_ok())
}

/// Resolves the FFmpeg binary path, falling back to AppData if missing from PATH
pub(crate) async fn resolve_ffmpeg_path(app: Option<&AppHandle>) -> String {
    if check_ffmpeg().await.unwrap_or(false) {
        return "ffmpeg".to_string();
    }
    if let Some(app) = app {
        if let Ok(app_data) = app.path().app_local_data_dir() {
            let ffmpeg_ext = if cfg!(windows) { "exe" } else { "" };
            let mut ffmpeg_path = app_data.join("ffmpeg_bin").join("ffmpeg");
            if cfg!(windows) {
                ffmpeg_path.set_extension(ffmpeg_ext);
            }
            if ffmpeg_path.exists() {
                return ffmpeg_path.to_string_lossy().to_string();
            }
        }
    }
    "ffmpeg".to_string()
}

/// Resolves the FFprobe binary path, falling back to AppData if missing from PATH
pub(crate) async fn resolve_ffprobe_path(app: Option<&AppHandle>) -> String {
    // Check if it's in system PATH
    let is_in_path = tokio::process::Command::new("ffprobe")
        .arg("-version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    if is_in_path {
        return "ffprobe".to_string();
    }

    // Check local AppData
    if let Some(app) = app {
        if let Ok(app_data) = app.path().app_local_data_dir() {
            let ffprobe_ext = if cfg!(windows) { "exe" } else { "" };
            let mut ffprobe_path = app_data.join("ffmpeg_bin").join("ffprobe");
            if cfg!(windows) {
                ffprobe_path.set_extension(ffprobe_ext);
            }
            if ffprobe_path.exists() {
                return ffprobe_path.to_string_lossy().to_string();
            }
        }
    }
    "ffprobe".to_string()
}

/// Format milliseconds as ffmpeg timestamp HH:MM:SS.mmm
pub(crate) fn ms_to_ffmpeg_ts(ms: i64) -> String {
    let ms = ms.max(0);
    let total_secs = ms / 1000;
    let millis = ms % 1000;
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = total_secs / 3600;
    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, millis)
}

/// Extract audio clip for a single subtitle line
pub(crate) async fn extract_audio_clip(
    source_path: &str,
    output_path: &Path,
    start_ms: i64,
    end_ms: i64,
    pad_start_ms: i64,
    pad_end_ms: i64,
    bitrate: u32,
    audio_track_index: Option<usize>,
    ffmpeg_cmd: &str,
) -> Result<()> {
    let actual_start = (start_ms - pad_start_ms).max(0);
    let duration_ms = (end_ms + pad_end_ms) - actual_start;

    let mut cmd = tokio::process::Command::new(ffmpeg_cmd);
    cmd.args([
        "-nostdin",
        "-loglevel",
        "error",
        "-y",
        "-ss",
        &ms_to_ffmpeg_ts(actual_start),
        "-t",
        &ms_to_ffmpeg_ts(duration_ms),
        "-i",
        source_path,
        "-vn",
        "-sn",
        "-dn",
    ]);
    if let Some(track_index) = audio_track_index {
        let audio_map = format!("0:a:{}", track_index);
        cmd.args(["-map", audio_map.as_str()]);
    }
    cmd.args([
        "-ac",
        "2",
        "-ab",
        &format!("{}k", bitrate),
        "-ar",
        "44100",
        "-f",
        "mp3",
    ]);
    cmd.arg(output_path.as_os_str());

    let output = cmd
        .output()
        .await
        .context("Failed to run ffmpeg for audio")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ffmpeg audio error: {}", stderr);
    }
    Ok(())
}

/// Extract snapshot at midpoint of subtitle
pub(crate) async fn extract_snapshot(
    video_path: &str,
    output_path: &Path,
    start_ms: i64,
    end_ms: i64,
    width: u32,
    height: u32,
    crop_bottom: u32,
    ffmpeg_cmd: &str,
) -> Result<()> {
    let midpoint_ms = start_ms + (end_ms - start_ms) / 2;

    let mut vf_filters = Vec::new();
    if crop_bottom > 0 {
        vf_filters.push(format!("crop=in_w:in_h-{}:0:0", crop_bottom));
    }
    vf_filters.push(format!("scale={}:{}:flags=bicubic", width, height));
    let vf = vf_filters.join(",");

    let mut cmd = tokio::process::Command::new(ffmpeg_cmd);
    cmd.args([
        "-nostdin",
        "-loglevel",
        "error",
        "-y",
        "-ss",
        &ms_to_ffmpeg_ts(midpoint_ms),
        "-i",
        video_path,
        "-an",
        "-sn",
        "-dn",
        "-vframes",
        "1",
        "-vf",
        &vf,
        "-pix_fmt",
        "yuvj420p",
        "-q:v",
        "2",
    ]);
    cmd.arg(output_path.as_os_str());

    let output = cmd
        .output()
        .await
        .context("Failed to run ffmpeg for snapshot")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ffmpeg snapshot error: {}", stderr);
    }
    Ok(())
}

/// Extract video clip for a single subtitle line
pub(crate) async fn extract_video_clip(
    video_path: &str,
    output_path: &Path,
    start_ms: i64,
    end_ms: i64,
    pad_start_ms: i64,
    pad_end_ms: i64,
    codec: &str,
    preset: &str,
    video_bitrate: u32,
    audio_bitrate: u32,
    audio_track_index: Option<usize>,
    width: u32,
    height: u32,
    crop_bottom: u32,
    ffmpeg_cmd: &str,
) -> Result<()> {
    let actual_start = (start_ms - pad_start_ms).max(0);
    let duration_ms = (end_ms + pad_end_ms) - actual_start;

    let mut vf_filters = Vec::new();
    if crop_bottom > 0 {
        vf_filters.push(format!("crop=in_w:in_h-{}:0:0", crop_bottom));
    }
    vf_filters.push(format!("scale={}:{}:flags=bicubic", width, height));
    let vf = vf_filters.join(",");

    let mut cmd = tokio::process::Command::new(ffmpeg_cmd);
    cmd.args([
        "-nostdin",
        "-loglevel",
        "error",
        "-y",
        "-ss",
        &ms_to_ffmpeg_ts(actual_start),
        "-t",
        &ms_to_ffmpeg_ts(duration_ms),
        "-i",
        video_path,
        "-vf",
        &vf,
    ]);
    if let Some(track_index) = audio_track_index {
        let audio_map = format!("0:a:{}", track_index);
        cmd.args(["-map", "0:v:0", "-map", audio_map.as_str()]);
    }

    match codec {
        "h264" => {
            cmd.args([
                "-c:v",
                "libx264",
                "-preset",
                preset,
                "-b:v",
                &format!("{}k", video_bitrate),
                "-c:a",
                "aac",
                "-b:a",
                &format!("{}k", audio_bitrate),
            ]);
        }
        _ => {
            // mpeg4
            cmd.args([
                "-c:v",
                "mpeg4",
                "-b:v",
                &format!("{}k", video_bitrate),
                "-c:a",
                "mp3",
                "-b:a",
                &format!("{}k", audio_bitrate),
            ]);
        }
    }

    cmd.arg(output_path.as_os_str());

    let output = cmd
        .output()
        .await
        .context("Failed to run ffmpeg for video clip")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ffmpeg video error: {}", stderr);
    }
    Ok(())
}

pub(crate) async fn normalize_audio(file_path: &Path, ffmpeg_cmd: &str) -> Result<()> {
    let temp_path = file_path.with_extension("normalized.mp3");

    let mut cmd = tokio::process::Command::new(ffmpeg_cmd);
    cmd.args(["-y", "-i"]);
    cmd.arg(file_path.as_os_str());
    cmd.args([
        "-af",
        "loudnorm=I=-16:TP=-1.5:LRA=11",
        "-ar",
        "44100",
        "-ac",
        "2",
    ]);
    cmd.arg(temp_path.as_os_str());

    let output = cmd.output().await.context("Failed to normalize audio")?;
    if output.status.success() {
        std::fs::rename(&temp_path, file_path)?;
    } else {
        let _ = std::fs::remove_file(&temp_path);
    }
    Ok(())
}
