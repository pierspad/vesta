## Release Notes v0.11.0

### New Features

- **Note Types as First-Class Objects**: The flashcards screen now uses a single **note type selector** instead of separate language and field controls. Note types define the export name, study language and active schema in one place.
- **Standalone CLI Tools**: Split the processing pipeline into independent command-line applications (`srt-extract`, `srt-translate`, `srt-flashcards`) so each stage can be automated and used without the GUI.
- **Benchmark Suite**: Added a reproducible benchmarking environment comparing Vesta against subs2srs in both single-core and multi-core execution modes, including automated reports and performance plots.
- **Deterministic Anki Export Identity**: APKG exports now generate stable model and deck identifiers, allowing Anki to correctly recognise repeated exports and merge them instead of creating duplicated note types or decks.
- **Custom Note Types**: Added support for user-defined note types with configurable fields while keeping predefined note types consistent and migration-safe.

### Fixes

- **Anki Note Type Duplication**: Fixed duplicated note models (`Name`, `Name+`, ...) when importing multiple APKG files generated from the same configuration. Model identity is now derived from the note type rather than the deck.
- **Canonical APKG Schema**: Predefined note types now always export the same nine-field schema in a fixed order:
  `Expression, Meaning, Audio, Snapshot, Video, Tags, SequenceMarker, Reading, Notes`.
- **APKG / TSV Compatibility**: Unified APKG and TSV exports so both formats use the same field ordering and inclusion rules.
- **Subtitle Matching Logic**: Improved and isolated subtitle/media matching logic, making suggestions more reliable and reusable outside the GUI.
- **Filters**: Restored and corrected subtitle filtering behaviour.
- **Template Validation**: Prevented invalid custom note types from being saved without required fields such as `Expression`.
- **Settings Navigation**: Fixed returning from settings always opening the wrong section by preserving the previous active tab.

### Improvements

- **Anki Configuration Reorganisation**: Moved export format, note type and field configuration into Settings for a cleaner workflow.
- **Simplified Note Type Architecture**: Removed language-specific note type coupling and moved to a generic note type system.
- **Locked Required Fields**: Predefined schemas now protect required fields such as `Expression` and `SequenceMarker` from accidental changes.
- **Improved Disabled Field Feedback**: Inactive fields are now kept visible and clearly marked instead of disappearing when unavailable.
- **Flashcard Layout Redesign**: Rebalanced the flashcard workspace into a cleaner three-column layout:
  - Files and naming
  - Audio/video configuration
  - Snapshots, filters and results
- **Shortcut Improvements**: Added better formatting and sorting for keyboard shortcuts and quick navigation actions.
- **Settings Shortcuts**: Added context actions to quickly open related settings from export format and note type controls.
- **UI Stability Improvements**: Removed dynamic layout shifting caused by hidden configuration panels; unavailable options remain visible but disabled.
- **Update Status Feedback**: Improved update checking UI with loading state, up-to-date confirmation and available update notifications.
- **Codebase Cleanup**: Reduced duplicated logic, separated responsibilities across crates, improved retry handling and simplified backend state management.
- **Localization Updates**: Completed missing translations and moved remaining hardcoded UI strings into the translation system.
- **Regression Tests**: Added coverage for note type schemas, stable identifiers and export consistency to prevent future regressions.

> **One-time migration note for existing collections**: if you already have a Vesta note type (for example `English_Vesta`) created by an older version, your existing schema may differ from the new canonical structure. To avoid Anki creating a duplicate note type (`Name+`), update your existing note type fields to match:
>
> `Expression, Meaning, Audio, Snapshot, Video, Tags, SequenceMarker, Reading, Notes`
>
> In particular, rename `Tag` → `Tags` and add the missing `Notes` field. Future exports using the same note type will then merge correctly.
