# srt-parser — SRT parsing & writing

`core/srt-parser` is the foundational crate every other Vesta module builds
on: a fast, dependency-light parser and writer for SubRip (`.srt`) files.

**What you get**

- `SrtParser::parse_file(path)` / `parse_string(content)` → `HashMap<u32, Subtitle>`
- `SrtParser::normalize_subtitles(&mut subs)` — fills numbering gaps with placeholders
- `SrtParser::save_file(...)` — writes a well-formed SRT back to disk
- `Timestamp` (ms-precision, `from_srt_string` / `to_srt_string`) and `Subtitle` types
- `encoding::read_text_auto(path)` / `decode_auto(bytes)` — encoding-tolerant
  reading: BOM sniffing, strict UTF-8 fast path, BOM-less UTF-16 heuristic,
  then statistical detection (`chardetng` + `encoding_rs`) for legacy code
  pages (GBK, Windows-1252, …). `parse_file` uses it, so any SRT loads
  correctly regardless of how it was saved

## Use as a Rust dependency

```toml
[dependencies]
srt-parser = { git = "https://github.com/pierspad/vesta" }
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

Copy `core/srt-parser/` into your own workspace — it only depends on `anyhow`,
`serde`, `encoding_rs` and `chardetng` from crates.io, nothing internal. It is the single crate you must
bring along when vendoring any other `srt-*` module.

## Who uses it

Every other module: `srt-extract`, `srt-translate`, `srt-sync`,
`srt-flashcards` (via its own multi-format parser for ASS/VTT), and the GUI.
