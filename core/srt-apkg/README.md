# srt-apkg

Low-level helpers for reading and writing Anki `.apkg` ZIP archives.

This crate is used by Vesta's deck generation and refinement modules. It intentionally contains only archive-level operations; card generation belongs in `srt-flashcards`.

```bash
cargo test -p srt-apkg
```

License: GPL-3.0-only.
