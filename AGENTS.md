# Repository Instructions

- **Commit Language & Style**: Always write commit subjects and bodies in **English**, regardless of the conversation language. Use Conventional Commits (`feat:`, `fix:`, `perf:`, `refactor:`, `docs:`, `test:`, `chore:`, `ci:`, `build:`); commit subjects become end-user release notes. Keep commit bodies empty or at most 1–3 concise lines.
- **Conventions & Architecture**: Follow [`.cursorrules`](.cursorrules) for codebase conventions, [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for crate/IPC boundaries, [`docs/QUALITY.md`](docs/QUALITY.md) for verification gates, and [`docs/guide_to_create_good_software.md`](docs/guide_to_create_good_software.md) for the engineering blueprint.
- **Backend Error Messages**: Write all Rust backend error strings, logs, and code comments in English. User-facing localization lives exclusively in the 15 locale JSON catalogs under `apps/vesta-gui/src/lib/i18n/locales/`.

## Verification & Quality Scripts

Run these from the repository root before considering work complete:

| Script / Command | Purpose |
|---|---|
| `python3 build-scripts/quality_check.py --desktop --smoke` | Full pre-release gate: Tauri version sync, workflow tests, doc/API contracts, Svelte/TS checks, AST pattern checks, Vitest, dev-CSS, i18n audit, frontend build, headless + desktop Rust tests, and synthetic FFmpeg/APKG smoke test. |
| `python3 build-scripts/quality_check.py` | Fast headless gate (skips native `vesta` desktop crate and synthetic media smoke test). |
| `cargo fmt --all --check` | Verify Rust formatting across the workspace. |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Strict Clippy lint gate across all workspace targets. |

## Contract & Hash Verification (`docs/contracts_manifest.json`)

[`build-scripts/check_doc_contracts.py`](build-scripts/check_doc_contracts.py) blocks code-to-doc drift across 6 surfaces:
1. **Tauri IPC commands** (`apps/vesta-gui/src-tauri/src/main.rs` & `commands/` vs frontend `invoke(...)` and `docs/ARCHITECTURE.md`; browser test payloads are also checked by `python3 build-scripts/browser_ipc_contract.py --check`)
2. **MCP server tools** (`cli/vesta-mcp/src/lib.rs` 24 tools & schemas vs `docs/MCP.md` and `docs/ARCHITECTURE.md`)
3. **CLI crates & per-subcommand flags** (`cli/*/src/*.rs` vs `docs/CLI.md` and `docs/modules/*.md`)
4. **Plugin SDK widget parity** (`PluginRenderer.svelte` `node.type` branches vs `pluginClient.ts`, `vesta_plugin.py`, and `docs/PLUGINS.md`)
5. **Rust library public API signatures & fields** (`core/*`, `lib/*` `pub fn`/`pub struct`/`pub enum` hashes vs `docs/modules/*.md`)
6. **Markdown links & heading anchors** across `README.md`, `AGENTS.md`, `CONTRIBUTING.md`, and `docs/**/*.md`

Commands:
- `python3 build-scripts/check_doc_contracts.py` — Verify all contracts, links, and hashes against [`docs/contracts_manifest.json`](docs/contracts_manifest.json).
- `python3 build-scripts/check_doc_contracts.py --report` — Print a JSON report of drifted areas or modified API hashes.
- `python3 build-scripts/check_doc_contracts.py --update` — Refresh `docs/contracts_manifest.json` after intentional documented updates.
- `python3 build-scripts/test_doc_contracts.py` — Unit tests for the contract checker and Plugin SDK CLI.
- `python3 build-scripts/check_library_integration.py` — Verify standalone crate integration and dependency boundaries (`docs/library-integration.md`).

## Localization (i18n) Verification

All 15 locale catalogs in `apps/vesta-gui/src/lib/i18n/locales/` (`ar`, `de`, `en`, `es`, `fr`, `hi`, `it`, `ja`, `ko`, `nl`, `pl`, `pt`, `ru`, `tr`, `zh`) must stay in lockstep with `en.json`:
- `bash build-scripts/check-i18n.sh` — Unified gate (runs both checks below; enforced in Git hooks and CI).
- `node apps/vesta-gui/scripts/check-i18n.mjs` — Key parity, `{{placeholder}}` equality, invisible-char/whitespace hygiene, and Svelte template scans.
- `python3 apps/vesta-gui/scripts/check_missing_translations.py --fail-on-issues` — Missing/empty/`same_as_english` audit across all 15 locales (pass `--check-stale` for `git blame` staleness checks).

## Plugin SDK & Headless Testing

[`docs/examples/plugin-hello/vesta_plugin.py`](docs/examples/plugin-hello/vesta_plugin.py) provides the zero-dependency Python SDK and headless validator:
- `python3 docs/examples/plugin-hello/vesta_plugin.py init <dir> --id <id> --name "<Name>"` — Scaffold a plugin.
- `python3 docs/examples/plugin-hello/vesta_plugin.py validate <dir>` — Validate `vesta-plugin.json`.
- `python3 docs/examples/plugin-hello/vesta_plugin.py test <dir>` — Headless stdio JSON-RPC smoke test (`initialize` → `render` → widget tree validation → `shutdown`).

## Feature Scorecard & Continuous Audit

Vesta tracks feature coverage (GUI, CLI, MCP, Docs, Unit Tests, Playwright, Smoke), reliability controls, performance items, and open bugs across 12 areas in [`scorecard/*.toml`](scorecard/features.toml):
- [`docs/SCORECARD.md`](docs/SCORECARD.md) — Auto-generated dashboard (`python3 build-scripts/generate_scorecard.py`, validated in hooks and CI).
- **Queryable SQLite database (`build-scripts/scorecard_db.py`)**: Query [`scorecard/scorecard.db`](scorecard/scorecard.db) instead of loading `scorecard/*.toml` into context (verified via `python3 build-scripts/scorecard_db.py check`):
  - `python3 build-scripts/scorecard_db.py summary` — Compact status totals and per-area coverage counts.
  - `python3 build-scripts/scorecard_db.py open` — Non-fixed bugs & performance items (`--area`, `--severity`, `--no-include-deferred`).
  - `python3 build-scripts/scorecard_db.py gaps [--dim playwright]` — Missing/partial coverage (`gui`, `cli`, `mcp`, `docs`, `unit_tests`, `playwright`, `smoke`, `audit_status`).
  - `python3 build-scripts/scorecard_db.py show <ID>` / `query "<SQL>"` — Inspect a single item or run read-only SQLite queries (`--json` supported).
  - `python3 build-scripts/scorecard_db.py impact [--since origin/dev] [--run] [--playwright]` — Map modified files via [`scorecard/impact_map.toml`](scorecard/impact_map.toml) to impacted features and targeted Cargo/Vitest/Playwright suites.
  - `python3 build-scripts/scorecard_db.py sync` / `check` — Rebuild or verify `scorecard/scorecard.db` against `scorecard/*.toml` and `scorecard/impact_map.toml`.
  - `python3 build-scripts/scorecard_db.py update-bug|update-perf|update-feature|update-reliability|add-bug ...` — Atomically update `scorecard/*.toml`, sync `scorecard.db`, and regenerate `docs/SCORECARD.md` (verify with `python3 build-scripts/test_scorecard.py`).

## Build Artifacts & Disk Hygiene

- **Avoid extra target directories**: Never set custom `CARGO_TARGET_DIR` paths or leave temporary worktrees containing `target/` directories (`check_library_integration.py` reuses `target/integration`).
- **Avoid unnecessary `cargo clean`**: Rebuilding `whisper-rs-sys` and Tauri from scratch is expensive; use targeted `-p <crate>` checks during iteration.
- **Keep `target/` bounded (< 40 GiB)**: Check `du -sh target` periodically; prune stale artifacts with `python3 build-scripts/quality_check.py --prune-only` or remove `target/debug/incremental` and `target/integration`.
