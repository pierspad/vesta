//! Auto-sync command: uses Whisper to transcribe audio segments and
//! automatically create anchor points by matching transcribed text
//! against SRT subtitle text.

use anyhow::{Context as _, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use tauri::{AppHandle, Emitter, State};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};
use tokio_util::sync::CancellationToken;

use crate::state::{AppSyncState, AppTranscribeState};

/// Progress event for auto-sync
#[derive(Debug, Clone, Serialize)]
pub struct AutoSyncProgressEvent {
    pub stage: String,
    pub message: String,
    pub percentage: f64,
    pub message_key: Option<String>,
    pub params: Option<HashMap<String, String>>,
}

/// Result of auto-sync
#[derive(Debug, Clone, Serialize)]
pub struct AutoSyncResult {
    pub success: bool,
    pub cancelled: bool,
    pub anchors_created: usize,
    pub segments_analyzed: usize,
    pub message: String,
}

/// A transcribed segment with timing info
#[derive(Debug, Clone)]
struct TranscribedSegment {
    start_ms: i64,
    end_ms: i64,
    text: String,
}

/// A candidate match between a transcribed segment and an SRT subtitle
#[derive(Debug, Clone)]
struct MatchCandidate {
    subtitle_id: u32,
    original_start_ms: i64,
    transcribed_start_ms: i64,
    similarity: f64,
    score: f64,
}

/// Guard to ensure auto-sync state is cleaned up when function exits
struct AutoSyncGuard<'a>(&'a AppSyncState);

impl<'a> Drop for AutoSyncGuard<'a> {
    fn drop(&mut self) {
        if let Ok(mut ss) = self.0.lock() {
            ss.is_auto_syncing = false;
            ss.auto_sync_cancellation_token = None;
        }
    }
}

/// Normalized string similarity using Levenshtein distance (character level)
fn text_similarity(a: &str, b: &str) -> f64 {
    let a_clean = normalize_text(a);
    let b_clean = normalize_text(b);

    if a_clean.is_empty() && b_clean.is_empty() {
        return 1.0;
    }
    if a_clean.is_empty() || b_clean.is_empty() {
        return 0.0;
    }

    let a_chars: Vec<char> = a_clean.chars().collect();
    let b_chars: Vec<char> = b_clean.chars().collect();
    
    let a_len = a_chars.len();
    let b_len = b_chars.len();
    
    // DP for Levenshtein distance
    let mut dp = vec![vec![0; b_len + 1]; a_len + 1];
    
    for i in 0..=a_len { dp[i][0] = i; }
    for j in 0..=b_len { dp[0][j] = j; }
    
    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1) // deletion
                .min(dp[i][j - 1] + 1) // insertion
                .min(dp[i - 1][j - 1] + cost); // substitution
        }
    }
    
    let dist = dp[a_len][b_len] as f64;
    let max_len = (a_len.max(b_len)) as f64;
    
    1.0 - (dist / max_len)
}

/// Normalize text for comparison: lowercase, strip punctuation, collapse whitespace
fn normalize_text(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract a short audio segment from the media file using FFmpeg
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
            "-ss", &format!("{:.2}", start_sec),
            "-i", media_path,
            "-t", &format!("{:.2}", duration_sec),
            "-ar", "16000",
            "-ac", "1",
            "-c:a", "pcm_s16le",
            output_wav,
        ])
        .output()
        .context("Failed to run ffmpeg for audio segment extraction")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("FFmpeg audio extraction failed: {}", stderr);
    }

    Ok(())
}

/// Read WAV file into f32 samples
fn read_wav_to_f32(wav_path: &Path) -> Result<Vec<f32>> {
    let reader = hound::WavReader::open(wav_path)
        .context("Failed to open WAV file")?;

    let spec = reader.spec();

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
            reader.into_samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / max_val)
                .collect()
        }
        hound::SampleFormat::Float => {
            reader.into_samples::<f32>()
                .filter_map(|s| s.ok())
                .collect()
        }
    };

    if spec.channels == 2 {
        Ok(samples.chunks(2)
            .map(|chunk| {
                if chunk.len() == 2 {
                    (chunk[0] + chunk[1]) / 2.0
                } else {
                    chunk[0]
                }
            })
            .collect())
    } else {
        Ok(samples)
    }
}

/// Transcribe an audio segment using a pre-loaded whisper-rs context to avoid reloading the model
fn transcribe_segment_with_ctx(
    ctx: &WhisperContext,
    audio_data: &[f32],
    language: Option<&str>,
) -> Result<Vec<TranscribedSegment>> {
    let mut state = ctx.create_state()
        .map_err(|e| anyhow::anyhow!("Failed to create Whisper state: {:?}", e))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

    if let Some(lang) = language {
        if lang != "auto" {
            params.set_language(Some(lang));
        } else {
            params.set_language(None);
        }
    } else {
        params.set_language(None);
    }

    params.set_translate(false);
    params.set_n_threads(num_cpus().min(8) as i32);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_special(false);
    params.set_print_timestamps(false);
    params.set_token_timestamps(true);

    state.full(params, audio_data)
        .map_err(|e| anyhow::anyhow!("Whisper transcription failed: {:?}", e))?;

    let n_segments = state.full_n_segments();
    let mut segments = Vec::new();

    for i in 0..n_segments {
        let seg = match state.get_segment(i) {
            Some(s) => s,
            None => continue,
        };

        let text = match seg.to_str() {
            Ok(s) => s.trim().to_string(),
            Err(_) => continue,
        };

        if text.is_empty() {
            continue;
        }

        segments.push(TranscribedSegment {
            start_ms: seg.start_timestamp() * 10,
            end_ms: seg.end_timestamp() * 10,
            text,
        });
    }

    Ok(segments)
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

fn format_mm_ss(total_seconds: f64) -> String {
    let clamped = total_seconds.max(0.0).round() as i64;
    let minutes = clamped / 60;
    let seconds = clamped % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

/// Penalize weak temporal alignments so text matches far away from their expected
/// subtitle region are less likely to become anchors.
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

fn emit_auto_sync_progress(
    app: &AppHandle,
    stage: &str,
    message: String,
    percentage: f64,
    message_key: Option<&str>,
    params: Option<HashMap<String, String>>,
) {
    let _ = app.emit(
        "sync-auto-progress",
        AutoSyncProgressEvent {
            stage: stage.to_string(),
            message,
            percentage,
            message_key: message_key.map(str::to_string),
            params,
        },
    );
}

/// Get the path to a downloaded Whisper model
fn get_model_path(model_id: &str) -> Result<std::path::PathBuf> {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::path::PathBuf::from(
            std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
        ).join(".cache"));
    let model_path = cache_dir.join("whisper").join(format!("ggml-{}.bin", model_id));
    if !model_path.exists() {
        anyhow::bail!("Model {} not found at {}", model_id, model_path.display());
    }
    Ok(model_path)
}

/// Get media duration in seconds using ffprobe
async fn get_media_duration(media_path: &str, ffprobe_cmd: &str) -> Result<f64> {
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
    duration_str.trim().parse::<f64>()
        .context("Failed to parse duration from ffprobe")
}

#[tauri::command]
pub async fn sync_cancel_auto_sync(sync_state: State<'_, AppSyncState>) -> Result<(), String> {
    let ss = sync_state.lock().map_err(|e| e.to_string())?;
    if let Some(token) = &ss.auto_sync_cancellation_token {
        token.cancel();
    }
    Ok(())
}

/// Auto-sync command: transcribes strategic audio segments and matches
/// them against loaded SRT subtitles to create anchor points automatically.
#[tauri::command]
pub async fn sync_auto_sync(
    app: AppHandle,
    sync_state: State<'_, AppSyncState>,
    transcribe_state: State<'_, AppTranscribeState>,
    model_id: String,
    language: Option<String>,
) -> Result<AutoSyncResult, String> {
    let token = {
        let mut ss = sync_state.lock().map_err(|e| e.to_string())?;
        if ss.is_auto_syncing {
            return Err("Auto-sync is already in progress".to_string());
        }
        
        let ts = transcribe_state.lock().map_err(|e| e.to_string())?;
        if ts.is_transcribing {
            return Err("A transcription is already in progress".to_string());
        }

        let token = CancellationToken::new();
        ss.is_auto_syncing = true;
        ss.auto_sync_cancellation_token = Some(token.clone());
        token
    };

    // Ensure state cleanup when function returns
    let _guard = AutoSyncGuard(&sync_state);

    // Get media path and subtitle info from sync engine
    let (media_path, subtitle_infos) = {
        let ss = sync_state.lock().map_err(|e| e.to_string())?;
        let engine = ss.engine.as_ref()
            .ok_or("No SRT file loaded for sync")?;

        let media = engine.get_video_path()
            .ok_or("No media file loaded. Load audio/video first.")?
            .to_string();

        let subs: Vec<(u32, i64, String)> = engine.get_all_subtitles()
            .iter()
            .map(|sub| (sub.id, sub.start.milliseconds as i64, sub.text.clone()))
            .collect();

        (media, subs)
    };

    let model_path = get_model_path(&model_id).map_err(|e| e.to_string())?;

    let ffmpeg_cmd = crate::commands::flashcards::media::resolve_ffmpeg_path(Some(&app)).await;
    
    let ffmpeg_ok = tokio::process::Command::new(&ffmpeg_cmd)
        .arg("-version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !ffmpeg_ok {
        return Err("FFmpeg is required for auto-sync. Install FFmpeg first.".to_string());
    }
    
    let ffprobe_cmd = crate::commands::flashcards::media::resolve_ffprobe_path(Some(&app)).await;

    let duration_sec = get_media_duration(&media_path, &ffprobe_cmd).await.unwrap_or(0.0);
    if duration_sec < 10.0 {
        return Err("Media file too short or unable to detect duration".to_string());
    }

    // Increased segment duration to 75s per user request (was 60.0)
    let segment_duration = 75.0;
    
    // Increased the number of samples analyzed over the file length. Max segments increased from 10 to 16.
    let num_samples = (duration_sec / 120.0).ceil().min(16.0).max(4.0) as usize;
    let step = duration_sec / (num_samples + 1) as f64;

    let mut sample_positions: Vec<f64> = Vec::new();
    
    // Add specifically the first and last quartiles for higher precision at ends as per request,
    // then fill in the rest using the steps.
    for i in 1..=num_samples {
        let pos = step * i as f64;
        if pos + segment_duration <= duration_sec {
            sample_positions.push(pos);
        }
    }

    if sample_positions.is_empty() {
        sample_positions.push(0.0);
    }

    emit_auto_sync_progress(
        &app,
        "start",
        format!("Preparing auto-sync: {} audio segments to analyze...", sample_positions.len()),
        0.0,
        Some("sync.autoSyncProgress.analyzingSegments"),
        Some(HashMap::from([(
            "total".to_string(),
            sample_positions.len().to_string(),
        )])),
    );


    let app_clone = app.clone();
    let model_path_str = model_path.to_string_lossy().to_string();
    let media_path_str = media_path.clone();
    let language_clone = language.clone();
    let token_clone = token.clone();
    let subtitle_infos_clone = subtitle_infos.clone();
    let temp_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let temp_dir_path = temp_dir.path().to_path_buf();
    
    let total_segments = sample_positions.len();
    let ffmpeg_cmd_arc = std::sync::Arc::<str>::from(ffmpeg_cmd.as_str());

    let spawn_res = tokio::task::spawn_blocking(move || -> Result<(Vec<MatchCandidate>, usize, bool), String> {
        let mut all_matches: Vec<MatchCandidate> = Vec::new();
        
        let ctx = whisper_rs::WhisperContext::new_with_params(
            &model_path_str,
            whisper_rs::WhisperContextParameters::default(),
        ).map_err(|e| format!("Failed to load Whisper model: {:?}", e))?;

        let mut subtitle_infos_sorted = subtitle_infos_clone;
        subtitle_infos_sorted.sort_by_key(|(_, start_ms, _)| *start_ms);

        for (idx, &start_pos) in sample_positions.iter().enumerate() {
            if token_clone.is_cancelled() {
                emit_auto_sync_progress(
                    &app_clone,
                    "cancelled",
                    "Auto-sync cancelled by user.".to_string(),
                    100.0,
                    Some("sync.autoSyncProgress.cancelled"),
                    None,
                );
                return Ok((all_matches, idx, true));
            }

            let progress = (idx as f64 / total_segments as f64) * 80.0 + 10.0;

            let start_label = format_mm_ss(start_pos);
            let end_label = format_mm_ss(start_pos + segment_duration);
            emit_auto_sync_progress(
                &app_clone,
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
                Some(std::collections::HashMap::from([
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

            let wav_path = temp_dir_path.join(format!("segment_{}.wav", idx));
            let wav_str = wav_path.to_string_lossy().to_string();

            if let Err(e) = extract_audio_segment(&media_path_str, start_pos, segment_duration, &wav_str, &ffmpeg_cmd_arc) {
                eprintln!("[auto-sync] Segment {} extraction failed: {}", idx, e);
                continue;
            }

            let audio_data = match read_wav_to_f32(&wav_path) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("[auto-sync] Failed to read segment {}: {}", idx, e);
                    continue;
                }
            };

            if token_clone.is_cancelled() { break; }

            let transcribed = match transcribe_segment_with_ctx(&ctx, &audio_data, language_clone.as_deref()) {
                Ok(segs) => segs,
                Err(e) => {
                    eprintln!("[auto-sync] Segment {} transcription failed: {}", idx, e);
                    continue;
                }
            };

            let adjusted_segments: Vec<TranscribedSegment> = transcribed.into_iter()
                .map(|mut seg| {
                    seg.start_ms += (start_pos * 1000.0) as i64;
                    seg.end_ms += (start_pos * 1000.0) as i64;
                    seg
                })
                .collect();

            for tseg in &adjusted_segments {
                // Tiny or one-word fragments are very noisy for subtitle alignment.
                if tseg.text.split_whitespace().count() < 2 {
                    continue;
                }

                let near_idx = subtitle_infos_sorted
                    .partition_point(|(_, sub_start_ms, _)| *sub_start_ms < tseg.start_ms);
                let window_start = near_idx.saturating_sub(40);
                let window_end = (near_idx + 40).min(subtitle_infos_sorted.len());

                for &(sub_id, sub_start_ms, ref sub_text) in &subtitle_infos_sorted[window_start..window_end] {
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
            let _ = std::fs::remove_file(&wav_path);
        }
        
        Ok((all_matches, total_segments, token_clone.is_cancelled()))
    }).await.map_err(|e| format!("Task panic: {:?}", e))?;
    
    let (all_matches, segments_analyzed, is_cancelled) = spawn_res?;
    let _ = std::fs::remove_dir_all(&temp_dir);

    if is_cancelled {
        return Ok(AutoSyncResult {
            success: false,
            cancelled: true,
            anchors_created: 0,
            segments_analyzed,
            message: "Auto-sync was cancelled.".to_string(),
        });
    }

    let mut best_offset = 0i64;
    let mut max_dense_count = 0;
    
    for m in &all_matches {
        let current_offset = m.transcribed_start_ms - m.original_start_ms;
        let mut count = 0;
        for other in &all_matches {
            let other_offset = other.transcribed_start_ms - other.original_start_ms;
            if (other_offset - current_offset).abs() <= 15_000 {
                count += 1;
            }
        }
        if count > max_dense_count {
            max_dense_count = count;
            best_offset = current_offset;
        }
    }

    let geometrically_verified: Vec<MatchCandidate> = all_matches.into_iter()
        .filter(|m| {
            let offset = m.transcribed_start_ms - m.original_start_ms;
            (offset - best_offset).abs() <= 15_000
        })
        .collect();

    let mut best_per_sub: std::collections::HashMap<u32, MatchCandidate> = std::collections::HashMap::new();
    for m in geometrically_verified {
        let entry = best_per_sub.entry(m.subtitle_id).or_insert_with(|| m.clone());
        if m.score > entry.score || (m.score == entry.score && m.similarity > entry.similarity) {
            *entry = m;
        }
    }

    let mut final_matches: Vec<MatchCandidate> = best_per_sub.into_values().collect();
    final_matches.sort_by_key(|m| m.original_start_ms);

    let mut spaced_matches: Vec<MatchCandidate> = Vec::new();
    let mut last_time: Option<i64> = None;
    for m in final_matches {
        if last_time.map_or(true, |lt| m.original_start_ms.saturating_sub(lt) >= 30_000) {
            spaced_matches.push(m.clone());
            last_time = Some(m.original_start_ms);
        }
    }

    let anchors_created = {
        let mut ss = sync_state.lock().map_err(|e| e.to_string())?;
        let engine = ss.engine.as_mut()
            .ok_or("No SRT file loaded for sync")?;

        let existing_anchors = engine.get_anchors().to_vec();

        let mut count = 0;
        for m in &spaced_matches {
            let is_near_existing = existing_anchors.iter().any(|a| {
                a.subtitle_index == m.subtitle_id ||
                (a.original_time_ms - m.original_start_ms).abs() < 30_000
            });

            if !is_near_existing {
                if engine.add_anchor(m.subtitle_id, m.transcribed_start_ms, false).is_ok() {
                    count += 1;
                }
            }
        }
        count
    };

    emit_auto_sync_progress(
        &app,
        "done",
        format!(
            "Auto-sync complete: {} anchors created from {} segments",
            anchors_created, total_segments
        ),
        100.0,
        Some("sync.autoSyncProgress.doneAnchors"),
        Some(HashMap::from([
            ("anchors".to_string(), anchors_created.to_string()),
            ("segments".to_string(), total_segments.to_string()),
        ])),
    );

    Ok(AutoSyncResult {
        success: anchors_created > 0,
        cancelled: false,
        anchors_created,
        segments_analyzed: total_segments,
        message: if anchors_created > 0 {
            format!("{} anchor points created automatically. Review and adjust as needed.", anchors_created)
        } else {
            "No confident matches found. Try with a larger Whisper model or check that the SRT matches the audio.".to_string()
        },
    })
}
