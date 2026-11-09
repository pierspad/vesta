# srt-refine

Headless flashcard refinement engine for TSV and `.apkg` decks (`load_cards`, `save_cards`, `refine_card_llm`, `refine_cards_tiered`, `analyze_tsv_columns`).

Loads existing cards, interpolates `{{expression}}`, `{{meaning}}`, and `{{notes}}` templates, generates or updates notes through Vesta's tiered LLM provider pool (`srt-translate`), and writes the updated deck atomically while preserving `.apkg` media and Anki scheduling fields.

For standalone inspection and tiered annotation from the terminal, use [`srt-refine-cli`](../../cli/srt-refine-cli/README.md).

```bash
cargo test -p srt-refine
```

See [`docs/modules/srt-refine.md`](../../docs/modules/srt-refine.md) for API details and usage examples. License: GPL-3.0-only.
