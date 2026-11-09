# srt-refine — LLM enrichment of Anki decks

`lib/srt-refine` is the engine behind Vesta's Revision/Refine tab: load an
Anki deck (TSV or `.apkg`), send each card through an LLM prompt to enrich its
*Notes* field (word explanations, etymology, grammar hints…), and save the
deck back without losing media or scheduling data.

**What's inside**

- TSV column classification heuristics (`analyze_tsv_columns`: skips `[sound:…]`/`<img …>` media
  columns and sequence markers to identify expression/meaning/notes columns);
- APKG round-trip: unzip → read `collection.anki2` / `collection.anki21` (SQLite) → map fields by
  Anki model names (Expression/Front/Target…, Meaning/Back/Native…,
  Notes/Note/Comment…) → update `notes.flds`, `mod`, and `usn` → re-zip while preserving media and scheduling;
- prompt interpolation (`interpolate_prompt`: `{{expression}}`, `{{meaning}}`, `{{notes}}`) and
  single-card or multi-worker tiered LLM enrichment (`refine_card_llm`, `refine_cards_tiered`) through
  [`srt-translate`](srt-translate.md)'s `Translator` and `TranslatorPool`.

**Library API**

- `load_cards(path)` → `Vec<RefineCard { id, expression, meaning, notes }>`
- `save_cards(input_path, output_path, updates)` — TSV→TSV, APKG→TSV,
  APKG→APKG (atomic note update inside the archive)
- `refine_card_llm(&card, prompt, RefineLlmConfig)` → single-card notes text
- `refine_cards_tiered(cards, &tiers, &config, cancel_token, on_event)` → `Result<(Vec<RefineUpdate>, RefineRunSummary), String>`
- `analyze_tsv_columns(path)`, `interpolate_prompt(template, &card)`, `strip_html(input)`

Errors are user-presentable `String`s, mirroring `srt-flashcards`.

## Use as a CLI

```sh
cargo run -p srt-refine-cli -- inspect --input deck.tsv
cargo run -p srt-refine-cli -- enrich --input deck.tsv --output deck.refined.tsv --config refine.json
```

Inspection prints JSON without contacting a provider. Enrichment uses
`refine_cards_tiered` with the shared provider pool, cancellation and card
serialization. It saves only complete success by default; `--allow-partial`
explicitly permits successful annotations alongside unchanged failed cards and
still exits nonzero. Ctrl+C cancels active work and prevents publication.
See the [CLI guide](../../cli/srt-refine-cli/README.md) for the configuration
schema, JSON summary, supported output conversions and exit codes.

## Use as a Rust dependency

```toml
[dependencies]
srt-refine = { git = "https://github.com/pierspad/vesta" }
tokio      = { version = "1", features = ["full"] }
```

```rust
use srt_refine::{load_cards, refine_card_llm, save_cards, RefineLlmConfig, RefineUpdate};

#[tokio::main]
async fn main() -> Result<(), String> {
    let cards = load_cards("deck.apkg")?;

    let mut updates = Vec::new();
    for card in cards.iter().take(3) {
        let notes = refine_card_llm(
            card,
            "Explain the hardest words in {{expression}} (meaning: {{meaning}}). \
             Current notes: {{notes}}",
            RefineLlmConfig {
                api_type: "local".into(),          // Ollama
                api_key: None,
                api_url: None,                      // http://localhost:11434/v1
                model: Some("llama3.2".into()),
            },
        ).await?;
        updates.push(RefineUpdate { id: card.id.clone(), notes });
    }

    save_cards("deck.apkg", "deck.refined.apkg", updates)?;
    Ok(())
}
```

## Vendoring & Standalone Integration

Prefer the Cargo Git dependency above. When vendoring into another workspace,
include `lib/srt-refine/`, `lib/srt-translate/`, `core/srt-parser/`, and
`core/srt-apkg/`, and preserve workspace metadata and dependencies (`rusqlite`
with bundled SQLite, `tempfile`, `serde`, `serde_json`, `tokio`, `tokio-util`);
see [`docs/library-integration.md`](../library-integration.md).
