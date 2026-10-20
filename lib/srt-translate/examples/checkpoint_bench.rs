use std::collections::HashMap;
use std::hint::black_box;
use std::time::{Duration, Instant};

use srt_parser::{SrtParser, Subtitle, Timestamp};

fn subtitles(count: usize) -> HashMap<u32, Subtitle> {
    (1..=count)
        .map(|id| {
            let start_ms = id as u64 * 2_000;
            (
                id as u32,
                Subtitle {
                    id: id as u32,
                    start: Timestamp {
                        milliseconds: start_ms,
                    },
                    end: Timestamp {
                        milliseconds: start_ms + 1_500,
                    },
                    text: format!(
                        "Checkpoint benchmark subtitle {id}: a representative line of text."
                    ),
                },
            )
        })
        .collect()
}

fn measure(path: &std::path::Path, data: &HashMap<u32, Subtitle>, repetitions: usize) -> Duration {
    let started = Instant::now();
    for _ in 0..repetitions {
        SrtParser::save_file(path, black_box(data)).expect("save checkpoint SRT");
    }
    started.elapsed()
}

fn main() {
    // An explicit directory measures the filesystem used for real subtitle files.
    // The system temporary directory may be a RAM-backed filesystem.
    let directory = match std::env::args_os().nth(1) {
        Some(parent) => tempfile::tempdir_in(parent),
        None => tempfile::tempdir(),
    }
    .expect("create benchmark directory");
    let path = directory.path().join("checkpoint.srt");

    println!("SrtParser::save_file checkpoint cost (includes actual filesystem writes)");
    println!("subtitles,repetitions,total_ms,mean_ms_per_save,file_bytes");
    for (count, repetitions) in [(100, 20), (1_000, 20), (10_000, 10), (50_000, 10)] {
        let data = subtitles(count);
        SrtParser::save_file(&path, &data).expect("warm up checkpoint write");
        let elapsed = measure(&path, &data, repetitions);
        let bytes = std::fs::metadata(&path).expect("stat checkpoint").len();
        println!(
            "{count},{repetitions},{:.3},{:.3},{bytes}",
            elapsed.as_secs_f64() * 1_000.0,
            elapsed.as_secs_f64() * 1_000.0 / repetitions as f64,
        );
    }
}
