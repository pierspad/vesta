# srt-extract-cli

Command-line adapter for `srt-extract`. It formats external SRT files and lists or extracts embedded text subtitles without loading the desktop application.

```bash
cargo build --release -p srt-extract-cli

./target/release/srt-extract --input movie.srt --format json
./target/release/srt-extract --input movie.srt --format stats
./target/release/srt-extract --input movie.srt --format summary
./target/release/srt-extract --input movie.srt --format debug --output report.txt
```

| Format | Output |
|---|---|
| `json` | Structured subtitle records |
| `stats` | Counts, timing, duration, and word statistics |
| `summary` | Short file overview |
| `debug` | Detailed diagnostic representation |

Run `srt-extract --help` for the authoritative option list. Licensed GPL-3.0-only.

## Embedded tracks

```bash
srt-extract --input movie.mkv --list-tracks
srt-extract --input movie.mkv --track 8 --output movie_en_8.srt
```

Discovery emits JSON with absolute stream indices. Extraction requires `--output`;
`--list-tracks` and `--track` are mutually exclusive. FFmpeg/ffprobe must be on
PATH, or supplied with `--ffmpeg /path/to/ffmpeg --ffprobe /path/to/ffprobe`.
Bitmap tracks require OCR and cannot be exported as text by this tool. Failed
extraction preserves an existing output; successful extraction replaces it atomically.
