# srt-refine — LLM enrichment of Anki decks

`lib/srt-refine` is the engine behind Vesta's Revision/Refine tab: load an
Anki deck (TSV or `.apkg`), send each card through an LLM prompt to enrich its
*Notes* field (word explanations, etymology, grammar hints…), and save the
deck back without losing media or scheduling data.

**What's inside**

- TSV column classification heuristics (skips `[sound:…]`/`<img …>` media
  columns and sequence markers to find expression/meaning/notes);
- APKG round-trip: unzip → read `collection.anki2` (SQLite) → map fields by
  Anki model names (Expression/Front/Target…, Meaning/Back/Native…,
  Notes/Note/Comment…) → update notes + `csum` → re-zip;
- prompt interpolation (`{{expression}}`, `{{meaning}}`, `{{notes}}`) and the
  LLM call itself, through [`srt-translate`](srt-translate.md)'s `Translator`
  (Gemini, Groq, or any OpenAI-compatible endpoint incl. Ollama).

**Library API**

- `load_cards(path)` → `Vec<RefineCard { id, expression, meaning, notes }>`
- `save_cards(input_path, output_path, updates)` — TSV→TSV, APKG→TSV,
  APKG→APKG (in-place note update inside the archive)
- `refine_card_llm(&card, prompt, RefineLlmConfig)` → new notes text

Errors are user-presentable `String`s, mirroring `srt-flashcards`.

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

## Extract it standalone

Copy `lib/srt-refine/` + `lib/srt-translate/` + `core/srt-parser/`
(srt-translate depends on it). External deps: `rusqlite` (bundled SQLite),
`zip`, `sha1_smol`, `tempfile`, `serde`, `serde_json`.
