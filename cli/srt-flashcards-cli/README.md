# srt-flashcards (CLI)

Headless command-line front-end for the [`srt-flashcards`](../../lib/srt-flashcards)
engine. Turn subtitles + a video into an Anki deck from the terminal — the exact
same engine the Vesta desktop app uses, with **no GUI dependency**.

Want only the flashcard feature? Clone the repo and build just this binary:

```bash
cargo build --release -p srt-flashcards-cli
# → target/release/srt-flashcards   (needs ffmpeg/ffprobe on PATH)
```

## Usage

```bash
# Inspect a subtitle file
srt-flashcards info Detour-en.srt

# Dry-run the pipeline (counts only, no media)
srt-flashcards preview --target Detour-en.srt --native Detour-it.srt --output out

# Generate a TSV deck + media folder
srt-flashcards generate \
  --target Detour-en.srt --native Detour-it.srt \
  --video "Detour(1945).mp4" --output out --format tsv --deck Detour

# Generate a self-contained .apkg, on all cores but one
srt-flashcards generate -t Detour-en.srt -n Detour-it.srt \
  -v "Detour(1945).mp4" -o out -f apkg -d Detour -j 7
```

## Key options

| Flag | Meaning |
|---|---|
| `-t, --target` | Target-language subtitles (the language you're learning). Required. |
| `-n, --native` | Native-language subtitles; enables time-overlap matching. |
| `-v, --video` | Video file (snapshots, video clips, and audio by default). |
| `-o, --output` | Output directory. Required. |
| `-f, --format` | `tsv` (default) or `apkg`. |
| `-j, --jobs` | Parallel ffmpeg workers (clamped to `[1, available cores]`; `1` = single ffmpeg at a time). |
| `--no-optimize` | Disable automatic clip preparation and nearby snapshot batching. |
| `--no-audio` / `--no-snapshots` / `--no-video` | Skip a media type. |
| `--span-start` / `--span-end` | Restrict to a time window (ms). |
| `--min-chars` / `--exclude-words` / … | Subtitle filters (see `--help`). |
| `--ffmpeg` / `--ffprobe` | Override the binaries to invoke. |

Run `srt-flashcards generate --help` for the full list (filters, padding,
context lines, codecs, bitrates, …).

Part of the [Vesta](../../README.md) workspace. Licensed GPL-3.0-only.

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
