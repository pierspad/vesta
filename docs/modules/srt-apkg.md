# srt-apkg

Low-level ZIP utilities used while reading and modifying Anki `.apkg` archives.

## What it does

Anki `.apkg` packages are standard ZIP archives containing SQLite databases (`collection.anki2` / `collection.anki21`) alongside numbered media files and a `media` JSON manifest map.

`srt-apkg` provides generic archive handling used by `srt-refine` (deck
inspection and modification). It does not create Anki collection databases or
media manifests; `srt-flashcards` owns APKG deck creation:
- **`unzip_to`**: Extracts a ZIP archive into a directory with buffered streaming.
- **`zip_from_dir`**: Packages the flat files in a directory into a ZIP, storing recognized media extensions without compression and deflating other files.

## Crate Info

- **Path**: `core/srt-apkg`
- **Dependencies**: `zip`, `tempfile`

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
