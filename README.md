# <img src="docs/fireplace.svg" alt="Vesta" height="42" align="absmiddle"> Vesta

**Contents:** [Overview](#what-it-does) · [Workflow](#the-workflow) · [Architecture](#how-vesta-was-built) · [CLI](#headless-cli-use) · [Build](#building-from-source) · [Benchmarks](#benchmarks) · [Documentation](#documentation-map)

**subs2srs, but faster and with more features**

Vesta is a modern successor to subs2srs: a desktop application for language learners that turns media and subtitle files into Anki decks. It also covers the work usually required around deck generation—transcription, translation, subtitle synchronization, revision, and card annotation—without forcing users to assemble a chain of unrelated tools.

The desktop application is local-first. Media processing, subtitle parsing, APKG creation, and optional Whisper transcription run on the user's machine. Network services are only used when the user explicitly configures cloud transcription or language-model providers.

## Benchmarks

<img src="docs/benchmark_speedup_summary.svg" alt="Vesta Average Speedup vs subs2srs Across All Test Movies" width="800">

![Suite Overview](docs/benchmark_overview.svg)

<details closed>
  <summary><b>Legend & Pipeline Modes</b></summary>

  - **`1t` vs `16t`**: Worker threads.
    - `1t`: Single-thread control matching subs2srs sequential execution 1:1.
    - `16t`: Parallel workers across all 16 CPU threads.
  - **`Direct` vs `GPU`**: Video cutting strategy.
    - `Direct`: Cuts audio and snapshots directly from the source video (same as subs2srs). Best for standard bitrates and files with frequent keyframes.
    - `GPU`: Pre-transcodes video first via GPU hardware acceleration (VA-API), making seeking instant on heavy codecs (like 1080p HEVC).
  - **`TSV` vs `APKG`**: Output format.
    - `TSV`: Raw text cards + media folder (same as subs2srs).
    - `APKG`: Bundles cards, media, and SQLite database into an Anki `.apkg` ready to import.
  - **`subs2srs baseline (1.0×)`**: Original subs2srs runtime. Values above 1.0× mean faster generation (e.g. 3.8× finishes in ~26% of the time).

</details>

> Tested across **8 feature-length movies (~12,000+ subtitles)**, Vesta achieves a **~3.5× to 3.8× average speedup** (and up to **6.22× peak speedup**) compared to subs2srs.
>
> Even on a single thread (`1t`), like subs2srs, Vesta is **~1.3× to 1.9× faster**.
>
> For full test details and reproduction steps, see [**docs/BENCHMARK_STEPS.md**](docs/BENCHMARK_STEPS.md). For per-movie charts and raw data across all 9 variants, see the [**Benchmark Report**](docs/BENCHMARK_REPORT.md).

## What it does

Set up with your preferences and drop in your media and subtitle files and create your flashcards.

## The workflow

The first-run setup asks for two language choices:

- **Native language** — used for the interface when a translation is available, translation targets, meanings, and reference subtitles.
- **Study language** — used for source flashcards and as the default spoken language for transcription.

Quick setup applies conservative defaults: MP3 audio, APKG export, and the simple interface. Custom setup additionally exposes export format, audio format, Expert Mode, and optional local Whisper installation. Every choice can be changed later in Settings.

The main tabs correspond to independent stages:

1. **Flashcards** matches media and subtitle files, previews cards, applies filters, extracts audio and snapshots with FFmpeg, and exports APKG, TSV, or through AnkiConnect.
2. **Synchronize** fixes subtitle drift with manual anchors or Whisper-assisted automatic matching.
3. **Translate** processes subtitles in resumable, context-aware batches through configurable provider tiers.
4. **Transcribe** converts media to SRT with local whisper.cpp or cloud speech-to-text endpoints. Word-level token timing is always enabled by the GUI for better segment boundaries.
5. **Revise** compares subtitle tracks side by side and supports timing and text corrections.
6. **Annotate** loads TSV or APKG decks and adds manual or generated notes while preserving the deck structure.

AI-backed features remain optional and can be disabled globally with the AI Kill Switch.

#### What if the timestamps of my srt are not synced with the media?
Go to the **"Synchronize"** tab; here you can add manual anchors and check synchronization step by step.
A few anchors are enough if the SRT timestamps are just offsetted.
If there are many discrepancies, use **Auto-Sync** to automatically realign timestamps with help from Whisper.

#### What if I lack an SRT file in my language?
After setting up your tiers and providers for LLM answers in Settings, go to the **"Translate"** tab and translate your file. Subtitles are translated in context-aware batches, with progress saved incrementally so you can resume at any time.

#### What if I only have a media file and I don't have any SRT file?
Go to the **"Transcribe"** tab. You can generate a new `.srt` file directly from your audio or video file using local **Whisper** (via `whisper.cpp` with optional GPU acceleration) or by using fast cloud STT providers specified by you.

#### What if I want to check any missing subtitle?
Go to the **"Revise"** tab. You can load two subtitle files side-by-side (such as a target language SRT and a reference or native SRT) to inspect them line by line. You can quickly jump to missing or empty subtitles, edit lines directly, insert or remove dialogue, and align timings.

#### What if I want to add notes?
Go to the **"Annotate"** tab. You can load an Anki `.apkg` deck or flashcard collection to enrich your cards with definitions, grammar explanations, and context notes. You can edit notes **manually** for each card with the built-in editor, or generate them **automatically** using LLMs.

#### What are tiers and providers?
Tiers and providers are a form of load-balancing for generative operations:
- **Providers** represent individual endpoints (e.g. a Self-Hosted models, OpenRouter, Google, Groq, Mistral, OpenAI, GitHub Models etc.)
- **Tiers** define priority levels. Within each tier, requests rotate across providers in a round-robin cycle to share the workload while respecting their configured requests per minute and maximum request limits. If all providers in a tier exhaust their rate limits or quotas, Vesta automatically fails over to the next tier without interrupting your process.

#### What do I do if I don't want to use any AI feature?
Turn on the **AI Kill Switch**! 


---

## How Vesta was built

Generating cards is only the last step: real source material may have missing subtitles, bad timing, inconsistent encodings, several audio tracks, or no translation. Vesta therefore grew as a collection of small engines rather than one GUI-bound pipeline.

The backend is a Rust workspace. `core/` contains low-level formats with few policy decisions; `lib/` contains reusable workflow engines; `cli/` wraps those engines for automation; and the Tauri application is one consumer of the same APIs. This keeps media and subtitle logic testable without opening a window and prevents the interface from owning business logic.

```
vesta/
├── core/
│   ├── srt-parser/        # SRT parsing, timing, and formatting engine
│   └── srt-apkg/          # Native Anki package (.apkg) SQLite collection builder
├── lib/
│   ├── srt-flashcards/    # Flashcard generation, media orchestration & filters
│   ├── srt-transcribe/    # Whisper.cpp + Silero VAD + Cloud STT pipeline
│   ├── srt-autosync/      # Automatic Whisper-assisted subtitle synchronization
│   ├── srt-sync/          # Anchor-based timing interpolation
│   ├── srt-translate/     # Multi-tier LLM subtitle translation
│   ├── srt-ankiconnect/   # AnkiConnect API client for direct sync
│   ├── srt-extract/       # Subtitle text and metadata extraction
│   └── srt-refine/        # LLM-powered deck enrichment
├── cli/
│   ├── srt-flashcards-cli # Headless flashcard & deck generation CLI
│   ├── srt-transcribe-cli # Media to SRT transcription CLI
│   ├── srt-autosync-cli   # Subtitle re-sync CLI
│   ├── srt-translate-cli  # Subtitle translation CLI
│   └── srt-extract-cli    # Subtitle data extraction CLI
└── apps/
    ├── srt-gui/           # Desktop GUI (Tauri + Svelte 5 + Tailwind)
    └── whisper-bench/     # Transcription benchmarking tool
```

Feel free to reuse these crates for your projects, or to optimize them for niche use cases and make a PR!

The GUI uses Tauri 2, Svelte 5, TypeScript, Tailwind CSS, and Vite. Each main tab is loaded as a separate JavaScript chunk on first use and remains mounted afterwards, preserving in-progress work while keeping startup small. Shared state lives in focused Svelte stores, native operations cross typed Tauri commands, and long-running Rust tasks expose progress and cancellation instead of blocking the frontend.

FFmpeg performs media probing, audio extraction, and snapshots. whisper.cpp provides local transcription, with optional Vulkan, CUDA, ROCm, or SYCL builds and automatic CPU fallback. Silero VAD can skip silence before decoding. APKG output is assembled locally through SQLite and ZIP primitives, so generating a deck does not require a running Anki instance.

Provider tiers decouple translation and cloud transcription from any single vendor. Entries in a tier share work while respecting configured limits; exhaustion falls through to the next tier. Progress is saved incrementally so interrupted work does not need to restart from the beginning.

## Headless CLI Use

### CLI Quick Examples

Build only the standalone CLI tool you need:

```bash
# Build the flashcards CLI
cargo build --release -p srt-flashcards-cli

# Generate an Anki .apkg deck from subtitles and video
./target/release/srt-flashcards generate \
  --target episode-ja.srt \
  --native episode-en.srt \
  --video episode.mkv \
  --output ./out_deck \
  --format apkg \
  --deck "Anime Season 1" \
  --snapshot-format webp \
  --audio-format opus
```

For comprehensive module guides and Rust integration examples, see [`docs/modules/README.md`](docs/modules/README.md) and [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

---

## Documentation Map

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — Architectural design contracts, layering rules, and conventions.
- [`docs/BENCHMARK_STEPS.md`](docs/BENCHMARK_STEPS.md) — Step-by-step benchmark reproduction guide, fairness controls, and subs2srs harness explanation.
- [`docs/BENCHMARK_REPORT.md`](docs/BENCHMARK_REPORT.md) — Full 8-film benchmark results, throughput tables, and per-film charts.
- [`docs/modules/`](docs/modules/) — Detailed module specifications and embedding instructions.

---

## Building from Source

### Prerequisites
- **Rust**: 1.97+ (`rustup default stable`)
- **Node.js**: 20.19+ or 22+ (LTS recommended) and `npm`
- **System dependencies**:
  - **Runtime**: `ffmpeg` and `ffprobe` on your system PATH.
  - **Build (Linux)**: C/C++ compiler (`gcc`/`clang`), `cmake`, `pkg-config`, and Tauri v2 development libraries (`libwebkit2gtk-4.1-dev` or `4.0`, `libappindicator3-dev`, `librsvg2-dev`).

### Development Setup
```bash
# Clone the repository
git clone https://github.com/pierspad/vesta.git
cd vesta

# Install frontend dependencies
cd apps/srt-gui && npm install && cd ../..

# Then either run the GUI in development mode
./run_gui.sh

# Or manually:
cd apps/srt-gui && npx tauri dev
```

Frontend validation and tests:

```bash
cd apps/srt-gui
pnpm check
pnpm test
pnpm build
```

Run every Rust unit, regression, and documentation test from the repository root:

```bash
cargo test --workspace
```

GPU support is selected at compile time. Systems without a usable accelerator fall back to CPU at runtime. See [`docs/modules/srt-transcribe.md`](docs/modules/srt-transcribe.md) for backend-specific requirements.

---

## Contributing

Pull requests are welcome! For major changes, please open an issue first to discuss your ideas.

If Vesta is useful to you and you want to support its maintenance, you can [sponsor the project on GitHub](https://github.com/sponsors/pierspad). Sponsorship is optional and does not unlock features.

---

## LLM Disclosure

This project was developed with the assistance of Large Language Models, used to support code writing and documentation.

---

## License

This project is licensed under the GPL v3 License — see the [LICENSE](LICENSE) file for details.
