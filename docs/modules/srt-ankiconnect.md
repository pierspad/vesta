# srt-ankiconnect

Asynchronous HTTP client for direct integration with local Anki instances via the AnkiConnect add-on.

## What it does

`srt-ankiconnect` communicates with Anki's local JSON-RPC API (`http://127.0.0.1:8765`, API version 6) to perform direct deck synchronization:
- **Ping & Version Check**: Confirms Anki is running and reachable.
- **Deck & Model Inspection**: Fetches existing deck names, model names, and model field lists.
- **Deck Creation**: Creates missing target decks automatically.
- **Note Injection**: Pushes generated flashcards directly into the user's active Anki database.
- **Shared Connection Pool**: Uses a persistent `reqwest::Client` with a shared HTTP connection pool.

## Crate Info

- **Path**: `lib/srt-ankiconnect`
- **Dependencies**: `reqwest`, `serde`, `serde_json`

## Rust API Example

```rust
use std::collections::HashMap;
use srt_ankiconnect::{DEFAULT_URL, ping, deck_names, add_notes, AnkiNote};

// Verify AnkiConnect is active
let version = ping(DEFAULT_URL).await?;
println!("AnkiConnect version: {version}");

// Create note
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
```
