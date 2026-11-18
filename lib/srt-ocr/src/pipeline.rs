//! End-to-end bitmap and hardcoded video subtitle OCR pipelines and preview generators.

use crate::backend::OcrBackend;
use crate::bitmap::{BitmapSubtitleCue, extract_bitmap_subtitles};
use crate::hardcoded::{VideoRoi, extract_hardcoded_subtitles};
use crate::local::LocalOcrBackend;
use crate::vision::VisionLlmBackend;
use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use srt_parser::{Subtitle, Timestamp, persistence};
use srt_translate::{TierEntry, TranslationDiagnostic};
use std::collections::VecDeque;
use std::future::Future;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

const DEFAULT_OCR_CONCURRENCY: usize = 4;
const DEFAULT_PREVIEW_LIMIT: usize = 15;
const DEFAULT_LOCAL_OCR_MODEL_ID: &str = "paddleocr-v4-mobile-multi";

/// Progress event emitted during subtitle extraction and OCR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OcrProgress {
    /// Pipeline stage (`"extract"`, `"ocr"`, `"done"`).
    pub stage: String,
    /// Human-readable status message.
    pub message: String,
    /// Completion percentage in `0.0..=100.0`.
    pub percentage: f64,
    /// Number of cues processed so far.
    pub current: usize,
    /// Total number of cues to process.
    pub total: usize,
}

/// Recognized subtitle segment emitted by the OCR pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OcrSegment {
    /// Sequential 1-based cue ID (or original cue ID when streamed mid-run).
    pub id: u32,
    /// Cue start time in milliseconds.
    pub start_ms: u64,
    /// Cue end time in milliseconds.
    pub end_ms: u64,
    /// Recognized subtitle text.
    pub text: String,
}

/// Configuration for bitmap subtitle OCR extraction and conversion to SRT.
#[derive(Debug, Clone)]
pub struct OcrConfig {
    /// Input media container path.
    pub input_path: PathBuf,
    /// Destination `.srt` file path.
    pub output_path: PathBuf,
    /// FFprobe stream index of the bitmap subtitle track.
    pub track_index: u32,
    /// Optional ISO-639 / language name hint passed to the OCR backend.
    pub language: Option<String>,
    /// Priority-ordered OCR backend tiers (Vision LLM and/or `local_ocr`).
    pub tiers: Vec<Vec<TierEntry>>,
    /// Optional cap on the number of cues processed (primarily for previews).
    pub preview_limit: Option<usize>,
    /// Path or executable name for `ffmpeg`.
    pub ffmpeg_path: String,
    /// Path or executable name for `ffprobe`.
    pub ffprobe_path: String,
}

/// Configuration for burned-in (hardcoded) video subtitle ROI OCR extraction and conversion to SRT.
#[derive(Debug, Clone)]
pub struct HardcodedOcrConfig {
    /// Input video container path.
    pub input_path: PathBuf,
    /// Destination `.srt` file path.
    pub output_path: PathBuf,
    /// Normalized `[0.0, 1.0]` ROI rectangle where burned-in subtitles appear.
    pub roi: VideoRoi,
    /// Frame sampling rate in frames per second (`0.5..=15.0`).
    pub sample_fps: f64,
    /// Optional ISO-639 / language name hint passed to the OCR backend.
    pub language: Option<String>,
    /// Priority-ordered OCR backend tiers (Vision LLM and/or `local_ocr`).
    pub tiers: Vec<Vec<TierEntry>>,
    /// Optional cap on the number of cues processed (primarily for previews).
    pub preview_limit: Option<usize>,
    /// Path or executable name for `ffmpeg`.
    pub ffmpeg_path: String,
    /// Path or executable name for `ffprobe`.
    pub ffprobe_path: String,
}

/// Summary returned after writing an OCR-converted `.srt` file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OcrOutcome {
    /// Destination `.srt` path written by the pipeline.
    pub output_path: String,
    /// Number of non-empty subtitle cues written to the `.srt` file.
    pub subtitle_count: usize,
}

/// Optional progress and per-segment streaming callbacks for OCR pipelines.
#[derive(Default, Clone)]
pub struct OcrCallbacks {
    /// Invoked whenever extraction or OCR progress advances.
    pub on_progress: Option<Arc<dyn Fn(OcrProgress) + Send + Sync>>,
    /// Invoked whenever a non-empty subtitle cue is recognized.
    pub on_segment: Option<Arc<dyn Fn(OcrSegment) + Send + Sync>>,
}

impl OcrCallbacks {
    fn progress(
        &self,
        stage: &str,
        message: impl Into<String>,
        percentage: f64,
        current: usize,
        total: usize,
    ) {
        if let Some(cb) = &self.on_progress {
            cb(OcrProgress {
                stage: stage.to_string(),
                message: message.into(),
                percentage,
                current,
                total,
            });
        }
    }

    fn segment(&self, segment: OcrSegment) {
        if let Some(cb) = &self.on_segment {
            cb(segment);
        }
    }
}

/// Priority-ordered OCR backend wrapper that fails over across heterogeneous backends.
pub struct CompositeOcrBackend {
    backends: Vec<Arc<dyn OcrBackend>>,
}

impl CompositeOcrBackend {
    /// Creates a composite backend trying `backends` in priority order on non-cancellation errors.
    pub fn new(backends: Vec<Arc<dyn OcrBackend>>) -> Result<Self> {
        anyhow::ensure!(
            !backends.is_empty(),
            "CompositeOcrBackend requires at least one backend"
        );
        Ok(Self { backends })
    }
}

impl OcrBackend for CompositeOcrBackend {
    fn recognize_cue<'a>(
        &'a self,
        cue: &'a BitmapSubtitleCue,
        language_hint: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move {
            let mut last_err: Option<anyhow::Error> = None;
            for backend in &self.backends {
                if cancel.is_cancelled() {
                    bail!("OCR cancelled by user");
                }
                match backend.recognize_cue(cue, language_hint, cancel).await {
                    Ok(text) => return Ok(text),
                    Err(err) => {
                        if cancel.is_cancelled() {
                            return Err(err);
                        }
                        last_err = Some(err);
                    }
                }
            }
            Err(last_err.unwrap_or_else(|| anyhow::anyhow!("All OCR backends failed")))
        })
    }

    fn recommended_concurrency(&self) -> usize {
        self.backends
            .first()
            .map_or(DEFAULT_OCR_CONCURRENCY, |b| b.recommended_concurrency())
    }
}

fn is_local_ocr_provider(provider: &str) -> bool {
    let trimmed = provider.trim();
    trimmed.eq_ignore_ascii_case("local_ocr") || trimmed.eq_ignore_ascii_case("paddle")
}

fn build_vision_backend(
    tiers: &[Vec<TierEntry>],
    diagnostics: Option<&Arc<dyn Fn(TranslationDiagnostic) + Send + Sync>>,
) -> Result<Arc<dyn OcrBackend>> {
    let backend = match diagnostics {
        Some(obs) => VisionLlmBackend::with_diagnostics(tiers, obs.clone())?,
        None => VisionLlmBackend::new(tiers)?,
    };
    Ok(Arc::new(backend))
}

/// Builds an [`OcrBackend`] (Vision LLM, pure-Rust `LocalOcrBackend`, or composite failover)
/// from priority-ordered `tiers`.
pub fn build_backend_from_tiers(
    tiers: &[Vec<TierEntry>],
    diagnostics: Option<Arc<dyn Fn(TranslationDiagnostic) + Send + Sync>>,
) -> Result<Arc<dyn OcrBackend>> {
    let mut backends: Vec<Arc<dyn OcrBackend>> = Vec::new();
    let mut pending_vision_tiers: Vec<Vec<TierEntry>> = Vec::new();
    let mut first_err: Option<anyhow::Error> = None;

    let flush_vision = |pending: &mut Vec<Vec<TierEntry>>,
                        backends: &mut Vec<Arc<dyn OcrBackend>>,
                        first_err: &mut Option<anyhow::Error>| {
        if pending.is_empty() {
            return;
        }
        let chunk = std::mem::take(pending);
        match build_vision_backend(&chunk, diagnostics.as_ref()) {
            Ok(b) => backends.push(b),
            Err(err) => {
                if first_err.is_none() {
                    *first_err = Some(err);
                }
            }
        }
    };

    for tier in tiers {
        let mut vision_in_tier = Vec::new();
        let mut local_in_tier = Vec::new();
        for entry in tier {
            if is_local_ocr_provider(&entry.provider) {
                local_in_tier.push(entry);
            } else {
                vision_in_tier.push(entry.clone());
            }
        }

        if !vision_in_tier.is_empty() {
            pending_vision_tiers.push(vision_in_tier);
        }

        if !local_in_tier.is_empty() {
            flush_vision(&mut pending_vision_tiers, &mut backends, &mut first_err);
            for entry in local_in_tier {
                let model_id = if entry.model.trim().is_empty() {
                    DEFAULT_LOCAL_OCR_MODEL_ID
                } else {
                    entry.model.trim()
                };
                match LocalOcrBackend::from_model_id(model_id) {
                    Ok(local) => backends.push(Arc::new(local)),
                    Err(err) => {
                        if first_err.is_none() {
                            first_err = Some(err);
                        }
                    }
                }
            }
        }
    }

    flush_vision(&mut pending_vision_tiers, &mut backends, &mut first_err);

    if backends.is_empty() {
        return Err(first_err.unwrap_or_else(|| {
            anyhow::anyhow!("No valid OCR tiers configured. Add a Vision LLM or local OCR model.")
        }));
    }

    if backends.len() == 1 {
        Ok(backends.remove(0))
    } else {
        Ok(Arc::new(CompositeOcrBackend::new(backends)?))
    }
}

fn format_srt_segments(segments: &[OcrSegment]) -> String {
    let mut out = String::with_capacity(segments.len() * 64);
    for (idx, seg) in segments.iter().enumerate() {
        let sub = Subtitle {
            id: seg.id,
            start: Timestamp {
                milliseconds: seg.start_ms,
            },
            end: Timestamp {
                milliseconds: seg.end_ms,
            },
            text: seg.text.clone(),
        };
        sub.write_srt_to(&mut out);
        if idx + 1 < segments.len() {
            out.push('\n');
        }
    }
    out
}

async fn recognize_extracted_cues(
    cues: Vec<BitmapSubtitleCue>,
    language: Option<&str>,
    backend: Arc<dyn OcrBackend>,
    concurrency: usize,
    callbacks: &OcrCallbacks,
    cancel: &CancellationToken,
) -> Result<Vec<OcrSegment>> {
    if cancel.is_cancelled() {
        bail!("OCR cancelled by user");
    }

    let total = cues.len();
    callbacks.progress(
        "ocr",
        format!("Recognizing {total} subtitle cues..."),
        0.0,
        0,
        total,
    );

    if cues.is_empty() {
        return Ok(Vec::new());
    }

    let worker_count = concurrency.clamp(1, 16).min(total);
    let queue_items: VecDeque<(usize, BitmapSubtitleCue)> =
        cues.iter().cloned().enumerate().collect();
    let queue = Arc::new(Mutex::new(queue_items));
    let results: Arc<Mutex<Vec<Option<String>>>> = Arc::new(Mutex::new(vec![None; total]));
    let completed = Arc::new(AtomicUsize::new(0));
    let language = language.map(ToString::to_string);

    let mut workers = JoinSet::new();
    for _ in 0..worker_count {
        let queue = queue.clone();
        let results = results.clone();
        let completed = completed.clone();
        let backend = backend.clone();
        let callbacks = callbacks.clone();
        let language = language.clone();
        let token = cancel.clone();

        workers.spawn(async move {
            loop {
                if token.is_cancelled() {
                    bail!("OCR cancelled by user");
                }
                let next = { queue.lock().await.pop_front() };
                let Some((idx, cue)) = next else {
                    return Ok::<(), anyhow::Error>(());
                };

                let recognized = backend
                    .recognize_cue(&cue, language.as_deref(), &token)
                    .await?;
                let trimmed = recognized.trim().to_string();

                {
                    let mut guard = results.lock().await;
                    if let Some(slot) = guard.get_mut(idx) {
                        *slot = Some(trimmed.clone());
                    }
                }

                let done = completed.fetch_add(1, Ordering::AcqRel) + 1;
                let pct = ((done as f64 / total as f64) * 100.0).min(100.0);
                callbacks.progress(
                    "ocr",
                    format!("Recognized {done}/{total} subtitle cues..."),
                    pct,
                    done,
                    total,
                );

                if !trimmed.is_empty() {
                    callbacks.segment(OcrSegment {
                        id: cue.id,
                        start_ms: cue.start_ms,
                        end_ms: cue.end_ms,
                        text: trimmed,
                    });
                }
            }
        });
    }

    while let Some(joined) = workers.join_next().await {
        match joined {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                workers.abort_all();
                return Err(err);
            }
            Err(join_err) => {
                workers.abort_all();
                return Err(join_err).context("OCR worker task failed");
            }
        }
    }

    if cancel.is_cancelled() {
        bail!("OCR cancelled by user");
    }

    let ordered_texts = std::mem::take(&mut *results.lock().await);
    let mut segments = Vec::new();
    for (cue, maybe_text) in cues.into_iter().zip(ordered_texts) {
        let text = maybe_text.unwrap_or_default();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }
        let id = u32::try_from(segments.len() + 1).context("Subtitle count overflow")?;
        segments.push(OcrSegment {
            id,
            start_ms: cue.start_ms,
            end_ms: cue.end_ms,
            text: trimmed.to_string(),
        });
    }

    Ok(segments)
}

fn publish_ocr_srt(
    output_path: &Path,
    segments: &[OcrSegment],
    callbacks: &OcrCallbacks,
    cancel: &CancellationToken,
) -> Result<OcrOutcome> {
    anyhow::ensure!(!cancel.is_cancelled(), "OCR cancelled by user");

    let srt_content = format_srt_segments(segments);
    persistence::atomic_write_with(output_path, |file| {
        let mut writer = std::io::BufWriter::new(file);
        writer.write_all(srt_content.as_bytes())?;
        writer.flush()?;
        Ok(())
    })
    .with_context(|| format!("Failed to write OCR output file: {}", output_path.display()))?;

    let count = segments.len();
    callbacks.progress(
        "done",
        format!("OCR completed: {count} subtitles"),
        100.0,
        count,
        count,
    );

    Ok(OcrOutcome {
        output_path: output_path.to_string_lossy().into_owned(),
        subtitle_count: count,
    })
}

async fn ocr_bitmap_track_with_backend_concurrency(
    config: &OcrConfig,
    backend: Arc<dyn OcrBackend>,
    concurrency: usize,
    callbacks: OcrCallbacks,
    cancel: &CancellationToken,
) -> Result<OcrOutcome> {
    if cancel.is_cancelled() {
        bail!("OCR cancelled by user");
    }

    persistence::ensure_distinct_files(&config.input_path, &config.output_path)?;
    callbacks.progress("extract", "Extracting bitmap subtitle frames...", 0.0, 0, 0);

    let cues = extract_bitmap_subtitles(
        &config.ffmpeg_path,
        &config.ffprobe_path,
        &config.input_path,
        config.track_index,
        config.preview_limit,
        Some(cancel),
    )
    .await?;

    let segments = recognize_extracted_cues(
        cues,
        config.language.as_deref(),
        backend,
        concurrency,
        &callbacks,
        cancel,
    )
    .await?;

    publish_ocr_srt(&config.output_path, &segments, &callbacks, cancel)
}

/// Runs bitmap subtitle extraction and OCR using configured tiers (Vision LLM and/or local OCR),
/// publishing the resulting `.srt` file atomically.
pub async fn ocr_bitmap_track_to_srt(
    config: &OcrConfig,
    callbacks: OcrCallbacks,
    cancel: &CancellationToken,
) -> Result<OcrOutcome> {
    let backend = build_backend_from_tiers(&config.tiers, None)?;
    let concurrency = backend.recommended_concurrency();
    ocr_bitmap_track_with_backend_concurrency(config, backend, concurrency, callbacks, cancel).await
}

/// Runs bitmap subtitle extraction and OCR using a custom [`OcrBackend`] implementation,
/// publishing the resulting `.srt` file atomically.
pub async fn ocr_bitmap_track_with_backend(
    config: &OcrConfig,
    backend: Arc<dyn OcrBackend>,
    callbacks: OcrCallbacks,
    cancel: &CancellationToken,
) -> Result<OcrOutcome> {
    let concurrency = backend.recommended_concurrency();
    ocr_bitmap_track_with_backend_concurrency(config, backend, concurrency, callbacks, cancel).await
}

async fn preview_ocr_bitmap_track_with_backend(
    config: &OcrConfig,
    backend: Arc<dyn OcrBackend>,
    concurrency: usize,
    cancel: &CancellationToken,
) -> Result<String> {
    if cancel.is_cancelled() {
        bail!("OCR cancelled by user");
    }
    let limit = Some(config.preview_limit.unwrap_or(DEFAULT_PREVIEW_LIMIT));
    let callbacks = OcrCallbacks::default();
    let cues = extract_bitmap_subtitles(
        &config.ffmpeg_path,
        &config.ffprobe_path,
        &config.input_path,
        config.track_index,
        limit,
        Some(cancel),
    )
    .await?;
    let segments = recognize_extracted_cues(
        cues,
        config.language.as_deref(),
        backend,
        concurrency,
        &callbacks,
        cancel,
    )
    .await?;
    Ok(format_srt_segments(&segments))
}

/// Extracts and recognizes up to `config.preview_limit.unwrap_or(15)` bitmap cues in memory,
/// returning formatted SRT text without writing to `config.output_path`.
pub async fn preview_ocr_bitmap_track(
    config: &OcrConfig,
    cancel: &CancellationToken,
) -> Result<String> {
    let backend = build_backend_from_tiers(&config.tiers, None)?;
    let concurrency = backend.recommended_concurrency();
    preview_ocr_bitmap_track_with_backend(config, backend, concurrency, cancel).await
}

async fn ocr_hardcoded_with_backend_concurrency(
    config: &HardcodedOcrConfig,
    backend: Arc<dyn OcrBackend>,
    concurrency: usize,
    callbacks: OcrCallbacks,
    cancel: &CancellationToken,
) -> Result<OcrOutcome> {
    if cancel.is_cancelled() {
        bail!("OCR cancelled by user");
    }

    persistence::ensure_distinct_files(&config.input_path, &config.output_path)?;
    callbacks.progress(
        "extract",
        "Sampling hardcoded video subtitle ROI frames...",
        0.0,
        0,
        0,
    );

    let cues = extract_hardcoded_subtitles(
        &config.ffmpeg_path,
        &config.ffprobe_path,
        &config.input_path,
        config.roi,
        config.sample_fps,
        config.preview_limit,
        Some(cancel),
    )
    .await?;

    let segments = recognize_extracted_cues(
        cues,
        config.language.as_deref(),
        backend,
        concurrency,
        &callbacks,
        cancel,
    )
    .await?;

    publish_ocr_srt(&config.output_path, &segments, &callbacks, cancel)
}

/// Runs burned-in video ROI extraction and OCR using configured tiers (Vision LLM and/or local OCR),
/// publishing the resulting `.srt` file atomically.
pub async fn ocr_hardcoded_to_srt(
    config: &HardcodedOcrConfig,
    callbacks: OcrCallbacks,
    cancel: &CancellationToken,
) -> Result<OcrOutcome> {
    let backend = build_backend_from_tiers(&config.tiers, None)?;
    let concurrency = backend.recommended_concurrency();
    ocr_hardcoded_with_backend_concurrency(config, backend, concurrency, callbacks, cancel).await
}

/// Runs burned-in video ROI extraction and OCR using a custom [`OcrBackend`] implementation,
/// publishing the resulting `.srt` file atomically.
pub async fn ocr_hardcoded_with_backend(
    config: &HardcodedOcrConfig,
    backend: Arc<dyn OcrBackend>,
    callbacks: OcrCallbacks,
    cancel: &CancellationToken,
) -> Result<OcrOutcome> {
    let concurrency = backend.recommended_concurrency();
    ocr_hardcoded_with_backend_concurrency(config, backend, concurrency, callbacks, cancel).await
}

/// Extracts and recognizes up to `config.preview_limit.unwrap_or(15)` burned-in ROI cues in memory,
/// returning formatted SRT text without writing to `config.output_path`.
pub async fn preview_ocr_hardcoded(
    config: &HardcodedOcrConfig,
    cancel: &CancellationToken,
) -> Result<String> {
    if cancel.is_cancelled() {
        bail!("OCR cancelled by user");
    }
    let backend = build_backend_from_tiers(&config.tiers, None)?;
    let concurrency = backend.recommended_concurrency();
    let limit = Some(config.preview_limit.unwrap_or(DEFAULT_PREVIEW_LIMIT));
    let callbacks = OcrCallbacks::default();
    let cues = extract_hardcoded_subtitles(
        &config.ffmpeg_path,
        &config.ffprobe_path,
        &config.input_path,
        config.roi,
        config.sample_fps,
        limit,
        Some(cancel),
    )
    .await?;
    let segments = recognize_extracted_cues(
        cues,
        config.language.as_deref(),
        backend,
        concurrency,
        &callbacks,
        cancel,
    )
    .await?;
    Ok(format_srt_segments(&segments))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    struct MockOcrBackend {
        delay_ms: u64,
    }

    impl OcrBackend for MockOcrBackend {
        fn recognize_cue<'a>(
            &'a self,
            cue: &'a BitmapSubtitleCue,
            _language_hint: Option<&'a str>,
            cancel: &'a CancellationToken,
        ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
            Box::pin(async move {
                if self.delay_ms > 0 {
                    tokio::select! {
                        biased;
                        _ = cancel.cancelled() => bail!("OCR cancelled by user"),
                        _ = tokio::time::sleep(Duration::from_millis(self.delay_ms)) => {}
                    }
                }
                match cue.id {
                    2 => Ok(String::new()),
                    id => Ok(format!("Subtitle cue {id}")),
                }
            })
        }
    }

    struct FailingBackend {
        message: &'static str,
    }

    impl OcrBackend for FailingBackend {
        fn recognize_cue<'a>(
            &'a self,
            _cue: &'a BitmapSubtitleCue,
            _language_hint: Option<&'a str>,
            _cancel: &'a CancellationToken,
        ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
            Box::pin(async move { bail!("{}", self.message) })
        }
    }

    fn pgs_segment(pts_ms: u64, seg_type: u8, payload: &[u8]) -> Vec<u8> {
        let pts_90k = ((pts_ms * 90_000) / 1000) as u32;
        let mut out = Vec::with_capacity(13 + payload.len());
        out.extend_from_slice(b"PG");
        out.extend_from_slice(&pts_90k.to_be_bytes());
        out.extend_from_slice(&pts_90k.to_be_bytes());
        out.push(seg_type);
        out.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        out.extend_from_slice(payload);
        out
    }

    fn create_three_cue_pgs_mkv(dir: &std::path::Path) -> PathBuf {
        let mut sup = Vec::new();
        let mut pds = vec![0u8, 0u8];
        pds.extend_from_slice(&[0, 16, 128, 128, 0, 1, 235, 128, 128, 255]);
        let mut rle = Vec::new();
        for _ in 0..8 {
            rle.extend_from_slice(&[0x00, 0x80 | 20, 1, 0x00, 0x00]);
        }
        let obj_len = (4 + rle.len()) as u32;
        let mut ods = vec![
            0,
            0,
            0,
            0xC0,
            ((obj_len >> 16) & 0xFF) as u8,
            ((obj_len >> 8) & 0xFF) as u8,
            (obj_len & 0xFF) as u8,
        ];
        ods.extend_from_slice(&20u16.to_be_bytes());
        ods.extend_from_slice(&8u16.to_be_bytes());
        ods.extend_from_slice(&rle);

        for (idx, (start_ms, end_ms)) in [(1000u64, 2000u64), (2500, 3500), (4000, 5000)]
            .into_iter()
            .enumerate()
        {
            let mut pcs = Vec::new();
            pcs.extend_from_slice(&1280u16.to_be_bytes());
            pcs.extend_from_slice(&720u16.to_be_bytes());
            pcs.push(0x10);
            pcs.extend_from_slice(&((idx as u16) + 1).to_be_bytes());
            pcs.push(if idx == 0 { 0x80 } else { 0x00 });
            pcs.push(0x00);
            pcs.push(0x00);
            pcs.push(1);
            pcs.extend_from_slice(&0u16.to_be_bytes());
            pcs.push(0);
            pcs.push(0);
            pcs.extend_from_slice(&100u16.to_be_bytes());
            pcs.extend_from_slice(&600u16.to_be_bytes());

            sup.extend(pgs_segment(start_ms, 0x16, &pcs));
            if idx == 0 {
                sup.extend(pgs_segment(start_ms, 0x14, &pds));
                sup.extend(pgs_segment(start_ms, 0x15, &ods));
            }
            sup.extend(pgs_segment(start_ms, 0x80, &[]));

            let mut clear_pcs = Vec::new();
            clear_pcs.extend_from_slice(&1280u16.to_be_bytes());
            clear_pcs.extend_from_slice(&720u16.to_be_bytes());
            clear_pcs.push(0x10);
            clear_pcs.extend_from_slice(&((idx as u16) + 10).to_be_bytes());
            clear_pcs.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
            sup.extend(pgs_segment(end_ms, 0x16, &clear_pcs));
            sup.extend(pgs_segment(end_ms, 0x80, &[]));
        }

        let sup_path = dir.join("three.sup");
        std::fs::write(&sup_path, sup).unwrap();
        let mkv_path = dir.join("three.mkv");
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-nostdin",
                "-v",
                "error",
                "-copyts",
                "-f",
                "lavfi",
                "-i",
                "color=size=1280x720:duration=6:rate=1",
                "-i",
            ])
            .arg(&sup_path)
            .args([
                "-map",
                "0:v",
                "-map",
                "1:s",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-c:s",
                "copy",
                "-y",
            ])
            .arg(&mkv_path)
            .status()
            .expect("ffmpeg is required for pipeline tests");
        assert!(status.success());
        mkv_path
    }

    fn create_hardcoded_mkv(dir: &std::path::Path) -> PathBuf {
        let mkv_path = dir.join("hardcoded.mkv");
        let vf = concat!(
            "drawbox=x=80:y=270:w=220:h=36:color=white@1.0:t=fill:enable='between(t,0.4,1.4)',",
            "drawbox=x=320:y=270:w=220:h=36:color=white@1.0:t=fill:enable='between(t,2.0,3.0)'"
        );
        let status = std::process::Command::new("ffmpeg")
            .args([
                "-nostdin",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=640x360:d=3.5:r=10",
                "-vf",
                vf,
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-y",
            ])
            .arg(&mkv_path)
            .status()
            .expect("ffmpeg required for hardcoded pipeline test");
        assert!(status.success());
        mkv_path
    }

    #[tokio::test]
    async fn pipeline_writes_sequentially_renumbered_srt_and_preview_skips_disk_write() {
        let dir = tempfile::tempdir().unwrap();
        let mkv_path = create_three_cue_pgs_mkv(dir.path());
        let output_path = dir.path().join("out.srt");
        let preview_output_path = dir.path().join("should_not_exist.srt");

        let config = OcrConfig {
            input_path: mkv_path.clone(),
            output_path: output_path.clone(),
            track_index: 1,
            language: Some("en".to_string()),
            tiers: vec![],
            preview_limit: None,
            ffmpeg_path: "ffmpeg".to_string(),
            ffprobe_path: "ffprobe".to_string(),
        };

        let progress_events = Arc::new(Mutex::new(Vec::new()));
        let segment_events = Arc::new(Mutex::new(Vec::new()));
        let p_clone = progress_events.clone();
        let s_clone = segment_events.clone();

        let callbacks = OcrCallbacks {
            on_progress: Some(Arc::new(move |evt| {
                if let Ok(mut guard) = p_clone.try_lock() {
                    guard.push(evt);
                }
            })),
            on_segment: Some(Arc::new(move |seg| {
                if let Ok(mut guard) = s_clone.try_lock() {
                    guard.push(seg);
                }
            })),
        };

        let cancel = CancellationToken::new();
        let backend: Arc<dyn OcrBackend> = Arc::new(MockOcrBackend { delay_ms: 0 });
        let outcome = ocr_bitmap_track_with_backend(&config, backend.clone(), callbacks, &cancel)
            .await
            .unwrap();

        assert_eq!(outcome.subtitle_count, 2);
        assert!(output_path.exists());
        let parsed = srt_parser::SrtParser::parse_file(&output_path).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[&1].text, "Subtitle cue 1");
        assert_eq!(
            parsed[&1].end.milliseconds - parsed[&1].start.milliseconds,
            1000
        );
        // Cue 2 returned empty and was dropped; cue 3 was renumbered to ID 2.
        assert_eq!(parsed[&2].text, "Subtitle cue 3");
        assert_eq!(
            parsed[&2].end.milliseconds - parsed[&2].start.milliseconds,
            1000
        );
        assert!(parsed[&2].start.milliseconds > parsed[&1].end.milliseconds);

        let prog = progress_events.lock().await;
        assert_eq!(prog.last().unwrap().stage, "done");
        assert_eq!(segment_events.lock().await.len(), 2);

        // Preview mode respects preview_limit and must not create output_path.
        let preview_cfg = OcrConfig {
            output_path: preview_output_path.clone(),
            preview_limit: Some(1),
            ..config.clone()
        };
        let preview_srt = preview_ocr_bitmap_track_with_backend(&preview_cfg, backend, 2, &cancel)
            .await
            .unwrap();
        assert!(!preview_output_path.exists());
        let preview_parsed = srt_parser::SrtParser::parse_string(&preview_srt).unwrap();
        assert_eq!(preview_parsed.len(), 1);
        assert_eq!(preview_parsed[&1].text, "Subtitle cue 1");
    }

    #[tokio::test]
    async fn cancellation_aborts_pipeline_without_modifying_existing_output() {
        let dir = tempfile::tempdir().unwrap();
        let mkv_path = create_three_cue_pgs_mkv(dir.path());
        let output_path = dir.path().join("existing.srt");
        std::fs::write(&output_path, "existing-content").unwrap();

        let config = OcrConfig {
            input_path: mkv_path,
            output_path: output_path.clone(),
            track_index: 1,
            language: None,
            tiers: vec![],
            preview_limit: None,
            ffmpeg_path: "ffmpeg".to_string(),
            ffprobe_path: "ffprobe".to_string(),
        };

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            cancel_clone.cancel();
        });

        let backend: Arc<dyn OcrBackend> = Arc::new(MockOcrBackend { delay_ms: 500 });
        let err = ocr_bitmap_track_with_backend(&config, backend, OcrCallbacks::default(), &cancel)
            .await
            .unwrap_err();
        assert!(err.to_string().to_lowercase().contains("cancel"));
        assert_eq!(
            std::fs::read_to_string(&output_path).unwrap(),
            "existing-content"
        );
    }

    #[tokio::test]
    async fn ocr_bitmap_track_to_srt_and_preview_end_to_end_with_mock_vision_http() {
        let dir = tempfile::tempdir().unwrap();
        let mkv_path = create_three_cue_pgs_mkv(dir.path());
        let output_path = dir.path().join("vision.srt");

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for idx in 1..=2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                let header_end = loop {
                    let n = stream.read(&mut chunk).await.unwrap();
                    assert!(n > 0);
                    buf.extend_from_slice(&chunk[..n]);
                    if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        break pos + 4;
                    }
                };
                let headers = String::from_utf8_lossy(&buf[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (k, v) = line.split_once(':')?;
                        k.trim()
                            .eq_ignore_ascii_case("content-length")
                            .then(|| v.trim().parse::<usize>().ok())?
                    })
                    .unwrap_or(0);
                while buf.len() < header_end + content_length {
                    let n = stream.read(&mut chunk).await.unwrap();
                    assert!(n > 0);
                    buf.extend_from_slice(&chunk[..n]);
                }
                let body = format!(
                    r#"{{"choices":[{{"message":{{"role":"assistant","content":"OCR line {idx}"}}}}]}}"#
                );
                let http = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(http.as_bytes()).await.unwrap();
            }
        });

        let config = OcrConfig {
            input_path: mkv_path,
            output_path: output_path.clone(),
            track_index: 1,
            language: Some("en".to_string()),
            tiers: vec![vec![TierEntry {
                provider: "local".to_string(),
                model: "vision-test".to_string(),
                api_key: None,
                api_url: Some(format!("http://{addr}/v1")),
                rpm: Some(0),
                max_requests: None,
            }]],
            preview_limit: Some(1),
            ffmpeg_path: "ffmpeg".to_string(),
            ffprobe_path: "ffprobe".to_string(),
        };

        let cancel = CancellationToken::new();
        let preview = preview_ocr_bitmap_track(&config, &cancel).await.unwrap();
        assert!(preview.contains("OCR line 1"));
        assert!(!output_path.exists());

        let outcome = ocr_bitmap_track_to_srt(&config, OcrCallbacks::default(), &cancel)
            .await
            .unwrap();
        assert_eq!(outcome.subtitle_count, 1);
        assert!(output_path.exists());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn hardcoded_pipeline_and_preview_run_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let mkv = create_hardcoded_mkv(dir.path());
        let output_path = dir.path().join("hardcoded.srt");

        let config = HardcodedOcrConfig {
            input_path: mkv,
            output_path: output_path.clone(),
            roi: VideoRoi {
                x: 0.05,
                y: 0.65,
                width: 0.90,
                height: 0.30,
            },
            sample_fps: 5.0,
            language: Some("en".to_string()),
            tiers: vec![],
            preview_limit: None,
            ffmpeg_path: "ffmpeg".to_string(),
            ffprobe_path: "ffprobe".to_string(),
        };

        let cancel = CancellationToken::new();
        let backend: Arc<dyn OcrBackend> = Arc::new(MockOcrBackend { delay_ms: 0 });
        let outcome =
            ocr_hardcoded_with_backend(&config, backend, OcrCallbacks::default(), &cancel)
                .await
                .unwrap();

        // Cue 1 keeps text, cue 2 returns empty in MockOcrBackend -> 1 cue written.
        assert_eq!(outcome.subtitle_count, 1);
        let parsed = srt_parser::SrtParser::parse_file(&output_path).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[&1].text, "Subtitle cue 1");
    }

    #[tokio::test]
    async fn hardcoded_preview_and_to_srt_work_with_vision_http_mock() {
        let dir = tempfile::tempdir().unwrap();
        let mkv = create_hardcoded_mkv(dir.path());
        let output_path = dir.path().join("hardcoded_http.srt");

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for idx in 1..=2 {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                let header_end = loop {
                    let n = stream.read(&mut chunk).await.unwrap();
                    assert!(n > 0);
                    buf.extend_from_slice(&chunk[..n]);
                    if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        break pos + 4;
                    }
                };
                let headers = String::from_utf8_lossy(&buf[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (k, v) = line.split_once(':')?;
                        k.trim()
                            .eq_ignore_ascii_case("content-length")
                            .then(|| v.trim().parse::<usize>().ok())?
                    })
                    .unwrap_or(0);
                while buf.len() < header_end + content_length {
                    let n = stream.read(&mut chunk).await.unwrap();
                    assert!(n > 0);
                    buf.extend_from_slice(&chunk[..n]);
                }
                let body = format!(
                    r#"{{"choices":[{{"message":{{"role":"assistant","content":"Hardcoded line {idx}"}}}}]}}"#
                );
                let http = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(http.as_bytes()).await.unwrap();
            }
        });

        let config = HardcodedOcrConfig {
            input_path: mkv,
            output_path: output_path.clone(),
            roi: VideoRoi {
                x: 0.05,
                y: 0.65,
                width: 0.90,
                height: 0.30,
            },
            sample_fps: 5.0,
            language: Some("en".to_string()),
            tiers: vec![vec![TierEntry {
                provider: "local".to_string(),
                model: "vision-test".to_string(),
                api_key: None,
                api_url: Some(format!("http://{addr}/v1")),
                rpm: Some(0),
                max_requests: None,
            }]],
            preview_limit: Some(1),
            ffmpeg_path: "ffmpeg".to_string(),
            ffprobe_path: "ffprobe".to_string(),
        };

        let cancel = CancellationToken::new();
        let preview = preview_ocr_hardcoded(&config, &cancel).await.unwrap();
        assert!(preview.contains("Hardcoded line 1"));
        assert!(!output_path.exists());

        let outcome = ocr_hardcoded_to_srt(&config, OcrCallbacks::default(), &cancel)
            .await
            .unwrap();
        assert_eq!(outcome.subtitle_count, 1);
        assert!(output_path.exists());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn composite_backend_fails_over_on_error_and_respects_cancellation() {
        let cue = BitmapSubtitleCue {
            id: 1,
            start_ms: 0,
            end_ms: 1000,
            png_bytes: Vec::new(),
            width: 10,
            height: 10,
        };
        let composite = CompositeOcrBackend::new(vec![
            Arc::new(FailingBackend {
                message: "primary failed",
            }),
            Arc::new(MockOcrBackend { delay_ms: 0 }),
        ])
        .unwrap();

        let cancel = CancellationToken::new();
        let text = composite.recognize_cue(&cue, None, &cancel).await.unwrap();
        assert_eq!(text, "Subtitle cue 1");

        cancel.cancel();
        let err = composite
            .recognize_cue(&cue, None, &cancel)
            .await
            .unwrap_err();
        assert!(err.to_string().to_lowercase().contains("cancel"));
    }

    #[test]
    fn build_backend_from_tiers_recognizes_local_ocr_and_falls_back() {
        // Uncached local_ocr with empty model defaults to paddleocr-v4-mobile-multi.
        let tiers = vec![vec![TierEntry {
            provider: "Paddle".to_string(),
            model: String::new(),
            api_key: None,
            api_url: None,
            rpm: None,
            max_requests: None,
        }]];
        let err = match build_backend_from_tiers(&tiers, None) {
            Ok(_) => panic!("expected error when local OCR model is not downloaded"),
            Err(err) => err,
        };
        assert!(err.to_string().contains(DEFAULT_LOCAL_OCR_MODEL_ID));

        // Mixed tiers: if tier 1 (local_ocr) is uncached, tier 2 (valid Vision LLM) builds.
        let mixed = vec![
            vec![TierEntry {
                provider: "local_ocr".to_string(),
                model: "paddleocr-v4-mobile-multi".to_string(),
                api_key: None,
                api_url: None,
                rpm: None,
                max_requests: None,
            }],
            vec![TierEntry {
                provider: "local".to_string(),
                model: "llava".to_string(),
                api_key: None,
                api_url: Some("http://127.0.0.1:11434/v1".to_string()),
                rpm: None,
                max_requests: None,
            }],
        ];
        assert!(build_backend_from_tiers(&mixed, None).is_ok());
    }
}
