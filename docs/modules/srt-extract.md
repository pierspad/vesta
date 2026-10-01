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

## Embedded media subtitles

The `embedded` module adds asynchronous FFprobe discovery and FFmpeg extraction:
`list_embedded_subtitles(ffprobe, input)` returns global stream indices, codec,
language, title, and text-conversion support. `extract_embedded_subtitle(ffmpeg,
ffprobe, input, index, output)` writes an SRT atomically. ASS/SSA styling is lost;
PGS/VobSub are explicitly unsupported without OCR. This API is used by the desktop
Extract tab; the existing CLI continues to format external SRT files.

### Embedded subtitle browser

The desktop browser groups tracks by inferred language and paginates ten languages
per page. Groups with a text track come first; variants prefer ordinary text over
SDH/forced/commentary and bitmap tracks. Explicit language names in titles override
conflicting container tags for grouping and filename suggestions, while the UI
retains a metadata-conflict warning. This is label inference, not content detection.

Text variants offer an eye button, double click and a context menu for preview.
Preview extraction stops at 100 cues, returns at most 64 KiB of plain text and
cleans its temporary directory. The UI caches up to 16 previews for the selected
media; downloads still use a separate save dialog and atomic extraction.
