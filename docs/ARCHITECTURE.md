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
│  • srt-parser       (SRT parser/writer with automatic charset detection)    │
│  • srt-apkg         (Anki .apkg ZIP archive builder & extractor)            │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Architectural Layers

| Layer | Crates | Responsibilities & Design Rules |
|---|---|---|
| **core** | `srt-parser`, `srt-apkg` | Foundational primitives. Minimal external dependencies, zero knowledge of higher engines or GUI. Auto-charset detection via `chardetng` + `encoding_rs`, lossless timing arithmetic, and direct ZIP/SQLite archive serialization. |
| **lib** | `srt-flashcards`, `srt-translate`, `srt-transcribe`, `srt-autosync`, `srt-extract`, `srt-refine`, `srt-sync`, `srt-ankiconnect` | Self-contained domain engines. **Zero GUI/Tauri coupling**. Long-running tasks accept a `tokio_util::sync::CancellationToken` and report progress through callbacks. External tools and heavy dependencies (FFmpeg processes, `whisper-rs`, `rusqlite`, `reqwest`) are encapsulated here. |
| **cli** | `srt-flashcards-cli`, `srt-translate-cli`, `srt-transcribe-cli`, `srt-autosync-cli`, `srt-extract-cli` | Headless, terminal frontends powered by `clap`. Thin wrappers over corresponding `lib/` engines. Perfect for server scripting, batch processing, CI pipelines, and benchmarking. |
| **apps** | `apps/srt-gui` (`vesta`), `apps/whisper-bench` | Desktop composition roots. Svelte owns presentation/state; Tauri commands adapt IPC to `lib/` crates and own process-level services such as local media streaming, dialogs, and window integration. |

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
             generate() ───┴─► Optional video-clip source preparation + bounded FFmpeg pool
                                     │ • Audio: MP3 / Opus + loudness normalization
                                     │ • Snapshots: WebP / AVIF / JPEG + border crop
                                     │ • Video: H.264 / MPEG-4 snippets
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
LLM Providers (Google Gemini, Groq, OpenAI, Mistral, OpenRouter, NVIDIA NIM, Ollama / Local)
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

## Frontend execution boundaries

The desktop tabs compose UI and event lifetimes. The modules in
`apps/srt-gui/src/lib/workflows` own flashcard execution/aggregation/merge,
bounded file discovery, transcription failover and model/backend/VAD discovery.
Their injected IPC functions and callbacks support headless regression tests.
Reactive discovery state lives in `TranscriptionResources`; live transcript
rendering lives in `TranscriptionSegmentsPanel`. Formatting helpers are pure.
This frontend orchestration chooses and sequences native commands; the Rust
feature libraries still perform the underlying processing.

## Runtime Boundaries

The desktop UI never calls FFmpeg, Whisper, SQLite, or provider APIs directly.
Svelte invokes typed Tauri commands; those commands translate persisted/UI
configuration into library structs, hold cancellation state, and forward
progress events. Domain behavior stays in `lib/`, so the same pipeline is used
by the CLI and benchmark harness.

Media preview is the one process-level service owned by the desktop adapter:
an ephemeral loopback HTTP server supports byte-range requests for WebKitGTK.
On Linux, WebKitGTK decodes preview media through GStreamer. This is separate
from the FFmpeg-based generation/transcription pipelines.

Long-running work follows the same ownership model:

1. the caller creates a `CancellationToken` and progress callback;
2. the feature engine owns orchestration and bounded concurrency;
3. external processes are killed on cancellation/drop where supported;
4. only serializable progress/results cross the Tauri boundary.

## Acceleration and Fallback Policy

GPU use is selective rather than global:

| Workload | Preferred path | Fallback | Why |
|---|---|---|---|
| H.264 video optimization/clips | Runtime-probed NVENC, VA-API, QSV, AMF, or VideoToolbox | `libx264` | Encode/transcode is sufficiently large to amortize device transfer. |
| Local Whisper inference | One compiled backend: Vulkan, CUDA, ROCm, or SYCL | whisper.cpp CPU backend | Matrix-heavy inference benefits from GPU offload. |
| Audio decode/resample/encode | FFmpeg CPU path | CPU | Short per-card clips and codec support make GPU transfer unattractive. |
| Snapshots | FFmpeg CPU filters/encoders | CPU | Small still outputs are normally transfer-bound. |
| Parsing, matching, retiming, SQLite/ZIP, HTTP | CPU | CPU | Branch-heavy, I/O-bound, or small workloads are not good GPU candidates. |

`video_hw_accel = "auto"` performs an actual test encode before choosing a
hardware encoder. Local transcription defaults to `use_gpu = true`, but this
only activates a backend included at compile time. Official Linux desktop
builds enable Vulkan; development or CLI CPU builds remain valid. A missing or
unusable GPU must never prevent completion on the CPU.

Do not combine all Whisper GPU Cargo features in one binary: their vendor
toolchains and link requirements are alternative build targets. The
`whisper-bench` app handles this by using a Vulkan launcher and separate
downloadable single-backend workers.

## Linux Media Stack

- FFmpeg/ffprobe: generation, extraction, source optimization, and audio
  preparation for Whisper.
- GStreamer through WebKitGTK: playback inside the desktop webview only.
- Vulkan loader/driver: local Whisper offload in Vulkan-enabled builds.
- VA-API/NVENC/QSV: selected at runtime by FFmpeg when a real probe succeeds.

On Arch Linux, preview support requires `gstreamer`, `gst-plugins-base`,
`gst-plugins-good`, `gst-plugins-bad`, `gst-plugins-ugly`, and `gst-libav`.
The AUR `PKGBUILD` declares these runtime dependencies.

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

## Configuration, embedded subtitles, and output files

`vestaConfig.ts` hydrates a string-keyed cache before dynamically importing the
Svelte app/stores. Mutations are serialized over IPC. Setup awaits a durable snapshot via `replaceAll()`
before reloading: an in-memory flag is insufficient to mark onboarding complete.
Rust serializes disk access under `ConfigState`, writes a unique temporary file,
syncs it, and publishes it atomically. Invalid/unreadable configuration is surfaced
instead of silently replaced by defaults. API keys are stored in this configuration
file; it is not an encrypted keychain.

Embedded subtitle discovery/extraction lives in `srt-extract::embedded` and uses
FFprobe/FFmpeg. Only text tracks are converted; PGS/VobSub need OCR. The desktop
Extract tab saves independent SRT files, keeping the existing workflows simple.

Fresh per-episode APKG exports can be merged by `srt-flashcards::merge_apkg`.
The frontend writes episodes to distinct directories; the merger remaps note/card
IDs, combines metadata/media, rejects conflicts, and publishes a complete archive.
It is intended for fresh Vesta exports, not scheduled Anki collections.
`open_output_path` opens the saved file or its parent with the system association.

## Preparation progress and quality

Audio plus snapshots use the original media directly. A full-film H.264
intermediate is reserved for video-clip workflows with `optimize_video` enabled;
it can amortize repeated seeks for heavy codecs/high resolutions, but is lossy
and does not improve source quality. Modest H.264 downscaling alone does not
justify converting the whole film, even with a hardware encoder.

FFmpeg preparation emits `out_time_us` progress relative to the source duration.
The UI displays preparation percentage separately from the overall pipeline
percentage (12–15%). This is a phase completion measure, not an estimate of
elapsed time for the entire job. Cancellation kills the active child; a stalled
progress stream is bounded and a failed attempt falls back to another encoder.

Cloud transcription is one selected engine, not the translation pool. OpenAI
Whisper returns segment timing; GPT-4o transcription uses JSON and a whole-chunk
time interval. It cannot provide precise line timing in the current integration.
Cloud requests/polling are cancellable by dropping the selected future; remote
AssemblyAI jobs may continue after local cancellation. See [QUALITY](QUALITY.md).
