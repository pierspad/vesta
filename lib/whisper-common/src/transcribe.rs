use anyhow::Result;
use serde::{Deserialize, Serialize};
use whisper_rs::WhisperContext;
use tokio_util::sync::CancellationToken;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscribedSegment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct TranscribeOptions {
    pub language: Option<String>,
    pub translate_to_english: bool,
    pub n_threads: Option<usize>,
    pub word_timestamps: bool,
    pub max_segment_length: Option<u32>,
}

/// Run full transcription on the whole audio sample, returning the segments and the detected language
pub fn transcribe_full(
    ctx: &WhisperContext,
    audio_data: &[f32],
    options: &TranscribeOptions,
    cancel_token: Option<&CancellationToken>,
) -> Result<(Vec<TranscribedSegment>, Option<String>)> {
    let mut state = ctx
        .create_state()
        .map_err(|e| anyhow::anyhow!("Failed to create Whisper state: {:?}", e))?;
        
    let mut params = whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
    
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
    
    let threads = options.n_threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .min(8)
    });
    params.set_n_threads(threads as i32);
    
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_special(false);
    params.set_print_timestamps(false);
    params.set_token_timestamps(options.word_timestamps);
    
    if let Some(max_len) = options.max_segment_length {
        if max_len > 0 {
            params.set_max_len(max_len as i32);
        }
    }
    
    if let Some(token) = cancel_token {
        if token.is_cancelled() {
            anyhow::bail!("Transcription cancelled");
        }
    }
    
    state
        .full(params, audio_data)
        .map_err(|e| anyhow::anyhow!("Whisper transcription failed: {:?}", e))?;
        
    if let Some(token) = cancel_token {
        if token.is_cancelled() {
            anyhow::bail!("Transcription cancelled");
        }
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
        
        // Whisper timestamps are in centiseconds (10ms units)
        segments.push(TranscribedSegment {
            start_ms: seg.start_timestamp() * 10,
            end_ms: seg.end_timestamp() * 10,
            text,
        });
    }
    
    Ok((segments, detected_language))
}

/// Run chunked sliding-window transcription on the audio sample with deduplication, returning the segments and detected language of the first chunk
pub fn transcribe_chunked(
    ctx: &WhisperContext,
    audio_data: &[f32],
    options: &TranscribeOptions,
    cancel_token: Option<&CancellationToken>,
) -> Result<(Vec<TranscribedSegment>, Option<String>)> {
    let sample_rate = 16_000usize;
    let chunk_samples = sample_rate * 45;
    let step_samples = sample_rate * 40;
    
    if audio_data.len() <= chunk_samples {
        return transcribe_full(ctx, audio_data, options, cancel_token);
    }
    
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut detected_language = None;
    
    while start < audio_data.len() {
        if let Some(token) = cancel_token {
            if token.is_cancelled() {
                anyhow::bail!("Transcription cancelled");
            }
        }
        
        let end = (start + chunk_samples).min(audio_data.len());
        let offset_ms = (start as i64 * 1000) / sample_rate as i64;
        let chunk = &audio_data[start..end];
        
        let (chunk_segments, chunk_lang) = transcribe_full(ctx, chunk, options, cancel_token)?;
        
        if detected_language.is_none() {
            detected_language = chunk_lang;
        }
        
        for mut segment in chunk_segments {
            segment.start_ms += offset_ms;
            segment.end_ms += offset_ms;
            
            let is_duplicate = segments.iter().any(|existing: &TranscribedSegment| {
                (existing.start_ms - segment.start_ms).abs() < 2_500
                    && text_similarity(&existing.text, &segment.text) > 0.72
            });
            
            if !is_duplicate {
                segments.push(segment);
            }
        }
        
        if end == audio_data.len() {
            break;
        }
        start += step_samples;
    }
    
    segments.sort_by_key(|segment| segment.start_ms);
    Ok((segments, detected_language))
}

// ─── Text Similarity Utilities for Deduplication ────────────────────────────

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
