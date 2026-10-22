use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use srt_refine::{
    RefineEvent, RefineRunConfig, RefineUpdate, load_cards, refine_cards_tiered, save_cards,
};
use srt_translate::{TierEntry, build_pool, provider_allows_missing_key};
use std::{
    path::{Path, PathBuf},
    process::ExitCode,
    sync::{Arc, Mutex},
};
use tokio_util::sync::CancellationToken;

#[derive(Parser)]
#[command(
    name = "srt-refine",
    about = "Inspect and annotate TSV/APKG decks",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print card fields as JSON without contacting any provider
    #[command(
        after_help = "Example:\n  srt-refine inspect --input deck.apkg > cards.json\n\nInput must be a TSV or APKG deck."
    )]
    Inspect {
        /// TSV or APKG deck to read.
        #[arg(short, long)]
        input: PathBuf,
    },
    /// Annotate notes using the shared tiered engine
    #[command(long_about = r#"Annotate notes using the shared tiered engine.

The JSON config must contain a non-empty `prompt` and `tiers` as arrays of
provider entries. `batch_mode` defaults to false and `batch_size` defaults to 8.
Each entry needs `provider` and `model`; hosted providers also need `api_key`.
For example:
{"prompt":"Explain this phrase","tiers":[[{"provider":"openai","model":"gpt-4o-mini","api_key":"YOUR_API_KEY"}]]}

Supported output conversions are TSV to TSV, APKG to TSV, and APKG to APKG."#)]
    Enrich {
        /// TSV or APKG deck to annotate.
        #[arg(short, long)]
        input: PathBuf,
        /// Destination deck path (allowed conversions are listed above).
        #[arg(short, long)]
        output: PathBuf,
        /// JSON file with non-empty prompt and tier arrays; batch_mode defaults false, batch_size defaults 8
        #[arg(short, long)]
        config: PathBuf,
        /// Publish successful annotations even if some cards fail; still exits 1
        #[arg(long)]
        allow_partial: bool,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    tiers: Vec<Vec<TierEntry>>,
    prompt: String,
    #[serde(default)]
    batch_mode: bool,
    #[serde(default = "default_batch_size")]
    batch_size: usize,
}
fn default_batch_size() -> usize {
    8
}

fn path_text(path: &Path) -> Result<&str> {
    path.to_str()
        .context("The card engine requires a UTF-8 file path")
}

fn validate_output(input: &Path, output: &Path) -> Result<()> {
    let extension = |path: &Path| {
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
    };
    ensure!(
        matches!(
            (extension(input).as_str(), extension(output).as_str()),
            ("tsv", "tsv") | ("apkg", "tsv") | ("apkg", "apkg")
        ),
        "Supported conversions: TSV to TSV, APKG to TSV or APKG to APKG"
    );
    let parent = output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    ensure!(
        parent.is_dir(),
        "Output directory does not exist: {}",
        parent.display()
    );
    ensure!(
        !output.is_dir(),
        "Output is a directory: {}",
        output.display()
    );
    Ok(())
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

async fn run(args: Vec<std::ffi::OsString>) -> Result<u8> {
    match Cli::parse_from(args).command {
        Command::Inspect { input } => {
            let cards = load_cards(path_text(&input)?)
                .map_err(anyhow::Error::msg)
                .with_context(|| format!("Failed to load {}", input.display()))?;
            println!("{}", serde_json::to_string_pretty(&cards)?);
            Ok(0)
        }
        Command::Enrich {
            input,
            output,
            config,
            allow_partial,
        } => {
            validate_output(&input, &output)?;
            let input_text = path_text(&input)?;
            let output_text = path_text(&output)?;
            let cards = load_cards(input_text)
                .map_err(anyhow::Error::msg)
                .with_context(|| format!("Failed to load {}", input.display()))?;
            ensure!(!cards.is_empty(), "The input contains no annotatable cards");
            let bytes = std::fs::read(&config)
                .with_context(|| format!("Failed to read configuration {}", config.display()))?;
            let config: Config = serde_json::from_slice(&bytes).map_err(|error| {
                anyhow::anyhow!(
                    "Invalid refinement configuration at line {}, column {}",
                    error.line(),
                    error.column()
                )
            })?;
            ensure!(
                !config.prompt.trim().is_empty(),
                "The annotation prompt must not be empty"
            );
            ensure!(config.batch_size > 0, "batch_size must be positive");
            for entry in config.tiers.iter().flatten() {
                ensure!(
                    matches!(
                        entry.provider.to_ascii_lowercase().as_str(),
                        "local"
                            | "custom"
                            | "openai"
                            | "google"
                            | "gemini"
                            | "groq"
                            | "openrouter"
                            | "mistral"
                            | "github"
                            | "nvidia"
                    ),
                    "Unsupported provider in refinement configuration"
                );
                ensure!(
                    !entry.model.trim().is_empty(),
                    "Every endpoint must specify a model"
                );
                ensure!(
                    provider_allows_missing_key(&entry.provider)
                        || entry
                            .api_key
                            .as_deref()
                            .is_some_and(|key| !key.trim().is_empty()),
                    "A hosted endpoint is missing its API key"
                );
            }
            let mut pool = build_pool(&config.tiers).map_err(anyhow::Error::msg)?;
            for tier in &mut pool {
                for entry in tier {
                    entry.translator = entry.translator.clone().with_diagnostics(|event| {
                        eprintln!(
                            "Provider HTTP {}: retry {}/{} in {} ms",
                            event.status, event.attempt, event.max_retries, event.retry_in_ms
                        );
                    });
                }
            }
            let updates = Arc::new(Mutex::new(Vec::<RefineUpdate>::new()));
            let pending_updates = updates.clone();
            let token = CancellationToken::new();
            let refinement = refine_cards_tiered(
                cards,
                RefineRunConfig {
                    prompt: config.prompt,
                    batch_mode: config.batch_mode,
                    batch_size: config.batch_size,
                },
                pool,
                move |event| match event {
                    RefineEvent::CardDone {
                        id,
                        notes,
                        done,
                        total,
                    } => {
                        pending_updates
                            .lock()
                            .expect("annotation update lock poisoned")
                            .push(RefineUpdate { id, notes });
                        eprintln!("Annotated {done}/{total}");
                    }
                    RefineEvent::CardFailed { id, error } => {
                        eprintln!("Annotation failed for card {id}: {error}")
                    }
                    RefineEvent::Info { .. } => {}
                },
                token.clone(),
            );
            tokio::pin!(refinement);
            let result = tokio::select! {
                biased;
                signal = tokio::signal::ctrl_c() => {
                    token.cancel();
                    let result = refinement.await;
                    signal.context("Cannot listen for Ctrl+C")?;
                    eprintln!("Refinement cancelled");
                    result
                }
                result = &mut refinement => result,
            };
            let summary = result.map_err(anyhow::Error::msg)?;
            let complete = summary.failed == 0 && summary.remaining == 0 && !summary.pool_exhausted;
            let cancelled = summary.cancelled || token.is_cancelled();
            let publish = !cancelled && summary.done > 0 && (complete || allow_partial);
            if publish {
                let updates = std::mem::take(
                    &mut *updates
                        .lock()
                        .map_err(|_| anyhow::anyhow!("Annotation update lock poisoned"))?,
                );
                save_cards(input_text, output_text, updates)
                    .map_err(anyhow::Error::msg)
                    .with_context(|| format!("Failed to save {}", output.display()))?;
                eprintln!("Saved annotations to {}", output.display());
            }
            if !complete && !cancelled {
                eprintln!(
                    "Refinement incomplete: {} failed, {} remaining, pool exhausted: {}; output saved: {publish}",
                    summary.failed, summary.remaining, summary.pool_exhausted
                );
            }
            println!(
                "{}",
                serde_json::json!({"summary": summary, "output": publish.then_some(output_text)})
            );
            Ok(if cancelled {
                130
            } else if complete {
                0
            } else {
                1
            })
        }
    }
}

async fn run_async(args: Vec<std::ffi::OsString>) -> ExitCode {
    match run(args).await {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("srt-refine: {error:#}");
            ExitCode::FAILURE
        }
    }
}
