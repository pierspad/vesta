# srt-flashcards

The headless engine behind Vesta's flashcard feature: turn a subtitle file
(optionally a *target* + *native* pair) plus a media file into an Anki deck —
either a subs2srs-style **TSV + media folder** or a self-contained **`.apkg`**.

This crate is deliberately **GUI-agnostic** (no Tauri, no Svelte): progress is
reported through a plain callback and work is cancelled through a
`CancellationToken`. The same code powers the Vesta desktop app, the
`srt-flashcards` CLI, and the benchmark harness.

## Pipeline

```text
parse (SRT/ASS/VTT) → time-shift → match (dual subs) → span → filter
                     → combine sentences → context lines
                     → parallel ffmpeg media extraction → TSV / APKG export
```

Media extraction runs ffmpeg across up to `cores-1` workers via a streaming
semaphore (no batch barriers), so the slowest clip never stalls the rest.

## Public API

| Function | Purpose |
|---|---|
| `generate(config, tools, cancel, &progress)` | Full run: pipeline + media + export. |
| `preview(&config)` | Parse/match/filter only; returns every line (no media). |
| `build_matched_lines(&config)` | The shared deterministic pipeline (used by both of the above). |
| `load_sub_file_info(path)` | Summarise a subtitle file (count, format, actors, duration). |
| `list_audio_tracks(path, ffprobe)` | Probe a media file's audio streams. |
| `check_ffmpeg(cmd)` | Verify an ffmpeg executable is runnable. |

`MediaTools` selects the ffmpeg/ffprobe executables (PATH by default, or absolute
paths to a bundled build). The progress callback receives `FlashcardProgressEvent`s.

## Example

```rust
use srt_flashcards::{generate, FlashcardConfig, MediaTools};
use tokio_util::sync::CancellationToken;

# async fn run(config: FlashcardConfig) -> Result<(), String> {
let result = generate(
    config,
    MediaTools::default(),                       // resolve ffmpeg/ffprobe from PATH
    CancellationToken::new(),
    &|p| eprintln!("[{:>3.0}%] {}", p.percentage, p.stage),
).await?;
println!("Generated {} cards", result.cards_generated);
# Ok(())
# }
```

## Dependencies

Pure Rust + ffmpeg-as-a-subprocess. APKG packaging uses `rusqlite` (bundled
SQLite), `zip`, and `sha1_smol`. No GUI toolkit, no whisper, no network.

## Where it's used

* `cli/srt-flashcards-cli` — the headless command-line front-end.
* `apps/srt-gui/src-tauri` — the desktop app (thin Tauri command wrappers).

Part of the [Vesta](../../README.md) workspace. Licensed GPL-3.0-only.
