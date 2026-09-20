# srt-apkg

Low-level utilities for creating and extracting Anki `.apkg` ZIP archives.

## What it does

Anki `.apkg` packages are standard ZIP archives containing SQLite databases (`collection.anki2` / `collection.anki21`) alongside numbered media files and a `media` JSON manifest map.

`srt-apkg` provides minimal, high-throughput archive handling used by `srt-flashcards` (deck creation) and `srt-refine` (deck inspection and modification):
- **`unzip_to`**: Extracts an `.apkg` archive into a directory with buffered streaming.
- **`zip_from_dir`**: Packages files from a directory into a standard `.apkg` archive with media extension recognition and compression.

## Crate Info

- **Path**: `core/srt-apkg`
- **Dependencies**: `zip`

## Rust API Example

```rust
use std::path::Path;
use srt_apkg::{unzip_to, zip_from_dir};

// Extract an existing APKG
let temp_dir = tempfile::tempdir()?;
unzip_to(Path::new("deck.apkg"), temp_dir.path())?;

// Re-package into a new APKG
zip_from_dir(temp_dir.path(), Path::new("output_deck.apkg"))?;
```
