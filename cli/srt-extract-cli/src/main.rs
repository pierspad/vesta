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

    /// List embedded subtitle tracks in a media container as JSON
    #[arg(long, conflicts_with = "track")]
    list_tracks: bool,

    /// Extract the embedded subtitle stream with this absolute index (requires --output)
    #[arg(long, requires = "output")]
    track: Option<u32>,

    /// FFmpeg executable for embedded subtitle extraction
    #[arg(long, default_value = "ffmpeg")]
    ffmpeg: String,

    /// ffprobe executable for embedded subtitle discovery
    #[arg(long, default_value = "ffprobe")]
    ffprobe: String,

    /// Output format: json, debug, summary, stats
    #[arg(short, long, default_value = "debug")]
    format: String,

    /// Output file (optional, otherwise prints to stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.list_tracks {
        let tracks =
            srt_extract::embedded::list_embedded_subtitles(&cli.ffprobe, &cli.input).await?;
        let output = serde_json::to_string_pretty(&tracks)?;
        if let Some(path) = cli.output {
            std::fs::write(path, output)?;
        } else {
            println!("{output}");
        }
        return Ok(());
    }
    if let Some(index) = cli.track {
        srt_extract::embedded::extract_embedded_subtitle(
            &cli.ffmpeg,
            &cli.ffprobe,
            &cli.input,
            index,
            cli.output.as_deref().expect("clap requires output"),
        )
        .await?;
        return Ok(());
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_modes_require_unambiguous_arguments() {
        assert!(
            Cli::try_parse_from(["srt-extract", "--input", "film.mkv", "--track", "2"]).is_err()
        );
        assert!(
            Cli::try_parse_from([
                "srt-extract",
                "--input",
                "film.mkv",
                "--list-tracks",
                "--track",
                "2",
                "--output",
                "out.srt"
            ])
            .is_err()
        );
        let cli = Cli::try_parse_from([
            "srt-extract",
            "--input",
            "film.mkv",
            "--track",
            "2",
            "--output",
            "out.srt",
        ])
        .unwrap();
        assert_eq!(cli.track, Some(2));
        assert_eq!(cli.ffmpeg, "ffmpeg");
        assert!(
            Cli::try_parse_from(["srt-extract", "--input", "film.mkv", "--list-tracks"])
                .unwrap()
                .list_tracks
        );
    }
}
