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
| [srt-flashcards](srt-flashcards.md) | Flashcard generation and media cutting | `lib/srt-flashcards` | `srt-flashcards` | FFmpeg/ffprobe |
| [srt-parser](srt-parser.md) | Parse/write SRT with charset detection | `core/srt-parser` | — | — |
| [srt-download](srt-download.md) | Cancellable atomic font/model downloads | `core/srt-download` | — | HTTP endpoint |
| [srt-apkg](srt-apkg.md) | Anki package (`.apkg`) archive generator & reader | `core/srt-apkg` | — | — |
| [srt-extract](srt-extract.md) | Convert parsed subtitle data; discover/extract embedded text tracks | `lib/srt-extract` | `srt-extract` | FFmpeg/ffprobe for embedded tracks |
| [srt-translate](srt-translate.md) | LLM subtitle translation with tiered failover | `lib/srt-translate` | `srt-translate` | HTTP APIs or local endpoint |
| [srt-sync](srt-sync.md) | Anchor-based subtitle re-timing engine | `lib/srt-sync` | `srt-sync` | — |
| [srt-autosync](srt-autosync.md) | Automatic alignment via Whisper/VAD anchors | `lib/srt-autosync` | `srt-autosync` | FFmpeg + Whisper model |
| [srt-transcribe](srt-transcribe.md) | Media → SRT with Whisper/VAD or cloud STT | `lib/srt-transcribe` | `srt-transcribe` | FFmpeg + model, or HTTP API |
| [srt-ankiconnect](srt-ankiconnect.md) | AnkiConnect HTTP client | `lib/srt-ankiconnect` | — | Anki + AnkiConnect |
| [srt-refine](srt-refine.md) | LLM enrichment of TSV/APKG decks | `lib/srt-refine` | `srt-refine` | HTTP API or local endpoint |

The authoritative dependency direction is `apps/cli → lib → core`; lower
layers never import desktop code. See [the architecture guide](../ARCHITECTURE.md)
for process boundaries, data flow, and acceleration policy.

## Unified command

Run `vesta --help` with the desktop installation, or build `cargo build --release -p vesta-cli` for a terminal-only executable without Tauri. Both dispatch the seven operations to the same CLI implementations; for example, `vesta-cli flashcards generate --help`. Run `vesta` without arguments to open the desktop interface. On Windows, use the standalone `vesta-cli.exe` for terminal output.

## MCP integration

The optional `vesta-mcp` server lets compatible assistants inspect subtitles, preview cards and start/cancel TSV or APKG jobs through the same Rust engine. It uses local stdio and explicitly configured file roots. See [the MCP guide](../MCP.md) for setup, tool parameters and limits.

## Two ways to reuse a module

**As a binary** — build only the CLI you need; nothing else gets compiled:

```bash
git clone https://github.com/pierspad/vesta && cd vesta
cargo build --release -p srt-flashcards-cli
./target/release/srt-flashcards --help
```

**As a Rust dependency** — depend on the crate straight from the git repo
(Cargo resolves path dependencies inside the workspace automatically):

```toml
[dependencies]
srt-flashcards = { git = "https://github.com/pierspad/vesta" }
```

Vendoring requires the crate's dependency closure, not only its folder:
manifests inherit edition, Rust version and dependency definitions from the root
workspace. Preserve those definitions or make them explicit in the receiving
workspace. Transcription and autosync also require the root Whisper patch and
native toolchain; see [the integration audit](../RELIABILITY_AUDIT.md#forking-and-integration).

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
