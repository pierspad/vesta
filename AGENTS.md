# Repository Instructions

- **Commit Language & Style**: Always write commit subjects and bodies in **English**, regardless of the conversation language. Use Conventional Commits (`feat:`, `fix:`, `perf:`, `refactor:`, `docs:`, `test:`, `chore:`); commit subjects become release notes. Keep commit bodies empty or at most 1–3 concise lines.
- **Conventions & Architecture**: Follow [`.cursorrules`](.cursorrules) for repository conventions, [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for crate/IPC boundaries, and [`docs/QUALITY.md`](docs/QUALITY.md) for verification gates.
- **Backend Error Messages**: Write all Rust backend error strings, logs, and code comments in English. User-facing localization is handled in the frontend via the 15 locale JSON files in `apps/srt-gui/src/lib/i18n/locales/`.

## Verification & Quality Scripts

Run these scripts from the repository root before considering work complete:

| Script / Command | Purpose |
|---|---|
| `python3 build-scripts/quality_check.py --desktop --smoke` | Complete pre-release verification gate: Tauri version sync, workflow tests, documentation/contract checks, Svelte/TS checks, AST pattern checks, Vitest, dev-CSS, i18n audit, frontend build, headless + desktop Rust tests, and synthetic FFmpeg/APKG smoke test. |
| `python3 build-scripts/quality_check.py` | Fast headless gate (skips native `vesta` desktop package and synthetic media smoke test). |
| `cargo fmt --all --check` | Verify Rust formatting across the workspace. |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Strict Clippy lint gate across all workspace targets. |

## Contract & Hash Verification (`docs/contracts_manifest.json`)

[`build-scripts/check_doc_contracts.py`](build-scripts/check_doc_contracts.py) prevents drift between code and documentation across 6 areas:
1. **Tauri IPC commands** (`apps/srt-gui/src-tauri/src/main.rs` vs frontend `invoke(...)` and `docs/ARCHITECTURE.md`)
2. **MCP server tools** (`cli/vesta-mcp/src/lib.rs` vs `docs/MCP.md`)
3. **CLI crates & per-subcommand flags** (`cli/*/src/*.rs` vs `docs/modules/*.md`)
4. **Plugin SDK widget parity** (`PluginRenderer.svelte` `node.type` branches vs AST-extracted builders in `vesta_plugin.py` and `docs/PLUGINS.md`)
5. **Rust library public API signatures & fields** (`core/*`, `lib/*` full `pub fn`/`pub struct`/`pub enum` signatures vs `docs/modules/*.md`)
6. **Markdown links & heading anchors** across `README.md`, `AGENTS.md`, `CONTRIBUTING.md`, and `docs/**/*.md`

Commands:
- `python3 build-scripts/check_doc_contracts.py` — Verify all contracts and hashes against [`docs/contracts_manifest.json`](docs/contracts_manifest.json).
- `python3 build-scripts/check_doc_contracts.py --report` — Print a detailed JSON report of any drifted areas or modified public API hashes.
- `python3 build-scripts/check_doc_contracts.py --update` — Recompute and update `docs/contracts_manifest.json` after intentional API/CLI/IPC/doc updates.
- `python3 build-scripts/test_doc_contracts.py` — Run unit tests for the contract checker and Plugin SDK CLI.
- `python3 build-scripts/check_library_integration.py` — Verify standalone crate integration and dependency boundaries (`docs/library-integration.md`).

## Localization (i18n) Verification

All 15 locale catalogs in `apps/srt-gui/src/lib/i18n/locales/` (`ar`, `de`, `en`, `es`, `fr`, `hi`, `it`, `ja`, `ko`, `nl`, `pl`, `pt`, `ru`, `tr`, `zh`) must stay in lockstep with `en.json`:
- `bash build-scripts/check-i18n.sh` — Full localization gate (blocks on missing keys, empty values, mismatched `{placeholder}` tokens, hardcoded template strings, or untranslated English strings outside the technical allowlist).
- `node apps/srt-gui/scripts/check-i18n.mjs` — Check key parity, unused keys, and template hygiene in `apps/srt-gui`.
- `python3 build-scripts/check_missing_translations.py` — Audit untranslated or missing strings across all 15 locales.

## Plugin SDK & Headless Testing

[`docs/examples/plugin-hello/vesta_plugin.py`](docs/examples/plugin-hello/vesta_plugin.py) provides both the zero-dependency Python SDK and a headless CLI validator:
- `python3 docs/examples/plugin-hello/vesta_plugin.py init <dir> --id <id> --name "<Name>"` — Scaffold a new plugin.
- `python3 docs/examples/plugin-hello/vesta_plugin.py validate <dir>` — Validate `vesta-plugin.json`.
- `python3 docs/examples/plugin-hello/vesta_plugin.py test <dir>` — Run a headless stdio JSON-RPC smoke test (`initialize` -> `render` -> widget tree validation -> `shutdown`).

## Feature Scorecard & Continuous Audit

Vesta tracks feature coverage (GUI, CLI, MCP, Docs, Unit Tests, Playwright, Smoke), reliability controls, performance opportunities, and verified bugs across 12 areas in [`scorecard/*.toml`](scorecard/features.toml):
- [`docs/SCORECARD.md`](docs/SCORECARD.md) — Auto-generated dashboard (`python3 build-scripts/generate_scorecard.py`, validated by `quality_check.py`).
- [`docs/AUDIT_LOG.md`](docs/AUDIT_LOG.md) — Chronological audit wave history and prioritized **Wave 2 Resume Guide**.
- **Queryable SQLite cache (`build-scripts/scorecard_db.py`)**: Instead of reading large `scorecard/*.toml` files into context, query the gitignored auto-synced SQLite database at `scorecard/.cache/scorecard.db`:
  - `python3 build-scripts/scorecard_db.py summary` — Compact status totals and per-area coverage counts.
  - `python3 build-scripts/scorecard_db.py open` — Non-fixed bugs & performance items (`--area`, `--severity`, `--no-include-deferred`).
  - `python3 build-scripts/scorecard_db.py gaps --dim playwright` — Missing/partial coverage across any dimension (`gui`, `cli`, `mcp`, `docs`, `unit_tests`, `playwright`, `smoke`, `audit_status`).
  - `python3 build-scripts/scorecard_db.py show <ID>` / `query "<SQL>"` — Inspect a single item or run read-only SQLite queries (`--json` supported).
  - `python3 build-scripts/scorecard_db.py update-bug|update-perf|update-feature|update-reliability|add-bug ...` — Atomically update `scorecard/*.toml`, validate schema, refresh `scorecard.db`, and regenerate `docs/SCORECARD.md` (then verify with `python3 build-scripts/test_scorecard.py`).

## Build Artifacts & Disk Hygiene

Cargo does not automatically garbage-collect stale hashes in `target/` across toolchain updates, feature/flag changes, or multiple binaries, which can cause `target/debug` to grow past 100+ GB over time.

- **Avoid extra target directories**: Do not set custom `CARGO_TARGET_DIR` locations or leave behind git worktrees/temp clones containing Rust `target/` builds (note: `build-scripts/check_library_integration.py` reuses `target/integration`). If a task creates an isolated worktree or temporary target directory, delete its `target/` before finishing.
- **Avoid unnecessary `cargo clean`**: Do not run `cargo clean` mid-task unless diagnosing a build issue, as rebuilding `whisper-rs-sys` and Tauri dependencies from scratch is slow.
- **Keep `target/` bounded**: Periodically check `du -sh target` (and `target/integration`). If `target/` exceeds ~40 GiB, prune stale artifacts before wrapping up via `python3 build-scripts/quality_check.py --prune-target` (or `python3 build-scripts/quality_check.py --prune-only` without running the full suite), `cargo sweep -t 7` (if installed), or removing stale `target/debug/incremental` and `target/integration` directories.
