use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use serde::Serialize;
use tokio_util::sync::CancellationToken;

use srt_transcribe::audio::{extract_wav_segment, read_wav_to_f32};
use srt_transcribe::transcribe::{TranscribeOptions, TranscribedSegment, transcribe_full};

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
}

#[derive(Debug, Clone, Serialize)]
pub struct AnchorSuggestion {
    pub subtitle_id: u32,
    pub original_start_ms: i64,
    pub corrected_time_ms: i64,
    pub similarity: f64,
    pub score: f64,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
struct MatchCandidate {
    subtitle_id: u32,
    original_start_ms: i64,
    transcribed_start_ms: i64,
    similarity: f64,
    score: f64,
}

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

            struct SubtitleCandidate {
                id: u32,
                start_ms: i64,
                norm: srt_transcribe::transcribe::NormalizedText,
            }

            let mut subtitles_sorted: Vec<SubtitleCandidate> = subtitles
                .into_iter()
                .map(|s| SubtitleCandidate {
                    id: s.id,
                    start_ms: s.start_ms,
                    norm: srt_transcribe::transcribe::NormalizedText::new(&s.text),
                })
                .collect();
            subtitles_sorted.sort_unstable_by_key(|s| s.start_ms);

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
                    let tseg_norm = srt_transcribe::transcribe::NormalizedText::new(&tseg.text);
                    if tseg_norm.tokens.len() < 2 {
                        continue;
                    }

                    let near_idx =
                        subtitles_sorted.partition_point(|sub| sub.start_ms < tseg.start_ms);
                    let window_start = near_idx.saturating_sub(40);
                    let window_end = (near_idx + 40).min(subtitles_sorted.len());

                    for sub in &subtitles_sorted[window_start..window_end] {
                        let time_diff = (sub.start_ms - tseg.start_ms).abs();
                        if time_diff > 45_000 {
                            continue;
                        }

                        let sim = tseg_norm.similarity_with_min_threshold(&sub.norm, 0.42);
                        if sim > 0.42 {
                            let score = sim * temporal_weight(time_diff);
                            if score < 0.40 {
                                continue;
                            }

                            all_matches.push(MatchCandidate {
                                subtitle_id: sub.id,
                                original_start_ms: sub.start_ms,
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

    let mut best_offset = 0i64;
    let mut max_dense_count = 0;

    if !all_matches.is_empty() {
        let mut sorted_offsets: Vec<i64> = all_matches
            .iter()
            .map(|m| m.transcribed_start_ms - m.original_start_ms)
            .collect();
        sorted_offsets.sort_unstable();

        for &current_offset in &sorted_offsets {
            let left_bound = current_offset - 15_000;
            let right_bound = current_offset + 15_000;
            let left_idx = sorted_offsets.partition_point(|&off| off < left_bound);
            let right_idx = sorted_offsets.partition_point(|&off| off <= right_bound);
            let count = right_idx.saturating_sub(left_idx);
            if count > max_dense_count {
                max_dense_count = count;
                best_offset = current_offset;
            }
        }
    }

    let geometrically_verified: Vec<MatchCandidate> = all_matches
        .into_iter()
        .filter(|m| {
            let offset = m.transcribed_start_ms - m.original_start_ms;
            (offset - best_offset).abs() <= 15_000
        })
        .collect();

    let mut best_per_sub: HashMap<u32, MatchCandidate> = HashMap::new();
    for m in geometrically_verified {
        let entry = best_per_sub
            .entry(m.subtitle_id)
            .or_insert_with(|| m.clone());
        if m.score > entry.score || (m.score == entry.score && m.similarity > entry.similarity) {
            *entry = m;
        }
    }

    let mut final_matches: Vec<MatchCandidate> = best_per_sub.into_values().collect();
    final_matches.sort_unstable_by_key(|m| m.original_start_ms);

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
        assert_eq!(format_mm_ss(-10.0), "00:00"); // clamped to 0
    }

    #[test]
    fn test_temporal_weight() {
        // <= 8000ms is 1.0
        assert_eq!(temporal_weight(0), 1.0);
        assert_eq!(temporal_weight(5000), 1.0);
        assert_eq!(temporal_weight(8000), 1.0);

        // >= 45000ms is 0.65
        assert_eq!(temporal_weight(45000), 0.65);
        assert_eq!(temporal_weight(60000), 0.65);

        // Intermediate value
        let mid = temporal_weight(26500); // exactly halfway between 8000 and 45000
        assert!((mid - (1.0 - 0.35 * 0.5)).abs() < 1e-6);
    }
}

#[cfg(all(test, unix))]
mod lifecycle_tests;
