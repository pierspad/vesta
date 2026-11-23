//! # vesta-flashcards CLI
//!
//! Headless front-end for the [`vesta_flashcards`] engine. It turns a subtitle
//! file (optionally a target + native pair) plus a media file into an Anki deck
//! (`tsv` + media folder, or a self-contained `.apkg`) — the exact same engine
//! the Vesta desktop app uses, with no GUI dependency.
//!
//! This binary is the "shell": it parses CLI arguments and delegates all logic
//! to the library, mirroring the `vesta-extract` / `vesta-translate` CLIs.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicI64, Ordering};

use anyhow::{Context, Result, ensure};
use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Deserializer, Serialize};
use tokio_util::sync::CancellationToken;
use vesta_flashcards::{
    AudioFormat, ContextConfig, DuelingExportOptions, DuelingFormat, FieldNamesConfig,
    FlashcardConfig, FlashcardProgressEvent, FlashcardResult, MediaTools, OutputFields,
    PreviewLine, SnapshotFormat, SubtitleFilters,
};

#[derive(Parser)]
#[command(
    name = "vesta-flashcards",
    version,
    about = "Turn subtitles + media into Anki flashcards (headless subs2srs-style engine)",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate flashcards (TSV + media folder, or a .apkg package).
    #[command(
        after_help = "Examples:\n  vesta-flashcards generate -t target.srt -o deck --video movie.mkv\n  vesta-flashcards generate -t target.srt -o deck --no-audio --no-snapshots --no-video\n\nA media source is needed only for enabled media outputs. Outputs without a source are skipped: --audio/--video supply audio, while snapshots/clips need --video. Use --format apkg for a packaged deck."
    )]
    Generate(GenerateArgs),
    /// Print a summary of a subtitle file (count, format, actors, duration).
    #[command(after_help = "Example:\n  vesta-flashcards info subtitles.srt")]
    Info {
        /// Subtitle file to inspect.
        file: String,
    },
    /// Run the parse/match/filter pipeline and report line counts or JSON rows (no media).
    #[command(
        after_help = "Examples:\n  vesta-flashcards preview -t target.srt --native native.srt --only-cjk\n  vesta-flashcards preview -t target.srt --json --offset 0 --limit 25\n\nPreview reads subtitle files only and never creates the --output directory (--output is optional for preview). Its CLI options are shared with generate; media/export options are ignored."
    )]
    Preview(PreviewArgs),
    /// Export bilingual "dueling" subtitles (.ass or .srt) from matched subtitle tracks.
    #[command(
        after_help = "Examples:\n  vesta-flashcards dueling -t target.srt --native native.srt -o episode.dueling.ass\n  vesta-flashcards dueling -t target.srt --native native.srt --dueling-format srt -o episode.dueling.srt"
    )]
    Dueling(DuelingArgs),
    /// Merge two or more fresh Vesta .apkg packages into a unified .apkg file.
    #[command(
        alias = "merge-apkg",
        after_help = "Example:\n  vesta-flashcards merge ep1.apkg ep2.apkg --output series.apkg\n\nInputs must be fresh Vesta .apkg exports without review history."
    )]
    Merge(MergeArgs),
    /// Generate multiple episodes from a JSON manifest (optionally merging .apkg outputs).
    #[command(
        after_help = "Example:\n  vesta-flashcards batch --manifest series.json\n\nManifest JSON requires non-empty `output_dir` and at least one entry in `episodes`. Optional `merge_output` combines generated .apkg decks into a single package when `defaults.format` is `apkg`."
    )]
    Batch(BatchArgs),
}

#[derive(Args, Clone)]
struct PreviewArgs {
    #[command(flatten)]
    common: GenerateArgs,
    /// Emit a JSON object with summary counts and paginated preview rows.
    #[arg(long)]
    json: bool,
    /// Zero-based row offset when `--json` is enabled.
    #[arg(long, default_value_t = 0)]
    offset: usize,
    /// Maximum number of rows to emit when `--json` is enabled.
    #[arg(long, default_value_t = 50)]
    limit: usize,
}

#[derive(Args, Clone)]
struct BatchArgs {
    /// Path to the JSON batch manifest.
    #[arg(long)]
    manifest: PathBuf,
    /// Suppress progress output on stderr.
    #[arg(short, long)]
    quiet: bool,
    /// ffmpeg executable (path or command on PATH).
    #[arg(long, default_value = "ffmpeg")]
    ffmpeg: String,
    /// ffprobe executable (path or command on PATH).
    #[arg(long, default_value = "ffprobe")]
    ffprobe: String,
}

#[derive(Args, Clone)]
struct MergeArgs {
    /// Two or more Vesta .apkg packages to merge.
    #[arg(required = true, num_args = 2..)]
    inputs: Vec<std::path::PathBuf>,
    /// Output .apkg file path.
    #[arg(short, long)]
    output: std::path::PathBuf,
}

#[derive(Args, Clone)]
struct DuelingArgs {
    /// Target-language subtitles (the language you are learning). Required.
    #[arg(short, long)]
    target: String,
    /// Native-language subtitles matched by time overlap.
    #[arg(short, long)]
    native: Option<String>,
    /// Output file path (defaults to `<target>.dueling.<ext>`).
    #[arg(short, long)]
    output: Option<String>,
    /// Dueling subtitle format (`ass` or `srt`).
    #[arg(long = "dueling-format", default_value = "ass", value_parser = ["ass", "srt"])]
    dueling_format: String,
    /// Deck or title used in ASS metadata and default filenames.
    #[arg(short, long, default_value = "Vesta")]
    deck: String,
    /// Shift target-subtitle timestamps by N milliseconds.
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    shift_target: i64,
    /// Shift native-subtitle timestamps by N milliseconds.
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    shift_native: i64,
    /// Restrict to lines overlapping [span-start, span-end] (milliseconds).
    #[arg(long)]
    span_start: Option<i64>,
    /// End of the optional time span in milliseconds.
    #[arg(long)]
    span_end: Option<i64>,
    /// Merge consecutive lines into full sentences.
    #[arg(long)]
    combine_sentences: bool,
    /// Characters that mark an unfinished sentence (for --combine-sentences).
    #[arg(long, default_value = "")]
    continuation_chars: String,
    /// Keep only lines whose script is CJK.
    #[arg(long)]
    only_cjk: bool,
    /// Drop lines that have no native-subtitle match.
    #[arg(long)]
    remove_no_match: bool,
    /// Place native subtitles first (bottom in ASS / top line in SRT).
    #[arg(long)]
    native_first: bool,
}

#[derive(Args, Clone)]
struct GenerateArgs {
    // ── Inputs / outputs ─────────────────────────────────────────────────
    /// Target-language subtitles (the language you are learning). Required.
    #[arg(short, long)]
    target: String,
    /// Native-language subtitles. When given, lines are matched by time overlap.
    #[arg(short, long)]
    native: Option<String>,
    /// Video file (source for snapshots and video clips, and audio by default).
    #[arg(short, long)]
    video: Option<String>,
    /// Separate audio source (defaults to the video file).
    #[arg(long)]
    audio: Option<String>,
    /// Output directory.
    #[arg(short, long)]
    output: Option<String>,
    /// Export format.
    #[arg(short, long, default_value = "tsv", value_parser = ["tsv", "apkg"])]
    format: String,
    /// Deck name (used for filenames and the Anki deck).
    #[arg(short, long, default_value = "Vesta")]
    deck: String,
    /// Anki note type name (only applied to .apkg exports).
    #[arg(long)]
    note_type: Option<String>,
    /// ISO language code for card font selection and dictionary lookup hints.
    #[arg(long)]
    target_language: Option<String>,
    /// Disable word-lookup links in .apkg card templates.
    #[arg(long)]
    no_dictionary_lookup: bool,
    /// Disable automatic language-matched card font styling in .apkg exports.
    #[arg(long)]
    no_auto_card_font: bool,
    /// Do not bundle downloaded card fonts inside .apkg archives.
    #[arg(long)]
    no_embed_card_font: bool,
    /// Episode number (used in the media filename pattern).
    #[arg(long, default_value_t = 1)]
    episode: u32,

    // ── Performance ──────────────────────────────────────────────────────
    /// Number of parallel ffmpeg workers (clamped to [1, available cores]; defaults to ~3/4 of logical cores).
    #[arg(short = 'j', long)]
    jobs: Option<usize>,

    // ── Media toggles ────────────────────────────────────────────────────
    /// Do not extract audio clips.
    #[arg(long)]
    no_audio: bool,
    /// Do not extract snapshots.
    #[arg(long)]
    no_snapshots: bool,
    /// Do not extract video clips.
    #[arg(long)]
    no_video: bool,
    /// Disable automatic video pre-transcoding and nearby snapshot batching.
    #[arg(long)]
    no_optimize: bool,
    /// Loudness-normalize audio clips (EBU R128).
    #[arg(long)]
    normalize_audio: bool,
    /// Additional padding at the start of audio clips (milliseconds).
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    pad_audio_start: i64,
    /// Additional padding at the end of audio clips (milliseconds).
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    pad_audio_end: i64,
    /// Audio gain in dB.
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    audio_gain: i32,
    /// Audio bitrate in kbps.
    #[arg(long, default_value_t = 128)]
    audio_bitrate: u32,
    /// Audio codec (`mp3` or `opus`).
    #[arg(long, default_value = "mp3", value_parser = ["mp3", "opus"])]
    audio_format: String,
    /// Explicit audio track index (ffmpeg 0:a:N order).
    #[arg(long)]
    audio_track: Option<usize>,
    /// Snapshot / video width in pixels.
    #[arg(long, default_value_t = 256)]
    snapshot_width: u32,
    /// Snapshot / video height in pixels.
    #[arg(long, default_value_t = 144)]
    snapshot_height: u32,
    /// Snapshot image codec (`webp`, `jpeg`, or `avif`).
    #[arg(long, default_value = "webp", value_parser = ["webp", "jpeg", "avif"])]
    snapshot_format: String,
    /// Snapshot quality, 0-100 (100 = best). Mapped onto each codec's own scale.
    #[arg(long, default_value_t = 80, value_parser = parse_snapshot_quality)]
    snapshot_quality: u8,
    /// Crop N pixels off the bottom (e.g. to remove burned-in subs).
    #[arg(long, default_value_t = 0)]
    crop_bottom: u32,
    /// Video codec.
    #[arg(long, default_value = "h264", value_parser = ["h264", "mpeg4"])]
    video_codec: String,
    /// libx264 preset.
    #[arg(long, default_value = "ultrafast")]
    h264_preset: String,
    /// Video bitrate (kbps).
    #[arg(long, default_value_t = 1000)]
    video_bitrate: u32,
    /// Video audio-track bitrate (kbps).
    #[arg(long, default_value_t = 128)]
    video_audio_bitrate: u32,
    /// Hardware acceleration mode for H.264 video encoding (`auto` or `off`).
    #[arg(long, default_value = "auto", value_parser = ["auto", "off"])]
    video_hw_accel: String,
    /// Additional padding at the start of video clips (milliseconds).
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    pad_video_start: i64,
    /// Additional padding at the end of video clips (milliseconds).
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    pad_video_end: i64,

    // ── Filters ──────────────────────────────────────────────────────────
    /// Keep only lines with at least this many characters.
    #[arg(long)]
    min_chars: Option<usize>,
    /// Keep only lines with at most this many characters.
    #[arg(long)]
    max_chars: Option<usize>,
    /// Keep only lines lasting at least this many milliseconds.
    #[arg(long)]
    min_duration: Option<i64>,
    /// Keep only lines lasting at most this many milliseconds.
    #[arg(long)]
    max_duration: Option<i64>,
    /// Only keep lines containing one of these comma-separated words.
    #[arg(long)]
    include_words: Option<String>,
    /// Drop lines containing one of these comma-separated words.
    #[arg(long)]
    exclude_words: Option<String>,
    /// Drop duplicate target lines.
    #[arg(long)]
    exclude_duplicates: bool,
    /// Keep only lines spoken by these actors (comma-separated or repeatable).
    #[arg(long, value_delimiter = ',')]
    actor_filter: Vec<String>,
    /// Keep only lines whose script is CJK.
    #[arg(long)]
    only_cjk: bool,
    /// Drop lines that have no native-subtitle match.
    #[arg(long)]
    remove_no_match: bool,
    /// Restrict to lines overlapping [span-start, span-end] (milliseconds).
    #[arg(long)]
    span_start: Option<i64>,
    /// End of the optional time span in milliseconds.
    #[arg(long)]
    span_end: Option<i64>,
    /// Shift target-subtitle timestamps by N milliseconds (negative values advance them).
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    shift_target: i64,
    /// Shift native-subtitle timestamps by N milliseconds (negative values advance them).
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    shift_native: i64,

    // ── Context / sentence combining ─────────────────────────────────────
    /// Attach N leading context lines to each card.
    #[arg(long, default_value_t = 0)]
    context_leading: usize,
    /// Attach N trailing context lines to each card.
    #[arg(long, default_value_t = 0)]
    context_trailing: usize,
    /// Maximum gap (seconds) between a card and a context line.
    #[arg(long, default_value_t = 0.0)]
    max_gap: f64,
    /// Merge consecutive lines into full sentences.
    #[arg(long)]
    combine_sentences: bool,
    /// Characters that mark an unfinished sentence (for --combine-sentences).
    #[arg(long, default_value = "")]
    continuation_chars: String,

    // ── External tools ───────────────────────────────────────────────────
    /// ffmpeg executable (path or command on PATH).
    #[arg(long, default_value = "ffmpeg")]
    ffmpeg: String,
    /// ffprobe executable (path or command on PATH).
    #[arg(long, default_value = "ffprobe")]
    ffprobe: String,

    /// Suppress progress output on stderr.
    #[arg(short, long)]
    quiet: bool,
}

fn parse_snapshot_quality(value: &str) -> std::result::Result<u8, String> {
    let quality = value
        .parse::<u8>()
        .map_err(|_| "snapshot quality must be an integer from 0 to 100".to_string())?;
    if quality > 100 {
        return Err("snapshot quality must be from 0 to 100".to_string());
    }
    Ok(quality)
}

impl GenerateArgs {
    fn to_config(&self) -> FlashcardConfig {
        FlashcardConfig {
            target_subs_path: self.target.clone(),
            native_subs_path: self.native.clone(),
            video_path: self.video.clone(),
            audio_path: self.audio.clone(),
            output_dir: self.output.clone().unwrap_or_default(),
            span_start_ms: self.span_start,
            span_end_ms: self.span_end,
            time_shift_target_ms: self.shift_target,
            time_shift_native_ms: self.shift_native,
            filters: SubtitleFilters {
                include_words: self.include_words.clone(),
                exclude_words: self.exclude_words.clone(),
                exclude_duplicates_subs1: self.exclude_duplicates,
                min_chars: self.min_chars,
                max_chars: self.max_chars,
                min_duration_ms: self.min_duration,
                max_duration_ms: self.max_duration,
                actor_filter: if self.actor_filter.is_empty() {
                    None
                } else {
                    Some(self.actor_filter.join(","))
                },
                only_cjk: self.only_cjk,
                remove_no_match: self.remove_no_match,
                ..SubtitleFilters::default()
            },
            context: ContextConfig {
                leading: self.context_leading,
                trailing: self.context_trailing,
                max_gap_seconds: self.max_gap,
            },
            combine_sentences: self.combine_sentences,
            continuation_chars: self.continuation_chars.clone(),
            generate_audio: !self.no_audio,
            audio_bitrate: self.audio_bitrate,
            audio_track_index: self.audio_track,
            normalize_audio: self.normalize_audio,
            audio_boost: self.audio_gain != 0,
            audio_gain_db: self.audio_gain,
            audio_pad_start_ms: self.pad_audio_start,
            audio_pad_end_ms: self.pad_audio_end,
            audio_format: match self.audio_format.as_str() {
                "opus" => AudioFormat::Opus,
                _ => AudioFormat::Mp3,
            },
            generate_snapshots: !self.no_snapshots,
            snapshot_width: self.snapshot_width,
            snapshot_height: self.snapshot_height,
            crop_bottom: self.crop_bottom,
            snapshot_format: match self.snapshot_format.as_str() {
                "jpeg" => SnapshotFormat::Jpeg,
                "avif" => SnapshotFormat::Avif,
                _ => SnapshotFormat::Webp,
            },
            snapshot_quality: self.snapshot_quality,
            generate_video_clips: !self.no_video,
            video_codec: self.video_codec.clone(),
            h264_preset: self.h264_preset.clone(),
            video_hw_accel: self.video_hw_accel.clone(),
            video_bitrate: self.video_bitrate,
            video_audio_bitrate: self.video_audio_bitrate,
            video_pad_start_ms: self.pad_video_start,
            video_pad_end_ms: self.pad_video_end,
            deck_name: self.deck.clone(),
            episode_number: self.episode,
            export_format: Some(self.format.clone()),
            note_type_name: self.note_type.clone(),
            field_names: Some(FieldNamesConfig::default()),
            output_fields: OutputFields {
                include_audio: !self.no_audio,
                include_snapshot: !self.no_snapshots,
                include_video: !self.no_video,
                include_subs2: self.native.is_some(),
                ..OutputFields::default()
            },
            cpu_cores: self.jobs,
            optimize_video: !self.no_optimize,
            target_language: self.target_language.clone(),
            dictionary_lookup: !self.no_dictionary_lookup,
            auto_card_font: !self.no_auto_card_font,
            embed_card_font: !self.no_embed_card_font,
            ..FlashcardConfig::default()
        }
    }
}

#[derive(Serialize)]
struct PreviewJsonOutput<'a> {
    total: usize,
    active: usize,
    matched: usize,
    offset: usize,
    limit: usize,
    rows: Vec<&'a PreviewLine>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchManifest {
    output_dir: String,
    #[serde(default)]
    merge_output: Option<PathBuf>,
    #[serde(default)]
    defaults: BatchDefaults,
    episodes: Vec<BatchEpisode>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchDefaults {
    format: Option<String>,
    deck: Option<String>,
    note_type: Option<String>,
    target_language: Option<String>,
    #[serde(default)]
    no_dictionary_lookup: bool,
    #[serde(default)]
    no_auto_card_font: bool,
    #[serde(default)]
    no_embed_card_font: bool,
    jobs: Option<usize>,
    #[serde(default)]
    no_audio: bool,
    #[serde(default)]
    no_snapshots: bool,
    #[serde(default)]
    no_video: bool,
    #[serde(default)]
    no_optimize: bool,
    #[serde(default)]
    normalize_audio: bool,
    pad_audio_start: Option<i64>,
    pad_audio_end: Option<i64>,
    audio_gain: Option<i32>,
    audio_bitrate: Option<u32>,
    audio_format: Option<String>,
    audio_track: Option<usize>,
    snapshot_width: Option<u32>,
    snapshot_height: Option<u32>,
    snapshot_format: Option<String>,
    snapshot_quality: Option<u8>,
    crop_bottom: Option<u32>,
    video_codec: Option<String>,
    h264_preset: Option<String>,
    video_bitrate: Option<u32>,
    video_audio_bitrate: Option<u32>,
    video_hw_accel: Option<String>,
    pad_video_start: Option<i64>,
    pad_video_end: Option<i64>,
    min_chars: Option<usize>,
    max_chars: Option<usize>,
    min_duration: Option<i64>,
    max_duration: Option<i64>,
    include_words: Option<String>,
    exclude_words: Option<String>,
    #[serde(default)]
    exclude_duplicates: bool,
    #[serde(default, deserialize_with = "deserialize_actor_filter")]
    actor_filter: Vec<String>,
    #[serde(default)]
    only_cjk: bool,
    #[serde(default)]
    remove_no_match: bool,
    span_start: Option<i64>,
    span_end: Option<i64>,
    shift_target: Option<i64>,
    shift_native: Option<i64>,
    context_leading: Option<usize>,
    context_trailing: Option<usize>,
    max_gap: Option<f64>,
    #[serde(default)]
    combine_sentences: bool,
    continuation_chars: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BatchEpisode {
    target: String,
    native: Option<String>,
    video: Option<String>,
    audio: Option<String>,
    deck: Option<String>,
    episode: Option<u32>,
    audio_track: Option<usize>,
    crop_bottom: Option<u32>,
    shift_target: Option<i64>,
    shift_native: Option<i64>,
}

fn deserialize_actor_filter<'de, D>(deserializer: D) -> std::result::Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        Str(String),
        Vec(Vec<String>),
        Null,
    }
    match Option::<StringOrVec>::deserialize(deserializer)? {
        None | Some(StringOrVec::Null) => Ok(Vec::new()),
        Some(StringOrVec::Str(s)) => Ok(s
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(str::to_string)
            .collect()),
        Some(StringOrVec::Vec(v)) => Ok(v),
    }
}

fn sanitize_deck_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

impl BatchManifest {
    fn format(&self) -> &str {
        self.defaults.format.as_deref().unwrap_or("tsv")
    }

    fn validate(&self) -> Result<()> {
        ensure!(
            !self.output_dir.trim().is_empty(),
            "Manifest output_dir must not be empty"
        );
        ensure!(
            !self.episodes.is_empty(),
            "Manifest must contain at least one episode"
        );
        let format = self.format();
        ensure!(
            matches!(format, "tsv" | "apkg"),
            "Unsupported batch format '{format}': expected 'tsv' or 'apkg'"
        );
        if let Some(ref merge_output) = self.merge_output {
            ensure!(
                !merge_output.as_os_str().is_empty(),
                "Manifest merge_output must not be empty when specified"
            );
            ensure!(
                format == "apkg",
                "Manifest merge_output requires defaults.format to be 'apkg'"
            );
        }
        if let Some(ref deck) = self.defaults.deck {
            ensure!(
                !deck.trim().is_empty(),
                "Manifest defaults.deck must not be empty"
            );
        }
        if let Some(ref audio_format) = self.defaults.audio_format {
            ensure!(
                matches!(audio_format.as_str(), "mp3" | "opus"),
                "Unsupported audio_format '{audio_format}': expected 'mp3' or 'opus'"
            );
        }
        if let Some(ref snapshot_format) = self.defaults.snapshot_format {
            ensure!(
                matches!(snapshot_format.as_str(), "webp" | "jpeg" | "avif"),
                "Unsupported snapshot_format '{snapshot_format}': expected 'webp', 'jpeg', or 'avif'"
            );
        }
        if let Some(quality) = self.defaults.snapshot_quality {
            ensure!(
                quality <= 100,
                "snapshot_quality must be from 0 to 100 (got {quality})"
            );
        }
        if let Some(ref video_codec) = self.defaults.video_codec {
            ensure!(
                matches!(video_codec.as_str(), "h264" | "mpeg4"),
                "Unsupported video_codec '{video_codec}': expected 'h264' or 'mpeg4'"
            );
        }
        if let Some(ref hw_accel) = self.defaults.video_hw_accel {
            ensure!(
                matches!(hw_accel.as_str(), "auto" | "off"),
                "Unsupported video_hw_accel '{hw_accel}': expected 'auto' or 'off'"
            );
        }
        for (idx, ep) in self.episodes.iter().enumerate() {
            ensure!(
                !ep.target.trim().is_empty(),
                "Manifest episode {} has an empty target subtitle path",
                idx + 1
            );
            if let Some(ref deck) = ep.deck {
                ensure!(
                    !deck.trim().is_empty(),
                    "Manifest episode {} has an empty deck name",
                    idx + 1
                );
            }
        }
        Ok(())
    }

    fn uses_isolated_episode_dirs(&self) -> bool {
        let mut seen = HashSet::new();
        for ep in &self.episodes {
            let deck = ep
                .deck
                .as_deref()
                .or(self.defaults.deck.as_deref())
                .unwrap_or("Vesta");
            if !seen.insert(sanitize_deck_filename(deck)) {
                return true;
            }
        }
        false
    }

    fn build_episode_config(
        &self,
        index: usize,
        episode: &BatchEpisode,
        isolated_dir: bool,
        any_native: bool,
        tools_ffmpeg: &str,
        tools_ffprobe: &str,
    ) -> FlashcardConfig {
        let ep_number = episode.episode.unwrap_or((index + 1) as u32);
        let output_dir = if isolated_dir {
            Path::new(&self.output_dir)
                .join("episodes")
                .join(format!("episode-{:03}", index + 1))
                .to_string_lossy()
                .to_string()
        } else {
            self.output_dir.clone()
        };
        let d = &self.defaults;
        let args = GenerateArgs {
            target: episode.target.clone(),
            native: episode.native.clone(),
            video: episode.video.clone(),
            audio: episode.audio.clone(),
            output: Some(output_dir),
            format: self.format().to_string(),
            deck: episode
                .deck
                .clone()
                .or_else(|| d.deck.clone())
                .unwrap_or_else(|| "Vesta".to_string()),
            note_type: d.note_type.clone(),
            target_language: d.target_language.clone(),
            no_dictionary_lookup: d.no_dictionary_lookup,
            no_auto_card_font: d.no_auto_card_font,
            no_embed_card_font: d.no_embed_card_font,
            episode: ep_number,
            jobs: d.jobs,
            no_audio: d.no_audio,
            no_snapshots: d.no_snapshots,
            no_video: d.no_video,
            no_optimize: d.no_optimize,
            normalize_audio: d.normalize_audio,
            pad_audio_start: d.pad_audio_start.unwrap_or(0),
            pad_audio_end: d.pad_audio_end.unwrap_or(0),
            audio_gain: d.audio_gain.unwrap_or(0),
            audio_bitrate: d.audio_bitrate.unwrap_or(128),
            audio_format: d.audio_format.clone().unwrap_or_else(|| "mp3".to_string()),
            audio_track: episode.audio_track.or(d.audio_track),
            snapshot_width: d.snapshot_width.unwrap_or(256),
            snapshot_height: d.snapshot_height.unwrap_or(144),
            snapshot_format: d
                .snapshot_format
                .clone()
                .unwrap_or_else(|| "webp".to_string()),
            snapshot_quality: d.snapshot_quality.unwrap_or(80),
            crop_bottom: episode.crop_bottom.or(d.crop_bottom).unwrap_or(0),
            video_codec: d.video_codec.clone().unwrap_or_else(|| "h264".to_string()),
            h264_preset: d
                .h264_preset
                .clone()
                .unwrap_or_else(|| "ultrafast".to_string()),
            video_bitrate: d.video_bitrate.unwrap_or(1000),
            video_audio_bitrate: d.video_audio_bitrate.unwrap_or(128),
            video_hw_accel: d
                .video_hw_accel
                .clone()
                .unwrap_or_else(|| "auto".to_string()),
            pad_video_start: d.pad_video_start.unwrap_or(0),
            pad_video_end: d.pad_video_end.unwrap_or(0),
            min_chars: d.min_chars,
            max_chars: d.max_chars,
            min_duration: d.min_duration,
            max_duration: d.max_duration,
            include_words: d.include_words.clone(),
            exclude_words: d.exclude_words.clone(),
            exclude_duplicates: d.exclude_duplicates,
            actor_filter: d.actor_filter.clone(),
            only_cjk: d.only_cjk,
            remove_no_match: d.remove_no_match,
            span_start: d.span_start,
            span_end: d.span_end,
            shift_target: episode.shift_target.or(d.shift_target).unwrap_or(0),
            shift_native: episode.shift_native.or(d.shift_native).unwrap_or(0),
            context_leading: d.context_leading.unwrap_or(0),
            context_trailing: d.context_trailing.unwrap_or(0),
            max_gap: d.max_gap.unwrap_or(0.0),
            combine_sentences: d.combine_sentences,
            continuation_chars: d.continuation_chars.clone().unwrap_or_default(),
            ffmpeg: tools_ffmpeg.to_string(),
            ffprobe: tools_ffprobe.to_string(),
            quiet: true,
        };
        let mut config = args.to_config();
        // Keep APKG field schemas uniform across episodes when merging.
        if self.merge_output.is_some() && any_native {
            config.output_fields.include_subs2 = true;
        }
        config
    }
}

fn spawn_ctrl_c_token() -> CancellationToken {
    let cancel = CancellationToken::new();
    let cancel_signal = cancel.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            eprintln!("\nCancelling…");
            cancel_signal.cancel();
        }
    });
    cancel
}

async fn run_generation_with_progress(
    config: FlashcardConfig,
    tools: MediaTools,
    cancel: &CancellationToken,
    quiet: bool,
) -> Result<FlashcardResult> {
    let last_pct = AtomicI64::new(-1);
    let progress = move |ev: FlashcardProgressEvent| {
        if quiet {
            return;
        }
        let pct = ev.percentage as i64;
        if last_pct.swap(pct, Ordering::Relaxed) == pct && ev.stage == "media" {
            return;
        }
        if ev.stage == "media" {
            eprint!(
                "\r  [{:>3}%] extracting media {}/{}    ",
                pct, ev.current, ev.total
            );
        } else {
            eprintln!("\r  [{:>3}%] {}", pct, ev.stage);
        }
        use std::io::Write;
        let _ = std::io::stderr().flush();
    };

    let result = vesta_flashcards::generate(config, tools, cancel.clone(), &progress)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    if !quiet {
        eprintln!();
    }

    Ok(result)
}

fn copy_apkg_if_distinct(source: &Path, dest: &Path) -> Result<()> {
    if let (Ok(src_canon), Ok(dst_canon)) =
        (std::fs::canonicalize(source), std::fs::canonicalize(dest))
        && src_canon == dst_canon
    {
        return Ok(());
    }
    if let Some(parent) = dest.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {}", parent.display()))?;
    }
    std::fs::copy(source, dest)
        .with_context(|| format!("Failed to copy {} to {}", source.display(), dest.display()))?;
    Ok(())
}

async fn run_async(args: Vec<std::ffi::OsString>) -> ExitCode {
    match run(args).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

pub fn run_from(args: Vec<std::ffi::OsString>) -> ExitCode {
    match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime.block_on(run_async(args)),
        Err(error) => {
            eprintln!("Failed to start Tokio runtime: {error}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: Vec<std::ffi::OsString>) -> Result<ExitCode> {
    let cli = Cli::parse_from(args);

    match cli.command {
        Command::Info { file } => {
            let info = vesta_flashcards::load_sub_file_info(&file)
                .map_err(|e| anyhow::anyhow!(e))
                .with_context(|| format!("Failed to read {}", file))?;
            println!("File:     {}", info.path);
            println!("Format:   {}", info.format);
            println!("Lines:    {}", info.count);
            println!("Duration: {:.1} s", info.duration_ms as f64 / 1000.0);
            if !info.actors.is_empty() {
                println!("Actors:   {}", info.actors.join(", "));
            }
            println!("First:    {}", info.first_text);
            println!("Last:     {}", info.last_text);
        }

        Command::Preview(args) => {
            let config = args.common.to_config();
            let lines = vesta_flashcards::preview(&config).map_err(|e| anyhow::anyhow!(e))?;
            let active = lines.iter().filter(|l| l.active).count();
            let matched = lines.iter().filter(|l| l.subs2_text.is_some()).count();
            if args.json {
                let rows: Vec<&PreviewLine> =
                    lines.iter().skip(args.offset).take(args.limit).collect();
                let payload = PreviewJsonOutput {
                    total: lines.len(),
                    active,
                    matched,
                    offset: args.offset,
                    limit: args.limit,
                    rows,
                };
                println!("{}", serde_json::to_string_pretty(&payload)?);
            } else {
                println!("Total lines:   {}", lines.len());
                println!("Active lines:  {}", active);
                println!("With native:   {}", matched);
            }
        }

        Command::Dueling(args) => {
            let format = match args.dueling_format.as_str() {
                "srt" => DuelingFormat::Srt,
                _ => DuelingFormat::Ass,
            };
            let config = FlashcardConfig {
                target_subs_path: args.target,
                native_subs_path: args.native,
                deck_name: args.deck,
                time_shift_target_ms: args.shift_target,
                time_shift_native_ms: args.shift_native,
                span_start_ms: args.span_start,
                span_end_ms: args.span_end,
                combine_sentences: args.combine_sentences,
                continuation_chars: args.continuation_chars,
                filters: SubtitleFilters {
                    only_cjk: args.only_cjk,
                    remove_no_match: args.remove_no_match,
                    ..SubtitleFilters::default()
                },
                ..FlashcardConfig::default()
            };
            let result = vesta_flashcards::export_dueling_subtitles(
                &config,
                DuelingExportOptions {
                    format,
                    output_path: args.output,
                    target_first: !args.native_first,
                    include_unmatched: !args.remove_no_match,
                },
            )
            .map_err(|e| anyhow::anyhow!(e))?;
            println!(
                "✅ Exported {} dueling lines ({} matched) to {}",
                result.line_count, result.matched_count, result.output_path
            );
        }

        Command::Merge(args) => {
            for input in &args.inputs {
                ensure!(
                    input.is_file(),
                    "Input APKG file not found: {}",
                    input.display()
                );
            }
            if let Some(parent) = args.output.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create output directory {}", parent.display())
                })?;
            }
            vesta_flashcards::merge_apkg(&args.inputs, &args.output).with_context(|| {
                format!("Failed to merge APKG decks into {}", args.output.display())
            })?;
            println!(
                "✅ Merged {} APKG decks into {}",
                args.inputs.len(),
                args.output.display()
            );
        }

        Command::Batch(args) => {
            let bytes = std::fs::read(&args.manifest).with_context(|| {
                format!("Failed to read batch manifest {}", args.manifest.display())
            })?;
            let manifest: BatchManifest = serde_json::from_slice(&bytes).with_context(|| {
                format!("Failed to parse batch manifest {}", args.manifest.display())
            })?;
            manifest.validate()?;

            let tools = MediaTools::new(args.ffmpeg.clone(), args.ffprobe.clone());
            let cancel = spawn_ctrl_c_token();
            let isolated_dir = manifest.uses_isolated_episode_dirs();
            let any_native = manifest.episodes.iter().any(|ep| ep.native.is_some());
            let total_episodes = manifest.episodes.len();
            let mut apkg_paths = Vec::new();
            let start = std::time::Instant::now();

            for (idx, episode) in manifest.episodes.iter().enumerate() {
                if cancel.is_cancelled() {
                    if !args.quiet {
                        eprintln!("Batch generation cancelled.");
                    }
                    return Ok(ExitCode::from(130));
                }

                if !args.quiet {
                    eprintln!("Episode {}/{}: {}", idx + 1, total_episodes, episode.target);
                }

                let config = manifest.build_episode_config(
                    idx,
                    episode,
                    isolated_dir,
                    any_native,
                    &args.ffmpeg,
                    &args.ffprobe,
                );
                let result =
                    run_generation_with_progress(config, tools.clone(), &cancel, args.quiet)
                        .await?;

                if !result.success {
                    if cancel.is_cancelled() {
                        if !args.quiet {
                            eprintln!("Batch generation cancelled.");
                        }
                        return Ok(ExitCode::from(130));
                    }
                    anyhow::bail!(
                        "Batch episode {} ({}) failed: {}",
                        idx + 1,
                        episode.target,
                        result.message
                    );
                }

                println!(
                    "✅ Episode {}/{}: {}",
                    idx + 1,
                    total_episodes,
                    result.message
                );
                if let Some(ref p) = result.tsv_path {
                    println!("   TSV:  {p}");
                }
                if let Some(ref p) = result.apkg_path {
                    println!("   APKG: {p}");
                    if manifest.format() == "apkg" {
                        apkg_paths.push(PathBuf::from(p));
                    }
                }
            }

            if let Some(ref merge_output) = manifest.merge_output {
                if apkg_paths.len() == 1 {
                    copy_apkg_if_distinct(&apkg_paths[0], merge_output)?;
                    println!(
                        "✅ Copied 1 APKG deck to {} in {:.1}s",
                        merge_output.display(),
                        start.elapsed().as_secs_f64()
                    );
                } else {
                    ensure!(
                        apkg_paths.len() >= 2,
                        "Expected generated APKG paths to merge into {}",
                        merge_output.display()
                    );
                    if let Some(parent) =
                        merge_output.parent().filter(|p| !p.as_os_str().is_empty())
                    {
                        std::fs::create_dir_all(parent).with_context(|| {
                            format!("Failed to create output directory {}", parent.display())
                        })?;
                    }
                    vesta_flashcards::merge_apkg(&apkg_paths, merge_output).with_context(|| {
                        format!("Failed to merge APKG decks into {}", merge_output.display())
                    })?;
                    println!(
                        "✅ Merged {} APKG decks into {} in {:.1}s",
                        apkg_paths.len(),
                        merge_output.display(),
                        start.elapsed().as_secs_f64()
                    );
                }
            }
        }

        Command::Generate(args) => {
            if args.output.as_ref().is_none_or(|s| s.trim().is_empty()) {
                eprintln!(
                    "error: the following required arguments were not provided:\n  --output <OUTPUT>\n\nUsage: vesta-flashcards generate --target <TARGET> --output <OUTPUT>\n\nFor more information, try '--help'."
                );
                return Ok(ExitCode::from(2));
            }
            let config = args.to_config();
            let tools = MediaTools::new(args.ffmpeg.clone(), args.ffprobe.clone());
            let cancel = spawn_ctrl_c_token();

            let start = std::time::Instant::now();
            let result = run_generation_with_progress(config, tools, &cancel, args.quiet).await?;

            if result.success {
                println!(
                    "✅ {} ({} audio, {} snapshots, {} video) in {:.1}s",
                    result.message,
                    result.audio_clips,
                    result.snapshots,
                    result.video_clips,
                    start.elapsed().as_secs_f64()
                );
                if let Some(p) = result.tsv_path {
                    println!("   TSV:  {}", p);
                }
                if let Some(p) = result.apkg_path {
                    println!("   APKG: {}", p);
                }
            } else {
                if cancel.is_cancelled() {
                    if !args.quiet {
                        eprintln!("Generation cancelled.");
                    }
                    return Ok(ExitCode::from(130));
                }
                anyhow::bail!("Generation failed: {}", result.message);
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}
