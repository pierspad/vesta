# srt-refine CLI

Inspect and annotate TSV/APKG decks using `srt-refine`'s shared card I/O and
tiered refinement engine. No desktop, Whisper, or Anki installation is required.

```sh
cargo run -p srt-refine-cli -- inspect --input deck.tsv
cargo run -p srt-refine-cli -- enrich --input deck.tsv --output deck.annotated.tsv --config refine.json
```

`inspect` writes a JSON array of card fields to stdout and never contacts a
provider. `enrich` writes a JSON object with `summary` and `output`; progress and
safe errors go to stderr. Configuration is JSON:

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
Explicitly choosing the input as output performs an atomic in-place update.

Tests use loopback HTTP fixtures, including real SIGINT cancellation on Unix:

```sh
cargo test -p srt-refine-cli
```

GPL-3.0-only. Embedding applications should call `srt_refine::{load_cards,
refine_cards_tiered, save_cards}` directly.

See the [command-by-command CLI guide](../../docs/CLI.md) for examples, misuse
corrections and the current GUI/library coverage boundaries.
