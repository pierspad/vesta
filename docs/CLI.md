# Command-Line Reference

`vesta-cli` runs Vesta workflows without Tauri or desktop dependencies. The installed desktop binary also accepts `vesta <operation>` before launching the window (`vesta` with no arguments opens the GUI; `vesta-cli` with no arguments prints help). On Windows, use `vesta-cli.exe` for terminal output.

```sh
cargo install --locked --path cli/vesta-cli
vesta-cli --help
vesta-cli help flashcards generate
```

Each subcommand invokes the corresponding `srt-*` CLI parser and library engine directly; standalone `srt-*` binaries are not required at runtime. Delegated usage output may display the standalone `srt-*` command name. Standalone build paths and library APIs are listed in the [module index](modules/README.md).

CLI configuration is isolated from desktop preferences: commands do not read the GUI's stored API keys, provider tiers, AI Kill Switch, or note templates. Translation TOML files expand `${VAR_NAME}` from the environment or `.env`; refinement JSON files do not. For the stdio Model Context Protocol server, see [MCP.md](MCP.md).

## Feature Coverage by Interface

| Workflow | CLI Flags & Subcommands | GUI / Library Capabilities Not Exposed via CLI Flags |
|---|---|---|
| Flashcards | Subtitle info, count-only preview, single-episode TSV/APKG generation; filters, context lines, codecs, dimensions, audio padding/gain/normalization, audio track index, and worker count | Interactive row selection/editing and media previews; multi-episode orchestration and APKG merge; custom note templates/field mapping; explicit video padding, font/language/dictionary, and hardware-encoder settings |
| Extract | SRT reports (`json`, `stats`, `summary`, `debug`); embedded stream discovery (`--list-tracks`) and text-to-SRT track extraction (`--track`) | Embedded-track preview and existing-output discovery; bitmap subtitle tracks require external OCR |
| Sync | Apply subtitle-ID/time anchor pairs (`--anchor ID:MS`) to an SRT file | Interactive playback, anchor sampling, session files, undo/redo stack, and side-by-side editing |
| Auto-sync | Whisper sample analysis and automatic application of all suggested anchors | Interactive review or selective acceptance of suggested anchors |
| Transcribe | Single local or cloud endpoint per run; model list/download/removal; default Silero VAD, beam-search quality, segmentation, and JSON-Lines progress | Multi-tier provider failover, remote model discovery, and per-run custom VAD variant or model directory selection |
| Translate | Tiered TOML provider pools, batch size, resume overlap, language list, and completeness check | Context preview/editing, interactive subtitle table editing, and remote model discovery |
| Refine | JSON card inspection (`inspect`) and tiered LLM annotation (`enrich`) for TSV/APKG decks | Interactive card selection/editing, custom TSV column mapping, and GUI auto-run controls |
| AnkiConnect & Desktop | No dedicated CLI command | Direct AnkiConnect import (`srt-ankiconnect`), setup wizard, update installer, support logs, and local media playback server |

## Subcommands and Options

Timestamps, anchor targets, duration filters, and time shifts are expressed in milliseconds unless noted otherwise (`--max-gap` uses seconds). Pass negative numbers using `=` syntax (for example, `--shift-target=-500`).

### `flashcards` (`info`, `preview`, `generate`)

```sh
vesta-cli flashcards info episode-ja.srt
vesta-cli flashcards preview --target episode-ja.srt --native episode-en.srt
vesta-cli flashcards generate --target episode-ja.srt --native episode-en.srt \
  --video episode.mkv --output out --format apkg --deck Episode --no-video
# Text-only export:
vesta-cli flashcards generate --target episode-ja.srt --output out --deck Text \
  --no-audio --no-snapshots --no-video
```

- `preview` accepts the same filter and matching options as `generate`, does not require `--output`, writes no files, and prints card counts only.
- `generate` enables audio clips, snapshots, and video clips by default. Pass `--no-video`, `--no-snapshots`, or `--no-audio` to disable specific media types. If a media source (`--video` or `--audio`) is omitted, the corresponding media extraction is skipped.
- `--audio-track N` selects the zero-based audio stream (`0:a:N`), not the absolute container stream index reported by `extract --list-tracks`.
- `--jobs` clamps the number of concurrent FFmpeg worker processes to `[1, logical_cores]`.
- `--no-optimize` disables both video clip pre-transcoding and nearby WebP snapshot batching.
- Re-running `generate` with the same output directory and deck name atomically replaces the previous export after media extraction completes; see [`cli/srt-flashcards-cli/README.md`](../cli/srt-flashcards-cli/README.md#regenerating-tsv-exports).

### `extract`

```sh
vesta-cli extract --input episode.srt --format stats
vesta-cli extract --input episode.mkv --list-tracks
vesta-cli extract --input episode.mkv --track 8 --output episode.en.srt
```

- `--format` (`json`, `stats`, `summary`, `debug`) formats external `.srt` files.
- `--list-tracks` (requires `ffprobe`) prints embedded subtitle tracks as JSON with absolute stream indices.
- `--track <INDEX>` (requires `ffmpeg`, `ffprobe`, and `--output`) extracts the specified text subtitle stream to SRT. `--list-tracks` and `--track` are mutually exclusive.
- Output path must not resolve to the input file.

### `sync`

```sh
vesta-cli sync --input episode.srt --output episode.synced.srt --anchor 1:1500
vesta-cli sync --input episode.srt --output episode.synced.srt \
  --anchor 1:1500 --anchor 100:305000
```

- Each `--anchor SUBTITLE_ID:CORRECTED_MS` pairs a 1-based subtitle ID from the input SRT with its target start timestamp in milliseconds.
- A single anchor applies a constant offset; two or more anchors apply piecewise-linear drift interpolation.
- Anchor IDs must exist in the input file, be unique, and have strictly increasing original and corrected timestamps.

### `autosync`

```sh
vesta-cli autosync episode.srt episode.mkv --output episode.synced.srt --model base
vesta-cli autosync episode.srt episode.mkv --output episode.quick.srt --quick
```

- Positional arguments are `<SRT> <MEDIA>`. Requires `ffmpeg` and `ffprobe`.
- Downloads the requested local Whisper model on demand if not already cached.
- `--quick` reduces the number and duration of audio samples. All matched anchors are applied before writing `--output`.

### `transcribe` (`run`, `models`, `download`, `remove`)

```sh
vesta-cli transcribe models
vesta-cli transcribe download base
vesta-cli transcribe download vad
vesta-cli transcribe run episode.mkv --output episode.srt --model base --vad
vesta-cli transcribe run episode.mkv --output episode.srt --model base --json
vesta-cli transcribe remove vad
```

- `run <INPUT> --output <PATH>` transcribes audio or video to SRT. Local Whisper models download on demand.
- `download` and `remove` accept a Whisper model ID (`tiny`, `base`, `small`, `medium`, `large-v3`, `large-v3-turbo`), `"vad"` (default Silero VAD), or `"vad:<variant>"`.
- `--word-timestamps`, `--quality`, `--vad`, and `--no-gpu` apply only to `--provider local` and return an error if combined with cloud providers (`groq`, `openai`, `deepgram`, `assemblyai`, `custom`).
- For cloud providers, pass the provider's model identifier via `--model` and credentials via `--api-key` (`--api-url` is required for `custom`).
- When `--language auto` is used with local Whisper, the detected ISO code may be inserted before `.srt` in the output filename; check the reported output path.

### `translate`

```sh
vesta-cli translate --language-list
vesta-cli translate --input episode.srt --language it --config translation.toml \
  --output episode.it.srt
vesta-cli translate --input episode.srt --check-missing episode.it.srt
```

- Translation requires `--input`, `--language`, and `--config` (defaults to `config.toml`; see [`cli/srt-translate-cli/README.md`](../cli/srt-translate-cli/README.md#configuration)).
- `--language-list` and `--check-missing <TRANSLATED_FILE>` run locally without reading `config.toml` or contacting providers.
- If the output file already exists, completed cues are preserved and translation resumes from the first missing cue (with `resume_overlap` context). Output must not resolve to the input path.

### `refine` (`inspect`, `enrich`)

```sh
vesta-cli refine inspect --input deck.apkg
vesta-cli refine enrich --input deck.apkg --output deck.annotated.apkg --config refine.json
```

- `inspect` prints deck cards as JSON to stdout without network requests.
- `enrich` annotates cards using a JSON configuration file (`prompt`, `tiers`, optional `batch_mode` and `batch_size`; see [`cli/srt-refine-cli/README.md`](../cli/srt-refine-cli/README.md)).
- Supported input→output pairs: `TSV → TSV`, `APKG → TSV`, and `APKG → APKG`. In-place updates (`--input` equal to `--output`) are written atomically.
- By default, any failed or unannotated card aborts publication and exits `1`. `--allow-partial` writes completed card updates before exiting `1`.

## Standard Streams and Exit Codes

| Command | `stdout` | `stderr` |
|---|---|---|
| `flashcards` | Summary counts and output paths | Progress and errors (`-q, --quiet` hides progress) |
| `extract` | Formatted SRT report or `--list-tracks` JSON (when `-o` is omitted) | Status messages and errors |
| `sync` | Empty | Completion summary and errors |
| `autosync` | Alignment summary | Progress and errors (`--quiet` hides progress) |
| `transcribe` | Final output path, or JSON-Lines stream with `--json` | Progress and errors (`-q, --quiet` or `--json` hides progress) |
| `translate` | Progress, summary, or `--check-missing` report | Errors and cancellation notice |
| `refine` | `inspect` card JSON or `enrich` summary JSON | Progress and errors |

- `0`: Completed without error.
- `1`: Validation, I/O, or runtime processing error (including partial refinement with `--allow-partial`).
- `2`: Invalid CLI arguments (`clap` parse failure).
- `130`: Cooperative interruption via `SIGINT` (`Ctrl+C`). If `SIGINT` arrives after atomic output publication has started, the command may complete publication and exit `0`.
