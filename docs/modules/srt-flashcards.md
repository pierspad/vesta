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

Copy `lib/srt-flashcards/` + `core/srt-parser/`. External deps include
`rusqlite` (bundled SQLite), `zip`,
`sha1_smol`, `tokio`, `tokio-util`, `serde`, `serde_json`, `tempfile`.
FFmpeg/ffprobe are runtime requirements passed in via `MediaTools`.

With `video_hw_accel = "auto"` (the default), the engine runs a real FFmpeg
encode probe and uses a working platform encoder for H.264/pre-transcoding.
Unsupported or failing hardware paths transparently use `libx264`. Audio
encoding, still-image extraction, matching, and packaging remain CPU work.

The benchmark harness in [`benchmarking_against_subs2srs/`](../../benchmarking_against_subs2srs) uses exactly this
crate through the CLI, pitted against the original subs2srs code.

## Preparation and series export

Audio plus snapshots use the original media directly. Optional full-film
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

The desktop's APKG snapshot/video exclusivity is a UI choice, not a TSV or Anki
file-format restriction. TSV episode overrides permit both media kinds. Series
TSV output is always per episode; a saved single-APKG preference does not affect
TSV directories or trigger a package merge. The result exposes the first generated
TSV for opening, as separate episode files are not merged into one TSV.

The footer note picker shows the actual `_Vesta` name in a compact control.
Its automatic option remains explicit in the open menu; it chooses the note type
from the subtitle language with the configured fallback. Manual/custom note types
remain available. Selecting a note type determines export fields and, for APKG,
the model/templates; a TSV alone cannot install a new note type in Anki.
