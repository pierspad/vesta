# Vesta desktop app

Tauri 2 desktop interface for the Vesta subtitle and flashcard toolchain. The frontend uses Svelte 5, TypeScript, Vite, and Tailwind CSS; native commands live in `src-tauri`.

```bash
npm ci
npm run check
npm test
npm run tauri -- dev
```

Build the production frontend with `npm run build`, or the complete desktop bundle with `npm run tauri -- build`.

Node.js 22.12+ is required. Install the native dependencies listed in the
[root README](../../README.md#building-from-source) before starting Tauri.

## Runtime map

`src/App.svelte` loads tabs lazily and keeps them mounted. `src/lib/stores`
holds workflow state; `src/lib/services` wraps native IPC/HTTP; `src/lib/config`
holds provider catalogs and persisted preferences. `src-tauri/src/commands`
adapts GUI requests to the workspace libraries. The desktop process also owns
loopback media streaming and opening exported files through system associations.

Use **Extract subtitles** for text tracks embedded in MKV/MP4 containers.
The onboarding wizard awaits durable persistence before reloading. Generation
uses source snapshots directly and reports real FFmpeg preparation progress for
video clips. Export filenames open the saved file; the adjacent folder button
opens its directory. `.apkg` opening requires an Anki file association.

From the repository root, run `python3 build-scripts/quality_check.py --desktop --smoke`.
See [architecture](../../docs/ARCHITECTURE.md), [quality gates](../../docs/QUALITY.md)
and [the dated audit](../../docs/STABILITY_AUDIT.md) for test scope and limitations.

## Frontend workflow boundaries

`src/lib/tabs/FlashcardsTab.svelte` and `TranscribeTab.svelte` compose controls,
settings and desktop event subscriptions. They delegate execution to
`src/lib/workflows`:

- `flashcardSeries.ts`: episode configuration, sequential generation, totals,
  package merging and final import. Output choices are captured for the run.
- `flashcardGeneration.ts`: single generation, response mapping and cleanup.
- `flashcardFileDiscovery.ts`: best-effort companion discovery, capped at four
  concurrent requests with deterministic ordering and deduplication.
- `transcription.ts`: endpoint readiness, credential resolution, local-only
  options and sequential failover. Cancellation suppresses retries and late results.
- `transcriptionResources.svelte.ts`: reactive backend/model/VAD discovery;
  version checks prevent older requests overwriting newer state or disposed tabs.

Workflows inject IPC and callbacks so their error, cancellation and partial-success
paths can be tested in Node without a desktop window or provider charges.
`TranscriptionSegmentsPanel.svelte` owns only live transcript presentation and
scrolling; path and timestamp transformations live in pure utilities.

Keep UI-specific dialogs, keyboard handling and element references in components.
Put execution policy in workflows and payload/format transformations in utilities.
The existing Rust engines remain responsible for actual media processing.

## Shared UI and startup state

`LanguageSelect.svelte` uses the shared language catalog and `SearchableSelect`;
language labels and Anki note names remain separate concerns. Generated language
note types preserve the `_Vesta` suffix. `EmptyStatusLabel.svelte` supplies the
same empty-media/subtitle indication across panels. Translation renders its
preview before a file is loaded; simple mode uses batch size 15 and resume overlap
2, preserving expert values for later use.

Transient notifications use `snackbarStore.svelte.ts` and the single `Snackbar`
in `App.svelte`: 1700 ms for info/success, 3500 ms for warning/error unless a caller
explicitly overrides the duration. Use panel state for errors that must remain
available after the notification disappears.

**Experimental → Try without saving** reuses the first-run UI without persistence
or downloads. **Restart setup** applies real preferences and downloads selected
resources. The app waits for durable settings before reloading. See the
[setup and installer guide](../../docs/reviews/provare-setup-e-installer-2026-10-07.md).

## Logs and updates

`supportLogStore.svelte.ts` buffers frontend diagnostics and forwards sanitized
entries to native support-log commands; `SupportLogsPanel` exposes saving and
copying the log path. Keep credentials out of messages at their source.

`updateCheckerStore.svelte.ts` checks the latest stable GitHub release. Native
`commands/updates.rs` independently resolves the official installer, checks its
SHA-256 digest and opens it through the OS. Windows, DEB and RPM use this path;
managed/portable installations expose their manager or release-page guidance.
Development builds and prereleases are not an installer-update smoke test.


## Export controls

The note-type picker in the footer is a compact searchable control; the closed
value shows the actual note name and the open menu distinguishes automatic
subtitle-language selection from manual types. It retains the shared selector
and accessible label. APKG merge controls appear only for multiple episodes with
an effective APKG export. TSV series always export separate episode files and
can include both snapshot and video fields. See the
[TSV import contract](../../docs/modules/srt-flashcards.md#tsv-media-and-note-types).
