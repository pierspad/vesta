# Reliability and component audit — 14 October 2026

## Architectural assessment

The existing `core → lib → cli/apps` split is worth preserving. No feature engine
imports Tauri or a GUI component. The typed frontend services already isolate
IPC; workflow modules provide testable orchestration. Moving everything into a
generic manager would obscure ownership rather than improve reuse.

| Crate | Reuse and CLI assessment | Priority |
|---|---|---|
| `srt-parser` | Foundational format library; extraction CLI already exposes parsed output. A second parser binary would duplicate that interface. | Keep independent; test encodings and invalid timestamps. |
| `srt-apkg` | Foundational archive API; flashcards/refine are its workflow consumers. | Keep archive validation and publication in the library. |
| `srt-download` | Shared cancellable, atomic download primitive, with progress supplied by the caller. | No UI notification responsibility; retain local HTTP tests. |
| `srt-extract` | Standalone library and CLI; embedded extraction stays in the engine. | Fixed stdout contamination, unknown formats and Unicode statistics. |
| `srt-sync` | Manual interpolation engine, separate matching/playback modules. | Added manual CLI and overflow regression; playback cancellation remains separate work. |
| `srt-autosync` | CLI wraps the shared speech/alignment orchestration. Depends on native Whisper. | Keep CPU and GPU builds distinct; test failed segments/cancellation. |
| `srt-transcribe` | CLI and GUI share pipeline/model/audio/cloud implementation. Native Whisper is always a compile dependency, even for cloud consumers. | A future optional local feature could reduce cloud-only build requirements; this needs a dedicated compatibility change. |
| `srt-translate` | Shared pool, scheduler, retry and translation engine; CLI exists. | Provider client now has typed safe errors and optional retry telemetry; other engine crates still merit a structured-diagnostics pass. |
| `srt-flashcards` | Headless generation with callbacks/cancellation and its own CLI. | Retain real FFmpeg/export/cancellation tests; avoid splitting orchestration merely by file size. |
| `srt-refine` | Reusable card I/O and tiered refinement, now with inspect/enrich CLI. | Complete-only publication by default, explicit partial policy, real SIGINT and local provider tests. |
| `srt-ankiconnect` | HTTP client reused through workflow callers. Anki is a runtime service, not a GUI crate dependency. | Prefer API reuse; a binary only makes sense with a concrete import/query workflow. |

## Shared frontend responsibilities

- `i18n/index.ts` owns UI dictionary loading and locale selection. In-flight
  loads are reused and stale completions cannot override the latest selection.
- `config/languages.ts` owns study-language names, aliases and normalization.
  `getLanguageOptions(locale)` adapts that metadata into selector options without
  importing stores or duplicating language detection.
- `snackbarStore.svelte.ts` owns lifecycle; `Snackbar.svelte` owns presentation.
  Short-message defaults and replacement semantics remain compatible. Long
  messages get more reading time; duration zero persists; hover/focus pause the
  remaining countdown, including overlapping interactions.
- `utils/errorMessage.ts` gives readable, credential-redacted rejection details.
  `utils/clipboard.ts` accepts a writer and notifier so UI components do not
  report success before clipboard completion. IPC preserves original errors;
  `supportLog.ts` treats diagnostic recording as best effort.

The new snackbar uses a neutral dark surface with semantic accents, wrapping for
long paths, internal scrolling for tall content, safe-area margins, a keyboard
focus indicator and no entrance animation under reduced-motion preferences.
It sits above ordinary dialogs so their errors remain visible. The shortcut
overlay retains its higher layer. This does not guarantee delivery of every
notification during bursts: the single-slot replacement policy is deliberate.
A future bounded queue needs priorities and duplicate suppression, not an
unbounded array.

## Testing priorities and evidence

More tests are useful when they protect observable contracts, rather than repeat
implementation details. Added regressions cover binary stdout parsing, invalid
formats, preservation of existing files on failed input/anchors, Unicode text,
manual drift, extreme retiming, persistent/invalid/paused snackbar durations,
async locale races, readable errors, failed clipboard writes and log-sink failure.
The real IPC adapter is exercised with only the native transport mocked.

The browser preview exercised the actual Svelte snackbar at 700 × 420 and
280 × 240. In the latter, the panel was x=16…264 and y=16…224: 16 px on every
edge. Long content had 390 px of scrollable content inside a 206 px content
viewport. This is Chromium validation; installed WebKitGTK/native window smoke
testing remains useful, especially for playback and platform dialogs.

Initial-pass local validation:

| Check | Result |
|---|---|
| `npm test` | 256 tests passed in 47 files |
| `npm run check` | 0 errors, 0 warnings |
| `npm run build` | Passed |
| `npm run test:dev-css` | Cold/repeated requests and server restart passed |
| `cargo test --workspace --no-fail-fast` | 270 tests passed |
| Package-selected headless test command from CI | 220 tests passed, default CPU Whisper |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed |
| `cargo fmt --all --check`, `git diff --check`, internal version check | Passed |
| Synthetic CLI/APKG smoke, one debug sample | 10 notes, 20 nonempty media files; ZIP/SQLite integrity passed |
| `npm run check:i18n` | Existing dictionary work: 10 missing keys, 22 warnings |

The smoke run verifies correctness, not a performance improvement. Node 26 emits
a dependency deprecation warning for `module.register()` during frontend checks;
this is separate from Svelte/TypeScript diagnostics. The temporary browser preview
ran outside Tauri and emitted expected IPC-unavailable configuration errors; it
validated component rendering and interactions rather than native persistence.

CI now has a separate package-selected headless job with no Node, GTK/WebKit or
Vulkan SDK installation. It tests the default CPU engine/CLI dependency graph;
the full desktop workspace job retains GPU/desktop coverage. The CI job itself
must still run on GitHub to validate the clean Ubuntu environment.

Existing untracked dictionary components were preserved. Their missing selector
adapter was supplied from the common language module, allowing TypeScript checks
to pass. The localization audit still reports ten missing dictionary keys and
hardcoded copy in that preexisting work; this is an explicit remaining issue.

## Forking and integration

Prefer a Git dependency for a library or a package-selected Cargo build for a
binary. Copying a crate directory alone does **not** produce a standalone Cargo
project: manifests inherit edition, Rust 1.99 and dependencies from the root.
Transcription/autosync additionally need the vendor Whisper patch and its native
toolchain. Preserve the dependency closure and root patch, or make inherited
manifest fields explicit in the receiving workspace. License obligations remain
those of each crate; this audit does not change licenses or publish packages.

No blanket error-type migration was made: changing `Result<_, String>` into a
shared typed error across IPC and Rust APIs needs versioning and consumer
migration. The next architectural priority is a stable domain error schema with
codes, safe user text and optional diagnostics, introduced one workflow at a time.

## Follow-up: refinement CLI and provider contract

The second implementation pass added standalone deck inspection and annotation.
The CLI validates conversion/configuration before requesting a provider, preserves
existing output on incomplete/default runs and cancellation, and offers an
explicit partial-publication option that still reports failure through its exit
code. Tests cover success, malformed secret-bearing config, Unicode fields,
partial work, batch execution, invalid conversion and real Unix SIGINT.

The translation provider client now returns `ProviderError` with safe categories,
status and provider family; it rejects failed HTTP responses even when the body
resembles success. Empty/quote-only translations, empty batch items, duplicate IDs
and invalid legacy counts cannot silently succeed. Error bodies and credential
URLs are not echoed. An optional numeric retry observer is retained by clones;
desktop adapters and the support recorder keep that metadata without adding
snackbar noise. The translation CLI's TOML parser also reports an error location
without printing credential-bearing source lines.

Independent review exposed the quote-normalization gap; tests reproduced it,
the fix was verified, and the follow-up review found no remaining actionable issue.
This adds a stable error schema at one provider boundary, preserving existing
outer `anyhow`/String signatures rather than forcing every consumer to migrate.

Follow-up validation: 286 workspace Rust tests, 236 package-selected headless
tests and 257 frontend tests passed. Strict workspace Clippy, Svelte/TypeScript
checking, production build and formatting/diff checks passed. The dependency
graph of `srt-refine-cli` contains no Tauri, GTK or Whisper. The existing
dictionary localization gap from the initial pass is unchanged.
