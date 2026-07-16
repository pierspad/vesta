//! # srt-extract CLI
//!
//! Interfaccia a riga di comando per l'estrazione di dati da file SRT.
//! Questo è il "guscio" che gestisce l'I/O utente e delega la logica
//! di business alla libreria `srt-extract`.

use anyhow::Result;
use clap::Parser;
use srt_extract::{OutputFormat, calculate_stats, extract};
use srt_parser::SrtParser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "srt-extract")]
#[command(about = "Extract data from SRT files", long_about = None)]
struct Cli {
    /// Path to the SRT file to read
    #[arg(short, long)]
    input: PathBuf,

    /// Output format: json, debug, summary, stats
    #[arg(short, long, default_value = "debug")]
    format: String,

    /// Output file (optional, otherwise prints to stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Parse the SRT file
    println!("📖 Reading file: {:?}", cli.input);
    let subtitles = SrtParser::parse_file(&cli.input)?;
    println!("✅ Found {} subtitles", subtitles.len());

    // Generate output based on format
    let output = if cli.format == "stats" {
        // Special case: statistics
        let stats = calculate_stats(&subtitles);
        format!(
            "📊 Subtitle Statistics:\n\
            \n\
            Total subtitles: {}\n\
            Total duration: {:.2} seconds ({:.2} minutes)\n\
            Average duration: {:.2} seconds\n\
            \n\
            Text Statistics:\n\
            Shortest text: {} characters\n\
            Longest text: {} characters\n\
            Average text length: {:.2} characters\n",
            stats.total_count,
            stats.total_duration_seconds,
            stats.total_duration_seconds / 60.0,
            stats.average_duration_seconds,
            stats.shortest_text_length,
            stats.longest_text_length,
            stats.average_text_length
        )
    } else {
        // Use library for standard formats
        let format = OutputFormat::parse(&cli.format).unwrap_or(OutputFormat::Debug);
        extract(&subtitles, format)?
    };

    // Save or print
    if let Some(output_path) = cli.output {
        std::fs::write(&output_path, output)?;
        println!("💾 Output saved to: {:?}", output_path);
    } else {
        println!("\n{}", output);
    }

    Ok(())
}
