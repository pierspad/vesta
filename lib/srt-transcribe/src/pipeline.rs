//! High-level, GUI-agnostic transcription pipeline: media file → SRT.
//!
//! This module orchestrates the whole "give me subtitles for this file" flow
//! on top of the low-level primitives in [`crate::audio`], [`crate::model`],
//! [`crate::transcribe`] and [`crate::cloud`]:
//!
//! 1. (local) download the Whisper model if missing;
//! 2. convert the input media to 16 kHz mono WAV via FFmpeg;
//! 3. transcribe locally (whisper.cpp) or through a cloud provider (chunked);
//! 4. post-process the raw segments (merge tiny ones, split overlong ones);
//! 5. write a numbered SRT file, optionally suffixing the detected language.
//!
//! Progress is reported through plain callbacks, cancellation through a
//! [`CancellationToken`] — no UI framework involved. The Vesta GUI, the
//! `srt-transcribe` CLI and any third-party embedder all share this exact code.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::audio::{convert_to_wav, read_wav_to_f32, segment_to_wav_chunks};
use crate::cloud::{transcribe_chunk, CloudConfig};
use crate::transcribe::{transcribe_full, TranscribeOptions, TranscribedSegment};

// ─── Configuration ───────────────────────────────────────────────────────────

/// Full configuration for a media → SRT transcription run.
///
/// Field names are part of the (serde) contract shared with the Vesta
/// frontend; keep them stable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionConfig {
    pub input_path: String,
    pub output_path: String,
    /// Whisper model id (e.g. "base", "small", "large-v3") for the local
    /// backend, or the provider-specific model name for cloud backends.
    pub model: String,
    /// ISO language code, or "auto" to autodetect (local backend only).
    pub language: String,
    pub translate_to_english: bool,
    pub word_timestamps: bool,
    /// Maximum characters per SRT line (0 = default of 80).
    pub max_segment_length: u32,
    /// Transcription engine: `None`/"local" (whisper.cpp) or a cloud provider
    /// ("groq" | "openai" | "deepgram" | "assemblyai" | "custom").
    #[serde(default)]
    pub provider: Option<String>,
    /// API key for cloud providers.
    #[serde(default)]
    pub api_key: Option<String>,
    /// Optional base URL (override / required for "custom").
    #[serde(default)]
    pub api_url: Option<String>,
}

/// Outcome of a successful transcription run.
#[derive(Debug, Clone, Serialize)]
pub struct TranscriptionOutcome {
    /// The SRT actually written (may differ from `config.output_path` when
    /// the detected language is appended to the file name).
    pub output_path: String,
    pub subtitle_count: usize,
    pub detected_language: Option<String>,
}

// ─── Progress reporting ──────────────────────────────────────────────────────

/// A coarse progress update: `stage` is a stable machine-readable identifier
/// ("download", "convert", "transcribe", "writing", "done", …), `percentage`
/// is 0–100 across the whole pipeline.
#[derive(Debug, Clone, Serialize)]
pub struct ProgressUpdate {
    pub stage: String,
    pub message: String,
    pub percentage: f64,
}

/// Callback invoked with coarse progress updates.
pub type ProgressCallback = Arc<dyn Fn(ProgressUpdate) + Send + Sync>;
/// Callback invoked for every transcribed segment (start_ms, end_ms, text).
pub type SegmentCallback = Arc<dyn Fn(i64, i64, &str) + Send + Sync>;

/// Optional callbacks for observing a pipeline run.
#[derive(Default, Clone)]
pub struct PipelineCallbacks {
    pub on_progress: Option<ProgressCallback>,
    pub on_segment: Option<SegmentCallback>,
}

impl PipelineCallbacks {
    fn progress(&self, stage: &str, message: impl Into<String>, percentage: f64) {
        if let Some(cb) = &self.on_progress {
            cb(ProgressUpdate {
                stage: stage.to_string(),
                message: message.into(),
                percentage,
            });
        }
    }

    fn segment(&self, start_ms: i64, end_ms: i64, text: &str) {
        if let Some(cb) = &self.on_segment {
            cb(start_ms, end_ms, text);
        }
    }
}

/// True when `provider` selects a cloud backend rather than local whisper.cpp.
pub fn is_cloud_provider(provider: Option<&str>) -> bool {
    !matches!(
        provider.unwrap_or("local").to_lowercase().as_str(),
        "local" | "whisper" | "local_whisper" | ""
    )
}

// ─── Segments & SRT output ───────────────────────────────────────────────────

/// A raw transcribed segment, before SRT post-processing.
#[derive(Debug, Clone)]
pub struct RawSegment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

impl From<TranscribedSegment> for RawSegment {
    fn from(s: TranscribedSegment) -> Self {
        Self { start_ms: s.start_ms, end_ms: s.end_ms, text: s.text }
    }
}

/// Post-process segments: merge very short ones, split excessively long ones
/// at sentence boundaries with proportional timestamps.
pub fn postprocess_segments(raw: Vec<RawSegment>, max_segment_len: u32) -> Vec<RawSegment> {
    if raw.is_empty() {
        return raw;
    }

    let max_chars = if max_segment_len > 0 { max_segment_len as usize } else { 80 };

    // Phase 1: merge segments that are very short (< 1s) or have very little text.
    let mut merged: Vec<RawSegment> = Vec::new();
    for seg in raw {
        let text = seg.text.trim().to_string();
        if text.is_empty() {
            continue;
        }

        let duration_ms = seg.end_ms - seg.start_ms;
        let should_merge = !merged.is_empty()
            && (duration_ms < 1000 && text.len() < 10 || text.len() < 3);

        if should_merge {
            let last = merged.last_mut().unwrap();
            last.end_ms = seg.end_ms;
            last.text = format!("{} {}", last.text, text);
        } else {
            merged.push(RawSegment { start_ms: seg.start_ms, end_ms: seg.end_ms, text });
        }
    }

    // Phase 2: split segments that exceed max_chars at sentence boundaries.
    let mut result: Vec<RawSegment> = Vec::new();
    for seg in merged {
        if seg.text.len() <= max_chars {
            result.push(seg);
            continue;
        }

        let total_duration = seg.end_ms - seg.start_ms;
        let text = &seg.text;
        let total_chars = text.len();

        let mut splits: Vec<usize> = Vec::new();
        let mut last_split = 0;

        for (i, c) in text.char_indices() {
            if (c == '.' || c == '!' || c == '?' || c == ';') && i > last_split + 10 {
                // Check the next char to avoid splitting inside "3.5".
                let next_char = text[i + c.len_utf8()..].chars().next();
                if next_char.is_none_or(|nc| nc == ' ' || nc.is_uppercase()) {
                    splits.push(i + c.len_utf8());
                    last_split = i + c.len_utf8();
                }
            }
        }

        if splits.is_empty() {
            result.push(seg);
            continue;
        }

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

            result.push(RawSegment { start_ms: sub_start, end_ms: sub_end, text: sub_text });
            prev_pos = *split_pos;

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

/// Format milliseconds as an SRT timestamp `HH:MM:SS,mmm`.
pub fn ms_to_srt_timestamp(ms: i64) -> String {
    let ms = ms.max(0);
    let total_secs = ms / 1000;
    let millis = ms % 1000;
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = total_secs / 3600;
    format!("{hours:02}:{mins:02}:{secs:02},{millis:03}")
}

/// Write segments as a numbered SRT file; returns the subtitle count.
pub fn write_srt(segments: &[RawSegment], output_path: &str) -> Result<usize> {
    use std::io::Write as _;

    let mut file = std::fs::File::create(output_path)
        .with_context(|| format!("Cannot create output file: {output_path}"))?;

    for (i, seg) in segments.iter().enumerate() {
        writeln!(file, "{}", i + 1)?;
        writeln!(
            file,
            "{} --> {}",
            ms_to_srt_timestamp(seg.start_ms),
            ms_to_srt_timestamp(seg.end_ms)
        )?;
        writeln!(file, "{}", seg.text.trim())?;
        writeln!(file)?;
    }

    Ok(segments.len())
}

/// Insert/replace a language suffix in an `.srt` path ("movie.en.srt" style).
/// If the stem already ends with a 2–3 letter language-like token it is
/// replaced, otherwise the language is appended before the extension.
pub fn apply_language_suffix_to_srt_path(output_path: &str, language: &str) -> String {
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
        new_stem = format!("{stem}.{lang}");
    }

    let new_file_name = format!("{new_stem}.srt");
    let mut new_path = path.parent().map_or_else(PathBuf::new, PathBuf::from);
    new_path.push(new_file_name);
    new_path.to_string_lossy().to_string()
}

// ─── Local backend (whisper.cpp via whisper-rs) ──────────────────────────────

/// Run local transcription on already-loaded audio samples. Blocking: call it
/// from a blocking thread (`tokio::task::spawn_blocking`) in async contexts.
pub fn run_local(
    config: &TranscriptionConfig,
    model_path: &Path,
    audio_data: &[f32],
    callbacks: &PipelineCallbacks,
    cancel_token: &CancellationToken,
) -> Result<TranscriptionOutcome> {
    callbacks.progress("transcribe", "Loading Whisper model...", 12.0);

    let model_path_str = model_path.to_string_lossy().to_string();
    let ctx = whisper_rs::WhisperContext::new_with_params(
        &model_path_str,
        whisper_rs::WhisperContextParameters::default(),
    )
    .map_err(|e| anyhow::anyhow!("Failed to load Whisper model: {e:?}"))?;

    if cancel_token.is_cancelled() {
        anyhow::bail!("Transcription cancelled");
    }

    callbacks.progress("transcribe", "Transcribing audio...", 15.0);

    // Total audio duration in ms (16 kHz mono float samples) — lets the
    // segment callback map "where we are in the audio" onto the 15–90% window
    // of the overall progress instead of leaving it frozen at 15%.
    let total_audio_ms = (audio_data.len() as f64 / 16.0).max(1.0);

    let callbacks_for_segments = callbacks.clone();
    let segment_callback = move |start_ms: i64, end_ms: i64, text: &str| {
        callbacks_for_segments.segment(start_ms, end_ms, text);

        let ratio = (end_ms as f64 / total_audio_ms).clamp(0.0, 1.0);
        callbacks_for_segments.progress(
            "transcribe",
            format!("Transcribing audio... {:.0}%", ratio * 100.0),
            15.0 + ratio * 75.0,
        );
    };

    let options = TranscribeOptions {
        language: (config.language != "auto").then(|| config.language.clone()),
        translate_to_english: config.translate_to_english,
        n_threads: None,
        word_timestamps: config.word_timestamps,
        max_segment_length: (config.max_segment_length > 0).then_some(config.max_segment_length),
        segment_callback: Some(Arc::new(segment_callback)),
    };

    let (raw_segments, detected_language) =
        transcribe_full(&ctx, audio_data, &options, Some(cancel_token))?;

    let raw: Vec<RawSegment> = raw_segments.into_iter().map(Into::into).collect();
    let segments = postprocess_segments(raw, config.max_segment_length);

    callbacks.progress(
        "writing",
        match &detected_language {
            Some(lang) => format!("Writing SRT file (language: {lang})..."),
            None => "Writing SRT file...".to_string(),
        },
        90.0,
    );

    let effective_output_path = match (&*config.language, &detected_language) {
        ("auto", Some(lang)) => apply_language_suffix_to_srt_path(&config.output_path, lang),
        _ => config.output_path.clone(),
    };

    let count = write_srt(&segments, &effective_output_path)?;

    Ok(TranscriptionOutcome {
        output_path: effective_output_path,
        subtitle_count: count,
        detected_language,
    })
}

// ─── Cloud backend ───────────────────────────────────────────────────────────

/// Segment length (seconds) for cloud chunking. ~8 min of 16 kHz mono WAV is
/// well under every provider's upload limit (~15 MB).
const CLOUD_CHUNK_SECONDS: i64 = 480;

/// Run transcription through a cloud provider: split the audio into chunks,
/// transcribe each (offsetting timestamps), then post-process and write SRT.
pub async fn run_cloud(
    config: &TranscriptionConfig,
    ffmpeg_cmd: &str,
    callbacks: &PipelineCallbacks,
    cancel_token: &CancellationToken,
) -> Result<TranscriptionOutcome> {
    let provider = config.provider.clone().unwrap_or_else(|| "local".to_string());

    callbacks.progress("preparing", "Preparing audio...", 5.0);

    let tmp_dir = tempfile::Builder::new()
        .prefix("vesta_cloud_")
        .tempdir()
        .context("Failed to create temp dir for cloud transcription")?;

    let chunks = segment_to_wav_chunks(
        ffmpeg_cmd,
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
        language: (config.language != "auto").then(|| config.language.clone()),
        translate_to_english: config.translate_to_english,
    };

    if cloud_cfg.api_key.trim().is_empty() {
        anyhow::bail!("Missing API key for cloud provider '{provider}'");
    }

    let client = crate::cloud::default_client();

    let total = chunks.len();
    let mut all: Vec<TranscribedSegment> = Vec::new();

    for (idx, chunk_path) in chunks.iter().enumerate() {
        if cancel_token.is_cancelled() {
            anyhow::bail!("Transcription cancelled");
        }

        let pct = 10.0 + (idx as f64 / total.max(1) as f64) * 80.0;
        callbacks.progress(
            "transcribe",
            format!("Transcribing chunk {}/{} via {}...", idx + 1, total, provider),
            pct,
        );

        let bytes = std::fs::read(chunk_path)
            .with_context(|| format!("Failed to read audio chunk {}", idx + 1))?;
        let offset_ms = idx as i64 * CLOUD_CHUNK_SECONDS * 1000;

        let segs = transcribe_chunk(&client, &cloud_cfg, bytes, "audio.wav")
            .await
            .with_context(|| format!("Cloud transcription failed on chunk {}", idx + 1))?;

        for mut s in segs {
            s.start_ms += offset_ms;
            s.end_ms += offset_ms;
            callbacks.segment(s.start_ms, s.end_ms, &s.text);
            all.push(s);
        }
    }

    all.sort_by_key(|s| s.start_ms);

    let raw: Vec<RawSegment> = all.into_iter().map(Into::into).collect();
    let segments = postprocess_segments(raw, config.max_segment_length);

    callbacks.progress("writing", "Writing SRT file...", 92.0);

    let count = write_srt(&segments, &config.output_path)?;

    let detected_language = (config.language != "auto").then(|| config.language.clone());

    Ok(TranscriptionOutcome {
        output_path: config.output_path.clone(),
        subtitle_count: count,
        detected_language,
    })
}

// ─── Full pipeline ───────────────────────────────────────────────────────────

/// Transcribe a media file to SRT, end to end.
///
/// Dispatches to the local (whisper.cpp) or cloud backend based on
/// `config.provider`. For the local backend the Whisper model is downloaded
/// on demand (with progress on the "download" stage) and the input is
/// converted to 16 kHz mono WAV via `ffmpeg_cmd` first.
pub async fn transcribe_to_srt(
    config: &TranscriptionConfig,
    ffmpeg_cmd: &str,
    callbacks: PipelineCallbacks,
    cancel_token: &CancellationToken,
) -> Result<TranscriptionOutcome> {
    if !Path::new(&config.input_path).exists() {
        anyhow::bail!("Input file not found: {}", config.input_path);
    }

    if is_cloud_provider(config.provider.as_deref()) {
        let outcome = run_cloud(config, ffmpeg_cmd, &callbacks, cancel_token).await?;
        callbacks.progress(
            "done",
            format!("Transcription completed: {} segments", outcome.subtitle_count),
            100.0,
        );
        return Ok(outcome);
    }

    // ── Local whisper.cpp path ────────────────────────────────────────────
    callbacks.progress("download", format!("Checking model {}...", config.model), 0.0);

    let callbacks_dl = callbacks.clone();
    let model_label = config.model.clone();
    let model_path = crate::model::download_model(
        &config.model,
        move |percentage| {
            callbacks_dl.progress(
                "download",
                format!("Downloading model {model_label} ({percentage}%)..."),
                percentage as f64,
            );
        },
        Some(cancel_token),
    )
    .await
    .context("Model download failed")?;

    let temp_wav = tempfile::Builder::new()
        .suffix(".wav")
        .tempfile()
        .context("Failed to create temp WAV file")?;
    let wav_path = temp_wav.into_temp_path().to_path_buf();

    callbacks.progress("convert", "Converting audio format...", 5.0);

    convert_to_wav(ffmpeg_cmd, Path::new(&config.input_path), &wav_path, Some(cancel_token))
        .await
        .context("Audio conversion failed")?;

    callbacks.progress("convert", "Audio converted successfully", 10.0);

    let audio_data = read_wav_to_f32(&wav_path).inspect_err(|_| {
        let _ = std::fs::remove_file(&wav_path);
    })?;

    let config_clone = config.clone();
    let callbacks_clone = callbacks.clone();
    let cancel_clone = cancel_token.clone();

    let result = tokio::task::spawn_blocking(move || {
        run_local(&config_clone, &model_path, &audio_data, &callbacks_clone, &cancel_clone)
    })
    .await
    .context("Transcription task failed")?;

    let _ = std::fs::remove_file(&wav_path);

    let outcome = result?;
    callbacks.progress(
        "done",
        format!("Transcription completed: {} segments", outcome.subtitle_count),
        100.0,
    );

    Ok(outcome)
}
