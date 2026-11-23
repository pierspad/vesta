//! Hardcoded (burned-in) video subtitle ROI preview and frame deduplication extractor.

use crate::bitmap::{BitmapSubtitleCue, decode_png_to_rgba, run_cancellable_output};
use anyhow::{Context as _, Result, bail};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt as _;
use tokio_util::sync::CancellationToken;

const GRID_COLS: usize = 32;
const GRID_ROWS: usize = 12;
const GRID_CELLS: usize = GRID_COLS * GRID_ROWS;
const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

/// Normalized `[0.0, 1.0]` region of interest where burned-in subtitles appear.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct VideoRoi {
    /// Normalized left coordinate in `0.0..=1.0`.
    pub x: f64,
    /// Normalized top coordinate in `0.0..=1.0`.
    pub y: f64,
    /// Normalized width (`>= 0.02`).
    pub width: f64,
    /// Normalized height (`>= 0.02`).
    pub height: f64,
}

impl VideoRoi {
    /// Validates that ROI coordinates are finite, non-tiny, and within normalized video bounds.
    pub fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            self.x.is_finite()
                && self.y.is_finite()
                && self.width.is_finite()
                && self.height.is_finite(),
            "Video ROI coordinates must be finite numbers"
        );
        anyhow::ensure!(
            self.x >= 0.0 && self.y >= 0.0,
            "Video ROI x and y must be >= 0.0"
        );
        anyhow::ensure!(
            self.width >= 0.02 && self.height >= 0.02,
            "Video ROI width and height must be >= 0.02"
        );
        anyhow::ensure!(
            self.x + self.width <= 1.001 && self.y + self.height <= 1.001,
            "Video ROI extends beyond normalized frame bounds [0.0, 1.0]"
        );
        Ok(())
    }

    /// Validates this ROI and clamps `width`/`height` so `x + width <= 1.0` and `y + height <= 1.0`.
    pub fn clamped(&self) -> Result<Self> {
        self.validate()?;
        let x = self.x.clamp(0.0, 0.98);
        let y = self.y.clamp(0.0, 0.98);
        let width = self.width.min((1.0 - x).max(0.02));
        let height = self.height.min((1.0 - y).max(0.02));
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }
}

/// Base64-encoded PNG preview frame extracted from a video for ROI selection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoFramePreview {
    /// `data:image/png;base64,...` URI for UI display.
    pub data_url: String,
    /// Decoded preview image width in pixels.
    pub width: u32,
    /// Decoded preview image height in pixels.
    pub height: u32,
    /// Probed video duration in milliseconds (`0` if container lacks duration metadata).
    pub duration_ms: u64,
    /// Timestamp in milliseconds at which this frame was captured.
    pub time_ms: u64,
}

#[derive(Debug, Deserialize)]
struct FfprobeVideoProbe {
    streams: Option<Vec<FfprobeVideoStream>>,
    format: Option<FfprobeVideoFormat>,
}

#[derive(Debug, Deserialize)]
struct FfprobeVideoStream {
    width: Option<u32>,
    height: Option<u32>,
    duration: Option<String>,
    start_time: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeVideoFormat {
    duration: Option<String>,
    start_time: Option<String>,
}

struct ProbedVideoInfo {
    duration_ms: u64,
    start_offset_ms: u64,
}

fn compute_stream_start_offset_ms(format_start: Option<&str>, stream_start: Option<&str>) -> u64 {
    let parse_sec = |raw: Option<&str>| -> Option<f64> {
        let secs = raw?.trim().parse::<f64>().ok()?;
        secs.is_finite().then_some(secs)
    };
    let fmt_start = parse_sec(format_start).unwrap_or(0.0);
    let strm_start = parse_sec(stream_start).unwrap_or(fmt_start);
    ((strm_start - fmt_start).max(0.0) * 1000.0).round() as u64
}

async fn probe_video_stream(
    ffprobe: &str,
    input: &Path,
    cancel: Option<&CancellationToken>,
) -> Result<ProbedVideoInfo> {
    let mut cmd = tokio::process::Command::new(ffprobe);
    cmd.args([
        "-v",
        "error",
        "-select_streams",
        "v:0",
        "-show_entries",
        "stream=width,height,duration,start_time:format=duration,start_time",
        "-of",
        "json",
    ])
    .arg(input);

    let out = run_cancellable_output(cmd, Duration::from_secs(30), cancel, "ffprobe video stream")
        .await?;
    if !out.status.success() {
        bail!(
            "Failed to probe video stream in {}: {}",
            input.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }

    let probe: FfprobeVideoProbe =
        serde_json::from_slice(&out.stdout).context("Failed to parse ffprobe video JSON")?;
    let streams = probe.streams.unwrap_or_default();
    let stream = streams
        .iter()
        .find(|s| s.width.unwrap_or(0) > 0 && s.height.unwrap_or(0) > 0)
        .with_context(|| format!("No video stream found in {}", input.display()))?;

    let parse_dur_ms = |raw: Option<&str>| -> Option<u64> {
        let secs = raw?.trim().parse::<f64>().ok()?;
        if secs.is_finite() && secs > 0.0 {
            Some((secs * 1000.0).round() as u64)
        } else {
            None
        }
    };

    let format_info = probe.format.as_ref();
    let duration_ms = parse_dur_ms(format_info.and_then(|f| f.duration.as_deref()))
        .or_else(|| parse_dur_ms(stream.duration.as_deref()))
        .unwrap_or(0);
    let start_offset_ms = compute_stream_start_offset_ms(
        format_info.and_then(|f| f.start_time.as_deref()),
        stream.start_time.as_deref(),
    );

    Ok(ProbedVideoInfo {
        duration_ms,
        start_offset_ms,
    })
}

/// Extracts a single PNG preview frame from `input` at `time_ms` (or ~10% into the video, capped at 30s).
pub async fn extract_video_preview_frame(
    ffmpeg: &str,
    ffprobe: &str,
    input: &Path,
    time_ms: Option<u64>,
    cancel: Option<&CancellationToken>,
) -> Result<VideoFramePreview> {
    if cancel.is_some_and(CancellationToken::is_cancelled) {
        bail!("Video preview extraction cancelled");
    }

    let probed = probe_video_stream(ffprobe, input, cancel).await?;
    let default_ms = (probed.duration_ms / 10).min(30_000);
    let target_ms = if probed.duration_ms > 0 {
        time_ms
            .unwrap_or(default_ms)
            .min(probed.duration_ms.saturating_sub(100))
    } else {
        time_ms.unwrap_or(0)
    };
    let seek_sec = format!("{:.3}", target_ms as f64 / 1000.0);

    let mut cmd = tokio::process::Command::new(ffmpeg);
    cmd.args(["-nostdin", "-v", "error", "-ss", &seek_sec, "-i"])
        .arg(input)
        .args([
            "-frames:v",
            "1",
            "-vf",
            "scale='min(1280,iw*sar)':trunc(ow/dar/2)*2,setsar=1",
            "-f",
            "image2pipe",
            "-vcodec",
            "png",
            "-",
        ]);

    let out = run_cancellable_output(
        cmd,
        Duration::from_secs(30),
        cancel,
        "ffmpeg video preview frame",
    )
    .await?;
    if !out.status.success() {
        bail!(
            "Failed to extract video preview frame: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    anyhow::ensure!(
        !out.stdout.is_empty(),
        "FFmpeg produced an empty preview frame for {}",
        input.display()
    );

    let decoder = png::Decoder::new(Cursor::new(&out.stdout));
    let reader = decoder
        .read_info()
        .context("Failed to read preview PNG header")?;
    let info = reader.info();
    let width = info.width;
    let height = info.height;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&out.stdout);
    Ok(VideoFramePreview {
        data_url: format!("data:image/png;base64,{b64}"),
        width,
        height,
        duration_ms: probed.duration_ms,
        time_ms: target_ms,
    })
}

#[derive(Debug, Clone)]
struct FrameSignature {
    active_cells: [bool; GRID_CELLS],
    sharpness_score: u64,
    width: u32,
    height: u32,
}

fn grid_iou(a: &FrameSignature, b: &FrameSignature) -> f64 {
    let mut inter = 0usize;
    let mut union = 0usize;
    for (&ca, &cb) in a.active_cells.iter().zip(b.active_cells.iter()) {
        if ca && cb {
            inter += 1;
        }
        if ca || cb {
            union += 1;
        }
    }
    if union == 0 {
        1.0
    } else {
        inter as f64 / union as f64
    }
}

fn active_cell_norm_dist(a: &FrameSignature, b: &FrameSignature) -> f64 {
    let diff = a
        .active_cells
        .iter()
        .zip(b.active_cells.iter())
        .filter(|&(ca, cb)| ca != cb)
        .count();
    diff as f64 / GRID_CELLS as f64
}

fn median_i32(mut vals: Vec<i32>) -> i32 {
    if vals.is_empty() {
        return 0;
    }
    let mid = vals.len() / 2;
    let (_, med, _) = vals.select_nth_unstable(mid);
    *med
}

fn analyze_roi_frame(png_bytes: &[u8]) -> Result<Option<FrameSignature>> {
    let (rgba, w_u32, h_u32) = decode_png_to_rgba(png_bytes)?;
    let width = w_u32 as usize;
    let height = h_u32 as usize;
    if width < 4 || height < 4 {
        return Ok(None);
    }

    let total_pixels = width * height;
    let mut luma = Vec::with_capacity(total_pixels);
    for px in rgba.as_chunks::<4>().0 {
        let y = (299 * i32::from(px[0]) + 587 * i32::from(px[1]) + 114 * i32::from(px[2])) / 1000;
        luma.push(y);
    }

    let mut border = Vec::with_capacity(width * 2 + height * 2);
    for x in 0..width {
        border.push(luma[x]);
        border.push(luma[(height - 1) * width + x]);
    }
    for y in 1..height.saturating_sub(1) {
        border.push(luma[y * width]);
        border.push(luma[y * width + (width - 1)]);
    }
    let border_luma = median_i32(border);

    let mut cell_fg = [0usize; GRID_CELLS];
    let mut fg_count = 0usize;
    let mut sharpness_score = 0u64;

    for y in 0..height {
        let y0 = y.saturating_sub(1);
        let y1 = (y + 1).min(height - 1);
        let gy = (y * GRID_ROWS) / height;

        for x in 0..width {
            let x0 = x.saturating_sub(1);
            let x1 = (x + 1).min(width - 1);
            let center = luma[y * width + x];
            let luma_diff = (center - border_luma).abs();
            if luma_diff <= 42 {
                continue;
            }

            let mut min_nb = center;
            let mut max_nb = center;
            for ny in y0..=y1 {
                let row = &luma[ny * width..ny * width + width];
                for &val in &row[x0..=x1] {
                    min_nb = min_nb.min(val);
                    max_nb = max_nb.max(val);
                }
            }
            let local_contrast = max_nb - min_nb;
            if local_contrast > 32 || luma_diff > 75 {
                fg_count += 1;
                sharpness_score += u64::try_from(local_contrast).unwrap_or(0);
                let gx = (x * GRID_COLS) / width;
                let cell_idx = gy * GRID_COLS + gx;
                if let Some(cell) = cell_fg.get_mut(cell_idx) {
                    *cell += 1;
                }
            }
        }
    }

    let fg_ratio = fg_count as f64 / total_pixels as f64;
    if !(0.002..=0.80).contains(&fg_ratio) {
        return Ok(None);
    }

    let mut active_cells = [false; GRID_CELLS];
    let mut active_count = 0usize;
    for (dst, &count) in active_cells.iter_mut().zip(cell_fg.iter()) {
        if count > 0 {
            *dst = true;
            active_count += 1;
        }
    }

    if active_count < 2 {
        return Ok(None);
    }

    Ok(Some(FrameSignature {
        active_cells,
        sharpness_score,
        width: w_u32,
        height: h_u32,
    }))
}

struct OpenHardcodedCue {
    start_ms: u64,
    end_ms: u64,
    best_png_bytes: Vec<u8>,
    best_width: u32,
    best_height: u32,
    best_score: u64,
    last_png_bytes: Vec<u8>,
    last_sig: FrameSignature,
}

impl OpenHardcodedCue {
    fn from_frame(t_ms: u64, step_ms: u64, png_bytes: Vec<u8>, sig: FrameSignature) -> Self {
        Self {
            start_ms: t_ms,
            end_ms: t_ms + step_ms,
            best_png_bytes: png_bytes.clone(),
            best_width: sig.width,
            best_height: sig.height,
            best_score: sig.sharpness_score,
            last_png_bytes: png_bytes,
            last_sig: sig,
        }
    }

    fn matches(&self, png_bytes: &[u8], sig: &FrameSignature) -> bool {
        if self.last_png_bytes == png_bytes {
            return true;
        }
        let iou = grid_iou(&self.last_sig, sig);
        let dist = active_cell_norm_dist(&self.last_sig, sig);
        iou >= 0.55 && dist <= 0.14
    }

    fn merge(&mut self, t_ms: u64, step_ms: u64, png_bytes: Vec<u8>, sig: FrameSignature) {
        self.end_ms = t_ms + step_ms;
        if sig.sharpness_score >= self.best_score {
            self.best_score = sig.sharpness_score;
            self.best_png_bytes = png_bytes.clone();
            self.best_width = sig.width;
            self.best_height = sig.height;
        }
        self.last_png_bytes = png_bytes;
        self.last_sig = sig;
    }

    fn into_cue(self, id: u32, clamp_end_ms: Option<u64>) -> BitmapSubtitleCue {
        let mut end_ms = self.end_ms;
        if let Some(cap) = clamp_end_ms {
            end_ms = end_ms.min(cap);
        }
        end_ms = end_ms.max(self.start_ms + 1);
        BitmapSubtitleCue {
            id,
            start_ms: self.start_ms,
            end_ms,
            png_bytes: self.best_png_bytes,
            width: self.best_width,
            height: self.best_height,
        }
    }
}

/// Attempts to pop one complete PNG frame (`\x89PNG...IEND`) from `buf`, updating `scan_pos`.
fn pop_complete_png(buf: &mut Vec<u8>, scan_pos: &mut usize) -> Result<Option<Vec<u8>>> {
    if buf.len() < PNG_SIGNATURE.len() {
        return Ok(None);
    }
    if !buf.starts_with(PNG_SIGNATURE) {
        let Some(sig_idx) = buf
            .windows(PNG_SIGNATURE.len())
            .position(|w| w == PNG_SIGNATURE)
        else {
            buf.clear();
            *scan_pos = 0;
            return Ok(None);
        };
        buf.drain(..sig_idx);
        *scan_pos = PNG_SIGNATURE.len();
    } else if *scan_pos < PNG_SIGNATURE.len() {
        *scan_pos = PNG_SIGNATURE.len();
    }

    while *scan_pos + 12 <= buf.len() {
        let len_bytes = [
            buf[*scan_pos],
            buf[*scan_pos + 1],
            buf[*scan_pos + 2],
            buf[*scan_pos + 3],
        ];
        let chunk_len = u32::from_be_bytes(len_bytes) as usize;
        let chunk_type = [
            buf[*scan_pos + 4],
            buf[*scan_pos + 5],
            buf[*scan_pos + 6],
            buf[*scan_pos + 7],
        ];
        let chunk_total = 12usize
            .checked_add(chunk_len)
            .context("PNG chunk size overflow")?;
        if *scan_pos + chunk_total > buf.len() {
            return Ok(None);
        }
        *scan_pos += chunk_total;
        if &chunk_type == b"IEND" {
            let png = buf.drain(..*scan_pos).collect();
            *scan_pos = 0;
            return Ok(Some(png));
        }
    }

    Ok(None)
}

/// Samples burned-in subtitle ROI frames via FFmpeg and emits deduplicated [`BitmapSubtitleCue`] intervals.
pub async fn extract_hardcoded_subtitles(
    ffmpeg: &str,
    ffprobe: &str,
    input: &Path,
    roi: VideoRoi,
    sample_fps: f64,
    limit: Option<usize>,
    cancel: Option<&CancellationToken>,
) -> Result<Vec<BitmapSubtitleCue>> {
    if cancel.is_some_and(CancellationToken::is_cancelled) {
        bail!("Hardcoded subtitle extraction cancelled");
    }
    let roi = roi.clamped()?;
    anyhow::ensure!(
        sample_fps.is_finite() && (0.5..=15.0).contains(&sample_fps),
        "sample_fps must be finite and within 0.5..=15.0 (got {sample_fps})"
    );

    let probed = probe_video_stream(ffprobe, input, cancel).await?;
    if limit == Some(0) {
        return Ok(Vec::new());
    }

    let filter = format!(
        "setpts=PTS-STARTPTS,crop=max(16\\,iw*{w:.6}):max(16\\,ih*{h:.6}):iw*{x:.6}:ih*{y:.6},fps={fps:.3},scale='min(960,iw)':-2",
        w = roi.width,
        h = roi.height,
        x = roi.x,
        y = roi.y,
        fps = sample_fps
    );

    let mut cmd = tokio::process::Command::new(ffmpeg);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_4000);
    cmd.kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(["-nostdin", "-v", "error", "-i"])
        .arg(input)
        .args(["-vf", &filter, "-f", "image2pipe", "-vcodec", "png", "-"]);

    let mut attempts = 0;
    let mut child = loop {
        match cmd.spawn() {
            Err(e) if e.raw_os_error() == Some(26) && attempts < 10 => {
                attempts += 1;
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            res => {
                break res.context("Failed to spawn ffmpeg for hardcoded subtitle extraction")?;
            }
        }
    };
    let mut stdout = child
        .stdout
        .take()
        .context("FFmpeg child stdout pipe missing")?;
    let stderr_pipe = child.stderr.take();
    let stderr_task = tokio::spawn(async move {
        let mut buf = Vec::new();
        if let Some(mut err) = stderr_pipe {
            let _ = err.read_to_end(&mut buf).await;
        }
        String::from_utf8_lossy(&buf).into_owned()
    });

    let step_ms = (1000.0 / sample_fps).round() as u64;
    let mut buf = Vec::with_capacity(64 * 1024);
    let mut scan_pos = 0usize;
    let mut chunk = [0u8; 32 * 1024];
    let mut frame_idx: u64 = 0;
    let mut cues: Vec<BitmapSubtitleCue> = Vec::new();
    let mut open_cue: Option<OpenHardcodedCue> = None;
    let mut hit_limit = false;

    loop {
        if cancel.is_some_and(CancellationToken::is_cancelled) {
            let _ = child.kill().await;
            let _ = child.wait().await;
            stderr_task.abort();
            bail!("Hardcoded subtitle extraction cancelled");
        }

        let n = if let Some(token) = cancel {
            tokio::select! {
                biased;
                _ = token.cancelled() => {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    stderr_task.abort();
                    bail!("Hardcoded subtitle extraction cancelled");
                }
                res = stdout.read(&mut chunk) => res.context("Failed reading ffmpeg PNG stream")?,
            }
        } else {
            stdout
                .read(&mut chunk)
                .await
                .context("Failed reading ffmpeg PNG stream")?
        };

        if n > 0 {
            buf.extend_from_slice(&chunk[..n]);
        }

        while let Some(png_bytes) = pop_complete_png(&mut buf, &mut scan_pos)? {
            let t_ms =
                probed.start_offset_ms + ((frame_idx as f64 / sample_fps) * 1000.0).round() as u64;
            frame_idx += 1;

            let (png_bytes, maybe_sig) = tokio::task::spawn_blocking(move || {
                let sig = analyze_roi_frame(&png_bytes)?;
                anyhow::Ok((png_bytes, sig))
            })
            .await
            .context("Hardcoded subtitle frame analysis task failed")??;
            match (open_cue.take(), maybe_sig) {
                (None, None) => {}
                (None, Some(sig)) => {
                    open_cue = Some(OpenHardcodedCue::from_frame(t_ms, step_ms, png_bytes, sig));
                }
                (Some(mut open), Some(sig)) => {
                    if open.matches(&png_bytes, &sig) {
                        open.merge(t_ms, step_ms, png_bytes, sig);
                        open_cue = Some(open);
                    } else {
                        let id = u32::try_from(cues.len() + 1).context("Cue count overflow")?;
                        cues.push(open.into_cue(id, Some(t_ms)));
                        if limit.is_some_and(|max| cues.len() >= max) {
                            hit_limit = true;
                            break;
                        }
                        open_cue =
                            Some(OpenHardcodedCue::from_frame(t_ms, step_ms, png_bytes, sig));
                    }
                }
                (Some(open), None) => {
                    let id = u32::try_from(cues.len() + 1).context("Cue count overflow")?;
                    cues.push(open.into_cue(id, Some(t_ms)));
                    if limit.is_some_and(|max| cues.len() >= max) {
                        hit_limit = true;
                        break;
                    }
                }
            }
        }

        if hit_limit {
            let _ = child.kill().await;
            break;
        }
        if n == 0 {
            break;
        }
    }

    if !hit_limit && let Some(open) = open_cue.take() {
        let id = u32::try_from(cues.len() + 1).context("Cue count overflow")?;
        cues.push(open.into_cue(id, None));
    }

    if hit_limit {
        let _ = child.wait().await;
        stderr_task.abort();
    } else {
        let status = child
            .wait()
            .await
            .context("Failed waiting for ffmpeg hardcoded subtitle process")?;
        let stderr_text = stderr_task.await.unwrap_or_default();
        if !status.success() {
            bail!(
                "FFmpeg hardcoded subtitle extraction failed: {}",
                stderr_text.trim()
            );
        }
    }

    Ok(cues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn video_roi_validate_and_clamp() {
        for bad in [
            VideoRoi {
                x: -0.01,
                y: 0.7,
                width: 0.5,
                height: 0.2,
            },
            VideoRoi {
                x: 0.1,
                y: -0.1,
                width: 0.5,
                height: 0.2,
            },
            VideoRoi {
                x: 0.1,
                y: 0.7,
                width: 0.01,
                height: 0.2,
            },
            VideoRoi {
                x: 0.1,
                y: 0.7,
                width: 0.5,
                height: 0.019,
            },
            VideoRoi {
                x: 0.5,
                y: 0.7,
                width: 0.55,
                height: 0.2,
            },
            VideoRoi {
                x: 0.1,
                y: 0.85,
                width: 0.5,
                height: 0.2,
            },
            VideoRoi {
                x: f64::NAN,
                y: 0.7,
                width: 0.5,
                height: 0.2,
            },
            VideoRoi {
                x: 0.1,
                y: 0.7,
                width: f64::INFINITY,
                height: 0.2,
            },
        ] {
            assert!(bad.validate().is_err(), "expected invalid: {bad:?}");
        }

        let valid = VideoRoi {
            x: 0.1,
            y: 0.7,
            width: 0.9005,
            height: 0.3005,
        };
        let clamped = valid.clamped().unwrap();
        assert!((clamped.x + clamped.width - 1.0).abs() < 1e-9);
        assert!((clamped.y + clamped.height - 1.0).abs() < 1e-9);
    }

    fn create_synthetic_burned_in_mkv(dir: &Path) -> PathBuf {
        let mkv_path = dir.join("burned_in.mkv");
        let vf = concat!(
            "drawbox=x=80:y=270:w=220:h=36:color=white@1.0:t=fill:enable='between(t,0.4,1.4)',",
            "drawbox=x=320:y=270:w=220:h=36:color=white@1.0:t=fill:enable='between(t,2.0,3.0)'"
        );
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-nostdin",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=640x360:d=3.5:r=10",
                "-vf",
                vf,
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-y",
            ])
            .arg(&mkv_path)
            .status()
            .expect("ffmpeg required for hardcoded subtitle integration test");
        assert!(status.success());
        mkv_path
    }

    #[tokio::test]
    async fn extracts_preview_frame_and_deduplicated_hardcoded_cues() {
        let dir = tempfile::tempdir().unwrap();
        let mkv = create_synthetic_burned_in_mkv(dir.path());

        let preview = extract_video_preview_frame("ffmpeg", "ffprobe", &mkv, Some(500), None)
            .await
            .unwrap();
        assert!(preview.data_url.starts_with("data:image/png;base64,"));
        assert_eq!(preview.width, 640);
        assert_eq!(preview.height, 360);
        assert_eq!(preview.time_ms, 500);
        assert!((3300..=3700).contains(&preview.duration_ms));

        let roi = VideoRoi {
            x: 0.05,
            y: 0.65,
            width: 0.90,
            height: 0.30,
        };
        let cues = extract_hardcoded_subtitles("ffmpeg", "ffprobe", &mkv, roi, 5.0, None, None)
            .await
            .unwrap();
        assert_eq!(cues.len(), 2, "expected 2 cues, got {cues:?}");

        assert_eq!(cues[0].id, 1);
        assert!(
            (350..=550).contains(&cues[0].start_ms),
            "cue0 start={}",
            cues[0].start_ms
        );
        assert!(
            (1250..=1650).contains(&cues[0].end_ms),
            "cue0 end={}",
            cues[0].end_ms
        );

        assert_eq!(cues[1].id, 2);
        assert!(
            (1950..=2250).contains(&cues[1].start_ms),
            "cue1 start={}",
            cues[1].start_ms
        );
        assert!(
            (2850..=3250).contains(&cues[1].end_ms),
            "cue1 end={}",
            cues[1].end_ms
        );

        let limited =
            extract_hardcoded_subtitles("ffmpeg", "ffprobe", &mkv, roi, 5.0, Some(1), None)
                .await
                .unwrap();
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].id, 1);

        let cancel = CancellationToken::new();
        cancel.cancel();
        let err =
            extract_hardcoded_subtitles("ffmpeg", "ffprobe", &mkv, roi, 5.0, None, Some(&cancel))
                .await
                .unwrap_err();
        assert!(err.to_string().to_lowercase().contains("cancel"));
    }

    #[test]
    fn compute_stream_start_offset_ms_handles_relative_and_invalid_values() {
        assert_eq!(
            compute_stream_start_offset_ms(Some("1.000"), Some("3.250")),
            2250
        );
        assert_eq!(
            compute_stream_start_offset_ms(Some("5.000"), Some("5.000")),
            0
        );
        assert_eq!(
            compute_stream_start_offset_ms(Some("5.000"), Some("2.000")),
            0
        );
        assert_eq!(compute_stream_start_offset_ms(None, Some("1.500")), 1500);
        assert_eq!(compute_stream_start_offset_ms(Some("bad"), Some("NaN")), 0);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn extract_hardcoded_subtitles_applies_setpts_and_stream_start_offset() {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;

        let dir = tempfile::tempdir().unwrap();
        let encode_solid_png = |luma: u8| -> Vec<u8> {
            let mut rgba = vec![16u8; 32 * 8 * 4];
            for px in rgba.as_chunks_mut::<4>().0 {
                px[3] = 255;
            }
            if luma > 100 {
                for y in 2..6 {
                    for x in 4..28 {
                        let off = (y * 32 + x) * 4;
                        rgba[off..off + 4].copy_from_slice(&[luma, luma, luma, 255]);
                    }
                }
            }
            let mut out = Vec::new();
            let mut encoder = png::Encoder::new(&mut out, 32, 8);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&rgba)
                .unwrap();
            out
        };

        let mut stream_bytes = Vec::new();
        stream_bytes.extend_from_slice(&encode_solid_png(16)); // t = offset + 0ms (dark)
        stream_bytes.extend_from_slice(&encode_solid_png(240)); // t = offset + 500ms (bright cue)
        stream_bytes.extend_from_slice(&encode_solid_png(16)); // t = offset + 1000ms (dark close)

        let png_stream_path = dir.path().join("frames.pngs");
        std::fs::write(&png_stream_path, &stream_bytes).unwrap();
        let probe_path = dir.path().join("probe.json");
        std::fs::write(
            &probe_path,
            br#"{"streams":[{"width":320,"height":240,"duration":"2.0","start_time":"5.500"}],"format":{"duration":"2.0","start_time":"1.000"}}"#,
        )
        .unwrap();
        let args_log = dir.path().join("ffmpeg.args");

        let ffprobe_script = dir.path().join("mock-ffprobe");
        let ffmpeg_script = dir.path().join("mock-ffmpeg");
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o700)
            .open(&ffprobe_script)
            .unwrap()
            .write_all(format!("#!/bin/sh\ncat '{}'\n", probe_path.display()).as_bytes())
            .unwrap();
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o700)
            .open(&ffmpeg_script)
            .unwrap()
            .write_all(
                format!(
                    "#!/bin/sh\nprintf '%s\\n' \"$*\" > '{}'\ncat '{}'\n",
                    args_log.display(),
                    png_stream_path.display()
                )
                .as_bytes(),
            )
            .unwrap();

        let video = dir.path().join("offset.mkv");
        std::fs::write(&video, b"stub").unwrap();
        let roi = VideoRoi {
            x: 0.1,
            y: 0.7,
            width: 0.8,
            height: 0.2,
        };
        let cues = extract_hardcoded_subtitles(
            ffmpeg_script.to_str().unwrap(),
            ffprobe_script.to_str().unwrap(),
            &video,
            roi,
            2.0,
            None,
            None,
        )
        .await
        .unwrap();

        assert_eq!(cues.len(), 1);
        assert_eq!(cues[0].start_ms, 5000);
        assert_eq!(cues[0].end_ms, 5500);
        let logged = std::fs::read_to_string(&args_log).unwrap();
        assert!(logged.contains("setpts=PTS-STARTPTS,"));
    }
}
