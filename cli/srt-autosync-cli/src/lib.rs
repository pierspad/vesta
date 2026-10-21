//! # srt-autosync CLI
//!
//! Headless front-end for the [`srt_autosync`] engine: aligns an out-of-sync
//! SRT file to a media file by transcribing strategic audio samples with
//! Whisper and matching them against the subtitle text — the exact same
//! engine behind Vesta's "Auto Sync" button, with no GUI dependency.
//!
//! This binary is the "shell": it parses CLI arguments, runs the engine, and
//! applies the suggested anchors through `srt_sync::SyncEngine` (anchor-based
//! piecewise-linear interpolation) to write the synced SRT.

use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use clap::Parser;
use srt_autosync::{AnchorSuggestion, AutoSyncConfig, SubtitleLine, run_auto_sync};
use srt_sync::SyncEngine;
use tokio_util::sync::CancellationToken;

#[derive(Parser)]
#[command(
    name = "srt-autosync",
    version,
    about = "Automatically re-sync an SRT file to a media file using Whisper anchor points",
    long_about = None
)]
struct Cli {
    /// SRT file to re-sync.
    srt: String,
    /// Media file (video or audio) to align against.
    media: String,
    /// Output path for the synced SRT.
    #[arg(short, long)]
    output: String,
    /// Whisper model id (e.g. "tiny", "base", "small"). Downloaded on demand.
    #[arg(short, long, default_value = "base")]
    model: String,
    /// Spoken language hint (ISO code). Omit to autodetect.
    #[arg(short, long)]
    language: Option<String>,
    /// Quick mode: fewer, shorter audio samples (faster, less precise).
    #[arg(short, long)]
    quick: bool,
    /// ffmpeg executable (path or command on PATH).
    #[arg(long, default_value = "ffmpeg")]
    ffmpeg: String,
    /// ffprobe executable (path or command on PATH).
    #[arg(long, default_value = "ffprobe")]
    ffprobe: String,
    /// Suppress progress output on stderr.
    #[arg(long)]
    quiet: bool,
}

async fn run_async(args: Vec<std::ffi::OsString>) -> ExitCode {
    match run(args).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("srt-autosync: {error:#}");
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

    // Ctrl-C cancels both the on-demand model download and the analysis.
    let cancel_token = CancellationToken::new();
    {
        let token = cancel_token.clone();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                token.cancel();
            }
        });
    }

    // Load the SRT into the sync engine.
    let mut engine = SyncEngine::new(&cli.srt)
        .with_context(|| format!("Failed to load SRT file: {}", cli.srt))?;
    engine.set_video_path(&cli.media);

    let subtitles: Vec<SubtitleLine> = engine
        .get_all_subtitles()
        .iter()
        .map(|sub| SubtitleLine {
            id: sub.id,
            start_ms: sub.start.milliseconds as i64,
            text: sub.text.clone(),
        })
        .collect();

    // Ensure the Whisper model is available (download on demand).
    let model_path = match download_model(&cli.model, cli.quiet, &cancel_token).await? {
        ModelDownload::Ready(path) => path,
        ModelDownload::Cancelled => {
            eprintln!("Auto-sync cancelled during model download.");
            return Ok(ExitCode::from(130));
        }
    };

    if cancel_token.is_cancelled() {
        eprintln!("Auto-sync cancelled.");
        return Ok(ExitCode::from(130));
    }

    let config = AutoSyncConfig {
        media_path: cli.media.clone(),
        model_path,
        language: cli.language.clone(),
        quick: cli.quick,
        ffmpeg_cmd: cli.ffmpeg.clone(),
        ffprobe_cmd: cli.ffprobe.clone(),
    };

    let on_progress = (!cli.quiet).then(|| -> srt_autosync::ProgressCallback {
        Arc::new(|update| {
            eprintln!(
                "[{:>3.0}%] {} — {}",
                update.percentage, update.stage, update.message
            );
        })
    });

    let outcome = match run_auto_sync(&config, subtitles, on_progress, &cancel_token).await {
        Ok(outcome) => outcome,
        Err(_) if cancel_token.is_cancelled() => {
            eprintln!("Auto-sync cancelled.");
            return Ok(ExitCode::from(130));
        }
        Err(error) => return Err(error),
    };

    if outcome.cancelled {
        eprintln!(
            "Auto-sync cancelled after {} segments.",
            outcome.segments_analyzed
        );
        return Ok(ExitCode::from(130));
    }

    if outcome.suggestions.is_empty() {
        anyhow::bail!(
            "No confident matches found. Try a larger Whisper model or check that the SRT matches the audio."
        );
    }

    let Some(anchors_created) = apply_suggestions_and_save(
        &mut engine,
        &outcome.suggestions,
        &cli.output,
        &cancel_token,
    )?
    else {
        eprintln!("Auto-sync cancelled before writing output.");
        return Ok(ExitCode::from(130));
    };

    println!(
        "Created {} anchors from {} analyzed segments; synced SRT written to {}",
        anchors_created, outcome.segments_analyzed, cli.output
    );
    Ok(ExitCode::SUCCESS)
}

enum ModelDownload {
    Ready(std::path::PathBuf),
    Cancelled,
}

/// Apply the complete suggestion set before publishing. Cancellation is
/// checked immediately before the synchronous save begins; a save already in
/// progress cannot be interrupted safely.
fn apply_suggestions_and_save(
    engine: &mut SyncEngine,
    suggestions: &[AnchorSuggestion],
    output: &str,
    cancel_token: &CancellationToken,
) -> Result<Option<usize>> {
    for suggestion in suggestions {
        engine
            .add_anchor(suggestion.subtitle_id, suggestion.corrected_time_ms, false)
            .with_context(|| {
                format!(
                    "Failed to apply suggestion for subtitle {}",
                    suggestion.subtitle_id
                )
            })?;
    }

    if cancel_token.is_cancelled() {
        return Ok(None);
    }

    engine
        .save_synced_file(output)
        .with_context(|| format!("Failed to write synced SRT: {output}"))?;
    Ok(Some(suggestions.len()))
}

async fn download_model(
    model: &str,
    quiet: bool,
    cancel_token: &CancellationToken,
) -> Result<ModelDownload> {
    let model_id = model.to_owned();
    let downloaded = srt_transcribe::model::download_model(
        model,
        move |pct| {
            if !quiet {
                eprint!("\rDownloading model {model_id}... {pct}%");
            }
        },
        Some(cancel_token),
    )
    .await;

    match downloaded {
        Ok(path) => {
            if !quiet {
                eprintln!();
            }
            Ok(ModelDownload::Ready(path))
        }
        Err(_) if cancel_token.is_cancelled() => Ok(ModelDownload::Cancelled),
        Err(error) => Err(error).context("Model download failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancelled_model_download_returns_without_network_access() {
        let token = CancellationToken::new();
        token.cancel();

        let result = download_model("base", true, &token).await.unwrap();
        assert!(matches!(result, ModelDownload::Cancelled));
    }

    #[test]
    fn invalid_suggestion_does_not_replace_existing_output() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.srt");
        let output = dir.path().join("output.srt");
        std::fs::write(&input, "1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
        std::fs::write(&output, "existing output").unwrap();
        let mut engine = SyncEngine::new(&input).unwrap();
        let suggestions = [
            AnchorSuggestion {
                subtitle_id: 1,
                original_start_ms: 1_000,
                corrected_time_ms: 1_500,
                similarity: 1.0,
                score: 1.0,
            },
            AnchorSuggestion {
                subtitle_id: 99,
                original_start_ms: 2_000,
                corrected_time_ms: 2_500,
                similarity: 1.0,
                score: 1.0,
            },
        ];

        assert!(
            apply_suggestions_and_save(
                &mut engine,
                &suggestions,
                output.to_str().unwrap(),
                &CancellationToken::new(),
            )
            .is_err()
        );
        assert_eq!(std::fs::read_to_string(output).unwrap(), "existing output");
    }
}
