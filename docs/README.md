# Vesta

> [!WARNING]
> **Work in Progress**: This README is currently temporary and a work in progress (WIP), is subject to ongoing reorganization, and will be further refined and expanded.

**subs2srs, but actually fast.**

Vesta is a modern desktop application for language learners and power users that turns video and subtitle files into rich, synchronized Anki flashcard decks, auto-aligned subtitles, and translated media in minutes instead of hours.

Built with **Rust (Tauri)** + **Svelte 5** + **TypeScript**.

---

## What it does

Load a video and its subtitles. Synchronize them, translate them with AI if needed, and export a ready-to-study Anki deck with high-quality audio clips, snapshots, and video clips synced to the exact lines of dialogue.

![Benchmark comparison: Vesta vs subs2srs](fireplace.png)

### Why Vesta?

- **Parallelized & Multi-core by Default**: Written from scratch in Rust, distributing ffmpeg extractions, media encoding, and database operations across all available CPU cores.
- **2.3× – 2.6× Faster Than subs2srs**: Completes large multi-episode deck generation workflows in a fraction of the time required by legacy tools.
- **100% Offline Capable**: Core media processing, local Whisper transcription, and VAD speech segmentation run entirely on your local machine without mandatory internet access.
- **Decoupled & Modular Architecture**: Every engine is a standalone, headless Rust crate with matching CLI tools.

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
- **Cloud STT Providers**: Integrated support for Groq, OpenAI, Mistral, and Deepgram for lightning-fast cloud transcription.

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

## Pipeline Overview

```
Video / Audio ──► [Transcribe] ──► [Sync / Autosync] ──► [Translate] ──► [Flashcards & Anki Export]
```
*Already have subtitles? Skip directly to Sync or Flashcards generation.*

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

For comprehensive module guides and Rust integration examples, see [`modules/README.md`](modules/README.md) and [`ARCHITECTURE.md`](ARCHITECTURE.md).

---

## Documentation Map

- [`ARCHITECTURE.md`](ARCHITECTURE.md) — Architectural design contracts, layering rules, and conventions.
- [`modules/`](modules/) — Detailed module specifications and embedding instructions.
- [`plans/`](plans/) — Development roadmaps, design documents, and feature plans.
- [`superpowers/specs/`](superpowers/specs/) — Technical specifications for media presets, codec evaluations, and format benchmarks.
- [`../benchmarking_against_subs2srs/`](../benchmarking_against_subs2srs/) — Reproducible benchmarking scripts and methodology.

---

## Building from Source

### Prerequisites
- **Rust**: 1.85+ (`rustup default stable`)
- **Node.js**: 18+ and `npm`
- **System dependencies**: `ffmpeg` and `ffprobe` on your system PATH.

### Development Setup
```bash
# Clone the repository
git clone https://github.com/pierspad/vesta.git
cd vesta

# Install frontend dependencies
cd apps/srt-gui && npm install && cd ../..

# Run GUI in development mode
./run_gui.sh
# Or manually:
cd apps/srt-gui && npx tauri dev
```

---

## Contributing

Pull requests are welcome! For major changes or architectural proposals, please open an issue first to discuss what you would like to change.

---

## License & Disclosures

- **License**: GNU General Public License v3.0 (GPLv3) — see the [../LICENSE](../LICENSE) file for details.
- **AI Disclosure**: This project was developed with the assistance of Large Language Models to support implementation, refactoring, and documentation.


