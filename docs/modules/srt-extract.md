# srt-extract — subtitle data extraction

`lib/srt-extract` turns parsed subtitles into other representations: plain
text, JSON, a human-readable summary, debug dumps, and aggregate statistics
(count, duration, characters-per-second…).

**Library API** (see `lib/srt-extract/src/lib.rs`)

- `OutputFormat` (`Text` / `Json` / `Summary` / `Debug`) + `OutputFormat::parse("json")`
- `extract(&subs, format)` → `String`
- `calculate_stats(&subs)` → `SubtitleStats`

## Use as a binary

```bash
cargo build --release -p srt-extract-cli
./target/release/srt-extract movie.srt --format json
```

## Use as a Rust dependency

```toml
[dependencies]
srt-parser  = { git = "https://github.com/pierspad/vesta" }
srt-extract = { git = "https://github.com/pierspad/vesta" }
```

```rust
use srt_extract::{extract, OutputFormat};
use srt_parser::SrtParser;

fn main() -> anyhow::Result<()> {
    let subs = SrtParser::parse_file("movie.srt")?;
    println!("{}", extract(&subs, OutputFormat::Json)?);
    Ok(())
}
```

## Extract it standalone

Copy `lib/srt-extract/` + `core/srt-parser/`. External deps: `anyhow`,
`serde`, `serde_json` only.
