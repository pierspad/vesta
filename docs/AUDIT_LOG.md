# Vesta Audit Log & Continuous Improvement Tracker

This file records each audit & hardening wave performed against Vesta so future sessions can resume directly from verified progress without repeating completed discovery work.

See [`SCORECARD.md`](./SCORECARD.md) for the auto-generated live metrics dashboard and [`../scorecard/features.toml`](../scorecard/features.toml), [`../scorecard/reliability.toml`](../scorecard/reliability.toml), [`../scorecard/performance.toml`](../scorecard/performance.toml), and [`../scorecard/bugs.toml`](../scorecard/bugs.toml) for the structured backlog.

---

## Wave 1 — Baseline Full-Workspace Audit & High-Impact Fixes (`2026-11-19`)

- **Base Commit:** `443c967b` (`main`, with 28 pre-existing dirty files under `apps/srt-gui/` preserved untouched)
- **Swarm Configuration:** `"Vesta deep audit & scorecard"`
  - `audit-core-media` (`gemini-4-hydrogen-high`): core crates (`srt-apkg`, `srt-download`, `srt-parser`, `srt-flashcards`, `srt-extract`, `srt-ankiconnect`) — **19 verified findings** (`#10.1`–`#10.19`)
  - `audit-ai-sync-ocr` (`gemini-4-hydrogen-high`): AI & alignment engines (`srt-translate`, `srt-transcribe`, `srt-sync`, `srt-autosync`, `srt-refine`, `srt-ocr`) — **26 verified findings** (`#12.1`–`#12.26`)
  - `audit-cli-mcp-tauri` (`gemini-4-hydrogen-high`): `vesta-mcp`, 9 CLI crates, Tauri IPC & plugin host, `whisper-bench` — **23 verified findings** (`#8.1`–`#8.23`)
  - `audit-frontend` (`gemini-3.8-flash-high`): `apps/srt-gui/src/` tabs, stores, modals, panels, components — **20 verified findings** (`#14.1`–`#14.20`)
  - `audit-playwright-docs` (`gemini-3.8-flash-high`): Playwright suite, `vesta-mock.mjs`, docs/CI/build scripts — **18 verified findings** (`#15.1`–`#15.18`)
  - `verifier` (`gemini-4-hydrogen-high`): Independent cross-check of all **106 findings** against workspace files (**0 false positives**, 2 invariant-guarded downgrades, 3 minor citation corrections)
- **Scorecard System Established:**
  - Created `scorecard/{features,reliability,performance,bugs}.toml` tracking **50 features** across **12 areas**, **10 reliability controls**, **8 performance optimizations**, and **98 bug/hardening/doc/test findings** (`106` total findings).
  - Created [`../build-scripts/generate_scorecard.py`](../build-scripts/generate_scorecard.py) and [`../build-scripts/test_scorecard.py`](../build-scripts/test_scorecard.py), and integrated `--check` + unit tests into [`../build-scripts/quality_check.py`](../build-scripts/quality_check.py).

### Wave 1 Implementation & Verification Status

Completed (`audit_wave = "wave-1-complete"`): **33/106** total findings resolved (**32/98** in `scorecard/bugs.toml` fixed + **4** `in_progress`; **1/8** in `scorecard/performance.toml` fixed [`PERF-002`] + **1** `in_progress` [`PERF-008`]). Critical findings reduced from `1` to **`0`**, High findings reduced from `30` to **`13`** (`17` fixed), and Reliability controls improved to **`3 Pass / 5 Partial / 2 Fail`** (`REL-02`, `REL-04`, `REL-05` now `Pass`).

- **Wave 1A (Critical/High Security & Panics):** Fixed `AUDIT-001` (`plugins/mod.rs::uninstall` ID + registry + canonical path guard), `AUDIT-004`/`AUDIT-005`/`AUDIT-006` (`plugins/archive.rs` UTF-8 boundary truncation, streaming download/decompression caps, git arg validation), `AUDIT-008` (`cli/vesta-mcp/src/lib.rs` `call_plugin` ID guard, `kill_on_drop(true)`, 10s init timeout, bounded RPC lines, kill+wait cleanup), `AUDIT-009` (`commands/net.rs` link-local metadata IP block & cross-boundary redirect guard), and `AUDIT-010` (`apps/whisper-bench/src-tauri/src/workers.rs` fail-closed SHA-256 verification, shared timeout client, unwrap removal).
- **Wave 1B (Core/Engine Bugs & Performance):** Fixed `AUDIT-011` (`srt-flashcards/src/merge_apkg.rs` `did` stripping), `AUDIT-014` + partial `AUDIT-044` (`srt-flashcards/src/export_tsv.rs` orphan tag suppression & TSV tag sanitization), `AUDIT-016` (`srt-translate/src/lib.rs::compute_resume_position` + `cli/srt-translate-cli/tests/cli.rs` partial-checkpoint Ctrl-C fixture), `AUDIT-017` (`srt-translate/src/{language_info.rs,prompts.rs}` normalization + 5 new languages), `AUDIT-019` + partial `AUDIT-033` (`srt-autosync/src/lib.rs` sorting & `f64::total_cmp` NaN guards), and `PERF-002` (`srt-ocr/src/local.rs` `&model.initializers` borrow).
- **Wave 1C (Frontend, Scripts & Doc Drift):** Fixed `AUDIT-022`/`AUDIT-061`/`AUDIT-092` (`AlignTab.svelte` post-await race guard, empty save guard, dead imports), `AUDIT-062` (`TranslateTab.svelte` `contextLoading` race guard), `AUDIT-064` (`PathPreviewModal.svelte` `Escape` key propagation), partial `PERF-008` (`TranscriptionSegmentsPanel.svelte` keyed `{#each}`), `AUDIT-023` (`build-scripts/update_project_info.sh`), `AUDIT-095` (`commands/translate.rs` English comments/errors), and `AUDIT-070`–`AUDIT-073`/`AUDIT-093`–`AUDIT-094` (`.cursorrules`, `docs/CLI.md`, `cli/*/README.md`, `docs/modules/*.md`).
- **Wave 1D (Playwright Mock & E2E Expansion):** Fixed all 6 schema mismatches (`AUDIT-024`, `AUDIT-025`, `AUDIT-074`–`AUDIT-076`, `AUDIT-096`) and added all 33 missing IPC handlers (`AUDIT-026`, **106/106** IPC commands covered) in `apps/srt-gui/browser-tests/vesta-mock.mjs`; added [`extended-workflows.spec.mjs`](../apps/vesta-gui/browser-tests/extended-workflows.spec.mjs) and [`settings-and-modals.spec.mjs`](../apps/vesta-gui/browser-tests/settings-and-modals.spec.mjs) (**47/106** IPC names exercised across specs).
- **Verification Gates Passed (Post-Final-Edit, Node `v25.5.0` on `PATH`):**
  - `cargo fmt --all --check` and `cargo clippy --locked --workspace --all-targets -- -D warnings` (exit `0`, 0 warnings)
  - `python3 build-scripts/quality_check.py --desktop --browser --smoke` (exit `0`, all 19 stages: workflow safety, Tauri pairs, `check_doc_contracts.py` [`106` IPC, `12` MCP, `9` CLI, `23` widgets, `12` libs, `243` links], `generate_scorecard.py --check`, `test_scorecard.py` [`5` tests], `svelte-check` [`0` errors/`0` warnings], `test:merge` [`4`], `test:glob` [`3`], `check:patterns` [`4` tests, `0` findings], Vitest [`133/133` files, `639/639` tests], `test:dev-css`, `check:i18n` + missing translations, Vite build, `cargo test --locked --workspace --exclude whisper-bench` [`78` suites, `626` passed, `0` failed, `0` ignored], internal crate version check, Playwright [`30/30` passed across 5 spec files], `quality_smoke.py` FFmpeg/APKG [`10` notes, `20` media files])
  - `cargo test --locked -p whisper-bench` (`2` passed, `0` failed)
  - `python3 build-scripts/check_library_integration.py` (resolved `12` headless library crates + smoke binary pass)
  - `bash build-scripts/check-i18n.sh` (`1414` keys across `15` locales, `0` errors, `0` warnings)

---

## Wave 2 — Security, Reliability, Performance, Installed Release Notes & Test Hardening (`2026-11-21`)

- **Base Commit:** `cb3e6f40` (`dev`, above `v0.34.1`)
- **Installed-Version Release Details Bugfix (User Request):**
  - Updated [`updateCheckerStore.svelte.ts`](../apps/vesta-gui/src/lib/stores/updateCheckerStore.svelte.ts) (`installedReleaseNotes`, `loadInstalledReleaseNotes()` querying `/releases/tags/<installed>` without surfacing 404/offline errors as `updateError`) and [`NotificationInbox.svelte`](../apps/vesta-gui/src/lib/components/NotificationInbox.svelte) so the Release Details accordion always references the currently installed Vesta version (`updates.appVersionNum`, e.g. `0.33.0`) rather than a newer available release's notes. Added unit and mounted regression tests in [`updateCheckerStore.test.ts`](../apps/vesta-gui/src/lib/stores/updateCheckerStore.test.ts) and [`NotificationInbox.mounted.test.ts`](../apps/vesta-gui/src/lib/components/NotificationInbox.mounted.test.ts).
- **Security & Plugin/Media Server Hardening:** Fixed `AUDIT-002`, `AUDIT-003`, `AUDIT-031` (`main.rs`, `media_stream.rs`, `state.rs`: `MediaAllowlist` canonical path enforcement on `stream://` and HTTP endpoints, `getrandom` CSPRNG token, `subtle::ConstantTimeEq`, restricted `AllowOrigin`, fallible bind/serve error handling) and `AUDIT-007` (`plugins/runtime.rs`: env allowlist isolation, scoped permissions, bounded RPC stdout/stderr reads, `kill_on_drop(true)`, `try_wait()` in `is_alive()`, and lock-free `initialize`).
- **Flashcards, Extract, APKG & Transcribe Fixes:**
  - Fixed `AUDIT-012`, `AUDIT-030`, `AUDIT-044` (`core/srt-apkg` bounded `copy_entry_bounded` ZIP extraction + symlink skipping; `export_apkg.rs` atomic monotonic ID block + SHA-1 16-hex GUID + `0x1f` sanitization; `merge_apkg.rs` checked SQLite ID math + 512 MiB extraction bound).
  - Fixed `AUDIT-013`, `AUDIT-015`, `AUDIT-041`, `AUDIT-047` (`srt-flashcards/src/media.rs` `-map 0:v:0 -map 0:a?` multi-audio preservation, 120s `run_ffmpeg` timeout + `CancellationToken` `kill+wait`, cancellable probe/encoder detect, even H.264 clip dimensions; `dueling.rs` bare-relative parent fallback).
  - Fixed `AUDIT-032`, `AUDIT-037`, `AUDIT-042`, `AUDIT-045`, `AUDIT-046`, `AUDIT-084`, `AUDIT-087` (`core/srt-parser` & `srt-flashcards/src/{parser,filters,matcher,snapshot_batch}.rs`, `srt-extract/src/embedded.rs` `tempdir_in` staging, `commands/flashcards/commands.rs` `spawn_blocking` preview & atomic temp suffix).
  - Fixed `AUDIT-018`, `AUDIT-036`, `AUDIT-053`, `AUDIT-054`, `AUDIT-055` (`srt-transcribe` wordless Deepgram/AssemblyAI synthetic segments, AssemblyAI poll timeout underflow guard, case-insensitive custom exemption, Silero VAD `CancellationToken` & span clamping, `OperationGuard` on uninstall IPC).
- **Translate, Refine, OCR, Sync, Auto-Sync, Desktop Commands & CLIs:**
  - Fixed `AUDIT-035`, `AUDIT-048`, `AUDIT-049`, `AUDIT-050`, `AUDIT-051`, `AUDIT-052`, `AUDIT-043`, `AUDIT-086` (`srt-translate` legacy/JSON/Gemini parsing, `suggest_translation_context` `OperationGuard`, `srt-refine` blank defaults + TSV/APKG escaping, `srt-ankiconnect` 120s import timeout + error body + mock tests, `srt-refine-cli` lock-poison recovery).
  - Fixed `AUDIT-056`, partial `AUDIT-057` (`srt-ocr/src/{local,model,hardcoded}.rs` broadcast rank fix, 0-byte model check, child reap + `stderr_task.abort()`), `AUDIT-058`, `AUDIT-088` (`srt-sync` LNDS auto-anchor filter & `end.max(start)`), `AUDIT-033`, `AUDIT-059`, `AUDIT-060`, `AUDIT-089` (`srt-autosync` saturated matching bounds, target-sorted prior, silent attempt-0 WAV cache, span anchor exclusion, degenerate cloud segment clamp), `AUDIT-034`, `AUDIT-082`, `AUDIT-083`, `AUDIT-038`, `AUDIT-039`, `AUDIT-040` (`commands/{config,updates}.rs`, `whisper-bench`, `srt-{translate,transcribe,extract}-cli`).
- **Performance Optimizations (`PERF-003`..`006`, `PERF-008`) & Frontend/CI Fixes:**
  - Fixed `PERF-003` (`core/srt-download` shared `LazyLock` client), partial `PERF-004` (`spawn_blocking` `build_matched_lines`, bounded `snapshot_batch` pipe reads, `Arc<str>` codec strings), `PERF-005` (`hardcoded.rs` `spawn_blocking` ROI decode), `PERF-006` (`CleanText::chars` cache, stack Levenshtein `[usize; 128]`, threshold early-exit), and `PERF-008` (`TranscriptionSegmentsPanel.svelte` rAF scroll coalescing).
  - Fixed `AUDIT-020` (`ShortcutsTab.svelte` `active` prop listener cleanup + `ShortcutsTab.mounted.test.ts`), `AUDIT-021` (`aiStore.svelte.ts` + `Sidebar.svelte` Kill Switch active cancellation + `aiStore.test.ts`), `AUDIT-090` (`apiKeyEditorStore.svelte.ts`), `AUDIT-063` (`PluginRenderer.svelte` `writeClipboard`), `AUDIT-067` (RTL logical directional classes), partial `AUDIT-077` (added `uiModeStore`, `ocrModelsStore`, `autoSyncStore`, `cpuRamStore` Vitest suites), and partial `AUDIT-080` (`build-scripts/pre-push --locked`).
- **Scorecard Delta (Wave 1 → Wave 2):**
  - Bugs resolved: **`81/98` fixed, `6` in progress, `11` open** (`0` Critical, **`0` remaining High functional/security bugs** — all `3` remaining High items are frontend/Playwright test-coverage gaps).
  - Performance items: **`5/8` fixed, `1` in progress, `2` open** (`86/106` total findings fixed).
  - Reliability controls: **`8 Pass / 2 Partial / 0 Fail`** (`REL-01`, `REL-03`, `REL-06`, `REL-07`, `REL-08` promoted to `Pass`).
  - Feature matrix (`50` features): **`22 Verified / 28 Partial / 0 Issues Found`**.

---

## Wave 3 — Resume Guide for Next Session

Future agents continuing Vesta hardening should read [`SCORECARD.md`](./SCORECARD.md) and query remaining open / in-progress items in `scorecard/bugs.toml` and `scorecard/performance.toml`. Priority next targets after Wave 2:

1. **Test Coverage Expansion (Remaining High Items: `AUDIT-027`, `AUDIT-028`, `AUDIT-029`, `AUDIT-077`, `AUDIT-097`, `AUDIT-098`):** Add Playwright E2E breadth across untested IPC commands (`AUDIT-027`), add `TranscribeTab`/`TranslateTab`/full `RefineTab` mounted tests (`AUDIT-028`), cover remaining panels under `apps/srt-gui/src/lib/panels/` (`AUDIT-029`), finish `generationStore`/`ankiStore` Vitest suites (`AUDIT-077`), and close remaining Rust unit/integration test gaps (`AUDIT-097`, `AUDIT-098`).
2. **Remaining High/Medium Performance (`PERF-001`, `PERF-004` remainder, `PERF-007`):** Single-pass bitmap demux & aspect-preserving OCR scaling in `lib/srt-ocr/src/bitmap.rs` (`PERF-001`), parallel fallback snapshots (`PERF-004`), and `SubtitleListPanel.svelte` list virtualization (`PERF-007`).
3. **Frontend Accessibility, Styling & Parser Hygiene (`AUDIT-065`, `AUDIT-066`, `AUDIT-068`, `AUDIT-069`, `AUDIT-091`):** Replace native `<select>` in `PluginRenderer.svelte` (`AUDIT-065`), migrate hardcoded Tailwind palette classes to semantic theme tokens (`AUDIT-066`, `AUDIT-091`), add modal Tab focus traps & focus restoration (`AUDIT-068`), and eliminate duplicated JS SRT parser/serializer in `AlignTab.svelte` (`AUDIT-069`).
4. **Hardcoded Frame-0 PTS & Quality Gate / CI Expansion (`AUDIT-057`, `AUDIT-078`–`081`, `AUDIT-085`):** Handle non-zero first-frame PTS in `hardcoded.rs` (`AUDIT-057`), tighten `browser_ipc_contract.py` argument parity (`AUDIT-078`), align `quality_check.py`/`pre-push`/CI with `check_library_integration.py`, `fmt`, `clippy`, and Playwright E2E (`AUDIT-079`, `080`, `081`), and close remaining `vesta-mcp`/`srt-flashcards-cli` parity items (`AUDIT-085`).

---

## Wave 3 — Performance, Frontend Hygiene, Test Coverage & CI Parity (`2026-11-22`)

### Scope & Outcome

Resolved all remaining Wave 3 bugs, performance items, UI/accessibility improvements, and test/CI gaps across `lib/*`, `apps/srt-gui`, `build-scripts/`, and `.github/workflows/ci.yml`, intentionally deferring only `AUDIT-085` (optional `vesta-mcp` / `srt-flashcards-cli` feature-parity expansion).

### Key Changes Landed

- **Rust Performance & Test Coverage (`PERF-001`, `PERF-004`, `AUDIT-057`, `AUDIT-097`, `AUDIT-098`):**
  - `lib/srt-ocr/src/bitmap.rs`: Added `FfprobePacketProbe` / `parse_ffprobe_packet_probe` stream dimension probing (`1..=4096`) preserving non-16:9 bitmap aspect ratios, bounded parallel chunked FFmpeg cue rendering (`Semaphore(2..=6)` + `JoinSet`) with `spawn_blocking` PNG cropping, `ETXTBSY` spawn retry in `run_cancellable_output`, and PGS RLE `0x40`/`0xC0` & mock parallel render unit tests.
  - `lib/srt-ocr/src/hardcoded.rs`: Added `compute_stream_start_offset_ms`, probed stream/format `start_time`, prepended `setpts=PTS-STARTPTS,` in `-vf`, offset cue timestamps, added `ETXTBSY` spawn retry, and unit tests.
  - `lib/srt-flashcards/src/{snapshot_batch.rs,media.rs,merge_apkg.rs}`: Converted `snapshot_batch::batch()` file operations to `tokio::fs` and bounded-parallelized fallback `extract_snapshot` tasks (`JoinSet` + `Semaphore(4)`), added `spawn_retry_etxtbsy` and runtime tests for multi-audio track optimization and multi-deck `merge_apkg`.
  - `lib/srt-ocr/src/local.rs`, `lib/srt-translate/src/lib.rs`, `lib/srt-transcribe/src/transcribe.rs`: Added unit tests covering ONNX `eval_layer_norm`/`eval_pad("edge")`, `repair_missing_tiered`, and `vad_speech_spans` cancellation/missing-model handling.
- **Typed Align SRT IPC, Virtualization, Accessibility & Theme Tokens (`AUDIT-065`, `066`, `068`, `069`, `091`, `PERF-007`):**
  - `apps/srt-gui/src-tauri/src/commands/info.rs` & `main.rs`: Added `SubtitleEntry`, async `parse_subtitle_file`, and `save_subtitle_file` backed by `core/srt-parser` (`108` total registered IPC commands).
  - `apps/srt-gui/src/lib/tabs/AlignTab.svelte`: Migrated from custom JS regex SRT parsing / `writeTextFile` to `parse_subtitle_file` and `save_subtitle_file`.
  - `apps/srt-gui/src/lib/panels/SubtitleListPanel.svelte` & `SubtitleListPanel.mounted.test.ts`: Added grid-aware virtual windowing (`> 100` cues, `ROW_STRIDE_PX = 96`, `VIRTUAL_OVERSCAN_ROWS = 4`), semantic CSS theme variables, and a 300-item scroll test.
  - `apps/srt-gui/src/lib/actions/focusTrap.ts` & modals (`SupportModal.svelte`, `EpisodeMediaSettingsModal.svelte`, `PathPreviewModal.svelte`): Added reusable Tab focus trapping with trigger focus restoration and `focusTrap.test.ts`.
  - `PluginRenderer.svelte`, `ExperimentalTab.svelte`, `TranslateTab.svelte`, `TranslationPreviewPanel.svelte`, `App.svelte`: Replaced native `<select>` with `<SearchableSelect>`, wired `active` prop on `ExperimentalTab`, and migrated hardcoded palette classes to semantic theme tokens.
- **Frontend & Playwright Test Coverage (`AUDIT-027`, `AUDIT-028`, `AUDIT-029`, `AUDIT-077`):**
  - Added `generationStore.test.ts` and `ankiStore.test.ts` (`apps/srt-gui/src/lib/stores/`).
  - Added `TranscribeTab.mounted.test.ts`, `TranslateTab.mounted.test.ts`, and `RefineTab.mounted.test.ts` (`apps/srt-gui/src/lib/tabs/`).
  - Added `FlashcardsPanels.mounted.test.ts`, `WorkflowPanels.mounted.test.ts`, and `SettingsPanels.mounted.test.ts` covering all 15 remaining panels (`148` Vitest files / `661` cases).
  - Added `apps/srt-gui/browser-tests/ipc-coverage.spec.mjs` exercising remaining registered IPC commands (`108/108` mocked, `108/108` referenced across `6` Playwright specs / `28` `test()` cases).
- **Quality Gates, Hooks & CI (`AUDIT-078`, `AUDIT-079`, `AUDIT-080`, `AUDIT-081`):**
  - Tightened `build-scripts/browser_ipc_contract.py --check` (`vesta-mock.mjs` coverage + static literal `invoke(...)` argument key parity) + `test_browser_ipc_contract.py`.
  - Wired `browser_ipc_contract.py --check`, `test_browser_ipc_contract.py`, `check_library_integration.py`, `cargo fmt --all --check`, and `cargo clippy --locked --workspace --all-targets -- -D warnings` into `build-scripts/quality_check.py`.
  - Added `CARGO_SCOPE_ARGS` with Linux `pkg-config --exists webkit2gtk-4.1` detection in `build-scripts/pre-push` (+ regression test in `test_git_hooks.py`) and added the `browser` Playwright E2E job plus scorecard/contract gates in `.github/workflows/ci.yml`.
- **Scorecard Delta (Wave 2 → Wave 3):**
  - Bugs resolved: **`97/98` fixed, `1` deferred (`AUDIT-085`, Low severity), `0` open** (`0` Critical, `0` High, `0` Medium).
  - Performance items: **`8/8` fixed (`105/106` total findings resolved)**.
  - Reliability controls: **`10 Pass / 0 Partial / 0 Fail`** (`REL-09` and `REL-10` promoted to `Pass`).
  - Feature matrix (`50` features): **`38 Verified / 12 Partial / 0 Issues Found`** (remaining partial items primarily reflect intentionally deferred `AUDIT-085` CLI/MCP parity or unimplemented Playwright/CLI/MCP surface expansion).
