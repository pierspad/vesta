# vesta-flashcards (`vesta-flashcards-cli`)

Headless command-line interface for the [`vesta-flashcards`](../../lib/vesta-flashcards)
engine. Generates Anki flashcard decks (`tsv` + media folder or `.apkg`) from
subtitle and media files without a GUI dependency.

```bash
cargo build --release -p vesta-flashcards-cli
# → target/release/vesta-flashcards   (requires ffmpeg/ffprobe on PATH for media)
```

## Usage

```bash
# Inspect a subtitle file
vesta-flashcards info Detour-en.srt

# Dry-run the pipeline (counts only, no media, --output not required)
vesta-flashcards preview --target Detour-en.srt --native Detour-it.srt

# Dry-run with paginated JSON rows (for programmatic previewing)
vesta-flashcards preview --target Detour-en.srt --native Detour-it.srt --json --offset 0 --limit 50

# Generate a TSV deck + media folder
vesta-flashcards generate \
  --target Detour-en.srt --native Detour-it.srt \
  --video "Detour(1945).mp4" --output out --format tsv --deck Detour

# Generate a self-contained .apkg with seven requested media workers
vesta-flashcards generate -t Detour-en.srt -n Detour-it.srt \
  -v "Detour(1945).mp4" -o out -f apkg -d Detour -j 7

# Batch generate multiple episodes from a JSON manifest (optionally merging .apkg outputs)
vesta-flashcards batch --manifest series.json

# Export bilingual dueling subtitles (.ass or .srt)
vesta-flashcards dueling -t Detour-en.srt --native Detour-it.srt \
  --dueling-format ass -o Detour.dueling.ass

# Merge fresh .apkg packages without review history
vesta-flashcards merge Ep01.apkg Ep02.apkg --output Detour.series.apkg
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
| `--note-type` | Override the Anki note type name (`.apkg` exports). |
| `--target-language` | Target ISO language code for card font selection and dictionary hints. |
| `--no-dictionary-lookup` | Disable clickable word lookup links in `.apkg` card templates. |
| `--no-auto-card-font` / `--no-embed-card-font` | Disable automatic language fonts or bundling fonts inside `.apkg`. |
| `-j, --jobs` | Parallel ffmpeg workers (clamped to `[1, available cores]`). |
| `--pad-audio-start` / `--pad-audio-end` | Pad audio clip start/end in ms (negative values trim). |
| `--pad-video-start` / `--pad-video-end` | Pad video clip start/end in ms (negative values trim). |
| `--video-hw-accel` | Hardware acceleration mode for H.264 clips (`auto` or `off`). |
| `--audio-gain` | Audio gain adjustment in dB (`-20` to `+20`). |
| `--normalize-audio` | Normalize audio loudness via EBU R128 (`loudnorm`). |
| `--no-optimize` | Disable automatic clip preparation and nearby snapshot batching. |
| `--no-audio` / `--no-snapshots` / `--no-video` | Skip a media type. |
| `--span-start` / `--span-end` | Restrict to a time window (ms). |
| `--min-chars` / `--exclude-words` / … | Subtitle filters (see `--help`). |
| `--ffmpeg` / `--ffprobe` | Override the binaries to invoke. |
| `--json` / `--offset` / `--limit` | `preview` only: emit JSON summary counts and paginated `PreviewLine` rows. |
| `--manifest` | `batch` only: path to JSON manifest (`output_dir`, optional `merge_output`, `defaults`, `episodes`). |

Run `vesta-flashcards generate --help` for the full list of filters, context lines,
codecs, and bitrates. Custom HTML/CSS note templates and font downloads
are configured through the library or GUI; see the
[coverage matrix](../../docs/CLI.md#feature-coverage-by-interface).

## Batch manifest schema (`batch --manifest`)

```json
{
  "output_dir": "./out",
  "merge_output": "./out/Series.apkg",
  "defaults": {
    "format": "apkg",
    "deck": "Series",
    "no_audio": false,
    "no_snapshots": false,
    "no_video": true,
    "jobs": 4,
    "no_optimize": false
  },
  "episodes": [
    {
      "target": "ep01-ja.srt",
      "native": "ep01-en.srt",
      "video": "ep01.mkv",
      "deck": "Series Ep01",
      "episode": 1
    },
    {
      "target": "ep02-ja.srt",
      "native": "ep02-en.srt",
      "video": "ep02.mkv",
      "deck": "Series Ep02",
      "episode": 2
    }
  ]
}
```

`output_dir` must be non-empty and `episodes` must contain at least one item. Each
episode entry supports `target` (required) and optional overrides (`native`, `video`,
`audio`, `deck`, `episode`, `audio_track`, `crop_bottom`, `shift_target`,
`shift_native`). When `merge_output` is set, `defaults.format` must be `apkg`: two
or more generated `.apkg` files are merged via `merge_apkg`, and a single-episode
batch copies its `.apkg` to `merge_output`.

## Script contract

`info` and `preview` print human-readable summaries to stdout (`preview --json`
emits `{ "total", "active", "matched", "offset", "limit", "rows" }`); `preview`
never creates the output directory. Successful generation (`generate`, `batch`,
`merge`, `dueling`) prints its summary and output paths to stdout. Progress and
errors use stderr; `--quiet` suppresses progress, but keeps the success summary and
failures visible.

Exit codes are `0` for success, `1` for input/processing errors, `2` for invalid
CLI arguments, and `130` when generation cooperatively returns an unsuccessful
result after Ctrl+C. An export successfully published during a late cancellation
still exits `0`. Snapshot quality must be an integer from `0` through `100`.

Run the offline subprocess regressions with `cargo test -p vesta-flashcards-cli`.
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
