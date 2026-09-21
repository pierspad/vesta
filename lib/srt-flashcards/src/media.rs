use anyhow::{Context as _, Result};
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

use super::types::{AudioFormat, SnapshotFormat};

/// Which per-card media file a name refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Audio(AudioFormat),
    Snapshot(SnapshotFormat),
    /// Video clips keep their existing codec-derived extension.
    Video(&'static str),
}

impl MediaKind {
    fn extension(self) -> &'static str {
        match self {
            Self::Audio(f) => f.extension(),
            Self::Snapshot(f) => f.extension(),
            Self::Video(ext) => ext,
        }
    }
}

/// The single place that knows what a per-card media file is called.
///
/// The generator, the TSV exporter and the apkg exporter (both its field
/// references and its media manifest) must all agree on this name; when they
/// each built it themselves, adding a format meant a five-file hunt and any
/// miss shipped a deck whose media silently failed to resolve.
pub fn media_filename(kind: MediaKind, deck_sanitized: &str, ep: u32, seq_num: usize) -> String {
    format!(
        "{}_{:03}_{:04}.{}",
        deck_sanitized,
        ep,
        seq_num,
        kind.extension()
    )
}

/// Extension used for a video clip produced with `video_codec`.
pub fn video_clip_extension(video_codec: &str) -> &'static str {
    if video_codec == "h264" { "mp4" } else { "avi" }
}

pub fn media_command(cmd: &str) -> tokio::process::Command {
    #[allow(unused_mut)]
    let mut command = tokio::process::Command::new(cmd);
    #[cfg(windows)]
    {
        command.creation_flags(0x0800_4000);
    }
    command.kill_on_drop(true);
    command
}

pub async fn check_ffmpeg(ffmpeg_cmd: &str) -> bool {
    media_command(ffmpeg_cmd)
        .arg("-version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false)
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum H264Encoder {
    #[default]
    Libx264,

    Nvenc,

    Amf,

    Qsv,

    Vaapi,

    VideoToolbox,
}

impl H264Encoder {
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
        _ => 8,
    };
    match encoder {
        H264Encoder::Nvenc => {
            let p = ["p1", "p1", "p2", "p3", "p4", "p5", "p6", "p7", "p7"][speed_rank];
            vec!["-preset".into(), p.into()]
        }
        H264Encoder::Qsv => {
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

        _ => Vec::new(),
    }
}

async fn test_h264_encoder(ffmpeg_cmd: &str, encoder: H264Encoder) -> bool {
    let mut cmd = media_command(ffmpeg_cmd);
    cmd.args([
        "-nostdin",
        "-loglevel",
        "error",
        "-f",
        "lavfi",
        "-i",
        "color=black:s=256x144:d=0.2",
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

#[derive(Debug, Clone)]
pub struct VideoStreamInfo {
    pub width: u32,
    pub height: u32,
    pub codec: String,
    pub duration_secs: f64,
}

pub async fn probe_video_stream(ffprobe_cmd: &str, video_path: &str) -> Option<VideoStreamInfo> {
    let output = media_command(ffprobe_cmd)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,codec_name:format=duration",
            "-of",
            "json",
            video_path,
        ])
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let val: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let stream = val.get("streams")?.as_array()?.first()?;
    let width = stream.get("width")?.as_u64()? as u32;
    let height = stream.get("height")?.as_u64()? as u32;
    let codec = stream.get("codec_name")?.as_str()?.to_string();
    let duration = val
        .get("format")
        .and_then(|f| f.get("duration"))
        .and_then(|d| d.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    Some(VideoStreamInfo {
        width,
        height,
        codec,
        duration_secs: duration,
    })
}

pub struct OptimizedVideo {
    pub path: PathBuf,
    pub original_used: bool,
    pub crop_applied: bool,
    _temp_file: Option<tempfile::NamedTempFile>,
}

impl OptimizedVideo {
    pub fn original(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            original_used: true,
            crop_applied: false,
            _temp_file: None,
        }
    }
}

pub fn should_optimize_video(
    info: &VideoStreamInfo,
    target_width: u32,
    target_height: u32,
    crop_bottom: u32,
    encoder: H264Encoder,
) -> bool {
    if crop_bottom > 0 {
        return true;
    }

    let is_heavy_codec = matches!(
        info.codec.to_lowercase().as_str(),
        "hevc" | "h265" | "av1" | "vp9" | "vp8" | "prores" | "vc1"
    );
    if is_heavy_codec {
        return true;
    }

    if encoder.is_hardware() && (info.width > target_width || info.height > target_height) {
        return true;
    }

    if info.width >= target_width * 2 || info.height >= target_height * 2 {
        return true;
    }

    false
}

#[allow(clippy::too_many_arguments)]
pub async fn optimize_video_source(
    source_path: &str,
    target_width: u32,
    target_height: u32,
    crop_bottom: u32,
    include_audio: bool,
    hw_accel_enabled: bool,
    ffmpeg_cmd: &str,
    ffprobe_cmd: &str,
    cancel: &CancellationToken,
) -> Result<OptimizedVideo> {
    if cancel.is_cancelled() {
        return Ok(OptimizedVideo::original(source_path));
    }

    let Some(stream_info) = probe_video_stream(ffprobe_cmd, source_path).await else {
        return Ok(OptimizedVideo::original(source_path));
    };

    let encoder = if hw_accel_enabled {
        detect_h264_encoder(ffmpeg_cmd).await
    } else {
        H264Encoder::Libx264
    };

    if !should_optimize_video(&stream_info, target_width, target_height, crop_bottom, encoder) {
        return Ok(OptimizedVideo::original(source_path));
    }

    let temp_file = tempfile::Builder::new()
        .prefix("vesta_opt_")
        .suffix(".mp4")
        .tempfile()
        .context("Failed to create temporary file for optimized video")?;
    let dest_path = temp_file.path().to_path_buf();
    let dest_str = dest_path.to_str().context("Invalid temp path")?;

    let target_w = (target_width.max(256) / 2) * 2;
    let target_h = (target_height.max(144) / 2) * 2;

    let mut candidate_args: Vec<(&'static str, Vec<String>)> = Vec::new();

    match encoder {
        H264Encoder::Vaapi => {
            if crop_bottom == 0 {
                let mut full_hw = vec![
                    "-init_hw_device".into(),
                    "vaapi=va".into(),
                    "-filter_hw_device".into(),
                    "va".into(),
                    "-hwaccel".into(),
                    "vaapi".into(),
                    "-hwaccel_output_format".into(),
                    "vaapi".into(),
                    "-i".into(),
                    source_path.into(),
                    "-vf".into(),
                    format!("scale_vaapi=w={target_w}:h={target_h}:format=nv12"),
                    "-c:v".into(),
                    "h264_vaapi".into(),
                    "-b:v".into(),
                    "1500k".into(),
                    "-g".into(),
                    "24".into(),
                ];
                if include_audio {
                    full_hw.extend(["-c:a".into(), "aac".into(), "-b:a".into(), "192k".into()]);
                } else {
                    full_hw.push("-an".into());
                }
                full_hw.extend(["-sn".into(), "-dn".into(), "-y".into(), dest_str.into()]);
                candidate_args.push(("vaapi_full_hw", full_hw));
            }

            let vf = if crop_bottom > 0 {
                format!("crop=in_w:in_h-{crop_bottom}:0:0,scale={target_w}:{target_h},format=nv12,hwupload")
            } else {
                format!("scale={target_w}:{target_h},format=nv12,hwupload")
            };
            let mut sw_hw = vec![
                "-init_hw_device".into(),
                "vaapi=va".into(),
                "-filter_hw_device".into(),
                "va".into(),
                "-i".into(),
                source_path.into(),
                "-vf".into(),
                vf,
                "-c:v".into(),
                "h264_vaapi".into(),
                "-b:v".into(),
                "1500k".into(),
                "-g".into(),
                "24".into(),
            ];
            if include_audio {
                sw_hw.extend(["-c:a".into(), "aac".into(), "-b:a".into(), "192k".into()]);
            } else {
                sw_hw.push("-an".into());
            }
            sw_hw.extend(["-sn".into(), "-dn".into(), "-y".into(), dest_str.into()]);
            candidate_args.push(("vaapi_sw_hw", sw_hw));
        }

        H264Encoder::Nvenc => {
            if crop_bottom == 0 {
                let mut hw_nv = vec![
                    "-hwaccel".into(),
                    "cuda".into(),
                    "-i".into(),
                    source_path.into(),
                    "-vf".into(),
                    format!("scale={target_w}:{target_h}"),
                    "-c:v".into(),
                    "h264_nvenc".into(),
                    "-preset".into(),
                    "p1".into(),
                    "-b:v".into(),
                    "1500k".into(),
                    "-g".into(),
                    "24".into(),
                ];
                if include_audio {
                    hw_nv.extend(["-c:a".into(), "aac".into(), "-b:a".into(), "192k".into()]);
                } else {
                    hw_nv.push("-an".into());
                }
                hw_nv.extend(["-sn".into(), "-dn".into(), "-y".into(), dest_str.into()]);
                candidate_args.push(("nvenc_hw", hw_nv));
            }

            let vf = if crop_bottom > 0 {
                format!("crop=in_w:in_h-{crop_bottom}:0:0,scale={target_w}:{target_h}")
            } else {
                format!("scale={target_w}:{target_h}")
            };
            let mut sw_nv = vec![
                "-i".into(),
                source_path.into(),
                "-vf".into(),
                vf,
                "-c:v".into(),
                "h264_nvenc".into(),
                "-preset".into(),
                "p1".into(),
                "-b:v".into(),
                "1500k".into(),
                "-g".into(),
                "24".into(),
            ];
            if include_audio {
                sw_nv.extend(["-c:a".into(), "aac".into(), "-b:a".into(), "192k".into()]);
            } else {
                sw_nv.push("-an".into());
            }
            sw_nv.extend(["-sn".into(), "-dn".into(), "-y".into(), dest_str.into()]);
            candidate_args.push(("nvenc_sw", sw_nv));
        }

        H264Encoder::VideoToolbox => {
            let vf = if crop_bottom > 0 {
                format!("crop=in_w:in_h-{crop_bottom}:0:0,scale={target_w}:{target_h}")
            } else {
                format!("scale={target_w}:{target_h}")
            };
            let mut vt = vec![
                "-i".into(),
                source_path.into(),
                "-vf".into(),
                vf,
                "-c:v".into(),
                "h264_videotoolbox".into(),
                "-b:v".into(),
                "1500k".into(),
                "-g".into(),
                "24".into(),
            ];
            if include_audio {
                vt.extend(["-c:a".into(), "aac".into(), "-b:a".into(), "192k".into()]);
            } else {
                vt.push("-an".into());
            }
            vt.extend(["-sn".into(), "-dn".into(), "-y".into(), dest_str.into()]);
            candidate_args.push(("videotoolbox", vt));
        }

        H264Encoder::Qsv => {
            let vf = if crop_bottom > 0 {
                format!("crop=in_w:in_h-{crop_bottom}:0:0,scale={target_w}:{target_h}")
            } else {
                format!("scale={target_w}:{target_h}")
            };
            let mut qsv = vec![
                "-init_hw_device".into(),
                "qsv=hw".into(),
                "-filter_hw_device".into(),
                "hw".into(),
                "-i".into(),
                source_path.into(),
                "-vf".into(),
                vf,
                "-c:v".into(),
                "h264_qsv".into(),
                "-preset".into(),
                "veryfast".into(),
                "-b:v".into(),
                "1500k".into(),
                "-g".into(),
                "24".into(),
            ];
            if include_audio {
                qsv.extend(["-c:a".into(), "aac".into(), "-b:a".into(), "192k".into()]);
            } else {
                qsv.push("-an".into());
            }
            qsv.extend(["-sn".into(), "-dn".into(), "-y".into(), dest_str.into()]);
            candidate_args.push(("qsv", qsv));
        }

        _ => {}
    }

    // Always include fast CPU fallback
    let cpu_vf = if crop_bottom > 0 {
        format!("crop=in_w:in_h-{crop_bottom}:0:0,scale={target_w}:{target_h}")
    } else {
        format!("scale={target_w}:{target_h}")
    };
    let mut cpu = vec![
        "-i".into(),
        source_path.into(),
        "-vf".into(),
        cpu_vf,
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "veryfast".into(),
        "-crf".into(),
        "23".into(),
        "-g".into(),
        "24".into(),
    ];
    if include_audio {
        cpu.extend(["-c:a".into(), "aac".into(), "-b:a".into(), "192k".into()]);
    } else {
        cpu.push("-an".into());
    }
    cpu.extend(["-sn".into(), "-dn".into(), "-y".into(), dest_str.into()]);
    candidate_args.push(("cpu_libx264", cpu));

    for (_name, args) in candidate_args {
        if cancel.is_cancelled() {
            return Ok(OptimizedVideo::original(source_path));
        }

        let mut cmd = media_command(ffmpeg_cmd);
        cmd.arg("-nostdin");
        cmd.arg("-loglevel").arg("error");
        cmd.args(&args);

        let res = cmd.status().await;
        if let Ok(status) = res {
            if status.success() {
                if let Ok(meta) = std::fs::metadata(&dest_path) {
                    if meta.len() > 1000 {
                        return Ok(OptimizedVideo {
                            path: dest_path,
                            original_used: false,
                            crop_applied: crop_bottom > 0,
                            _temp_file: Some(temp_file),
                        });
                    }
                }
            }
        }
    }

    eprintln!("Warning: all video optimization attempts failed; using original video source");
    Ok(OptimizedVideo::original(source_path))
}

pub(crate) fn ms_to_ffmpeg_ts(ms: i64) -> String {
    let ms = ms.max(0);
    let total_secs = ms / 1000;
    let millis = ms % 1000;
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = total_secs / 3600;
    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, millis)
}

fn clip_window(start_ms: i64, end_ms: i64, pad_start_ms: i64, pad_end_ms: i64) -> (String, String) {
    let actual_start = (start_ms - pad_start_ms).max(0);
    let duration_ms = (end_ms + pad_end_ms) - actual_start;
    (ms_to_ffmpeg_ts(actual_start), ms_to_ffmpeg_ts(duration_ms))
}

fn scale_vf(width: u32, height: u32, crop_bottom: u32) -> String {
    let mut filters = Vec::new();
    if crop_bottom > 0 {
        filters.push(format!("crop=in_w:in_h-{crop_bottom}:0:0"));
    }
    filters.push(format!("scale={width}:{height}:flags=bicubic"));
    filters.join(",")
}

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
    format: AudioFormat,
    normalize: bool,
    gain_db: i32,
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
    cmd.args(["-ac", "2"]);

    // Loudness normalisation rides along with the one encode we already do.
    // Target -14 LUFS (modern web / dialogue standard) and optional gain.
    let mut af_filters = Vec::new();
    if normalize {
        af_filters.push("loudnorm=I=-14:TP=-1.5:LRA=11".to_string());
    }
    if gain_db != 0 {
        af_filters.push(format!("volume={gain_db}dB"));
        af_filters.push("alimiter=limit=-0.2dB".to_string());
    }
    if !af_filters.is_empty() {
        cmd.args(["-af", &af_filters.join(",")]);
    }

    for arg in format.ffmpeg_args(bitrate) {
        cmd.arg(arg);
    }
    cmd.arg(output_path.as_os_str());

    run_ffmpeg(cmd, "audio").await
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn extract_snapshot(
    video_path: &str,
    output_path: &Path,
    start_ms: i64,
    end_ms: i64,
    width: u32,
    height: u32,
    crop_bottom: u32,
    format: SnapshotFormat,
    quality: u8,
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
    ]);
    for arg in format.ffmpeg_args(quality) {
        cmd.arg(arg);
    }
    cmd.arg(output_path.as_os_str());

    run_ffmpeg(cmd, "snapshot").await
}

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
    normalize: bool,
    gain_db: i32,
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

    let mut af_filters = Vec::new();
    if normalize {
        af_filters.push("loudnorm=I=-14:TP=-1.5:LRA=11".to_string());
    }
    if gain_db != 0 {
        af_filters.push(format!("volume={gain_db}dB"));
        af_filters.push("alimiter=limit=-0.2dB".to_string());
    }
    if !af_filters.is_empty() {
        cmd.args(["-af", &af_filters.join(",")]);
    }

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

#[allow(clippy::too_many_arguments)]
pub async fn extract_preview_audio_clip(
    source_path: &str,
    output_path: &Path,
    start_ms: i64,
    end_ms: i64,
    pad_start_ms: i64,
    pad_end_ms: i64,
    bitrate: u32,
    audio_track_index: Option<usize>,
    format: AudioFormat,
    normalize: bool,
    gain_db: i32,
    ffmpeg_cmd: &str,
) -> Result<()> {
    extract_audio_clip(
        source_path,
        output_path,
        start_ms,
        end_ms,
        pad_start_ms,
        pad_end_ms,
        bitrate,
        audio_track_index,
        format,
        normalize,
        gain_db,
        ffmpeg_cmd,
    )
    .await
}

pub async fn extract_preview_snapshot(
    video_path: &str,
    output_path: &Path,
    time_ms: i64,
    ffmpeg_cmd: &str,
) -> Result<()> {
    extract_snapshot(
        video_path,
        output_path,
        time_ms,
        time_ms + 1000,
        320,
        180,
        0,
        SnapshotFormat::Jpeg,
        80,
        ffmpeg_cmd,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filenames_carry_the_extension_of_their_format() {
        assert_eq!(
            media_filename(MediaKind::Audio(AudioFormat::Opus), "Deck", 3, 42),
            "Deck_003_0042.opus"
        );
        assert_eq!(
            media_filename(MediaKind::Snapshot(SnapshotFormat::Webp), "Deck", 3, 42),
            "Deck_003_0042.webp"
        );
        assert_eq!(
            media_filename(MediaKind::Snapshot(SnapshotFormat::Jpeg), "Deck", 1, 1),
            "Deck_001_0001.jpg"
        );
    }

    #[test]
    fn sequence_and_episode_stay_zero_padded() {
        // Anki sorts these as strings; losing the padding reorders the deck.
        assert_eq!(
            media_filename(MediaKind::Audio(AudioFormat::Mp3), "D", 1, 7),
            "D_001_0007.mp3"
        );
        assert_eq!(
            media_filename(MediaKind::Audio(AudioFormat::Mp3), "D", 12, 1234),
            "D_012_1234.mp3"
        );
    }

    #[test]
    fn video_extension_follows_codec() {
        assert_eq!(video_clip_extension("h264"), "mp4");
        assert_eq!(video_clip_extension("mpeg4"), "avi");
    }

    #[tokio::test]
    async fn detects_vaapi_on_linux_when_available() {
        let enc = detect_h264_encoder("ffmpeg").await;
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            if Path::new("/dev/dri/renderD128").exists() {
                assert_eq!(enc, H264Encoder::Vaapi);
            }
        }
    }

    #[tokio::test]
    async fn probes_detour_video_stream() {
        let path = "../../Test_Subs/FILM/Detour(1945).mp4";
        if Path::new(path).exists() {
            let info = probe_video_stream("ffprobe", path).await.expect("probed");
            assert_eq!(info.width, 412);
            assert_eq!(info.height, 300);
            assert_eq!(info.codec, "h264");
            assert!(info.duration_secs > 4000.0);
        }
    }
}
