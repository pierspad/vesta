# Vesta Feature Scorecard & Audit Dashboard

> **Auto-generated** by [`build-scripts/generate_scorecard.py`](../build-scripts/generate_scorecard.py) from [`scorecard/*.toml`](../scorecard/features.toml) · **Wave:** `wave-3-complete` · **Last Updated:** `2026-11-23`  
> Do not edit manually; update `scorecard/*.toml` and re-run `python3 build-scripts/generate_scorecard.py`.

## Live Codebase & Test Metrics

| Metric | Value | Source |
|---|---|---|
| Cargo workspace crates | 24 | `Cargo.toml` (`core/`, `lib/`, `cli/`, `apps/`) |
| Rust production LOC (non-test) | 33,301 | `core/`, `lib/`, `cli/`, `apps/*/src-tauri/src/` |
| Rust unit & integration `#[test]` cases | 637 | Workspace `*.rs` |
| Rust prod `.unwrap()` / `.expect()` / `panic!()` / `unsafe` | 26 / 12 / 1 / 4 | Comment/test-stripped scan |
| Registered Tauri IPC commands | 109 | `apps/vesta-gui/src-tauri/src/main.rs` |
| Playwright mock handlers (`vesta-mock.mjs`) | 109/109 | `apps/vesta-gui/browser-tests/vesta-mock.mjs` |
| IPC names referenced in Playwright specs | 109/109 | `apps/vesta-gui/browser-tests/*.spec.mjs` |
| Vitest files / cases | 148 / 676 | `apps/vesta-gui/src/**/*.test.*` |
| Playwright spec files / `test()` decls | 6 / 40 | `apps/vesta-gui/browser-tests/*.spec.mjs` |
| MCP tools (`vesta-mcp`) | 24 | `cli/vesta-mcp/src/lib.rs` |
| Unified `vesta-cli` workflows | 9 | `cli/vesta-cli/src/lib.rs` |
| Plugin SDK widget primitives | 23 | `PluginRenderer.svelte` |
| Frontend localization catalogs | 15 | `apps/vesta-gui/src/lib/i18n/locales/*.json` |

## Executive Summary by Area

> *Coverage denominators:* `GUI`, `CLI`, `MCP`, and `Playwright` count only applicable features (excluding `N/A`), whereas `Docs`, `Unit Tests`, `Smoke`, and `Verified` cover all 50 features.

| Area | Features | GUI | CLI | MCP | Docs | Unit Tests | Playwright | Smoke | Verified | Open Findings |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Flashcards & APKG/TSV | 8 | 8/8 | 8/8 | 8/8 | 8/8 | 8/8 | 8/8 | 8/8 | 8/8 | 0 |
| Extract & OCR | 5 | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 | 5/5 | 0 |
| Transcribe & VAD | 4 | 4/4 | 4/4 | 4/4 | 4/4 | 4/4 | 4/4 | 2/4 | 4/4 | 0 |
| Translate (LLM) | 4 | 4/4 | 4/4 | 4/4 | 4/4 | 4/4 | 4/4 | 4/4 | 4/4 | 0 |
| Sync & Align | 3 | 3/3 | 1/1 | 1/1 | 3/3 | 3/3 | 3/3 | 1/3 | 3/3 | 0 |
| Auto-Sync | 3 | 3/3 | 3/3 | 3/3 | 3/3 | 3/3 | 3/3 | 2/3 | 3/3 | 0 |
| Refine & AnkiConnect | 4 | 4/4 | 4/4 | 4/4 | 4/4 | 4/4 | 4/4 | 4/4 | 4/4 | 0 |
| Plugins & Experimental | 4 | 4/4 | 1/1 | 2/2 | 4/4 | 4/4 | 4/4 | 3/4 | 4/4 | 0 |
| GUI, Settings & i18n | 6 | 6/6 | 1/1 | 2/2 | 6/6 | 6/6 | 6/6 | 0/6 | 6/6 | 0 |
| Media Server, Net & Tools | 3 | 3/3 | 1/1 | 1/1 | 3/3 | 3/3 | 3/3 | 1/3 | 3/3 | 0 |
| CLI & MCP | 2 | 0/0 | 1/1 | 1/1 | 2/2 | 2/2 | 0/0 | 2/2 | 2/2 | 0 |
| Quality Gates, Docs & CI | 4 | 2/2 | 4/4 | 0/0 | 4/4 | 4/4 | 2/2 | 4/4 | 4/4 | 0 |
| **Total** | **50** | **46/46** | **37/37** | **35/35** | **50/50** | **50/50** | **46/46** | **36/50** | **50/50** | **0** |

## Findings Summary

- **Bugs & Hardening (`bugs.toml`):** 0 open/deferred (0 total)
- **Performance (`performance.toml`):** 0 open/deferred (0 total)

## Reliability & Security Controls

| ID | Area | Control | Status | Target | Notes |
|---|---|---|---|---|---|
| `REL-01` | Media Server, Net & Tools | Local media HTTP & stream:// authentication and canonical path allowlisting | Pass | `apps/vesta-gui/src-tauri/src/{main.rs,media_stream.rs}` | Wave 2 fixed #8.2, #8.3, #8.18: added MediaAllowlist canonical file allowlisting, CSPRNG token via getrandom, subtle::ConstantTimeEq token verification, restricted AllowOrigin, and checked bind/serve error handling (verified via cargo test -p vesta --lib). |
| `REL-02` | Plugins & Experimental | Plugin install/uninstall path traversal, archive bomb & git arg injection guards | Pass | `apps/vesta-gui/src-tauri/src/plugins/{mod.rs,archive.rs,url.rs}` | Wave 1 fixed #8.1, #8.4, #8.5, #8.6: plugins/mod.rs validates plugin IDs + canonical path containment; archive.rs enforces UTF-8 boundary truncation, streaming download/decompression caps, and git arg validation (verified via cargo test -p vesta --lib). |
| `REL-03` | Plugins & Experimental | Plugin & MCP subprocess isolation, bounded I/O, timeouts & kill_on_drop | Pass | `apps/vesta-gui/src-tauri/src/plugins/runtime.rs, cli/vesta-mcp/src/lib.rs` | Wave 1 fixed #8.8 in cli/vesta-mcp/src/lib.rs; Wave 2 fixed #8.7 in plugins/runtime.rs (env allowlist isolation, scoped permissions, bounded stdout/stderr reads, kill_on_drop(true), try_wait() in is_alive(), and lock-free initialize) verified via cargo test -p vesta --lib. |
| `REL-04` | Media Server, Net & Tools | Network proxy SSRF protection (metadata IPs & cross-origin redirects) | Pass | `apps/vesta-gui/src-tauri/src/commands/net.rs` | Wave 1 fixed #8.9 in commands/net.rs: link-local metadata IPs (169.254.0.0/16, fe80::/10) blocked and CLIENT_FOLLOW / CLIENT_FOLLOW_CUSTOM forbid cross-boundary public-to-local/private redirects (verified via cargo test -p vesta --lib). |
| `REL-05` | Transcribe & VAD | Downloaded worker binary integrity verification (fail-closed SHA-256) & HTTP timeouts | Pass | `apps/whisper-bench/src-tauri/src/workers.rs` | Wave 1 fixed #8.10 in apps/whisper-bench/src-tauri/src/workers.rs: fail-closed SHA-256 checksum verification + shared timeout HTTP client + panic removal (verified via cargo test -p whisper-bench). |
| `REL-06` | Flashcards & APKG/TSV | APKG/ZIP extraction bounds, collision-safe IDs & delimiter sanitization | Pass | `core/vesta-apkg/src/lib.rs, lib/vesta-flashcards/src/{export_apkg.rs,export_tsv.rs,merge_apkg.rs}` | Wave 1 fixed #10.1/#10.4; Wave 2 fixed #10.2 (atomic monotonic reserve_note_id_block + SHA-1 16-hex apkg_guid), #10.9 (\x1f APKG field sanitization + TSV deck_name sanitization), and #10.13 (core/vesta-apkg bounded ZIP extraction & symlink skip, merge_apkg checked SQLite ID arithmetic & 512 MiB cap) verified via cargo test -p vesta-apkg -p vesta-flashcards. |
| `REL-07` | Flashcards & APKG/TSV | FFmpeg subprocess timeouts, process reaping & cancellation responsiveness | Pass | `lib/vesta-flashcards/src/media.rs, lib/vesta-extract/src/embedded.rs` | Wave 2 fixed #10.5/#10.6 (120s timeout + tokio::select! cancel+kill+wait in run_ffmpeg and cancellable probe_video_stream / detect_h264_encoder) and #10.7 (tempdir_in staging + create_dir_all + symlink refusal in vesta-extract/src/embedded.rs) verified via cargo test -p vesta-flashcards -p vesta-extract. |
| `REL-08` | Translate (LLM) | Checkpoint resume integrity, NaN-free numeric math & panic-free Rust production paths | Pass | `lib/vesta-translate/src/lib.rs, lib/vesta-autosync/src/lib.rs, cli/vesta-refine-cli/src/lib.rs` | Wave 1 fixed #12.1/#12.20 NaN sort; Wave 2 fixed #12.20 matching.rs (saturating_* + i128 interpolation + anchors_by_target), #8.23 vesta-refine-cli lock-poison recovery, and translate JSON/legacy/Gemini parsing verified via cargo test. |
| `REL-09` | GUI, Settings & i18n | Frontend async race guards, modal keyboard safety & AI kill switch reachability | Pass | `apps/vesta-gui/src/lib/{tabs,stores,modals}/` | Wave 1 fixed #14.3/#14.4/#14.5/#14.7; Wave 2 fixed #14.1 (ShortcutsTab active prop + mounted test), #14.2 (AI Kill Switch + aiStore tests), and #14.8 (apiKeyEditorStore); Wave 3 added reusable focusTrap Svelte action (focusTrap.ts + focusTrap.test.ts) on SupportModal, EpisodeMediaSettingsModal, and PathPreviewModal plus ExperimentalTab active prop (AUDIT-068, AUDIT-091). |
| `REL-10` | Quality Gates, Docs & CI | Backend/Frontend IPC, Mock schema, Docs, Integration & i18n automated verification | Pass | `build-scripts/, apps/vesta-gui/browser-tests/vesta-mock.mjs, .github/workflows/ci.yml` | Wave 1 fixed #15.1-#15.8/#15.10-#15.15; Wave 2 added --locked flags in pre-push (AUDIT-080); Wave 3 expanded Playwright IPC coverage to 108/108 commands (ipc-coverage.spec.mjs), enforced mock & frontend invoke parity via browser_ipc_contract.py --check + test_browser_ipc_contract.py (AUDIT-078), wired check_library_integration.py/fmt/clippy/scorecard checks into quality_check.py (AUDIT-079), added webkit2gtk-4.1 headless guard in pre-push + test_git_hooks.py (AUDIT-080), and added CI browser Playwright job & contract/scorecard gates (AUDIT-081). |

## Performance Tracker

_No open performance optimizations queued._

## Full Feature Coverage Matrix

| ID | Area | Feature | GUI | CLI | MCP | Docs | Unit | Playwright | Smoke | Audit |
|---|---|---|---|---|---|---|---|---|---|---|
| `FC-01` | Flashcards & APKG/TSV | Single-episode flashcard generation (TSV & APKG) | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `FC-02` | Flashcards & APKG/TSV | Batch / series flashcard processing & per-episode media overrides | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `FC-03` | Flashcards & APKG/TSV | Merge multiple .apkg decks into a unified deck | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `FC-04` | Flashcards & APKG/TSV | Live card preview & on-demand audio/snapshot preview | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `FC-05` | Flashcards & APKG/TSV | Subtitle filtering, sentence combining & smart bilingual matching | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `FC-06` | Flashcards & APKG/TSV | Bilingual dueling subtitle export (.ass / .srt) | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `FC-07` | Flashcards & APKG/TSV | Dictionary lookup & auto/embedded Anki card fonts | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `FC-08` | Flashcards & APKG/TSV | Hardware video encoding, padding, resolution & audio track selection | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `EX-01` | Extract & OCR | Subtitle parsing & inspection (SRT, ASS/SSA, VTT, LRC) | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `EX-02` | Extract & OCR | Embedded subtitle track listing & text stream extraction | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `EX-03` | Extract & OCR | Bitmap subtitle extraction & OCR (PGS, VobSub, DVB, XSUB) | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `EX-04` | Extract & OCR | Hardcoded video subtitle ROI detection & crop preview | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `EX-05` | Extract & OCR | Hybrid local ONNX / Cloud Vision OCR & model management | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `TR-01` | Transcribe & VAD | Cloud speech-to-text providers & word timestamps | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `TR-02` | Transcribe & VAD | Local Whisper inference backends & whisper-bench harness | Yes | Yes | Yes | Yes | Yes | Yes | No | Verified |
| `TR-03` | Transcribe & VAD | Silero VAD speech segmentation & audio chunking | Yes | Yes | Yes | Yes | Yes | Yes | No | Verified |
| `TR-04` | Transcribe & VAD | Whisper model & VAD download / uninstall lifecycle | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `TL-01` | Translate (LLM) | Tiered multi-provider LLM batch subtitle translation | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `TL-02` | Translate (LLM) | Translation checkpoint resume & missing-cue repair | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `TL-03` | Translate (LLM) | Translation context suggestion & language prompt specialization | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `TL-04` | Translate (LLM) | Translation cancellation, path preview & CLI failure exit codes | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `SY-01` | Sync & Align | Manual subtitle time shift, speed scaling & anchor interpolation | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `SY-02` | Sync & Align | Sync session persistence, autosave, file export & reset | Yes | N/A | N/A | Yes | Yes | Yes | No | Verified |
| `SY-03` | Sync & Align | Dual-subtitle Align tab & companion subtitle suggestion | Yes | N/A | N/A | Yes | Yes | Yes | No | Verified |
| `AS-01` | Auto-Sync | Voice-overlap auto-sync & consensus anchor estimation | Yes | Yes | Yes | Yes | Yes | Yes | No | Verified |
| `AS-02` | Auto-Sync | Cloud-transcribe guided auto-sync alignment | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `AS-03` | Auto-Sync | Auto-sync cancellation, progress overlay & quality assessment | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `RF-01` | Refine & AnkiConnect | Tiered LLM flashcard deck annotation (.apkg & .tsv) | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `RF-02` | Refine & AnkiConnect | Manual card search, editing & per-card notes history | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `RF-03` | Refine & AnkiConnect | AnkiConnect connectivity ping & live .apkg import | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `RF-04` | Refine & AnkiConnect | Refine run cancellation, lock safety & summary diagnostics | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `PL-01` | Plugins & Experimental | Plugin registry, archive/git install, README inspect & uninstall | Yes | N/A | N/A | Yes | Yes | Yes | Yes | Verified |
| `PL-02` | Plugins & Experimental | Stdio JSON-RPC plugin host runtime & child lifecycle | Yes | N/A | Yes | Yes | Yes | Yes | Yes | Verified |
| `PL-03` | Plugins & Experimental | Host widget tree SDK (23 PluginRenderer widgets & vesta_plugin.py) | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `PL-04` | Plugins & Experimental | Experimental plugins workspace & Playwright smoke integration | Yes | N/A | N/A | Yes | Yes | Yes | No | Verified |
| `GS-01` | GUI, Settings & i18n | Persistent vesta_config.json, backup recovery & installation reset | Yes | N/A | Yes | Yes | Yes | Yes | No | Verified |
| `GS-02` | GUI, Settings & i18n | API keys, custom endpoints & LLM/ITT tier configuration | Yes | Yes | Yes | Yes | Yes | Yes | No | Verified |
| `GS-03` | GUI, Settings & i18n | 15-locale i18n parity, RTL (ar) layout & responsive UI | Yes | N/A | N/A | Yes | Yes | Yes | No | Verified |
| `GS-04` | GUI, Settings & i18n | Keyboard shortcuts customization, overlay & modal accessibility | Yes | N/A | N/A | Yes | Yes | Yes | No | Verified |
| `GS-05` | GUI, Settings & i18n | First-run setup wizard, auto-updater & CPU/RAM monitor | Yes | N/A | N/A | Yes | Yes | Yes | No | Verified |
| `GS-06` | GUI, Settings & i18n | AI kill switch, support log redaction & notification inbox | Yes | N/A | N/A | Yes | Yes | Yes | No | Verified |
| `MS-01` | Media Server, Net & Tools | Local media HTTP server & stream:// custom URI protocol | Yes | N/A | N/A | Yes | Yes | Yes | No | Verified |
| `MS-02` | Media Server, Net & Tools | Sandboxed HTTP proxy fetch & redirect/SSRF guard (http_fetch) | Yes | N/A | N/A | Yes | Yes | Yes | No | Verified |
| `MS-03` | Media Server, Net & Tools | FFmpeg / ffprobe discovery, bundling & auto-download | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Verified |
| `CM-01` | CLI & MCP | Unified vesta-cli (8 workflows) & 8 standalone engine CLIs | N/A | Yes | N/A | Yes | Yes | N/A | Yes | Verified |
| `CM-02` | CLI & MCP | vesta-mcp stdio Model Context Protocol server (24 tools) | N/A | N/A | Yes | Yes | Yes | N/A | Yes | Verified |
| `QC-01` | Quality Gates, Docs & CI | Contract & manifest drift verification (check_doc_contracts.py) | N/A | Yes | N/A | Yes | Yes | N/A | Yes | Verified |
| `QC-02` | Quality Gates, Docs & CI | Workspace dependency boundaries (check_library_integration.py) | N/A | Yes | N/A | Yes | Yes | N/A | Yes | Verified |
| `QC-03` | Quality Gates, Docs & CI | 15-locale i18n parity & template hygiene gate (check-i18n.sh) | Yes | Yes | N/A | Yes | Yes | Yes | Yes | Verified |
| `QC-04` | Quality Gates, Docs & CI | Pre-release quality gate (quality_check.py), Playwright E2E & CI | Yes | Yes | N/A | Yes | Yes | Yes | Yes | Verified |

## Open Findings

_No open bugs._
