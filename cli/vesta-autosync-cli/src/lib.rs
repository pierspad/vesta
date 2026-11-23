//! # vesta-autosync CLI
//!
//! Headless front-end for the [`vesta_autosync`] engine: aligns an out-of-sync
//! SRT file to a media file by transcribing strategic audio samples with
//! Whisper and matching them against the subtitle text — the exact same
//! engine behind Vesta's "Auto Sync" button, with no GUI dependency.
//!
//! This binary is the "shell": it parses CLI arguments, runs the engine, and
//! applies the suggested anchors through `vesta_sync::SyncEngine` (anchor-based
//! piecewise-linear interpolation) to write the synced SRT.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context as _, Result, ensure};
use clap::Parser;
use tokio_util::sync::CancellationToken;
use vesta_autosync::{AnchorSuggestion, AutoSyncConfig, SubtitleLine, run_auto_sync_with_cloud};
use vesta_sync::SyncEngine;
use vesta_transcribe::cloud::CloudConfig;

#[derive(Parser)]
#[command(
    name = "vesta-autosync",
    version,
    about = "Automatically re-sync an SRT file to a media file using Whisper anchor points",
    long_about = None,
    after_help = "Examples:\n  vesta-autosync input.srt movie.mkv -o synced.srt --model small --language ja\n  vesta-autosync input.srt movie.mkv -o synced.srt --provider groq --model whisper-large-v3-turbo --api-key \"$GROQ_API_KEY\"\n  vesta-autosync input.srt movie.mkv -o synced.srt --provider custom --model whisper-1 --api-url http://localhost:8000/v1\n\nFor local providers, the Whisper model is downloaded on demand. Cloud providers require --api-key (except anonymous --provider custom endpoints, which require --api-url). ffmpeg and ffprobe must be installed or selected with --ffmpeg/--ffprobe. Use --quick for fewer, faster samples."
)]
struct Cli {
    /// SRT file to re-sync.
    srt: String,
    /// Media file (video or audio) to align against.
    media: String,
    /// Output path for the synced SRT.
    #[arg(short, long)]
    output: String,
    /// Whisper model id (local backend, e.g. "tiny", "base", "small") or provider model name (cloud).
    #[arg(short, long, default_value = "base")]
    model: String,
    /// Spoken language hint (ISO code). Omit to autodetect.
    #[arg(short, long)]
    language: Option<String>,
    /// Quick mode: fewer, shorter audio samples (faster, less precise).
    #[arg(short, long)]
    quick: bool,
    /// Transcription engine: "local" (whisper.cpp) or a cloud provider
    /// ("openai" | "groq" | "deepgram" | "assemblyai" | "custom").
    #[arg(
        long,
        default_value = "local",
        ignore_case = true,
        value_parser = [
            "local",
            "whisper",
            "local_whisper",
            "openai",
            "groq",
            "deepgram",
            "assemblyai",
            "custom"
        ]
    )]
    provider: String,
    /// API key for hosted cloud providers (optional for anonymous --provider custom endpoints).
    #[arg(long)]
    api_key: Option<String>,
    /// Base URL override (required when --provider is custom).
    #[arg(long)]
    api_url: Option<String>,
    /// Maximum subtitle-to-audio drift search window in seconds (defaults to 120 in the engine).
    #[arg(long)]
    max_offset_secs: Option<u32>,
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

fn is_local_provider(provider: &str) -> bool {
    matches!(
        provider.to_ascii_lowercase().as_str(),
        "local" | "whisper" | "local_whisper"
    )
}

fn validate_cli(cli: &Cli) -> Result<()> {
    let provider = cli.provider.to_ascii_lowercase();
    ensure!(
        provider != "custom"
            || cli
                .api_url
                .as_deref()
                .is_some_and(|url| !url.trim().is_empty()),
        "--api-url is required when --provider is custom"
    );
    ensure!(
        is_local_provider(&provider)
            || provider == "custom"
            || cli
                .api_key
                .as_deref()
                .is_some_and(|key| !key.trim().is_empty()),
        "--api-key is required for cloud provider '{provider}'"
    );
    if let Some(secs) = cli.max_offset_secs {
        ensure!(secs > 0, "--max-offset-secs must be at least 1");
    }
    Ok(())
}

fn build_cloud_config(cli: &Cli) -> Option<CloudConfig> {
    if is_local_provider(&cli.provider) {
        return None;
    }
    Some(CloudConfig {
        provider: cli.provider.to_ascii_lowercase(),
        api_key: cli.api_key.clone().unwrap_or_default(),
        api_url: cli
            .api_url
            .as_deref()
            .map(str::trim)
            .filter(|url| !url.is_empty())
            .map(str::to_string),
        model: cli.model.clone(),
        language: cli.language.clone(),
        translate_to_english: false,
    })
}

async fn run_async(args: Vec<std::ffi::OsString>) -> ExitCode {
    match run(args).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("vesta-autosync: {error:#}");
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
    validate_cli(&cli)?;

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

    let cloud = build_cloud_config(&cli);
    let model_path = if cloud.is_some() {
        PathBuf::new()
    } else {
        // Ensure the local Whisper model is available (download on demand).
        match download_model(&cli.model, cli.quiet, &cancel_token).await? {
            ModelDownload::Ready(path) => path,
            ModelDownload::Cancelled => {
                eprintln!("Auto-sync cancelled during model download.");
                return Ok(ExitCode::from(130));
            }
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
        use_gpu: None,
        max_offset_secs: cli.max_offset_secs,
        existing_anchors: Vec::new(),
    };

    let on_progress = (!cli.quiet).then(|| -> vesta_autosync::ProgressCallback {
        Arc::new(|update| {
            eprintln!(
                "[{:>3.0}%] {} — {}",
                update.percentage, update.stage, update.message
            );
        })
    });

    let outcome =
        match run_auto_sync_with_cloud(&config, subtitles, on_progress, &cancel_token, cloud).await
        {
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
    let downloaded = vesta_transcribe::model::download_model(
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

    #[test]
    fn cli_parses_cloud_and_offset_flags_with_defaults() {
        let defaults =
            Cli::try_parse_from(["vesta-autosync", "subs.srt", "video.mkv", "-o", "out.srt"])
                .unwrap();
        assert_eq!(defaults.provider, "local");
        assert!(defaults.api_key.is_none());
        assert!(defaults.api_url.is_none());
        assert!(defaults.max_offset_secs.is_none());
        assert!(validate_cli(&defaults).is_ok());
        assert!(build_cloud_config(&defaults).is_none());

        let cloud = Cli::try_parse_from([
            "vesta-autosync",
            "subs.srt",
            "video.mkv",
            "-o",
            "out.srt",
            "--provider",
            "Groq",
            "--model",
            "whisper-large-v3-turbo",
            "--api-key",
            "secret-key",
            "--language",
            "ja",
            "--max-offset-secs",
            "240",
        ])
        .unwrap();
        assert!(validate_cli(&cloud).is_ok());
        assert_eq!(cloud.max_offset_secs, Some(240));
        let cfg = build_cloud_config(&cloud).expect("cloud config");
        assert_eq!(cfg.provider, "groq");
        assert_eq!(cfg.api_key, "secret-key");
        assert_eq!(cfg.api_url, None);
        assert_eq!(cfg.model, "whisper-large-v3-turbo");
        assert_eq!(cfg.language.as_deref(), Some("ja"));
        assert!(!cfg.translate_to_english);

        assert!(
            Cli::try_parse_from([
                "vesta-autosync",
                "subs.srt",
                "video.mkv",
                "-o",
                "out.srt",
                "--provider",
                "unknown-provider",
            ])
            .is_err()
        );
    }

    #[test]
    fn cloud_validation_enforces_keys_for_hosted_and_url_for_custom() {
        for provider in ["local", "whisper", "local_whisper"] {
            let cli = Cli::try_parse_from([
                "vesta-autosync",
                "subs.srt",
                "video.mkv",
                "-o",
                "out.srt",
                "--provider",
                provider,
            ])
            .unwrap();
            assert!(validate_cli(&cli).is_ok(), "{provider}");
            assert!(build_cloud_config(&cli).is_none(), "{provider}");
        }

        for provider in ["openai", "groq", "deepgram", "assemblyai"] {
            let missing = Cli::try_parse_from([
                "vesta-autosync",
                "subs.srt",
                "video.mkv",
                "-o",
                "out.srt",
                "--provider",
                provider,
            ])
            .unwrap();
            let err = validate_cli(&missing).unwrap_err().to_string();
            assert!(err.contains("--api-key"), "{provider}: {err}");

            let blank = Cli::try_parse_from([
                "vesta-autosync",
                "subs.srt",
                "video.mkv",
                "-o",
                "out.srt",
                "--provider",
                provider,
                "--api-key",
                "   ",
            ])
            .unwrap();
            assert!(validate_cli(&blank).is_err(), "{provider}");

            let valid = Cli::try_parse_from([
                "vesta-autosync",
                "subs.srt",
                "video.mkv",
                "-o",
                "out.srt",
                "--provider",
                provider,
                "--api-key",
                "k",
            ])
            .unwrap();
            assert!(validate_cli(&valid).is_ok(), "{provider}");
        }

        let custom_no_url = Cli::try_parse_from([
            "vesta-autosync",
            "subs.srt",
            "video.mkv",
            "-o",
            "out.srt",
            "--provider",
            "custom",
        ])
        .unwrap();
        assert!(
            validate_cli(&custom_no_url)
                .unwrap_err()
                .to_string()
                .contains("--api-url")
        );

        let custom_anon = Cli::try_parse_from([
            "vesta-autosync",
            "subs.srt",
            "video.mkv",
            "-o",
            "out.srt",
            "--provider",
            "custom",
            "--api-url",
            " http://127.0.0.1:8000/v1 ",
        ])
        .unwrap();
        assert!(validate_cli(&custom_anon).is_ok());
        let custom_cfg = build_cloud_config(&custom_anon).unwrap();
        assert_eq!(custom_cfg.provider, "custom");
        assert_eq!(custom_cfg.api_key, "");
        assert_eq!(
            custom_cfg.api_url.as_deref(),
            Some("http://127.0.0.1:8000/v1")
        );

        let zero_offset = Cli::try_parse_from([
            "vesta-autosync",
            "subs.srt",
            "video.mkv",
            "-o",
            "out.srt",
            "--max-offset-secs",
            "0",
        ])
        .unwrap();
        assert!(validate_cli(&zero_offset).is_err());
    }
}
