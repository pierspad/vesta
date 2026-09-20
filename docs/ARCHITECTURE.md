# Vesta Architecture

Vesta is built as a modular Cargo workspace organized into **decoupled, reusable crates** layered from pure foundational logic up to the desktop GUI. Every major feature engine lives in its own standalone library crate (`lib/` or `core/`) with clean public APIs, zero GUI coupling, and matching headless CLI tools (`cli/`).

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
│  • srt-difficulty   (CEFR/HSK/JLPT vocabulary profiler & tagger)            │
│  • srt-refine       (LLM subtitle & deck refiner / context merger)          │
│  • srt-condense     (Dialogue extractor & silence eliminator)               │
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
| **lib** | `srt-flashcards`, `srt-translate`, `srt-transcribe`, `srt-autosync`, `srt-extract`, `srt-difficulty`, `srt-refine`, `srt-condense`, `srt-sync`, `srt-ankiconnect` | Self-contained domain engines. **Zero GUI/Tauri coupling**. Long-running tasks accept a `tokio_util::sync::CancellationToken` and report progress through `Arc<dyn Fn(...) + Send + Sync>` closures. Heavy dependencies (`ffmpeg`, `whisper-rs`, `rusqlite`, `reqwest`) are strictly encapsulated here. |
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
                               Lexical Difficulty Tagging (`srt-difficulty`)
                                     │ Analyzes sentences against embedded CEFR/HSK/JLPT/TOPIK/TOCFL
                                     │ or custom user TSV databases ─► assigns level tags (e.g. CEFR::B1)
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
        ├── Local Engine: `whisper.cpp` (via `whisper-rs`, GPU accelerated: CUDA, Metal, Vulkan, OpenVINO)
        └── Cloud Engine: Multi-provider failover (Groq Whisper, OpenAI Whisper, Deepgram, AssemblyAI)
        │
        ▼
Post-processing & Formatting ─► Synchronized `.srt` / `.vtt` output
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

### 4. Lexical Difficulty Profiling (`srt-difficulty`)

```
Target Text Sentence
        │
        ▼
Tokenizer Engine (`srt-difficulty::tokenizer`)
        ├── Chinese: Jieba segmentation
        ├── Japanese: Lindera / Kanji-Kana morphological analysis
        └── European / General: Unicode word boundary segmentation & lemmatization
        │
        ▼
Vocabulary Matcher (`LevelTable`)
        ├── Pre-bundled databases: HSK 1–6, TOCFL 1–6, JLPT N5–N1, TOPIK 1–6, CEFR A1–C2 (EN, DE, IT, ES, FR, RU, PT)
        └── User custom frequency TSV lists
        │
        ▼
Card Difficulty Evaluation (`UnknownPolicy`: Ignore | Highest | Level0)
        └── Produces `CardLevel` + hierarchical Anki tag (e.g. `JLPT::N2`, `HSK::3`, `CEFR::B2`, `Level::0`)
```

### 5. Automatic Subtitle Synchronization (`srt-autosync`, `srt-sync`)

```
Video/Audio File + Desynchronized Subtitle
        │
        ▼
VAD Speech Segmentation & Anchor Extraction
        │ Detects speech start/end timestamps from the audio stream
        ▼
Cross-Correlation & Alignment Matcher
        │ Matches detected voice activity segments against subtitle text timestamps
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
  --video episode01.mkv \
  --target-srt japanese.srt \
  --secondary-srt english.srt \
  --output my_deck.apkg \
  --audio-format opus \
  --snapshot-format webp
```

For module-specific documentation and integration examples, see [`docs/modules/`](modules/README.md).

---

## Development & Build Notes

- **Rapid Development**: The `core/`, `lib/`, and `cli/` crates compile in seconds without pulling in Tauri or Whisper dependencies. Use `cargo check -p srt-flashcards` for fast feedback loops.
- **Fast Linking**: Linux development uses the `mold` linker configured in `.cargo/config.toml`.
- **Version Lockstep**: All internal crates share synchronized version numbers enforced by `build-scripts/check_internal_crate_versions.sh` and pre-push hooks.
- **Quality Gates**: Pre-push hooks validate crate version consistency, `cargo clippy --workspace --all-targets -D warnings`, `rustfmt`, and i18n key parity across 14 supported languages.
