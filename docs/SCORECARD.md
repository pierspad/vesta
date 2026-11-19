# Vesta Feature Scorecard & Audit Dashboard

> **Auto-generated** by [`build-scripts/generate_scorecard.py`](../build-scripts/generate_scorecard.py) from [`scorecard/*.toml`](../scorecard/features.toml) · **Wave:** `wave-1-complete` · **Last Updated:** `2026-11-19`  
> Do not edit manually; update `scorecard/*.toml` and re-run `python3 build-scripts/generate_scorecard.py`. See [`AUDIT_LOG.md`](./AUDIT_LOG.md) for chronological wave history and resumption pointers.

## Live Codebase & Test Metrics

| Metric | Value | Source |
|---|---|---|
| Cargo workspace crates | 24 | `Cargo.toml` (`core/`, `lib/`, `cli/`, `apps/`) |
| Rust production LOC (non-test) | 29,042 | `core/`, `lib/`, `cli/`, `apps/*/src-tauri/src/` |
| Rust unit & integration `#[test]` cases | 558 | Workspace `*.rs` |
| Rust prod `.unwrap()` / `.expect()` / `panic!()` / `unsafe` | 33 / 14 / 1 / 4 | Comment/test-stripped scan |
| Registered Tauri IPC commands | 106 | `apps/srt-gui/src-tauri/src/main.rs` |
| Playwright mock handlers (`vesta-mock.mjs`) | 106/106 | `apps/srt-gui/browser-tests/vesta-mock.mjs` |
| IPC names referenced in Playwright specs | 47/106 | `apps/srt-gui/browser-tests/*.spec.mjs` |
| Vitest files / cases | 133 / 617 | `apps/srt-gui/src/**/*.test.*` |
| Playwright spec files / `test()` decls | 5 / 26 | `apps/srt-gui/browser-tests/*.spec.mjs` |
| MCP tools (`vesta-mcp`) | 12 | `cli/vesta-mcp/src/lib.rs` |
| Unified `vesta-cli` workflows | 8 | `cli/vesta-cli/src/lib.rs` |
| Plugin SDK widget primitives | 23 | `PluginRenderer.svelte` |
| Frontend localization catalogs | 15 | `apps/srt-gui/src/lib/i18n/locales/*.json` |

## Executive Summary by Area

| Area | Features | GUI | CLI | MCP | Docs | Unit Tests | Playwright | Smoke | Verified | Open Findings |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Flashcards & APKG/TSV | 8 | 8/8 | 3/8 | 2/8 | 8/8 | 6/8 | 4/8 | 2/8 | 0/8 | 13 |
| Extract & OCR | 5 | 5/5 | 5/5 | 3/5 | 5/5 | 3/5 | 2/5 | 1/5 | 0/5 | 6 |
| Transcribe & VAD | 4 | 4/4 | 4/4 | 3/4 | 4/4 | 2/4 | 1/4 | 0/4 | 0/4 | 7 |
| Translate (LLM) | 4 | 4/4 | 4/4 | 4/4 | 4/4 | 4/4 | 3/4 | 0/4 | 0/4 | 5 |
| Sync & Align | 3 | 3/3 | 1/3 | 0/3 | 3/3 | 2/3 | 2/3 | 0/3 | 0/3 | 3 |
| Auto-Sync | 3 | 3/3 | 3/3 | 0/3 | 3/3 | 3/3 | 1/3 | 0/3 | 0/3 | 5 |
| Refine & AnkiConnect | 4 | 4/4 | 3/4 | 0/4 | 4/4 | 3/4 | 2/4 | 0/4 | 0/4 | 4 |
| Plugins & Experimental | 4 | 4/4 | 1/4 | 1/4 | 4/4 | 3/4 | 4/4 | 3/4 | 1/4 | 4 |
| GUI, Settings & i18n | 6 | 6/6 | 0/6 | 1/6 | 6/6 | 3/6 | 4/6 | 0/6 | 0/6 | 13 |
| Media Server, Net & Tools | 3 | 3/3 | 1/1 | 1/1 | 1/3 | 2/3 | 1/3 | 1/3 | 1/3 | 4 |
| CLI & MCP | 2 | 0/0 | 1/1 | 1/1 | 2/2 | 2/2 | 0/0 | 1/2 | 0/2 | 4 |
| Quality Gates, Docs & CI | 4 | 2/2 | 4/4 | 0/0 | 4/4 | 4/4 | 1/2 | 4/4 | 1/4 | 5 |
| **Total** | **50** | **46/46** | **30/47** | **16/43** | **48/50** | **37/50** | **25/46** | **12/50** | **3/50** | **73** |

## Audit Findings Summary (106 Verified Stage-1 Findings)

- **Bugs & Hardening (`bugs.toml`):** 98 total (32 fixed, 66 open/deferred)
- **Performance (`performance.toml`):** 8 total (1 fixed, 7 open)
- **Combined Progress:** 33/106 resolved, 73 remaining

| Severity | Open / Deferred | Fixed | Total |
|---|---:|---:|---:|
| CRITICAL | 0 | 1 | 1 |
| HIGH | 13 | 17 | 30 |
| MEDIUM | 48 | 10 | 58 |
| LOW | 12 | 5 | 17 |

## Reliability & Security Controls

| ID | Area | Control | Status | Target | Notes |
|---|---|---|---|---|---|
| `REL-01` | Media Server, Net & Tools | Local media HTTP & stream:// authentication and canonical path allowlisting | Fail | `apps/srt-gui/src-tauri/src/{main.rs,media_stream.rs}` | stream:// has no token/allowlist; HTTP server uses non-CSPRNG token, non-constant-time comparison, CorsLayer::Any, and unallowlisted ServeFile. |
| `REL-02` | Plugins & Experimental | Plugin install/uninstall path traversal, archive bomb & git arg injection guards | Pass | `apps/srt-gui/src-tauri/src/plugins/{mod.rs,archive.rs,url.rs}` | Wave 1 fixed #8.1, #8.4, #8.5, #8.6: plugins/mod.rs validates plugin IDs + canonical path containment; archive.rs enforces UTF-8 boundary truncation, streaming download/decompression caps, and git arg validation (verified via cargo test -p vesta --lib). |
| `REL-03` | Plugins & Experimental | Plugin & MCP subprocess isolation, bounded I/O, timeouts & kill_on_drop | Fail | `apps/srt-gui/src-tauri/src/plugins/runtime.rs, cli/vesta-mcp/src/lib.rs` | Wave 1 fixed #8.8 in cli/vesta-mcp/src/lib.rs (validate_plugin_id, kill_on_drop, bounded RPC lines, 10s init timeout); GUI runtime.rs (#8.7 env scoping, bounded lines, mutex-across-await) remains for Wave 2. |
| `REL-04` | Media Server, Net & Tools | Network proxy SSRF protection (metadata IPs & cross-origin redirects) | Pass | `apps/srt-gui/src-tauri/src/commands/net.rs` | Wave 1 fixed #8.9 in commands/net.rs: link-local metadata IPs (169.254.0.0/16, fe80::/10) blocked and CLIENT_FOLLOW / CLIENT_FOLLOW_CUSTOM forbid cross-boundary public-to-local/private redirects (verified via cargo test -p vesta --lib). |
| `REL-05` | Transcribe & VAD | Downloaded worker binary integrity verification (fail-closed SHA-256) & HTTP timeouts | Pass | `apps/whisper-bench/src-tauri/src/workers.rs` | Wave 1 fixed #8.10 in apps/whisper-bench/src-tauri/src/workers.rs: fail-closed SHA-256 checksum verification + shared timeout HTTP client + panic removal (verified via cargo test -p whisper-bench). |
| `REL-06` | Flashcards & APKG/TSV | APKG/ZIP extraction bounds, collision-safe IDs & delimiter sanitization | Partial | `core/srt-apkg/src/lib.rs, lib/srt-flashcards/src/{export_apkg.rs,export_tsv.rs,merge_apkg.rs}` | Wave 1 fixed #10.1 (merge_apkg strips did), #10.4 (export_tsv omits orphan media tags), and partial #10.9 (TSV deck_name tag sanitized); #10.2 ID collisions, #10.13 core/srt-apkg ZIP bounds, and APKG \x1f escaping remain for Wave 2. |
| `REL-07` | Flashcards & APKG/TSV | FFmpeg subprocess timeouts, process reaping & cancellation responsiveness | Partial | `lib/srt-flashcards/src/media.rs, lib/srt-extract/src/embedded.rs` | Per-card run_ffmpeg awaits without timeout/select+kill+wait; video probe/encoder detect ignore cancel; embedded extraction passes open NamedTempFile path. |
| `REL-08` | Translate (LLM) | Checkpoint resume integrity, NaN-free numeric math & panic-free Rust production paths | Partial | `lib/srt-translate/src/lib.rs, lib/srt-autosync/src/lib.rs, cli/srt-refine-cli/src/lib.rs` | Wave 1 fixed #12.1 (translate partial resume), partial #12.20 (autosync total_cmp/NaN guards), and workers.rs unwraps (#8.23); matching.rs arithmetic (#12.20) & srt-refine-cli lock poison remain for Wave 2. |
| `REL-09` | GUI, Settings & i18n | Frontend async race guards, modal keyboard safety & AI kill switch reachability | Partial | `apps/srt-gui/src/lib/{tabs,stores,modals}/` | Wave 1 fixed #14.3/#14.4/#14.20 (AlignTab race & empty save), #14.5 (TranslateTab contextLoading race), and #14.7 (PathPreviewModal Escape); #14.1 ShortcutsTab listener leak, #14.2 kill switch reachability & #14.12 focus traps remain for Wave 2. |
| `REL-10` | Quality Gates, Docs & CI | Backend/Frontend IPC, Mock schema, Docs, Integration & i18n automated verification | Partial | `build-scripts/, apps/srt-gui/browser-tests/vesta-mock.mjs` | Wave 1 fixed #15.1-#15.8 (106/106 IPCs mocked in vesta-mock.mjs + 30/30 Playwright passing) and #15.10-#15.15 doc/script drift; #15.9, #15.16-#15.18 remain for Wave 2. |

## Performance Optimization Tracker

| ID | Ref | Severity | Status | Target | Summary | Verification |
|---|---|---|---|---|---|---|
| `PERF-001` | `#12.5` | HIGH | `open` | `lib/srt-ocr/src/bitmap.rs:955-1006` | Spawns a sequential ffmpeg subprocess per cue (-ss ... -i ...) for VobSub/DVB/XSUB tracks and forces scale=1280:720 (distorting non-16:9 bitmaps). | Deferred to Wave 2 benchmark & bitmap integration tests. |
| `PERF-002` | `#12.6` | HIGH | `fixed` | `lib/srt-ocr/src/local.rs:2745` | model.initializers.clone() deep-copies all ONNX model tensor weights on every line box inside run_graph(). | Refactored lib/srt-ocr/src/local.rs (eval_node/run_graph) to borrow &model.initializers without per-inference HashMap cloning; verified with cargo test --locked -p srt-ocr. |
| `PERF-003` | `#10.16` | MEDIUM | `open` | `core/srt-download/src/lib.rs:50-54` | Builds a fresh reqwest::Client on every download_to() call instead of sharing a LazyLock<reqwest::Client>. | Deferred to Wave 2 (verify signature invariance via check_doc_contracts.py). |
| `PERF-004` | `#10.17` | MEDIUM | `open` | `lib/srt-flashcards/src/{media.rs:83-96,generation/mod.rs:108,snapshot_batch.rs:163-167,jobs.rs:110-111}` | Sync video_has_audio spawns OS thread + nested Tokio runtime; build_matched_lines & snapshot copies run sync on async workers; sequential fallback snapshots & repeated codec string clones. | Deferred to Wave 2 (ensure public signatures or contracts_manifest stay aligned). |
| `PERF-005` | `#12.17` | MEDIUM | `open` | `lib/srt-ocr/src/hardcoded.rs:587` | analyze_roi_frame(&png_bytes) decodes PNG images and scans pixels synchronously inside the async Tokio loop. | Deferred to Wave 2. |
| `PERF-006` | `#12.22` | MEDIUM | `open` | `lib/srt-autosync/src/normalization.rs:199-233` | CleanText::similarity re-allocates two Vec<char> and heap Levenshtein DP buffers per candidate in lib.rs:921 without threshold early-exit when token_score == 0.0. | Deferred to Wave 2. |
| `PERF-007` | `#14.13` | MEDIUM | `open` | `apps/srt-gui/src/lib/panels/SubtitleListPanel.svelte:109` | Unvirtualized {#each subtitles as sub (sub.id)} mounts thousands of DOM nodes for full movies. | Deferred to Wave 2. |
| `PERF-008` | `#14.14` | MEDIUM | `in_progress` | `apps/srt-gui/src/lib/panels/TranscriptionSegmentsPanel.svelte:9-16,38` | Unkeyed {#each segments as segment} forces full DOM node recreation plus unthrottled scroll $effect during streaming. | Wave 1 added keyed {#each segments as segment, idx} in TranscriptionSegmentsPanel.svelte; rAF auto-scroll coalescing deferred to Wave 2. |

## Full Feature Coverage Matrix

| ID | Area | Feature | GUI | CLI | MCP | Docs | Unit | Playwright | Smoke | Audit |
|---|---|---|---|---|---|---|---|---|---|---|
| `FC-01` | Flashcards & APKG/TSV | Single-episode flashcard generation (TSV & APKG) | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Issues |
| `FC-02` | Flashcards & APKG/TSV | Batch / series flashcard processing & per-episode media overrides | Yes | No | No | Yes | Yes | Partial | No | Partial |
| `FC-03` | Flashcards & APKG/TSV | Merge multiple .apkg decks into a unified deck | Yes | No | No | Yes | Yes | No | No | Partial |
| `FC-04` | Flashcards & APKG/TSV | Live card preview & on-demand audio/snapshot preview | Yes | Partial | Yes | Yes | Yes | Yes | No | Partial |
| `FC-05` | Flashcards & APKG/TSV | Subtitle filtering, sentence combining & smart bilingual matching | Yes | Yes | Partial | Yes | Yes | Partial | Yes | Partial |
| `FC-06` | Flashcards & APKG/TSV | Bilingual dueling subtitle export (.ass / .srt) | Yes | Yes | No | Yes | Yes | Yes | No | Partial |
| `FC-07` | Flashcards & APKG/TSV | Dictionary lookup & auto/embedded Anki card fonts | Yes | No | No | Yes | Partial | Partial | No | Partial |
| `FC-08` | Flashcards & APKG/TSV | Hardware video encoding, padding, resolution & audio track selection | Yes | Partial | No | Yes | Partial | Yes | Partial | Issues |
| `EX-01` | Extract & OCR | Subtitle parsing & inspection (SRT, ASS/SSA, VTT, LRC) | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Partial |
| `EX-02` | Extract & OCR | Embedded subtitle track listing & text stream extraction | Yes | Yes | Partial | Yes | Yes | Yes | No | Partial |
| `EX-03` | Extract & OCR | Bitmap subtitle extraction & OCR (PGS, VobSub, DVB, XSUB) | Yes | Yes | Yes | Yes | Partial | Partial | No | Partial |
| `EX-04` | Extract & OCR | Hardcoded video subtitle ROI detection & crop preview | Yes | Yes | Partial | Yes | Yes | Partial | No | Partial |
| `EX-05` | Extract & OCR | Hybrid local ONNX / Cloud Vision OCR & model management | Yes | Yes | Yes | Yes | Partial | Partial | No | Partial |
| `TR-01` | Transcribe & VAD | Cloud speech-to-text providers & word timestamps | Yes | Yes | Yes | Yes | Yes | Yes | No | Issues |
| `TR-02` | Transcribe & VAD | Local Whisper inference backends & whisper-bench harness | Yes | Yes | Yes | Yes | Yes | Partial | No | Partial |
| `TR-03` | Transcribe & VAD | Silero VAD speech segmentation & audio chunking | Yes | Yes | Yes | Yes | Partial | No | No | Partial |
| `TR-04` | Transcribe & VAD | Whisper model & VAD download / uninstall lifecycle | Yes | Yes | No | Yes | Partial | Partial | No | Partial |
| `TL-01` | Translate (LLM) | Tiered multi-provider LLM batch subtitle translation | Yes | Yes | Yes | Yes | Yes | Yes | No | Partial |
| `TL-02` | Translate (LLM) | Translation checkpoint resume & missing-cue repair | Yes | Yes | Yes | Yes | Yes | No | No | Partial |
| `TL-03` | Translate (LLM) | Translation context suggestion & language prompt specialization | Yes | Yes | Yes | Yes | Yes | Yes | No | Partial |
| `TL-04` | Translate (LLM) | Translation cancellation, path preview & CLI failure exit codes | Yes | Yes | Yes | Yes | Yes | Yes | No | Partial |
| `SY-01` | Sync & Align | Manual subtitle time shift, speed scaling & anchor interpolation | Yes | Yes | No | Yes | Yes | Yes | No | Partial |
| `SY-02` | Sync & Align | Sync session persistence, autosave, file export & reset | Yes | No | No | Yes | Partial | Yes | No | Partial |
| `SY-03` | Sync & Align | Dual-subtitle Align tab & companion subtitle suggestion | Yes | No | No | Yes | Yes | Partial | No | Partial |
| `AS-01` | Auto-Sync | Voice-overlap auto-sync & consensus anchor estimation | Yes | Yes | No | Yes | Yes | Yes | No | Partial |
| `AS-02` | Auto-Sync | Cloud-transcribe guided auto-sync alignment | Yes | Yes | No | Yes | Yes | No | No | Partial |
| `AS-03` | Auto-Sync | Auto-sync cancellation, progress overlay & quality assessment | Yes | Yes | No | Yes | Yes | Partial | No | Partial |
| `RF-01` | Refine & AnkiConnect | Tiered LLM flashcard deck annotation (.apkg & .tsv) | Yes | Yes | No | Yes | Yes | Yes | No | Partial |
| `RF-02` | Refine & AnkiConnect | Manual card search, editing & per-card notes history | Yes | No | No | Yes | Yes | Yes | No | Partial |
| `RF-03` | Refine & AnkiConnect | AnkiConnect connectivity ping & live .apkg import | Yes | Yes | No | Yes | Partial | Partial | No | Partial |
| `RF-04` | Refine & AnkiConnect | Refine run cancellation, lock safety & summary diagnostics | Yes | Yes | No | Yes | Yes | Partial | No | Partial |
| `PL-01` | Plugins & Experimental | Plugin registry, archive/git install, README inspect & uninstall | Yes | No | No | Yes | Yes | Yes | Yes | Verified |
| `PL-02` | Plugins & Experimental | Stdio JSON-RPC plugin host runtime & child lifecycle | Yes | No | Yes | Yes | Partial | Yes | Yes | Issues |
| `PL-03` | Plugins & Experimental | Host widget tree SDK (23 PluginRenderer widgets & vesta_plugin.py) | Yes | Yes | Partial | Yes | Yes | Yes | Yes | Partial |
| `PL-04` | Plugins & Experimental | Experimental plugins workspace & Playwright smoke integration | Yes | No | No | Yes | Yes | Yes | No | Partial |
| `GS-01` | GUI, Settings & i18n | Persistent vesta_config.json, backup recovery & installation reset | Yes | No | Yes | Yes | Yes | Yes | No | Partial |
| `GS-02` | GUI, Settings & i18n | API keys, custom endpoints & LLM/ITT tier configuration | Yes | Partial | Partial | Yes | Yes | Partial | No | Partial |
| `GS-03` | GUI, Settings & i18n | 15-locale i18n parity, RTL (ar) layout & responsive UI | Yes | No | No | Yes | Yes | Yes | No | Partial |
| `GS-04` | GUI, Settings & i18n | Keyboard shortcuts customization, overlay & modal accessibility | Yes | No | No | Yes | Partial | Yes | No | Issues |
| `GS-05` | GUI, Settings & i18n | First-run setup wizard, auto-updater & CPU/RAM monitor | Yes | No | No | Yes | Partial | Yes | No | Partial |
| `GS-06` | GUI, Settings & i18n | AI kill switch, support log redaction & notification inbox | Yes | No | No | Yes | Partial | Partial | No | Issues |
| `MS-01` | Media Server, Net & Tools | Local media HTTP server & stream:// custom URI protocol | Yes | N/A | N/A | Partial | No | No | No | Issues |
| `MS-02` | Media Server, Net & Tools | Sandboxed HTTP proxy fetch & redirect/SSRF guard (http_fetch) | Yes | N/A | N/A | Partial | Yes | No | No | Verified |
| `MS-03` | Media Server, Net & Tools | FFmpeg / ffprobe discovery, bundling & auto-download | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Partial |
| `CM-01` | CLI & MCP | Unified vesta-cli (8 workflows) & 8 standalone engine CLIs | N/A | Yes | N/A | Yes | Yes | N/A | Yes | Partial |
| `CM-02` | CLI & MCP | vesta-mcp stdio Model Context Protocol server (12 tools) | N/A | N/A | Yes | Yes | Yes | N/A | No | Partial |
| `QC-01` | Quality Gates, Docs & CI | Contract & manifest drift verification (check_doc_contracts.py) | N/A | Yes | N/A | Yes | Yes | N/A | Yes | Partial |
| `QC-02` | Quality Gates, Docs & CI | Workspace dependency boundaries (check_library_integration.py) | N/A | Yes | N/A | Yes | Yes | N/A | Yes | Partial |
| `QC-03` | Quality Gates, Docs & CI | 15-locale i18n parity & template hygiene gate (check-i18n.sh) | Yes | Yes | N/A | Yes | Yes | No | Yes | Verified |
| `QC-04` | Quality Gates, Docs & CI | Pre-release quality gate (quality_check.py), Playwright E2E & CI | Yes | Yes | N/A | Yes | Yes | Yes | Yes | Partial |

## Resolved Wave Findings

| ID | Ref | Severity | Area | Target | Resolution |
|---|---|---|---|---|---|
| `AUDIT-001` | `#8.1` | CRITICAL | Plugins & Experimental | `apps/srt-gui/src-tauri/src/plugins/mod.rs:384-390` | Added is_valid_plugin_id, registry check, canonical prefix containment in plugins/mod.rs::uninstall + unit test (cargo test -p vesta --lib). |
| `AUDIT-004` | `#8.4` | HIGH | Plugins & Experimental | `apps/srt-gui/src-tauri/src/plugins/archive.rs:399-400` | Replaced raw byte slice with UTF-8 char-boundary truncate_readme helper in plugins/archive.rs + unit test (cargo test -p vesta --lib). |
| `AUDIT-005` | `#8.5` | HIGH | Plugins & Experimental | `apps/srt-gui/src-tauri/src/plugins/archive.rs:49-52,210-211,252-253,307` | Added streaming MAX_ARCHIVE_BYTES download cap and checked_add + Read::take decompression bounds in plugins/archive.rs + unit tests (cargo test -p vesta --lib). |
| `AUDIT-006` | `#8.6` | HIGH | Plugins & Experimental | `apps/srt-gui/src-tauri/src/plugins/archive.rs:71-75` | Added validate_git_clone_args (scheme + '-' rejection) and '--' separator in plugins/archive.rs::git_clone_fallback + unit test (cargo test -p vesta --lib). |
| `AUDIT-008` | `#8.8` | HIGH | CLI & MCP | `cli/vesta-mcp/src/lib.rs:345-569` | Added validate_plugin_id, kill_on_drop(true), bounded read_bounded_rpc_line, 10s initialize timeout, and child kill+wait cleanup in cli/vesta-mcp/src/lib.rs + unit test (cargo test -p vesta-mcp). |
| `AUDIT-009` | `#8.9` | HIGH | Media Server, Net & Tools | `apps/srt-gui/src-tauri/src/commands/net.rs:10-39,98-120` | Excluded 169.254.0.0/16 & fe80::/10 in is_private_lan_ip and blocked public-to-local/private redirects in CLIENT_FOLLOW and CLIENT_FOLLOW_CUSTOM in commands/net.rs + unit test (cargo test -p vesta --lib). |
| `AUDIT-010` | `#8.10` | HIGH | Transcribe & VAD | `apps/whisper-bench/src-tauri/src/workers.rs:115-141,160,171` | Added shared LazyLock timeout client and fail-closed parse_sha256_checksum verification in apps/whisper-bench/src-tauri/src/workers.rs + unit test (cargo test -p whisper-bench). |
| `AUDIT-011` | `#10.1` | HIGH | Flashcards & APKG/TSV | `lib/srt-flashcards/src/{merge_apkg.rs:85-97,export_apkg/model.rs:129}` | Stripped 'did' alongside 'mod' when comparing shared model definitions in lib/srt-flashcards/src/merge_apkg.rs + unit test (cargo test -p srt-flashcards). |
| `AUDIT-014` | `#10.4` | HIGH | Flashcards & APKG/TSV | `lib/srt-flashcards/src/export_tsv.rs:121-169` | Suppressed orphan [sound:...]/<img ...> tags in lib/srt-flashcards/src/export_tsv.rs when real staging media directories lack those files + unit test (cargo test -p srt-flashcards). |
| `AUDIT-016` | `#12.1` | HIGH | Translate (LLM) | `lib/srt-translate/src/lib.rs:539-548` | Added compute_resume_position in lib/srt-translate/src/lib.rs to resume partial runs cleanly and skip completed runs + 5-case unit test (cargo test -p srt-translate). |
| `AUDIT-017` | `#12.2` | HIGH | Translate (LLM) | `lib/srt-translate/src/language_info.rs:7-209` | Added case-insensitive ISO/full-name normalization + ko/nl/pl/tr/hi examples in lib/srt-translate/src/{language_info.rs,prompts.rs} + unit tests (cargo test -p srt-translate). |
| `AUDIT-019` | `#12.4` | HIGH | Auto-Sync | `lib/srt-autosync/src/lib.rs:421,466-467,477,528-529` | Sorted local copies by start_ms and added finite >0.0 guards in lib/srt-autosync/src/lib.rs + unit test (cargo test -p srt-autosync). |
| `AUDIT-022` | `#14.3` | HIGH | Sync & Align | `apps/srt-gui/src/lib/tabs/AlignTab.svelte:380-404` | Added post-await selection guard in AlignTab.svelte for tryAutoSelectSourceForTarget / tryAutoSelectTargetForSource (Vitest + npm run check). |
| `AUDIT-023` | `#15.10` | HIGH | Quality Gates, Docs & CI | `build-scripts/update_project_info.sh:170,178` | Fixed paths in build-scripts/update_project_info.sh to src/lib/components/Sidebar.svelte and src/lib/tabs/SettingsTab.svelte. |
| `AUDIT-024` | `#15.1` | HIGH | Quality Gates, Docs & CI | `apps/srt-gui/browser-tests/vesta-mock.mjs:121` | Updated config_replace_all in apps/srt-gui/browser-tests/vesta-mock.mjs to read args.values and verified in Playwright. |
| `AUDIT-025` | `#15.2` | HIGH | Quality Gates, Docs & CI | `apps/srt-gui/browser-tests/vesta-mock.mjs:343-344` | Updated refine_cards_llm_tiered in vesta-mock.mjs to emit refine-progress events + RefineRunSummary and added E2E test in extended-workflows.spec.mjs. |
| `AUDIT-026` | `#15.3` | HIGH | Quality Gates, Docs & CI | `apps/srt-gui/browser-tests/vesta-mock.mjs:398` | Added mock handlers for all 33 previously unmocked IPC commands in apps/srt-gui/browser-tests/vesta-mock.mjs (106/106 backend IPC commands covered). |
| `AUDIT-061` | `#14.4` | MEDIUM | Sync & Align | `apps/srt-gui/src/lib/tabs/AlignTab.svelte:119,463-481` | Added empty sourceSubs guard in AlignTab.svelte::saveSource() so alignSave no-ops when no subtitle lines are loaded. |
| `AUDIT-062` | `#14.5` | MEDIUM | Translate (LLM) | `apps/srt-gui/src/lib/tabs/TranslateTab.svelte:185` | Guarded contextLoading = false with if (request === contextRequest) in TranslateTab.svelte::suggestContext(). |
| `AUDIT-064` | `#14.7` | MEDIUM | Translate (LLM) | `apps/srt-gui/src/lib/modals/PathPreviewModal.svelte:79-87` | Allowed Escape key propagation in PathPreviewModal.svelte (if (e.key !== 'Escape') e.stopPropagation()) + Playwright regression test in extended-workflows.spec.mjs. |
| `AUDIT-070` | `#10.15` | MEDIUM | Flashcards & APKG/TSV | `docs/modules/srt-flashcards.md:28,97-99` | Updated docs/modules/srt-flashcards.md to match actual LRC routing, list_audio_tracks signature, and vendored runtime layout (check_doc_contracts.py pass). |
| `AUDIT-071` | `#15.11` | MEDIUM | CLI & MCP | `.cursorrules:78,216` | Updated .cursorrules MCP tool count and listing to all 12 vesta-mcp tools (including list_embedded_subtitles and ocr_subtitles). |
| `AUDIT-072` | `#15.12` | MEDIUM | Quality Gates, Docs & CI | `.cursorrules:67-79` | Updated .cursorrules to include lib/srt-ocr (9 lib crates) and all 8 standalone CLIs + vesta-cli/vesta-mcp (10 CLI crates). |
| `AUDIT-073` | `#15.13` | MEDIUM | CLI & MCP | `cli/vesta-cli/README.md:3,13` | Updated cli/vesta-cli/README.md to document all 8 workflows including the ocr subcommand (check_doc_contracts.py pass). |
| `AUDIT-074` | `#15.5` | MEDIUM | Quality Gates, Docs & CI | `apps/srt-gui/browser-tests/vesta-mock.mjs:168` | Updated cancel_translation mock in vesta-mock.mjs to return boolean true matching Rust backend. |
| `AUDIT-075` | `#15.6` | MEDIUM | Quality Gates, Docs & CI | `apps/srt-gui/browser-tests/vesta-mock.mjs:374-383` | Updated flashcard_preview mock items in vesta-mock.mjs to include duration_ms and actor fields. |
| `AUDIT-076` | `#15.7` | MEDIUM | Quality Gates, Docs & CI | `apps/srt-gui/browser-tests/vesta-mock.mjs:396,422-430` | Updated flashcard_cancel and browserFixture.complete() in vesta-mock.mjs to return complete FlashcardResult fields. |
| `AUDIT-092` | `#14.20` | LOW | Sync & Align | `apps/srt-gui/src/lib/tabs/AlignTab.svelte:6,8` | Removed unused listen and onDestroy imports from AlignTab.svelte. |
| `AUDIT-093` | `#12.25` | LOW | Quality Gates, Docs & CI | `docs/modules/{srt-refine,srt-ocr,srt-transcribe,srt-sync,srt-autosync,srt-translate}.md` | Aligned parameter, field, return-type, and deprecation notes across docs/modules/{srt-refine,srt-ocr,srt-transcribe,srt-sync,srt-autosync,srt-translate}.md (check_doc_contracts.py pass). |
| `AUDIT-094` | `#15.14` | LOW | Flashcards & APKG/TSV | `cli/srt-flashcards-cli/README.md:14` | Documented the dueling subcommand in cli/srt-flashcards-cli/README.md and docs/CLI.md. |
| `AUDIT-095` | `#15.15` | LOW | Translate (LLM) | `apps/srt-gui/src-tauri/src/commands/translate.rs:351,359,370` | Translated Italian doc comments and error message to English in apps/srt-gui/src-tauri/src/commands/translate.rs. |
| `AUDIT-096` | `#15.8` | LOW | Quality Gates, Docs & CI | `apps/srt-gui/browser-tests/vesta-mock.mjs:114-115` | Updated config_load_all mock in vesta-mock.mjs to return { entries, recovered_backup_path } ConfigLoadResult shape. |

## Open Critical & High Findings

Full 98-item bug/doc/test tracker lives in [`scorecard/bugs.toml`](../scorecard/bugs.toml).

| ID | Ref | Severity | Area | Target | Summary |
|---|---|---|---|---|---|
| `AUDIT-002` | `#8.2` | HIGH | Media Server, Net & Tools | `apps/srt-gui/src-tauri/src/media_stream.rs:12-38` | stream://localhost/<path> URL-decodes and opens arbitrary local files with no session token check or path allowlist. |
| `AUDIT-003` | `#8.3` | HIGH | Media Server, Net & Tools | `apps/srt-gui/src-tauri/src/main.rs:36-69,118-121` | generate_media_token hashes UNIX_EPOCH nanoseconds with RandomState (non-CSPRNG); media_handler uses != and ServeFile without allowlist; Axum uses CorsLayer::allow_origin/methods/headers(Any). |
| `AUDIT-007` | `#8.7` | HIGH | Plugins & Experimental | `apps/srt-gui/src-tauri/src/plugins/runtime.rs:60-62,117-299` | Plugin subprocesses inherit full parent env without enforcing scopes, read unbounded stdout/stderr lines, omit kill_on_drop(true), check !stdin_tx.is_closed() in is_alive(), and hold mutex across 10s initialize await. |
| `AUDIT-012` | `#10.2` | HIGH | Flashcards & APKG/TSV | `lib/srt-flashcards/src/export_apkg.rs:46-49,66-67,159` | note_id = timestamp_secs * 1000 + seq and 10-char hex guid collide across decks with >1000 cards exported in consecutive seconds. |
| `AUDIT-013` | `#10.3` | HIGH | Flashcards & APKG/TSV | `lib/srt-flashcards/src/media.rs:483-688,961-964` | Video optimization omits -map (dropping secondary audio tracks), breaking extract_video_clip -map 0:a:{track_index} whenever audio_track_index >= 1. |
| `AUDIT-015` | `#10.5` | HIGH | Flashcards & APKG/TSV | `lib/srt-flashcards/src/media.rs:786-796` | Per-card run_ffmpeg uses cmd.output().await without timeout or tokio::select! cancel+kill+wait. |
| `AUDIT-018` | `#12.3` | HIGH | Transcribe & VAD | `lib/srt-transcribe/src/cloud.rs:93-103,369-376,484-493` | Wordless Deepgram/AssemblyAI cloud responses emit synthetic 0..0 timestamps even via transcribe_timed_chunk, failing srt-autosync. |
| `AUDIT-020` | `#14.1` | HIGH | GUI, Settings & i18n | `apps/srt-gui/src/lib/tabs/ShortcutsTab.svelte:90,231` | Capturing window keydown listeners stay active app-wide when switching tabs because SettingsTab stays mounted hidden and ShortcutsTab lacks active prop cleanup. (Wave 2: SettingsTab.svelte in active user uncommitted set.) |
| `AUDIT-021` | `#14.2` | HIGH | GUI, Settings & i18n | `apps/srt-gui/src/lib/stores/aiStore.svelte.ts:25-31` | AI Kill Switch disables itself and returns early when hasActiveAiProcess is true, rendering UI cancellation unreachable during runaway AI jobs. (Wave 2: coordinate with SettingsTab/Sidebar behavior.) |
| `AUDIT-027` | `#15.4` | HIGH | Quality Gates, Docs & CI | `apps/srt-gui/browser-tests/` | ~78 registered Tauri IPC commands are never exercised or asserted in Playwright specs. |
| `AUDIT-028` | `#14.15` | HIGH | GUI, Settings & i18n | `apps/srt-gui/src/lib/tabs/` | ShortcutsTab, TranscribeTab, and TranslateTab lack *.mounted.test.ts; RefineTab only has RefineTab.history.mounted.test.ts. |
| `AUDIT-029` | `#14.16` | HIGH | GUI, Settings & i18n | `apps/srt-gui/src/lib/panels/` | 15 of 19 panels in apps/srt-gui/src/lib/panels/ lack unit or mounted tests. |
