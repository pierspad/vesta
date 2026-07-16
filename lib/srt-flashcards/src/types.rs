use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubEntry {
    pub id: u32,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,

    pub actor: Option<String>,

    pub style: Option<String>,

    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedLine {
    pub index: usize,
    pub subs1: SubEntry,
    pub subs2: Option<SubEntry>,
    pub active: bool,

    pub leading_context: Vec<usize>,

    pub trailing_context: Vec<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubFileInfo {
    pub path: String,
    pub format: String,
    pub count: usize,
    pub first_text: String,
    pub last_text: String,

    pub actors: Vec<String>,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlashcardConfig {
    pub target_subs_path: String,
    pub native_subs_path: Option<String>,
    pub video_path: Option<String>,
    pub audio_path: Option<String>,
    pub output_dir: String,

    #[allow(dead_code)]
    pub use_timings_from: String,
    pub span_start_ms: Option<i64>,
    pub span_end_ms: Option<i64>,
    pub time_shift_target_ms: i64,
    pub time_shift_native_ms: i64,

    pub filters: SubtitleFilters,

    pub context: ContextConfig,

    pub combine_sentences: bool,
    pub continuation_chars: String,

    pub generate_audio: bool,
    pub audio_bitrate: u32,
    pub audio_track_index: Option<usize>,
    pub normalize_audio: bool,
    pub audio_pad_start_ms: i64,
    pub audio_pad_end_ms: i64,

    pub generate_snapshots: bool,
    pub snapshot_width: u32,
    pub snapshot_height: u32,
    pub crop_bottom: u32,

    pub generate_video_clips: bool,
    pub video_codec: String,
    pub h264_preset: String,

    #[serde(default = "default_video_hw_accel")]
    pub video_hw_accel: String,
    pub video_bitrate: u32,
    pub video_audio_bitrate: u32,
    pub video_pad_start_ms: i64,
    pub video_pad_end_ms: i64,

    pub deck_name: String,
    pub episode_number: u32,

    pub export_format: Option<String>,

    pub note_type_name: Option<String>,

    pub field_names: Option<FieldNamesConfig>,

    pub output_fields: OutputFields,

    pub cpu_cores: Option<usize>,

    pub card_front_html: Option<String>,
    pub card_back_html: Option<String>,
    pub card_css: Option<String>,
}

fn default_video_hw_accel() -> String {
    "auto".to_string()
}

impl Default for FlashcardConfig {
    fn default() -> Self {
        Self {
            target_subs_path: String::new(),
            native_subs_path: None,
            video_path: None,
            audio_path: None,
            output_dir: String::new(),
            use_timings_from: "target".to_string(),
            span_start_ms: None,
            span_end_ms: None,
            time_shift_target_ms: 0,
            time_shift_native_ms: 0,
            filters: SubtitleFilters::default(),
            context: ContextConfig::default(),
            combine_sentences: false,
            continuation_chars: String::new(),
            generate_audio: false,
            audio_bitrate: 128,
            audio_track_index: None,
            normalize_audio: false,
            audio_pad_start_ms: 0,
            audio_pad_end_ms: 0,
            generate_snapshots: false,
            snapshot_width: 240,
            snapshot_height: 160,
            crop_bottom: 0,
            generate_video_clips: false,
            video_codec: "h264".to_string(),
            h264_preset: "ultrafast".to_string(),
            video_hw_accel: default_video_hw_accel(),
            video_bitrate: 1000,
            video_audio_bitrate: 128,
            video_pad_start_ms: 0,
            video_pad_end_ms: 0,
            deck_name: String::new(),
            episode_number: 1,
            export_format: Some("tsv".to_string()),
            note_type_name: None,
            field_names: None,
            output_fields: OutputFields::default(),
            cpu_cores: None,
            card_front_html: None,
            card_back_html: None,
            card_css: None,
        }
    }
}

impl FlashcardConfig {
    pub fn benchmark(
        target_subs_path: String,
        native_subs_path: String,
        video_path: String,
        output_dir: String,
        export_format: String,
        has_audio: bool,
        cpu_cores: Option<usize>,
    ) -> Self {
        Self {
            target_subs_path,
            native_subs_path: Some(native_subs_path),
            audio_path: has_audio.then(|| video_path.clone()),
            video_path: Some(video_path),
            output_dir,
            generate_audio: has_audio,
            generate_snapshots: true,
            generate_video_clips: true,
            deck_name: "BenchmarkDeck".to_string(),
            export_format: Some(export_format),
            output_fields: OutputFields {
                include_audio: true,
                include_snapshot: true,
                include_video: true,
                ..OutputFields::default()
            },
            cpu_cores,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
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

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ContextConfig {
    pub leading: usize,
    pub trailing: usize,
    pub max_gap_seconds: f64,
}

fn default_true() -> bool {
    true
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

    #[serde(default = "default_true")]
    pub include_reading: bool,
    #[serde(default = "default_true")]
    pub include_notes: bool,
}

impl Default for OutputFields {
    fn default() -> Self {
        Self {
            include_tag: true,
            include_sequence: true,
            include_audio: false,
            include_snapshot: false,
            include_video: false,
            include_subs1: true,
            include_subs2: true,
            include_reading: true,
            include_notes: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldNamesConfig {
    pub expression: String,
    pub meaning: String,
    pub reading: String,
    pub audio: String,
    pub snapshot: String,
    pub video: String,
    pub tags: String,
    pub sequence_marker: String,
    pub notes: String,
}

impl Default for FieldNamesConfig {
    fn default() -> Self {
        Self {
            expression: "Expression".to_string(),
            meaning: "Meaning".to_string(),
            reading: "Reading".to_string(),
            audio: "Audio".to_string(),
            snapshot: "Snapshot".to_string(),
            video: "Video".to_string(),
            tags: "Tags".to_string(),
            sequence_marker: "SequenceMarker".to_string(),
            notes: "Notes".to_string(),
        }
    }
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

#[derive(Debug, Clone, Serialize)]
pub struct FlashcardProgressEvent {
    pub stage: String,
    pub message: String,
    pub current: usize,
    pub total: usize,
    pub percentage: f64,
    pub params: HashMap<String, String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
