# srt-ocr-cli

Headless command-line interface for extracting bitmap subtitle tracks (PGS,
VobSub, DVB, XSUB) from media containers and converting them into SRT subtitles
via Vision LLMs.

```bash
cargo build --release -p srt-ocr-cli
./target/release/srt-ocr --help
```

## Subcommands

- `run`: Extract a bitmap subtitle track by FFprobe stream index (`--track`) and
  recognize cues to an `.srt` output file (`--output`).
- `models`: List cataloged local OCR models and their cache status.
- `download <model_id>`: Download a cataloged local OCR model.
- `remove <model_id>`: Remove a cached local OCR model.

## Providers and JSON Mode

- `--provider local` (default) targets an OpenAI-compatible Vision endpoint
  (such as Ollama at `http://localhost:11434/v1`) and does not require
  `--api-key`.
- `--provider custom` requires `--api-url` (`--api-key` is optional).
- Cloud providers (`google`, `gemini`, `openai`, `anthropic`, `groq`,
  `openrouter`, `mistral`, `nvidia`) require `--api-key`.

With `run --json`, stdout contains one `OcrProgress` JSON object per progress
update followed by a single terminal object: `{"ok":true,"outcome":...}` on
success, or `{"ok":false,"cancelled":false,"error":"..."}` on failure (exit 1).
Ctrl+C emits `cancelled:true` and exits 130 while keeping stderr quiet so
subprocess callers can parse stdout cleanly. Argument parsing errors still use
clap's standard stderr output and exit 2.

See the [command-by-command CLI guide](../../docs/CLI.md) for usage examples.
