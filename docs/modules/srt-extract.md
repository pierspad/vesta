# srt-extract — subtitle data extraction

`lib/srt-extract` turns parsed subtitles into JSON, a human-readable summary,
debug dumps, and aggregate statistics (count, duration, and text lengths).

**Library API** (see `lib/srt-extract/src/lib.rs` and `src/embedded.rs`)

- `OutputFormat` (`Json` / `Summary` / `Debug`) + `OutputFormat::parse("json")`
- `extract(&subs, format)` → `Result<String>` (`extract_to_json`, `extract_to_summary`, `extract_to_debug`)
- `calculate_stats(&subs)` → `SubtitleStats`
- `embedded::list_embedded_subtitles(ffprobe, input)` → `Result<Vec<EmbeddedSubtitleTrack>>`
- `embedded::extract_embedded_subtitle(ffmpeg, ffprobe, input, index, output)` → `Result<()>`
- `embedded::preview_embedded_subtitle(ffmpeg, ffprobe, input, index)` → `Result<String>` (extracts up to 100 cues / 64 KiB of plain text for track previews)

## Use as a binary

```bash
cargo build --release -p srt-extract-cli
./target/release/srt-extract --input movie.srt --format json
```

## Use as a Rust dependency

```toml
[dependencies]
srt-parser  = { git = "https://github.com/pierspad/vesta" }
srt-extract = { git = "https://github.com/pierspad/vesta" }
anyhow = "1"
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

Prefer the Git dependency above: Cargo resolves the workspace dependencies.
To copy the sources into another workspace, include `lib/srt-extract/` and
`core/srt-parser/` and replace inherited workspace manifest fields/dependencies.
The extraction library also uses `tokio` and `tempfile` for embedded media.

## Embedded media subtitles

The `embedded` module adds asynchronous FFprobe discovery and FFmpeg extraction:
`list_embedded_subtitles(ffprobe, input)` returns global stream indices, codec,
language, title, and text-conversion support. `extract_embedded_subtitle(ffmpeg,
ffprobe, input, index, output)` writes an SRT atomically. ASS/SSA styling is lost;
PGS/VobSub are explicitly unsupported without OCR. This API is used by the desktop
Extract tab and the headless CLI. The CLI also formats external SRT files.

```bash
srt-extract --input film.mkv --list-tracks
srt-extract --input film.mkv --track 8 --output film_en_8.srt
```

`--track` uses the absolute stream index returned by `--list-tracks`, requires
`--output`, and cannot be combined with `--list-tracks`. Override executable
locations with `--ffmpeg` and `--ffprobe`.
`preview_embedded_subtitle` extracts up to 100 cues (capped at 64 KiB of plain
text) into a temporary directory and cleans up automatically, powering track
previews without writing permanent output files.
