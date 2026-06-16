
## Release Notes v0.11.0

### New Features

* **Note Types as First-Class Objects**: The flashcards screen now has a single **note type selector** (replacing the old "language + field pills" duplication). It lists the locked, per-language predefined note types (e.g. `English_Vesta`) plus any custom note types you create. Picking one sets the export name, the study language and the active fields in one place.
* **Standalone CLIs**: Split the engine into independent command-line tools (`srt-extract`, `srt-translate`, `srt-flashcards`) so each stage can be scripted and run on its own, separately from the GUI.
* **Benchmark Suite**: Added a reproducible methodology for benchmarking Vesta against subs2srs in both single-core and multi-core modes, with build helpers and report generation.

### Fixes

* **Anki Note Type Duplication (APKG)**: Fixed the long-standing issue where importing several `.apkg` files built on the same note type produced duplicate note types (`Name`, `Name+`, ...) instead of merging into one. The Anki **model id is now derived from the note type name** (not the deck), and the exported **schema is a fixed property of the note type** — predefined note types are locked to all nine fields in a canonical order, so every export of the same note type is structurally identical. That is exactly what Anki needs to recognise repeated imports as the same note type and merge them.
* **APKG / TSV Field Consistency**: Both export paths emit the same canonical field order (Expression, Meaning, Audio, Snapshot, Video, Tags, SequenceMarker, Reading, Notes) and the same inclusion rules, so a TSV import and an APKG import of the same configuration map onto an identical note type.
* **Filters**: Restored and corrected subtitle filtering behaviour.

### Improvements

* **Custom Note Types Can Trim Fields**: Predefined note types stay locked to the full nine-field schema; custom note types may switch individual fields off to get a smaller, cleaner schema, while still merging cleanly on re-import.
* **Settings Reorganisation**: Moved note type, field name and card template configuration into the Settings section for a clearer, more centralised setup.
* **Code Deduplication & Efficiency**: Separated responsibilities across the CLI crates, removed duplicated logic and streamlined hot paths.
* **Build Scripts**: Improved the build and release scripts.
* **Regression Coverage**: Added automated tests that lock in the per-note-type schema and the stable model id, guarding the merge behaviour against future regressions.

> **One-time migration note for existing collections**: if you already have a Vesta note type (e.g. `English_Vesta`) created by an older version, your current note type may have a slightly different schema (for example a `Tag` field instead of `Tags`, and no `Notes` field). For Anki to merge new imports into it rather than creating `English_Vesta+`, align your existing note type's fields with the canonical schema above (rename `Tag` → `Tags`, add a `Notes` field). From then on every Vesta export of that note type will merge cleanly.