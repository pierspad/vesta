use crate::paths::Paths;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use vesta_flashcards::FlashcardConfig;

/// Translate a subtitle file with a provider profile configured by the server.
/// Provider secrets are never accepted in tool arguments.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TranslateRequest {
    /// Absolute path to an SRT (.srt) file within a configured read root.
    pub input: String,
    /// Target language name or code, for example "Italian" or "it".
    pub target_language: String,
    /// Name of a server-configured translation profile.
    pub profile_id: String,
}

/// Transcribe a media file using a provider profile configured by the server.
/// Provider secrets are never accepted in tool arguments.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TranscribeRequest {
    /// Absolute path to an audio or video file within a configured read root.
    pub input: String,
    /// Name of a server-configured transcription profile.
    pub profile_id: String,
    /// Optional ISO language code or "auto" for backend language detection.
    pub language: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SubtitleRequest {
    /// Absolute path to a subtitle file within a configured read root.
    pub path: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewRequest {
    pub target: String,
    pub native: Option<String>,
    /// Zero-based offset into the preview. Defaults to zero.
    #[serde(default)]
    pub offset: usize,
    /// Maximum rows returned, from 1 to 100. Defaults to 20.
    #[serde(default = "preview_limit")]
    pub limit: usize,
}
fn preview_limit() -> usize {
    20
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Apkg,
    Tsv,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateRequest {
    pub target: String,
    pub native: Option<String>,
    pub video: Option<String>,
    pub audio: Option<String>,
    /// Deck title, from 1 to 256 characters.
    pub deck: String,
    pub format: Format,
    pub note_type: Option<String>,
    #[serde(default)]
    pub include_audio: bool,
    #[serde(default)]
    pub include_snapshots: bool,
    #[serde(default)]
    pub include_video: bool,
    /// Maximum concurrent media workers, from 1 to 64; capped to available cores.
    pub workers: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JobRequest {
    pub job_id: String,
}

/// List embedded subtitle tracks inside a media container within a configured read root.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmbeddedSubtitlesRequest {
    /// Absolute path to a media container within a configured read root.
    pub input: String,
}

/// Normalized video region of interest (`0.0..=1.0`) for hardcoded subtitle OCR.
#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OcrRoiRequest {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl OcrRoiRequest {
    /// Converts into [`vesta_ocr::VideoRoi`] and validates normalized coordinates.
    pub fn into_video_roi(self) -> Result<vesta_ocr::VideoRoi, String> {
        let roi = vesta_ocr::VideoRoi {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        };
        roi.validate().map_err(|e| e.to_string())?;
        Ok(roi)
    }
}

/// Extract and OCR a bitmap subtitle track or burned-in video ROI using a server-configured profile.
/// Provider secrets are never accepted in tool arguments.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OcrSubtitlesRequest {
    /// Absolute path to a media container within a configured read root.
    pub input: String,
    /// Absolute FFprobe subtitle stream index from `list_embedded_subtitles` (mutually exclusive with `roi`).
    pub track_index: Option<u32>,
    /// Normalized video ROI (`0.0..=1.0`) for hardcoded/burned-in subtitles (mutually exclusive with `track_index`).
    pub roi: Option<OcrRoiRequest>,
    /// Frame sampling rate in FPS (`0.5..=15.0`, defaults to `2.5`) when `roi` is used.
    pub sample_fps: Option<f64>,
    /// Name of a server-configured translation/OCR profile providing Vision LLM or local OCR tiers.
    pub profile_id: String,
    /// Optional ISO language code or language name hint for OCR.
    pub language: Option<String>,
}

pub fn subtitle_config(
    paths: &Paths,
    target: &str,
    native: Option<&str>,
) -> Result<FlashcardConfig, String> {
    Ok(FlashcardConfig {
        target_subs_path: paths.input(target.as_ref(), true)?,
        native_subs_path: native
            .map(|path| paths.input(path.as_ref(), true))
            .transpose()?,
        // The MCP adapter never downloads fonts implicitly.
        auto_card_font: false,
        embed_card_font: false,
        ..FlashcardConfig::default()
    })
}

impl GenerateRequest {
    pub fn config(&self, paths: &Paths) -> Result<FlashcardConfig, String> {
        if self.deck.trim().is_empty() || self.deck.chars().count() > 256 {
            return Err("Deck title must contain 1 to 256 characters".into());
        }
        if self.workers.is_some_and(|n| !(1..=64).contains(&n)) {
            return Err("Workers must be from 1 to 64".into());
        }
        if self
            .note_type
            .as_ref()
            .is_some_and(|name| name.trim().is_empty() || name.chars().count() > 128)
        {
            return Err("Note type must contain 1 to 128 characters".into());
        }
        let mut config = subtitle_config(paths, &self.target, self.native.as_deref())?;
        config.video_path = self
            .video
            .as_ref()
            .map(|p| paths.input(p.as_ref(), false))
            .transpose()?;
        config.audio_path = self
            .audio
            .as_ref()
            .map(|p| paths.input(p.as_ref(), false))
            .transpose()?;
        if self.include_audio && config.video_path.is_none() && config.audio_path.is_none() {
            return Err("Audio generation requires a video or audio input".into());
        }
        if (self.include_snapshots || self.include_video) && config.video_path.is_none() {
            return Err("Snapshots and video clips require a video input".into());
        }
        config.deck_name = self.deck.clone();
        config.export_format = Some(
            match self.format {
                Format::Apkg => "apkg",
                Format::Tsv => "tsv",
            }
            .into(),
        );
        config.note_type_name = self.note_type.clone();
        config.generate_audio = self.include_audio;
        config.generate_snapshots = self.include_snapshots;
        config.generate_video_clips = self.include_video;
        config.cpu_cores = self.workers;
        Ok(config)
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PluginCallRequest {
    /// ID of the installed plugin to call.
    pub plugin_id: String,
    /// Screen name to render or dispatch event on. Defaults to "main".
    pub screen: Option<String>,
    /// Optional widget ID for event dispatch.
    pub widget_id: Option<String>,
    /// Optional event type (e.g. "click", "change", "submit"). Defaults to "click".
    pub event_type: Option<String>,
    /// Optional snapshot of widget values.
    pub values: Option<Value>,
    /// Optional expert mode flag.
    pub expert_mode: Option<bool>,
}

/// Extract an embedded text subtitle track from a media container within a configured read root.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExtractEmbeddedSubtitleRequest {
    /// Absolute path to a media container within a configured read root.
    pub input: String,
    /// Absolute FFprobe subtitle stream index from `list_embedded_subtitles`.
    pub track_index: u32,
}

/// Explicit subtitle-ID to corrected-timestamp anchor for `sync_subtitles`.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SyncAnchorRequest {
    /// Positive 1-based subtitle ID in the input SRT file.
    pub subtitle_id: usize,
    /// Corrected subtitle start timestamp in milliseconds (non-negative).
    pub corrected_time_ms: i64,
}

/// Retime an SRT subtitle file using offset/speed scaling or explicit manual anchors.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SyncSubtitlesRequest {
    /// Absolute path to an SRT (.srt) file within a configured read root.
    pub input: String,
    /// Constant time shift in milliseconds (mutually exclusive with `anchors`).
    pub offset_ms: Option<i64>,
    /// Linear playback speed factor (`0.5..=2.0`, defaults to `1.0`, mutually exclusive with `anchors`).
    pub speed: Option<f64>,
    /// Explicit anchor points applied atomically (mutually exclusive with `offset_ms` / `speed`).
    pub anchors: Option<Vec<SyncAnchorRequest>>,
}

/// Automatically align an SRT file against media audio using a server-configured STT profile.
/// Provider secrets are never accepted in tool arguments.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AutosyncSubtitlesRequest {
    /// Absolute path to an SRT (.srt) file within a configured read root.
    pub input: String,
    /// Absolute path to a video or audio file within a configured read root.
    pub media: String,
    /// Name of a server-configured transcription profile (local Whisper or cloud STT).
    pub profile_id: String,
    /// Optional ISO language code or `"auto"` for automatic language detection.
    pub language: Option<String>,
    /// Use fewer, shorter audio samples for faster alignment (defaults to `false`).
    pub quick: Option<bool>,
    /// Maximum allowed subtitle offset window in seconds (`1..=3600`, defaults to `180`).
    pub max_offset_secs: Option<f64>,
}

/// Annotate cards in a `.tsv` or `.apkg` deck with an LLM profile configured by the server.
/// Provider secrets are never accepted in tool arguments.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RefineFlashcardsRequest {
    /// Absolute path to a `.tsv` or `.apkg` deck inside a configured read root or output root.
    pub input: String,
    /// Name of a server-configured translation/LLM profile (non-OCR endpoints).
    pub profile_id: String,
    /// Non-empty prompt template for annotating cards (`{{expression}}`, `{{meaning}}`, `{{notes}}`).
    pub prompt: String,
    /// Output deck format (`apkg` or `tsv`). Defaults to the input format (`tsv -> apkg` is not supported).
    pub output_format: Option<Format>,
    /// Group multiple cards into each LLM request when `true` (defaults to `false`).
    pub batch_mode: Option<bool>,
    /// Number of cards per batch when `batch_mode` is `true` (`1..=50`, defaults to `8`).
    pub batch_size: Option<usize>,
    /// Write completed annotations to output even if some cards fail (defaults to `false`).
    pub allow_partial: Option<bool>,
}

/// Output subtitle format for bilingual dueling subtitles.
#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DuelingOutputFormat {
    Ass,
    Srt,
}

/// Export bilingual dueling subtitles (`.ass` or `.srt`) from target and native tracks.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportDuelingRequest {
    /// Absolute path to target-language subtitles within a configured read root.
    pub target: String,
    /// Absolute path to native-language subtitles within a configured read root.
    pub native: String,
    /// Output format (`ass` or `srt`).
    pub format: DuelingOutputFormat,
    /// Place target subtitles first (bottom in ASS / top line in SRT; defaults to `true`).
    pub target_first: Option<bool>,
    /// Retain target subtitles without a native match (defaults to `true`).
    pub include_unmatched: Option<bool>,
}

/// Merge two or more Vesta `.apkg` exports into a single `.apkg` package.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MergeApkgRequest {
    /// Paths to 2–64 `.apkg` packages (without review history) inside read or output roots.
    pub inputs: Vec<String>,
}
