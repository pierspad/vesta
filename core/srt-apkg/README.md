# srt-apkg

Low-level helpers for reading and writing Anki `.apkg` ZIP archives (`unzip_to`, `extract_file_from_zip`, `zip_from_dir`).

This crate is used by Vesta's deck generation and refinement modules. It contains archive-level operations (storing recognized media files uncompressed and deflating database/manifest entries); card and SQLite deck generation belong in `srt-flashcards`.

```bash
cargo test -p srt-apkg
```

See [`docs/modules/srt-apkg.md`](../../docs/modules/srt-apkg.md) for API signatures and examples. License: GPL-3.0-only.
