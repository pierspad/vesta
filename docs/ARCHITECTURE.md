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
│  • srt-refine                 │   │  • srt-extract-cli               │
│  • srt-sync                   │   │  • srt-transcribe-cli            │
│  • srt-autosync     ──────────┼──►│  • srt-autosync-cli              │
│  • srt-extract                │   └──────────────────────────────────┘
│  • srt-transcribe   ──────────┼──►  (srt-transcribe)
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
| **lib** | `srt-flashcards`, `srt-translate`, `srt-refine`, `srt-sync`, `srt-autosync`, `srt-extract`, `srt-transcribe` | Feature engines. **No GUI coupling.** Progress via callbacks, cancellation via `CancellationToken`, heavy deps (whisper, ffmpeg, sqlite) encapsulated here. |
| **cli** | `srt-flashcards-cli`, `srt-translate-cli`, `srt-extract-cli`, `srt-transcribe-cli`, `srt-autosync-cli` | Thin `clap` shells. Depend only on their libraries. |
| **apps** | `srt-gui` (`vesta`) | Tauri commands that translate between the GUI (AppHandle, events, state) and the engines. No business logic. |

This is the `.cursorrules` §9 principle: *"Extract common functionality into a
highly-cohesive, decoupled, standalone library crate… expose generic interfaces
using standard callbacks and optional cancellation tokens instead of coupling
with Tauri-specific state."*

## Using a feature headlessly

Each library has a CLI you can build in isolation:

```bash
cargo build --release -p srt-flashcards-cli   # just the flashcard maker
cargo build --release -p srt-transcribe-cli    # just the transcriber (media → SRT)
cargo build --release -p srt-autosync-cli      # just the Whisper-based auto-sync
cargo build --release -p srt-translate-cli     # just the translator
cargo build --release -p srt-extract-cli       # just the SRT data extractor
```

Per-module guides — what each crate does, how to build only its binary, and
how to embed it in another Rust project — live in
[`docs/modules/`](modules/README.md).

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

`benchmarking_against_subs2srs/` measures the flashcard engine against the original subs2srs using
two headless CLIs over the same media. See
[`benchmarking_against_subs2srs/README.md`](../benchmarking_against_subs2srs/README.md).

## Build & iteration notes

* The libraries and CLIs compile in seconds and don't pull in Tauri/whisper, so
  iterate against them (`cargo check -p srt-flashcards`) rather than the full app.
* Linux builds use the `mold` linker via `.cargo/config.toml`.
* Internal crate versions are kept in lock-step (see
  `build-scripts/check_internal_crate_versions.sh`).
