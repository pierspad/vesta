use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use srt_transcribe::audio::{extract_wav_segment, read_wav_to_f32};
use srt_transcribe::transcribe::{TranscribeOptions, TranscribedSegment, transcribe_full};

pub mod consensus;
pub mod matching;
pub mod normalization;

pub use consensus::{LinearModel, extract_consensus_anchors};
pub use matching::{CandidateCue, MatchCandidate, SubtitleIndex, calculate_temporal_weight};
pub use normalization::{CleanText, detect_subtitle_language, strip_markup_and_sdh};

#[derive(Debug, Clone)]
pub struct SubtitleLine {
    pub id: u32,
    pub start_ms: i64,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct AutoSyncConfig {
    pub media_path: String,
    pub model_path: PathBuf,
    pub language: Option<String>,
    pub quick: bool,
    pub ffmpeg_cmd: String,
    pub ffprobe_cmd: String,
    pub use_gpu: Option<bool>,
    pub max_offset_secs: Option<u32>,
}

impl AutoSyncConfig {
    pub fn new(media_path: String, model_path: PathBuf) -> Self {
        Self {
            media_path,
            model_path,
            language: None,
            quick: false,
            ffmpeg_cmd: "ffmpeg".into(),
            ffprobe_cmd: "ffprobe".into(),
            use_gpu: None,
            max_offset_secs: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorSuggestion {
    pub subtitle_id: u32,
    pub original_start_ms: i64,
    pub corrected_time_ms: i64,
    pub similarity: f64,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutoSyncOutcome {
    pub suggestions: Vec<AnchorSuggestion>,
    pub segments_analyzed: usize,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutoSyncProgress {
    pub stage: String,
    pub message: String,
    pub percentage: f64,
    pub message_key: Option<String>,
    pub params: Option<HashMap<String, String>>,
}

pub type ProgressCallback = Arc<dyn Fn(AutoSyncProgress) + Send + Sync>;

fn is_silent(samples: &[f32], threshold: f32) -> bool {
    if samples.is_empty() {
        return true;
    }
    let sum_sq: f32 = samples.iter().map(|&x| x * x).sum();
    let rms = (sum_sq / samples.len() as f32).sqrt();
    rms < threshold
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

fn format_mm_ss(total_seconds: f64) -> String {
    let clamped = total_seconds.max(0.0).round() as i64;
    format!("{:02}:{:02}", clamped / 60, clamped % 60)
}

pub fn temporal_weight(time_diff_ms: i64) -> f64 {
    if time_diff_ms <= 8_000 {
        return 1.0;
    }
    if time_diff_ms >= 45_000 {
        return 0.65;
    }
    let normalized = (time_diff_ms - 8_000) as f64 / (45_000 - 8_000) as f64;
    1.0 - (normalized * 0.35)
}

pub async fn get_media_duration(media_path: &str, ffprobe_cmd: &str) -> Result<f64> {
    let mut command = tokio::process::Command::new(ffprobe_cmd);
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        command
            .kill_on_drop(true)
            .stdin(std::process::Stdio::null())
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                media_path,
            ])
            .output(),
    )
    .await
    .context("Media probe timed out")?
    .context("Failed to run ffprobe")?;

    if !output.status.success() {
        anyhow::bail!("ffprobe failed");
    }

    let duration_str = String::from_utf8_lossy(&output.stdout);
    let duration = duration_str
        .trim()
        .parse::<f64>()
        .context("Failed to parse duration from ffprobe")?;
    anyhow::ensure!(
        duration.is_finite() && duration > 0.0,
        "Invalid media duration"
    );
    Ok(duration)
}

struct PreparedSegment {
    idx: usize,
    current_pos: f64,
    audio_data: Vec<f32>,
    _wav_path: PathBuf,
}

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
    cancel: CancellationToken,
) -> Result<PreparedSegment, String> {
    let mut current_pos = start_pos;
    let mut attempts = 0;
    let mut audio_data = Vec::new();
    let mut wav_path = temp_dir_path.join(format!("segment_{idx}.wav"));

    let max_attempts = if quick { 5 } else { 3 };
    let shift_amount = if quick { 15.0 } else { 20.0 };

    while attempts < max_attempts && current_pos + segment_duration <= duration_sec {
        let temp_wav_path = temp_dir_path.join(format!("segment_{idx}_try{attempts}.wav"));
        let extraction = extract_wav_segment(
            &ffmpeg_cmd,
            std::path::Path::new(&media_path),
            &temp_wav_path,
            current_pos,
            segment_duration,
            Some(&cancel),
        )
        .await;
        if cancel.is_cancelled() {
            return Err("Audio preparation cancelled".into());
        }
        if extraction.is_err() {
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
        extract_wav_segment(
            &ffmpeg_cmd,
            std::path::Path::new(&media_path),
            &wav_path,
            start_pos,
            segment_duration,
            Some(&cancel),
        )
        .await
        .map_err(|error| format!("Failed to prepare segment {idx}: {error:#}"))?;
        let wav_path_clone = wav_path.clone();
        audio_data = tokio::task::spawn_blocking(move || read_wav_to_f32(&wav_path_clone))
            .await
            .map_err(|error| format!("WAV decoding task failed: {error}"))?
            .map_err(|error| format!("Failed to decode segment {idx}: {error:#}"))?;
        current_pos = start_pos;
    }

    Ok(PreparedSegment {
        idx,
        current_pos,
        audio_data,
        _wav_path: wav_path,
    })
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

pub fn load_whisper_context(
    model_path: &str,
) -> Result<whisper_rs::WhisperContext, whisper_rs::WhisperError> {
    load_whisper_context_with_gpu(model_path, srt_transcribe::gpu_supported())
}

pub fn load_whisper_context_with_gpu(
    model_path: &str,
    use_gpu: bool,
) -> Result<whisper_rs::WhisperContext, whisper_rs::WhisperError> {
    if use_gpu && srt_transcribe::gpu_supported() {
        let mut gpu_params = whisper_rs::WhisperContextParameters::default();
        gpu_params.use_gpu(true);
        if let Ok(ctx) = whisper_rs::WhisperContext::new_with_params(model_path, gpu_params) {
            return Ok(ctx);
        }
    }

    let mut cpu_params = whisper_rs::WhisperContextParameters::default();
    cpu_params.use_gpu(false);
    whisper_rs::WhisperContext::new_with_params(model_path, cpu_params)
}

/// Calculate strategic sample positions across the media duration,
/// guided by subtitle distribution where dialogue actually occurs.
fn calculate_sample_positions(
    duration_sec: f64,
    segment_duration: f64,
    num_samples: usize,
    subtitles: &[SubtitleLine],
) -> Vec<f64> {
    let mut positions = Vec::with_capacity(num_samples);

    if subtitles.len() >= 10 {
        let first_sub_sec = (subtitles[0].start_ms as f64 / 1000.0).max(5.0);
        let last_sub_sec = (subtitles.last().unwrap().start_ms as f64 / 1000.0)
            .min(duration_sec - segment_duration);

        if last_sub_sec > first_sub_sec + 60.0 {
            let span = last_sub_sec - first_sub_sec;
            let step = span / (num_samples + 1) as f64;

            for i in 1..=num_samples {
                let target_time = first_sub_sec + (step * i as f64);
                // Find a nearby subtitle to anchor the audio sample on dialogue
                let target_ms = (target_time * 1000.0) as i64;
                let near_idx = subtitles.partition_point(|s| s.start_ms < target_ms);
                let best_sub = if near_idx < subtitles.len() {
                    &subtitles[near_idx]
                } else {
                    &subtitles[subtitles.len() - 1]
                };

                let pos = ((best_sub.start_ms as f64 / 1000.0) - 2.0)
                    .clamp(0.0, (duration_sec - segment_duration).max(0.0));
                positions.push(pos);
            }
        }
    }

    if positions.is_empty() {
        let step = duration_sec / (num_samples + 1) as f64;
        for i in 1..=num_samples {
            let pos = step * i as f64;
            if pos + segment_duration <= duration_sec {
                positions.push(pos);
            }
        }
    }

    if positions.is_empty() {
        positions.push(0.0);
    }

    positions.sort_by(|a, b| a.partial_cmp(b).unwrap());
    positions.dedup_by(|a, b| (*a - *b).abs() < segment_duration * 0.7);
    positions
}

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
        .is_ok_and(|o| o.status.success());
    if !ffmpeg_ok {
        anyhow::bail!("FFmpeg is required for auto-sync. Install FFmpeg first.");
    }

    let duration_sec = tokio::select! {
        _ = cancel_token.cancelled() => return Ok(AutoSyncOutcome {
            suggestions: Vec::new(), segments_analyzed: 0, cancelled: true,
        }),
        result = get_media_duration(&config.media_path, &config.ffprobe_cmd) => result?,
    };
    if duration_sec < 10.0 {
        anyhow::bail!("Media file too short or unable to detect duration");
    }

    let quick = config.quick;
    let segment_duration = if quick { 20.0 } else { 35.0 };
    let num_samples = if quick { 12 } else { 20 };

    let sample_positions =
        calculate_sample_positions(duration_sec, segment_duration, num_samples, &subtitles);
    let total_segments = sample_positions.len();

    // Detect subtitle language if not provided
    let effective_language = match &config.language {
        Some(lang) if !lang.trim().is_empty() && lang != "auto" => Some(lang.clone()),
        _ => detect_subtitle_language(&subtitles),
    };

    let start_msg = if let Some(ref lang) = effective_language {
        format!(
            "Preparing auto-sync (language: {lang}): {total_segments} audio segments to analyze..."
        )
    } else {
        format!("Preparing auto-sync: {total_segments} audio segments to analyze...")
    };

    emit(
        &on_progress,
        "start",
        start_msg,
        0.0,
        Some("sync.autoSyncProgress.analyzingSegments"),
        Some(HashMap::from([(
            "total".to_string(),
            total_segments.to_string(),
        )])),
    );

    let temp_dir = tempfile::tempdir().context("Failed to create temp dir")?;
    let temp_dir_path = temp_dir.path().to_path_buf();

    emit(
        &on_progress,
        "prepare",
        format!("Preparing and extracting {total_segments} audio segments in parallel..."),
        3.0,
        Some("sync.autoSyncProgress.preparingSegments"),
        Some(HashMap::from([(
            "total".to_string(),
            total_segments.to_string(),
        )])),
    );

    let max_concurrency = num_cpus().min(6);
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrency));
    let mut prep_handles = tokio::task::JoinSet::new();

    for (idx, &start_pos) in sample_positions.iter().enumerate() {
        let sem = semaphore.clone();
        let media_path = config.media_path.clone();
        let temp_dir_path = temp_dir_path.clone();
        let ffmpeg_cmd = config.ffmpeg_cmd.clone();

        let token = cancel_token.clone();
        prep_handles.spawn(async move {
            let _permit = sem.acquire().await.map_err(|e| e.to_string())?;
            prepare_single_segment(
                idx,
                start_pos,
                segment_duration,
                duration_sec,
                quick,
                media_path,
                temp_dir_path,
                ffmpeg_cmd,
                token,
            )
            .await
        });
    }

    let mut prepared_segments = Vec::new();
    let mut completed = 0;
    while !prep_handles.is_empty() {
        let result = tokio::select! {
            biased;
            _ = cancel_token.cancelled() => return Ok(AutoSyncOutcome {
                suggestions: Vec::new(), segments_analyzed: completed, cancelled: true,
            }),
            result = prep_handles.join_next() => result,
        };
        match result {
            Some(Ok(Ok(prep))) => prepared_segments.push(prep),
            Some(Ok(Err(error))) => eprintln!("[auto-sync] Segment preparation failed: {error}"),
            Some(Err(error)) => return Err(error).context("Audio preparation worker failed"),
            None => break,
        }
        completed += 1;
    }
    anyhow::ensure!(
        !prepared_segments.is_empty(),
        "No audio segments could be prepared"
    );

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
    let use_gpu = config.use_gpu.unwrap_or_else(srt_transcribe::gpu_supported);
    let max_offset_ms = (config.max_offset_secs.unwrap_or(120) as i64) * 1000;
    let language = effective_language;
    let token_clone = cancel_token.clone();
    let progress_clone = on_progress.clone();

    let spawn_res = tokio::task::spawn_blocking(
        move || -> Result<(Vec<MatchCandidate>, usize, bool), String> {
            let mut all_matches: Vec<MatchCandidate> = Vec::new();

            let ctx = load_whisper_context_with_gpu(&model_path_str, use_gpu)
                .map_err(|e| format!("Failed to load Whisper model: {e:?}"))?;

            let subtitle_index = SubtitleIndex::build(&subtitles);

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
                        (
                            "duration".to_string(),
                            format!("{}s", segment_duration.round() as i64),
                        ),
                    ])),
                );

                let options = TranscribeOptions {
                    language: language.clone(),
                    translate_to_english: false,
                    n_threads: None,
                    word_timestamps: true,
                    max_segment_length: None,
                    segment_callback: None,
                    ..TranscribeOptions::default()
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
                    let tseg_clean = CleanText::new(&tseg.text);
                    if tseg_clean.tokens.len() < 2 {
                        continue;
                    }

                    let candidates = subtitle_index.find_in_window(tseg.start_ms, max_offset_ms);

                    for cand in candidates {
                        let time_diff = (cand.start_ms - tseg.start_ms).abs();
                        let sim = tseg_clean.similarity(&cand.text);

                        // Adaptive similarity threshold: short phrases require higher similarity, longer phrases 0.40
                        let threshold = if tseg_clean.tokens.len() <= 3 {
                            0.46
                        } else {
                            0.40
                        };
                        if sim >= threshold {
                            let weight = calculate_temporal_weight(time_diff, max_offset_ms);
                            let score = sim * weight;

                            if score >= 0.35 {
                                all_matches.push(MatchCandidate {
                                    subtitle_id: cand.subtitle_id,
                                    original_start_ms: cand.start_ms,
                                    transcribed_start_ms: tseg.start_ms,
                                    similarity: sim,
                                    score,
                                });
                            }
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

    // Extract consensus model and well-spaced inlier anchors (spacing >= 30s)
    let (_model, suggestions) = extract_consensus_anchors(&all_matches, 30_000);

    Ok(AutoSyncOutcome {
        suggestions,
        segments_analyzed,
        cancelled: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_silent() {
        assert!(is_silent(&[], 0.01));

        let zeros = vec![0.0f32; 1000];
        assert!(is_silent(&zeros, 0.01));

        let low_noise = vec![0.001f32; 1000];
        assert!(is_silent(&low_noise, 0.01));

        let signal = vec![0.5f32; 1000];
        assert!(!is_silent(&signal, 0.01));
    }

    #[test]
    fn test_format_mm_ss() {
        assert_eq!(format_mm_ss(0.0), "00:00");
        assert_eq!(format_mm_ss(59.4), "00:59");
        assert_eq!(format_mm_ss(65.0), "01:05");
        assert_eq!(format_mm_ss(3600.0), "60:00");
        assert_eq!(format_mm_ss(-10.0), "00:00");
    }

    #[test]
    fn test_temporal_weight() {
        assert_eq!(temporal_weight(0), 1.0);
        assert_eq!(temporal_weight(5000), 1.0);
        assert_eq!(temporal_weight(8000), 1.0);

        assert_eq!(temporal_weight(45000), 0.65);
        assert_eq!(temporal_weight(60000), 0.65);

        let mid = temporal_weight(26500);
        assert!((mid - (1.0 - 0.35 * 0.5)).abs() < 1e-6);
    }

    #[test]
    fn test_load_whisper_context_fallback_on_invalid_path() {
        let result = load_whisper_context("/non/existent/path/ggml-model.bin");
        assert!(result.is_err());
    }

    #[test]
    fn test_wide_window_heavily_desynced_matching() {
        let subtitles = vec![
            SubtitleLine {
                id: 1,
                start_ms: 10_000,
                text: "Well, here we are at the station.".into(),
            },
            SubtitleLine {
                id: 2,
                start_ms: 15_000,
                text: "Thanks mister, I'll get off right here.".into(),
            },
            SubtitleLine {
                id: 3,
                start_ms: 22_000,
                text: "Where are you heading tonight?".into(),
            },
        ];

        let index = SubtitleIndex::build(&subtitles);

        // Subtitle is desynced by +65.0 seconds (65_000 ms)
        let tseg_start = 10_000 + 65_000;
        let tseg_text = CleanText::new("Well here we are at the station");

        let candidates = index.find_in_window(tseg_start, 120_000);
        assert!(!candidates.is_empty());

        let matched = candidates.iter().find(|c| c.subtitle_id == 1).unwrap();
        let sim = tseg_text.similarity(&matched.text);
        assert!(sim > 0.85);

        let weight = calculate_temporal_weight(tseg_start - matched.start_ms, 120_000);
        let score = sim * weight;
        assert!(score > 0.60);
    }

    #[test]
    fn test_calculate_sample_positions_centers_on_dialogue() {
        let subtitles = (0..50)
            .map(|i| SubtitleLine {
                id: i,
                start_ms: (100 + i as i64 * 30) * 1000,
                text: format!("Subtitle dialogue line {i}"),
            })
            .collect::<Vec<_>>();

        let positions = calculate_sample_positions(2000.0, 20.0, 10, &subtitles);
        assert!(!positions.is_empty());
        // Positions must fall within the dialogue bounds
        assert!(positions[0] >= 50.0);
        assert!(*positions.last().unwrap() <= 1900.0);
    }
}

#[cfg(all(test, unix))]
mod lifecycle_tests;
