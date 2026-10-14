//! # srt-transcribe CLI
//!
//! Headless front-end for [`srt_transcribe::pipeline`]: turns a media file
//! (video or audio) into an SRT subtitle file using whisper.cpp locally or a
//! cloud provider — the exact same engine the Vesta desktop app uses, with no
//! GUI dependency.
//!
//! This binary is the "shell": it parses CLI arguments and delegates all logic
//! to the library, mirroring the `srt-flashcards` / `srt-translate` CLIs.

use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use anyhow::Result;
use clap::{Parser, Subcommand};
use srt_transcribe::model::{
    DEFAULT_VAD_MODEL_ID, download_model, download_vad_model, list_models, list_vad_models,
    uninstall_model, uninstall_vad_model,
};
use srt_transcribe::pipeline::{PipelineCallbacks, TranscriptionConfig, transcribe_to_srt};
use tokio_util::sync::CancellationToken;

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
    /// List available Whisper models (and the Silero VAD add-on) with status.
    Models,
    /// Download a Whisper model (e.g. "base", "small") or "vad" (Silero VAD).
    Download { model_id: String },
    /// Delete a downloaded Whisper model, or "vad" for the Silero VAD model.
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
    /// Quality mode: beam-search decoding (width 5) instead of greedy.
    /// ~2-3x slower, sometimes more accurate on difficult audio (local only).
    #[arg(long)]
    quality: bool,
    /// Run Silero VAD before transcribing: skips silence/music and reduces
    /// hallucinations (local only; requires `srt-transcribe download vad`).
    #[arg(long)]
    vad: bool,
    /// Force CPU execution even if a GPU backend is compiled in. By default,
    /// GPU acceleration is used whenever available, with automatic CPU fallback.
    #[arg(long)]
    no_gpu: bool,
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
    /// Machine-readable mode: one JSON `ProgressUpdate` per line on stdout
    /// while running, then a final `{"ok":true,"outcome":{...}}` or
    /// `{"ok":false,"error":"..."}` line. Meant for callers that spawn this
    /// binary as a subprocess (e.g. whisper-bench's GPU worker binaries)
    /// rather than for interactive use. Implies --quiet for stderr.
    #[arg(long)]
    json: bool,
}

/// Resolve a `--model-id` value into a VAD variant id, if it refers to one:
/// bare `"vad"` means the default variant, `"vad:<id>"` an explicit one.
fn vad_variant_id(model_id: &str) -> Option<&str> {
    if model_id == "vad" {
        Some(DEFAULT_VAD_MODEL_ID)
    } else {
        model_id.strip_prefix("vad:")
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    match run_cli().await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("srt-transcribe: {error:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run_cli() -> Result<ExitCode> {
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
            // "vad" is shorthand for the default variant; other variants are
            // addressed as "vad:<id>" (e.g. "vad:v6.2.0").
            for vad in list_vad_models() {
                let status = if vad.downloaded { "[downloaded]" } else { "" };
                let id_label = if vad.id == DEFAULT_VAD_MODEL_ID {
                    "vad".to_string()
                } else {
                    format!("vad:{}", vad.id)
                };
                println!(
                    "{:<8} {:<8} {:>8}  {:<11} {}",
                    id_label, "Silero", vad.size, "(VAD add-on)", status
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Download { model_id } => {
            let label = model_id.clone();
            let progress = move |pct| eprint!("\rDownloading {label}... {pct}%");
            let path = if let Some(vad_id) = vad_variant_id(&model_id) {
                download_vad_model(vad_id, progress, None).await?
            } else {
                download_model(&model_id, progress, None).await?
            };
            eprintln!("\nModel saved to {}", path.display());
            Ok(ExitCode::SUCCESS)
        }
        Command::Remove { model_id } => {
            let removed = if let Some(vad_id) = vad_variant_id(&model_id) {
                uninstall_vad_model(vad_id)?
            } else {
                uninstall_model(&model_id)?
            };
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
        Command::Run(args) => run_transcription(args).await,
    }
}

async fn run_transcription(args: RunArgs) -> Result<ExitCode> {
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
        quality: args.quality,
        vad: args.vad,
        vad_model_id: None,
        vad_custom_path: None,
        use_gpu: !args.no_gpu,
    };

    // Ctrl-C → cooperative cancellation.
    let cancel_token = CancellationToken::new();
    {
        let token = cancel_token.clone();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                if !args.json {
                    eprintln!("\nCancelling...");
                }
                token.cancel();
            }
        });
    }

    if args.json {
        return run_json(config, &args.ffmpeg, &cancel_token).await;
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

    let outcome = match transcribe_to_srt(&config, &args.ffmpeg, callbacks, &cancel_token).await {
        Ok(outcome) => outcome,
        Err(_) if cancel_token.is_cancelled() => {
            eprintln!("Transcription cancelled.");
            return Ok(ExitCode::from(130));
        }
        Err(error) => return Err(error),
    };

    println!(
        "Wrote {} subtitles to {}{}",
        outcome.subtitle_count,
        outcome.output_path,
        outcome
            .detected_language
            .map(|l| format!(" (language: {l})"))
            .unwrap_or_default()
    );
    Ok(ExitCode::SUCCESS)
}

/// `--json` mode: one `ProgressUpdate` JSON object per line on stdout while
/// running, then a single final result line. Never propagates a transcription
/// error via `?` — it always reports it as a `{"ok":false,...}` line so a
/// parent process reading line-by-line always gets a clean terminal message,
/// then exits non-zero.
async fn run_json(
    config: TranscriptionConfig,
    ffmpeg: &str,
    cancel_token: &CancellationToken,
) -> Result<ExitCode> {
    let callbacks = PipelineCallbacks {
        on_progress: Some(Arc::new(|update| {
            if let Ok(line) = serde_json::to_string(&update) {
                println!("{line}");
            }
        })),
        on_segment: None,
    };

    match transcribe_to_srt(&config, ffmpeg, callbacks, cancel_token).await {
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
