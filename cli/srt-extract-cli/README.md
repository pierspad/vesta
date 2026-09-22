# srt-extract-cli

Command-line adapter for `srt-extract`. It parses an SRT file and emits one of four representations without loading the desktop application.

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
