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
./target/release/srt-extract --input movie.srt --format json
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

### Embedded subtitle browser

The desktop browser groups tracks by inferred language and paginates ten languages
per page. Groups with a text track come first; variants prefer ordinary text over
SDH/forced/commentary and bitmap tracks. Explicit language names in titles override
conflicting container tags for grouping and filename suggestions, while the UI
retains a metadata-conflict warning. This is label inference, not content detection.

Text variants offer an eye button, double click and a context menu for preview.
Preview extraction stops at 100 cues, returns at most 64 KiB of plain text and
cleans its temporary directory. The UI uses a dialog-local sliding cache of the current text variant and its
previous/next text variants (bitmap tracks are skipped). Opening or navigating
starts adjacent prefetches, deduplicates in-flight requests, and permits at most
two simultaneous extractions. Queued foreground requests take priority;
obsolete queued work and cache entries outside the window are discarded.
Prefetch errors stay silent and are retried when the user selects that track.
The preview dialog navigates text variants of the same language using
Previous/Next or left/right arrows. Choosing a preview updates the card selection;
download writes the previewed track without closing the dialog or hiding its text.
Navigation remains available while a preview loads, and request tokens prevent
older results from replacing the current preview. Closing, choosing a track,
changing media, or unmounting clears the preview text/cache and rejects pending
frontend requests. Already dispatched Tauri commands cannot be cancelled by this
cache: they finish under the backend timeout and clean their temporary files,
but their late results cannot restore a closed session. Reopening uses a new cache.

The dialog uses neutral compact buttons (36 px high) with SVG check/download icons
and the application's chevron paths at 20 px. Scoped padding overrides global
button rules so Tailwind utility precedence cannot enlarge the footer buttons.

Both Extract and Flashcards default to the OS Downloads directory if it exists,
otherwise the home directory. Flashcards preserves an existing saved output folder.
The Extract folder picker applies to all downloads in that tab session. Output names
include a sanitized media stem, inferred language, and absolute stream index
(`film_fr_17.srt`), so language variants do not overwrite each other.

The desktop adapter checks those exact filenames on media/folder changes, after
a download, and when the app window regains focus. Only regular files count;
the checkmark follows the card's selected variant. This indicates file presence,
not a content checksum: renamed files and older names without a stream index
cannot be identified reliably. Replacing the source media under the same name
can also invalidate that correspondence. Downloads use atomic extraction and the
shared snackbar's standard timeout (2800 ms).
