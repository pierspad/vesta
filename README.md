# <img src="docs/fireplace.svg" alt="Vesta" height="42" align="absmiddle"> Vesta

> [!WARNING]
> **Work in Progress**: This README is currently temporary and a work in progress (WIP), is subject to ongoing reorganization, and will be further refined and expanded.

**subs2srs, but actually fast.**

Vesta is a modern desktop application for language learners and power users that turns video and subtitle files into rich, synchronized Anki flashcard decks, auto-aligned subtitles, and translated media in minutes instead of hours — running **~2.5× to 5.2× faster than subs2srs** with parallel multi-core processing and transparent GPU hardware acceleration.

![Benchmark comparison: Vesta vs subs2srs](docs/benchmark.svg)

> ⚡ **GPU Acceleration & Pre-Transcoding**: On heavy 1080p and HEVC videos, Vesta automatically leverages hardware acceleration (VA-API, NVENC, VideoToolbox) to pre-transcode lightweight streams at 50–100× realtime, delivering up to **5.17× speedup over subs2srs** and cutting generation time in half compared to multi-core CPU alone. See the full [Benchmark Report with GPU acceleration](docs/BENCHMARK_REPORT.md#gpu-hardware-acceleration--adaptive-pre-transcoding).

![Benchmark with GPU acceleration](docs/benchmark_gpu.svg)

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

### 2. Difficulty Tagging & Vocabulary Profiling
Vesta automatically analyzes the lexical complexity of each subtitle sentence and tags cards with their proficiency level (e.g., `HSK::3`, `CEFR::B1`, `JLPT::N2`, `TOPIK::3`, `TOCFL::B2`):

#### Built-in Databases & Provenance:
- 🇨🇳 **HSK (Simplified Chinese)** — `12,543 entries` (HSK 1–6): Official Hanban / [CTI](http://www.chinesetest.cn/) syllabus & [Ministry of Education PRC](http://www.moe.gov.cn/) with Simplified/Traditional variants ([gigacover/hsk](https://github.com/gigacover/hsk)).
- 🇹🇼 **TOCFL (Traditional Chinese)** — `11,170 entries` (Levels 1–6 / Novice to Superior): Official Taiwan Ministry of Education [SC-TOP](https://tocfl.edu.tw/) *8000 Vocabulary List* ([Downloads](https://tocfl.edu.tw/index.php/exam/download)).
- 🇯🇵 **JLPT (Japanese)** — `13,930 entries` (Levels N5–N1): [Jonathan Waller's JLPT (Tanos)](https://www.tanos.co.uk/jlpt/) & [EDRDG / JMdict](https://www.edrdg.org/jmdict/j_jmdict.html) (Kanji, Kana, and dictionary forms).
- 🇰🇷 **TOPIK (Korean)** — `6,671 entries` (Levels 1–6): Official [NIIED](https://www.topik.go.kr/) & [National Institute of Korean Language](https://www.korean.go.kr/) standard vocabulary syllabus.
- 🇬🇧 **CEFR English** — `8,845 entries` (Levels A1–C2): [Cambridge English Profile (EVP)](https://www.englishprofile.org/) & [Oxford 3000/5000](https://www.oxfordlearnersdictionaries.com/wordlists/) aligned with the [Council of Europe CEFR](https://www.coe.int/en/web/common-european-framework-reference-languages).
- 🇪🇺 **CEFR Multi-lingual European Databases** (🇩🇪 German `41,900`, 🇮🇹 Italian `19,951`, 🇪🇸 Spanish `19,945`, 🇫🇷 French `19,941`, 🇵🇹 Portuguese `19,950`, 🇷🇺 Russian `19,970 entries`): Frequency distributions derived from [OPUS OpenSubtitles (HermitDave)](https://opus.nlpl.eu/OpenSubtitles.php) and [Leipzig Corpora Collection](https://wortschatz.uni-leipzig.de/), mapped to 6 logarithmic CEFR brackets (A1=Top 1K to C2=20K+).

> 📖 **Full Linguistic Provenance & Reproduction Guide**: See [`docs/VOCABULARY_SOURCES.md`](docs/VOCABULARY_SOURCES.md) for full dataset licenses, exact entry counts, methodology, and the deterministic build script.

- **Zero Internet Required & 1-Click Database Export**: Embedded directly into the binary. Export any database to `.tsv` with one click from Settings for inspection, customization, or community sharing.
- **Custom Vocabulary TSVs & User Schemes**: Load your own custom frequency or vocabulary lists with user-defined tag prefixes.
- **Configurable Unknown Word Policies**:
  - `Ignore`: Ignore unlisted words and tag based on known vocabulary.
  - `Highest`: Treat unlisted/rare words as maximum difficulty.
  - `Level 0`: Assign explicit Level 0 (e.g. `HSK::0`, `CEFR::0`, `JLPT::0`, `TOPIK::0`, `TOCFL::0`, `Level::0`) for unclassified sentences.

### 3. Speech-to-Text Transcription
Generate accurate SRT subtitles directly from media files:
- **Local Whisper (whisper.cpp)**: Offline transcription with GPU acceleration (Vulkan) and beam search quality modes.
- **Silero VAD (Voice Activity Detection)**: Pre-filters silence and background music, dramatically reducing hallucinations and subtitle drift.
- **Cloud STT Providers**: Integrated support for Groq, OpenAI, Mistral, and Deepgram for lightning-fast cloud transcription.

### 4. Smart Synchronization & Alignment
- **Anchor-based Re-timing (`srt-sync`)**: Align drifting subtitles interactively using waveform anchors.
- **Automatic Whisper Re-sync (`srt-autosync`)**: Automatically generate phonetic anchors with Whisper to realign out-of-sync subtitles with zero manual effort.

### 5. AI Subtitle Translation (`srt-translate`)
- Translate foreign subtitle lines into your native language using Large Language Models.
- Context-aware batching to preserve dialogue flow, slang, and pronouns.
- Multi-tier provider failover (Ollama local, OpenAI, Claude, DeepSeek, OpenRouter).

### 6. Smart Episode & Subtitle Matching
- Drag-and-drop video and subtitle files in bulk.
- Automatically pairs files across complex naming conventions:
  - Western TV: `S01E05`, `1x05`, `Episode 01`, `Folge 06`, `Episodio 03`, `Серия 09`.
  - Anime / Fansub releases: `[Group] Title - 01 [1080p].mkv`, `OVA 01`, `SP 02`.
  - Chinese / Korean dramas: `第01话`, `第12集`, `01화`.
- Auto-detects original vs reference subtitle roles (`source`, `vostfr`, `sub_ita`, `traduzione`, etc.).

### 7. Additional Modules & Integrations
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
│   ├── srt-difficulty/    # Lexical complexity analyzer & multi-lingual vocab tables
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
- [`docs/VOCABULARY_SOURCES.md`](docs/VOCABULARY_SOURCES.md) — Exact linguistic dataset provenance, upstream sources, licenses, and build scripts.
- [`docs/modules/`](docs/modules/) — Detailed module specifications and embedding instructions.
- [`docs/superpowers/specs/`](docs/superpowers/specs/) — Technical specifications for media presets, codec evaluations, and format benchmarks.
- [`benchmarking_against_subs2srs/`](benchmarking_against_subs2srs/) — Reproducible benchmarking scripts and methodology.

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

- **License**: GNU General Public License v3.0 (GPLv3) — see the [LICENSE](LICENSE) file for details.
- **AI Disclosure**: This project was developed with the assistance of Large Language Models to support implementation, refactoring, and documentation.
