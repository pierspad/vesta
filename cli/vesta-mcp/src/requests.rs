use crate::paths::Paths;
use schemars::JsonSchema;
use serde::Deserialize;
use srt_flashcards::FlashcardConfig;

/// Translate a subtitle file with a provider profile configured by the server.
/// Provider secrets are never accepted in tool arguments.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TranslateRequest {
    /// Absolute path to an SRT, ASS, SSA, VTT or LRC file within a configured read root.
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
