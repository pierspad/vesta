# Vesta modules

Vesta is a Cargo workspace of decoupled crates. Every heavy feature is a
GUI-agnostic library in `lib/` (progress via callbacks, cancellation via
`CancellationToken`, zero Tauri coupling); most have a matching headless CLI
in `cli/`. The desktop app adapts those engines and also owns presentation,
workflow orchestration and native services such as media streaming.

That means you can take any single feature and reuse it in your own project —
as a standalone binary or as a Rust dependency — without dragging in the GUI.

| Module | What it does | Library | CLI binary | Heavy runtime |
|---|---|---|---|---|
| [vesta-flashcards](vesta-flashcards.md) | Flashcard generation and media cutting | `lib/vesta-flashcards` | `vesta-flashcards` | FFmpeg/ffprobe |
| [vesta-parser](vesta-parser.md) | Parse/write SRT with charset detection | `core/vesta-parser` | — | — |
| [vesta-download](vesta-download.md) | Cancellable atomic font/model downloads | `core/vesta-download` | — | HTTP endpoint |
| [vesta-apkg](vesta-apkg.md) | Anki package (`.apkg`) archive generator & reader | `core/vesta-apkg` | — | — |
| [vesta-extract](vesta-extract.md) | Convert parsed subtitle data; discover/extract embedded text tracks | `lib/vesta-extract` | `vesta-extract` | FFmpeg/ffprobe for embedded tracks |
| [vesta-ocr](vesta-ocr.md) | Extract and OCR bitmap subtitle tracks (PGS, VobSub, DVB, XSUB) | `lib/vesta-ocr` | `vesta-ocr` | FFmpeg/ffprobe + Vision LLM / model endpoint |
| [vesta-translate](vesta-translate.md) | LLM subtitle translation with tiered failover | `lib/vesta-translate` | `vesta-translate` | HTTP APIs or local endpoint |
| [vesta-sync](vesta-sync.md) | Anchor-based subtitle re-timing engine | `lib/vesta-sync` | `vesta-sync` | — |
| [vesta-autosync](vesta-autosync.md) | Automatic alignment via Whisper/VAD anchors | `lib/vesta-autosync` | `vesta-autosync` | FFmpeg + Whisper model |
| [vesta-transcribe](vesta-transcribe.md) | Media → SRT with Whisper/VAD or cloud STT | `lib/vesta-transcribe` | `vesta-transcribe` | FFmpeg + model, or HTTP API |
| [vesta-ankiconnect](vesta-ankiconnect.md) | AnkiConnect HTTP client | `lib/vesta-ankiconnect` | — | Anki + AnkiConnect |
| [vesta-refine](vesta-refine.md) | LLM enrichment of TSV/APKG decks | `lib/vesta-refine` | `vesta-refine` | HTTP API or local endpoint |

The authoritative dependency direction is `apps/cli → lib → core`; lower
layers never import desktop code. See [the architecture guide](../ARCHITECTURE.md)
for process boundaries, data flow, and acceleration policy.

## Unified command

Run `vesta --help` with the desktop installation, or build `cargo build --release -p vesta-cli` for a terminal-only executable without Tauri. Both dispatch the eight operations to the same CLI implementations; for example, `vesta-cli flashcards generate --help`. Run `vesta` without arguments to open the desktop interface. On Windows, use the standalone `vesta-cli.exe` for terminal output.

See the [command-line guide](../CLI.md) for the unified and standalone command interfaces.

## MCP integration

The optional `vesta-mcp` server lets compatible assistants inspect subtitles, list and extract embedded tracks, preview cards, start/cancel TSV/APKG, dueling, APKG merge, manual/automatic sync, card refinement, translation, transcription, and OCR jobs, and query or invoke installed Vesta plugins (18 tools total) through the same Rust engines. Translation, refinement, transcription, autosync, and OCR require named server-side provider profiles. It uses local stdio and explicitly configured file roots. See [the MCP guide](../MCP.md) for setup, tool parameters and limits.

## Two ways to reuse a module

**As a binary** — build the CLI you need and its required dependency closure:

```bash
git clone https://github.com/pierspad/vesta && cd vesta
cargo build --release -p vesta-flashcards-cli
./target/release/vesta-flashcards --help
```

**As a Rust dependency** — depend on the crate straight from the git repo
(Cargo resolves path dependencies inside the workspace automatically):

```toml
[dependencies]
vesta-flashcards = { git = "https://github.com/pierspad/vesta" }
```

Vendoring requires the crate's dependency closure, not only its folder:
manifests inherit edition, Rust version and dependency definitions from the root
workspace. Preserve those definitions or make them explicit in the receiving
workspace. Transcription and autosync also require the root Whisper patch and
native toolchain; see [the library integration guide](../library-integration.md).

## Design contract

Feature engines use these conventions where the operation is long-running:

- **No UI coupling** — no Tauri, no GUI types in any `lib/` crate.
- **Progress = callbacks** — plain Rust callbacks, without GUI event types.
- **Cancellation = `tokio_util::sync::CancellationToken`** — cooperative cancellation in pipelines that perform sustained work.
- **External tools are parameters** — ffmpeg/ffprobe are passed as commands or
  paths, never auto-resolved by the libraries.
- **CLIs are adapters** — each shipped CLI parses arguments and delegates to
  its corresponding library; support crates need no separate binary.

## CPU and GPU policy

- Parsing, matching, archive/SQLite work, HTTP orchestration, and audio
  encoding remain on the CPU: moving these small or branch-heavy workloads to
  a GPU would add overhead without a useful speedup.
- Flashcard video optimization probes FFmpeg hardware encoders at runtime
  (NVENC, VA-API, QSV, AMF, or VideoToolbox, depending on the OS). It uses a
  working hardware path and otherwise falls back to `libx264`.
- Local Whisper inference uses the single GPU backend compiled into the binary
  (`vulkan`, `cuda`, `rocm`, or `sycl`) when requested. CPU-only builds and
  machines without a usable device fall back to CPU execution.
- GPU backend Cargo features are alternative build targets, not features to
  enable all at once. Vulkan is the portable Linux release default.
