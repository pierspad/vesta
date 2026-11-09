# srt-ankiconnect

Asynchronous HTTP client for a local [AnkiConnect](https://foosoft.net/projects/anki-connect/) instance (`DEFAULT_URL = "http://127.0.0.1:8765"`, `API_VERSION = 6`).

Exposes connection checks (`ping`), deck listing and creation (`deck_names`, `create_deck`), `.apkg` package import (`import_package`), base64 media storage (`store_media_file`), and batch note insertion (`add_notes`, `AnkiNote`).

```bash
cargo test -p srt-ankiconnect
```

See [`docs/modules/srt-ankiconnect.md`](../../docs/modules/srt-ankiconnect.md) for usage examples. License: GPL-3.0-only.
