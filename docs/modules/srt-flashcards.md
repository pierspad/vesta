# srt-flashcards — subs2srs-style Anki deck generation

`lib/srt-flashcards` is the core of Vesta: a headless engine that turns
subtitle pairs + a media file into an Anki deck — audio snippets, snapshots
and video clips per card, exported as TSV + media folder or a self-contained
`.apkg`. It is the modern, parallel replacement for the subs2srs pipeline
(media extraction runs on a semaphore-bounded worker pool, saturating
the configured number of FFmpeg processes).

**What's inside**

- multi-format subtitle parsing (SRT / ASS / VTT), target+native matching by
  time overlap, time shifting, span limits;
- card filters (length, duration, words, duplicates, CJK, no-match), context
  lines, sentence combining;
- media extraction via FFmpeg (audio bitrate/normalization, snapshot size and
  cropping, video codec/preset/bitrate, audio track selection);
- export to TSV or APKG (SQLite deck built from scratch), naming templates,
  field configuration.

**Library API highlights**

- `FlashcardConfig` — configuration struct covering inputs, filters, context lines, media options, font/dictionary settings, and export options (`Default` and `FlashcardConfig::benchmark(...)`)
- `generate(config, tools, cancel_token, &on_progress)` — full pipeline run; `MediaTools::new("ffmpeg", "ffprobe")` specifies the media executables
- `preview(config)` — parse/match/filter pipeline without extracting media
- `build_matched_lines(config)` — deterministic parse → match → filter pipeline
- `merge_apkg(paths, output)` — combines fresh Vesta `.apkg` exports into a single multi-episode package
- `load_sub_file_info(path)`, `parse_subtitle_file(path)` — multi-format subtitle inspection and parsing (`SRT`, `ASS`/`SSA`, `VTT`, `LRC`)
- `media` module — `list_audio_tracks`, `check_ffmpeg`, `detect_h264_encoder`, `optimize_video_source_with_progress`, `extract_preview_audio_clip`, `extract_preview_snapshot`
- `fonts` module — language-aware Google Noto font catalog (`FONT_CATALOG`, `list_fonts`, `font_status`, `download_font`, `delete_font`, `font_stack_for`)

Errors are user-presentable `String`s; progress is a callback; cancellation a
`CancellationToken`.

## Use as a binary

```bash
cargo build --release -p srt-flashcards-cli

./target/release/srt-flashcards generate \
  --target movie-en.srt --native movie-it.srt \
  --video movie.mp4 --output out --format apkg --deck "Detour"

./target/release/srt-flashcards info movie-en.srt
./target/release/srt-flashcards preview --target movie-en.srt --native movie-it.srt
```

Run `srt-flashcards generate --help` for the full option list (filters,
context, media parameters, `-j` parallelism…).

## Use as a Rust dependency

```toml
[dependencies]
srt-flashcards = { git = "https://github.com/pierspad/vesta" }
tokio          = { version = "1", features = ["full"] }
tokio-util     = "0.7"
```

```rust
use srt_flashcards::{generate, FlashcardConfig, MediaTools};
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), String> {
    let config = FlashcardConfig {
        target_subs_path: "movie-en.srt".into(),
        native_subs_path: Some("movie-it.srt".into()),
        video_path: Some("movie.mp4".into()),
        output_dir: "out".into(),
        export_format: Some("apkg".into()),
        deck_name: "Detour".into(),
        ..Default::default()
    };

    let result = generate(
        config,
        MediaTools::new("ffmpeg", "ffprobe"),
        CancellationToken::new(),
        &|ev| eprintln!("{:?}", ev),
    ).await?;

    println!("Deck written: {:?}", result);
    Ok(())
}
```

(Check `lib/srt-flashcards/src/types.rs` for the authoritative
`FlashcardConfig` fields — it implements `Default` — and
`cli/srt-flashcards-cli/src/main.rs` for a complete mapping.)

## Vendoring & Standalone Integration

Prefer the Cargo Git dependency above. When vendoring into another workspace,
include `lib/srt-flashcards/`, `core/srt-parser/`, and `core/srt-download/`,
and preserve workspace metadata and dependencies (`rusqlite` with bundled SQLite,
`zip`, `sha1_smol`, `tokio`, `tokio-util`, `serde`, `serde_json`, `tempfile`,
`reqwest`, `futures`, `anyhow`); see [`docs/library-integration.md`](../library-integration.md).
FFmpeg/ffprobe are runtime requirements passed in via `MediaTools`.

With `video_hw_accel = "auto"` (the default), the engine runs a real FFmpeg
encode probe and uses a working platform encoder for H.264/pre-transcoding.
Unsupported or failing hardware paths transparently use `libx264`. Audio
encoding, still-image extraction, matching, and packaging remain CPU work.

The benchmark harness in [`benchmarking_against_subs2srs/`](../../benchmarking_against_subs2srs) uses exactly this
crate through the CLI, pitted against the original subs2srs code.

## Preparation and series export

Audio/snapshot-only workflows use the original media directly. Optional full-film
preparation is reserved for video clips; it can trade initial latency and lossy
re-encoding for faster repeated seeks. The phase reports actual processed media
seconds separately from overall progress. See [architecture](../ARCHITECTURE.md).

`merge_apkg(paths, output)` combines fresh Vesta APKG exports, remapping note/card
IDs and merging compatible model/deck metadata and media. Conflicting metadata
or media names fail before replacing the destination. Packages with review
history are outside its intended input contract.


## TSV media and note types

TSV can contain both a snapshot and a video clip for the same card. They occupy
separate enabled fields: snapshot as `<img src="filename">`, video as
`[sound:filename.mp4]` (or the configured video extension). Audio has its own
`[sound:filename]` field. Turning generation off leaves an empty enabled field,
so later fields retain their column positions. Both media generation and the
corresponding output field must be enabled for a reference to be written.

The TSV stores references, not embedded media or card templates. Import into an
existing Anki note type, map the columns, enable HTML, and copy generated media
files into `collection.media` without subdirectories. See the
[Anki text import manual](https://docs.ankiweb.net/manual/importing/text-files).
Client codec support still determines whether a particular video plays.

## Snapshot batching and video optimization

When `optimize_video` is enabled (the default), `snapshot_batch` groups up to
eight nearby WebP snapshot requests within an 8-second window for eligible CFR
720p/1080p H.264/HEVC streams, using FFprobe packet PTS with fallback to
individual seeks. Batch operations share the worker semaphore with audio/video
tasks and terminate child processes on cancellation. See
[`docs/ARCHITECTURE.md`](../ARCHITECTURE.md#native-snapshot-extraction-and-verification)
and [`docs/BENCHMARK_NATIVE.md`](../BENCHMARK_NATIVE.md).
