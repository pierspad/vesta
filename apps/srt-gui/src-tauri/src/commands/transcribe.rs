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
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::state::AppTranscribeState;

// ─── Data Types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhisperModelInfo {
    pub id: String,
    pub name: String,
    pub size: String,
    pub speed: String,
    pub downloaded: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TranscribeConfig {
    pub input_path: String,
    pub output_path: String,
    pub model: String,
    pub language: String,
    pub translate_to_english: bool,
    pub word_timestamps: bool,
    pub max_segment_length: u32,
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

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Get the directory where Whisper models are stored
fn get_models_dir() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".cache"));
    let models_dir = cache_dir.join("whisper");
    Ok(models_dir)
}

/// Check if a whisper model file exists locally
fn model_file_path(model_id: &str) -> Result<PathBuf> {
    let models_dir = get_models_dir()?;
    let filename = format!("ggml-{}.bin", model_id);
    Ok(models_dir.join(filename))
}

/// Download Whisper model from Hugging Face
async fn download_model(
    app: &AppHandle,
    model_id: &str,
    cancel_token: &CancellationToken,
) -> Result<PathBuf> {
    let model_path = model_file_path(model_id)?;
    
    // If already exists, return immediately
    if model_path.exists() {
        return Ok(model_path);
    }
    
    // Create directory if needed
    if let Some(parent) = model_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let url = format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
        model_id
    );
    
    app.emit("transcribe-progress", TranscribeProgressEvent {
        stage: "download".to_string(),
        message: format!("Downloading model {} ...", model_id),
        percentage: 0.0,
    }).ok();
    
    // Use curl for download with progress
    let temp_path = model_path.with_extension("bin.partial");
    let mut cmd = tokio::process::Command::new("curl");
    cmd.args([
        "-L",
        "--progress-bar",
        "-o", temp_path.to_str().unwrap(),
        &url,
    ]);
    
    let output = cmd.output().await.context("Failed to run curl for model download")?;
    
    if cancel_token.is_cancelled() {
        let _ = std::fs::remove_file(&temp_path);
        anyhow::bail!("Download cancelled");
    }
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = std::fs::remove_file(&temp_path);
        anyhow::bail!("Failed to download model: {}", stderr);
    }
    
    // Rename temp file to final path
    std::fs::rename(&temp_path, &model_path)?;
    
    app.emit("transcribe-progress", TranscribeProgressEvent {
        stage: "download".to_string(),
        message: format!("Model {} downloaded successfully", model_id),
        percentage: 100.0,
    }).ok();
    
    Ok(model_path)
}

/// Convert input audio/video to 16kHz mono WAV using ffmpeg
async fn convert_to_wav(
    app: &AppHandle,
    input_path: &str,
    cancel_token: &CancellationToken,
) -> Result<PathBuf> {
    app.emit("transcribe-progress", TranscribeProgressEvent {
        stage: "convert".to_string(),
        message: "Converting audio to WAV format...".to_string(),
        percentage: 10.0,
    }).ok();
    
    let temp_wav = tempfile::Builder::new()
        .suffix(".wav")
        .tempfile()
        .context("Failed to create temp WAV file")?;
    let wav_path = temp_wav.into_temp_path().to_path_buf();
    
    let convert_output = tokio::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-i", input_path,
            "-ar", "16000",
            "-ac", "1",
            "-c:a", "pcm_s16le",
            wav_path.to_str().unwrap(),
        ])
        .output()
        .await
        .context("Failed to run ffmpeg for audio conversion")?;
    
    if !convert_output.status.success() {
        let stderr = String::from_utf8_lossy(&convert_output.stderr);
        anyhow::bail!("Audio conversion failed: {}", stderr);
    }
    
    if cancel_token.is_cancelled() {
        anyhow::bail!("Transcription cancelled");
    }
    
    Ok(wav_path)
}

/// Read WAV file into f32 samples for whisper-rs
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
    
    // If stereo, convert to mono by averaging channels
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
                if next_char.map_or(true, |nc| nc == ' ' || nc.is_uppercase()) {
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
    
    // Load model
    let ctx = WhisperContext::new_with_params(
        model_path.to_str().unwrap(),
        WhisperContextParameters::default(),
    ).map_err(|e| anyhow::anyhow!("Failed to load Whisper model: {:?}", e))?;
    
    let mut state = ctx.create_state()
        .map_err(|e| anyhow::anyhow!("Failed to create Whisper state: {:?}", e))?;
    
    if cancel_token.is_cancelled() {
        anyhow::bail!("Transcription cancelled");
    }
    
    // Configure parameters
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    
    // Set language
    if config.language != "auto" {
        params.set_language(Some(&config.language));
    } else {
        params.set_language(None);
    }
    
    // Set translate mode
    params.set_translate(config.translate_to_english);
    
    // Performance settings
    params.set_n_threads(num_cpus().min(8) as i32);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_special(false);
    params.set_print_timestamps(false);
    params.set_token_timestamps(config.word_timestamps);
    
    // Segment length control
    if config.max_segment_length > 0 {
        params.set_max_len(config.max_segment_length as i32);
    }
    
    app.emit("transcribe-progress", TranscribeProgressEvent {
        stage: "transcribe".to_string(),
        message: "Transcribing audio...".to_string(),
        percentage: 30.0,
    }).ok();
    
    // Run inference
    state.full(params, audio_data)
        .map_err(|e| anyhow::anyhow!("Whisper transcription failed: {:?}", e))?;
    
    if cancel_token.is_cancelled() {
        anyhow::bail!("Transcription cancelled");
    }
    
    // Extract segments
    let n_segments = state.full_n_segments();

    let detected_language = if config.language == "auto" {
        let lang_id = state.full_lang_id_from_state();
        if lang_id >= 0 {
            whisper_rs::get_lang_str(lang_id).map(|s| s.to_string())
        } else {
            None
        }
    } else {
        Some(config.language.clone())
    };
    
    app.emit("transcribe-progress", TranscribeProgressEvent {
        stage: "transcribe".to_string(),
        message: format!("Processing {} segments...", n_segments),
        percentage: 80.0,
    }).ok();
    
    let mut raw_segments: Vec<RawSegment> = Vec::with_capacity(n_segments as usize);
    
    for i in 0..n_segments {
        let seg = match state.get_segment(i) {
            Some(s) => s,
            None => continue,
        };
        
        let start_t = seg.start_timestamp();
        let end_t = seg.end_timestamp();
        let text = match seg.to_str() {
            Ok(s) => s.trim().to_string(),
            Err(_) => continue,
        };
        
        if text.is_empty() {
            continue;
        }
        
        // whisper timestamps are in centiseconds (10ms units)
        raw_segments.push(RawSegment {
            start_ms: start_t * 10,
            end_ms: end_t * 10,
            text,
        });
    }
    
    // Post-process segments for better SRT formatting
    let segments = postprocess_segments(raw_segments, config.max_segment_length);
    
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

/// Get number of available CPU cores
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

// ─── Tauri Commands ──────────────────────────────────────────────────────────

/// Check what Whisper backends are available
#[tauri::command]
pub async fn transcribe_check_backends() -> Result<serde_json::Value, String> {
    let ffmpeg_available = tokio::process::Command::new("ffmpeg")
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
    let models = vec![
        ("tiny", "Tiny", "~75MB", "~32x"),
        ("base", "Base", "~150MB", "~16x"),
        ("small", "Small", "~500MB", "~6x"),
        ("medium", "Medium", "~1.5GB", "~2x"),
        ("large", "Large", "~3GB", "~1x"),
    ];
    
    let mut result = Vec::new();
    for (id, name, size, speed) in models {
        let downloaded = model_file_path(id)
            .map(|p| p.exists())
            .unwrap_or(false);
        
        result.push(WhisperModelInfo {
            id: id.to_string(),
            name: name.to_string(),
            size: size.to_string(),
            speed: speed.to_string(),
            downloaded,
        });
    }
    
    Ok(result)
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
    let model_path = model_file_path(&model_id).map_err(|e| e.to_string())?;
    if model_path.exists() {
        std::fs::remove_file(&model_path).map_err(|e| format!("Failed to delete model: {}", e))?;
        Ok(true)
    } else {
        Err(format!("Model file not found: {}", model_path.display()))
    }
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
    
    // Download model if needed
    let model_path = match download_model(&app, &config.model, &cancel_token).await {
        Ok(p) => p,
        Err(e) => {
            let mut s = state.lock().map_err(|e2| e2.to_string())?;
            s.is_transcribing = false;
            return Err(format!("Model download failed: {}", e));
        }
    };
    
    // Convert input to WAV
    let wav_path = match convert_to_wav(&app, &config.input_path, &cancel_token).await {
        Ok(p) => p,
        Err(e) => {
            let mut s = state.lock().map_err(|e2| e2.to_string())?;
            s.is_transcribing = false;
            return Err(format!("Audio conversion failed: {}", e));
        }
    };
    
    // Read audio data
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
