# <img src="docs/fireplace.svg" alt="Vesta" height="42" align="absmiddle"> Vesta

> [!WARNING]
> **Work in Progress**: This README is currently temporary and a work in progress (WIP), is subject to ongoing reorganization, and will be further refined and expanded.

**subs2srs, but faster and with more features**

Vesta is a modernized spiritual son of subs2srs. 
A desktop application for language learners that turns video/audio files and subtitle files into flashcard decks for anki, allows you to fix desynced subtitles, add missing translation and more 

## Benchmarks

![Vesta Average Speedup vs subs2srs Across All Test Films](docs/benchmark_speedup_summary.svg)

> Tested across **8 feature-length films (~12,000+ subtitles)**, Vesta achieves a **~3.5× to 3.8× average speedup** (and up to **6.22× peak speedup**) compared to subs2srs.
>
> Even when restricted to a single core (`1c`), Vesta is **~1.3× to 1.9× faster** than subs2srs due to new optimizations.
>
> For complete details on the test setup, the vendored subs2srs headless harness, and instructions to run the suite yourself, see [**docs/BENCHMARK_STEPS.md**](docs/BENCHMARK_STEPS.md). For per-film charts and raw data across all 9 variants, see the [**Benchmark Report**](docs/BENCHMARK_REPORT.md).

## What it does

Set up with your preferences and drop in your media and subtitle files and create your flashcards.

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

## Core Features

### 1. Flashcard Generation & Anki (.apkg / TSV) Export
- **Self-Contained `.apkg` Packages**: Exports native SQLite Anki collections with zero manual import mapping needed.
- **Rich Media Cards**: Attach synchronized audio snippets, high-resolution snapshots, or compact video clips to each card.
- **Modern Codec Support**:
  - **Audio**: MP3 or Opus (ultra-low bitrate speech compression).
  - **Snapshots**: WebP, AVIF, or JPEG with customizable resolution presets (144p to 1080p) and quality tuning.
  - **Video Clips**: H.264 or MPEG-4 with hardware acceleration and ultrafast encoding presets.
- **Audio & Video Enhancements**:
  - **EBU R128 Loudness Normalization**: Balances whispering and loud action scenes across cards.
  - **Audio Track Selection**: Extract from multi-language audio streams (e.g. Japanese audio from dual-audio releases).
  - **Subtitle Crop**: Automatically crop bottom borders to remove burned-in hardsubs from snapshots.
  - **Context & Sentence Merging**: Attach leading/trailing context dialogue lines or automatically join split subtitle sentences.
- **Card Styling & Dark Mode**: Beautiful, responsive card templates with native dark-mode support and automatic font stack injection tailored for target languages (CJK Noto, Arabic, Thai, Devanagari, Hebrew, Cyrillic, etc.).

### 2. Speech-to-Text Transcription
Generate accurate SRT subtitles directly from media files:
- **Local Whisper (whisper.cpp)**: Offline transcription with GPU acceleration (Vulkan) and beam search quality modes.
- **Silero VAD (Voice Activity Detection)**: Pre-filters silence and background music, dramatically reducing hallucinations and subtitle drift.
- **Cloud STT Providers**: Integrated support for Groq, OpenAI, Deepgram, and AssemblyAI for lightning-fast cloud transcription.

### 3. Smart Synchronization & Alignment
- **Anchor-based Re-timing (`srt-sync`)**: Align drifting subtitles interactively using waveform anchors.
- **Automatic Whisper Re-sync (`srt-autosync`)**: Automatically generate phonetic anchors with Whisper to realign out-of-sync subtitles with zero manual effort.

### 4. AI Subtitle Translation (`srt-translate`)
- Translate foreign subtitle lines into your native language using Large Language Models.
- Context-aware batching to preserve dialogue flow, slang, and pronouns.
- Multi-tier provider failover (Ollama local, OpenAI, Claude, DeepSeek, OpenRouter).

### 5. Smart Episode & Subtitle Matching
- Drag-and-drop video and subtitle files in bulk.
- Automatically pairs files across complex naming conventions:
  - Western TV: `S01E05`, `1x05`, `Episode 01`, `Folge 06`, `Episodio 03`, `Серия 09`.
  - Anime / Fansub releases: `[Group] Title - 01 [1080p].mkv`, `OVA 01`, `SP 02`.
  - Chinese / Korean dramas: `第01话`, `第12集`, `01화`.
- Auto-detects original vs reference subtitle roles (`source`, `vostfr`, `sub_ita`, `traduzione`, etc.).

### 6. Additional Modules & Integrations
- **Dialogue Condenser (`srt-condense`)**: Strips silence and non-speech intervals to generate condensed audio for listening immersion.
- **AnkiConnect Direct Sync (`srt-ankiconnect`)**: Push notes, media, and decks directly into a running Anki instance without manual `.apkg` file import.
- **Deck Refiner (`srt-refine`)**: Enrich existing Anki decks using LLMs with explanations, grammar notes, and usage examples.

---

## Modular Architecture & Headless CLI Use

Vesta is organized as a Cargo workspace of decoupled, single-responsibility crates. Every backend engine is a GUI-agnostic library in `lib/` or `core/` paired with a standalone CLI binary in `cli/`.

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
│   ├── srt-condense/      # Dialogue extraction and audio condensation
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

---

## Contributing

Pull requests are welcome! For major changes, please open an issue first to discuss your ideas.

---

## AI Disclosure

This project was developed with the assistance of Large Language Models, used to support code writing and documentation.

---

## License

This project is licensed under the GPL v3 License — see the [LICENSE](LICENSE) file for details.
