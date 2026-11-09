# srt-parser — SRT parsing & writing

`core/srt-parser` is a foundational, dependency-light parser and writer for
SubRip (`.srt`) files.

**What you get**

- `SrtParser::parse_file(path)` / `parse_string(content)` / `parse_block(block)` → `HashMap<u32, Subtitle>` / `Subtitle`
- `SrtParser::normalize_subtitles(&mut subs)` — fills numbering gaps with placeholders when the estimated gap is at most 50,000; leaves zero-based input unchanged
- `SrtParser::save_file(...)` — writes a well-formed UTF-8 SRT back to disk atomically
- `Timestamp` (`from_milliseconds`, `total_milliseconds`, `from_srt_string`, `to_srt_string`, `write_srt_to`) and `Subtitle` (`from_srt_block`, `to_srt_string`, `write_srt_to`, `duration_ms`) types
- `encoding::read_text_auto(path)` / `decode_auto(bytes)` — encoding-tolerant
  reading: BOM sniffing, strict UTF-8 fast path, BOM-less UTF-16 heuristic,
  then statistical detection (`chardetng` + `encoding_rs`) for legacy code
  pages (GBK, Windows-1252, …). `parse_file` uses it to handle common saved
  encodings; statistical detection remains heuristic for ambiguous byte sequences.
- `persistence` module — `atomic_write`, `atomic_write_with`, and `ensure_distinct_files(input, output)` for safe temporary-sibling file publication and alias detection.

## Use as a Rust dependency

```toml
[dependencies]
srt-parser = { git = "https://github.com/pierspad/vesta" }
anyhow = "1"
```

```rust
use srt_parser::SrtParser;

fn main() -> anyhow::Result<()> {
    let mut subs = SrtParser::parse_file("movie.en.srt")?;
    SrtParser::normalize_subtitles(&mut subs);
    println!("{} subtitles", subs.len());
    Ok(())
}
```

## Vendoring & Standalone Integration

Prefer the Cargo Git dependency above. When vendoring `core/srt-parser/` into
another workspace, supply its inherited workspace metadata (`edition`,
`rust-version`) and dependencies (`anyhow`, `serde`, `encoding_rs`,
`chardetng`, `tempfile`, `same-file`); see
[`docs/library-integration.md`](../library-integration.md). It has no internal
Vesta crate dependencies.

## Who uses it

Direct consumers are `srt-extract`, `srt-translate`, `srt-sync`,
`srt-flashcards`, and the GUI adapter. Other feature crates may consume
subtitle data through those libraries without depending on `srt-parser`
directly.
