# srt-flashcards (CLI)

Headless command-line interface for the [`srt-flashcards`](../../lib/srt-flashcards)
engine. Generates Anki flashcard decks (`tsv` + media folder or `.apkg`) from
subtitle and media files without a GUI dependency.

```bash
cargo build --release -p srt-flashcards-cli
# → target/release/srt-flashcards   (requires ffmpeg/ffprobe on PATH for media)
```

## Usage

```bash
# Inspect a subtitle file
srt-flashcards info Detour-en.srt

# Dry-run the pipeline (counts only, no media, --output not required)
srt-flashcards preview --target Detour-en.srt --native Detour-it.srt

# Generate a TSV deck + media folder
srt-flashcards generate \
  --target Detour-en.srt --native Detour-it.srt \
  --video "Detour(1945).mp4" --output out --format tsv --deck Detour

# Generate a self-contained .apkg with seven requested media workers
srt-flashcards generate -t Detour-en.srt -n Detour-it.srt \
  -v "Detour(1945).mp4" -o out -f apkg -d Detour -j 7
```

## Key options

| Flag | Meaning |
|---|---|
| `-t, --target` | Target-language subtitles (the language being studied). Required. |
| `-n, --native` | Native-language subtitles; enables time-overlap matching. |
| `-v, --video` | Video file (snapshots, video clips, and audio by default). |
| `--audio` | Separate audio source (defaults to `--video`). |
| `-o, --output` | Output directory. Required for `generate`, optional for `preview`. |
| `-f, --format` | `tsv` (default) or `apkg`. |
| `-j, --jobs` | Parallel ffmpeg workers (clamped to `[1, available cores]`). |
| `--pad-audio-start` / `--pad-audio-end` | Pad audio clip start/end in ms (negative values trim). |
| `--audio-gain` | Audio gain adjustment in dB (`-20` to `+20`). |
| `--normalize-audio` | Normalize audio loudness via EBU R128 (`loudnorm`). |
| `--no-optimize` | Disable automatic clip preparation and nearby snapshot batching. |
| `--no-audio` / `--no-snapshots` / `--no-video` | Skip a media type. |
| `--span-start` / `--span-end` | Restrict to a time window (ms). |
| `--min-chars` / `--exclude-words` / … | Subtitle filters (see `--help`). |
| `--ffmpeg` / `--ffprobe` | Override the binaries to invoke. |

Run `srt-flashcards generate --help` for the full list of filters, context lines,
codecs, and bitrates. Video padding, custom note templates, and font management
are configured through the library or GUI; see the
[coverage matrix](../../docs/CLI.md#coverage-shared-engines-different-interfaces).

## Script contract

`info` and `preview` print human-readable summaries to stdout; `preview` never
creates the output directory. Successful generation prints its summary and output
paths to stdout. Progress and errors use stderr; `--quiet` suppresses progress,
but keeps the success summary and failures visible. These commands do not expose
a JSON protocol.

Exit codes are `0` for success, `1` for input/processing errors, `2` for invalid
CLI arguments, and `130` when generation cooperatively returns an unsuccessful
result after Ctrl+C. An export successfully published during a late cancellation
still exits `0`. Snapshot quality must be an integer from `0` through `100`.

Run the offline subprocess regressions with `cargo test -p srt-flashcards-cli`.
The SIGINT and simulated media-process cases run on Unix hosts.

Part of the [Vesta](../../README.md) workspace. Licensed GPL-3.0-only.

## Snapshot batching and video optimization

When `optimize_video` is enabled (the default unless `--no-optimize` is passed),
the engine batches up to eight nearby WebP snapshot requests within an 8-second
window for eligible CFR 720p/1080p H.264/HEVC streams, using FFprobe packet PTS
with automatic fallback to individual seeks. See
[`docs/ARCHITECTURE.md`](../../docs/ARCHITECTURE.md#native-snapshot-extraction-and-verification)
and [`docs/BENCHMARK_NATIVE.md`](../../docs/BENCHMARK_NATIVE.md) for eligibility
rules and A/B equivalence measurements.

## Regenerating TSV exports

Generation stages new media beside the destination before replacing
`<deck>.media`. Extraction failure or cancellation before publication preserves
the previous TSV and media. If writing the new TSV fails, the publisher restores
the previous media. Publishing the same deck concurrently is rejected while its
OS lock is held. The hidden `.<deck>.tsv.export.lock` file remains for lock
identity; do not remove it while exports may be running.

Staging needs disk space for both old and new media. Publication is synchronous:
cancellation arriving after the final check can still complete successfully.
A crash between the media swap and TSV replacement is not recovered
automatically. If an interrupted export leaves `.vesta-export-*` directories,
inspect `previous.media` there before deleting them. With no export running,
retain the current media as a backup and restore `previous.media` to
`<deck>.media` when the TSV still belongs to that older generation. A failed
rollback reports the staging path explicitly. APKG publication remains a
single-file atomic replacement.
