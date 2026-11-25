---
description: LLM instructions for Vesta development, i18n, quality gates, and release automation.
---

# Copilot Instructions for Vesta

Follow [`AGENTS.md`](../AGENTS.md) and [`.cursorrules`](../.cursorrules) as the authoritative engineering reference, alongside [`docs/ARCHITECTURE.md`](../docs/ARCHITECTURE.md) and [`docs/QUALITY.md`](../docs/QUALITY.md).

## 1. Conventional Commits (Required)

`@semantic-release/release-notes-generator` builds GitHub Release notes directly from commit subjects:
- Write all commit subjects, bodies, Rust errors, logs, and comments in **English** (`type(scope): description`):
  - `feat:` → minor bump ("✨ New Features"); `fix:` → patch bump ("🐛 Bug Fixes"); `perf:` → patch bump ("🔧 Improvements"); `bump:` → patch bump ("🚀 Updated App Support"); `feat!:` / `BREAKING CHANGE:` → major bump; `chore:` / `docs:` / `refactor:` / `test:` / `ci:` → no release bump.
- Keep commit bodies empty or at most 1–3 concise technical lines.

## 2. 15-Locale Internationalization

Vesta supports 15 locales in `apps/vesta-gui/src/lib/i18n/locales/` (`ar`, `de`, `en`, `es`, `fr`, `hi`, `it`, `ja`, `ko`, `nl`, `pl`, `pt`, `ru`, `tr`, `zh`):
- Never hardcode UI text in Svelte components; derive `let t = $derived($locale);` and call `t("translation.key", { param })`.
- Update all 15 locale JSON files together, preserving `{{placeholder}}` / `{placeholder}` tokens, and verify with `bash build-scripts/check-i18n.sh`.

## 3. Architecture Boundaries

- **Core & Engine Crates (`core/*`, `lib/*`)**: Headless Rust crates using `CancellationToken` and progress callbacks; zero Tauri/GUI coupling.
- **CLI & MCP (`cli/*`)**: Unified `vesta-cli`, standalone `vesta-*-cli` binaries, and `vesta-mcp` (24 stdio tools; see `docs/MCP.md`).
- **Desktop GUI (`apps/vesta-gui`)**: Svelte 5 Runes (`$state`, `$derived`, `$props`, `$effect`) + Tauri v2 (`apps/vesta-gui/src-tauri`).
- **Plugins (`apps/vesta-gui/src-tauri/src/plugins/`, `docs/PLUGINS.md`)**: Out-of-process JSON-RPC 2.0 stdio plugins rendered via Vesta's 26-widget declarative schema (`PluginRenderer.svelte`).

## 4. Verification & Release Governance

- Verify changes from repo root: `python3 build-scripts/check_doc_contracts.py`, `bash build-scripts/check-i18n.sh`, `python3 build-scripts/scorecard_db.py check`, `python3 build-scripts/quality_check.py` (`--desktop --smoke` pre-release), and `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings`.
- `build-scripts/PKGBUILD` (`pkgver`) is the sole version source of truth (`update_project_info.sh` propagates it; never edit versions manually).
- Pushing to `main` triggers an automated stable release (`vX.Y.Z`, published to GitHub Releases and AUR) and pushing to `dev` triggers an automated pre-release (`vX.Y.Z-dev.N`, skipping AUR), both gated on `ci.yml` passing for that exact commit SHA.
