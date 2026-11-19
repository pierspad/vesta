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
- **Wave 1D (Playwright Mock & E2E Expansion):** Fixed all 6 schema mismatches (`AUDIT-024`, `AUDIT-025`, `AUDIT-074`–`AUDIT-076`, `AUDIT-096`) and added all 33 missing IPC handlers (`AUDIT-026`, **106/106** IPC commands covered) in `apps/srt-gui/browser-tests/vesta-mock.mjs`; added [`extended-workflows.spec.mjs`](../apps/srt-gui/browser-tests/extended-workflows.spec.mjs) and [`settings-and-modals.spec.mjs`](../apps/srt-gui/browser-tests/settings-and-modals.spec.mjs) (**47/106** IPC names exercised across specs).
- **Verification Gates Passed (Post-Final-Edit, Node `v25.5.0` on `PATH`):**
  - `cargo fmt --all --check` and `cargo clippy --locked --workspace --all-targets -- -D warnings` (exit `0`, 0 warnings)
  - `python3 build-scripts/quality_check.py --desktop --browser --smoke` (exit `0`, all 19 stages: workflow safety, Tauri pairs, `check_doc_contracts.py` [`106` IPC, `12` MCP, `9` CLI, `23` widgets, `12` libs, `243` links], `generate_scorecard.py --check`, `test_scorecard.py` [`5` tests], `svelte-check` [`0` errors/`0` warnings], `test:merge` [`4`], `test:glob` [`3`], `check:patterns` [`4` tests, `0` findings], Vitest [`133/133` files, `639/639` tests], `test:dev-css`, `check:i18n` + missing translations, Vite build, `cargo test --locked --workspace --exclude whisper-bench` [`78` suites, `626` passed, `0` failed, `0` ignored], internal crate version check, Playwright [`30/30` passed across 5 spec files], `quality_smoke.py` FFmpeg/APKG [`10` notes, `20` media files])
  - `cargo test --locked -p whisper-bench` (`2` passed, `0` failed)
  - `python3 build-scripts/check_library_integration.py` (resolved `12` headless library crates + smoke binary pass)
  - `bash build-scripts/check-i18n.sh` (`1414` keys across `15` locales, `0` errors, `0` warnings)

---

## Wave 2 — Resume Guide for Next Session

Future agents continuing Vesta hardening should read [`SCORECARD.md`](./SCORECARD.md) and query open entries in `scorecard/bugs.toml` and `scorecard/performance.toml`. Priority next targets after Wave 1:

1. **Tauri Media Server & Custom Scheme Auth (`AUDIT-002`, `AUDIT-003`, `AUDIT-031`):** Add CSPRNG token generation + constant-time verification + canonical active-media allowlisting + restricted CORS on `media_stream.rs` and `main.rs` (`media_handler`), and remove `.unwrap()`/`.expect()` on Axum bind/serve.
2. **Plugin Runtime Sandboxing (`AUDIT-007`):** Filter inherited env vars, bound RPC line reads, switch `is_alive()` to `try_wait()`, enforce `permissions.scopes`, and drop `processes` lock across the 10s `initialize` await in `plugins/runtime.rs`.
3. **Frontend Protected Files (`AUDIT-020`, `AUDIT-021`, `AUDIT-090`):** Coordinate fixes to `ShortcutsTab.svelte` + `SettingsTab.svelte` (`#14.1`), `aiStore.svelte.ts` + `Sidebar.svelte` (`#14.2`), and `apiKeyEditorStore.svelte.ts` (`#14.8`) once the user's pending settings/API-key changes are committed.
4. **Flashcards ID Uniqueness & Multi-Track Media (`AUDIT-012`, `AUDIT-013`, `AUDIT-015`, `AUDIT-030`):** Collision-safe `note_id`/`guid` generation in `export_apkg.rs`, multi-audio-track `-map` preservation in `media.rs`, `tokio::select!` cancellation + timeout in `run_ffmpeg`, and ZIP extraction bounds in `core/srt-apkg`.
5. **High Performance Wins (`PERF-001`, `PERF-003`–`PERF-007`):** Batch/stream bitmap demux without forced `1280:720` scaling in `srt-ocr/bitmap.rs`, shared `LazyLock<reqwest::Client>` in `core/srt-download`, non-blocking `video_has_audio` & `analyze_roi_frame`, `CleanText::chars` cache, and `SubtitleListPanel.svelte` list virtualization.
6. **Frontend Mounted Tests & CI Expansion (`AUDIT-028`, `AUDIT-029`, `AUDIT-077`–`AUDIT-081`):** Add Vitest tests for `ShortcutsTab`, `TranscribeTab`, `TranslateTab`, 15 remaining panels, and 7 stores; wire Playwright E2E + `cargo clippy` / `fmt` / `check_library_integration.py` into `.github/workflows/ci.yml` and `quality_check.py`.
