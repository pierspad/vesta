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

Media extraction runs ffmpeg across a bounded number of workers via a streaming
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

Audio/snapshot-only exports extract from the original source without full-film
H.264 preparation. Video-clip workflows can prepare a heavy source, reporting
actual FFmpeg timestamp progress and honoring cancellation. Preparation is a
throughput tradeoff, not a quality enhancement.

`merge_apkg` merges fresh Vesta exports with remapped note/card IDs and validated
media/metadata; it is not a merger for scheduled Anki collections. Single-series
GUI output keeps per-episode packages in separate directories before merging.
See [QUALITY](../../docs/QUALITY.md) for a short synthetic media smoke benchmark.

## Native nearby snapshot batching

The shared Rust engine batches up to eight WebP snapshots within eight seconds
when `optimize_video` is enabled. Eligibility is conservative: CFR H.264/HEVC,
width 1280–1920, height ≤1080 and an unambiguous video stream. FFprobe packet
PTS guide frame selection; reordered keyframe boundaries and uncertain timings
retain the original individual-seek behavior. Unknown/VFR sources, other image
formats and isolated requests use individual extraction. Failed batch attempts
retry the original commands. There is no persistent cache or Python dependency
in the app; Python is used only by the benchmark harness.

Batch operations share the media semaphore with audio/video work, retain one
progress result per card and stop their direct child processes on cancellation.
An extraction failure prevents export of a deck with dangling media references.
The default worker count is roughly three quarters of logical cores, reserving
one core when possible; explicit counts are clamped to available logical cores.
Video-clip preparation can produce a lossy intermediate and is separate from
the original-source snapshot A/B quality guarantee.

See [native benchmark results](../../docs/BENCHMARK_NATIVE.md).
