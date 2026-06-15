//! # srt-flashcards CLI
//!
//! Headless front-end for the [`srt_flashcards`] engine. It turns a subtitle
//! file (optionally a target + native pair) plus a media file into an Anki deck
//! (`tsv` + media folder, or a self-contained `.apkg`) — the exact same engine
//! the Vesta desktop app uses, with no GUI dependency.
//!
//! This binary is the "shell": it parses CLI arguments and delegates all logic
//! to the library, mirroring the `srt-extract` / `srt-translate` CLIs.

use std::sync::atomic::{AtomicI64, Ordering};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use srt_flashcards::{
    ContextConfig, FieldNamesConfig, FlashcardConfig, FlashcardProgressEvent, MediaTools,
    OutputFields, SubtitleFilters,
};
use tokio_util::sync::CancellationToken;

#[derive(Parser)]
#[command(
    name = "srt-flashcards",
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
    Generate(GenerateArgs),
    /// Print a summary of a subtitle file (count, format, actors, duration).
    Info {
        /// Subtitle file to inspect.
        file: String,
    },
    /// Run the parse/match/filter pipeline and report line counts (no media).
    Preview(GenerateArgs),
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
    output: String,
    /// Export format.
    #[arg(short, long, default_value = "tsv", value_parser = ["tsv", "apkg"])]
    format: String,
    /// Deck name (used for filenames and the Anki deck).
    #[arg(short, long, default_value = "Vesta")]
    deck: String,
    /// Episode number (used in the media filename pattern).
    #[arg(long, default_value_t = 1)]
    episode: u32,

    // ── Performance ──────────────────────────────────────────────────────
    /// Number of parallel ffmpeg workers. Clamped to [1, cores-1]
    /// (use 1 to run a single ffmpeg at a time, like a single-threaded tool).
    /// Omit to default to ~3/4 of the logical cores.
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
    /// Loudness-normalize audio clips (EBU R128).
    #[arg(long)]
    normalize_audio: bool,
    /// Audio bitrate (kbps).
    #[arg(long, default_value_t = 128)]
    audio_bitrate: u32,
    /// Explicit audio track index (ffmpeg 0:a:N order).
    #[arg(long)]
    audio_track: Option<usize>,
    /// Snapshot / video width in pixels.
    #[arg(long, default_value_t = 240)]
    snapshot_width: u32,
    /// Snapshot / video height in pixels.
    #[arg(long, default_value_t = 160)]
    snapshot_height: u32,
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
    /// Keep only lines whose script is CJK.
    #[arg(long)]
    only_cjk: bool,
    /// Drop lines that have no native-subtitle match.
    #[arg(long)]
    remove_no_match: bool,
    /// Restrict to lines overlapping [span-start, span-end] (milliseconds).
    #[arg(long)]
    span_start: Option<i64>,
    #[arg(long)]
    span_end: Option<i64>,
    /// Shift all target / native timestamps by N milliseconds.
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    shift_target: i64,
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

impl GenerateArgs {
    fn to_config(&self) -> FlashcardConfig {
        FlashcardConfig {
            target_subs_path: self.target.clone(),
            native_subs_path: self.native.clone(),
            video_path: self.video.clone(),
            audio_path: self.audio.clone(),
            output_dir: self.output.clone(),
            use_timings_from: "target".to_string(),
            span_start_ms: self.span_start,
            span_end_ms: self.span_end,
            time_shift_target_ms: self.shift_target,
            time_shift_native_ms: self.shift_native,
            filters: SubtitleFilters {
                include_words: self.include_words.clone(),
                exclude_words: self.exclude_words.clone(),
                exclude_duplicates_subs1: self.exclude_duplicates,
                exclude_duplicates_subs2: false,
                min_chars: self.min_chars,
                max_chars: self.max_chars,
                min_duration_ms: self.min_duration,
                max_duration_ms: self.max_duration,
                exclude_styled: false,
                actor_filter: None,
                only_cjk: self.only_cjk,
                remove_no_match: self.remove_no_match,
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
            audio_pad_start_ms: 0,
            audio_pad_end_ms: 0,
            generate_snapshots: !self.no_snapshots,
            snapshot_width: self.snapshot_width,
            snapshot_height: self.snapshot_height,
            crop_bottom: self.crop_bottom,
            generate_video_clips: !self.no_video,
            video_codec: self.video_codec.clone(),
            h264_preset: self.h264_preset.clone(),
            video_bitrate: self.video_bitrate,
            video_audio_bitrate: self.video_audio_bitrate,
            video_pad_start_ms: 0,
            video_pad_end_ms: 0,
            deck_name: self.deck.clone(),
            episode_number: self.episode,
            export_format: Some(self.format.clone()),
            note_type_name: None,
            field_names: Some(FieldNamesConfig::default()),
            output_fields: OutputFields {
                include_tag: true,
                include_sequence: true,
                include_audio: !self.no_audio,
                include_snapshot: !self.no_snapshots,
                include_video: !self.no_video,
                include_subs1: true,
                include_subs2: self.native.is_some(),
            },
            cpu_cores: self.jobs,
            card_front_html: None,
            card_back_html: None,
            card_css: None,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Info { file } => {
            let info = srt_flashcards::load_sub_file_info(&file)
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
            let config = args.to_config();
            let lines = srt_flashcards::preview(&config).map_err(|e| anyhow::anyhow!(e))?;
            let active = lines.iter().filter(|l| l.active).count();
            let matched = lines.iter().filter(|l| l.subs2_text.is_some()).count();
            println!("Total lines:   {}", lines.len());
            println!("Active lines:  {}", active);
            println!("With native:   {}", matched);
        }

        Command::Generate(args) => {
            let config = args.to_config();
            let tools = MediaTools::new(args.ffmpeg.clone(), args.ffprobe.clone());

            // Cooperative cancellation on Ctrl-C.
            let cancel = CancellationToken::new();
            let cancel_signal = cancel.clone();
            tokio::spawn(async move {
                if tokio::signal::ctrl_c().await.is_ok() {
                    eprintln!("\nCancelling…");
                    cancel_signal.cancel();
                }
            });

            // Throttled progress: only redraw when the integer percentage moves.
            let quiet = args.quiet;
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
                    eprint!("\r  [{:>3}%] extracting media {}/{}    ", pct, ev.current, ev.total);
                } else {
                    eprintln!("\r  [{:>3}%] {}", pct, ev.stage);
                }
                use std::io::Write;
                let _ = std::io::stderr().flush();
            };

            let start = std::time::Instant::now();
            let result = srt_flashcards::generate(config, tools, cancel, &progress)
                .await
                .map_err(|e| anyhow::anyhow!(e))?;

            if !quiet {
                eprintln!();
            }

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
                anyhow::bail!("Generation failed: {}", result.message);
            }
        }
    }

    Ok(())
}
