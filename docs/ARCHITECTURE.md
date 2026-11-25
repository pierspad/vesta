# Vesta Architecture

Vesta is a modular Cargo workspace layered from foundational data formats to GUI-agnostic feature engines and thin CLI/MCP/desktop adapters.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  apps/vesta-gui            Tauri v2 + Svelte 5 desktop application          │
│  apps/whisper-bench        Whisper.cpp & VAD benchmarking utility           │
│  cli/vesta-cli             Unified CLI dispatcher (vesta-cli <command>)     │
│  cli/vesta-mcp             Model Context Protocol (stdio MCP, 24 tools)     │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │ depends on
┌──────────────────────────────────────▼──────────────────────────────────────┐
│  lib/ (feature engines — GUI-agnostic, cancellation & progress callbacks)   │
│  • vesta-flashcards   (Anki deck compiler)         ──► cli/vesta-flashcards-cli │
│  • vesta-translate    (LLM multi-tier translator)  ──► cli/vesta-translate-cli  │
│  • vesta-transcribe   (Whisper & VAD transcription)──► cli/vesta-transcribe-cli │
│  • vesta-autosync     (VAD speech auto-aligner)    ──► cli/vesta-autosync-cli   │
│  • vesta-extract      (Media & subtitle extractor) ──► cli/vesta-extract-cli    │
│  • vesta-ocr          (Bitmap & ROI OCR engine)    ──► cli/vesta-ocr-cli        │
│  • vesta-refine       (LLM deck annotation)        ──► cli/vesta-refine-cli     │
│  • vesta-sync         (Anchor-based retiming)      ──► cli/vesta-sync-cli       │
│  • vesta-ankiconnect  (AnkiConnect HTTP client)                                 │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │ depends on
┌──────────────────────────────────────▼──────────────────────────────────────┐
│  core/ (foundational utilities)                                             │
│  • vesta-parser       (SRT parser/writer with automatic charset detection)  │
│  • vesta-apkg         (Anki .apkg ZIP archive builder & extractor)          │
│  • vesta-download     (Cancellable downloads & atomic publication)          │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Architectural Layers

| Layer | Crates | Responsibilities & Design Rules |
|---|---|---|
| **core** | `vesta-parser`, `vesta-apkg`, `vesta-download` | Foundational primitives with zero knowledge of higher engines or GUI: auto-charset detection (`chardetng` + `encoding_rs`), lossless timing math, atomic file replacement, direct ZIP/SQLite `.apkg` serialization, and cancellable resource downloads. |
| **lib** | `vesta-flashcards`, `vesta-translate`, `vesta-transcribe`, `vesta-autosync`, `vesta-extract`, `vesta-ocr`, `vesta-refine`, `vesta-sync`, `vesta-ankiconnect` | Self-contained domain engines with **zero GUI/Tauri coupling** and no global logger initialization. Long-running tasks accept a `tokio_util::sync::CancellationToken` and report progress/diagnostics via plain callbacks. Heavy runtime dependencies (FFmpeg subprocesses, `whisper-rs`, ONNX OCR, `rusqlite`, `reqwest`) stay encapsulated here. |
| **cli** | `vesta-cli`, `vesta-mcp`, and `vesta-*-cli` (`flashcards`, `translate`, `transcribe`, `autosync`, `extract`, `ocr`, `sync`, `refine`) | Headless `clap` CLI adapters, the unified `vesta-cli` dispatcher, and the `vesta-mcp` stdio MCP server (24 tools). Adapters own `Ctrl+C` handling, CLI validation, and stderr/stdout formatting. |
| **apps** | `apps/vesta-gui` (`vesta`), `apps/whisper-bench` | Desktop composition roots. Svelte owns presentation and workflow sequencing; Tauri commands adapt IPC to `lib/` crates and host desktop-only services (loopback media range server, out-of-process JSON-RPC plugin host, support logs, dialogs, and installer updates). |

---

## Key Pipelines and Data Flow

### 1. Flashcard Generation (`vesta-flashcards`)

```
FlashcardConfig ──► build_matched_lines()  (Parse SRT/ASS/VTT ─► Normalize ─► Time Shift ─►
                           │                Match Target/Native ─► Span Bounds ─►
                           │                Duration/Length/Word Filters ─► Sentence Merge ─►
                           │                Attach Leading/Trailing Context)
                           │
              preview() ◄──┤ (Card counts, duration statistics, paginated preview rows)
                           │
             generate() ───┴─► Optional video-clip source preparation + bounded FFmpeg pool
                                     │ • Audio: MP3 / Opus + optional normalization & padding
                                     │ • Snapshots: bounded WebP PTS batches or per-card AVIF/JPEG + crop
                                     │ • Video: H.264 (probed HW or libx264) / MPEG-4 clips
                                     ▼
                               Export Output: TSV Deck + staged media  OR  Native `.apkg` (`vesta-apkg`)
```

- **Preparation & progress**: Audio/snapshot-only runs read original media directly. Full-film H.264 intermediate transcoding runs only when video clips and `optimize_video` are both enabled; FFmpeg `out_time_us` preparation progress is reported separately from overall deck completion, and stalled/failing hardware attempts fall back cleanly.
- **Export format & series boundaries**: `buildFlashcardConfig` maps media-generation flags independently of enabled note-type fields. `export_tsv` writes ordered media columns without embedding templates (allowing snapshot + video coexistence); `export_apkg` embeds media and Anki models. `runFlashcardSeries` captures the effective format at start and applies single-package merging (`vesta-flashcards::merge_apkg`, intended for fresh Vesta exports rather than scheduled Anki collections) only to APKG.

### 2. Speech-to-Text Transcription (`vesta-transcribe`)

```
Media Input ─► 16 kHz mono PCM WAV (FFmpeg) ─► VAD Segmentation (Silero / WebRTC / Energy)
   ─► Dispatcher: Local `whisper.cpp` (`whisper-rs`) OR Cloud STT ─► Atomic `.srt` output
```

- Each native transcription call runs a single engine; desktop `runTranscription` attempts ready endpoints sequentially across configured tiers rather than load-balancing concurrently.
- OpenAI Whisper returns segment timestamps; text-only/chunk-only cloud models (`gpt-transcribe`, JSON-mode GPT-4o) cannot provide line-level alignment and are excluded from synchronization. Dropping a cloud future cancels local waiting, though remote provider jobs (e.g., AssemblyAI) may finish server-side.

### 3. LLM Translation, OCR & Deck Refinement (`vesta-translate`, `vesta-ocr`, `vesta-refine`)

```
Tiered Pool (Tier 0 ─► Tier 1 ─► ...) ─► Round-robin within active tier; failover on 429/quota
   ─► Providers: Gemini, Claude, Groq, OpenAI, Mistral, OpenRouter, NVIDIA NIM, Ollama/Custom
```

- **Diagnostics decoupling**: Provider errors expose typed categories and HTTP status codes; optional retry observers allow CLI adapters to log to stderr and desktop `provider_diagnostics` to record `provider-retry` support events without coupling `lib/` to Tauri.
- **Embedded & hardcoded subtitles (`vesta-extract`, `vesta-ocr`)**: `vesta-extract::embedded` converts text streams directly via FFprobe/FFmpeg. Bitmap tracks (PGS, VobSub, DVB, XSUB) and burned-in video regions (`VideoRoi` in `vesta-ocr::hardcoded`) route through `CompositeOcrBackend`, combining offline pure-Rust PaddleOCR v4 ONNX models (`LocalOcrBackend`, unaffected by the AI Kill Switch) and Vision LLM tiers (preferring `vesta-itt-tiers` before general LLM tiers, gated by the AI Kill Switch).

### 4. Subtitle Synchronization (`vesta-autosync`, `vesta-sync`)

```
Media + SRT ─► Sampled Audio + Whisper/Cloud STT ─► Fuzzy Text Matcher ─► Anchor Proposal
   ─► `vesta-sync` (constant offset for 1 anchor; piecewise-linear drift for ≥2 anchors) ─► `.srt`
```

---

## Runtime, Network & Security Boundaries

- **Sandboxed HTTP proxy (`http_fetch` in `commands/net.rs`)**: Provider discovery, readiness probes, and update checks run through Rust rather than browser `fetch`. It enforces HTTPS/allowlisted hosts (unless a custom provider URL is explicitly configured), blocks loopback/private/link-local SSRF ranges (`169.254.0.0/16`, `fe80::/10`, RFC1918), forbids cross-origin redirects (`Policy::custom`), and caps streamed bodies at 8 MiB.
- **Loopback media server (`media_stream.rs` & `stream://`)**: Serves byte-range previews on `127.0.0.1:0` for WebKitGTK/webview playback, gated by a per-process CSPRNG token verified in constant time (`subtle::ConstantTimeEq`), strict CORS `AllowOrigin`, bounded range reads, and canonical `MediaAllowlist` path validation rejecting symlinks, relative paths, and unapproved files.
- **Playback cache (`sync_prepare_media_for_playback` / `vesta-sync::playback`)**: Natively playable audio (`mp3`, `wav`, `ogg`, `m4a`, `aac`, `opus`, `flac`) plays directly. Video/other containers transcode under a process-local mutex (with 5-minute FFmpeg timeout) to a cached 16 kHz mono PCM WAV (`{hash}.wav`, reused while `cache_modified >= source_modified`) published atomically without altering flashcard export settings.
- **Desktop host services**: `commands/support_logs.rs` owns diagnostic recordings; `commands/info.rs` exposes `get_app_info`, `get_system_diagnostics`, and `get_mcp_cli_info`; `commands/plugins.rs` + `src-tauri/src/plugins/` supervise external JSON-RPC 2.0 plugin processes (`plugin_inspect`, `plugin_install`, `plugin_list`, `plugin_enable`, `plugin_disable`, `plugin_uninstall`, `plugin_update`, `plugin_get_settings`, `plugin_save_settings`, `plugin_render`, `plugin_dispatch_event`; see [`PLUGINS.md`](PLUGINS.md)); `commands/updates.rs` resolves stable platform installers, verifies SHA-256 digests, and launches them (`open_output_path` opens exported files/folders via OS associations).

---

## Operation Ownership & Frontend Lifecycles

| Boundary | Mechanism & Invariants |
|---|---|
| **Rust operation slots (`OperationGuard`)** | Desktop commands reserve per-domain slots via RAII (`Auto-Sync` locks both sync and transcription with a shared `CancellationToken`). Cancellation stops child processes without releasing the slot early; slots release only on return, error, or future drop. Worker pools use `JoinSet` + `Semaphore`. |
| **Frontend locks (`AsyncOperation` & `LatestRequest`)** | `AsyncOperation` holds the UI lock until both native work and any in-flight cancellation settle (global broadcast events never unlock a run early). Whisper inference, Auto-Sync, and model mutations share one local Whisper lock. `LatestRequest` invalidates superseded file discovery, preview, and playback-prep calls. |
| **Persistent lazy tabs (`App.svelte`)** | `createTabLoader` deduplicates dynamic tab imports (`preloadTabs` warms tabs during idle time) and keeps loaded tabs mounted in hidden containers so background jobs survive navigation. Changing tabs toggles `active` rather than triggering `onDestroy`; polling and global shortcuts must check `active`, `isComposing`, and `isInputActive`. `manageWindowSize` checks disposal after async setup. |
| **Workflow modules (`src/lib/workflows/`)** | Own flashcard series/merge sequencing, bounded file discovery, transcription failover, and model/VAD discovery (`TranscriptionResources`) with injected IPC callbacks for Node unit testing, keeping `.svelte` views focused on DOM/presentation (`TranscriptionSegmentsPanel`, `TierList`, `FooterActions`, `PathPickerField`). |
| **Preview lifecycle (`FlashcardPreviewController`)** | Separates tab hiding from tab destruction: deactivating Flashcards closes `FlashcardsPreviewModal`, invalidates pending preview requests (`PreviewStore`), and releases preview media elements while preserving subtitle edits and running exports. |
| **Notifications (`snackbarStore.svelte.ts`)** | Coalesces duplicates, prioritizes errors in a bounded queue, enforces per-severity minimum durations (`1700ms` / `3500ms` / `5000ms`), pauses on hover/focus/hidden window, and prevents stale close timers from dismissing newer alerts. Denied clipboard operations never report success or perform destructive cuts. |

---

## Configuration, Onboarding & Atomic Output Publication

- **Configuration (`vestaConfig.ts` & `ConfigState`)**: Hydrates an in-memory string map before dynamically importing Svelte stores/app, migrating legacy `localStorage` atomically via `replaceAll()`. Rust serializes mutations under `ConfigState` via temp-file write + `sync_all` + atomic rename. Corrupted JSON is quarantined to `.corrupt.<timestamp>` (surfacing the backup path; unreadable files fail startup rather than silently resetting user data). Note: `vesta_config.json` stores API keys in plain JSON, not an OS keychain. Background write failures preserve session state and raise a localized error snackbar, whereas explicit `flush()` / `replaceAll()` reject so onboarding/import cannot claim durable completion on failure.
- **First-run setup (`FirstRunSetupModal`, `firstRunSetup.ts`, `services/setup.ts`)**: Plans and downloads missing media tools (`setup_media_status` checks FFmpeg/FFprobe and probes archive size via bounded `HEAD`), language fonts, and Whisper/VAD/OCR models (`Quick` vs `Custom`), skipping already-installed assets on retry and awaiting `replaceAll()` before reloading into the chosen initial tab (`Preview` mode skips persistence and downloads; see [`SETUP_TESTING.md`](SETUP_TESTING.md)).
- **Atomic single-file publication (`vesta-parser::persistence`)**: Used across SRT saves, manual sync, transcription, extraction, and APKG exports. Writers flush and finalize buffers/archives into a sibling temp file, check cancellation prior to commit, `sync_all`, and atomically replace the target (preserving existing symlinks and Unix mode bits while rejecting dangling symlinks and read-only destinations; errors remove the temp file and leave the prior file untouched).
- **Staged TSV media publication (`vesta-flashcards`)**: Generates media in `.vesta-export-*` on the target filesystem, acquires an OS lock on `.<deck>.tsv.export.lock`, moves prior media to `previous.media`, swaps in new media, and atomically writes the `.tsv`. Swap/TSV failures roll back `previous.media` (or retain staging with its recovery path if rollback fails; `SIGKILL`/power loss mid-swap requires manual inspection of staging). Media/TSV symlinks into the media tree are rejected.

---

## Acceleration Policy & Linux Media Stack

| Workload | Preferred Path | Fallback | Rationale |
|---|---|---|---|
| H.264 video optimization & clips | Runtime-probed NVENC, VA-API, QSV, AMF, or VideoToolbox | `libx264` | Large encode amortizes GPU transfer (`video_hw_accel = "auto"` runs a test encode first). |
| Local Whisper inference | Single compiled backend (`vulkan`, `cuda`, `rocm`, or `sycl`) | `whisper.cpp` CPU | Matrix-heavy inference benefits from GPU; missing/unusable GPU falls back to CPU. |
| Audio clips, snapshots, parsing, SQLite/ZIP, HTTP | CPU (FFmpeg / Rust) | CPU | Short clips, small stills, and branch/I/O-heavy tasks are transfer-bound on GPU. |

Do not enable multiple Whisper GPU Cargo features in one binary (toolchains conflict); official Linux desktop builds enable `vulkan`, while `whisper-bench` pairs a CPU+Vulkan launcher with single-backend worker binaries. On Linux, FFmpeg/FFprobe drive all processing pipelines, whereas WebKitGTK uses GStreamer (`gst-plugins-{base,good,bad,ugly}`, `gst-libav`) solely for in-window playback.

---

## Native snapshot extraction and verification

`lib/vesta-flashcards/src/snapshot_batch.rs` groups up to 8 nearby WebP snapshot requests within an 8-second window for eligible sources (width `1280..=1920`, height `<= 1080`, CFR, H.264/HEVC, unambiguous video stream, `optimize_video = true`; CLI `--no-optimize` disables both batching and clip pre-transcoding) and extracts exact packet PTS frames under the shared worker semaphore. Reordered keyframes, ambiguous timestamps, VFR, or unsupported streams fall back conservatively to per-card seeks. Cancelled runs abort tasks and kill FFmpeg/FFprobe children; any failed media aborts deck publication. Keep [native A/B results](BENCHMARK_NATIVE.md) separate from [full-pipeline subs2srs benchmarks](BENCHMARK_REPORT.md).

---

## Headless CLI and MCP Tooling

Every domain engine runs standalone without Tauri via `vesta-cli` (or per-engine `vesta-*-cli` crates; see [`CLI.md`](CLI.md) and [`docs/modules/`](modules/README.md)):

```bash
cargo build --release -p vesta-cli -p vesta-mcp
./target/release/vesta-cli flashcards generate \
  --video episode01.mkv --target japanese.srt --native english.srt \
  --output ./my-deck --format apkg --audio-format opus --snapshot-format webp
```

`vesta-mcp` (and `vesta-cli mcp`) exposes 24 stdio JSON-RPC tools (`get_configuration`, `inspect_subtitles`, `list_embedded_subtitles`, `extract_embedded_subtitle`, `preview_video_frame`, `preview_flashcards`, `generate_flashcards`, `batch_generate_flashcards`, `export_dueling_subtitles`, `merge_apkg`, `sync_subtitles`, `autosync_subtitles`, `inspect_deck`, `edit_deck_cards`, `refine_flashcards`, `ankiconnect`, `translate_subtitles`, `transcribe_media`, `ocr_subtitles`, `manage_local_models`, `get_job`, `cancel_job`, `list_plugins`, `call_plugin`). See [`MCP.md`](MCP.md).
