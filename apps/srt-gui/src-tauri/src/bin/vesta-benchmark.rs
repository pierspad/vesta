use std::time::Instant;

use srt_flashcards::{FlashcardConfig, MediaTools, generate, video_has_audio};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 6 {
        eprintln!(
            "Usage: {} <target.srt> <native.srt> <video> <output-dir> <tsv|apkg>",
            args.first()
                .map(String::as_str)
                .unwrap_or("vesta-benchmark")
        );
        std::process::exit(2);
    }

    let export_format = args.get(5).cloned().unwrap_or_else(|| "tsv".to_string());
    let cpu_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .saturating_sub(1)
        .max(2);
    let config = FlashcardConfig::benchmark(
        args[1].clone(),
        args[2].clone(),
        args[3].clone(),
        args[4].clone(),
        export_format,
        video_has_audio("ffprobe", &args[3]),
        Some(cpu_cores),
    );

    let start = Instant::now();
    match generate(
        config,
        MediaTools::default(),
        CancellationToken::new(),
        &|_| {},
    )
    .await
    {
        Ok(result) if result.success => {
            println!(
                "vesta_BENCHMARK_SUCCESS: {} ms",
                start.elapsed().as_millis()
            );
            println!(
                "vesta_BENCHMARK_CARDS: {} audio={} snapshots={} video={}",
                result.cards_generated, result.audio_clips, result.snapshots, result.video_clips
            );
        }
        Ok(result) => {
            eprintln!("vesta_BENCHMARK_ERROR: {}", result.message);
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("vesta_BENCHMARK_ERROR: {}", error);
            std::process::exit(1);
        }
    }
}
