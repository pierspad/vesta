use crate::paths::Paths;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use vesta_flashcards::{AudioFormat, FlashcardConfig, SnapshotFormat};

const MAX_TIME_SHIFT_MS: i64 = 86_400_000;
const MAX_MEDIA_PAD_MS: i64 = 60_000;

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

/// Shared subtitle filtering, time-shift, span, and sentence-combining options.
#[derive(Clone, Debug, Default, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct SubtitleFilterOptions {
    /// Keep only lines with at least this many characters.
    pub min_chars: Option<usize>,
    /// Keep only lines with at most this many characters.
    pub max_chars: Option<usize>,
    /// Keep only lines lasting at least this many milliseconds.
    pub min_duration_ms: Option<i64>,
    /// Keep only lines lasting at most this many milliseconds.
    pub max_duration_ms: Option<i64>,
    /// Comma-separated words; keep only lines containing at least one.
    pub include_words: Option<String>,
    /// Comma-separated words; drop lines containing any.
    pub exclude_words: Option<String>,
    /// Drop duplicate target-language lines.
    pub remove_duplicates: Option<bool>,
    /// Keep only lines containing CJK characters.
    pub only_cjk: Option<bool>,
    /// Drop lines without a matched native subtitle.
    pub remove_no_match: Option<bool>,
    /// Merge consecutive lines ending in `continuation_chars`.
    pub combine_sentences: Option<bool>,
    /// Characters indicating an unfinished sentence when `combine_sentences` is true.
    pub continuation_chars: Option<String>,
    /// Constant shift applied to target-subtitle timestamps (milliseconds).
    pub time_shift_target_ms: Option<i64>,
    /// Constant shift applied to native-subtitle timestamps (milliseconds).
    pub time_shift_native_ms: Option<i64>,
    /// Keep lines ending at or after this timestamp (milliseconds).
    pub span_start_ms: Option<i64>,
    /// Keep lines starting at or before this timestamp (milliseconds).
    pub span_end_ms: Option<i64>,
}

impl SubtitleFilterOptions {
    /// Validate and apply subtitle filtering, shift, span, and combining options to `config`.
    pub fn apply(&self, config: &mut FlashcardConfig) -> Result<(), String> {
        if let (Some(min), Some(max)) = (self.min_chars, self.max_chars)
            && min > max
        {
            return Err("min_chars cannot exceed max_chars".into());
        }
        if let Some(min) = self.min_duration_ms
            && min < 0
        {
            return Err("min_duration_ms must be non-negative".into());
        }
        if let Some(max) = self.max_duration_ms
            && max < 0
        {
            return Err("max_duration_ms must be non-negative".into());
        }
        if let (Some(min), Some(max)) = (self.min_duration_ms, self.max_duration_ms)
            && min > max
        {
            return Err("min_duration_ms cannot exceed max_duration_ms".into());
        }
        if let Some(start) = self.span_start_ms
            && start < 0
        {
            return Err("span_start_ms must be non-negative".into());
        }
        if let Some(end) = self.span_end_ms
            && end < 0
        {
            return Err("span_end_ms must be non-negative".into());
        }
        if let (Some(start), Some(end)) = (self.span_start_ms, self.span_end_ms)
            && start > end
        {
            return Err("span_start_ms cannot exceed span_end_ms".into());
        }
        validate_time_shift(self.time_shift_target_ms, "time_shift_target_ms")?;
        validate_time_shift(self.time_shift_native_ms, "time_shift_native_ms")?;

        if self.min_chars.is_some() {
            config.filters.min_chars = self.min_chars;
        }
        if self.max_chars.is_some() {
            config.filters.max_chars = self.max_chars;
        }
        if self.min_duration_ms.is_some() {
            config.filters.min_duration_ms = self.min_duration_ms;
        }
        if self.max_duration_ms.is_some() {
            config.filters.max_duration_ms = self.max_duration_ms;
        }
        if let Some(words) = &self.include_words {
            config.filters.include_words = Some(words.clone());
        }
        if let Some(words) = &self.exclude_words {
            config.filters.exclude_words = Some(words.clone());
        }
        if let Some(remove_duplicates) = self.remove_duplicates {
            config.filters.exclude_duplicates_subs1 = remove_duplicates;
        }
        if let Some(only_cjk) = self.only_cjk {
            config.filters.only_cjk = only_cjk;
        }
        if let Some(remove_no_match) = self.remove_no_match {
            config.filters.remove_no_match = remove_no_match;
        }
        if let Some(combine_sentences) = self.combine_sentences {
            config.combine_sentences = combine_sentences;
        }
        if let Some(continuation_chars) = &self.continuation_chars {
            config.continuation_chars = continuation_chars.clone();
        }
        if let Some(shift) = self.time_shift_target_ms {
            config.time_shift_target_ms = shift;
        }
        if let Some(shift) = self.time_shift_native_ms {
            config.time_shift_native_ms = shift;
        }
        if self.span_start_ms.is_some() {
            config.span_start_ms = self.span_start_ms;
        }
        if self.span_end_ms.is_some() {
            config.span_end_ms = self.span_end_ms;
        }
        Ok(())
    }
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
    pub min_chars: Option<usize>,
    pub max_chars: Option<usize>,
    pub min_duration_ms: Option<i64>,
    pub max_duration_ms: Option<i64>,
    pub include_words: Option<String>,
    pub exclude_words: Option<String>,
    pub remove_duplicates: Option<bool>,
    pub only_cjk: Option<bool>,
    pub remove_no_match: Option<bool>,
    pub combine_sentences: Option<bool>,
    pub continuation_chars: Option<String>,
    pub time_shift_target_ms: Option<i64>,
    pub time_shift_native_ms: Option<i64>,
    pub span_start_ms: Option<i64>,
    pub span_end_ms: Option<i64>,
}
fn preview_limit() -> usize {
    20
}

#[derive(Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Apkg,
    Tsv,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum McpAudioFormat {
    Mp3,
    Opus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum McpSnapshotFormat {
    Webp,
    Jpeg,
    Avif,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum McpVideoCodec {
    H264,
    Mpeg4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum McpVideoHwAccel {
    Auto,
    Off,
}

#[derive(Default)]
struct FontAndDictionaryOptions<'a> {
    target_language: Option<&'a str>,
    dictionary_lookup: Option<bool>,
    auto_card_font: Option<bool>,
    embed_card_font: Option<bool>,
}

impl FontAndDictionaryOptions<'_> {
    fn apply(&self, config: &mut FlashcardConfig) -> Result<(), String> {
        if let Some(raw_lang) = self.target_language {
            config.target_language = Some(validate_target_language(raw_lang)?);
        }
        config.dictionary_lookup = self.dictionary_lookup.unwrap_or(true);
        // Never enable font CSS or cached font embedding unless explicitly requested.
        config.auto_card_font = self.auto_card_font.unwrap_or(false);
        config.embed_card_font = self.embed_card_font.unwrap_or(false);
        Ok(())
    }
}

#[derive(Default)]
struct MediaOptions {
    audio_format: Option<McpAudioFormat>,
    audio_bitrate: Option<u32>,
    audio_track: Option<usize>,
    normalize_audio: Option<bool>,
    pad_audio_start_ms: Option<i64>,
    pad_audio_end_ms: Option<i64>,
    audio_gain: Option<i32>,
    snapshot_format: Option<McpSnapshotFormat>,
    snapshot_width: Option<u32>,
    snapshot_height: Option<u32>,
    snapshot_quality: Option<u8>,
    crop_bottom: Option<u32>,
    video_codec: Option<McpVideoCodec>,
    video_bitrate: Option<u32>,
    video_hw_accel: Option<McpVideoHwAccel>,
    pad_video_start_ms: Option<i64>,
    pad_video_end_ms: Option<i64>,
}

impl MediaOptions {
    fn apply(&self, config: &mut FlashcardConfig) -> Result<(), String> {
        if let Some(bitrate) = self.audio_bitrate {
            if !(16..=512).contains(&bitrate) {
                return Err("audio_bitrate must be from 16 to 512 kbps".into());
            }
            config.audio_bitrate = bitrate;
        }
        if let Some(track) = self.audio_track {
            validate_audio_track(track)?;
            config.audio_track_index = Some(track);
        }
        if let Some(normalize) = self.normalize_audio {
            config.normalize_audio = normalize;
        }
        if let Some(pad) = self.pad_audio_start_ms {
            validate_media_pad(pad, "pad_audio_start_ms")?;
            config.audio_pad_start_ms = pad;
        }
        if let Some(pad) = self.pad_audio_end_ms {
            validate_media_pad(pad, "pad_audio_end_ms")?;
            config.audio_pad_end_ms = pad;
        }
        if let Some(gain) = self.audio_gain {
            if !(-30..=30).contains(&gain) {
                return Err("audio_gain must be from -30 to 30 dB".into());
            }
            config.audio_gain_db = gain;
            config.audio_boost = gain != 0;
        }
        if let Some(format) = self.audio_format {
            config.audio_format = match format {
                McpAudioFormat::Mp3 => AudioFormat::Mp3,
                McpAudioFormat::Opus => AudioFormat::Opus,
            };
        }
        if let Some(format) = self.snapshot_format {
            config.snapshot_format = match format {
                McpSnapshotFormat::Webp => SnapshotFormat::Webp,
                McpSnapshotFormat::Jpeg => SnapshotFormat::Jpeg,
                McpSnapshotFormat::Avif => SnapshotFormat::Avif,
            };
        }
        if let Some(width) = self.snapshot_width {
            if !(16..=3840).contains(&width) {
                return Err("snapshot_width must be from 16 to 3840 pixels".into());
            }
            config.snapshot_width = width;
        }
        if let Some(height) = self.snapshot_height {
            if !(16..=2160).contains(&height) {
                return Err("snapshot_height must be from 16 to 2160 pixels".into());
            }
            config.snapshot_height = height;
        }
        if let Some(quality) = self.snapshot_quality {
            if quality > 100 {
                return Err("snapshot_quality must be from 0 to 100".into());
            }
            config.snapshot_quality = quality;
        }
        if let Some(crop) = self.crop_bottom {
            validate_crop_bottom(crop)?;
            config.crop_bottom = crop;
        }
        if let Some(codec) = self.video_codec {
            config.video_codec = match codec {
                McpVideoCodec::H264 => "h264".into(),
                McpVideoCodec::Mpeg4 => "mpeg4".into(),
            };
        }
        if let Some(bitrate) = self.video_bitrate {
            if !(100..=50_000).contains(&bitrate) {
                return Err("video_bitrate must be from 100 to 50000 kbps".into());
            }
            config.video_bitrate = bitrate;
        }
        if let Some(hw_accel) = self.video_hw_accel {
            config.video_hw_accel = match hw_accel {
                McpVideoHwAccel::Auto => "auto".into(),
                McpVideoHwAccel::Off => "off".into(),
            };
        }
        if let Some(pad) = self.pad_video_start_ms {
            validate_media_pad(pad, "pad_video_start_ms")?;
            config.video_pad_start_ms = pad;
        }
        if let Some(pad) = self.pad_video_end_ms {
            validate_media_pad(pad, "pad_video_end_ms")?;
            config.video_pad_end_ms = pad;
        }
        Ok(())
    }
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
    /// Optional BCP-47 / ISO-639 target language tag for dictionary lookup and card fonts.
    pub target_language: Option<String>,
    /// Include dictionary lookup script in APKG card templates (defaults to `true`).
    pub dictionary_lookup: Option<bool>,
    /// Inject language-matched CSS font stack into APKG card templates (defaults to `false`).
    pub auto_card_font: Option<bool>,
    /// Embed an already-cached local font file into APKG exports (never downloads; defaults to `false`).
    pub embed_card_font: Option<bool>,
    pub audio_format: Option<McpAudioFormat>,
    pub audio_bitrate: Option<u32>,
    pub audio_track: Option<usize>,
    pub normalize_audio: Option<bool>,
    pub pad_audio_start_ms: Option<i64>,
    pub pad_audio_end_ms: Option<i64>,
    pub audio_gain: Option<i32>,
    pub snapshot_format: Option<McpSnapshotFormat>,
    pub snapshot_width: Option<u32>,
    pub snapshot_height: Option<u32>,
    pub snapshot_quality: Option<u8>,
    pub crop_bottom: Option<u32>,
    pub video_codec: Option<McpVideoCodec>,
    pub video_bitrate: Option<u32>,
    pub video_hw_accel: Option<McpVideoHwAccel>,
    pub pad_video_start_ms: Option<i64>,
    pub pad_video_end_ms: Option<i64>,
    pub min_chars: Option<usize>,
    pub max_chars: Option<usize>,
    pub min_duration_ms: Option<i64>,
    pub max_duration_ms: Option<i64>,
    pub include_words: Option<String>,
    pub exclude_words: Option<String>,
    pub remove_duplicates: Option<bool>,
    pub only_cjk: Option<bool>,
    pub remove_no_match: Option<bool>,
    pub combine_sentences: Option<bool>,
    pub continuation_chars: Option<String>,
    pub time_shift_target_ms: Option<i64>,
    pub time_shift_native_ms: Option<i64>,
    pub span_start_ms: Option<i64>,
    pub span_end_ms: Option<i64>,
}

/// Single episode specification for `batch_generate_flashcards`.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BatchEpisodeRequest {
    pub target: String,
    pub native: Option<String>,
    pub video: Option<String>,
    pub audio: Option<String>,
    /// Optional episode-specific deck title override.
    pub deck: Option<String>,
    /// Optional episode number (`0..=9999`); defaults to 1-based position in `episodes`.
    pub episode: Option<u32>,
    /// Optional per-episode audio track index override (`0..=64`).
    pub audio_track: Option<usize>,
    /// Optional per-episode bottom crop override (`0..=2160` pixels).
    pub crop_bottom: Option<u32>,
    /// Optional per-episode target subtitle shift override (milliseconds).
    pub shift_target_ms: Option<i64>,
    /// Optional per-episode native subtitle shift override (milliseconds).
    pub shift_native_ms: Option<i64>,
}

/// Generate flashcards for 1 to 64 episodes in a single background job, optionally merging `.apkg` outputs.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BatchGenerateRequest {
    /// Ordered list of 1 to 64 episodes to generate.
    pub episodes: Vec<BatchEpisodeRequest>,
    /// Shared default deck title (`1..=256` characters) when an episode omits `deck`.
    pub deck: Option<String>,
    pub format: Format,
    /// When `true` and `format` is `apkg`, merge generated `.apkg` files into `merged.apkg`.
    pub merge_apkg: Option<bool>,
    pub note_type: Option<String>,
    #[serde(default)]
    pub include_audio: bool,
    #[serde(default)]
    pub include_snapshots: bool,
    #[serde(default)]
    pub include_video: bool,
    /// Maximum concurrent media workers per episode, from 1 to 64; capped to available cores.
    pub workers: Option<usize>,
    pub target_language: Option<String>,
    pub dictionary_lookup: Option<bool>,
    pub auto_card_font: Option<bool>,
    pub embed_card_font: Option<bool>,
    pub audio_format: Option<McpAudioFormat>,
    pub audio_bitrate: Option<u32>,
    pub audio_track: Option<usize>,
    pub normalize_audio: Option<bool>,
    pub pad_audio_start_ms: Option<i64>,
    pub pad_audio_end_ms: Option<i64>,
    pub audio_gain: Option<i32>,
    pub snapshot_format: Option<McpSnapshotFormat>,
    pub snapshot_width: Option<u32>,
    pub snapshot_height: Option<u32>,
    pub snapshot_quality: Option<u8>,
    pub crop_bottom: Option<u32>,
    pub video_codec: Option<McpVideoCodec>,
    pub video_bitrate: Option<u32>,
    pub video_hw_accel: Option<McpVideoHwAccel>,
    pub pad_video_start_ms: Option<i64>,
    pub pad_video_end_ms: Option<i64>,
    pub min_chars: Option<usize>,
    pub max_chars: Option<usize>,
    pub min_duration_ms: Option<i64>,
    pub max_duration_ms: Option<i64>,
    pub include_words: Option<String>,
    pub exclude_words: Option<String>,
    pub remove_duplicates: Option<bool>,
    pub only_cjk: Option<bool>,
    pub remove_no_match: Option<bool>,
    pub combine_sentences: Option<bool>,
    pub continuation_chars: Option<String>,
    pub time_shift_target_ms: Option<i64>,
    pub time_shift_native_ms: Option<i64>,
    pub span_start_ms: Option<i64>,
    pub span_end_ms: Option<i64>,
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

/// Normalized video region of interest (`0.0..=1.0`) for hardcoded subtitle OCR and frame previews.
#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OcrRoiRequest {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub type RoiRequest = OcrRoiRequest;

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

    /// Validates normalized coordinates and clamps `width`/`height` to frame bounds `[0.0, 1.0]`.
    pub fn into_clamped_video_roi(self) -> Result<vesta_ocr::VideoRoi, String> {
        let roi = vesta_ocr::VideoRoi {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        };
        roi.clamped().map_err(|e| e.to_string())
    }
}

/// Extract a PNG preview frame from a video container inside a configured read root, optionally computing pixel coordinates for a normalized ROI.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewVideoFrameRequest {
    /// Absolute path to a video file within a configured read root.
    pub input: String,
    /// Optional timestamp in milliseconds (defaults to ~10% into the video, capped at 30s).
    pub time_ms: Option<u64>,
    /// Optional normalized ROI (`0.0..=1.0`) to resolve into pixel coordinates (`roi_px`).
    pub roi: Option<RoiRequest>,
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

/// Inspect and search cards inside a `.tsv` or `.apkg` deck without modifying files.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectDeckRequest {
    /// Absolute path to a `.tsv` or `.apkg` deck inside a configured read or output root.
    pub input: String,
    /// Optional case-insensitive substring filter applied across expression, meaning, and notes.
    pub query: Option<String>,
    /// Zero-based offset into the matched cards. Defaults to zero.
    #[serde(default)]
    pub offset: usize,
    /// Maximum cards returned, from 1 to 100. Defaults to 20.
    #[serde(default = "preview_limit")]
    pub limit: usize,
}

/// Single card notes update for `edit_deck_cards`.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeckCardUpdate {
    /// Card/note ID returned by `inspect_deck`.
    pub id: String,
    /// Updated notes field content.
    pub notes: String,
}

/// Update notes for selected cards in a `.tsv` or `.apkg` deck and write a new deck in `output_root`.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditDeckCardsRequest {
    /// Absolute path to a `.tsv` or `.apkg` deck inside a configured read or output root.
    pub input: String,
    /// 1 to 200 card updates to apply.
    pub updates: Vec<DeckCardUpdate>,
    /// Output deck format (`apkg` or `tsv`). Defaults to the input format (`tsv -> apkg` is not supported).
    pub output_format: Option<Format>,
}

/// Loopback AnkiConnect operation.
#[derive(Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnkiConnectAction {
    Ping,
    DeckNames,
    ImportPackage,
}

/// Interact with a local loopback AnkiConnect add-on instance (`ping`, `deck_names`, or `import_package`).
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnkiConnectRequest {
    /// AnkiConnect action to invoke (`ping`, `deck_names`, or `import_package`).
    pub action: AnkiConnectAction,
    /// Optional loopback HTTP/HTTPS AnkiConnect URL (`127.0.0.1`, `localhost`, or `[::1]`). Defaults to `http://127.0.0.1:8765`.
    pub url: Option<String>,
    /// Absolute path to an `.apkg` file inside read or output roots (required for `import_package`).
    pub apkg_path: Option<String>,
}

/// Local model management action.
#[derive(Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ModelAction {
    List,
    Download,
    Remove,
}

/// Local model category.
#[derive(Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ModelKind {
    Whisper,
    Vad,
    Ocr,
}

/// List, download, or remove cached local Whisper, VAD, and OCR models.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ManageLocalModelsRequest {
    /// Operation to perform (`list`, `download`, or `remove`).
    pub action: ModelAction,
    /// Model kind (`whisper`, `vad`, or `ocr`). Optional for `list`; required for `download` and `remove`.
    pub kind: Option<ModelKind>,
    /// Catalog model ID (required for `download` and `remove`).
    pub model_id: Option<String>,
}

macro_rules! impl_filter_options {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl $ty {
                pub fn filter_options(&self) -> SubtitleFilterOptions {
                    SubtitleFilterOptions {
                        min_chars: self.min_chars,
                        max_chars: self.max_chars,
                        min_duration_ms: self.min_duration_ms,
                        max_duration_ms: self.max_duration_ms,
                        include_words: self.include_words.clone(),
                        exclude_words: self.exclude_words.clone(),
                        remove_duplicates: self.remove_duplicates,
                        only_cjk: self.only_cjk,
                        remove_no_match: self.remove_no_match,
                        combine_sentences: self.combine_sentences,
                        continuation_chars: self.continuation_chars.clone(),
                        time_shift_target_ms: self.time_shift_target_ms,
                        time_shift_native_ms: self.time_shift_native_ms,
                        span_start_ms: self.span_start_ms,
                        span_end_ms: self.span_end_ms,
                    }
                }
            }
        )+
    };
}

impl_filter_options!(
    PreviewRequest,
    GenerateRequest,
    ExportDuelingRequest,
    BatchGenerateRequest
);

macro_rules! impl_media_and_font_options {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl $ty {
                fn font_options(&self) -> FontAndDictionaryOptions<'_> {
                    FontAndDictionaryOptions {
                        target_language: self.target_language.as_deref(),
                        dictionary_lookup: self.dictionary_lookup,
                        auto_card_font: self.auto_card_font,
                        embed_card_font: self.embed_card_font,
                    }
                }

                fn media_options(&self) -> MediaOptions {
                    MediaOptions {
                        audio_format: self.audio_format,
                        audio_bitrate: self.audio_bitrate,
                        audio_track: self.audio_track,
                        normalize_audio: self.normalize_audio,
                        pad_audio_start_ms: self.pad_audio_start_ms,
                        pad_audio_end_ms: self.pad_audio_end_ms,
                        audio_gain: self.audio_gain,
                        snapshot_format: self.snapshot_format,
                        snapshot_width: self.snapshot_width,
                        snapshot_height: self.snapshot_height,
                        snapshot_quality: self.snapshot_quality,
                        crop_bottom: self.crop_bottom,
                        video_codec: self.video_codec,
                        video_bitrate: self.video_bitrate,
                        video_hw_accel: self.video_hw_accel,
                        pad_video_start_ms: self.pad_video_start_ms,
                        pad_video_end_ms: self.pad_video_end_ms,
                    }
                }
            }
        )+
    };
}

impl_media_and_font_options!(GenerateRequest, BatchGenerateRequest);

fn validate_time_shift(shift: Option<i64>, label: &str) -> Result<(), String> {
    if let Some(value) = shift
        && !(-MAX_TIME_SHIFT_MS..=MAX_TIME_SHIFT_MS).contains(&value)
    {
        return Err(format!(
            "{label} must be within -{MAX_TIME_SHIFT_MS}..={MAX_TIME_SHIFT_MS} ms"
        ));
    }
    Ok(())
}

fn validate_media_pad(pad: i64, label: &str) -> Result<(), String> {
    if !(-MAX_MEDIA_PAD_MS..=MAX_MEDIA_PAD_MS).contains(&pad) {
        return Err(format!(
            "{label} must be within -{MAX_MEDIA_PAD_MS}..={MAX_MEDIA_PAD_MS} ms"
        ));
    }
    Ok(())
}

fn validate_audio_track(track: usize) -> Result<(), String> {
    if track > 64 {
        return Err("audio_track must be from 0 to 64".into());
    }
    Ok(())
}

fn validate_crop_bottom(crop: u32) -> Result<(), String> {
    if crop > 2160 {
        return Err("crop_bottom must be from 0 to 2160 pixels".into());
    }
    Ok(())
}

fn validate_deck_title(deck: &str) -> Result<(), String> {
    if deck.trim().is_empty() || deck.chars().count() > 256 {
        return Err("Deck title must contain 1 to 256 characters".into());
    }
    Ok(())
}

fn validate_workers(workers: Option<usize>) -> Result<(), String> {
    if workers.is_some_and(|n| !(1..=64).contains(&n)) {
        return Err("Workers must be from 1 to 64".into());
    }
    Ok(())
}

fn validate_note_type(note_type: Option<&str>) -> Result<(), String> {
    if note_type.is_some_and(|name| name.trim().is_empty() || name.chars().count() > 128) {
        return Err("Note type must contain 1 to 128 characters".into());
    }
    Ok(())
}

fn validate_target_language(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.len() > 35
        || !trimmed
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || !trimmed.bytes().any(|b| b.is_ascii_alphabetic())
    {
        return Err(
            "target_language must be a 1 to 35 character ASCII language tag (e.g. ja, zh-tw, en-US)"
                .into(),
        );
    }
    Ok(trimmed.to_string())
}

fn format_str(format: Format) -> String {
    match format {
        Format::Apkg => "apkg",
        Format::Tsv => "tsv",
    }
    .into()
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

#[allow(clippy::too_many_arguments)]
fn build_episode_config(
    paths: &Paths,
    target: &str,
    native: Option<&str>,
    video: Option<&str>,
    audio: Option<&str>,
    deck: &str,
    format: Format,
    note_type: Option<&str>,
    include_audio: bool,
    include_snapshots: bool,
    include_video: bool,
    workers: Option<usize>,
    filters: &SubtitleFilterOptions,
    fonts: &FontAndDictionaryOptions<'_>,
    media: &MediaOptions,
) -> Result<FlashcardConfig, String> {
    validate_deck_title(deck)?;
    let mut config = subtitle_config(paths, target, native)?;
    config.video_path = video.map(|p| paths.input(p.as_ref(), false)).transpose()?;
    config.audio_path = audio.map(|p| paths.input(p.as_ref(), false)).transpose()?;
    if include_audio && config.video_path.is_none() && config.audio_path.is_none() {
        return Err("Audio generation requires a video or audio input".into());
    }
    if (include_snapshots || include_video) && config.video_path.is_none() {
        return Err("Snapshots and video clips require a video input".into());
    }
    filters.apply(&mut config)?;
    fonts.apply(&mut config)?;
    media.apply(&mut config)?;
    config.deck_name = deck.to_owned();
    config.export_format = Some(format_str(format));
    config.note_type_name = note_type.map(str::to_owned);
    config.generate_audio = include_audio;
    config.generate_snapshots = include_snapshots;
    config.generate_video_clips = include_video;
    config.output_fields.include_audio = include_audio;
    config.output_fields.include_snapshot = include_snapshots;
    config.output_fields.include_video = include_video;
    config.cpu_cores = workers;
    Ok(config)
}

impl GenerateRequest {
    pub fn config(&self, paths: &Paths) -> Result<FlashcardConfig, String> {
        validate_workers(self.workers)?;
        validate_note_type(self.note_type.as_deref())?;
        let filters = self.filter_options();
        let fonts = self.font_options();
        let media = self.media_options();
        build_episode_config(
            paths,
            &self.target,
            self.native.as_deref(),
            self.video.as_deref(),
            self.audio.as_deref(),
            &self.deck,
            self.format,
            self.note_type.as_deref(),
            self.include_audio,
            self.include_snapshots,
            self.include_video,
            self.workers,
            &filters,
            &fonts,
            &media,
        )
    }
}

impl BatchGenerateRequest {
    /// Validate shared defaults and every episode, building an isolated `FlashcardConfig` per episode.
    pub fn build_configs(&self, paths: &Paths) -> Result<Vec<FlashcardConfig>, String> {
        if !(1..=64).contains(&self.episodes.len()) {
            return Err("episodes must contain 1 to 64 entries".into());
        }
        if let Some(shared_deck) = self.deck.as_deref() {
            validate_deck_title(shared_deck)?;
        }
        validate_workers(self.workers)?;
        validate_note_type(self.note_type.as_deref())?;

        let filters = self.filter_options();
        let fonts = self.font_options();
        let media = self.media_options();

        let mut configs = Vec::with_capacity(self.episodes.len());
        for (idx, ep) in self.episodes.iter().enumerate() {
            let ep_num = idx + 1;
            let deck = ep.deck.as_deref().or(self.deck.as_deref()).ok_or_else(|| {
                format!("Episode {ep_num} requires a deck title (set episode.deck or shared deck)")
            })?;
            if let Some(episode_number) = ep.episode
                && episode_number > 9999
            {
                return Err(format!(
                    "Episode {ep_num}: episode number must be from 0 to 9999"
                ));
            }
            if let Some(track) = ep.audio_track {
                validate_audio_track(track).map_err(|err| format!("Episode {ep_num}: {err}"))?;
            }
            if let Some(crop) = ep.crop_bottom {
                validate_crop_bottom(crop).map_err(|err| format!("Episode {ep_num}: {err}"))?;
            }
            validate_time_shift(ep.shift_target_ms, "shift_target_ms")
                .map_err(|err| format!("Episode {ep_num}: {err}"))?;
            validate_time_shift(ep.shift_native_ms, "shift_native_ms")
                .map_err(|err| format!("Episode {ep_num}: {err}"))?;

            let mut config = build_episode_config(
                paths,
                &ep.target,
                ep.native.as_deref(),
                ep.video.as_deref(),
                ep.audio.as_deref(),
                deck,
                self.format,
                self.note_type.as_deref(),
                self.include_audio,
                self.include_snapshots,
                self.include_video,
                self.workers,
                &filters,
                &fonts,
                &media,
            )
            .map_err(|err| format!("Episode {ep_num}: {err}"))?;

            config.episode_number = ep.episode.unwrap_or(ep_num as u32);
            if let Some(track) = ep.audio_track {
                config.audio_track_index = Some(track);
            }
            if let Some(crop) = ep.crop_bottom {
                config.crop_bottom = crop;
            }
            if let Some(shift) = ep.shift_target_ms {
                config.time_shift_target_ms = shift;
            }
            if let Some(shift) = ep.shift_native_ms {
                config.time_shift_native_ms = shift;
            }
            configs.push(config);
        }
        Ok(configs)
    }

    pub fn should_merge_apkg(&self) -> bool {
        self.merge_apkg == Some(true) && matches!(self.format, Format::Apkg)
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
    pub min_chars: Option<usize>,
    pub max_chars: Option<usize>,
    pub min_duration_ms: Option<i64>,
    pub max_duration_ms: Option<i64>,
    pub include_words: Option<String>,
    pub exclude_words: Option<String>,
    pub remove_duplicates: Option<bool>,
    pub only_cjk: Option<bool>,
    pub remove_no_match: Option<bool>,
    pub combine_sentences: Option<bool>,
    pub continuation_chars: Option<String>,
    pub time_shift_target_ms: Option<i64>,
    pub time_shift_native_ms: Option<i64>,
    pub span_start_ms: Option<i64>,
    pub span_end_ms: Option<i64>,
}

/// Merge two or more Vesta `.apkg` exports into a single `.apkg` package.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MergeApkgRequest {
    /// Paths to 2–64 `.apkg` packages (without review history) inside read or output roots.
    pub inputs: Vec<String>,
}
