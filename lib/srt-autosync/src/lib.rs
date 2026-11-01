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

pub use consensus::{LinearModel, extract_consensus_anchors, extract_consensus_anchors_guided};
pub use matching::{
    AnchorPrior, CandidateCue, MatchCandidate, SubtitleIndex, calculate_guided_temporal_weight,
    calculate_temporal_weight, refine_speech_onset,
};
pub use normalization::{CleanText, detect_subtitle_language, strip_markup_and_sdh};

#[derive(Debug, Clone)]
pub struct SubtitleLine {
    pub id: u32,
    pub start_ms: i64,
    pub text: String,
}

/// An existing fixed ground-truth anchor constraint from the user or session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExistingAnchor {
    pub original_time_ms: i64,
    pub target_time_ms: i64,
}

impl ExistingAnchor {
    pub fn new(original_time_ms: i64, target_time_ms: i64) -> Self {
        Self {
            original_time_ms,
            target_time_ms,
        }
    }

    pub fn offset(&self) -> i64 {
        self.target_time_ms.saturating_sub(self.original_time_ms)
    }

    pub fn corrected_time_ms(&self) -> i64 {
        self.target_time_ms
    }
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
    pub existing_anchors: Vec<ExistingAnchor>,
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
            existing_anchors: Vec::new(),
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
/// guided by subtitle distribution and existing fixed ground-truth anchors.
pub fn calculate_sample_positions(
    duration_sec: f64,
    segment_duration: f64,
    num_samples: usize,
    subtitles: &[SubtitleLine],
    existing_anchors: &[ExistingAnchor],
) -> Vec<f64> {
    if !existing_anchors.is_empty() {
        let guided = calculate_guided_sample_positions(
            duration_sec,
            segment_duration,
            num_samples,
            subtitles,
            existing_anchors,
        );
        if !guided.is_empty() {
            return guided;
        }
    }

    calculate_unconstrained_sample_positions(duration_sec, segment_duration, num_samples, subtitles)
}

/// Sample audio segments specifically in large unanchored spans between or outside existing anchors.
fn calculate_guided_sample_positions(
    duration_sec: f64,
    segment_duration: f64,
    num_samples: usize,
    subtitles: &[SubtitleLine],
    existing_anchors: &[ExistingAnchor],
) -> Vec<f64> {
    let prior = AnchorPrior::new(existing_anchors.to_vec());
    let mut anchor_targets: Vec<f64> = existing_anchors
        .iter()
        .map(|a| a.target_time_ms as f64 / 1000.0)
        .collect();
    anchor_targets.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Define unanchored spans across the timeline
    let mut spans = Vec::new();
    let mut prev_bound = 0.0f64;

    for &anchor_t in &anchor_targets {
        if anchor_t > prev_bound {
            spans.push((prev_bound, anchor_t));
        }
        prev_bound = anchor_t;
    }
    if prev_bound < duration_sec {
        spans.push((prev_bound, duration_sec));
    }

    struct EligibleSpan {
        s_min: f64,
        s_max: f64,
        valid_len: f64,
    }

    let mut eligible_spans = Vec::new();
    for (start, end) in spans {
        let s_min = if start == 0.0 { 5.0 } else { start + 15.0 };
        let s_max = if end >= duration_sec {
            (duration_sec - segment_duration).max(0.0)
        } else {
            end - 15.0 - segment_duration
        };

        if s_max >= s_min + 5.0 {
            eligible_spans.push(EligibleSpan {
                s_min,
                s_max,
                valid_len: s_max - s_min,
            });
        }
    }

    if eligible_spans.is_empty() {
        return Vec::new();
    }

    let total_valid_len: f64 = eligible_spans.iter().map(|s| s.valid_len).sum();
    let mut positions = Vec::new();

    for span in &eligible_spans {
        let span_ratio = span.valid_len / total_valid_len;
        let count = ((num_samples as f64) * span_ratio).round() as usize;
        let span_samples = count.clamp(
            1,
            ((span.valid_len / (segment_duration * 0.75)) as usize).max(1),
        );

        let step = span.valid_len / (span_samples + 1) as f64;
        for j in 1..=span_samples {
            let target_time = span.s_min + (step * j as f64);
            let target_ms = (target_time * 1000.0) as i64;
            let target_orig_ms = prior.expected_orig_time(target_ms).unwrap_or(target_ms);

            let near_idx = subtitles.partition_point(|s| s.start_ms < target_orig_ms);
            let candidate_sub = if near_idx < subtitles.len() {
                Some(&subtitles[near_idx])
            } else if !subtitles.is_empty() {
                Some(&subtitles[subtitles.len() - 1])
            } else {
                None
            };

            let pos = if let Some(sub) = candidate_sub {
                let sub_target_ms = prior
                    .expected_target_time(sub.start_ms)
                    .unwrap_or(sub.start_ms);
                let sub_target_sec = (sub_target_ms as f64 / 1000.0) - 2.0;
                sub_target_sec.clamp(span.s_min, span.s_max)
            } else {
                target_time.clamp(span.s_min, span.s_max)
            };

            positions.push(pos);
        }
    }

    positions.sort_by(|a, b| a.partial_cmp(b).unwrap());
    positions.dedup_by(|a, b| (*a - *b).abs() < segment_duration * 0.7);

    // Keep at least 10s separation from existing manual anchors
    positions.retain(|&pos| {
        !anchor_targets
            .iter()
            .any(|&at| (at - pos).abs() < 10.0 || (at - (pos + segment_duration)).abs() < 5.0)
    });

    positions
}

fn calculate_unconstrained_sample_positions(
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
    let use_gpu = config.use_gpu.unwrap_or(true) && srt_transcribe::gpu_supported();

    // Dense sampling: scale segments up when GPU is available (24-32 segments)
    let num_samples = if use_gpu {
        let dynamic_count = (duration_sec / 150.0).round() as usize;
        if quick {
            dynamic_count.clamp(16, 24)
        } else {
            dynamic_count.clamp(24, 32)
        }
    } else if quick {
        12
    } else {
        20
    };
    let max_possible = ((duration_sec - 10.0) / (segment_duration * 0.75)) as usize;
    let num_samples = num_samples.min(max_possible.max(1));

    let sample_positions = calculate_sample_positions(
        duration_sec,
        segment_duration,
        num_samples,
        &subtitles,
        &config.existing_anchors,
    );
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

    let max_concurrency = if use_gpu {
        num_cpus().clamp(4, 8)
    } else {
        num_cpus().min(6)
    };
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
    let max_offset_ms = (config.max_offset_secs.unwrap_or(120) as i64) * 1000;
    let language = effective_language;
    let existing_anchors_for_task = config.existing_anchors.clone();
    let token_clone = cancel_token.clone();
    let progress_clone = on_progress.clone();

    let spawn_res = tokio::task::spawn_blocking(
        move || -> Result<(Vec<MatchCandidate>, usize, bool), String> {
            let mut all_matches: Vec<MatchCandidate> = Vec::new();

            let ctx = load_whisper_context_with_gpu(&model_path_str, use_gpu)
                .map_err(|e| format!("Failed to load Whisper model: {e:?}"))?;

            let subtitle_index = SubtitleIndex::build(&subtitles);
            let anchor_prior = AnchorPrior::new(existing_anchors_for_task);
            let has_prior = !anchor_prior.is_empty();

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

                let token_timings = Arc::new(std::sync::Mutex::new(HashMap::new()));
                let timing_sink = token_timings.clone();
                let options = TranscribeOptions {
                    language: language.clone(),
                    translate_to_english: false,
                    n_threads: Some(if use_gpu {
                        4
                    } else {
                        srt_transcribe::transcribe::default_n_threads()
                    }),
                    word_timestamps: true,
                    max_segment_length: None,
                    segment_callback: None,
                    token_callback: Some(Arc::new(move |start, tokens| {
                        timing_sink.lock().unwrap().insert(start, tokens);
                    })),
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

                let token_timings = token_timings.lock().unwrap();
                let clip_offset_ms = (prep.current_pos * 1000.0) as i64;
                for tseg in &adjusted_segments {
                    let tseg_clean = CleanText::new(&tseg.text);
                    if tseg_clean.tokens.len() < 2 {
                        continue;
                    }

                    let (candidates, expected_offset, local_radius_ms) = if has_prior {
                        const LOCAL_RADIUS: i64 = 15_000;
                        let (min_t, max_t) =
                            anchor_prior.candidate_orig_bounds(tseg.start_ms, LOCAL_RADIUS);
                        let cands = subtitle_index.find_in_range(min_t, max_t);
                        let exp_off = anchor_prior
                            .expected_offset_at_target(tseg.start_ms)
                            .unwrap_or(0);
                        (cands, exp_off, LOCAL_RADIUS)
                    } else {
                        let cands = subtitle_index.find_in_window(tseg.start_ms, max_offset_ms);
                        (cands, 0, max_offset_ms)
                    };

                    for cand in candidates {
                        let sim = tseg_clean.similarity(&cand.text);

                        // Adaptive similarity threshold: short phrases require higher similarity, longer phrases 0.40
                        let threshold = if tseg_clean.tokens.len() <= 3 {
                            0.46
                        } else {
                            0.40
                        };
                        if sim >= threshold {
                            let weight = if has_prior {
                                calculate_guided_temporal_weight(
                                    tseg.start_ms - cand.start_ms,
                                    expected_offset,
                                    local_radius_ms,
                                )
                            } else {
                                let time_diff = (cand.start_ms - tseg.start_ms).abs();
                                calculate_temporal_weight(time_diff, max_offset_ms)
                            };

                            let score = sim * weight;

                            if score >= 0.35 {
                                // Word-level phrase onset refinement
                                let refined_start_ms = refine_speech_onset(
                                    tseg.start_ms - clip_offset_ms,
                                    tseg.end_ms - clip_offset_ms,
                                    token_timings
                                        .get(&(tseg.start_ms - clip_offset_ms))
                                        .map(Vec::as_slice)
                                        .unwrap_or(&[]),
                                    &cand.text,
                                );

                                all_matches.push(MatchCandidate {
                                    subtitle_id: cand.subtitle_id,
                                    original_start_ms: cand.start_ms,
                                    transcribed_start_ms: refined_start_ms + clip_offset_ms,
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
    let (_model, suggestions) =
        extract_consensus_anchors_guided(&all_matches, 30_000, &config.existing_anchors);

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

        let positions = calculate_sample_positions(2000.0, 20.0, 10, &subtitles, &[]);
        assert!(!positions.is_empty());
        assert!(positions[0] >= 50.0);
        assert!(*positions.last().unwrap() <= 1900.0);
    }

    #[test]
    fn test_calculate_sample_positions_with_existing_anchors_targeted_spans() {
        let subtitles = (0..100)
            .map(|i| SubtitleLine {
                id: i,
                start_ms: (10 + i as i64 * 20) * 1000,
                text: format!("Subtitle dialogue line {i}"),
            })
            .collect::<Vec<_>>();

        // User placed 2 anchors: at 300s (target 310s) and at 900s (target 910s)
        let anchors = vec![
            ExistingAnchor::new(300_000, 310_000),
            ExistingAnchor::new(900_000, 910_000),
        ];

        let positions = calculate_sample_positions(1800.0, 20.0, 12, &subtitles, &anchors);
        assert!(!positions.is_empty());

        // Positions should avoid being right on top of 310s or 910s (>= 10s distance)
        for &pos in &positions {
            assert!(
                (pos - 310.0).abs() >= 10.0,
                "Position {pos} too close to anchor at 310s"
            );
            assert!(
                (pos - 910.0).abs() >= 10.0,
                "Position {pos} too close to anchor at 910s"
            );
        }

        // Must have positions in the middle span (between 310s and 910s)
        let mid_samples = positions
            .iter()
            .filter(|&&p| p > 310.0 && p < 910.0)
            .count();
        assert!(
            mid_samples >= 2,
            "Expected samples between anchors, got {mid_samples}"
        );

        // Must have positions in the later span (> 910s)
        let late_samples = positions.iter().filter(|&&p| p > 910.0).count();
        assert!(
            late_samples >= 1,
            "Expected samples after last anchor, got {late_samples}"
        );
    }
}

#[cfg(all(test, unix))]
mod lifecycle_tests;
