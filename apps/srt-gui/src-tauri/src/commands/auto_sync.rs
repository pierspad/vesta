//! Auto-sync command: uses Whisper to transcribe audio segments and
//! automatically create anchor points by matching transcribed text
//! against SRT subtitle text.

use anyhow::{Context as _, Result};
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, Emitter, State};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
use tokio_util::sync::CancellationToken;

use crate::state::{AppSyncState, AppTranscribeState};

/// Progress event for auto-sync
#[derive(Debug, Clone, Serialize)]
pub struct AutoSyncProgressEvent {
    pub stage: String,
    pub message: String,
    pub percentage: f64,
}

/// Result of auto-sync
#[derive(Debug, Clone, Serialize)]
pub struct AutoSyncResult {
    pub success: bool,
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

/// Normalized string similarity using longest common subsequence ratio (Word level)
fn text_similarity(a: &str, b: &str) -> f64 {
    let a_clean = normalize_text(a);
    let b_clean = normalize_text(b);

    if a_clean.is_empty() || b_clean.is_empty() {
        return 0.0;
    }

    let a_words: Vec<&str> = a_clean.split_whitespace().collect();
    let b_words: Vec<&str> = b_clean.split_whitespace().collect();
    let m = a_words.len();
    let n = b_words.len();

    if m == 0 || n == 0 {
        return 0.0;
    }

    // LCS via DP
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if a_words[i - 1] == b_words[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    let lcs_len = dp[m][n] as f64;
    // Dice coefficient for words
    2.0 * lcs_len / (m + n) as f64
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
async fn extract_audio_segment(
    media_path: &str,
    start_sec: f64,
    duration_sec: f64,
    output_wav: &str,
) -> Result<()> {
    let output = tokio::process::Command::new("ffmpeg")
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
        .await
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

/// Transcribe an audio segment using whisper-rs
fn transcribe_segment(
    model_path: &Path,
    audio_data: &[f32],
    language: Option<&str>,
) -> Result<Vec<TranscribedSegment>> {
    let ctx = WhisperContext::new_with_params(
        model_path.to_str().unwrap(),
        WhisperContextParameters::default(),
    ).map_err(|e| anyhow::anyhow!("Failed to load Whisper model: {:?}", e))?;

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
async fn get_media_duration(media_path: &str) -> Result<f64> {
    let output = tokio::process::Command::new("ffprobe")
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

    let ffmpeg_ok = tokio::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !ffmpeg_ok {
        return Err("FFmpeg is required for auto-sync. Install FFmpeg first.".to_string());
    }

    let duration_sec = get_media_duration(&media_path).await.unwrap_or(0.0);
    if duration_sec < 10.0 {
        return Err("Media file too short or unable to detect duration".to_string());
    }

    let segment_duration = 60.0;
    let num_samples = (duration_sec / 180.0).ceil().min(10.0).max(3.0) as usize;
    let step = duration_sec / (num_samples + 1) as f64;

    let mut sample_positions: Vec<f64> = Vec::new();
    for i in 1..=num_samples {
        let pos = step * i as f64;
        if pos + segment_duration <= duration_sec {
            sample_positions.push(pos);
        }
    }

    if sample_positions.is_empty() {
        sample_positions.push(0.0);
    }

    app.emit("sync-auto-progress", AutoSyncProgressEvent {
        stage: "start".to_string(),
        message: format!("Analyzing {} audio segments...", sample_positions.len()),
        percentage: 0.0,
    }).ok();

    let temp_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let mut all_matches: Vec<MatchCandidate> = Vec::new();
    let total_segments = sample_positions.len();

    for (idx, &start_pos) in sample_positions.iter().enumerate() {
        if token.is_cancelled() {
            app.emit("sync-auto-progress", AutoSyncProgressEvent {
                stage: "cancelled".to_string(),
                message: "Auto-sync cancelled by user".to_string(),
                percentage: 100.0,
            }).ok();
            return Ok(AutoSyncResult {
                success: false,
                anchors_created: 0,
                segments_analyzed: idx,
                message: "Auto-sync was cancelled.".to_string(),
            });
        }

        let progress = (idx as f64 / total_segments as f64) * 80.0 + 10.0;

        app.emit("sync-auto-progress", AutoSyncProgressEvent {
            stage: "transcribe".to_string(),
            message: format!("Transcribing segment {}/{} ({:.0}s - {:.0}s)...",
                idx + 1, total_segments, start_pos, start_pos + segment_duration),
            percentage: progress,
        }).ok();

        let wav_path = temp_dir.path().join(format!("segment_{}.wav", idx));
        let wav_str = wav_path.to_string_lossy().to_string();

        if let Err(e) = extract_audio_segment(&media_path, start_pos, segment_duration, &wav_str).await {
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

        if token.is_cancelled() { break; }

        let model_path_clone = model_path.clone();
        let lang_clone = language.clone();
        let transcribed = match tokio::task::spawn_blocking(move || {
            transcribe_segment(&model_path_clone, &audio_data, lang_clone.as_deref())
        }).await {
            Ok(Ok(segs)) => segs,
            _ => continue,
        };

        let adjusted_segments: Vec<TranscribedSegment> = transcribed.into_iter()
            .map(|mut seg| {
                seg.start_ms += (start_pos * 1000.0) as i64;
                seg.end_ms += (start_pos * 1000.0) as i64;
                seg
            })
            .collect();

        for tseg in &adjusted_segments {
            for &(sub_id, sub_start_ms, ref sub_text) in &subtitle_infos {
                let time_diff = (sub_start_ms - tseg.start_ms).abs();
                if time_diff > 120_000 {
                    continue;
                }

                let sim = text_similarity(&tseg.text, sub_text);
                if sim > 0.4 {
                    all_matches.push(MatchCandidate {
                        subtitle_id: sub_id,
                        original_start_ms: sub_start_ms,
                        transcribed_start_ms: tseg.start_ms,
                        similarity: sim,
                    });
                }
            }
        }
        let _ = std::fs::remove_file(&wav_path);
    }

    if token.is_cancelled() {
        return Ok(AutoSyncResult {
            success: false,
            anchors_created: 0,
            segments_analyzed: total_segments,
            message: "Auto-sync was cancelled.".to_string(),
        });
    }

    app.emit("sync-auto-progress", AutoSyncProgressEvent {
        stage: "matching".to_string(),
        message: format!("Found {} potential matches, selecting best anchors...", all_matches.len()),
        percentage: 90.0,
    }).ok();

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
        if m.similarity > entry.similarity {
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
                if engine.add_anchor(m.subtitle_id, m.transcribed_start_ms).is_ok() {
                    count += 1;
                }
            }
        }
        count
    };

    app.emit("sync-auto-progress", AutoSyncProgressEvent {
        stage: "done".to_string(),
        message: format!("Auto-sync complete: {} anchors created from {} segments",
            anchors_created, total_segments),
        percentage: 100.0,
    }).ok();

    Ok(AutoSyncResult {
        success: anchors_created > 0,
        anchors_created,
        segments_analyzed: total_segments,
        message: if anchors_created > 0 {
            format!("{} anchor points created automatically. Review and adjust as needed.", anchors_created)
        } else {
            "No confident matches found. Try with a larger Whisper model or check that the SRT matches the audio.".to_string()
        },
    })
}
