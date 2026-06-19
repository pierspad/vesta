//! Comandi Tauri per la trascrizione audio/video tramite whisper-rs.
//!
//! Implementazione nativa: download modelli, trascrizione via whisper-rs (bindings Rust per whisper.cpp),
//! generazione file SRT di output con segmentazione intelligente.

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use whisper_common::model::{list_models, uninstall_model, WhisperModelInfo};
use whisper_common::audio::{convert_to_wav, read_wav_to_f32, segment_to_wav_chunks};
use whisper_common::transcribe::{transcribe_full, TranscribeOptions, TranscribedSegment};
use whisper_common::cloud::{transcribe_chunk, CloudConfig};

use crate::state::AppTranscribeState;

// ─── Data Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct TranscribeConfig {
    pub input_path: String,
    pub output_path: String,
    pub model: String,
    pub language: String,
    pub translate_to_english: bool,
    pub word_timestamps: bool,
    pub max_segment_length: u32,
    /// Motore di trascrizione: "local" (default, whisper.cpp) oppure un provider
    /// cloud ("groq" | "openai" | "deepgram" | "assemblyai" | "custom").
    #[serde(default)]
    pub provider: Option<String>,
    /// API key per i provider cloud.
    #[serde(default)]
    pub api_key: Option<String>,
    /// Base URL opzionale (override / richiesto per "custom").
    #[serde(default)]
    pub api_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscribeResult {
    pub success: bool,
    pub message: String,
    pub output_path: Option<String>,
    pub subtitle_count: usize,
    pub detected_language: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscribeProgressEvent {
    pub stage: String,
    pub message: String,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscribeSegmentEvent {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Download Whisper model from Hugging Face
async fn download_model(
    app: &AppHandle,
    model_id: &str,
    cancel_token: &CancellationToken,
) -> Result<PathBuf> {
    app.emit("transcribe-progress", TranscribeProgressEvent {
        stage: "download".to_string(),
        message: format!("Downloading model {} ...", model_id),
        percentage: 0.0,
    }).ok();

    let app_progress = app.clone();
    let model_id_progress = model_id.to_string();

    whisper_common::model::download_model(
        model_id,
        move |percentage| {
            app_progress.emit("transcribe-progress", TranscribeProgressEvent {
                stage: "download".to_string(),
                message: format!("Downloading model {} ({}%)...", model_id_progress, percentage),
                percentage: percentage as f64,
            }).ok();
        },
        Some(cancel_token),
    ).await
}

/// A raw segment from whisper
#[derive(Debug, Clone)]
struct RawSegment {
    start_ms: i64,
    end_ms: i64,
    text: String,
}

/// Post-process segments: merge very short ones, split excessively long ones
fn postprocess_segments(raw: Vec<RawSegment>, max_segment_len: u32) -> Vec<RawSegment> {
    if raw.is_empty() {
        return raw;
    }
    
    let max_chars = if max_segment_len > 0 { max_segment_len as usize } else { 80 };
    
    // Phase 1: Merge segments that are very short (< 1s) or have very little text (< 5 chars)
    let mut merged: Vec<RawSegment> = Vec::new();
    for seg in raw {
        let text = seg.text.trim().to_string();
        if text.is_empty() {
            continue;
        }
        
        let duration_ms = seg.end_ms - seg.start_ms;
        let should_merge = !merged.is_empty() && (
            duration_ms < 1000 && text.len() < 10 ||
            text.len() < 3
        );
        
        if should_merge {
            let last = merged.last_mut().unwrap();
            last.end_ms = seg.end_ms;
            last.text = format!("{} {}", last.text, text);
        } else {
            merged.push(RawSegment {
                start_ms: seg.start_ms,
                end_ms: seg.end_ms,
                text,
            });
        }
    }
    
    // Phase 2: Split segments that exceed max_chars at sentence boundaries
    let mut result: Vec<RawSegment> = Vec::new();
    for seg in merged {
        if seg.text.len() <= max_chars {
            result.push(seg);
            continue;
        }
        
        // Try to split at sentence boundaries (. ! ? ; :)
        let total_duration = seg.end_ms - seg.start_ms;
        let text = &seg.text;
        let total_chars = text.len();
        
        let mut splits: Vec<usize> = Vec::new();
        let mut last_split = 0;
        
        for (i, c) in text.char_indices() {
            if (c == '.' || c == '!' || c == '?' || c == ';') && i > last_split + 10 {
                // Check if next char is space or end of string (to avoid splitting "3.5")
                let next_char = text[i+c.len_utf8()..].chars().next();
                if next_char.is_none_or(|nc| nc == ' ' || nc.is_uppercase()) {
                    splits.push(i + c.len_utf8());
                    last_split = i + c.len_utf8();
                }
            }
        }
        
        if splits.is_empty() {
            // No good split point found, keep as-is
            result.push(seg);
            continue;
        }
        
        // Create sub-segments with proportional timestamps
        let mut prev_pos = 0;
        for (idx, split_pos) in splits.iter().enumerate() {
            let sub_text = text[prev_pos..*split_pos].trim().to_string();
            if sub_text.is_empty() {
                prev_pos = *split_pos;
                continue;
            }
            
            let ratio_start = prev_pos as f64 / total_chars as f64;
            let ratio_end = *split_pos as f64 / total_chars as f64;
            let sub_start = seg.start_ms + (ratio_start * total_duration as f64) as i64;
            let sub_end = seg.start_ms + (ratio_end * total_duration as f64) as i64;
            
            result.push(RawSegment {
                start_ms: sub_start,
                end_ms: sub_end,
                text: sub_text,
            });
            prev_pos = *split_pos;
            
            // Handle last remainder
            if idx == splits.len() - 1 && prev_pos < total_chars {
                let remainder = text[prev_pos..].trim().to_string();
                if !remainder.is_empty() {
                    result.push(RawSegment {
                        start_ms: sub_end,
                        end_ms: seg.end_ms,
                        text: remainder,
                    });
                }
            }
        }
    }
    
    result
}

/// Format milliseconds as SRT timestamp HH:MM:SS,mmm
fn ms_to_srt_timestamp(ms: i64) -> String {
    let ms = ms.max(0);
    let total_secs = ms / 1000;
    let millis = ms % 1000;
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = total_secs / 3600;
    format!("{:02}:{:02}:{:02},{:03}", hours, mins, secs, millis)
}

/// Write segments as SRT file
fn write_srt(segments: &[RawSegment], output_path: &str) -> Result<usize> {
    let mut file = std::fs::File::create(output_path)
        .context(format!("Cannot create output file: {}", output_path))?;
    
    for (i, seg) in segments.iter().enumerate() {
        writeln!(file, "{}", i + 1)?;
        writeln!(file, "{} --> {}", ms_to_srt_timestamp(seg.start_ms), ms_to_srt_timestamp(seg.end_ms))?;
        writeln!(file, "{}", seg.text.trim())?;
        writeln!(file)?;
    }
    
    Ok(segments.len())
}

fn apply_language_suffix_to_srt_path(output_path: &str, language: &str) -> String {
    let lang = language.trim().to_lowercase();
    if lang.is_empty() {
        return output_path.to_string();
    }

    let path = Path::new(output_path);
    let Some(file_name_os) = path.file_name() else {
        return output_path.to_string();
    };

    let file_name = file_name_os.to_string_lossy();
    if !file_name.to_lowercase().ends_with(".srt") {
        return output_path.to_string();
    }

    let stem = &file_name[..file_name.len() - 4];
    let mut replaced = false;
    let mut new_stem = stem.to_string();

    if let Some(idx) = stem.rfind(['-', '_', '.']) {
        let token = &stem[idx + 1..];
        let is_lang_like = (token.len() == 2 || token.len() == 3)
            && token.chars().all(|c| c.is_ascii_alphabetic());
        if is_lang_like {
            new_stem = format!("{}{}{}", &stem[..idx], &stem[idx..=idx], lang);
            replaced = true;
        }
    }

    if !replaced {
        new_stem = format!("{}.{}", stem, lang);
    }

    let new_file_name = format!("{}.srt", new_stem);
    let mut new_path = path.parent().map_or_else(PathBuf::new, PathBuf::from);
    new_path.push(new_file_name);
    new_path.to_string_lossy().to_string()
}

/// Run transcription using whisper-rs (native Rust bindings for whisper.cpp)
fn run_whisper_rs(
    app: &AppHandle,
    config: &TranscribeConfig,
    model_path: &Path,
    audio_data: &[f32],
    cancel_token: &CancellationToken,
) -> Result<TranscribeResult> {
    app.emit("transcribe-progress", TranscribeProgressEvent {
        stage: "transcribe".to_string(),
        message: "Loading Whisper model...".to_string(),
        percentage: 20.0,
    }).ok();
    
    let model_path_str = model_path.to_string_lossy().to_string();
    let ctx = whisper_rs::WhisperContext::new_with_params(
        &model_path_str,
        whisper_rs::WhisperContextParameters::default(),
    ).map_err(|e| anyhow::anyhow!("Failed to load Whisper model: {:?}", e))?;
    
    if cancel_token.is_cancelled() {
        anyhow::bail!("Transcription cancelled");
    }
    
    app.emit("transcribe-progress", TranscribeProgressEvent {
        stage: "transcribe".to_string(),
        message: "Transcribing audio...".to_string(),
        percentage: 30.0,
    }).ok();
    
    let app_for_callback = app.clone();
    let segment_callback = move |start_ms: i64, end_ms: i64, text: &str| {
        let _ = app_for_callback.emit("transcribe-segment", TranscribeSegmentEvent {
            start_ms,
            end_ms,
            text: text.to_string(),
        });
    };

    let options = TranscribeOptions {
        language: if config.language != "auto" { Some(config.language.clone()) } else { None },
        translate_to_english: config.translate_to_english,
        n_threads: None,
        word_timestamps: config.word_timestamps,
        max_segment_length: if config.max_segment_length > 0 { Some(config.max_segment_length) } else { None },
        segment_callback: Some(std::sync::Arc::new(segment_callback)),
    };
    
    let (raw_segments, detected_language) = transcribe_full(&ctx, audio_data, &options, Some(cancel_token))?;
    
    let raw: Vec<RawSegment> = raw_segments.into_iter()
        .map(|s| RawSegment {
            start_ms: s.start_ms,
            end_ms: s.end_ms,
            text: s.text,
        })
        .collect();
    
    // Post-process segments for better SRT formatting
    let segments = postprocess_segments(raw, config.max_segment_length);
    
    app.emit("transcribe-progress", TranscribeProgressEvent {
        stage: "writing".to_string(),
        message: if let Some(lang) = &detected_language {
            format!("Writing SRT file (language: {})...", lang)
        } else {
            "Writing SRT file...".to_string()
        },
        percentage: 90.0,
    }).ok();
    
    let effective_output_path = if config.language == "auto" {
        if let Some(lang) = &detected_language {
            apply_language_suffix_to_srt_path(&config.output_path, lang)
        } else {
            config.output_path.clone()
        }
    } else {
        config.output_path.clone()
    };

    // Write SRT
    let count = write_srt(&segments, &effective_output_path)?;
    
    Ok(TranscribeResult {
        success: true,
        message: format!("Transcription completed: {} segments", count),
        output_path: Some(effective_output_path),
        subtitle_count: count,
        detected_language,
    })
}

/// Segment length (seconds) for cloud chunking. ~8 min of 16kHz mono WAV is
/// well under every provider's upload limit (~15 MB).
const CLOUD_CHUNK_SECONDS: i64 = 480;

/// Run transcription through a cloud provider: split the audio into chunks,
/// transcribe each (offsetting timestamps), then post-process and write SRT.
async fn run_cloud(
    app: &AppHandle,
    config: &TranscribeConfig,
    cancel_token: &CancellationToken,
) -> Result<TranscribeResult> {
    let provider = config.provider.clone().unwrap_or_else(|| "local".to_string());

    app.emit("transcribe-progress", TranscribeProgressEvent {
        stage: "preparing".to_string(),
        message: "Preparing audio...".to_string(),
        percentage: 5.0,
    }).ok();

    let ffmpeg_cmd = crate::commands::flashcards::media::resolve_ffmpeg_path(Some(app)).await;

    let tmp_dir = tempfile::Builder::new()
        .prefix("vesta_cloud_")
        .tempdir()
        .context("Failed to create temp dir for cloud transcription")?;

    let chunks = segment_to_wav_chunks(
        &ffmpeg_cmd,
        Path::new(&config.input_path),
        tmp_dir.path(),
        CLOUD_CHUNK_SECONDS as u32,
        Some(cancel_token),
    )
    .await
    .context("Audio segmentation failed")?;

    let cloud_cfg = CloudConfig {
        provider: provider.clone(),
        api_key: config.api_key.clone().unwrap_or_default(),
        api_url: config.api_url.clone(),
        model: config.model.clone(),
        language: if config.language != "auto" { Some(config.language.clone()) } else { None },
        translate_to_english: config.translate_to_english,
    };

    if cloud_cfg.api_key.trim().is_empty() {
        anyhow::bail!("Missing API key for cloud provider '{}'", provider);
    }

    let client = whisper_common::cloud::default_client();

    let total = chunks.len();
    let mut all: Vec<TranscribedSegment> = Vec::new();

    for (idx, chunk_path) in chunks.iter().enumerate() {
        if cancel_token.is_cancelled() {
            anyhow::bail!("Transcription cancelled");
        }

        let pct = 10.0 + (idx as f64 / total.max(1) as f64) * 80.0;
        app.emit("transcribe-progress", TranscribeProgressEvent {
            stage: "transcribe".to_string(),
            message: format!("Transcribing chunk {}/{} via {}...", idx + 1, total, provider),
            percentage: pct,
        }).ok();

        let bytes = std::fs::read(chunk_path)
            .with_context(|| format!("Failed to read audio chunk {}", idx + 1))?;
        let offset_ms = idx as i64 * CLOUD_CHUNK_SECONDS * 1000;

        let segs = transcribe_chunk(&client, &cloud_cfg, bytes, "audio.wav")
            .await
            .with_context(|| format!("Cloud transcription failed on chunk {}", idx + 1))?;

        for mut s in segs {
            s.start_ms += offset_ms;
            s.end_ms += offset_ms;
            let _ = app.emit("transcribe-segment", TranscribeSegmentEvent {
                start_ms: s.start_ms,
                end_ms: s.end_ms,
                text: s.text.clone(),
            });
            all.push(s);
        }
    }

    all.sort_by_key(|s| s.start_ms);

    let raw: Vec<RawSegment> = all
        .into_iter()
        .map(|s| RawSegment { start_ms: s.start_ms, end_ms: s.end_ms, text: s.text })
        .collect();
    let segments = postprocess_segments(raw, config.max_segment_length);

    app.emit("transcribe-progress", TranscribeProgressEvent {
        stage: "writing".to_string(),
        message: "Writing SRT file...".to_string(),
        percentage: 92.0,
    }).ok();

    let count = write_srt(&segments, &config.output_path)?;

    let detected_language = if config.language != "auto" {
        Some(config.language.clone())
    } else {
        None
    };

    Ok(TranscribeResult {
        success: true,
        message: format!("Transcription completed: {} segments", count),
        output_path: Some(config.output_path.clone()),
        subtitle_count: count,
        detected_language,
    })
}

// ─── Tauri Commands ──────────────────────────────────────────────────────────

/// Check what Whisper backends are available
#[tauri::command]
pub async fn transcribe_check_backends(app: AppHandle) -> Result<serde_json::Value, String> {
    let ffmpeg_cmd = crate::commands::flashcards::media::resolve_ffmpeg_path(Some(&app)).await;
    let ffmpeg_available = tokio::process::Command::new(&ffmpeg_cmd)
        .arg("-version")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);
    
    // whisper-rs is always available (compiled natively)
    Ok(serde_json::json!({
        "ffmpeg": ffmpeg_available,
        "whisper_cpp": true,
        "python_whisper": false,
        "whisper_binary": "whisper-rs (native)",
        "any_whisper": true,
    }))
}

/// Get list of models with their download status
#[tauri::command]
pub async fn transcribe_list_models() -> Result<Vec<WhisperModelInfo>, String> {
    list_models().map_err(|e| e.to_string())
}

/// Download a specific Whisper model
#[tauri::command]
pub async fn transcribe_download_model(
    app: AppHandle,
    state: State<'_, AppTranscribeState>,
    model_id: String,
) -> Result<bool, String> {
    let cancel_token = {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        let token = CancellationToken::new();
        s.cancellation_token = Some(token.clone());
        token
    };
    
    match download_model(&app, &model_id, &cancel_token).await {
        Ok(_) => Ok(true),
        Err(e) => Err(e.to_string()),
    }
}

/// Uninstall (delete) a specific Whisper model
#[tauri::command]
pub async fn transcribe_uninstall_model(
    model_id: String,
) -> Result<bool, String> {
    uninstall_model(&model_id).map_err(|e| e.to_string())
}

/// Start transcription
#[tauri::command]
pub async fn transcribe_start(
    app: AppHandle,
    state: State<'_, AppTranscribeState>,
    config: TranscribeConfig,
) -> Result<TranscribeResult, String> {
    // Check if already transcribing
    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        if s.is_transcribing {
            return Err("Transcription already in progress".to_string());
        }
        s.is_transcribing = true;
        let token = CancellationToken::new();
        s.cancellation_token = Some(token.clone());
    }
    
    let cancel_token = {
        let s = state.lock().map_err(|e| e.to_string())?;
        s.cancellation_token.clone().unwrap()
    };
    
    // Verify input file exists
    if !Path::new(&config.input_path).exists() {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.is_transcribing = false;
        return Err(format!("Input file not found: {}", config.input_path));
    }

    // ── Cloud provider path ───────────────────────────────────────────────────
    let provider = config.provider.clone().unwrap_or_else(|| "local".to_string());
    let is_cloud = !matches!(provider.to_lowercase().as_str(), "local" | "whisper" | "");
    if is_cloud {
        let result = run_cloud(&app, &config, &cancel_token).await;
        {
            let mut s = state.lock().map_err(|e| e.to_string())?;
            s.is_transcribing = false;
        }
        return match result {
            Ok(r) => {
                app.emit("transcribe-progress", TranscribeProgressEvent {
                    stage: "done".to_string(),
                    message: r.message.clone(),
                    percentage: 100.0,
                }).ok();
                Ok(r)
            }
            Err(e) => Err(e.to_string()),
        };
    }

    // ── Local whisper.cpp path ────────────────────────────────────────────────
    // Download model if needed
    let model_path = match download_model(&app, &config.model, &cancel_token).await {
        Ok(p) => p,
        Err(e) => {
            let mut s = state.lock().map_err(|e2| e2.to_string())?;
            s.is_transcribing = false;
            return Err(format!("Model download failed: {}", e));
        }
    };
    
    // Convert input to WAV using the common audio module!
    let temp_wav = tempfile::Builder::new()
        .suffix(".wav")
        .tempfile()
        .map_err(|e| format!("Failed to create temp WAV file: {}", e))?;
    let wav_path = temp_wav.into_temp_path().to_path_buf();
    
    let ffmpeg_cmd = crate::commands::flashcards::media::resolve_ffmpeg_path(Some(&app)).await;

    match convert_to_wav(&ffmpeg_cmd, Path::new(&config.input_path), &wav_path, Some(&cancel_token)).await {
        Ok(_) => {}
        Err(e) => {
            let mut s = state.lock().map_err(|e2| e2.to_string())?;
            s.is_transcribing = false;
            return Err(format!("Audio conversion failed: {}", e));
        }
    }
    
    // Read audio data using the common audio module!
    let audio_data = match read_wav_to_f32(&wav_path) {
        Ok(data) => data,
        Err(e) => {
            let mut s = state.lock().map_err(|e2| e2.to_string())?;
            s.is_transcribing = false;
            let _ = std::fs::remove_file(&wav_path);
            return Err(format!("Failed to read audio: {}", e));
        }
    };
    
    // Run whisper-rs transcription (blocking, run on blocking thread)
    let app_clone = app.clone();
    let config_clone = config.clone();
    let cancel_clone = cancel_token.clone();
    
    let result = tokio::task::spawn_blocking(move || {
        run_whisper_rs(&app_clone, &config_clone, &model_path, &audio_data, &cancel_clone)
    })
    .await
    .map_err(|e| format!("Transcription task failed: {}", e))?;
    
    // Clean up temp WAV
    let _ = std::fs::remove_file(&wav_path);
    
    // Update state
    {
        let mut s = state.lock().map_err(|e| e.to_string())?;
        s.is_transcribing = false;
    }
    
    match result {
        Ok(r) => {
            app.emit("transcribe-progress", TranscribeProgressEvent {
                stage: "done".to_string(),
                message: r.message.clone(),
                percentage: 100.0,
            }).ok();
            Ok(r)
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Cancel ongoing transcription
#[tauri::command]
pub async fn transcribe_cancel(
    state: State<'_, AppTranscribeState>,
) -> Result<(), String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    if let Some(token) = s.cancellation_token.take() {
        token.cancel();
    }
    s.is_transcribing = false;
    Ok(())
}

/// Check if a file exists at the given path  
#[tauri::command]
pub async fn transcribe_check_file_exists(path: String) -> bool {
    Path::new(&path).exists()
}
