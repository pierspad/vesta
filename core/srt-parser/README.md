# srt-parser

Foundational subtitle parser used throughout Vesta. It reads SRT files with automatic character-set detection, normalizes subtitle timing and identifiers, and writes valid UTF-8 SRT output.

The crate is GUI-independent and depends only on `anyhow`, `serde`, `encoding_rs`, and `chardetng`. Workspace crates should use the root `workspace.dependencies` entry; external users can depend on the Git repository:

```toml
[dependencies]
srt-parser = { git = "https://github.com/pierspad/vesta" }
```

```rust
use srt_parser::SrtParser;

fn main() -> anyhow::Result<()> {
    let mut subtitles = SrtParser::parse_file("movie.srt")?;
    SrtParser::normalize_subtitles(&mut subtitles);
    SrtParser::save_file("normalized.srt", &subtitles)?;
    Ok(())
}
```

See [`docs/modules/srt-parser.md`](../../docs/modules/srt-parser.md) for the public data model and extraction notes. Licensed GPL-3.0-only.
