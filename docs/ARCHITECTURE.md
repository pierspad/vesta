# Vesta architecture

Vesta is a Cargo workspace organised as **decoupled, reusable modules** layered
from pure logic up to the GUI. Each heavy feature lives in its own library crate
with a matching headless CLI, so you can build and use any one feature on its own
— for example, grab *only* the flashcard generator and run it from a terminal.

```
┌─────────────────────────────────────────────────────────────────────┐
│  apps/srt-gui            Tauri v2 + Svelte 5 desktop app             │
│                          (thin command wrappers over the libraries)  │
└───────────────┬─────────────────────────────────────────────────────┘
                │ depends on
┌───────────────▼──────────────┐   ┌──────────────────────────────────┐
│  lib/   (feature engines)     │   │  cli/   (headless front-ends)    │
│  • srt-flashcards   ──────────┼──►│  • srt-flashcards-cli            │
│  • srt-translate    ──────────┼──►│  • srt-translate-cli             │
│  • srt-sync                   │   │  • srt-extract-cli               │
│  • srt-extract                │   └──────────────────────────────────┘
│  • whisper-common             │
└───────────────┬──────────────┘
                │ depends on
┌───────────────▼──────────────┐
│  core/srt-parser              │   Foundational SRT parsing
└──────────────────────────────┘
```

## Layers

| Layer | Crates | Rule |
|---|---|---|
| **core** | `srt-parser` | Foundational, dependency-light parsing. |
| **lib** | `srt-flashcards`, `srt-translate`, `srt-sync`, `srt-extract`, `whisper-common` | Feature engines. **No GUI coupling.** Progress via callbacks, cancellation via `CancellationToken`, heavy deps (whisper, ffmpeg, sqlite) encapsulated here. |
| **cli** | `srt-flashcards-cli`, `srt-translate-cli`, `srt-extract-cli` | Thin `clap` shells. Depend only on their library. |
| **apps** | `srt-gui` (`vesta`) | Tauri commands that translate between the GUI (AppHandle, events, state) and the engines. No business logic. |

This is the `.cursorrules` §9 principle: *"Extract common functionality into a
highly-cohesive, decoupled, standalone library crate… expose generic interfaces
using standard callbacks and optional cancellation tokens instead of coupling
with Tauri-specific state."*

## Using a feature headlessly

Each library has a CLI you can build in isolation:

```bash
cargo build --release -p srt-flashcards-cli   # just the flashcard maker
cargo build --release -p srt-translate-cli     # just the translator
cargo build --release -p srt-extract-cli       # just the SRT data extractor
```

The flashcard engine (`lib/srt-flashcards`) is the newest extraction: its logic
previously lived inside the Tauri app and is now a standalone crate, so the
desktop app, the CLI, and the benchmark harness all share one implementation.
See [`lib/srt-flashcards/README.md`](../lib/srt-flashcards/README.md) and
[`cli/srt-flashcards-cli/README.md`](../cli/srt-flashcards-cli/README.md).

## Flashcard data flow

```
FlashcardConfig ─► build_matched_lines()           (parse→shift→match→span→filter→combine→context)
                     │
        preview() ◄──┤  (counts only, no media)
                     │
        generate() ──┴─► streaming ffmpeg workers (≤ cores-1) ─► TSV  | APKG
                              │ progress callback                      │ rusqlite+zip
                              ▼                                        ▼
                      FlashcardProgressEvent                    .apkg package
```

The Tauri layer feeds `generate()` a closure that re-emits each
`FlashcardProgressEvent` as a `flashcard-progress` Tauri event; the CLI prints a
throttled progress bar to stderr; the benchmark passes a no-op.

## Benchmarking

`benchmarks/` measures the flashcard engine against the original subs2srs using
two headless CLIs over the same media. See
[`benchmarks/README.md`](../benchmarks/README.md).

## Build & iteration notes

* The libraries and CLIs compile in seconds and don't pull in Tauri/whisper, so
  iterate against them (`cargo check -p srt-flashcards`) rather than the full app.
* Linux builds use the `mold` linker via `.cargo/config.toml`.
* Internal crate versions are kept in lock-step (see
  `build-scripts/check_internal_crate_versions.sh`).
