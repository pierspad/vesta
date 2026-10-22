# Command-line workflows and coverage

Use `vesta-cli` for automation without Tauri or desktop libraries. The installed
GUI executable also accepts `vesta <operation>` before starting its window;
`vesta` with no arguments opens the GUI, while `vesta-cli` with no arguments
prints help. On Windows use `vesta-cli.exe` for terminal output.

```sh
cargo install --locked --path cli/vesta-cli
vesta-cli --help
vesta-cli help flashcards generate
```

The seven operations call the individual `srt-*` CLI implementations directly;
you do not need those executables installed alongside `vesta-cli`. For example,
`vesta-cli sync ...` and `srt-sync ...` use the same argument parser and engine.
Delegated usage messages may display the `srt-*` name. Standalone install/build
paths and engine APIs are listed in the [module index](modules/README.md).

CLI arguments/configuration are independent of desktop preferences: the command
does not import the GUI's API keys, tiers, AI Kill Switch or note-type settings.
Cloud commands can incur provider charges. Set their configuration explicitly;
keep files containing secrets out of version control. Translation TOML expands
`${VAR_NAME}` from the environment/`.env`; refinement JSON does not. MCP is a
separate optional executable; see [its scope and setup](MCP.md).

## Coverage: shared engines, different interfaces

The CLI covers all seven main processing workflows, **not every GUI feature or
public Rust API**. These are current adapter boundaries, not promises of parity:

| Area | CLI exposes | GUI/library capabilities not exposed by these commands |
|---|---|---|
| Flashcards | Subtitle info, count-only preview, single-episode TSV/APKG generation; filters, context, codecs, sizes, audio track and workers | Interactive row selection/editing and media previews; multi-episode orchestration/APKG merge; custom note templates/field mapping; explicit audio/video padding, gain/boost, font/language/dictionary and hardware-encoder preferences |
| Extract | SRT reports; embedded stream discovery and text-to-SRT extraction | Embedded-track preview and automatic existing-output suggestions; bitmap subtitles still need external OCR |
| Sync | Apply supplied subtitle-ID/time anchors to SRT | Interactive playback, anchor suggestions/sampling, sessions, undo and side-by-side revision/editing |
| Auto-sync | Whisper sample analysis followed by applying all suggested anchors | Review/accept individual suggestions interactively |
| Transcribe | One local/cloud endpoint per run; model download/list/removal; default Silero VAD, quality/segmentation and JSON progress | GUI tier orchestration/fallback, endpoint model discovery, choosing a VAD variant/custom model path per run |
| Translate | TOML provider tiers, batch size, resume overlap, language list and completeness checks | GUI context preview/editing, interactive subtitle editing and provider/model configuration/discovery |
| Refine | JSON inspection and tiered annotation of supported TSV/APKG inputs | Interactive manual card editing/selection, TSV column mapping and GUI batch/auto-run controls |
| AnkiConnect and desktop services | No dedicated command | Direct Anki integration (`srt-ankiconnect` library), setup/download UI, preferences, updates, support logs and media playback server |

For functionality beyond an adapter's flags, use the GUI or call the relevant
Rust library. A shared engine guarantees shared processing logic only for the
same effective inputs/options; GUI defaults and orchestration can differ.

## Commands and safe usage

Examples below use local filenames as placeholders. Run `--help` at the relevant
level for the full option list, defaults and additional examples. SRT times,
anchor corrections, filters and shifts use milliseconds unless help says
otherwise. Negative option values such as shifts use `--shift-target=-500`.

### `flashcards info`, `preview`, `generate`

```sh
vesta-cli flashcards info episode-ja.srt
vesta-cli flashcards preview --target episode-ja.srt --native episode-en.srt --output out
vesta-cli flashcards generate --target episode-ja.srt --native episode-en.srt \
  --video episode.mkv --output out --format apkg --deck Episode --no-video
# Text-only export: disable all three media types explicitly.
vesta-cli flashcards generate --target episode-ja.srt --output out --deck Text \
  --no-audio --no-snapshots --no-video
```

`preview` shares generation's options and requires `--output`, but writes
nothing and reports counts only. For preview rows use the GUI, library or MCP.
`generate` enables audio, snapshots **and video clips** by default. Use
`--no-video` for audio + snapshots; for an audio-only source supply `--audio`
and disable snapshots/video. Enabled audio needs an audio/video source; enabled
snapshots/video need a video source. The CLI/engine skips extraction when its
source is absent; it does not treat a missing source as a usage error. Use
explicit disable flags to make a text-only/audio-only intent clear. MCP instead
rejects enabled media without its required source. FFmpeg/FFprobe must be
installed for media.

`--audio-track N` uses the **audio-relative** `0:a:N` index, not the absolute
subtitle stream index reported by `extract --list-tracks`. `--jobs` is bounded
to logical cores; it controls concurrent media jobs, not CPU affinity. A media
source without the requested stream/codec can still fail at runtime.

`--no-optimize` disables both clip preparation and nearby snapshot batching.
Clip preparation may use a lossy intermediate. MP3 is the default audio format;
check your target Anki player's codec support before choosing Opus. Reusing a
deck/output name replaces the previous export after successful processing; see
[publication and recovery limits](../cli/srt-flashcards-cli/README.md#regenerating-tsv-exports).

### `extract`

This operation uses flags, not `list`/`run` subcommands:

```sh
vesta-cli extract --input episode.srt --format stats
vesta-cli extract --input episode.mkv --list-tracks
vesta-cli extract --input episode.mkv --track 8 --output episode.en.srt
```

Use the absolute subtitle stream index returned by `--list-tracks`, not its row
position or an audio-track index. Listing and extraction are mutually exclusive;
`--track` requires an output. SRT report formats are `json`, `stats`, `summary`
and `debug` (default). `--format` formats external SRT reports; embedded tracks
are converted to SRT. Bitmap tracks require OCR. FFmpeg/FFprobe are needed only
for embedded tracks. Output must differ from input, including filesystem aliases.

### `sync`

```sh
vesta-cli sync --input episode.srt --output episode.synced.srt --anchor 1:1500
vesta-cli sync --input episode.srt --output episode.synced.srt \
  --anchor 1:1500 --anchor 100:305000
```

An anchor is `SUBTITLE_ID:CORRECTED_START_MS`, not a row number or time delta.
Use IDs that exist in the SRT. One anchor shifts the track; multiple anchors
correct drift. IDs must be unique/positive and corrected times non-negative and
increasing with distinct original times. Invalid anchors are rejected before
saving. The shared engine may fill small ID gaps with `[...]` placeholders.
No FFmpeg/model is needed. A separate output path preserves your source.

### `autosync`

```sh
vesta-cli autosync episode.srt episode.mkv --output episode.synced.srt --model base
vesta-cli autosync episode.srt episode.mkv --output episode.quick.srt --quick
```

The two positional inputs are **SRT first, media second**. The local Whisper
model downloads on demand; FFmpeg/FFprobe are required. `--language` is an
optional spoken-language hint. `--quick` analyzes fewer/shorter samples and may
be less precise. All suggestions are applied automatically before writing;
use the GUI if you need to inspect suggestions individually. Successful output
replaces the destination; Ctrl-C before saving leaves it unchanged.

### `transcribe run`, `models`, `download`, `remove`

```sh
vesta-cli transcribe models
vesta-cli transcribe download base
vesta-cli transcribe download vad
vesta-cli transcribe run episode.mkv --output episode.srt --model base --vad
vesta-cli transcribe run episode.mkv --output episode.srt --model base --json
vesta-cli transcribe remove vad
```

`run` takes the media path positionally and requires `--output`. Local models
also download on demand. `download vad` installs the default Silero variant;
`vad:<id>` addresses an explicitly listed variant for download/removal. The
current `run --vad` uses the default variant, not the last variant downloaded.
`--no-gpu` forces CPU; otherwise a usable backend must have been compiled into
the binary. The standalone default is CPU-only. Optional unified CLI build:
`cargo install --locked --path cli/vesta-cli --features vulkan` (requires the
Vulkan native toolchain; choose one supported GPU backend).

For cloud runs choose `--provider`, a **provider model name** with `--model`,
and credentials with `--api-key`; the default `base` is a local Whisper ID.
`custom` needs an explicit `--api-url`. API-key arguments can be visible in
shell history/process listings. `--word-timestamps`, `--quality`, `--vad` and `--no-gpu` are local-only
and are rejected for cloud runs; they do not tune a remote provider. There is no GUI-tier failover in a single CLI run. For OpenAI GPT
transcription, whole-chunk timing is less precise than Whisper segment timing.
With automatic local language detection, the actual output filename can gain a
language suffix: read the returned path rather than assuming the requested one.
`--json` uses JSON Lines progress plus one terminal result, not one JSON document.

### `translate`, `--language-list`, `--check-missing`

```sh
vesta-cli translate --language-list
vesta-cli translate --input episode.srt --language it --config translation.toml \
  --output episode.it.srt
vesta-cli translate --input episode.srt --check-missing episode.it.srt
```

Translation needs TOML configuration; see the [complete example](../cli/srt-translate-cli/README.md#configuration).
The language-list and completeness-check modes do not contact providers or need
that config. The check succeeds only if all expected subtitles and line counts
are present; it does not assess translation quality. `--check-missing` is a flag
with the translated filename, not a subcommand. Resume uses an existing output;
never select the input or its alias as the translation output. Use a distinct
output per language/configuration, since existing text is treated as completed
work. Ctrl-C retains completed batches for resumption.

### `refine inspect`, `enrich`

```sh
vesta-cli refine inspect --input deck.apkg
vesta-cli refine enrich --input deck.apkg --output deck.annotated.apkg --config refine.json
```

`inspect` returns card fields as JSON, with no provider requests. `enrich` uses
JSON tiers/prompt settings, not translation TOML or GUI preferences; see the
[configuration example](../cli/srt-refine-cli/README.md). Supported conversions
are TSV→TSV, APKG→TSV and APKG→APKG, **not TSV→APKG**. The parent output directory
must exist. An output equal to input intentionally updates it in place.
By default any failed/remaining card prevents publication. `--allow-partial`
can save successful annotations while leaving other cards unchanged, but exits
1; it is not a success override. Ctrl-C does not publish partial work.

## Output and exit status

| Operation | stdout | stderr |
|---|---|---|
| Flashcards | Human summaries/counts and export paths | Progress and errors (`--quiet` hides progress) |
| Extract | SRT report (selected format), or track-list JSON, unless written to `--output` | Read/save diagnostics and errors |
| Sync | Empty | Completion and errors |
| Auto-sync | Completion summary | Progress/errors (`--quiet` hides progress) |
| Transcribe | Human result; with `--json`, JSON Lines progress/result | Progress/errors, quiet in JSON mode except argument parsing |
| Translate | Human progress/summary/check output | Errors/cancellation |
| Refine | JSON inspection/result | Progress/errors |

Argument parsing errors exit 2; processing/validation failures generally exit 1;
success exits 0. Cancellable generation/translation/transcription/auto-sync/
refinement use 130 for cooperative Ctrl-C failures. A late cancellation can
still leave a successfully published result and exit 0. Do not treat a return
from `cancel_job` in MCP as completed cancellation: poll the job's terminal
status. Inspect stdout according to each command's contract rather than
assuming all commands share a JSON protocol.

## Verification scope

The CLI subprocess tests exercise real commands, help, offline fixtures, usage
errors and publication/cancellation contracts. MCP protocol tests connect a real
stdio SDK client, generate TSV/APKG outputs, and exercise translation/transcription
with local HTTP fixtures, real FFmpeg conversion and cooperative cancellation. See [QUALITY](QUALITY.md).
These checks do not certify authenticated provider access, every media codec,
GPU driver or installed Windows terminal behavior. Re-run the appropriate
release smoke checks for those environments.
