//! # srt-transcribe CLI
//!
//! Headless front-end for [`whisper_common::pipeline`]: turns a media file
//! (video or audio) into an SRT subtitle file using whisper.cpp locally or a
//! cloud provider — the exact same engine the Vesta desktop app uses, with no
//! GUI dependency.
//!
//! This binary is the "shell": it parses CLI arguments and delegates all logic
//! to the library, mirroring the `srt-flashcards` / `srt-translate` CLIs.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tokio_util::sync::CancellationToken;
use whisper_common::model::{download_model, list_models, uninstall_model};
use whisper_common::pipeline::{transcribe_to_srt, PipelineCallbacks, TranscriptionConfig};

#[derive(Parser)]
#[command(
    name = "srt-transcribe",
    version,
    about = "Transcribe a media file to SRT subtitles (whisper.cpp or cloud providers)",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Transcribe a media file to SRT.
    Run(RunArgs),
    /// List available Whisper models and their download status.
    Models,
    /// Download a Whisper model (e.g. "base", "small", "large-v3").
    Download { model_id: String },
    /// Delete a downloaded Whisper model.
    Remove { model_id: String },
}

#[derive(clap::Args)]
struct RunArgs {
    /// Input media file (video or audio).
    input: String,
    /// Output SRT path. With --language auto, the detected language may be
    /// appended to the file name ("movie.en.srt" style).
    #[arg(short, long)]
    output: String,
    /// Whisper model id (local backend) or provider model name (cloud).
    #[arg(short, long, default_value = "base")]
    model: String,
    /// ISO language code, or "auto" to autodetect (local backend only).
    #[arg(short, long, default_value = "auto")]
    language: String,
    /// Translate the transcription to English (local backend only).
    #[arg(long)]
    translate_to_english: bool,
    /// Enable word-level timestamps for finer segmentation.
    #[arg(long)]
    word_timestamps: bool,
    /// Maximum characters per subtitle line (0 = default of 80).
    #[arg(long, default_value_t = 0)]
    max_segment_length: u32,
    /// Transcription engine: "local" (whisper.cpp) or a cloud provider
    /// ("groq" | "openai" | "deepgram" | "assemblyai" | "custom").
    #[arg(long, default_value = "local")]
    provider: String,
    /// API key for cloud providers.
    #[arg(long)]
    api_key: Option<String>,
    /// Base URL override (required for --provider custom).
    #[arg(long)]
    api_url: Option<String>,
    /// ffmpeg executable (path or command on PATH).
    #[arg(long, default_value = "ffmpeg")]
    ffmpeg: String,
    /// Suppress progress output on stderr.
    #[arg(short, long)]
    quiet: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Models => {
            for model in list_models()? {
                let status = if model.downloaded { "[downloaded]" } else { "" };
                println!(
                    "{:<8} {:<8} {:>8}  speed {:<5} {}",
                    model.id, model.name, model.size, model.speed, status
                );
            }
            Ok(())
        }
        Command::Download { model_id } => {
            let label = model_id.clone();
            let path = download_model(
                &model_id,
                move |pct| eprint!("\rDownloading {label}... {pct}%"),
                None,
            )
            .await?;
            eprintln!("\nModel saved to {}", path.display());
            Ok(())
        }
        Command::Remove { model_id } => {
            let removed = uninstall_model(&model_id)?;
            println!("{}", if removed { "Model removed." } else { "Model was not installed." });
            Ok(())
        }
        Command::Run(args) => run(args).await,
    }
}

async fn run(args: RunArgs) -> Result<()> {
    let config = TranscriptionConfig {
        input_path: args.input,
        output_path: args.output,
        model: args.model,
        language: args.language,
        translate_to_english: args.translate_to_english,
        word_timestamps: args.word_timestamps,
        max_segment_length: args.max_segment_length,
        provider: Some(args.provider),
        api_key: args.api_key,
        api_url: args.api_url,
    };

    // Ctrl-C → cooperative cancellation.
    let cancel_token = CancellationToken::new();
    {
        let token = cancel_token.clone();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                eprintln!("\nCancelling...");
                token.cancel();
            }
        });
    }

    // Throttled progress on stderr (at most one line per whole percent).
    let last_pct = Arc::new(AtomicI64::new(-1));
    let callbacks = if args.quiet {
        PipelineCallbacks::default()
    } else {
        PipelineCallbacks {
            on_progress: Some(Arc::new(move |update| {
                let pct = update.percentage as i64;
                if last_pct.swap(pct, Ordering::Relaxed) != pct {
                    eprintln!("[{:>3}%] {} — {}", pct, update.stage, update.message);
                }
            })),
            on_segment: None,
        }
    };

    let outcome = transcribe_to_srt(&config, &args.ffmpeg, callbacks, &cancel_token).await?;

    println!(
        "Wrote {} subtitles to {}{}",
        outcome.subtitle_count,
        outcome.output_path,
        outcome
            .detected_language
            .map(|l| format!(" (language: {l})"))
            .unwrap_or_default()
    );
    Ok(())
}
