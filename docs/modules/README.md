# Vesta modules

Vesta is a Cargo workspace of decoupled crates. Every heavy feature is a
GUI-agnostic library in `lib/` (progress via callbacks, cancellation via
`CancellationToken`, zero Tauri coupling); most have a matching headless CLI
in `cli/`. The desktop app is just a thin adapter on top.

That means you can take any single feature and reuse it in your own project —
as a standalone binary or as a Rust dependency — without dragging in the GUI.

| Module | What it does | Library | CLI binary | Heavy runtime |
|---|---|---|---|---|
| [srt-flashcards](srt-flashcards.md) | Flashcard generation and media cutting | `lib/srt-flashcards` | `srt-flashcards` | FFmpeg/ffprobe |
| [srt-parser](srt-parser.md) | Parse/write SRT with charset detection | `core/srt-parser` | — | — |
| [srt-apkg](srt-apkg.md) | Anki package (`.apkg`) archive generator & reader | `core/srt-apkg` | — | — |
| [srt-extract](srt-extract.md) | Convert parsed subtitle data (JSON, text, stats) | `lib/srt-extract` | `srt-extract` | — |
| [srt-translate](srt-translate.md) | LLM subtitle translation with tiered failover | `lib/srt-translate` | `srt-translate` | HTTP APIs or local endpoint |
| [srt-sync](srt-sync.md) | Anchor-based subtitle re-timing engine | `lib/srt-sync` | — (see srt-autosync) | — |
| [srt-autosync](srt-autosync.md) | Automatic alignment via Whisper/VAD anchors | `lib/srt-autosync` | `srt-autosync` | FFmpeg + Whisper model |
| [srt-transcribe](srt-transcribe.md) | Media → SRT with Whisper/VAD or cloud STT | `lib/srt-transcribe` | `srt-transcribe` | FFmpeg + model, or HTTP API |
| [srt-ankiconnect](srt-ankiconnect.md) | AnkiConnect HTTP client | `lib/srt-ankiconnect` | — | Anki + AnkiConnect |
| [srt-refine](srt-refine.md) | LLM enrichment of TSV/APKG decks | `lib/srt-refine` | — | HTTP API or local endpoint |

The authoritative dependency direction is `apps/cli → lib → core`; lower
layers never import desktop code. See [the architecture guide](../ARCHITECTURE.md)
for process boundaries, data flow, and acceleration policy.

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

or vendor the crate folder into your own workspace (each module doc lists the
exact folders to copy and the crates.io dependencies involved).

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
