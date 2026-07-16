use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio_util::sync::CancellationToken;
use whisper_rs::WhisperContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscribedSegment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

pub type SegmentCallback = std::sync::Arc<dyn Fn(i64, i64, &str) + Send + Sync + 'static>;

#[derive(Default)]
pub struct TranscribeOptions {
    pub language: Option<String>,
    pub translate_to_english: bool,
    pub n_threads: Option<usize>,
    pub word_timestamps: bool,
    pub max_segment_length: Option<u32>,
    /// Beam width for beam-search decoding. `None` (or `Some(1)`) keeps the
    /// default greedy decoder; `Some(5)` is whisper.cpp's "quality" setting
    pub beam_size: Option<u32>,

    pub vad_model_path: Option<std::path::PathBuf>,
    pub segment_callback: Option<SegmentCallback>,
}

impl std::fmt::Debug for TranscribeOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranscribeOptions")
            .field("language", &self.language)
            .field("translate_to_english", &self.translate_to_english)
            .field("n_threads", &self.n_threads)
            .field("word_timestamps", &self.word_timestamps)
            .field("max_segment_length", &self.max_segment_length)
            .field("beam_size", &self.beam_size)
            .field("vad_model_path", &self.vad_model_path)
            .field(
                "segment_callback",
                &self.segment_callback.as_ref().map(|_| "Some(Fn)"),
            )
            .finish()
    }
}

impl Clone for TranscribeOptions {
    fn clone(&self) -> Self {
        Self {
            language: self.language.clone(),
            translate_to_english: self.translate_to_english,
            n_threads: self.n_threads,
            word_timestamps: self.word_timestamps,
            max_segment_length: self.max_segment_length,
            beam_size: self.beam_size,
            vad_model_path: self.vad_model_path.clone(),
            segment_callback: self.segment_callback.clone(),
        }
    }
}

/// Default worker count for whisper.cpp: the *physical* cores, capped at 8.
///
/// GGML's matrix kernels are memory-bandwidth-bound: SMT/hyper-threads add

pub fn default_n_threads() -> usize {
    num_cpus::get_physical().clamp(1, 8)
}

pub fn transcribe_full(
    ctx: &WhisperContext,
    audio_data: &[f32],
    options: &TranscribeOptions,
    cancel_token: Option<&CancellationToken>,
) -> Result<(Vec<TranscribedSegment>, Option<String>)> {
    let mut state = ctx
        .create_state()
        .map_err(|e| anyhow::anyhow!("Failed to create Whisper state: {:?}", e))?;

    let Some(ref vad_model_path) = options.vad_model_path else {
        return transcribe_span(&mut state, audio_data, options, 0, cancel_token);
    };

    let spans = vad_speech_spans(vad_model_path, audio_data, options)?;
    let mut segments = Vec::new();
    let mut detected_language = None;

    for span in spans {
        if let Some(token) = cancel_token
            && token.is_cancelled()
        {
            anyhow::bail!("Transcription cancelled");
        }

        let offset_ms = (span.start / (SAMPLE_RATE / 1000)) as i64;
        let (mut span_segments, span_language) = transcribe_span(
            &mut state,
            &audio_data[span],
            options,
            offset_ms,
            cancel_token,
        )?;
        detected_language = detected_language.or(span_language);
        segments.append(&mut span_segments);
    }

    Ok((segments, detected_language))
}

fn transcribe_span(
    state: &mut whisper_rs::WhisperState,
    audio_data: &[f32],
    options: &TranscribeOptions,
    offset_ms: i64,
    cancel_token: Option<&CancellationToken>,
) -> Result<(Vec<TranscribedSegment>, Option<String>)> {
    let strategy = match options.beam_size {
        Some(beam_size @ 2..) => whisper_rs::SamplingStrategy::BeamSearch {
            beam_size: beam_size as i32,
            patience: -1.0,
        },
        _ => whisper_rs::SamplingStrategy::Greedy { best_of: 1 },
    };
    let mut params = whisper_rs::FullParams::new(strategy);

    if let Some(ref lang) = options.language {
        if lang != "auto" {
            params.set_language(Some(lang));
        } else {
            params.set_language(None);
        }
    } else {
        params.set_language(None);
    }

    params.set_translate(options.translate_to_english);

    let threads = options.n_threads.unwrap_or_else(default_n_threads);
    params.set_n_threads(threads as i32);

    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_special(false);
    params.set_print_timestamps(false);
    params.set_token_timestamps(options.word_timestamps);

    if let Some(max_len) = options.max_segment_length
        && max_len > 0
    {
        params.set_max_len(max_len as i32);
    }

    if let Some(ref cb) = options.segment_callback {
        let cb = cb.clone();
        params.set_segment_callback_safe(move |data: whisper_rs::SegmentCallbackData| {
            cb(
                offset_ms + data.start_timestamp * 10,
                offset_ms + data.end_timestamp * 10,
                &data.text,
            );
        });
    }

    if let Some(token) = cancel_token {
        if token.is_cancelled() {
            anyhow::bail!("Transcription cancelled");
        }

        unsafe extern "C" fn whisper_abort_callback(user_data: *mut std::ffi::c_void) -> bool {
            if user_data.is_null() {
                return false;
            }
            let token = unsafe { &*(user_data as *const tokio_util::sync::CancellationToken) };
            token.is_cancelled()
        }
        unsafe {
            params.set_abort_callback(Some(whisper_abort_callback));
            params.set_abort_callback_user_data(
                token as *const tokio_util::sync::CancellationToken as *mut std::ffi::c_void,
            );
        }
    }

    state
        .full(params, audio_data)
        .map_err(|e| anyhow::anyhow!("Whisper transcription failed: {:?}", e))?;

    if let Some(token) = cancel_token
        && token.is_cancelled()
    {
        anyhow::bail!("Transcription cancelled");
    }

    let detected_language = {
        let lang_id = state.full_lang_id_from_state();
        if lang_id >= 0 {
            whisper_rs::get_lang_str(lang_id).map(|s| s.to_string())
        } else {
            None
        }
    };

    let n_segments = state.full_n_segments();
    let mut segments = Vec::with_capacity(n_segments as usize);

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
            start_ms: offset_ms + seg.start_timestamp() * 10,
            end_ms: offset_ms + seg.end_timestamp() * 10,
            text,
        });
    }

    Ok((segments, detected_language))
}

const SAMPLE_RATE: usize = 16_000;

const VAD_MERGE_GAP_MS: i64 = 2_000;

const VAD_PAD_MS: i64 = 150;

pub fn vad_speech_spans_ms(
    model_path: &std::path::Path,
    samples: &[f32],
    n_threads: usize,
    pad_ms: i64,
    merge_gap_ms: i64,
) -> Result<Vec<(i64, i64)>> {
    let mut ctx_params = whisper_rs::WhisperVadContextParams::default();
    ctx_params.set_n_threads(n_threads as i32);

    let mut vad = whisper_rs::WhisperVadContext::new(&model_path.to_string_lossy(), ctx_params)
        .map_err(|e| anyhow::anyhow!("Failed to load VAD model {}: {e:?}", model_path.display()))?;
    let detected = vad
        .segments_from_samples(whisper_rs::WhisperVadParams::default(), samples)
        .map_err(|e| anyhow::anyhow!("VAD speech detection failed: {e:?}"))?;

    let mut spans_ms: Vec<(i64, i64)> = Vec::new();
    for segment in detected {
        let start_ms = (segment.start as i64) * 10 - pad_ms;
        let end_ms = (segment.end as i64) * 10 + pad_ms;
        match spans_ms.last_mut() {
            Some((_, last_end)) if start_ms - *last_end <= merge_gap_ms => {
                *last_end = end_ms.max(*last_end);
            }
            _ => spans_ms.push((start_ms.max(0), end_ms)),
        }
    }
    Ok(spans_ms)
}

fn vad_speech_spans(
    model_path: &std::path::Path,
    samples: &[f32],
    options: &TranscribeOptions,
) -> Result<Vec<std::ops::Range<usize>>> {
    let spans_ms = vad_speech_spans_ms(
        model_path,
        samples,
        options.n_threads.unwrap_or_else(default_n_threads),
        VAD_PAD_MS,
        VAD_MERGE_GAP_MS,
    )?;

    let per_ms = SAMPLE_RATE / 1000;
    Ok(spans_ms
        .into_iter()
        .map(|(start_ms, end_ms)| {
            let end = ((end_ms as usize) * per_ms).min(samples.len());
            let start = ((start_ms as usize) * per_ms).min(end);
            start..end
        })
        .filter(|span| !span.is_empty())
        .collect())
}

pub fn text_similarity(left: &str, right: &str) -> f64 {
    let token_score = token_overlap_score(left, right);
    let char_score = normalized_char_similarity(left, right);
    token_score * 0.7 + char_score * 0.3
}

fn token_overlap_score(left: &str, right: &str) -> f64 {
    let left_tokens = normalized_tokens(left);
    let right_tokens = normalized_tokens(right);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0.0;
    }

    let right_set = right_tokens.iter().collect::<HashSet<_>>();
    let overlap = left_tokens
        .iter()
        .filter(|token| right_set.contains(token))
        .count() as f64;
    let precision = overlap / right_tokens.len() as f64;
    let recall = overlap / left_tokens.len() as f64;
    if precision + recall == 0.0 {
        0.0
    } else {
        (2.0 * precision * recall) / (precision + recall)
    }
}

fn normalized_char_similarity(left: &str, right: &str) -> f64 {
    let left = normalize_text(left);
    let right = normalize_text(right);
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    let distance = levenshtein_distance(&left_chars, &right_chars) as f64;
    let max_len = left_chars.len().max(right_chars.len()) as f64;
    (1.0 - distance / max_len).max(0.0)
}

fn levenshtein_distance(left: &[char], right: &[char]) -> usize {
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];

    for (i, left_char) in left.iter().enumerate() {
        current[0] = i + 1;
        for (j, right_char) in right.iter().enumerate() {
            let cost = usize::from(left_char != right_char);
            current[j + 1] = (previous[j + 1] + 1)
                .min(current[j] + 1)
                .min(previous[j] + cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right.len()]
}

pub fn normalized_tokens(value: &str) -> Vec<String> {
    normalize_text(value)
        .split_whitespace()
        .filter(|token| token.len() > 1)
        .map(str::to_string)
        .collect()
}

pub fn normalize_text(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch.is_whitespace() {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
