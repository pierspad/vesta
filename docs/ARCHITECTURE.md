# Vesta Architecture

Vesta is a modular Cargo workspace layered from foundational formats to GUI-agnostic feature engines and desktop adapters. The principal workflows have matching headless CLIs; smaller support crates are consumed directly by those workflows.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  apps/vesta-gui              Tauri v2 + Svelte 5 desktop application           │
│  apps/whisper-bench        Whisper.cpp & VAD benchmarking utility           │
│  cli/vesta-cli             Unified CLI dispatcher (vesta-cli <command>)         │
│  cli/vesta-mcp             Model Context Protocol (stdio MCP server)        │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │ depends on
┌──────────────────────────────────────▼──────────────────────────────────────┐
│  lib/ (feature engines — GUI-agnostic, cancellation & progress callbacks)    │
│  • vesta-flashcards   (Anki deck compiler)         ──► cli/vesta-flashcards-cli │
│  • vesta-translate    (LLM multi-tier translator)  ──► cli/vesta-translate-cli  │
│  • vesta-transcribe   (Whisper & VAD transcription)──► cli/vesta-transcribe-cli │
│  • vesta-autosync     (VAD speech auto-aligner)    ──► cli/vesta-autosync-cli   │
│  • vesta-extract      (Media & subtitle extractor) ──► cli/vesta-extract-cli    │
│  • vesta-ocr          (Bitmap & ROI OCR engine)    ──► cli/vesta-ocr-cli        │
│  • vesta-refine       (LLM deck annotation)       ──► cli/vesta-refine-cli     │
│  • vesta-sync         (Anchor-based retiming)     ──► cli/vesta-sync-cli       │
│  • vesta-ankiconnect  (AnkiConnect HTTP client)                               │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │ depends on
┌──────────────────────────────────────▼──────────────────────────────────────┐
│  core/ (foundational utilities)                                             │
│  • vesta-parser       (SRT parser/writer with automatic charset detection)    │
│  • vesta-apkg         (Anki .apkg ZIP archive builder & extractor)            │
│  • vesta-download     (Cancellable downloads & atomic publication)            │
└─────────────────────────────────────────────────────────────────────────────┘
```

The `core/` layer also contains `vesta-download`, shared by callers that need
cancellable resource downloads and atomic publication.

---

## Architectural Layers

| Layer | Crates | Responsibilities & Design Rules |
|---|---|---|
| **core** | `vesta-parser`, `vesta-apkg`, `vesta-download` | Foundational primitives. Minimal external dependencies, zero knowledge of higher engines or GUI. Auto-charset detection via `chardetng` + `encoding_rs`, lossless timing arithmetic, direct ZIP/SQLite archive serialization, and cancellable atomic resource downloads. |
| **lib** | `vesta-flashcards`, `vesta-translate`, `vesta-transcribe`, `vesta-autosync`, `vesta-extract`, `vesta-ocr`, `vesta-refine`, `vesta-sync`, `vesta-ankiconnect` | Self-contained domain engines. **Zero GUI/Tauri coupling**. Long-running tasks accept a `tokio_util::sync::CancellationToken` and report progress through callbacks. External tools and heavy dependencies (FFmpeg processes, `whisper-rs`, `rusqlite`, `reqwest`) are encapsulated here. |
| **cli** | `vesta-cli`, `vesta-mcp`, and `vesta-*-cli` (`vesta-flashcards`, `vesta-translate`, `vesta-transcribe`, `vesta-autosync`, `vesta-extract`, `vesta-ocr`, `vesta-sync`, `vesta-refine`) | Headless terminal frontends powered by `clap`, plus the `vesta-cli` unified entry point and `vesta-mcp` stdio MCP server (24 tools). Thin adapters over corresponding `lib/` engines for scripting, batch processing, CI, AI assistants, and benchmarking. |
| **apps** | `apps/vesta-gui` (`vesta`), `apps/whisper-bench` | Desktop composition roots. Svelte owns presentation/state; Tauri commands adapt IPC to `lib/` crates and own process-level services such as local media streaming, declarative plugin subprocess management (`src-tauri/src/plugins`), dialogs, and window integration. |

---

## Key Pipelines and Data Flow

### 1. Flashcard Generation Pipeline (`vesta-flashcards`)

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
                                     │ • Snapshots: bounded WebP PTS batches or individual AVIF / JPEG + crop
                                     │ • Video: H.264 / MPEG-4 snippets
                                     ▼
                               Export Output
                                     ├── TSV Deck (Anki-importable with media filenames)
                                     └── Native `.apkg` Package (`vesta-apkg` + SQLite collection)
```

### 2. Speech-to-Text Transcription (`vesta-transcribe`)

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

### 3. LLM Translation & Refinement (`vesta-translate`, `vesta-refine`)

```
Tiered Configuration (`TierEntry`: Provider + Model + Rate Limits)
        │
        ▼
Failover Orchestrator (Tier 0 ─► Tier 1 ─► Tier 2 ─► ...)
        │ Dispatches requests in round-robin across active endpoints in the current Tier.
        │ Automatically fails over to the next tier if quota/RPM/rate-limits are exhausted.
        ▼
LLM Providers (Google Gemini, Anthropic Claude, Groq, OpenAI, Mistral, OpenRouter, NVIDIA NIM, Ollama / Local)
        │
        ▼
Translation & Refine Engine
        ├── Subtitle Translation: Translates dialogue in context-aware batches.
        ├── Dialogue Merging: Reassembles split phrases into natural sentences.
        └── Deck Enrichment: Adds definitions, grammatical breakdowns, and target-language notes.
```

### 4. Automatic Subtitle Synchronization (`vesta-autosync`, `vesta-sync`)

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
Retiming Calculation (`vesta-sync`)
        │ Computes linear / affine time-stretch and global offset transformations
        ▼
Aligned Output Subtitle (.srt)
```

---

## Frontend execution boundaries

Translation provider errors expose safe typed categories and HTTP status codes.
Optional retry observers belong to callers: CLI adapters write numeric metadata
to stderr; desktop `provider_diagnostics` emits `provider-retry` events for the
support recorder. The engine neither initializes a logger nor imports Tauri.
This separates diagnostic policy from retries and preserves embedded consumers.

The refine CLI uses the same tiered annotation and card serialization as the
desktop. Its adapter owns Ctrl+C, output-format validation and complete/partial
publication policy; the engine owns batching, failover and cancellation cleanup.

The desktop tabs compose UI and event lifetimes. The modules in
`apps/vesta-gui/src/lib/workflows` own flashcard execution/aggregation/merge,
bounded file discovery, transcription failover and model/backend/VAD discovery.
Their injected IPC functions and callbacks support headless regression tests.
Reactive discovery state lives in `TranscriptionResources`; live transcript
rendering lives in `TranscriptionSegmentsPanel`. Formatting helpers are pure.
This frontend orchestration chooses and sequences native commands; the Rust
feature libraries still perform the underlying processing.

### Tab lifecycle and component reuse

`App.svelte` dynamically imports tabs and keeps loaded tabs mounted in hidden
containers. Idle preloading also mounts tabs before their first visit. Changing
tabs changes the `active` prop; it does not call `onDestroy`. Visibility-sensitive
polling and shortcuts must observe `active`. Operation ownership is independent
of visibility so users can navigate while a job runs. Destruction remains the
boundary for releasing component-owned listeners and invalidating pending UI work.

Shared presentation belongs in components such as `TierList`, `FooterActions`,
`PathPickerField` and the media panels. Feature-specific sequencing belongs in
`workflows/`, with explicit inputs and injected native services where practical.
Extract a complete responsibility rather than duplicating an existing component
or introducing one controller for unrelated operation lifecycles. Regression
tests should exercise stale responses, cancellation and retained results; mounted
tests additionally verify that the view connects events and cleanup correctly.

For flashcards preview, `FlashcardPreviewController` owns the tab-facing guards,
configuration, logs/errors and disposal. `PreviewStore` owns dialog data and
request validity. Destroying the tab closes its preview and prevents late
callbacks from updating the tab. Hiding the tab is a separate lifecycle event.

Flashcards passes visibility to its preview modal. Deactivation closes the
preview, invalidates pending UI requests and releases its media element, while
retaining subtitle edits for generation. Generation ownership remains independent
of this dialog. Keyboard handlers respect consumed events, IME composition and
editable targets; inactive tabs do not process global shortcuts.

Native window resize registration is owned by `manageWindowSize`. It checks
disposal after asynchronous setup and immediately releases registrations that
resolve after App teardown. `createTabLoader` deduplicates lazy imports and exposes
failures for retry; mounted tab state remains owned by App's persistent containers.

## Runtime Boundaries

The desktop UI never executes FFmpeg, Whisper or SQLite directly. Provider
discovery, readiness checks and update metadata use the sandboxed backend proxy
command `http_fetch` (`apps/vesta-gui/src-tauri/src/commands/net.rs`) rather
than ambient browser requests: it enforces HTTPS/allowlisted endpoints unless a
custom provider URL is explicitly configured, blocks private/link-local metadata
IP ranges (`169.254.0.0/16`, `fe80::/10`, loopback/private SSRF guards), forbids
cross-origin or cross-boundary redirects (`Policy::custom`), and caps streamed
responses to 8 MiB even when Content-Length is absent. Translation,
transcription, OCR, and deck annotation execution always runs in the Rust
domain engines (`lib/`).
Svelte invokes typed Tauri commands; those commands translate persisted/UI
configuration into library structs, hold cancellation state, and forward
progress events. Domain behavior stays in `lib/`, so the same pipeline is used
by the CLI and benchmark harness.

Media preview is the one process-level service owned by the desktop adapter
(`apps/vesta-gui/src-tauri/src/media_stream.rs` and `stream://` custom URI
protocol): an ephemeral loopback HTTP server (`127.0.0.1:0` via
`bind_media_listener`) serves byte-range requests (`serve_media` /
`stream_file`) for WebKitGTK. Access is gated by a per-process CSPRNG bearer
token (`generate_media_token` verified in constant time via
`subtle::ConstantTimeEq`), restricted CORS `AllowOrigin`, bounded initial/range
reads, and `MediaAllowlist` canonical path checks that reject empty paths,
relative paths, symlinks, and unapproved files.
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

`video_hw_accel = "auto"` performs a test encode before selecting a
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
- VA-API/NVENC/QSV: selected at runtime by FFmpeg when a hardware probe succeeds.

On Arch Linux, preview support requires `gstreamer`, `gst-plugins-base`,
`gst-plugins-good`, `gst-plugins-bad`, `gst-plugins-ugly`, and `gst-libav`.
The AUR `PKGBUILD` declares these runtime dependencies.

---

## Headless CLI Tooling

Every key engine can be built and run standalone without Tauri. In addition to individual standalone CLI binaries, Vesta provides `vesta-cli` as a unified entry point (`vesta-cli <operation>`) and `vesta-mcp` as a stdio Model Context Protocol (MCP) server exposing 24 tools for MCP clients:

```bash
# Build the unified CLI and MCP server
cargo build --release -p vesta-cli
cargo build --release -p vesta-mcp

# Build standalone CLIs
cargo build --release -p vesta-flashcards-cli
cargo build --release -p vesta-transcribe-cli
cargo build --release -p vesta-translate-cli
cargo build --release -p vesta-autosync-cli
cargo build --release -p vesta-extract-cli
cargo build --release -p vesta-ocr-cli
cargo build --release -p vesta-sync-cli
cargo build --release -p vesta-refine-cli

# Run flashcard generation via unified CLI (or standalone binary)
./target/release/vesta-cli flashcards \
  generate --video episode01.mkv \
  --target japanese.srt \
  --native english.srt \
  --output ./my-deck \
  --format apkg \
  --audio-format opus \
  --snapshot-format webp
```

The `vesta-mcp` server provides 24 discoverable tools (`get_configuration`, `inspect_subtitles`, `list_embedded_subtitles`, `extract_embedded_subtitle`, `preview_video_frame`, `preview_flashcards`, `generate_flashcards`, `batch_generate_flashcards`, `export_dueling_subtitles`, `merge_apkg`, `sync_subtitles`, `autosync_subtitles`, `inspect_deck`, `edit_deck_cards`, `refine_flashcards`, `ankiconnect`, `translate_subtitles`, `transcribe_media`, `ocr_subtitles`, `manage_local_models`, `get_job`, `cancel_job`, `list_plugins`, `call_plugin`) over stdio JSON-RPC without requiring GUI automation. See [`docs/MCP.md`](MCP.md) for complete details.

For module-specific documentation and integration examples, see [`docs/modules/`](modules/README.md).

## Configuration, embedded subtitles, and output files

`vestaConfig.ts` hydrates a string-keyed cache before dynamically importing the
Svelte app/stores, atomically migrating legacy `localStorage` entries together
with the completion flag via `replaceAll()`. Mutations are serialized over IPC.
Setup awaits a durable snapshot via `replaceAll()` before reloading: an in-memory
flag is insufficient to mark onboarding complete. Rust serializes disk access under
`ConfigState`, writes a unique temporary file, syncs it, and publishes it atomically.
Corrupted JSON is quarantined to a `.corrupt.<timestamp>` backup (blocking reset
if the backup cannot be preserved) and surfaced to the user with the backup path;
unreadable configuration fails startup instead of being silently replaced by defaults.
API keys are stored in this configuration file; it is not an encrypted keychain.

Embedded subtitle discovery/extraction lives in `vesta-extract::embedded` and uses
FFprobe/FFmpeg. Text tracks convert directly to SRT; bitmap tracks (PGS, VobSub,
DVB, XSUB) and burned-in video ROI regions (`VideoRoi` in `vesta-ocr::hardcoded`)
are extracted and converted via `vesta-ocr` using offline pure-Rust PaddleOCR v4
ONNX models (`LocalOcrBackend`) and/or configured Vision LLM tiers
(`VisionLlmBackend` via `CompositeOcrBackend`, preferring dedicated ITT tiers in
Settings → ITT (`vesta-itt-tiers`) before falling back to LLM tiers; Vision
LLM tiers remain subject to the AI Kill Switch, while downloaded local OCR
models keep offline OCR available). The desktop Extract tab saves independent
SRT files, keeping the existing workflows simple.

Fresh per-episode APKG exports can be merged by `vesta-flashcards::merge_apkg`.
The frontend writes episodes to distinct directories; the merger remaps note/card
IDs, combines metadata/media, rejects conflicts, and publishes a complete archive.
It is intended for fresh Vesta exports, not scheduled Anki collections.
`open_output_path` opens the saved file or its parent with the system association.

## Preparation progress and quality

Audio/snapshot-only workflows use the original media directly. A full-film H.264
intermediate is reserved for video-clip workflows with `optimize_video` enabled;
it can amortize repeated seeks for heavy codecs/high resolutions, but is lossy
and does not improve source quality. Modest H.264 downscaling alone does not
justify converting the whole film, even with a hardware encoder.

FFmpeg preparation emits `out_time_us` progress relative to the source duration.
The UI displays preparation percentage separately from the overall pipeline
percentage (12–15%). This is a phase completion measure, not an estimate of
elapsed time for the entire job. Cancellation kills the active child; a stalled
progress stream is bounded and a failed attempt falls back to another encoder.

Each native transcription request runs one engine. The desktop
`runTranscription` workflow attempts ready endpoints sequentially across configured
tiers; it does not use the translation pool for concurrent load balancing.
Cancellation prevents retries and publication of late results. OpenAI
Whisper returns segment timing; GPT-4o transcription uses JSON and a whole-chunk
time interval. It cannot provide precise line timing in the current integration.
Cloud requests/polling are cancellable by dropping the selected future; remote
AssemblyAI jobs may continue after local cancellation. See [QUALITY](QUALITY.md).


## Playback preparation, setup and desktop maintenance

`sync_prepare_media_for_playback` returns natively playable audio paths (`mp3`,
`wav`, `ogg`, `m4a`, `aac`, `opus`, `flac`) directly. Other containers (such as
MKV or MP4) are prepared by `vesta-sync::playback`: reuse a fresh nonempty cache
entry (`cache_modified >= source_modified`), or transcode the audio stream via
FFmpeg into uncompressed 16 kHz mono PCM WAV (`pcm_s16le`, `{hash}.wav`) via an
atomically published unique temporary sibling. This preview policy avoids video decoding and
compressed-audio encoding overhead, and does not change flashcard export
settings. Fresh cache hits bypass the preparation lock. A process-local mutex
serializes conversions and rechecks the cache after acquisition; temporary files
become cache entries only after a successful FFmpeg run with nonempty output.
Failed conversion or publication cleans the temporary file and preserves the
previous cache entry. Each FFmpeg child has a
five-minute timeout. This path has no caller cancellation token.

The loopback media server authenticates requests with a session token and supports
byte ranges. WebKitGTK/GStreamer still needs a decoder for the prepared media.
A cached path is not proof that a particular machine can decode that codec.

First-run setup is shown for a new installation or an explicit restart request.
`FirstRunSetupModal` collects language/export/audio/transcription defaults and
uses `firstRunSetup.ts` to plan sequential downloads and `services/setup.ts` to
discover/install resources. Quick Setup chooses Small + Silero; Custom Setup
preserves explicit model/VAD/transcription choices. Both include missing media
tools and language fonts, skip installed resources on retry, and verify readiness
before committing preferences and the chosen model in the local tier.
`setup_media_status` checks both FFmpeg and FFprobe and makes a bounded HEAD
request for the archive size when the tools are missing; unavailable sizes are
shown explicitly rather than guessed.
Its preview mode skips persistence, global language changes and downloads.
After persisted setup, a one-shot navigation flag preserves the requested
first activity (flashcards, transcription or subtitle extraction) across the
reload that reconstructs the stores.
For repeatable setup, installer and internal-update checks on installed builds,
see [setup testing](SETUP_TESTING.md).

Support logging, external plugin hosting, MCP CLI path discovery, and installer updates are desktop services, not feature engines:
`commands/support_logs.rs` owns log files; `commands/info.rs` exposes `get_app_info`, `get_system_diagnostics`, and `get_mcp_cli_info` (resolving the bundled sibling or `PATH` `vesta-cli` binary plus default media/export roots for the Settings MCP configuration card); `commands/plugins.rs` and `src-tauri/src/plugins/` manage external JSON-RPC 2.0 plugin processes (`plugin_inspect`, `plugin_install`, `plugin_list`, `plugin_enable`, `plugin_disable`, `plugin_uninstall`, `plugin_update`, `plugin_get_settings`, `plugin_save_settings`, `plugin_render`, `plugin_dispatch_event`; see [PLUGINS.md](PLUGINS.md)); `commands/updates.rs` chooses an official
stable-release installer for the installed channel/architecture, verifies its
digest, and opens it. Package-manager channels remain managed externally. Release
creation and package builds are separate GitHub workflows; a published tag alone
is not evidence that all installable assets are ready.


## Export format boundaries

`buildFlashcardConfig` maps media-generation flags and enabled note-type fields
independently. `export_tsv` writes media references in separate ordered columns;
it does not embed media or install Anki templates. `export_apkg` embeds media and
model data. Snapshot/video coexistence is valid in the TSV serializer; the GUI's
APKG exclusivity is a presentation policy rather than a schema limitation.

`runFlashcardSeries` captures the effective export format at run start and applies
single-package mode only to APKG. TSV output uses separate episode naming/output
and preserves its first result path. Format-specific disabled controls should be
hidden when irrelevant rather than influencing another export format's payload.

## Native snapshot extraction and verification

`lib/vesta-flashcards/src/snapshot_batch.rs` is an internal Rust module used by
both desktop and CLI generation. It groups at most eight nearby WebP requests
in an eight-second window, probes eligible H.264/HEVC HD streams and selects
actual packet PTS. Conservative fallbacks preserve original seek semantics at
reordered keyframes, missing/ambiguous timestamps, VFR and unsupported sources.
Batch extraction and retries obey the same semaphore as other media operations.
Each card retains a progress/error result; cancellation aborts and drains tasks
and terminates direct FFmpeg/FFprobe children. Failed media prevent deck export.
Temporary images are validated before copying and have generation-scoped
lifetimes; there is no persistent cache or Python adapter in the runtime.

The eligibility boundary is width 1280–1920, height ≤1080, CFR, WebP and an
unambiguous video stream. `FlashcardConfig.optimize_video` also gates batching;
CLI `--no-optimize` disables both batching and optional clip source preparation.
The latter can be lossy and has a separate quality contract. Default concurrency
reserves capacity; explicit worker requests are bounded by available cores.
[Native A/B evidence](BENCHMARK_NATIVE.md) and [the product comparison](BENCHMARK_REPORT.md)
measure different scopes and must not be pooled.


### Rust operation and I/O ownership

The desktop command adapters reserve operation slots using `OperationGuard`.
Cancellation requests stop work without releasing its slot early; normal return,
error and future drop release the slot through RAII. Auto-sync reserves both
sync and transcription resources with a shared cancellation token.

`core/vesta-download` owns HTTP streaming, unique temporary files, cache checks,
publication and cancellation. Font and model catalogs retain their domain policy.
Translation APIs share one worker engine, with tier selection in `scheduler.rs`;
`JoinSet` owns the workers and propagates failures. `vesta-refine::cards` owns TSV
and APKG persistence separately from LLM orchestration. Desktop `media_stream`
owns range serving independently of application startup and command registration.

## Frontend action lifecycles

`AsyncOperation` owns the frontend lock through asynchronous preparation, native
work and cancellation. It does not release a run until both work and an
in-flight cancellation have settled, and only the owning run can finish it.
Whisper inference, Auto-Sync and model mutation additionally share the local
Whisper resource lock. UI controls read these locks synchronously. The native
operation guards remain authoritative for process and file lifecycles.

`LatestRequest` invalidates superseded file discovery, previews and playback
preparation, including same-input requests and dialog close/reopen. Native IPC
promises own translation completion; globally broadcast completion events must
not release a frontend lock. Preview polling has at most one request in flight.

The snackbar store queues concurrent notifications, coalesces duplicates,
prioritizes queued errors and bounds the backlog. Per-severity minimum durations
apply to every caller, including legacy shorter overrides. Pointer, keyboard and
visibility pauses share one timer; old close handlers cannot dismiss a newer
notification. Notifications are transient, so persistent errors remain in the
existing panels. Clipboard denial follows the shared notification policy and
never triggers success feedback or destructive cut behavior.

Background preference writes retain their in-memory session values on failure
and report a localized error through the same snackbar surface. Explicit
`flush`/`replaceAll` calls still reject so setup/import flows cannot claim durable
success or reload after a failed write. Notification imports are lazy to avoid
a config/i18n initialization cycle.

## Atomic output publication

`vesta-parser::persistence` owns temporary-sibling publication for subtitle and
companion files. Parser saves, manual-sync session saves, transcription SRT,
flashcard TSV/APKG exports and CLI extraction outputs use it. The streaming
callback must finish and flush buffered/archive writers before publication;
the helper then synchronizes the temporary file and replaces the destination.
Errors before replacement preserve the previous file and remove the temporary.
Transcription also checks cancellation immediately before publication begins.

Existing symlinks are resolved and preserved; dangling links and read-only
destinations fail. Standard permission bits are retained, but ownership/extended
ACLs are not copied. Concurrent writers are last-successful-writer-wins, and
directory durability across power loss is not guaranteed.

TSV media are generated in a fresh `.vesta-export-*` staging directory on the
destination filesystem. The publisher takes an OS lock on the stable hidden
`.<deck>.tsv.export.lock` file, moves existing media into staging as
`previous.media`, swaps in the complete new media, and atomically publishes the
TSV. A failed swap or TSV write restores the previous media. If restoration
itself fails, staging is retained and the error reports its recovery location.
Media-directory symlinks and TSV symlinks into the media tree are rejected.

This prevents extraction errors and pre-publication cancellation from destroying
the previous export. It is not a crash-atomic two-file transaction: termination
between the media swap and TSV replacement can leave mismatched TSV/media and
`previous.media` in staging. There is no automatic crash recovery. Do not delete
such staging before inspecting/restoring the previous media. Keep the lock file
in place; the OS releases its lock when the process exits. Publication is
synchronous and completes once begun. Staging temporarily requires space for
both generations. Installed-platform validation remains necessary.
