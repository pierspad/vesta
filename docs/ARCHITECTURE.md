# Vesta Architecture

Vesta is a modular Cargo workspace layered from foundational formats to GUI-agnostic feature engines and desktop adapters. The principal workflows have matching headless CLIs; smaller support crates are consumed directly by those workflows.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  apps/srt-gui              Tauri v2 + Svelte 5 desktop application           │
│  apps/whisper-bench        Whisper.cpp & VAD benchmarking utility           │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │ depends on
┌──────────────────────────────────────▼──────────────────────────────────────┐
│  lib/ (feature engines — GUI-agnostic, cancellation & progress callbacks)    │
│  • srt-flashcards   (Anki deck compiler)         ──► cli/srt-flashcards-cli │
│  • srt-translate    (LLM multi-tier translator)  ──► cli/srt-translate-cli  │
│  • srt-transcribe   (Whisper & VAD transcription)──► cli/srt-transcribe-cli │
│  • srt-autosync     (VAD speech auto-aligner)    ──► cli/srt-autosync-cli   │
│  • srt-extract      (Media & subtitle extractor) ──► cli/srt-extract-cli    │
│  • srt-refine       (LLM subtitle & deck refiner / context merger)          │
│  • srt-sync         (Anchor-based retiming engine)                          │
│  • srt-ankiconnect  (AnkiConnect HTTP client)                               │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │ depends on
┌──────────────────────────────────────▼──────────────────────────────────────┐
│  core/ (foundational utilities)                                             │
│  • srt-parser       (High-performance SRT/ASS/VTT parser & charset detect) │
│  • srt-apkg         (Anki .apkg ZIP archive builder & extractor)            │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Architectural Layers

| Layer | Crates | Responsibilities & Design Rules |
|---|---|---|
| **core** | `srt-parser`, `srt-apkg` | Foundational primitives. Minimal external dependencies, zero knowledge of higher engines or GUI. Auto-charset detection via `chardetng` + `encoding_rs`, lossless timing arithmetic, and direct ZIP/SQLite archive serialization. |
| **lib** | `srt-flashcards`, `srt-translate`, `srt-transcribe`, `srt-autosync`, `srt-extract`, `srt-refine`, `srt-sync`, `srt-ankiconnect` | Self-contained domain engines. **Zero GUI/Tauri coupling**. Long-running tasks accept a `tokio_util::sync::CancellationToken` and report progress through callbacks. Heavy dependencies (`ffmpeg`, `whisper-rs`, `rusqlite`, `reqwest`) are encapsulated here. |
| **cli** | `srt-flashcards-cli`, `srt-translate-cli`, `srt-transcribe-cli`, `srt-autosync-cli`, `srt-extract-cli` | Headless, terminal frontends powered by `clap`. Thin wrappers over corresponding `lib/` engines. Perfect for server scripting, batch processing, CI pipelines, and benchmarking. |
| **apps** | `apps/srt-gui` (`vesta`), `apps/whisper-bench` | Desktop frontends. Tauri commands act as adapters converting GUI state and events into calls to `lib/` crates. Pure presentation, UI state (Svelte 5 runes), and system integrations (file dialogs, window controls). |

---

## Key Pipelines and Data Flow

### 1. Flashcard Generation Pipeline (`srt-flashcards`)

```
FlashcardConfig ──► build_matched_lines()  (Parse SRT ─► Normalize ─► Time Shift ─►
                           │                Match Target/Secondary ─► Gap Span ─►
                           │                Filter Min/Max Durations ─► Merge Split Sentences ─►
                           │                Attach Leading/Trailing Context)
                           │
              preview() ◄──┤ (Compute card counts, duration statistics, estimated media sizes)
                           │
             generate() ───┴─► Parallel Media Extraction Pool (ffmpeg / NVENC / VAAPI)
                                     │ • Audio: MP3 / Opus + EBU R128 Loudness Normalization
                                     │ • Snapshots: WebP / AVIF / JPEG + Subtitle Border Crop
                                     │ • Video: H.264 / MPEG-4 ultrafast snippets
                                     ▼
                               Export Output
                                     ├── TSV Deck (Anki-importable with media filenames)
                                     └── Native `.apkg` Package (`srt-apkg` + SQLite collection)
```

### 2. Speech-to-Text Transcription (`srt-transcribe`)

```
Media Input (Video/Audio)
        │
        ▼
Audio Extraction & Preprocessing (16kHz mono WAV via ffmpeg)
        │
        ▼
Voice Activity Detection (Silero VAD or Energy-based VAD)
        │ Splits stream into speech segments and discards prolonged silence
        ▼
Transcription Dispatcher
        ├── Local Engine: `whisper.cpp` via `whisper-rs` (CPU, or a backend compiled into the build)
        └── Cloud Engine: configured OpenAI-compatible and provider-specific STT endpoints
        │
        ▼
Post-processing & Formatting ─► `.srt` output
```

### 3. LLM Translation & Refinement (`srt-translate`, `srt-refine`)

```
Tiered Configuration (`TierEntry`: Provider + Model + Rate Limits)
        │
        ▼
Failover Orchestrator (Tier 0 ─► Tier 1 ─► Tier 2 ─► ...)
        │ Dispatches requests in round-robin across active endpoints in the current Tier.
        │ Automatically fails over to the next tier if quota/RPM/rate-limits are exhausted.
        ▼
LLM Providers (Google Gemini, Groq, OpenAI, Mistral, OpenRouter, GitHub Models, Ollama / Local)
        │
        ▼
Translation & Refine Engine
        ├── Subtitle Translation: Translates dialogue in context-aware batches.
        ├── Dialogue Merging: Reassembles split phrases into natural sentences.
        └── Deck Enrichment: Adds definitions, grammatical breakdowns, and target-language notes.
```

### 4. Automatic Subtitle Synchronization (`srt-autosync`, `srt-sync`)

```
Video/Audio File + Desynchronized Subtitle
        │
        ▼
Audio Sampling and Whisper Transcription
        │ Samples regions of the media and transcribes them locally
        ▼
Fuzzy Text Matcher
        │ Matches transcript fragments to subtitle text and proposes anchors
        ▼
Retiming Calculation (`srt-sync`)
        │ Computes linear / affine time-stretch and global offset transformations
        ▼
Aligned Output Subtitle (.srt)
```

---

## Headless CLI Tooling

Every key engine can be built and run standalone without Tauri:

```bash
# Build standalone CLIs
cargo build --release -p srt-flashcards-cli
cargo build --release -p srt-transcribe-cli
cargo build --release -p srt-translate-cli
cargo build --release -p srt-autosync-cli
cargo build --release -p srt-extract-cli

# Run flashcard generation from terminal
./target/release/srt-flashcards \
  generate --video episode01.mkv \
  --target japanese.srt \
  --native english.srt \
  --output ./my-deck \
  --format apkg \
  --audio-format opus \
  --snapshot-format webp
```

For module-specific documentation and integration examples, see [`docs/modules/`](modules/README.md).

---

## Development & Build Notes

- **Rapid Development**: The `core/`, `lib/`, and `cli/` crates compile in seconds without pulling in Tauri or Whisper dependencies. Use `cargo check -p srt-flashcards` for fast feedback loops.
- **Fast Linking**: Linux development uses the `mold` linker configured in `.cargo/config.toml`.
- **Version Lockstep**: All internal crates share synchronized version numbers enforced by `build-scripts/check_internal_crate_versions.sh` and pre-push hooks.
- **Quality Gates**: Pre-push hooks validate crate version consistency, `cargo clippy --workspace --all-targets -D warnings`, `rustfmt`, tests, and i18n key parity across all shipped locales.
