//! FFmpeg-backed media extraction (audio clips, snapshots, video clips).
//!
//! This module is intentionally free of any GUI/Tauri coupling: every function
//! receives the ffmpeg executable to invoke as a plain `&str`, so the same code
//! powers the desktop app, the headless CLI and the benchmark harness.

use anyhow::{Context as _, Result};
use std::path::Path;

// ─── FFmpeg Media Extraction ─────────────────────────────────────────────────

/// Build a `tokio` command for a media tool with platform-appropriate flags.
///
/// On Windows this suppresses the console window (no flashing terminals during
/// batch extraction) and lowers the process priority so that running dozens of
/// parallel ffmpeg instances doesn't freeze the user's machine.
pub fn media_command(cmd: &str) -> tokio::process::Command {
    #[allow(unused_mut)]
    let mut command = tokio::process::Command::new(cmd);
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW (0x0800_0000) | BELOW_NORMAL_PRIORITY_CLASS (0x0000_4000)
        command.creation_flags(0x0800_4000);
    }
    command
}

/// Returns `true` if the given ffmpeg executable can be invoked (`<cmd> -version`).
///
/// `ffmpeg_cmd` may be a bare command resolved through `PATH` (e.g. `"ffmpeg"`)
/// or an absolute path to a bundled binary.
pub async fn check_ffmpeg(ffmpeg_cmd: &str) -> bool {
    media_command(ffmpeg_cmd)
        .arg("-version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Returns `true` if `video_path` exposes at least one audio stream, probed via
/// `ffprobe_cmd`. Any failure to run ffprobe (missing binary, unreadable file)
/// is treated as "no audio". Synchronous: it is a one-shot startup probe used by
/// the benchmark harness, not part of the hot extraction loop.
pub fn video_has_audio(ffprobe_cmd: &str, video_path: &str) -> bool {
    std::process::Command::new(ffprobe_cmd)
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type",
            "-of",
            "csv=p=0",
            video_path,
        ])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).contains("audio"))
        .unwrap_or(false)
}

// ─── H.264 hardware encoder detection ────────────────────────────────────────

/// H.264 encoder to use for video clips.
///
/// `Libx264` is the software fallback and the only entry guaranteed to exist
/// in any ffmpeg build; the others are GPU-backed and must be probed with
/// [`detect_h264_encoder`] before use (being compiled in does not mean the
/// device is actually usable at runtime).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum H264Encoder {
    #[default]
    Libx264,
    /// NVIDIA NVENC (Linux/Windows).
    Nvenc,
    /// AMD AMF (Windows).
    Amf,
    /// Intel Quick Sync (Linux/Windows).
    Qsv,
    /// VA-API (Linux: AMD/Intel, portable DRM path).
    Vaapi,
    /// Apple VideoToolbox (macOS).
    VideoToolbox,
}

impl H264Encoder {
    /// ffmpeg encoder name (`-c:v` value).
    pub fn ffmpeg_name(self) -> &'static str {
        match self {
            Self::Libx264 => "libx264",
            Self::Nvenc => "h264_nvenc",
            Self::Amf => "h264_amf",
            Self::Qsv => "h264_qsv",
            Self::Vaapi => "h264_vaapi",
            Self::VideoToolbox => "h264_videotoolbox",
        }
    }

    /// Human-readable label for logs/UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::Libx264 => "libx264 (CPU)",
            Self::Nvenc => "NVENC (NVIDIA GPU)",
            Self::Amf => "AMF (AMD GPU)",
            Self::Qsv => "Quick Sync (Intel GPU)",
            Self::Vaapi => "VA-API (GPU)",
            Self::VideoToolbox => "VideoToolbox (Apple)",
        }
    }

    pub fn is_hardware(self) -> bool {
        self != Self::Libx264
    }
}

/// Map a libx264-style preset (`ultrafast`..`placebo`) onto the equivalent
/// option for a hardware encoder. Returns the extra `-preset`/`-quality` args.
fn hw_preset_args(encoder: H264Encoder, x264_preset: &str) -> Vec<String> {
    let speed_rank = match x264_preset {
        "ultrafast" => 0,
        "superfast" => 1,
        "veryfast" => 2,
        "faster" => 3,
        "fast" => 4,
        "medium" => 5,
        "slow" => 6,
        "slower" => 7,
        _ => 8, // veryslow / placebo
    };
    match encoder {
        H264Encoder::Nvenc => {
            // NVENC: p1 (fastest) .. p7 (slowest/best).
            let p = ["p1", "p1", "p2", "p3", "p4", "p5", "p6", "p7", "p7"][speed_rank];
            vec!["-preset".into(), p.into()]
        }
        H264Encoder::Qsv => {
            // QSV accepts the libx264 names from veryfast to veryslow.
            let p = [
                "veryfast", "veryfast", "veryfast", "faster", "fast", "medium", "slow", "slower",
                "veryslow",
            ][speed_rank];
            vec!["-preset".into(), p.into()]
        }
        H264Encoder::Amf => {
            let q = if speed_rank <= 2 {
                "speed"
            } else if speed_rank <= 5 {
                "balanced"
            } else {
                "quality"
            };
            vec!["-quality".into(), q.into()]
        }
        // VA-API and VideoToolbox have no preset concept.
        _ => Vec::new(),
    }
}

/// Functional probe: try a tiny synthetic encode with `encoder`. This catches
/// both "encoder not compiled in" and "no usable GPU/driver at runtime".
async fn test_h264_encoder(ffmpeg_cmd: &str, encoder: H264Encoder) -> bool {
    let mut cmd = media_command(ffmpeg_cmd);
    cmd.args([
        "-nostdin",
        "-loglevel",
        "error",
        "-f",
        "lavfi",
        "-i",
        "color=black:s=192x108:d=0.2",
    ]);
    if encoder == H264Encoder::Vaapi {
        cmd.args([
            "-init_hw_device",
            "vaapi=va",
            "-filter_hw_device",
            "va",
            "-vf",
            "format=nv12,hwupload",
        ]);
    }
    cmd.args(["-c:v", encoder.ffmpeg_name(), "-f", "null", "-"]);
    cmd.output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Detect the best available H.264 encoder, in platform-specific order of
/// preference, falling back to `libx264`. Run this ONCE per generation run
/// (it spawns a few short-lived ffmpeg processes), then reuse the result for
/// every clip.
pub async fn detect_h264_encoder(ffmpeg_cmd: &str) -> H264Encoder {
    #[cfg(target_os = "macos")]
    let candidates = [H264Encoder::VideoToolbox];
    #[cfg(windows)]
    let candidates = [H264Encoder::Nvenc, H264Encoder::Qsv, H264Encoder::Amf];
    #[cfg(all(unix, not(target_os = "macos")))]
    let candidates = [H264Encoder::Nvenc, H264Encoder::Vaapi, H264Encoder::Qsv];

    for encoder in candidates {
        if test_h264_encoder(ffmpeg_cmd, encoder).await {
            return encoder;
        }
    }
    H264Encoder::Libx264
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

/// `-ss` / `-t` timestamps for a padded `[start, end]` window (milliseconds).
/// The start is clamped at zero so leading padding never seeks before the file.
fn clip_window(start_ms: i64, end_ms: i64, pad_start_ms: i64, pad_end_ms: i64) -> (String, String) {
    let actual_start = (start_ms - pad_start_ms).max(0);
    let duration_ms = (end_ms + pad_end_ms) - actual_start;
    (ms_to_ffmpeg_ts(actual_start), ms_to_ffmpeg_ts(duration_ms))
}

/// Build the ffmpeg `-vf` value: an optional bottom crop followed by a scale.
fn scale_vf(width: u32, height: u32, crop_bottom: u32) -> String {
    let mut filters = Vec::new();
    if crop_bottom > 0 {
        filters.push(format!("crop=in_w:in_h-{crop_bottom}:0:0"));
    }
    filters.push(format!("scale={width}:{height}:flags=bicubic"));
    filters.join(",")
}

/// Run a fully-built ffmpeg command, turning a non-zero exit into an error that
/// carries ffmpeg's stderr. `what` names the step (e.g. `"audio"`) for diagnostics.
async fn run_ffmpeg(mut cmd: tokio::process::Command, what: &str) -> Result<()> {
    let output = cmd
        .output()
        .await
        .with_context(|| format!("Failed to run ffmpeg for {what}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ffmpeg {what} error: {stderr}");
    }
    Ok(())
}

/// Extract audio clip for a single subtitle line
#[allow(clippy::too_many_arguments)]
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
    let (start_ts, duration_ts) = clip_window(start_ms, end_ms, pad_start_ms, pad_end_ms);

    let mut cmd = media_command(ffmpeg_cmd);
    cmd.args([
        "-nostdin",
        "-loglevel",
        "error",
        "-y",
        "-ss",
        &start_ts,
        "-t",
        &duration_ts,
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

    run_ffmpeg(cmd, "audio").await
}

/// Extract snapshot at midpoint of subtitle
#[allow(clippy::too_many_arguments)]
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
    let vf = scale_vf(width, height, crop_bottom);

    let mut cmd = media_command(ffmpeg_cmd);
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

    run_ffmpeg(cmd, "snapshot").await
}

/// Extract video clip for a single subtitle line.
///
/// `encoder` is honoured only when `codec == "h264"`; pass the value returned
/// by [`detect_h264_encoder`] (or [`H264Encoder::Libx264`] to force software).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn extract_video_clip(
    video_path: &str,
    output_path: &Path,
    start_ms: i64,
    end_ms: i64,
    pad_start_ms: i64,
    pad_end_ms: i64,
    codec: &str,
    preset: &str,
    encoder: H264Encoder,
    video_bitrate: u32,
    audio_bitrate: u32,
    audio_track_index: Option<usize>,
    width: u32,
    height: u32,
    crop_bottom: u32,
    ffmpeg_cmd: &str,
) -> Result<()> {
    let (start_ts, duration_ts) = clip_window(start_ms, end_ms, pad_start_ms, pad_end_ms);

    let mut cmd = media_command(ffmpeg_cmd);
    cmd.args([
        "-nostdin",
        "-loglevel",
        "error",
        "-y",
        "-ss",
        &start_ts,
        "-t",
        &duration_ts,
        "-i",
        video_path,
    ]);

    // VA-API encodes from GPU surfaces: the scaled frame must be uploaded to
    // the device at the end of the filter chain.
    let vf = if codec == "h264" && encoder == H264Encoder::Vaapi {
        cmd.args(["-init_hw_device", "vaapi=va", "-filter_hw_device", "va"]);
        format!(
            "{},format=nv12,hwupload",
            scale_vf(width, height, crop_bottom)
        )
    } else {
        scale_vf(width, height, crop_bottom)
    };
    cmd.args(["-vf", &vf]);

    if let Some(track_index) = audio_track_index {
        let audio_map = format!("0:a:{}", track_index);
        cmd.args(["-map", "0:v:0", "-map", audio_map.as_str()]);
    }

    match codec {
        "h264" => {
            cmd.args(["-c:v", encoder.ffmpeg_name()]);
            if encoder == H264Encoder::Libx264 {
                cmd.args(["-preset", preset]);
            } else {
                for arg in hw_preset_args(encoder, preset) {
                    cmd.arg(arg);
                }
            }
            cmd.args([
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

    run_ffmpeg(cmd, "video").await
}

/// Loudness-normalize an audio file in place (EBU R128 via ffmpeg `loudnorm`).
pub(crate) async fn normalize_audio(file_path: &Path, ffmpeg_cmd: &str) -> Result<()> {
    let temp_path = file_path.with_extension("normalized.mp3");

    let mut cmd = media_command(ffmpeg_cmd);
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
