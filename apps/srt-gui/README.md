# Vesta desktop app

Tauri 2 desktop interface for the Vesta subtitle and flashcard toolchain. The frontend uses Svelte 5, TypeScript, Vite, and Tailwind CSS; native commands live in `src-tauri`.

```bash
pnpm install
pnpm check
pnpm test
pnpm tauri dev
```

Build the production frontend with `pnpm build`, or the complete desktop bundle with `pnpm tauri build`.

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
