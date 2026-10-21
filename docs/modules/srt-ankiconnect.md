# srt-ankiconnect

Asynchronous HTTP client for direct integration with local Anki instances via the AnkiConnect add-on.

## What it does

`srt-ankiconnect` communicates with Anki's local JSON-RPC API (`http://127.0.0.1:8765`, API version 6) to perform direct deck synchronization:
- **Ping & Version Check**: Confirms Anki is running and reachable.
- **Deck Inspection**: Fetches existing deck names.
- **Deck Creation**: Creates a deck when the caller explicitly invokes `create_deck`.
- **Note Injection**: Pushes generated flashcards directly into the user's active Anki database.
- **Package and media helpers**: Imports an `.apkg` with `import_package` and stores base64 media with `store_media_file`.
- **Shared Connection Pool**: Uses a persistent `reqwest::Client` with a shared HTTP connection pool.

## Crate Info

- **Path**: `lib/srt-ankiconnect`
- **Dependencies**: `reqwest`, `serde`, `serde_json`

## Rust API Example

```toml
[dependencies]
srt-ankiconnect = { git = "https://github.com/pierspad/vesta" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
use std::collections::HashMap;
use srt_ankiconnect::{DEFAULT_URL, add_notes, create_deck, deck_names, ping, AnkiNote};

#[tokio::main]
async fn main() -> Result<(), String> {
    // Verify AnkiConnect is active
    let version = ping(DEFAULT_URL).await?;
    println!("AnkiConnect version: {version}");
    let decks = deck_names(DEFAULT_URL).await?;
    println!("Available decks: {decks:?}");

    // Create the target deck explicitly; the Basic model must also exist.
    create_deck(DEFAULT_URL, "Japanese Immersion").await?;
    let mut fields = HashMap::new();
    fields.insert("Front".to_string(), "こんにちは".to_string());
    fields.insert("Back".to_string(), "Hello".to_string());

    let note = AnkiNote {
        deck_name: "Japanese Immersion".to_string(),
        model_name: "Basic".to_string(),
        fields,
        tags: vec!["JLPT::N5".to_string(), "Vesta".to_string()],
    };

    add_notes(DEFAULT_URL, &[note]).await?;
    Ok(())
}
```

`add_notes` does not create the deck or model. Ensure the note model exists in
Anki before adding notes.
