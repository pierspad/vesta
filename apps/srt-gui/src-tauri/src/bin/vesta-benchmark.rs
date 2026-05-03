use std::process::Command;
use std::time::Instant;

use vesta_lib::commands::flashcards::{
    flashcard_generate_headless, ContextConfig, FlashcardConfig, OutputFields, SubtitleFilters,
};

fn has_audio(video_path: &str) -> bool {
    Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type",
            "-of",
            "csv=p=0",
            video_path,
        ])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).contains("audio"))
        .unwrap_or(false)
}

fn build_config(
    target_subs_path: String,
    native_subs_path: String,
    video_path: String,
    output_dir: String,
    export_format: String,
) -> FlashcardConfig {
    let has_audio = has_audio(&video_path);
    let cpu_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .saturating_sub(1)
        .max(2);

    FlashcardConfig {
        target_subs_path,
        native_subs_path: Some(native_subs_path),
        video_path: Some(video_path.clone()),
        audio_path: if has_audio { Some(video_path) } else { None },
        output_dir,
        use_timings_from: "target".to_string(),
        span_start_ms: None,
        span_end_ms: None,
        time_shift_target_ms: 0,
        time_shift_native_ms: 0,
        filters: SubtitleFilters {
            include_words: None,
            exclude_words: None,
            exclude_duplicates_subs1: false,
            exclude_duplicates_subs2: false,
            min_chars: None,
            max_chars: None,
            min_duration_ms: None,
            max_duration_ms: None,
            exclude_styled: false,
            actor_filter: None,
            only_cjk: false,
            remove_no_match: false,
        },
        context: ContextConfig {
            leading: 0,
            trailing: 0,
            max_gap_seconds: 0.0,
        },
        combine_sentences: false,
        continuation_chars: String::new(),
        generate_audio: has_audio,
        audio_bitrate: 128,
        audio_track_index: None,
        normalize_audio: false,
        audio_pad_start_ms: 0,
        audio_pad_end_ms: 0,
        generate_snapshots: true,
        snapshot_width: 240,
        snapshot_height: 160,
        crop_bottom: 0,
        generate_video_clips: true,
        video_codec: "h264".to_string(),
        h264_preset: "ultrafast".to_string(),
        video_bitrate: 1000,
        video_audio_bitrate: 128,
        video_pad_start_ms: 0,
        video_pad_end_ms: 0,
        deck_name: "BenchmarkDeck".to_string(),
        episode_number: 1,
        export_format: Some(export_format),
        note_type_name: None,
        output_fields: OutputFields {
            include_tag: true,
            include_sequence: true,
            include_audio: true,
            include_snapshot: true,
            include_video: true,
            include_subs1: true,
            include_subs2: true,
        },
        cpu_cores: Some(cpu_cores),
        card_front_html: None,
        card_back_html: None,
        card_css: None,
    }
}

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
    let config = build_config(
        args[1].clone(),
        args[2].clone(),
        args[3].clone(),
        args[4].clone(),
        export_format,
    );

    let start = Instant::now();
    match flashcard_generate_headless(config).await {
        Ok(result) if result.success => {
            println!(
                "VESTA_BENCHMARK_SUCCESS: {} ms",
                start.elapsed().as_millis()
            );
            println!(
                "VESTA_BENCHMARK_CARDS: {} audio={} snapshots={} video={}",
                result.cards_generated, result.audio_clips, result.snapshots, result.video_clips
            );
        }
        Ok(result) => {
            eprintln!("VESTA_BENCHMARK_ERROR: {}", result.message);
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("VESTA_BENCHMARK_ERROR: {}", error);
            std::process::exit(1);
        }
    }
}
