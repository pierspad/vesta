use std::collections::{HashMap, HashSet};

mod export_apkg;
mod generation;
pub use generation::generate;
mod export_tsv;
mod filters;
mod matcher;
mod merge_apkg;
pub use merge_apkg::merge_apkg;
mod parser;
mod snapshot_batch;
mod tsv_publication;
mod types;

pub mod fonts;
pub mod media;

pub use media::{
    H264Encoder, OptimizedVideo, check_ffmpeg, detect_h264_encoder, extract_preview_audio_clip,
    extract_preview_snapshot, optimize_video_source, probe_video_stream, video_has_audio,
};
pub use types::*;

use filters::{apply_filters, apply_span, combine_sentences, compute_context};
use matcher::match_subtitles;
pub use parser::parse_subtitle_file;

#[derive(Debug, Clone)]
pub struct MediaTools {
    pub ffmpeg: String,
    pub ffprobe: String,
}

impl Default for MediaTools {
    fn default() -> Self {
        Self {
            ffmpeg: "ffmpeg".to_string(),
            ffprobe: "ffprobe".to_string(),
        }
    }
}

impl MediaTools {
    pub fn new(ffmpeg: impl Into<String>, ffprobe: impl Into<String>) -> Self {
        let ffmpeg = ffmpeg.into();
        let ffprobe = ffprobe.into();
        Self {
            ffmpeg: if ffmpeg.is_empty() {
                "ffmpeg".into()
            } else {
                ffmpeg
            },
            ffprobe: if ffprobe.is_empty() {
                "ffprobe".into()
            } else {
                ffprobe
            },
        }
    }
}

pub type ProgressCallback<'a> = &'a (dyn Fn(FlashcardProgressEvent) + Send + Sync);

fn emit(
    progress: ProgressCallback<'_>,
    stage: &str,
    message: &str,
    current: usize,
    total: usize,
    pct: f64,
    params: HashMap<String, String>,
) {
    progress(FlashcardProgressEvent {
        stage: stage.to_string(),
        message: message.to_string(),
        current,
        total,
        percentage: pct,
        params,
    });
}

// ─── Subtitle pipeline (shared by `preview` and `generate`) ──────────────────

/// Run the full deterministic subtitle pipeline:
/// parse → time-shift → match → span → filter → combine → context.
///
/// This single source of truth is shared by [`preview`] and [`generate`] so the
/// two can never drift apart (previously this block was copy-pasted between the
/// preview command and the generation routine).
pub fn build_matched_lines(config: &FlashcardConfig) -> Result<Vec<MatchedLine>, String> {
    // Target subtitles + time shift.
    let (mut subs1, _) =
        parse_subtitle_file(&config.target_subs_path).map_err(|e| e.to_string())?;
    shift_subtitles(&mut subs1, config.time_shift_target_ms)?;

    // Optional native subtitles + their own time shift.
    let subs2 = match config.native_subs_path.as_ref() {
        Some(path) => {
            let (mut entries, _) = parse_subtitle_file(path).map_err(|e| e.to_string())?;
            shift_subtitles(&mut entries, config.time_shift_native_ms)?;
            Some(entries)
        }
        None => None,
    };

    // Match dual subtitles, or build a 1:1 list when there is no native track.
    let mut matched = match subs2.as_ref() {
        Some(s2) => match_subtitles(&subs1, s2),
        None => subs1
            .iter()
            .enumerate()
            .map(|(i, s)| MatchedLine {
                index: i,
                subs1: s.clone(),
                subs2: None,
                active: true,
                leading_context: Vec::new(),
                trailing_context: Vec::new(),
            })
            .collect(),
    };

    apply_span(&mut matched, config.span_start_ms, config.span_end_ms);
    apply_filters(&mut matched, &config.filters);
    if config.combine_sentences {
        combine_sentences(&mut matched, &config.continuation_chars);
    }
    compute_context(&mut matched, &config.context);

    Ok(matched)
}

fn shift_subtitles(entries: &mut [SubEntry], shift: i64) -> Result<(), String> {
    for entry in entries {
        entry.start_ms = entry
            .start_ms
            .checked_add(shift)
            .ok_or_else(|| format!("Time shift overflows subtitle {} start", entry.id))?;
        entry.end_ms = entry
            .end_ms
            .checked_add(shift)
            .ok_or_else(|| format!("Time shift overflows subtitle {} end", entry.id))?;
    }
    Ok(())
}

/// Parse + match + filter and return every line (active and inactive) as a
/// [`PreviewLine`], ready to render in a preview table. Pure CPU work, no media.
pub fn preview(config: &FlashcardConfig) -> Result<Vec<PreviewLine>, String> {
    let matched = build_matched_lines(config)?;
    Ok(matched
        .iter()
        .map(|m| PreviewLine {
            index: m.index,
            subs1_text: m.subs1.text.clone(),
            subs2_text: m.subs2.as_ref().map(|s| s.text.clone()),
            start_ms: m.subs1.start_ms,
            end_ms: m.subs1.end_ms,
            duration_ms: m.subs1.end_ms - m.subs1.start_ms,
            active: m.active,
            actor: m.subs1.actor.clone(),
            leading_context: m.leading_context.clone(),
            trailing_context: m.trailing_context.clone(),
        })
        .collect())
}

/// Parse a subtitle file and summarise it (count, format, actors, duration).
pub fn load_sub_file_info(path: &str) -> Result<SubFileInfo, String> {
    let (entries, format) = parse_subtitle_file(path).map_err(|e| e.to_string())?;

    let mut actors: Vec<String> = entries
        .iter()
        .filter_map(|e| e.actor.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    actors.sort();

    Ok(SubFileInfo {
        path: path.to_string(),
        format: format.to_string(),
        count: entries.len(),
        first_text: entries.first().map(|e| e.text.clone()).unwrap_or_default(),
        last_text: entries.last().map(|e| e.text.clone()).unwrap_or_default(),
        actors,
        duration_ms: entries.last().map(|e| e.end_ms).unwrap_or(0),
    })
}

/// Probe a media file and return its audio streams in ffmpeg `0:a:N` order.
pub async fn list_audio_tracks(
    path: &str,
    ffprobe_cmd: &str,
) -> Result<Vec<AudioTrackInfo>, String> {
    let output = media::media_command(ffprobe_cmd)
        .args([
            "-v",
            "error",
            "-select_streams",
            "a",
            "-show_entries",
            "stream=index,codec_name,channels:stream_tags=language,title",
            "-of",
            "json",
            path,
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run ffprobe: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse ffprobe output: {}", e))?;
    let streams = value
        .get("streams")
        .and_then(|streams| streams.as_array())
        .ok_or_else(|| "ffprobe did not return stream data".to_string())?;

    Ok(streams
        .iter()
        .enumerate()
        .map(|(audio_index, stream)| {
            let tags = stream.get("tags");
            AudioTrackInfo {
                index: audio_index,
                stream_index: stream
                    .get("index")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(audio_index as u64) as usize,
                codec: stream
                    .get("codec_name")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                language: tags
                    .and_then(|tags| tags.get("language"))
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_string),
                title: tags
                    .and_then(|tags| tags.get("title"))
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_string),
                channels: stream
                    .get("channels")
                    .and_then(|value| value.as_u64())
                    .map(|value| value as u32),
            }
        })
        .collect())
}
