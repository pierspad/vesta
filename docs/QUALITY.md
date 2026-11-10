# Quality and Release Verification

This manual defines the automated verification gates, regression test scopes, performance benchmarks, and installed-product smoke checks for Vesta. See [`ARCHITECTURE.md`](ARCHITECTURE.md) for system boundaries and [`library-integration.md`](library-integration.md) for downstream crate requirements.

---

## 1. Automated Verification Gates

Run the complete local verification gate from the repository root (after `npm ci` in `apps/srt-gui`):

```bash
python3 build-scripts/quality_check.py --desktop --smoke
```

| Command | Scope |
|---|---|
| `python3 build-scripts/quality_check.py` | Headless gate: Tauri version sync, release workflow tests, documentation/contract checks, Svelte/TypeScript type checking, AST pattern checks, Vitest suite, dev-CSS check, i18n audit, frontend production build, and Rust library/CLI tests (`--exclude vesta --exclude whisper-bench`). |
| `python3 build-scripts/quality_check.py --desktop` | Adds desktop native Rust tests (`vesta` Tauri package). |
| `python3 build-scripts/quality_check.py --desktop --smoke` | Adds the synthetic 20-second FFmpeg/APKG media generation smoke test (`quality_smoke.py`). |
| `python3 build-scripts/check_doc_contracts.py` | Verifies documentation links/anchors, per-subcommand CLI flags, MCP tools, Tauri IPC commands, AST-extracted Plugin SDK builder parity, and full Rust public API signature/field hashes against [`docs/contracts_manifest.json`](contracts_manifest.json). Enforced locally in `quality_check.py` and in CI (`ci.yml`, `release.yml`). |
| `cargo fmt --all --check` | Rust formatting check across the workspace. |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Strict lint gate across all workspace targets. |
| `cargo test --locked --workspace` | Full workspace Rust test suite (including `vesta` and `whisper-bench`). |

Publication in CI waits for the successful push workflow on the exact release commit, including workspace tests and strict Clippy; a missing, failed, or skipped run blocks `semantic-release`. `python3 build-scripts/test_release_workflows.py` validates release queueing, platform prerequisites, and tag/version agreement.

---

## 2. Rust Engine and CLI Regression Suites

### Atomic Output and Staged Export Persistence

```bash
cargo test -p srt-parser -p srt-transcribe -p srt-flashcards
```

- **Atomic file publication (`srt-parser::persistence`)**: Injects failed writers, rejects read-only targets, verifies symlink and permission preservation on Unix, preserves existing `.apkg` archives after media streaming failures, and prevents cancelled transcription runs from publishing partial files.
- **Staged TSV media publication (`srt-flashcards`)**: Verifies replacement via `.vesta-export-*` staging directories and `.<deck>.tsv.export.lock` locks, rollback after a failed TSV write or media swap, rejection of overlapping publishers, and cleanup after a failed initial export.
- **Scope boundary**: These are process-level failure tests. They do not simulate `SIGKILL` between filesystem renames or power loss across unjournaled filesystems.

### Audio Decoding, Hardware Probes, and Process Lifetimes

- **WAV decoding and FFmpeg extraction (`cargo test -p srt-transcribe audio::tests`)**: Covers integer/float WAV downmix, truncated and non-finite sample rejection (preventing silent audio shortening that would shift timestamps), normalization through 32-bit PCM, and real FFmpeg conversion/segmentation. FFmpeg failures report exit status and the last 64 KiB of stderr while draining pipes to prevent deadlocks.
- **Hardware encoder fallback (`cargo test --locked -p srt-flashcards`)**:
  - `software_retries_release_gpu_sessions_for_pending_clips`: Verifies that a failed hardware video encode releases its 3-session GPU permit before retrying on CPU while holding its general worker permit.
  - `generation_probes_hardware_once_and_keeps_the_original_source`: Verifies that video preparation reuses the encoder already probed by the generator rather than running duplicate probes within the same export.
- **Sync preview preparation (`cargo test --locked -p srt-sync -p srt-autosync -p srt-transcribe`)**: Covers 16 kHz mono PCM WAV (`pcm_s16le`) preview generation, cache reuse (`cache_modified >= source_modified`), failed preview cleanup, external cloud STT synchronization without a local Whisper model, and rejection of text-only responses for alignment.

### Synchronization and Autosync Invariants

- **Anchor constraints (`srt-autosync`)**: Reruns treat manual anchors as fixed constraints with a 15-second exclusion window around each manual point. A successful run replaces automatic anchors; an empty result preserves previous anchors.
- **Phrase onset refinement**: Uses Whisper token timestamps only when an unambiguous contiguous match of at least two normalized words is found; otherwise falls back to segment onset.
- **Authoritative history (`srt-sync::SyncEngine`)**: Retains up to 100 anchor-edit snapshots (including mapper state and checked checkpoints). Add, update, delete, and grouped auto-sync replacements are reversible; no-op edits preserve the redo stack and new edits clear it.

### Headless CLI and MCP Verification

- **Headless CI job**: Compiles and tests all library and CLI crates without Node, Tauri system libraries, or GPU SDKs, verifying the default CPU dependency graph independently of desktop feature unification.
- **CLI subprocess tests (`cargo test -p vesta-cli -p srt-flashcards-cli -p srt-translate-cli -p srt-transcribe-cli -p srt-extract-cli -p srt-sync-cli -p srt-autosync-cli -p srt-refine-cli`)**: Verify `--help` contracts, exit codes (`0` success, `1` runtime error, `2` usage error, `130` `SIGINT` cancellation), stdout/stderr separation, and preservation of existing output files on failure or `SIGINT`.
- **MCP protocol tests (`cargo test --locked -p vesta-mcp`)**: Connect a real stdio MCP client (`rmcp`), discover all 10 tool schemas (`get_configuration`, `inspect_subtitles`, `preview_flashcards`, `generate_flashcards`, `translate_subtitles`, `transcribe_media`, `get_job`, `cancel_job`, `list_plugins`, `call_plugin`), and verify path-root isolation, job cancellation, and server-side credential isolation.

### HTTP Contracts and Provider Links

`cargo test -p srt-transcribe` exercises loopback HTTP requests for OpenAI JSON transcription, Groq translation, Deepgram, and AssemblyAI upload/create/poll, plus authorization failures and model-discovery capability filtering.

To probe external documentation and model-discovery URLs online (optional, not run in offline CI):

```bash
python3 build-scripts/check_provider_links.py > provider-links.json
```

---

## 3. Frontend, Localization, and Static Checks

```bash
python3 build-scripts/check_tauri_versions.py
python3 build-scripts/test_tauri_versions.py
```

- **Static AST pattern check (`npm run check:patterns` in `apps/srt-gui`)**: Parses TypeScript and Svelte script blocks to reject async `forEach` callbacks, async `Promise` executors, and `debugger` statements.
- **Development CSS check (`npm run test:dev-css` in `apps/srt-gui`)**: Starts ephemeral Vite servers, requests component CSS before component JavaScript, verifies scoped style output, and shuts down cleanly.
- **Mounted and workflow regression tests (`npm test` in `apps/srt-gui`)**:
  - Persistent tab containers (`active` prop transitions without unmounting), keyboard shortcut scoping, and portal menu cleanup (`SpeechEngineSelector`, `SearchableSelect`, context menus).
  - Document undo/redo stacks across Synchronize (`sync_undo`/`sync_redo`), Revise (dual-column subtitle alignment and file swap history), Annotate (notes-only snapshots keyed by card ID), and Flashcards preview.
  - Clipboard integration via Tauri's `clipboard-manager` text API with fallback to `navigator.clipboard` in browser previews.
  - Command registry verification (`commandRegistry.test.js`), ensuring every literal IPC command invoked by the frontend is registered in `apps/srt-gui/src-tauri/src/main.rs`.

### Localization Policy and Git Hooks

Install the tracked Git hooks once per repository clone:

```bash
bash build-scripts/install-hooks.sh
```

- **Pre-commit / Pre-push hooks**: Audit staged source and locale snapshots before commit and verify frontend/native gates before push (`python3 build-scripts/test_git_hooks.py`).
- **Localization gate (`bash build-scripts/check-i18n.sh`)**: Verifies all 15 locale catalogs (`ar`, `de`, `en`, `es`, `fr`, `hi`, `it`, `ja`, `ko`, `nl`, `pl`, `pt`, `ru`, `tr`, `zh`). Blocks on missing keys, empty values, mismatched `{placeholder}` tokens, and untranslated English strings (`same_as_english`) outside the explicit technical-term allowlist.

---

## 4. Security and Dependency Advisories

CI runs a dedicated `security` job against both checked-in lockfiles:

```bash
cargo install cargo-audit --locked --version 0.22.2
cargo audit
(cd apps/srt-gui && npm audit)
```

- **Vendored `deepmerge`**: `apps/srt-gui/vendor/deepmerge` provides a checked-in MIT-licensed security fix for `GHSA-gcqx-w48m-w2h4` (preventing prototype poisoning via own data-property writes). `npm run test:merge` tests poisoning regressions and config merge compatibility.
- **Transitive GTK/Tauri advisories**: See [`MAINTENANCE.md`](MAINTENANCE.md#transitive-desktop-dependencies) for notes on `glib` (`RUSTSEC-2024-0429`) and D-Bus crate maintenance notices in the Tauri 2 / GTK3 dependency graph.

---

## 5. Performance and Hardware Smoke Checks

### Synthetic Flashcard Smoke Benchmark

```bash
cargo build --release -p srt-flashcards-cli
python3 build-scripts/quality_smoke.py --profile release --samples 5
```

Generates a 20-second synthetic video with 10 subtitle lines, runs `preview` and `generate --format apkg` with 2 workers, and verifies ZIP integrity, the numbered media manifest, nonempty media files, SQLite integrity, and card counts. For full-film comparative benchmarks and native snapshot batching equivalence checks (`480` A/B runs), see [`BENCHMARK_STEPS.md`](BENCHMARK_STEPS.md), [`BENCHMARK_NATIVE.md`](BENCHMARK_NATIVE.md), and [`BENCHMARK_REPORT.md`](BENCHMARK_REPORT.md).

### Opt-In Vulkan GPU Inference Smoke

On a machine with a supported Vulkan GPU and an installed Whisper model:

```bash
cargo build -p srt-transcribe-cli --features vulkan
python3 build-scripts/gpu_smoke.py --device 'RX 7800 XT' --vad --output-dir /tmp/vesta-gpu-check
```

Verifies CPU execution, Vulkan backend selection (`Vulkan0`), CPU fallback when Vulkan devices are hidden, Silero VAD integration, valid bounded SRT output, and `SIGINT` cancellation after the first decoded segment without publishing a partial output file.

### Prompt Editor Scroll Benchmark

`CodeEditor.svelte` mirrors native textarea scroll offsets into clipped highlight and line-number viewports without rebuilding DOM nodes or measuring line heights on scroll events. To benchmark scroll frame times in the native development webview:

```bash
cd apps/srt-gui
npm run tauri -- dev --config '{"build":{"devUrl":"http://localhost:5175/scripts/editor-scroll-benchmark.html"}}'
```

Recorded reference baselines are stored in `docs/performance/editor-scroll-2026-10-23.json`, `docs/performance/editor-scroll-plain-textarea-2026-10-23.json`, and `docs/performance/accelerator-review-2026-11-02.json`.

---

## 6. Release Smoke Matrix (Installed Builds)

Automated unit and mocked-IPC tests do not certify native OS dialogs, file associations, WebKitGTK/GStreamer media decoding, physical GPU drivers, or authenticated cloud accounts. Before publishing a release, validate installed bundles on target platforms (Linux, Windows x64 and ARM64, and macOS Apple Silicon):

1. **First-run setup and persistence**: Complete onboarding, restart twice, and verify that language, media, and export defaults persist. Verify that unreadable or corrupt configuration files surface an error instead of silently resetting user data (see [`SETUP_TESTING.md`](SETUP_TESTING.md)).
2. **Flashcard export and Anki import**: Generate audio + snapshots, TSV, APKG, video clips, and a multi-episode merged APKG. Import into Anki and verify audio/video playback, snapshots, and word lookup links.
3. **Embedded subtitle extraction**: List and extract embedded text tracks with language/title metadata; confirm bitmap tracks (PGS/VobSub) are marked as requiring OCR.
4. **Media preview and sync**: Preview audio and video containers (verifying 16 kHz mono PCM WAV preparation and cache reuse), test byte-range seeking, run manual anchor retiming with undo/redo, and run Whisper-assisted Auto-Sync.
5. **Local and cloud transcription**: Run local Whisper on CPU and GPU (verifying CPU fallback when GPU is disabled or unavailable), test interrupted model downloads, and test cloud STT providers with cancellation.
6. **Translation and deck annotation**: Run multi-tier LLM translation and `.apkg`/`.tsv` refinement, testing rate-limit failover, checkpoint resume, and cancellation.
7. **System integration and updater**: Verify opening output files/folders in the system file manager, `.apkg` association with Anki, and the in-app updater flow on supported package formats (Windows `.exe` on x64/ARM64, Linux `.deb` and `.rpm`; macOS `.dmg` bundles are updated via the release page).
