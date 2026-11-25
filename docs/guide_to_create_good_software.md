# Guide to Creating Good Software

> **A pragmatic engineering blueprint distilled from building and scaling [Vesta](../README.md).**
> None of what follows is dogma: deviate whenever a simpler choice genuinely fits the problem. What this guide captures is a repeatable **winning formula** for starting new desktop, CLI, and AI-assisted projects fast—with a rock-solid structure on day one so velocity compounds instead of collapsing under rewrites, regressions, or LLM context bloat.

---

## 1. Core Philosophy: Fast *Because* Structured

Most software slows down after the first month for four predictable reasons:
1. **Domain logic leaks into UI handlers**, making features impossible to test, script, or expose to CLI/MCP tools without launching a window.
2. **Localization, keyboard shortcuts, and accessibility are bolted on late**, forcing a painful touch-every-file migration.
3. **Documentation and feature status live in long prose files**, which silently drift from the code and burn tens of thousands of tokens when AI coding agents read them.
4. **Monolithic commits and manual releases** make regressions hard to bisect and shipping stressful.

Three invariants prevent all four:

- **Strict unidirectional layering**: `apps/*` (UI) and `cli/*` (CLI + MCP) depend on `lib/*` (headless domain engines), which depend on `core/*` (foundational formats, persistence, downloads). Lower layers never know a GUI exists.
- **One canonical source of truth per concern**: one version anchor, one crash-safe config store, one reference locale (`en.json`), one contract manifest, and one queryable feature/verification database.
- **Automated scripts & databases over human memory**: if an invariant matters (missing translation keys, broken CLI flag docs, drifted public APIs, modified "verified" code), a deterministic script checks it in milliseconds in Git hooks and CI.

---

## 2. The 8-Step Bootstrap Sequence (Day 0 Playbook)

When starting a new repository, execute these steps **in order** before building secondary screens. Setting up the rails first takes a single afternoon and saves weeks later:

1. **Workspace & Fast Compiler Tooling**:
   - Create a Cargo workspace (`resolver = "3"`, `edition = "2024"`) with empty `core/`, `lib/`, `cli/`, and `apps/` directories.
   - Configure `.cargo/config.toml` to route local Linux linking through `clang` + `mold` (cutting incremental link times by 3–5×) and configure workspace-wide `rustfmt` and strict Clippy lints (`-D warnings`).
2. **Single Version Anchor + Propagator Script**:
   - Pick **one** authoritative version field (e.g., `[workspace.package].version` in root `Cargo.toml`, or `pkgver` in `build-scripts/PKGBUILD` when shipping Arch/Linux packages as in [`update_project_info.sh`](../build-scripts/update_project_info.sh)) and write a 40-line script that propagates it everywhere else (`Cargo.lock`, desktop manifests, `package.json`) plus a CI check that rejects mismatches.
3. **Crash-Safe Configuration & Atomic File Primitives (`core/`)**:
   - Implement atomic sibling-temp file replacement (`write temp → flush → sync_all → rename`) and a serialized user-preferences store before any feature writes to disk. Once this primitive exists in `core/`, every feature inherits corruption-proof saves for free.
4. **Localization (i18n) Pipeline Before UI Component #1**:
   - Create `locales/en.json`, the `t("key", params)` lookup helper, and the automated i18n audit script *before* building views. Writing `t("nav.settings")` from minute one costs zero extra effort; retrofitting 1,500 hardcoded strings six months later is misery.
5. **The 6-Zone UI Shell & Semantic Design Tokens**:
   - Mount the empty 6-zone application shell (§4) with semantic theme tokens (never hardcoded hex colors), collapsible sidebar, in-place Settings swap, and bottom bars.
6. **Queryable Feature Database + Fine-Grained Hash Tracker**:
   - Initialize the feature/audit SQLite schema and CLI (`scripts/devdb.py`, §7) alongside the contract verifier (`scripts/check_contracts.py`, §8) and `AGENTS.md`.
7. **First End-to-End Vertical Slice**:
   - Build Feature #1 strictly bottom-up: `core` parser/model → `lib` engine + unit tests → `cli` subcommand (+ `mcp` tool if applicable) → UI screen → synthetic smoke test.
8. **CI Matrix, Git Hooks & Zero-Touch Semantic Release**:
   - Wire tracked Git hooks (`pre-commit`, `pre-push`), parallel GitHub Actions jobs, and `semantic-release` (§9) on the very first vertical slice so every subsequent `feat:` or `fix:` is shippable at any moment.

---

## 3. Modern Idiomatic Rust & Choosing the Right UI Stack

### 3.1 Rust Workspace & Engine Conventions

```text
project/
├─ Cargo.toml              # [workspace] resolver = "3", edition = "2024", shared [workspace.dependencies] & [workspace.lints]
├─ core/                   # Pure formats, parsers, atomic file persistence, cancellable downloads (minimal deps)
├─ lib/                    # Self-contained domain engines (zero UI/framework imports, zero global logger init)
├─ cli/                    # Thin clap CLIs (<project>-cli) + local stdio MCP server (<project>-mcp via rmcp)
├─ apps/                   # Desktop GUI (eframe/egui pure-Rust app OR Tauri v2 + Svelte 5 shell)
├─ scorecard/              # Declarative TOML/JSONL state + synced SQLite database (scorecard.db)
├─ build-scripts/          # Quality gates, i18n auditor, contract/hash checker, version sync, git hooks
└─ docs/                   # Concise architecture, CLI/MCP specs, contracts_manifest.json, auto-generated SCORECARD.md
```

Follow modern, idiomatic Rust practices across all crates:
- **Typed errors at library boundaries, ergonomic context in binaries**:
  - Use `thiserror` in `core/*` and `lib/*` with structured variants (distinguishing retryable rate-limit/network errors, validation errors, and cancellation) so callers can decide retry/failover policy without parsing strings.
  - Reserve `anyhow::Result` for `cli/*` entry points, build tools, and tests.
  - Forbid `.unwrap()` and unguarded indexing on runtime inputs (`#[deny(clippy::unwrap_used)]` on library production code).
- **Cooperative cancellation & bounded concurrency**:
  - Every sustained operation in `lib/*` accepts a `tokio_util::sync::CancellationToken` (or `Arc<AtomicBool>` in pure-CPU threads) and plain Rust progress/diagnostic callbacks (`Fn(ProgressEvent) + Send + Sync`).
  - Bound external subprocesses and I/O fan-out with `tokio::sync::Semaphore`, track worker tasks in `tokio::task::JoinSet` (so errors or drops abort siblings cleanly), drain child `stderr` asynchronously to avoid OS pipe deadlocks, and hold per-domain execution slots via RAII `OperationGuard` structs that release only when both work and cancellation settle.
- **Selective async & pragmatic GPU offload**:
  - Keep pure parsing, layout math, and deterministic data transforms synchronous—do not infect CPU-bound code with `async`.
  - Treat GPU acceleration as an **opt-in, runtime-probed fast path with a mandatory CPU fallback** (see Vesta's [acceleration policy](ARCHITECTURE.md#acceleration-policy--linux-media-stack)): run a tiny test probe before committing to a hardware encoder/device, isolate conflicting vendor SDKs (`vulkan`, `cuda`, `rocm`, `sycl`) into separate Cargo features or worker binaries, and ensure missing drivers never break execution.

### 3.2 UI Stack Decision: `egui` (`eframe`) vs `Tauri v2 + Svelte 5`

Pick the UI technology that matches the rendering problem rather than defaulting to a browser engine out of habit:

| Criterion | Pure Rust Native GUI (`egui` / `eframe`) | Hybrid Webview (`Tauri v2 + Svelte 5 + Tailwind v4`) |
|---|---|---|
| **Best suited for** | Fast desktop tools, editors, dashboards, developer/scientific utilities, node graphs, real-time telemetry, single-binary portable apps | Rich multi-script HTML/CSS typography (e.g., Anki HTML card previews, complex RTL/CJK layouts), browser media players, heavy CSS animations/glassmorphism |
| **Startup & RAM** | Instant (< 100 ms), **20–50 MB RAM**, **5–15 MB** single executable | 300–800 ms webview init, **150–400 MB RAM**, WebView2 / WebKitGTK system runtime required |
| **Toolchain & Security** | **100% `cargo`**—zero Node/npm/Vite, zero `node_modules` supply-chain advisories | Dual Rust + Node/Vite/TypeScript stack (`cargo audit` + `npm audit`) |
| **State & IPC** | **Zero IPC**: UI reads Rust structs directly at 60–120+ FPS; background threads send updates via `mpsc` / `watch` + `ctx.request_repaint()` | Async JSON serialization across IPC (`#[tauri::command]`); requires explicit `AsyncOperation` / `LatestRequest` race guards |
| **Visual polish effort** | Default look is utilitarian; **requires a dedicated `theme.rs` design-system module** upfront to look sleek and modern | Effortless CSS styling, Tailwind tokens, smooth transitions, and browser accessibility/RTL primitives out of the box |

**Practical rule of thumb**:
- **Start with `egui` (`eframe`) by default** whenever the app does not fundamentally need an HTML/CSS document engine or browser media stack. You eliminate half the build pipeline, all IPC serialization boilerplate, and platform webview quirks (such as Linux WebKitGTK compositing or GStreamer codec gaps), while getting buttery-smooth 120 FPS reactivity.
- **How to make `egui` look genuinely great**: never scatter raw pixel paddings or `Color32::from_rgb(...)` across panels. Build one `ui/theme.rs` module on Day 1 that configures `egui::Style` and `egui::Visuals` (generous `item_spacing`, rounded `CornerRadius::same(10)`, subtle `1px` surface borders, custom embedded Inter + Noto CJK/Arabic fonts, and semantic helper widgets like `theme::card(ui, |ui| ...)`, `theme::primary_button(...)`, `theme::sidebar_item(...)`).
- **Choose `Tauri v2 + Svelte 5 (Runes)`** when—as in Vesta—the domain revolves around HTML templates, rich international text shaping, or webview media streaming. When you do, keep `.svelte` files strictly focused on presentation, move orchestration into pure TypeScript `workflows/*.ts` with unit tests, centralize all IPC calls in one wrapper (`tauriClient.ts`), and forbid raw browser `<select>` / unguarded native dialogs.

---

## 4. The 6-Zone Application Shell ("The Vesta Layout Pattern")

A consistent spatial layout gives users immediate muscle memory and eliminates per-screen layout bikeshedding. Divide the application window into **six fixed structural zones**—two compact top headers, two central panes (a collapsible sidebar and the main workspace), and two compact bottom bars—aligned along continuous horizontal and vertical split lines:

```text
┌────────────────────────┬──────────────────────────────────────────────────────┐
│ 1. TOP-LEFT (Compact)  │ 2. TOP-RIGHT (Compact)                               │
│ [⚙ / ←] Brand + Title  │ Contextual View Header / Breadcrumbs / Quick Toolbar │
│         + [«] Collapse │ Search filter, view mode pills, or active file badge │
├────────────────────────┼──────────────────────────────────────────────────────┤
│                        │                                                      │
│ 3. CENTER-LEFT         │ 4. CENTER-RIGHT                                      │
│ Collapsible Sidebar    │ Primary Workspace Canvas                             │
│ • Main workflow tabs   │ • Single inner scroll container (outer window never  │
│ • Mode switches (e.g., │   scrolls!)                                          │
│   Easy/Expert, Offline)│ • Domain cards, tables, previews, or editors         │
│                        │                                                      │
│ ⚡ IN-PLACE SETTINGS:  │ (When Settings is active, renders the selected       │
│ Opening Settings swaps │  Settings category directly here—no stacked modals!) │
│ these items for        │                                                      │
│ Settings sections!     │                                                      │
├────────────────────────┼──────────────────────────────────────────────────────┤
│ 5. BOTTOM-LEFT (Small) │ 6. BOTTOM-RIGHT (Small)                              │
│ Avatar / Profile link  │ Secondary status, counters, or animated progress     │
│ vX.Y.Z (Inbox) • GPL   │ fill on the left  ───►  Primary Action CTA on right  │
│ [♥ Donate] [GitHub]    │                                                      │
└────────────────────────┴──────────────────────────────────────────────────────┘
```

### Core UX & Geometry Invariants

1. **Continuous Horizontal Alignment**: Zones **1 & 2** share the exact same fixed height (e.g., `56–88px`), and Zones **5 & 6** share the exact same fixed height (e.g., `48–92px`), so their top/bottom `1px` borders form unbroken lines across the window.
2. **Collapsible Left Rail with Hysteresis**: Zones **1 + 3 + 5** form a single left column (`~270px` expanded, `~72px` icon-only collapsed). Auto-collapse below a narrow width threshold (e.g., `1300px`) and auto-expand only past `threshold + 190px` so dragging the window edge never flickers—unless the user explicitly clicked the collapse toggle, in which case their manual preference wins.
3. **In-Place Settings Swap (Zero Modal Maze)**: Clicking `[⚙ Settings]` in Zone 1 does **not** open a modal dialog. Instead:
   - Zone 1 transforms into `[← Settings]` (clicking returns immediately to `last_active_main_tab`),
   - Zone 3 replaces the workflow navigation list with Settings category buttons (`Overview`, `Language`, `Shortcuts`, `Diagnostics`, …),
   - Zone 4 renders the selected Settings section.
4. **Zero Outer Scroll & Persistent View State**: The root window never scrolls; only Zone 3's nav list and Zone 4's inner body scroll independently. Switching tabs preserves in-progress state and background jobs rather than destroying them.
5. **Bottom-Left Identity Dock (Zone 5) & Bottom-Right Action Bar (Zone 6)**:
   - **Zone 5** anchors human attribution and trust without cluttering workspaces: author avatar/website, clickable version string (doubling as a release/notification inbox), license link, a subtle `[♥]` Sponsor/Donations trigger, and the GitHub repository link (collapsing cleanly to a centered icon when minimized).
   - **Zone 6** (`FooterActions`) keeps primary triggers (`Generate`, `Save`, `Cancel`) and live progress in the exact same screen corner across every tab.
6. **Progressive Disclosure & Keyboard-First Navigation**:
   - Provide an **Easy / Expert Mode** toggle near the bottom of Zone 3: Easy mode hides low-level tuning knobs and restores safe defaults; Expert mode reveals full granular controls.
   - Bind `Alt+1..9` (guarded so typing in text inputs is never hijacked) to jump across Zone 3 items in both main and Settings modes.

### Minimal `egui` Implementation Skeleton

In `egui`, this exact 6-zone geometry takes ~45 lines of idiomatic panel composition (order matters in `egui`: declare outer/side panels before `CentralPanel`):

```rust
pub fn render_shell(ctx: &egui::Context, app: &mut AppState) {
    let rail_width = if app.sidebar_collapsed { 72.0 } else { 272.0 };
    const HEADER_H: f32 = 64.0;
    const FOOTER_H: f32 = 56.0;

    // Left Column: Zones 1 (top), 5 (bottom), and 3 (center nav / in-place settings)
    egui::SidePanel::left("left_rail")
        .exact_width(rail_width)
        .resizable(false)
        .show(ctx, |ui| {
            egui::TopBottomPanel::top("zone_1_brand")
                .exact_height(HEADER_H)
                .show_inside(ui, |ui| render_brand_or_settings_back(ui, app));

            egui::TopBottomPanel::bottom("zone_5_meta")
                .exact_height(FOOTER_H)
                .show_inside(ui, |ui| render_version_profile_donate(ui, app));

            egui::CentralPanel::default().show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if app.in_settings {
                        render_settings_sections_nav(ui, app);
                    } else {
                        render_workflow_tabs_nav(ui, app);
                    }
                });
            });
        });

    // Right Column: Zones 2 (top toolbar), 6 (bottom actions), and 4 (main workspace)
    egui::TopBottomPanel::top("zone_2_toolbar")
        .exact_height(HEADER_H)
        .show(ctx, |ui| render_contextual_header(ui, app));

    egui::TopBottomPanel::bottom("zone_6_actions")
        .exact_height(FOOTER_H)
        .show(ctx, |ui| render_footer_actions_and_progress(ui, app));

    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical()
            .id_salt(app.active_view_id())
            .show(ui, |ui| render_active_workspace(ui, app));
    });
}
```

---

## 5. Scripts vs. Declarative Files vs. Databases: Choosing the Right Medium

A recurring architectural question is *where* project metadata—translations, feature matrices, verification state, API contracts, and changelogs—should live: plain text files, validation scripts, or a SQLite database?

Each medium has a sharp failure mode when misused:
- **Large Markdown/TOML/JSON files alone** are great for Git diffs, but terrible when AI agents must read 3,000 lines into context just to answer *"which CLI features still lack smoke tests?"*
- **Raw binary SQLite `.db` files alone** are instant to query with SQL, but produce opaque binary merge conflicts in Git whenever two branches or two parallel agents touch different rows.

The winning pattern is **purpose-matched storage paired with deterministic Python/Rust CLI gates**:

| Information Kind | Best Storage Pattern | Why |
|---|---|---|
| **UI Translations (i18n)** | **Sorted flat JSON per locale** (`locales/en.json`, `locales/it.json`, …) + **AST/parity audit scripts** | Alphabetically sorted 1-line `"key": "value"` entries merge cleanly across branches, lazy-load or embed directly into binaries, and let scripts diff keys/placeholders in `< 1s`. |
| **Features, Test Matrix, Bugs, Perf & Code Hashes** | **Hybrid: Canonical TOML/JSONL + Synced SQLite (`scorecard.db`) + Query/Mutation CLI** | Parallel agents mutate disjoint lines in deterministic text files (conflict-free Git merges) while querying compact SQLite views (`summary`, `gaps`, `stale`, `impact`) in ~150 tokens instead of reading huge files. |
| **Cross-Layer Public Contracts (CLI, IPC, MCP, Public APIs)** | **Canonical JSON hash manifest** (`docs/contracts_manifest.json`) + **AST extractor script** | Deterministic structural hashes catch undocumented flag/signature changes immediately in CI without manual bookkeeping. |
| **Release History & Versioning** | **Conventional Commit history** + **Single version anchor** + **`semantic-release`** | Eliminates hand-edited `CHANGELOG.md` merge conflicts and human version-bump mistakes. |

---

## 6. Localization (i18n) from Day One & Automated Audit Gates

Never write human-facing copy directly inside a component (`ui.button("Generate")` or `<button>Generate</button>`). Every visible string must pass through a semantic translation key (`t("flashcards.generate")`).

### 6.1 Catalog Structure & Runtime Rules
- **Flat, dot-namespaced keys sorted alphabetically**: `"common.cancel"`, `"nav.settings"`, `"sync.anchor.added"`. Flat keys are trivial to grep, sort deterministically, and never require deep JSON merge logic.
- **`en.json` is the canonical schema and runtime fallback**:
  - In **Tauri/Web**: bundle only `en.json` statically in the startup chunk; load the active language chunk on demand (`{ ...en, ...dict }`) so startup never pays for 15+ dictionaries, and missing keys fall back to English rather than raw key IDs (see `apps/vesta-gui/src/lib/i18n/index.ts`).
  - In **`egui`/Rust**: parse `en.json` at compile time (generating an enum or verified `&'static str` key table so typos fail `cargo check`!) and merge the selected locale JSON over `en` in memory; load Noto fallback fonts dynamically when switching to CJK, Hindi, or Arabic (`ar`).
- **RTL readiness**: never hardcode physical left/right margins in web CSS where flow direction matters—use logical properties (`ps-*`, `pe-*`, `border-e`, `inset-inline-start`) and toggle `dir="rtl"` when `lang == "ar"`.

### 6.2 How to Build the Automated Periodic/CI i18n Checker

Wire a single entry-point script (like [`build-scripts/check-i18n.sh`](../build-scripts/check-i18n.sh) wrapping a fast AST scanner + catalog auditor) into `pre-commit`, `ci.yml`, and optional scheduled audits. It should enforce **six automated checks**:

1. **Exact Key Parity**:
   - Every `<lang>.json` must have `keys(lang) == keys(en)`. Report both **missing keys** (`in en, missing in lang`) and **orphan keys** (`in lang, deleted from en`).
2. **Interpolation Placeholder Equality**:
   - Extract placeholder tokens (e.g., `/\{\{(\w+)\}\}|\{(\w+)\}/g`) from `en[key]` and `lang[key]`. Fail immediately if sets differ (`en: {count, file}` vs `it: {count}`), preventing broken runtime interpolation or silent missing variables.
3. **Empty, Whitespace & Invisible-Character Hygiene**:
   - Reject blank strings, accidental leading/trailing spaces (`val != val.strip()`), and pasted zero-width Unicode characters (`U+200B` ZWSP, `U+FEFF` BOM, `U+00AD` soft hyphen).
4. **Untranslated Copy Detection (`same_as_english`) with Scoped Allowlists**:
   - Flag any key where `lang != "en"` and `lang[key] == en[key]`, **except** keys explicitly listed in:
     - a **global technical/brand allowlist** (`"FFmpeg"`, `"Opus"`, `"GPL-3.0"`, `"OpenRouter"`), or
     - a **per-language cognate allowlist** (`"Manual"` in `es`/`pt`, `"System"` in `de`/`pl`, `"File:"` in `it`).
   - This catches forgotten copy-pasted English strings that naive key-existence checks miss completely!
5. **Git-Blame Staleness Check (`--check-stale`)**:
   - Periodically (and in CI with `fetch-depth: 0`), compare the last commit timestamp modifying `key` in `en.json` against `<lang>.json`. When an English string's wording changes substantially without a matching update in `<lang>.json`, report it so translations never lag behind rewritten English UI text.
6. **AST Source Scanner (Hardcoded Copy, Dead Keys & Ternary Hacks)**:
   - Parse `.svelte`/`.ts` (or `.rs` widget calls in `egui`) to verify:
     - Every `t("...")` key referenced in source exists in `en.json`.
     - Unused keys in `en.json` are surfaced for cleanup.
     - Raw literal strings inside UI elements and bilingual shortcuts (`if lang == "it" { ... } else { ... }`) fail the check.

---

## 7. Feature Database & Fine-Grained Cryptographic Code Hashing

When multiple AI coding sessions (or developers) work across features over weeks, two problems emerge:
1. **Context Bloat & Drift**: Loading massive markdown checklists into every agent prompt wastes context tokens and quickly goes out of sync.
2. **Silent Invalidation of "Verified" Code**: Marking feature `FC-08` as `verified` on Monday means nothing on Thursday if Wednesday's bugfix altered a helper function inside `FC-08`'s call path without re-verifying `FC-08`. Conversely, hashing **whole files** is too coarse: fixing 3 lines in one function inside a 1,200-line file would invalidate *every* feature touching that file, forcing wasteful re-audits of thousands of unchanged lines.

### 7.1 The Multi-Dimensional SQLite Scorecard (See [`scorecard_db.py`](../build-scripts/scorecard_db.py))

Store project tracking in a relational SQLite database (`scorecard/scorecard.db`) mirrored from deterministic text files (`scorecard/*.toml` + `scorecard/symbol_hashes.jsonl`):
- **`features`**: `id` (`FC-01`), `area`, `feature`, interface & test coverage columns (`gui`, `cli`, `mcp`, `docs`, `unit_tests`, `e2e`, `smoke` ∈ `{yes, partial, no, n/a}`), `audit_status` ∈ `{verified, stale, partial, unverified}`, `last_audited`, `notes`.
- **`reliability` / `performance` / `bugs`**: tracked engineering controls, active optimization targets, and open findings with severity and target symbols.
- **`impact_rules`**: maps file/symbol patterns to `feature_ids` and the exact targeted `cargo test -p ...`, Vitest, or E2E commands that exercise them.

Agents never load the raw tables into context; they run millisecond CLI queries:
```bash
python3 scripts/devdb.py summary              # 15-line overview across all areas
python3 scripts/devdb.py gaps --dim smoke     # Only features missing smoke coverage
python3 scripts/devdb.py stale                # Verified features whose bound code hashes changed!
python3 scripts/devdb.py impact --staged --run # Map staged edits -> affected features & run only their tests
```

### 7.2 Fine-Grained Symbol/AST Merkle Hashing (Pinpoint Integrity Without False Positives)

To guarantee that `audit_status = 'verified'` is **cryptographically trustworthy** without re-checking thousands of lines after minor edits, hash **normalized AST symbols** rather than raw files:

```text
Crate / Module Rollup Hash
 ├── Production Symbol Hashes (normalized, comments & #[cfg(test)] stripped)
 │    ├── lib/audio.rs::fn::extract_clip        -> blake3: 9f41c8b20a11e4d0  [bound: FC-08]
 │    ├── lib/audio.rs::fn::apply_padding       -> blake3: 3a02d719c480f112  [bound: FC-08]  ◄── 4 lines edited!
 │    └── lib/audio.rs::fn::probe_streams       -> blake3: 77e19b04a2218c93  [bound: EX-02]  (unchanged -> EX-02 stays verified!)
 └── Test Unit Hashes (hashed separately!)
      └── lib/audio.rs::test::padding_clamps    -> blake3: 51c908ef1120ab44  [bound: FC-08]
```

#### How the 4-Step Mechanism Works

1. **Token-Normalized Symbol Extraction (`scripts/hash_symbols.py`)**:
   - Parse each `.rs` (via `syn` / `tree-sitter` or brace-balanced item scanner), `.ts`, or `.svelte` file into discrete addressable units:
     - Rust: `pub`/`priv` `fn`, `struct`, `enum`, `trait`, and individual `impl Type::method` blocks (`path::kind::qualname` + start/end line numbers).
     - Svelte/TS: exported functions, classes, and `<script>` / template blocks per component.
   - **Canonicalize before hashing**: strip `//` and `/* */` comments, separate `#[cfg(test)]` modules into `test` units, and collapse whitespace between tokens.
   - **Result**: running `cargo fmt`, fixing a comment typo, adding blank lines, or adding a brand-new `#[test]` at the bottom of a file produces **zero production symbol hash drift**.

2. **Lightweight Symbol-to-Feature Binding**:
   - Default binding comes from `impact_map.toml` path/symbol globs (e.g., `lib/vesta-sync/src/anchors.rs::*` → `SY-01`).
   - When a large file serves multiple features, annotate individual functions or structs with a zero-overhead comment tag (`// @feature(FC-08)` or `// @feature(FC-08, FC-05)`), which takes precedence over the file-level default.

3. **SQLite Hash Ledger Schema & Automatic Staleness Detection**:
   ```sql
   CREATE TABLE feature_symbol_hashes (
       feature_id   TEXT NOT NULL REFERENCES features(id),
       symbol_id    TEXT NOT NULL, -- e.g. "lib/audio.rs::fn::apply_padding"
       role         TEXT NOT NULL CHECK (role IN ('prod', 'test')),
       content_hash TEXT NOT NULL, -- 16-char BLAKE3 / SHA-256 hex digest
       verified_at  TEXT NOT NULL,
       PRIMARY KEY (feature_id, symbol_id)
   );
   ```
   - When an agent audits and verifies `FC-08` (`devdb mark-verified FC-08`), the CLI snapshots the current `content_hash` of every `prod` and `test` symbol bound to `FC-08` into `feature_symbol_hashes` (and writes sorted `scorecard/symbol_hashes.jsonl` for clean Git diffs).
   - Whenever code is modified (`devdb check-hashes` in pre-commit / CI / agent startup):
     - **Unchanged symbols**: features stay `verified`.
     - **Only new `test` symbols added**: `prod` hashes still match → feature stays `verified`!
     - **Existing `test` deleted or modified while `prod` unchanged**: flagged as `test_drifted` (verify test coverage wasn't weakened).
     - **1+ `prod` symbols modified or deleted**: **only** the feature(s) bound to those exact symbols are flagged `stale` (`needs_recheck`), and the CLI prints the exact surgical scope to review:
       ```text
       STALE FEATURE: FC-08 (Audio clip padding & gain)
         Changed prod symbols (1 of 14):
           • lib/audio.rs::fn::apply_padding (lines 84–112) [hash 3a02d719 -> 8c91e04f]
         Targeted re-verification command:
           cargo test --locked -p my-audio padding
       ```
       Instead of re-reading 14 files, the next agent inspects **lines 84–112**, runs the targeted test, and runs `devdb mark-verified FC-08` to refresh that symbol's hash!

4. **Conflict-Free Parallel Agent Workflow**:
   - Agent A (working on `FC-08` in branch/worktree A) and Agent B (working on `SY-01` in branch/worktree B) never fight over a binary lock: both mutate sorted records in `features.toml` and `symbol_hashes.jsonl`. On merge, `devdb`'s `ensure_db()` sees newer text inputs and rebuilds `scorecard.db` in `< 30 ms`.

---

## 8. Cross-Layer Contract Enforcement & Code Hygiene

### 8.1 Automated Contract & Link Verification (See [`check_doc_contracts.py`](../build-scripts/check_doc_contracts.py))
Documentation rots the day nobody checks it mechanically. Maintain a single fast Python script (`< 2s` runtime) enforced in CI that cross-checks code against docs and `docs/contracts_manifest.json`:
1. **Public Rust APIs**: extract `pub fn`, `pub struct`, and `pub enum` signatures across `core/*` and `lib/*`; fail if a crate's public API hash changes without reviewing its `docs/modules/<crate>.md` and updating the manifest.
2. **CLI Subcommands & Flags**: parse `#[arg(...)]` / `clap` structs and verify that every `--flag` used in Markdown examples actually exists on that subcommand.
3. **MCP Tool Schemas**: verify every `#[tool]` in `*-mcp` is documented in `docs/MCP.md` and `docs/ARCHITECTURE.md`.
4. **UI / IPC / Plugin Parity**: verify every frontend IPC `invoke("cmd")` matches a registered backend handler (and browser-test mocks match Rust parameter names), and every declarative plugin widget type exists across renderer, SDK, and spec.
5. **Markdown Relative Links & Anchors**: verify every `[link](path.md#heading)` across `README.md`, `AGENTS.md`, and `docs/**/*.md` points to a real, non-gitignored file and valid slugified heading.

### 8.2 Clean Code, Security & Disk Hygiene Rules
- **Small, single-responsibility modules**: split UI rendering from workflow orchestration; split network/retry scheduling from file persistence; extract shared UI primitives (`FooterActions`, `SearchableSelect`, `ConfirmDialog`) the second time a pattern appears, never the fifth.
- **Static AST anti-pattern linters**: write tiny AST checks (`check-code-patterns`) to ban recurring footguns (`debugger`, `array.forEach(async ...)`, `new Promise(async ...)`, raw `<select>`, hardcoded language checks).
- **Security by construction**:
  - Route external HTTP discovery through a backend client enforcing HTTPS, origin-bound redirects, response-size caps (e.g., 8 MiB), and private/link-local SSRF blocks (`127.0.0.0/8`, `169.254.0.0/16`, `fe80::/10`).
  - Canonicalize paths against explicit user-approved roots (`MediaAllowlist`, MCP `--read-root` / `--output-root`), rejecting symlinks escaping the boundary.
  - Guard native OS file dialogs with an in-flight mutex (`dialogGuard`) so rapid double-clicks cannot hang desktop portals.
- **Cargo `target/` hygiene**: Cargo never garbage-collects old incremental/feature hashes on its own. Avoid custom `CARGO_TARGET_DIR` sprawl, delete temporary worktree `target/` folders immediately after use, avoid mid-task `cargo clean`, and prune `target/debug/incremental` + stale build-script dirs whenever `target/` approaches ~40 GiB (`quality_check.py --prune-only`).

---

## 9. Atomic Commit Discipline, GitHub Actions CI/CD & Semantic Release

### 9.1 Small, Self-Contained Conventional Commits

Treat Git history as an **executable engineering log and rollback safety net**, not a scratchpad:

- **One self-contained logical change per commit**:
  - Prefer **5 small, focused commits** (`refactor(sync): extract anchor interpolation helper` → `feat(sync): support piecewise-linear drift anchors` → `feat(cli): add --anchor repeatable flag to sync-cli` → `feat(gui): wire multi-anchor timeline controls` → `test(sync): add drift regression fixtures`) over one 1,800-line mega-commit.
  - Every commit should compile and pass tests on its own. When a subtle regression surfaces weeks later, `git bisect` isolates the exact 40-line commit in seconds, and `git revert <sha>` backs it out surgically without entangling unrelated UI or i18n work.
- **Conventional Commits drive both SemVer and End-User Release Notes**:
  - Always write commit subjects and bodies in **English** using `type(scope): concise user-meaningful description`:
    - `feat:` → **Minor** version bump (`0.x.0`), published under *New Features*.
    - `fix:` → **Patch** version bump (`0.0.x`), published under *Bug Fixes*.
    - `perf:` → **Patch** version bump (`0.0.x`), published under *Improvements*.
    - `feat!:` / `fix!:` or `BREAKING CHANGE:` → **Major** version bump (`x.0.0`).
    - `refactor:`, `docs:`, `test:`, `chore:`, `ci:`, `build:` → **No release bump** (internal engineering commits stay out of user release notes).
- **High-signal subjects, minimal bodies**:
  - Write subjects in imperative mood under ~72 chars describing *what changed and why it matters*, never vague filler (`fix stuff`, `update files`).
  - Keep bodies **empty** when the diff is self-explanatory, or **1–3 concise lines** capturing a non-obvious root cause, invariant, or benchmarked trade-off. Never paste AI-generated bullet walls into commit messages.

### 9.2 Multi-Stage GitHub Actions CI Architecture (See [`ci.yml`](../.github/workflows/ci.yml))

Structure CI so **cheap structural errors fail in 20 seconds**, before spending runner minutes on heavy Rust/C++ builds:

```text
Push / PR
 ├─► 1. Fast Gates (< 30s, Python/Node only)
 │    • Version sync, doc/API contracts, markdown links, scorecard & code-hash check
 │    • i18n key parity, placeholders, hygiene, git-blame staleness
 ├─► 2. Security Audit (parallel)
 │    • `cargo audit` + `npm audit` against committed lockfiles
 ├─► 3. Headless Engines & CLIs (parallel, CPU-only Ubuntu, clang + mold + rust-cache)
 │    • `cargo test --locked` across all `core/*`, `lib/*`, `cli/*` (no GUI/GPU deps!)
 │    • External downstream consumer check (`--individual` per library crate)
 ├─► 4. Full Workspace Clippy & Desktop Tests (gated on Fast Gates, sccache + mold)
 │    • `cargo clippy --locked --workspace --all-targets -- -D warnings`
 │    • `cargo test --locked --workspace` + synthetic FFmpeg/output smoke test
 └─► 5. Frontend / E2E Regression Suite (parallel)
      • Type check, AST pattern linter, unit tests, browser/UI layout & RTL specs
```

Key CI & Hook practices:
- **Concurrency cancellation on PRs, never on `main`/`dev`/`release`**: set `cancel-in-progress: ${{ github.event_name == 'pull_request' }}` so rapid PR pushes save runners while branch/release runs always finish cleanly.
- **Index-aware Git hooks ([`pre-commit`](../build-scripts/pre-commit), [`pre-push`](../build-scripts/pre-push))**: validate the **staged Git index** (on commit) and **each outgoing commit snapshot** (on push), never the dirty working tree—otherwise an unstaged local fix can mask a broken commit being pushed!

### 9.3 Zero-Touch Semantic Release & Cross-Platform Packaging (See [`release.yml`](../.github/workflows/release.yml) & [`.releaserc`](../.releaserc))

Never edit version numbers, changelogs, or Git tags by hand:

1. **Branch Strategy**:
   - Day-to-day atomic commits are pushed to `dev`, automatically running full CI and—when releasable `feat:` / `fix:` / `perf:` commits are present—publishing a `vX.Y.Z-dev.N` GitHub Pre-release with full cross-platform installers (skipping stable-only channels like AUR and keeping the `Latest` badge on stable).
   - Merging `dev` → `main` triggers automatic stable publication (`vX.Y.Z`).
2. **Exact-SHA CI Barrier Before Tagging**:
   - `release.yml` queries GitHub Actions for the `push` `ci.yml` run matching the **exact `RELEASE_SHA`** and waits for `conclusion == "success"`. If CI failed, is missing, or was skipped, `semantic-release` refuses to run.
3. **Automated Prepare, Tag & Asset-Gated Publication**:
   - `semantic-release` analyzes commits since the last tag, computes the next SemVer version, runs the version propagator script (`prepareCmd`), commits the synced manifests (`chore: Release vX.Y.Z [skip ci]`), creates the Git tag + GitHub Release notes, and dispatches the cross-platform `build.yml` matrix (Linux `.deb`/`.rpm`/Arch/Flatpak, Windows `x64`/`ARM64`, macOS Apple Silicon `.dmg`).
   - **Pre-release hold until installers finish**: `release.yml` immediately marks the new stable GitHub Release as `--prerelease` while `build.yml` compiles and uploads binary installers + SHA-256 digests; only after all platform artifacts are uploaded and verified does `build.yml` flip the release to **Latest** (and trigger downstream package managers such as AUR), preventing users or in-app updaters from ever seeing an empty release tag.
   - Finally, `@cleyrop-org/semantic-release-backmerge` merges the release commit back from `main` into `dev` automatically so branch versions never diverge.

---

## 10. Quick Checklist for Starting Your Next Project

- [ ] Cargo workspace split into `core/`, `lib/`, `cli/` (+ `mcp`), and `apps/` (`egui` by default; `Tauri + Svelte` when HTML/media demands it)
- [ ] `.cargo/config.toml` (`clang` + `mold`), `rustfmt`, and `cargo clippy -- -D warnings` active from commit #1
- [ ] Single version anchor + `update_project_info` sync script + `semantic-release` config
- [ ] Atomic file writer + serialized config store in `core/`
- [ ] Semantic theme module + **6-Zone Shell** (2 top headers, collapsible sidebar with in-place Settings swap + inner-scroll workspace, bottom-left attribution/version/donate dock + bottom-right action/progress bar)
- [ ] `locales/en.json` + `t("key")` helper + 6-check i18n auditor (parity, placeholders, invisible chars, `same_as_english` allowlists, `git blame` staleness, AST hardcoded-copy scanner)
- [ ] `scorecard/` (`features.toml` + `symbol_hashes.jsonl` + synced `scorecard.db`) with normalized AST symbol hashing so `verified` status automatically becomes `stale` on touched functions only
- [ ] `check_doc_contracts.py` verifying public Rust signatures, CLI flags, MCP tools, UI/IPC calls, and Markdown links/anchors
- [ ] Index-aware `pre-commit`/`pre-push` hooks, staged fast-fail `ci.yml`, and SHA-gated `release.yml`
