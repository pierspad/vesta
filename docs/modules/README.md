# Vesta modules

Vesta is a Cargo workspace of decoupled crates. Every heavy feature is a
GUI-agnostic library in `lib/` (progress via callbacks, cancellation via
`CancellationToken`, zero Tauri coupling); most have a matching headless CLI
in `cli/`. The desktop app is just a thin adapter on top.

That means you can take any single feature and reuse it in your own project —
as a standalone binary or as a Rust dependency — without dragging in the GUI.

| Module | What it does | Library | CLI binary |
|---|---|---|---|
| [srt-parser](srt-parser.md) | Parse / write SRT files | `core/srt-parser` | — |
| [srt-extract](srt-extract.md) | Extract & convert subtitle data (JSON, stats…) | `lib/srt-extract` | `srt-extract` |
| [srt-translate](srt-translate.md) | LLM subtitle translation (multi-tier failover) | `lib/srt-translate` | `srt-translate` |
| [srt-sync](srt-sync.md) | Anchor-based subtitle re-timing engine | `lib/srt-sync` | — (see srt-autosync) |
| [srt-autosync](srt-autosync.md) | Automatic alignment via Whisper anchors | `lib/srt-autosync` | `srt-autosync` |
| [whisper-common](whisper-common.md) | Transcription pipeline: media → SRT | `lib/whisper-common` | `srt-transcribe` |
| [srt-flashcards](srt-flashcards.md) | subs2srs-style Anki deck generation | `lib/srt-flashcards` | `srt-flashcards` |
| [srt-refine](srt-refine.md) | LLM enrichment of Anki decks (TSV/APKG) | `lib/srt-refine` | — |

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

All engines follow the same conventions, so they compose predictably:

- **No UI coupling** — no Tauri, no GUI types in any `lib/` crate.
- **Progress = callbacks** — plain `Fn` callbacks (`Arc<dyn Fn(...) + Send + Sync>`).
- **Cancellation = `tokio_util::sync::CancellationToken`** — cooperative, safe to drop.
- **External tools are parameters** — ffmpeg/ffprobe are passed as commands or
  paths, never auto-resolved by the libraries.
- **CLIs are shells** — every CLI parses arguments and delegates to its
  library; if you can do it from the CLI, you can do it from Rust.
