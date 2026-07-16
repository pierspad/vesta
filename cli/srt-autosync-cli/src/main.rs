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

use std::sync::Arc;

use anyhow::{Context as _, Result};
use clap::Parser;
use srt_autosync::{AutoSyncConfig, SubtitleLine, run_auto_sync};
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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

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
    let model_id = cli.model.clone();
    let model_path = srt_transcribe::model::download_model(
        &cli.model,
        move |pct| eprint!("\rDownloading model {model_id}... {pct}%"),
        None,
    )
    .await
    .context("Model download failed")?;
    eprintln!();

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

    let outcome = run_auto_sync(&config, subtitles, on_progress, &cancel_token).await?;

    if outcome.cancelled {
        eprintln!(
            "Auto-sync cancelled after {} segments.",
            outcome.segments_analyzed
        );
        return Ok(());
    }

    if outcome.suggestions.is_empty() {
        anyhow::bail!(
            "No confident matches found. Try a larger Whisper model or check that the SRT matches the audio."
        );
    }

    let mut anchors_created = 0usize;
    for s in &outcome.suggestions {
        if engine
            .add_anchor(s.subtitle_id, s.corrected_time_ms, false)
            .is_ok()
        {
            anchors_created += 1;
        }
    }

    engine
        .save_synced_file(&cli.output)
        .with_context(|| format!("Failed to write synced SRT: {}", cli.output))?;

    println!(
        "Created {} anchors from {} analyzed segments; synced SRT written to {}",
        anchors_created, outcome.segments_analyzed, cli.output
    );
    Ok(())
}
