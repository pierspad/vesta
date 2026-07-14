//! # srt-autosync
//!
//! GUI-agnostic engine for automatic subtitle-to-audio alignment.
//!
//! The algorithm (the same one behind Vesta's "Auto Sync" button):
//!
//! 1. sample N strategic positions across the media file (12×20s in quick
//!    mode, 24×40s in precise mode);
//! 2. extract each position to 16 kHz mono WAV via FFmpeg, skipping silent
//!    windows (retrying at shifted positions);
//! 3. transcribe every segment with Whisper (word timestamps enabled);
//! 4. fuzzy-match transcribed text against the subtitle lines near that
//!    time (±45 s window, temporal weighting);
//! 5. estimate the dominant global offset by density (matches whose offset
//!    disagrees with the consensus are discarded — "geometric verification");
//! 6. keep the best match per subtitle and space anchors ≥ 30 s apart.
//!
//! The output is a list of [`AnchorSuggestion`]s: the caller decides how to
//! apply them (e.g. feed them to `srt_sync::SyncEngine::add_anchor`). Progress
//! is reported through a plain callback and cancellation through a
//! [`CancellationToken`] — no UI framework involved.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use serde::Serialize;
use tokio_util::sync::CancellationToken;

use whisper_common::audio::read_wav_to_f32;
use whisper_common::transcribe::{
    text_similarity, transcribe_full, TranscribeOptions, TranscribedSegment,
};

// ─── Public types ────────────────────────────────────────────────────────────

/// A subtitle line to align against (id + original start time + text).
#[derive(Debug, Clone)]
pub struct SubtitleLine {
    pub id: u32,
    pub start_ms: i64,
    pub text: String,
}

/// Configuration for an auto-sync run.
#[derive(Debug, Clone)]
pub struct AutoSyncConfig {
    /// Media file (video or audio) the subtitles should be aligned to.
    pub media_path: String,
    /// Path to a downloaded ggml Whisper model
    /// (see `whisper_common::model::model_file_path`).
    pub model_path: PathBuf,
    /// Spoken language hint (None = autodetect).
    pub language: Option<String>,
    /// Quick mode: fewer, shorter samples (12×20 s instead of 24×40 s).
    pub quick: bool,
    /// ffmpeg executable (path or command on PATH).
    pub ffmpeg_cmd: String,
    /// ffprobe executable (path or command on PATH).
    pub ffprobe_cmd: String,
}

/// A suggested anchor point: subtitle `subtitle_id`, whose line originally
/// starts at `original_start_ms`, actually spoken at `corrected_time_ms`.
#[derive(Debug, Clone, Serialize)]
pub struct AnchorSuggestion {
    pub subtitle_id: u32,
    pub original_start_ms: i64,
    pub corrected_time_ms: i64,
    pub similarity: f64,
    pub score: f64,
}

/// Result of an auto-sync run.
#[derive(Debug, Clone)]
pub struct AutoSyncOutcome {
    pub suggestions: Vec<AnchorSuggestion>,
    pub segments_analyzed: usize,
    pub cancelled: bool,
}

/// Progress update. `message_key`/`params` carry a stable identifier and its
/// interpolation parameters so UI layers can localize messages.
#[derive(Debug, Clone, Serialize)]
pub struct AutoSyncProgress {
    pub stage: String,
    pub message: String,
    pub percentage: f64,
    pub message_key: Option<String>,
    pub params: Option<HashMap<String, String>>,
}

/// Callback invoked with progress updates.
pub type ProgressCallback = Arc<dyn Fn(AutoSyncProgress) + Send + Sync>;

// ─── Internals ───────────────────────────────────────────────────────────────

/// A candidate match between a transcribed segment and an SRT subtitle.
#[derive(Debug, Clone)]
struct MatchCandidate {
    subtitle_id: u32,
    original_start_ms: i64,
    transcribed_start_ms: i64,
    similarity: f64,
    score: f64,
}

/// Extract a short audio segment from the media file using FFmpeg.
fn extract_audio_segment(
    media_path: &str,
    start_sec: f64,
    duration_sec: f64,
    output_wav: &str,
    ffmpeg_cmd: &str,
) -> Result<()> {
    let output = std::process::Command::new(ffmpeg_cmd)
        .args([
            "-y",
            "-ss", &format!("{start_sec:.2}"),
            "-i", media_path,
            "-t", &format!("{duration_sec:.2}"),
            "-ar", "16000",
            "-ac", "1",
            "-c:a", "pcm_s16le",
            output_wav,
        ])
        .output()
        .context("Failed to run ffmpeg for audio segment extraction")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("FFmpeg audio extraction failed: {stderr}");
    }

    Ok(())
}

/// True when the RMS of the samples is under `threshold`.
fn is_silent(samples: &[f32], threshold: f32) -> bool {
    if samples.is_empty() {
        return true;
    }
    let sum_sq: f32 = samples.iter().map(|&x| x * x).sum();
    let rms = (sum_sq / samples.len() as f32).sqrt();
    rms < threshold
}

fn num_cpus() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
}

fn format_mm_ss(total_seconds: f64) -> String {
    let clamped = total_seconds.max(0.0).round() as i64;
    format!("{:02}:{:02}", clamped / 60, clamped % 60)
}

/// Penalize weak temporal alignments so text matches far away from their
/// expected subtitle region are less likely to become anchors.
fn temporal_weight(time_diff_ms: i64) -> f64 {
    if time_diff_ms <= 8_000 {
        return 1.0;
    }
    if time_diff_ms >= 45_000 {
        return 0.65;
    }
    let normalized = (time_diff_ms - 8_000) as f64 / (45_000 - 8_000) as f64;
    1.0 - (normalized * 0.35)
}

/// Get media duration in seconds using ffprobe.
pub async fn get_media_duration(media_path: &str, ffprobe_cmd: &str) -> Result<f64> {
    let output = tokio::process::Command::new(ffprobe_cmd)
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "default=noprint_wrappers=1:nokey=1",
            media_path,
        ])
        .output()
        .await
        .context("Failed to run ffprobe")?;

    if !output.status.success() {
        anyhow::bail!("ffprobe failed");
    }

    let duration_str = String::from_utf8_lossy(&output.stdout);
    duration_str
        .trim()
        .parse::<f64>()
        .context("Failed to parse duration from ffprobe")
}

struct PreparedSegment {
    idx: usize,
    current_pos: f64,
    audio_data: Vec<f32>,
    _wav_path: PathBuf,
}

// Worker for a single segment of the auto-sync fan-out: the parameters are
// independent (positions, durations, paths, ffmpeg command) and the function
// is internal, so grouping them in a struct would add ceremony over clarity.
#[allow(clippy::too_many_arguments)]
async fn prepare_single_segment(
    idx: usize,
    start_pos: f64,
    segment_duration: f64,
    duration_sec: f64,
    quick: bool,
    media_path: String,
    temp_dir_path: PathBuf,
    ffmpeg_cmd: String,
) -> Result<PreparedSegment, String> {
    let mut current_pos = start_pos;
    let mut attempts = 0;
    let mut audio_data = Vec::new();
    let mut wav_path = temp_dir_path.join(format!("segment_{idx}.wav"));

    let max_attempts = if quick { 5 } else { 3 };
    let shift_amount = if quick { 15.0 } else { 20.0 };

    while attempts < max_attempts && current_pos + segment_duration <= duration_sec {
        let temp_wav_path = temp_dir_path.join(format!("segment_{idx}_try{attempts}.wav"));
        let temp_wav_str = temp_wav_path.to_string_lossy().to_string();

        let media_path_clone = media_path.clone();
        let ffmpeg_cmd_clone = ffmpeg_cmd.clone();
        let extract_res = tokio::task::spawn_blocking(move || {
            extract_audio_segment(
                &media_path_clone,
                current_pos,
                segment_duration,
                &temp_wav_str,
                &ffmpeg_cmd_clone,
            )
        })
        .await;

        let extract_ok = matches!(extract_res, Ok(Ok(())));

        if !extract_ok {
            attempts += 1;
            current_pos += shift_amount;
            continue;
        }

        let temp_wav_path_clone = temp_wav_path.clone();
        let samples_res =
            tokio::task::spawn_blocking(move || read_wav_to_f32(&temp_wav_path_clone)).await;

        let samples = match samples_res {
            Ok(Ok(data)) => data,
            _ => {
                let _ = std::fs::remove_file(&temp_wav_path);
                attempts += 1;
                current_pos += shift_amount;
                continue;
            }
        };

        if is_silent(&samples, 0.003) {
            let _ = std::fs::remove_file(&temp_wav_path);
            attempts += 1;
            current_pos += shift_amount;
        } else {
            audio_data = samples;
            wav_path = temp_wav_path;
            break;
        }
    }

    if audio_data.is_empty() {
        let wav_str = wav_path.to_string_lossy().to_string();
        let media_path_clone = media_path.clone();
        let ffmpeg_cmd_clone = ffmpeg_cmd.clone();
        let _ = tokio::task::spawn_blocking(move || {
            extract_audio_segment(
                &media_path_clone,
                start_pos,
                segment_duration,
                &wav_str,
                &ffmpeg_cmd_clone,
            )
        })
        .await;

        let wav_path_clone = wav_path.clone();
        if let Ok(Ok(samples)) =
            tokio::task::spawn_blocking(move || read_wav_to_f32(&wav_path_clone)).await
        {
            audio_data = samples;
            current_pos = start_pos;
        }
    }

    Ok(PreparedSegment { idx, current_pos, audio_data, _wav_path: wav_path })
}

fn emit(
    on_progress: &Option<ProgressCallback>,
    stage: &str,
    message: String,
    percentage: f64,
    message_key: Option<&str>,
    params: Option<HashMap<String, String>>,
) {
    if let Some(cb) = on_progress {
        cb(AutoSyncProgress {
            stage: stage.to_string(),
            message,
            percentage,
            message_key: message_key.map(str::to_string),
            params,
        });
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

/// Run the full auto-sync analysis and return anchor suggestions.
///
/// The caller applies the suggestions however it sees fit — typically by
/// filtering out those too close to already-existing anchors and feeding the
/// rest to `srt_sync::SyncEngine::add_anchor(id, corrected_time_ms, false)`.
pub async fn run_auto_sync(
    config: &AutoSyncConfig,
    subtitles: Vec<SubtitleLine>,
    on_progress: Option<ProgressCallback>,
    cancel_token: &CancellationToken,
) -> Result<AutoSyncOutcome> {
    if !config.model_path.exists() {
        anyhow::bail!("Whisper model not found at {}", config.model_path.display());
    }

    let ffmpeg_ok = tokio::process::Command::new(&config.ffmpeg_cmd)
        .arg("-version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ffmpeg_ok {
        anyhow::bail!("FFmpeg is required for auto-sync. Install FFmpeg first.");
    }

    let duration_sec = get_media_duration(&config.media_path, &config.ffprobe_cmd)
        .await
        .unwrap_or(0.0);
    if duration_sec < 10.0 {
        anyhow::bail!("Media file too short or unable to detect duration");
    }

    // Segment duration and sample positions (quick = brief, !quick = precise).
    let quick = config.quick;
    let segment_duration = if quick { 20.0 } else { 40.0 };
    let num_samples = if quick { 12 } else { 24 };
    let mut sample_positions: Vec<f64> = Vec::new();

    let step = duration_sec / (num_samples + 1) as f64;
    for i in 1..=num_samples {
        let pos = step * i as f64;
        if pos + segment_duration <= duration_sec {
            sample_positions.push(pos);
        }
    }
    if sample_positions.is_empty() {
        sample_positions.push(0.0);
    }

    let total_segments = sample_positions.len();

    emit(
        &on_progress,
        "start",
        format!("Preparing auto-sync: {total_segments} audio segments to analyze..."),
        0.0,
        Some("sync.autoSyncProgress.analyzingSegments"),
        Some(HashMap::from([("total".to_string(), total_segments.to_string())])),
    );

    let temp_dir = tempfile::tempdir().context("Failed to create temp dir")?;
    let temp_dir_path = temp_dir.path().to_path_buf();

    // Concurrently prepare all segments using a CPU-bounded semaphore.
    emit(
        &on_progress,
        "prepare",
        format!("Preparing and extracting {total_segments} audio segments in parallel..."),
        3.0,
        Some("sync.autoSyncProgress.preparingSegments"),
        Some(HashMap::from([("total".to_string(), total_segments.to_string())])),
    );

    let max_concurrency = num_cpus().min(6);
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrency));
    let mut prep_handles = Vec::new();

    for (idx, &start_pos) in sample_positions.iter().enumerate() {
        let sem = semaphore.clone();
        let media_path = config.media_path.clone();
        let temp_dir_path = temp_dir_path.clone();
        let ffmpeg_cmd = config.ffmpeg_cmd.clone();

        prep_handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            prepare_single_segment(
                idx,
                start_pos,
                segment_duration,
                duration_sec,
                quick,
                media_path,
                temp_dir_path,
                ffmpeg_cmd,
            )
            .await
        }));
    }

    let mut prepared_segments = Vec::new();
    for (idx, handle) in prep_handles.into_iter().enumerate() {
        if cancel_token.is_cancelled() {
            return Ok(AutoSyncOutcome {
                suggestions: Vec::new(),
                segments_analyzed: idx,
                cancelled: true,
            });
        }
        match handle.await {
            Ok(Ok(prep)) => prepared_segments.push(prep),
            Ok(Err(e)) => eprintln!("[auto-sync] Segment {idx} preparation failed: {e}"),
            Err(e) => eprintln!("[auto-sync] Segment {idx} task panicked: {e:?}"),
        }
    }

    // Sort chronologically to preserve geometric alignment sequence.
    prepared_segments.sort_by_key(|s| s.idx);

    emit(
        &on_progress,
        "prepare_done",
        "Audio preparation complete. Loading Whisper model...".to_string(),
        10.0,
        Some("sync.autoSyncProgress.loadingModel"),
        None,
    );

    let model_path_str = config.model_path.to_string_lossy().to_string();
    let language = config.language.clone();
    let token_clone = cancel_token.clone();
    let progress_clone = on_progress.clone();

    let spawn_res = tokio::task::spawn_blocking(
        move || -> Result<(Vec<MatchCandidate>, usize, bool), String> {
            let mut all_matches: Vec<MatchCandidate> = Vec::new();

            let ctx = whisper_rs::WhisperContext::new_with_params(
                &model_path_str,
                whisper_rs::WhisperContextParameters::default(),
            )
            .map_err(|e| format!("Failed to load Whisper model: {e:?}"))?;

            let mut subtitles_sorted: Vec<(u32, i64, String)> =
                subtitles.into_iter().map(|s| (s.id, s.start_ms, s.text)).collect();
            subtitles_sorted.sort_by_key(|(_, start_ms, _)| *start_ms);

            for (idx, prep) in prepared_segments.iter().enumerate() {
                if token_clone.is_cancelled() {
                    emit(
                        &progress_clone,
                        "cancelled",
                        "Auto-sync cancelled by user.".to_string(),
                        100.0,
                        Some("sync.autoSyncProgress.cancelled"),
                        None,
                    );
                    return Ok((all_matches, idx, true));
                }

                let progress = (idx as f64 / total_segments as f64) * 80.0 + 10.0;

                let start_label = format_mm_ss(prep.current_pos);
                let end_label = format_mm_ss(prep.current_pos + segment_duration);
                emit(
                    &progress_clone,
                    "transcribe",
                    format!(
                        "Analyzing segment {}/{} - media {} -> {} ({}s)",
                        idx + 1,
                        total_segments,
                        start_label,
                        end_label,
                        segment_duration.round() as i64
                    ),
                    progress,
                    Some("sync.autoSyncProgress.transcribingSegment"),
                    Some(HashMap::from([
                        ("current".to_string(), (idx + 1).to_string()),
                        ("total".to_string(), total_segments.to_string()),
                        ("start".to_string(), start_label),
                        ("end".to_string(), end_label),
                        ("duration".to_string(), format!("{}s", segment_duration.round() as i64)),
                    ])),
                );

                let options = TranscribeOptions {
                    language: language.clone(),
                    translate_to_english: false,
                    n_threads: None,
                    word_timestamps: true,
                    max_segment_length: None,
                    segment_callback: None,
                };

                let (transcribed, _) =
                    match transcribe_full(&ctx, &prep.audio_data, &options, Some(&token_clone)) {
                        Ok(segs) => segs,
                        Err(e) => {
                            eprintln!("[auto-sync] Segment {} transcription failed: {e}", prep.idx);
                            continue;
                        }
                    };

                let adjusted_segments: Vec<TranscribedSegment> = transcribed
                    .into_iter()
                    .map(|mut seg| {
                        seg.start_ms += (prep.current_pos * 1000.0) as i64;
                        seg.end_ms += (prep.current_pos * 1000.0) as i64;
                        seg
                    })
                    .collect();

                for tseg in &adjusted_segments {
                    // Tiny or one-word fragments are very noisy for alignment.
                    if tseg.text.split_whitespace().count() < 2 {
                        continue;
                    }

                    let near_idx = subtitles_sorted
                        .partition_point(|(_, sub_start_ms, _)| *sub_start_ms < tseg.start_ms);
                    let window_start = near_idx.saturating_sub(40);
                    let window_end = (near_idx + 40).min(subtitles_sorted.len());

                    for &(sub_id, sub_start_ms, ref sub_text) in
                        &subtitles_sorted[window_start..window_end]
                    {
                        let time_diff = (sub_start_ms - tseg.start_ms).abs();
                        if time_diff > 45_000 {
                            continue;
                        }

                        let sim = text_similarity(&tseg.text, sub_text);
                        if sim > 0.42 {
                            let score = sim * temporal_weight(time_diff);
                            if score < 0.40 {
                                continue;
                            }

                            all_matches.push(MatchCandidate {
                                subtitle_id: sub_id,
                                original_start_ms: sub_start_ms,
                                transcribed_start_ms: tseg.start_ms,
                                similarity: sim,
                                score,
                            });
                        }
                    }
                }
            }

            Ok((all_matches, total_segments, token_clone.is_cancelled()))
        },
    )
    .await
    .map_err(|e| anyhow::anyhow!("Task panic: {e:?}"))?;

    let (all_matches, segments_analyzed, is_cancelled) =
        spawn_res.map_err(|e| anyhow::anyhow!(e))?;

    if is_cancelled {
        return Ok(AutoSyncOutcome {
            suggestions: Vec::new(),
            segments_analyzed,
            cancelled: true,
        });
    }

    // Estimate the dominant global offset by density: the offset shared by the
    // largest cluster of matches (±15 s) wins.
    let mut best_offset = 0i64;
    let mut max_dense_count = 0;

    for m in &all_matches {
        let current_offset = m.transcribed_start_ms - m.original_start_ms;
        let count = all_matches
            .iter()
            .filter(|other| {
                let other_offset = other.transcribed_start_ms - other.original_start_ms;
                (other_offset - current_offset).abs() <= 15_000
            })
            .count();
        if count > max_dense_count {
            max_dense_count = count;
            best_offset = current_offset;
        }
    }

    // Geometric verification: drop matches disagreeing with the consensus.
    let geometrically_verified: Vec<MatchCandidate> = all_matches
        .into_iter()
        .filter(|m| {
            let offset = m.transcribed_start_ms - m.original_start_ms;
            (offset - best_offset).abs() <= 15_000
        })
        .collect();

    // Best match per subtitle.
    let mut best_per_sub: HashMap<u32, MatchCandidate> = HashMap::new();
    for m in geometrically_verified {
        let entry = best_per_sub.entry(m.subtitle_id).or_insert_with(|| m.clone());
        if m.score > entry.score || (m.score == entry.score && m.similarity > entry.similarity) {
            *entry = m;
        }
    }

    let mut final_matches: Vec<MatchCandidate> = best_per_sub.into_values().collect();
    final_matches.sort_by_key(|m| m.original_start_ms);

    // Space anchors at least 30 s apart.
    let mut suggestions: Vec<AnchorSuggestion> = Vec::new();
    let mut last_time: Option<i64> = None;
    for m in final_matches {
        if last_time.is_none_or(|lt| m.original_start_ms.saturating_sub(lt) >= 30_000) {
            last_time = Some(m.original_start_ms);
            suggestions.push(AnchorSuggestion {
                subtitle_id: m.subtitle_id,
                original_start_ms: m.original_start_ms,
                corrected_time_ms: m.transcribed_start_ms,
                similarity: m.similarity,
                score: m.score,
            });
        }
    }

    Ok(AutoSyncOutcome { suggestions, segments_analyzed, cancelled: false })
}
