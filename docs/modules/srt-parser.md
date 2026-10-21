# srt-parser — SRT parsing & writing

`core/srt-parser` is a foundational, dependency-light parser and writer for
SubRip (`.srt`) files.

**What you get**

- `SrtParser::parse_file(path)` / `parse_string(content)` → `HashMap<u32, Subtitle>`
- `SrtParser::normalize_subtitles(&mut subs)` — fills numbering gaps with placeholders when the estimated gap is at most 50,000; leaves zero-based input unchanged
- `SrtParser::save_file(...)` — writes a well-formed SRT back to disk
- `Timestamp` (ms-precision, `from_srt_string` / `to_srt_string`) and `Subtitle` types
- `encoding::read_text_auto(path)` / `decode_auto(bytes)` — encoding-tolerant
  reading: BOM sniffing, strict UTF-8 fast path, BOM-less UTF-16 heuristic,
  then statistical detection (`chardetng` + `encoding_rs`) for legacy code
  pages (GBK, Windows-1252, …). `parse_file` uses it to handle common saved
  encodings; statistical detection remains heuristic for ambiguous byte sequences.

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

## Extract it standalone

Copy `core/srt-parser/` into your own workspace — its external dependencies are
`anyhow`, `serde`, `encoding_rs`, `chardetng`, `tempfile` and `same-file`; it has
no internal Vesta dependency. Bring it along when vendoring a crate that lists
`srt-parser` as a dependency; support crates such as `srt-download` and
`srt-apkg` do not require it.

## Who uses it

Direct consumers are `srt-extract`, `srt-translate`, `srt-sync`,
`srt-flashcards`, and the GUI adapter. Other feature crates may consume
subtitle data through those libraries without depending on `srt-parser`
directly.
