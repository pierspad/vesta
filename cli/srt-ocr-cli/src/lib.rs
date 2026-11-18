//! # srt-ocr CLI
//!
//! Headless front-end for [`srt_ocr`]: extracts a bitmap subtitle track from a
//! media container and converts it into an SRT subtitle file via Vision LLMs.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use anyhow::{Result, ensure};
use clap::{Parser, Subcommand};
use srt_ocr::model::{download_model, list_models, uninstall_model};
use srt_ocr::pipeline::{OcrCallbacks, OcrConfig, ocr_bitmap_track_to_srt};
use srt_translate::{TierEntry, provider_allows_missing_key};
use tokio_util::sync::CancellationToken;

#[derive(Parser)]
#[command(
    name = "srt-ocr",
    version,
    about = "Extract and OCR bitmap subtitle tracks (PGS, VobSub, DVB, XSUB) to SRT",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Extract and OCR a bitmap subtitle track from a media container to SRT.
    #[command(
        after_help = "Examples:\n  srt-ocr run -i movie.mkv -o movie.srt -t 2\n  srt-ocr run -i movie.mkv -o movie.srt -t 2 --provider openai --model gpt-4o-mini --api-key \"$OPENAI_API_KEY\"\n  srt-ocr run -i movie.mkv -o movie.srt -t 2 --provider custom --model llava --api-url http://localhost:11434/v1\n\nFind subtitle stream indices with `srt-extract -i movie.mkv --list-tracks`. Cloud providers require --api-key; --provider custom requires --api-url."
    )]
    Run(RunArgs),
    /// List available local OCR models and cache status.
    #[command(after_help = "Example:\n  srt-ocr models")]
    Models,
    /// Download a local OCR model into the cache directory.
    #[command(after_help = "Example:\n  srt-ocr download paddleocr-v4-mobile-multi")]
    Download {
        /// Model ID to download (see `models`).
        model_id: String,
    },
    /// Remove a cached local OCR model.
    #[command(after_help = "Example:\n  srt-ocr remove paddleocr-v4-mobile-multi")]
    Remove {
        /// Model ID to remove (see `models`).
        model_id: String,
    },
}

#[derive(clap::Args)]
struct RunArgs {
    /// Input media container path.
    #[arg(short, long)]
    input: PathBuf,
    /// Output `.srt` file path.
    #[arg(short, long)]
    output: PathBuf,
    /// Absolute FFprobe stream index of the bitmap subtitle track.
    #[arg(short, long)]
    track: u32,
    /// Vision LLM provider ("local", "google", "gemini", "openai", "anthropic", "groq", "openrouter", "mistral", "nvidia", or "custom").
    #[arg(long, default_value = "local", ignore_case = true, value_parser = ["local", "google", "gemini", "openai", "anthropic", "groq", "openrouter", "mistral", "nvidia", "custom"])]
    provider: String,
    /// Vision model name for the selected provider.
    #[arg(short, long, default_value = "llava")]
    model: String,
    /// API key for cloud providers (optional for local/custom endpoints that allow anonymous access).
    #[arg(long)]
    api_key: Option<String>,
    /// Base URL override (required for --provider custom).
    #[arg(long)]
    api_url: Option<String>,
    /// Optional ISO language code or language name hint for OCR.
    #[arg(short, long)]
    language: Option<String>,
    /// ffmpeg executable (path or command on PATH).
    #[arg(long, default_value = "ffmpeg")]
    ffmpeg: String,
    /// ffprobe executable (path or command on PATH).
    #[arg(long, default_value = "ffprobe")]
    ffprobe: String,
    /// Suppress progress output on stderr.
    #[arg(short, long)]
    quiet: bool,
    /// Emit JSON-Lines progress and completion events on stdout (`OcrProgress` lines followed by `{"ok":...}`). Implies `--quiet` on stderr.
    #[arg(long)]
    json: bool,
}

async fn run_async(args: Vec<std::ffi::OsString>) -> ExitCode {
    match run_cli(args).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("srt-ocr: {error:#}");
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

async fn run_cli(args: Vec<std::ffi::OsString>) -> Result<ExitCode> {
    let cli = Cli::parse_from(args);

    match cli.command {
        Command::Models => {
            for model in list_models()? {
                let status = if model.downloaded { "[downloaded]" } else { "" };
                println!(
                    "{:<28} {:<36} {:>8}  {}",
                    model.id, model.name, model.size, status
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Download { model_id } => {
            let cancel_token = CancellationToken::new();
            {
                let token = cancel_token.clone();
                tokio::spawn(async move {
                    if tokio::signal::ctrl_c().await.is_ok() {
                        token.cancel();
                    }
                });
            }
            let label = model_id.clone();
            let progress = move |pct| eprint!("\rDownloading {label}... {pct}%");
            match download_model(&model_id, progress, Some(&cancel_token)).await {
                Ok(path) => {
                    eprintln!("\nModel saved to {}", path.display());
                    Ok(ExitCode::SUCCESS)
                }
                Err(_) if cancel_token.is_cancelled() => {
                    eprintln!("\nDownload cancelled.");
                    Ok(ExitCode::from(130))
                }
                Err(error) => Err(error),
            }
        }
        Command::Remove { model_id } => {
            let removed = uninstall_model(&model_id)?;
            println!(
                "{}",
                if removed {
                    "Model removed."
                } else {
                    "Model was not installed."
                }
            );
            Ok(ExitCode::SUCCESS)
        }
        Command::Run(args) => run_ocr(args).await,
    }
}

async fn run_ocr(args: RunArgs) -> Result<ExitCode> {
    if let Err(error) = validate_run_args(&args) {
        if args.json {
            println!(
                "{}",
                serde_json::json!({
                    "ok": false,
                    "cancelled": false,
                    "error": format!("{error:#}")
                })
            );
            return Ok(ExitCode::FAILURE);
        }
        return Err(error);
    }

    let tiers = vec![vec![TierEntry {
        provider: args.provider,
        model: args.model,
        api_key: args.api_key,
        api_url: args.api_url,
        rpm: None,
        max_requests: None,
    }]];
    let config = OcrConfig {
        input_path: args.input,
        output_path: args.output,
        track_index: args.track,
        language: args.language,
        tiers,
        preview_limit: None,
        ffmpeg_path: args.ffmpeg,
        ffprobe_path: args.ffprobe,
    };

    let cancel_token = CancellationToken::new();
    {
        let token = cancel_token.clone();
        let is_json = args.json;
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                if !is_json {
                    eprintln!("\nCancelling...");
                }
                token.cancel();
            }
        });
    }

    if args.json {
        return run_json(config, &cancel_token).await;
    }

    let last_progress = Arc::new(Mutex::new((String::new(), -1i64)));
    let callbacks = if args.quiet {
        OcrCallbacks::default()
    } else {
        OcrCallbacks {
            on_progress: Some(Arc::new(move |update| {
                let pct = update.percentage.round() as i64;
                let mut guard = last_progress
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if guard.0 != update.stage || guard.1 != pct {
                    guard.0 = update.stage.clone();
                    guard.1 = pct;
                    drop(guard);
                    eprintln!("[{:>3}%] {} — {}", pct, update.stage, update.message);
                }
            })),
            on_segment: None,
        }
    };

    let outcome = match ocr_bitmap_track_to_srt(&config, callbacks, &cancel_token).await {
        Ok(outcome) => outcome,
        Err(_) if cancel_token.is_cancelled() => {
            eprintln!("OCR cancelled.");
            return Ok(ExitCode::from(130));
        }
        Err(error) => return Err(error),
    };

    println!(
        "Wrote {} subtitles to {}",
        outcome.subtitle_count, outcome.output_path
    );
    Ok(ExitCode::SUCCESS)
}

fn validate_run_args(args: &RunArgs) -> Result<()> {
    let provider = args.provider.to_ascii_lowercase();
    if provider == "custom" {
        ensure!(
            args.api_url
                .as_deref()
                .is_some_and(|url| !url.trim().is_empty()),
            "--api-url is required when --provider is custom"
        );
    }
    if !provider_allows_missing_key(&provider) {
        ensure!(
            args.api_key
                .as_deref()
                .is_some_and(|key| !key.trim().is_empty()),
            "--api-key is required for cloud provider '{provider}'"
        );
    }
    ensure!(!args.model.trim().is_empty(), "--model must not be empty");
    srt_parser::persistence::ensure_distinct_files(&args.input, &args.output)?;
    Ok(())
}

/// `--json` mode: one `OcrProgress` JSON object per line on stdout while
/// running, then a single terminal result line.
async fn run_json(config: OcrConfig, cancel_token: &CancellationToken) -> Result<ExitCode> {
    let callbacks = OcrCallbacks {
        on_progress: Some(Arc::new(|update| {
            if let Ok(line) = serde_json::to_string(&update) {
                println!("{line}");
            }
        })),
        on_segment: None,
    };

    match ocr_bitmap_track_to_srt(&config, callbacks, cancel_token).await {
        Ok(outcome) => {
            println!("{}", serde_json::json!({ "ok": true, "outcome": outcome }));
            Ok(ExitCode::SUCCESS)
        }
        Err(e) => {
            let cancelled = cancel_token.is_cancelled();
            println!(
                "{}",
                serde_json::json!({
                    "ok": false,
                    "cancelled": cancelled,
                    "error": format!("{e:#}")
                })
            );
            Ok(if cancelled {
                ExitCode::from(130)
            } else {
                ExitCode::FAILURE
            })
        }
    }
}
