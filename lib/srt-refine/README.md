# srt-refine

For standalone inspection and tiered annotation, use the
[srt-refine CLI](../../cli/srt-refine-cli/README.md). It delegates card I/O,
processing and atomic publication to this library.

Headless flashcard refinement engine for TSV and APKG decks.

It loads existing cards, generates or updates notes through Vesta's LLM provider pool, and writes the result back while preserving the deck package.

```bash
cargo test -p srt-refine
```

License: GPL-3.0-only.
