---
description: LLM instructions for Vesta development, i18n, quality gates, and release automation.
---

# Copilot Instructions for Vesta

These instructions apply to GitHub Copilot and compatible coding assistants working in this repository. Follow [`AGENTS.md`](../AGENTS.md) and [`.cursorrules`](../.cursorrules) as the authoritative reference.

## 1. Conventional Commits (Required)

Commit messages are the release notes: `@semantic-release/release-notes-generator` builds the GitHub Release body and `CHANGELOG.md` directly from commit subjects.

- Always write commit subjects and bodies in **English** using Conventional Commits (`type(scope): description`):
  - `feat:` → minor bump ("✨ New Features");
  - `fix:` → patch bump ("🐛 Bug Fixes");
  - `perf:` → patch bump ("🔧 Improvements");
  - `bump:` → patch bump ("🚀 Updated App Support");
  - `feat!:` / `fix!:` or `BREAKING CHANGE:` footer → major bump;
  - `chore:` / `docs:` / `refactor:` / `test:` / `ci:` → no release bump.
- Keep commit bodies empty or at most 1–3 concise technical lines.

## 2. 15-Locale Internationalization

Vesta supports 15 UI locales in `apps/srt-gui/src/lib/i18n/locales/`:
`ar`, `de`, `en`, `es`, `fr`, `hi`, `it`, `ja`, `ko`, `nl`, `pl`, `pt`, `ru`, `tr`, `zh`.

When modifying user-facing text:

- Never hardcode UI strings in Svelte components; use `$t("translation.key")`.
- Update all 15 locale JSON files in the same task/commit, preserving `{placeholder}` tokens.
- Keep backend Rust error strings, logs, and code comments in English.
- Run `bash build-scripts/check-i18n.sh` from the repository root to verify key parity, placeholder consistency, and template hygiene.

## 3. Architecture & Quality Boundaries

- **Core & Library Crates (`core/*`, `lib/*`)**: GUI-agnostic Rust engines with `CancellationToken` and callback progress reporting.
- **CLI & MCP (`cli/*`)**: Headless CLI adapters (`vesta-cli`, `srt-*-cli`) and the stdio MCP server (`vesta-mcp`).
- **Desktop GUI (`apps/srt-gui`)**: Svelte 5 Runes (`$state`, `$derived`, `$props`, `$effect`) + Tauri v2 IPC commands (`apps/srt-gui/src-tauri`).
- **Plugins (`plugins/*`, `docs/PLUGINS.md`)**: Out-of-process JSON-RPC 2.0 plugins rendered via the declarative 23-widget UI schema.

## 4. Verification Commands

Run from the repository root before completing changes:

- Contract & documentation check: `python3 build-scripts/check_doc_contracts.py`
- Localization gate: `bash build-scripts/check-i18n.sh`
- Fast headless gate: `python3 build-scripts/quality_check.py`
- Full desktop + smoke gate: `python3 build-scripts/quality_check.py --desktop --smoke`
- Rust formatting & lints: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings`

## 5. Release Pipeline

Releases are automated by **semantic-release** (`.github/workflows/release.yml` and `.releaserc`):

- **Push to `main`** → stable release `vX.Y.Z`, published to GitHub Releases and AUR (`aur-publish.yml`).
- **Push to `dev`** → prerelease `vX.Y.Z-dev.N`, marked as Pre-release on GitHub (skips AUR).
- `build-scripts/PKGBUILD` (`pkgver`) is the single version source of truth; `update_project_info.sh` propagates versions across `Cargo.toml`, `package.json`, `tauri.conf.json`, and Flatpak metadata. Do not bump versions manually.

