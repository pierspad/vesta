use anyhow::{Context, Result, bail};
use clap::Parser;
use srt_sync::SyncEngine;
use std::{collections::HashSet, path::PathBuf};

#[derive(Clone, Debug)]
struct Anchor {
    id: u32,
    corrected_ms: i64,
}

fn parse_anchor(value: &str) -> Result<Anchor, String> {
    let (id, time) = value
        .split_once(':')
        .ok_or_else(|| "Expected SUBTITLE_ID:CORRECTED_MS".to_string())?;
    let id = id
        .parse::<u32>()
        .map_err(|_| "Subtitle ID must be a positive integer")?;
    let corrected_ms = time
        .parse::<i64>()
        .map_err(|_| "Corrected time must be a non-negative integer in milliseconds")?;
    if id == 0 || corrected_ms < 0 {
        return Err("Subtitle ID must be positive and corrected time non-negative".into());
    }
    Ok(Anchor { id, corrected_ms })
}

#[derive(Parser)]
#[command(
    name = "srt-sync",
    about = "Retime SRT subtitles with manual anchors",
    version
)]
struct Cli {
    #[arg(short, long)]
    input: PathBuf,
    #[arg(short, long)]
    output: PathBuf,
    /// Subtitle ID and corrected start time in milliseconds; repeat for drift correction
    #[arg(long = "anchor", required = true, value_parser = parse_anchor)]
    anchors: Vec<Anchor>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut engine = SyncEngine::new(&cli.input)
        .with_context(|| format!("Failed to load subtitles {}", cli.input.display()))?;
    let mut seen = HashSet::new();
    // Validate every anchor before mutating the mapper or writing any output.
    let mut timed = Vec::with_capacity(cli.anchors.len());
    for anchor in &cli.anchors {
        if !seen.insert(anchor.id) {
            bail!("Duplicate anchor for subtitle {}", anchor.id);
        }
        let subtitle = engine.get_subtitle(anchor.id).with_context(|| {
            format!(
                "Subtitle {} not found in {}",
                anchor.id,
                cli.input.display()
            )
        })?;
        let original = i64::try_from(subtitle.start.total_milliseconds())
            .context("Subtitle timestamp exceeds supported range")?;
        timed.push((original, anchor));
    }
    timed.sort_by_key(|(original, _)| *original);
    for pair in timed.windows(2) {
        if pair[0].0 == pair[1].0 || pair[0].1.corrected_ms >= pair[1].1.corrected_ms {
            bail!("Anchors must have distinct original times and increasing corrected times");
        }
    }
    for (_, anchor) in timed {
        engine.add_anchor(anchor.id, anchor.corrected_ms, true)?;
    }
    engine.save_synced_file(&cli.output).with_context(|| {
        format!(
            "Failed to save synchronized subtitles {}",
            cli.output.display()
        )
    })?;
    eprintln!(
        "Saved {} subtitles to {}",
        engine.total_subtitles(),
        cli.output.display()
    );
    Ok(())
}
