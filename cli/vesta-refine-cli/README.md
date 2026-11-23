# vesta-refine (`vesta-refine-cli`)

Inspect, edit, and annotate TSV/APKG decks using `vesta-refine`'s shared card I/O
and tiered refinement engine, or interact with Anki directly via `vesta-ankiconnect`.
No desktop or Whisper installation is required (`ankiconnect` requires Anki running
with the AnkiConnect add-on).

```sh
cargo run -p vesta-refine-cli -- inspect --input deck.tsv
cargo run -p vesta-refine-cli -- inspect --input deck.apkg --query verb --offset 0 --limit 25
cargo run -p vesta-refine-cli -- edit --input deck.apkg --output deck.edited.apkg --id 42 --notes "Manual note"
cargo run -p vesta-refine-cli -- enrich --input deck.tsv --output deck.annotated.tsv --config refine.json
cargo run -p vesta-refine-cli -- ankiconnect ping
cargo run -p vesta-refine-cli -- ankiconnect decks
cargo run -p vesta-refine-cli -- ankiconnect import --apkg deck.apkg
```

`inspect` writes a JSON array of card fields to stdout and never contacts a
provider (`--query` filters case-insensitively across `id`, `expression`, `meaning`,
and `notes`, while `--offset` and `--limit` paginate results). `edit` updates a
single card's `notes` field by `--id` and writes a JSON confirmation (`{"ok":true,...}`);
`edit` rejects choosing `--input` as `--output` (use a distinct output path).
`enrich` writes a JSON object with `summary` and `output`; progress and safe errors
go to stderr. `ankiconnect` delegates `ping`, `decks`, and `import --apkg <PATH>`
(canonicalized after checking existence) to `vesta-ankiconnect` using `--url`
(defaults to `http://127.0.0.1:8765`). Configuration for `enrich` is JSON:

```json
{
  "tiers": [[{
    "provider": "local",
    "model": "your-installed-model",
    "api_url": "http://localhost:11434/v1",
    "rpm": 0
  }]],
  "prompt": "Explain {{expression}} using {{meaning}}. Current notes: {{notes}}",
  "batch_mode": false,
  "batch_size": 8
}
```

Hosted endpoints use the same `TierEntry` schema as the translation library and
require `api_key`. Keep credential-bearing config files out of source control.
All endpoints require an explicit model; unknown providers and invalid config
are rejected before HTTP requests. No environment-variable expansion is applied.

Supported conversions are TSV→TSV, APKG→TSV and APKG→APKG. The output directory
must already exist. Card-loading, field mapping and atomic publication belong to
the library, including preservation of unmodified notes and APKG media.

By default, any failed/remaining card or exhausted pool prevents publication.
`--allow-partial` permits successful annotations to be saved while keeping the
other cards unchanged; the exit code is still 1. A run with no successful
annotations never replaces the destination. Ctrl+C cancels active engine work,
exits 130 and does not save partial output, even with `--allow-partial`. A complete
run exits 0; validation/processing errors exit 1 and clap argument errors exit 2.
Explicitly choosing the input as output during `enrich` performs an atomic in-place update (`edit` requires a distinct output file).

Tests use loopback HTTP fixtures, including real SIGINT cancellation on Unix:

```sh
cargo test -p vesta-refine-cli
```

GPL-3.0-only. Embedding applications should call `vesta_refine::{load_cards,
refine_cards_tiered, save_cards}` directly.

See the [command-by-command CLI guide](../../docs/CLI.md) for examples, misuse
corrections and the current GUI/library coverage boundaries.
