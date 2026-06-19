//! Auto-sync command: uses Whisper to transcribe audio segments and
//! automatically create anchor points by matching transcribed text
//! against SRT subtitle text.

use anyhow::{Context as _, Result};
use serde::Serialize;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use whisper_common::transcribe::{TranscribedSegment, TranscribeOptions, transcribe_full, text_similarity};
use whisper_common::audio::read_wav_to_f32;

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

/// Helper function to check if audio samples are silent (RMS under threshold)
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

struct PreparedSegment {
    idx: usize,
    current_pos: f64,
    audio_data: Vec<f32>,
    _wav_path: std::path::PathBuf,
}

// Worker per un singolo segmento del fan-out di auto-sync: i parametri sono
// indipendenti (posizioni, durate, path, comando ffmpeg) e la funzione è interna,
// quindi raggrupparli in una struct aggiungerebbe cerimonia senza chiarezza.
#[allow(clippy::too_many_arguments)]
async fn prepare_single_segment(
    idx: usize,
    start_pos: f64,
    segment_duration: f64,
    duration_sec: f64,
    quick: bool,
    media_path: String,
    temp_dir_path: std::path::PathBuf,
    ffmpeg_cmd: String,
) -> Result<PreparedSegment, String> {
    let mut current_pos = start_pos;
    let mut attempts = 0;
    let mut audio_data = Vec::new();
    let mut wav_path = temp_dir_path.join(format!("segment_{}.wav", idx));

    let max_attempts = if quick { 5 } else { 3 };
    let shift_amount = if quick { 15.0 } else { 20.0 };

    while attempts < max_attempts && current_pos + segment_duration <= duration_sec {
        let temp_wav_path = temp_dir_path.join(format!("segment_{}_try{}.wav", idx, attempts));
        let temp_wav_str = temp_wav_path.to_string_lossy().to_string();

        let media_path_clone = media_path.clone();
        let ffmpeg_cmd_clone = ffmpeg_cmd.clone();
        let extract_res = tokio::task::spawn_blocking(move || {
            extract_audio_segment(&media_path_clone, current_pos, segment_duration, &temp_wav_str, &ffmpeg_cmd_clone)
        }).await;

        let extract_ok = matches!(extract_res, Ok(Ok(())));

        if !extract_ok {
            attempts += 1;
            current_pos += shift_amount;
            continue;
        }

        let temp_wav_path_clone = temp_wav_path.clone();
        let samples_res = tokio::task::spawn_blocking(move || {
            read_wav_to_f32(&temp_wav_path_clone)
        }).await;

        let samples = match samples_res {
            Ok(Ok(data)) => data,
            _ => {
                let _ = std::fs::remove_file(&temp_wav_path);
                attempts += 1;
                current_pos += shift_amount;
                continue;
            }
        };

        let silent = is_silent(&samples, 0.003);
        if silent {
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
            extract_audio_segment(&media_path_clone, start_pos, segment_duration, &wav_str, &ffmpeg_cmd_clone)
        }).await;

        let wav_path_clone = wav_path.clone();
        if let Ok(Ok(samples)) = tokio::task::spawn_blocking(move || read_wav_to_f32(&wav_path_clone)).await {
            audio_data = samples;
            current_pos = start_pos;
        }
    }

    Ok(PreparedSegment {
        idx,
        current_pos,
        audio_data,
        _wav_path: wav_path,
    })
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
    quick: bool,
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
 
    // Determine segment duration and sample positions based on quick mode (quick = BREVE, false = PRECISO)
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
    
    // Concurrently prepare all segments using a CPU-bounded semaphore to maximize I/O throughput safely
    emit_auto_sync_progress(
        &app,
        "prepare",
        format!("Preparing and extracting {} audio segments in parallel...", total_segments),
        3.0,
        Some("sync.autoSyncProgress.preparingSegments"),
        Some(HashMap::from([
            ("total".to_string(), total_segments.to_string()),
        ])),
    );
 
    let max_concurrency = num_cpus().min(6);
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrency));
    let mut prep_handles = Vec::new();
 
    for (idx, &start_pos) in sample_positions.iter().enumerate() {
        let sem = semaphore.clone();
        let media_path = media_path_str.clone();
        let temp_dir_path = temp_dir_path.clone();
        let ffmpeg_cmd = ffmpeg_cmd.clone();
 
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
            ).await
        }));
    }
 
    let mut prepared_segments = Vec::new();
    for (idx, handle) in prep_handles.into_iter().enumerate() {
        if token.is_cancelled() {
            let _ = std::fs::remove_dir_all(&temp_dir_path);
            return Ok(AutoSyncResult {
                success: false,
                cancelled: true,
                anchors_created: 0,
                segments_analyzed: idx,
                message: "Auto-sync was cancelled.".to_string(),
            });
        }
        match handle.await {
            Ok(Ok(prep)) => prepared_segments.push(prep),
            Ok(Err(e)) => {
                eprintln!("[auto-sync] Segment {} preparation failed: {}", idx, e);
            }
            Err(e) => {
                eprintln!("[auto-sync] Segment {} task panicked: {:?}", idx, e);
            }
        }
    }
 
    // Sort chronologically to preserve geometric alignment sequence
    prepared_segments.sort_by_key(|s| s.idx);
 
    emit_auto_sync_progress(
        &app,
        "prepare_done",
        "Audio preparation complete. Loading Whisper model...".to_string(),
        10.0,
        Some("sync.autoSyncProgress.loadingModel"),
        None,
    );
 
    let spawn_res = tokio::task::spawn_blocking(move || -> Result<(Vec<MatchCandidate>, usize, bool), String> {
        let mut all_matches: Vec<MatchCandidate> = Vec::new();
        
        let ctx = whisper_rs::WhisperContext::new_with_params(
            &model_path_str,
            whisper_rs::WhisperContextParameters::default(),
        ).map_err(|e| format!("Failed to load Whisper model: {:?}", e))?;
 
        let mut subtitle_infos_sorted = subtitle_infos_clone;
        subtitle_infos_sorted.sort_by_key(|(_, start_ms, _)| *start_ms);
 
        for (idx, prep) in prepared_segments.iter().enumerate() {
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
 
            let start_label = format_mm_ss(prep.current_pos);
            let end_label = format_mm_ss(prep.current_pos + segment_duration);
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
 
            if token_clone.is_cancelled() {
                break;
            }
 
            let options = TranscribeOptions {
                language: language_clone.clone(),
                translate_to_english: false,
                n_threads: None,
                word_timestamps: true,
                max_segment_length: None,
                segment_callback: None,
            };

            let (transcribed, _) = match transcribe_full(&ctx, &prep.audio_data, &options, Some(&token_clone)) {
                Ok(segs) => segs,
                Err(e) => {
                    eprintln!("[auto-sync] Segment {} transcription failed: {}", prep.idx, e);
                    continue;
                }
            };
 
            let adjusted_segments: Vec<TranscribedSegment> = transcribed.into_iter()
                .map(|mut seg| {
                    seg.start_ms += (prep.current_pos * 1000.0) as i64;
                    seg.end_ms += (prep.current_pos * 1000.0) as i64;
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
        }
        
        Ok((all_matches, total_segments, token_clone.is_cancelled()))
    }).await.map_err(|e| format!("Task panic: {:?}", e))?;
    
    let (all_matches, segments_analyzed, is_cancelled) = spawn_res?;
    let _ = std::fs::remove_dir_all(&temp_dir_path);
 
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
        if last_time.is_none_or(|lt| m.original_start_ms.saturating_sub(lt) >= 30_000) {
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
                (a.original_time_ms - m.original_start_ms).abs() < 10_000
                    || (a.corrected_time_ms - m.transcribed_start_ms).abs() < 10_000
            });
 
            if !is_near_existing
                && engine.add_anchor(m.subtitle_id, m.transcribed_start_ms, false).is_ok()
            {
                count += 1;
            }
        }
        
        count
    };
 
    if anchors_created == 0 {
        emit_auto_sync_progress(
            &app,
            "done",
            "No confident matches found. Try with a larger Whisper model or check that the SRT matches the audio.".to_string(),
            100.0,
            Some("sync.autoSyncProgress.noMatches"),
            None,
        );
        return Ok(AutoSyncResult {
            success: false,
            cancelled: false,
            anchors_created: 0,
            segments_analyzed,
            message: "No confident matches found. Try with a larger Whisper model or check that the SRT matches the audio.".to_string(),
        });
    }
 
    emit_auto_sync_progress(
        &app,
        "done",
        format!("Auto-sync complete! Created {} new visual anchor points.", anchors_created),
        100.0,
        Some("sync.autoSyncProgress.done"),
        Some(HashMap::from([
            ("count".to_string(), anchors_created.to_string()),
        ])),
    );
 
    Ok(AutoSyncResult {
        success: true,
        cancelled: false,
        anchors_created,
        segments_analyzed,
        message: format!("Successfully created {} new visual anchor points.", anchors_created),
    })
}
