# srt-apkg

Low-level ZIP utilities used while reading and modifying Anki `.apkg` archives.

## What it does

Anki `.apkg` packages are standard ZIP archives containing SQLite databases (`collection.anki2` / `collection.anki21`) alongside numbered media files and a `media` JSON manifest map.

`srt-apkg` provides generic archive handling used by `srt-refine` (deck
inspection and modification). It does not create Anki collection databases or
media manifests; `srt-flashcards` owns APKG deck creation:
- **`unzip_to(zip_path, dest_dir)`**: Extracts a ZIP archive into a directory with buffered streaming and path-traversal validation.
- **`extract_file_from_zip(zip_path, entry_name, dest_path)`**: Extracts a single named entry (such as `collection.anki2` or `collection.anki21`) from an `.apkg` archive.
- **`zip_from_dir(src_dir, zip_path)`**: Packages the flat files in a directory into a ZIP, storing recognized media extensions (and numbered media entries referenced by `media`) without compression (`Stored`) and deflating other files.

## Crate Info

- **Path**: `core/srt-apkg`
- **Dependencies**: `zip` (re-exported as `srt_apkg::zip`), `tempfile`, `serde_json`

## Rust API Example

```toml
[dependencies]
srt-apkg = { git = "https://github.com/pierspad/vesta" }
tempfile = "3"
anyhow = "1"
```

```rust
use std::path::Path;
use srt_apkg::{unzip_to, zip_from_dir};

fn main() -> anyhow::Result<()> {
    // Extract an existing APKG
    let temp_dir = tempfile::tempdir()?;
    unzip_to(Path::new("deck.apkg"), temp_dir.path()).map_err(anyhow::Error::msg)?;

    // Re-package its flat files into a ZIP archive
    zip_from_dir(temp_dir.path(), Path::new("output_deck.apkg"))
        .map_err(anyhow::Error::msg)?;
    Ok(())
}
```
