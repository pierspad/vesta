# srt-flashcards — subs2srs-style Anki deck generation

`lib/srt-flashcards` is the core of Vesta: a headless engine that turns
subtitle pairs + a media file into an Anki deck — audio snippets, snapshots
and video clips per card, exported as TSV + media folder or a self-contained
`.apkg`. It is the modern, parallel replacement for the subs2srs pipeline
(media extraction runs on a semaphore-bounded worker pool, saturating
`cores-1` FFmpeg processes).

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

- `FlashcardConfig` — the one big config struct (inputs, filters, media
  options, export options)
- `generate(config, tools, on_progress, cancel_token)` — full run;
  `MediaTools::new("ffmpeg", "ffprobe")` tells it which binaries to use
- `preview(config)` — parse/match/filter pipeline without touching media
- `build_matched_lines(config)` — the shared parse→match→filter pipeline
- `load_sub_file_info(path)`, `list_audio_tracks(...)`, `check_ffmpeg(...)`

Errors are user-presentable `String`s; progress is a callback; cancellation a
`CancellationToken`.

## Use as a binary

```bash
cargo build --release -p srt-flashcards-cli

./target/release/srt-flashcards generate \
  --target movie-en.srt --native movie-it.srt \
  --video movie.mp4 --output out --format apkg --deck "Detour"

./target/release/srt-flashcards info movie-en.srt
./target/release/srt-flashcards preview --target movie-en.srt --output out
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

## Extract it standalone

Copy `lib/srt-flashcards/` (self-contained subtitle parsing — it does not
depend on `srt-parser`). External deps: `rusqlite` (bundled SQLite), `zip`,
`sha1_smol`, `tokio`, `tokio-util`, `serde`, `serde_json`, `tempfile`.
FFmpeg/ffprobe are runtime requirements passed in via `MediaTools`.

The benchmark harness in [`benchmarking_against_subs2srs/`](../../benchmarking_against_subs2srs) uses exactly this
crate through the CLI, pitted against the original subs2srs code.
