//! Comandi Tauri per la generazione di flashcard Anki da sottotitoli.
//!
//! Implementazione completa ispirata a subs2srs: parsing doppi sottotitoli,
//! matching temporale, estrazione audio/snapshot/video via FFmpeg,
//! generazione TSV per Anki, filtri avanzati, context lines.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Data Types ──────────────────────────────────────────────────────────────

/// A parsed subtitle entry (supports SRT, ASS, VTT)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubEntry {
    pub id: u32,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    /// Actor name (ASS/SSA only)
    pub actor: Option<String>,
    /// Style name (ASS/SSA only)
    pub style: Option<String>,
    /// Whether this line is active (passes filters)
    pub active: bool,
}

/// Matched pair of subs1 + subs2 lines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedLine {
    pub index: usize,
    pub subs1: SubEntry,
    pub subs2: Option<SubEntry>,
    pub active: bool,
    /// Context: indices of leading lines
    pub leading_context: Vec<usize>,
    /// Context: indices of trailing lines
    pub trailing_context: Vec<usize>,
}

/// Info returned after loading a subtitle file
#[derive(Debug, Clone, Serialize)]
pub struct SubFileInfo {
    pub path: String,
    pub format: String,
    pub count: usize,
    pub first_text: String,
    pub last_text: String,
    /// Unique actors found (ASS only)
    pub actors: Vec<String>,
    pub duration_ms: i64,
}

/// Full flashcard generation configuration from the frontend
#[derive(Debug, Clone, Deserialize)]
pub struct FlashcardConfig {
    // Files
    pub target_subs_path: String,
    pub native_subs_path: Option<String>,
    pub video_path: Option<String>,
    pub audio_path: Option<String>,
    pub output_dir: String,

    // Subtitle options
    #[allow(dead_code)]
    pub use_timings_from: String, // "target" or "native"
    pub span_start_ms: Option<i64>,
    pub span_end_ms: Option<i64>,
    pub time_shift_target_ms: i64,
    pub time_shift_native_ms: i64,

    // Filters
    pub filters: SubtitleFilters,

    // Context lines
    pub context: ContextConfig,

    // Sentence combining
    pub combine_sentences: bool,
    pub continuation_chars: String,

    // Audio
    pub generate_audio: bool,
    pub audio_bitrate: u32,
    pub audio_track_index: Option<usize>,
    pub normalize_audio: bool,
    pub audio_pad_start_ms: i64,
    pub audio_pad_end_ms: i64,

    // Snapshots
    pub generate_snapshots: bool,
    pub snapshot_width: u32,
    pub snapshot_height: u32,
    pub crop_bottom: u32,

    // Video clips
    pub generate_video_clips: bool,
    pub video_codec: String, // "h264" or "mpeg4"
    pub h264_preset: String, // ultrafast..placebo
    pub video_bitrate: u32,
    pub video_audio_bitrate: u32,
    pub video_pad_start_ms: i64,
    pub video_pad_end_ms: i64,

    // Naming
    pub deck_name: String,
    pub episode_number: u32,

    // Export format: "tsv" or "apkg"
    pub export_format: Option<String>,

    // Note type name for Anki
    pub note_type_name: Option<String>,

    // Output fields
    pub output_fields: OutputFields,

    // Performance: CPU cores to use (optional, defaults to 3/4 of available)
    pub cpu_cores: Option<usize>,

    // Custom Anki card templates (optional, overrides built-in defaults)
    pub card_front_html: Option<String>,
    pub card_back_html: Option<String>,
    pub card_css: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubtitleFilters {
    pub include_words: Option<String>,
    pub exclude_words: Option<String>,
    pub exclude_duplicates_subs1: bool,
    pub exclude_duplicates_subs2: bool,
    pub min_chars: Option<usize>,
    pub max_chars: Option<usize>,
    pub min_duration_ms: Option<i64>,
    pub max_duration_ms: Option<i64>,
    pub exclude_styled: bool,
    pub actor_filter: Option<String>,
    pub only_cjk: bool,
    pub remove_no_match: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContextConfig {
    pub leading: usize,
    pub trailing: usize,
    pub max_gap_seconds: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OutputFields {
    pub include_tag: bool,
    pub include_sequence: bool,
    pub include_audio: bool,
    pub include_snapshot: bool,
    pub include_video: bool,
    pub include_subs1: bool,
    pub include_subs2: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioTrackInfo {
    pub index: usize,
    pub stream_index: usize,
    pub codec: Option<String>,
    pub language: Option<String>,
    pub title: Option<String>,
    pub channels: Option<u32>,
}

/// Progress event emitted to frontend
#[derive(Debug, Clone, Serialize)]
pub struct FlashcardProgressEvent {
    pub stage: String,
    pub message: String,
    pub current: usize,
    pub total: usize,
    pub percentage: f64,
    pub params: HashMap<String, String>,
}

/// Final result
#[derive(Debug, Clone, Serialize)]
pub struct FlashcardResult {
    pub success: bool,
    pub message: String,
    pub cards_generated: usize,
    pub audio_clips: usize,
    pub snapshots: usize,
    pub video_clips: usize,
    pub tsv_path: Option<String>,
    pub apkg_path: Option<String>,
}

/// Preview data for a single line
#[derive(Debug, Clone, Serialize)]
pub struct PreviewLine {
    pub index: usize,
    pub subs1_text: String,
    pub subs2_text: Option<String>,
    pub start_ms: i64,
    pub end_ms: i64,
    pub duration_ms: i64,
    pub active: bool,
    pub actor: Option<String>,
    pub leading_context: Vec<usize>,
    pub trailing_context: Vec<usize>,
}
