# vesta-autosync (`vesta-autosync-cli`)

Headless command-line interface for Whisper-assisted subtitle synchronization (local whisper.cpp or cloud STT providers).

```bash
cargo build --release -p vesta-autosync-cli
./target/release/vesta-autosync --help
./target/release/vesta-autosync input.srt movie.mkv -o synced.srt --model base
./target/release/vesta-autosync input.srt movie.mkv -o synced.srt \
  --provider groq --model whisper-large-v3-turbo --api-key "$GROQ_API_KEY" --max-offset-secs 180
./target/release/vesta-autosync input.srt movie.mkv -o synced.srt \
  --provider custom --model whisper-1 --api-url http://localhost:8000/v1
```

The alignment engine is provided by `vesta-autosync`; this package handles CLI arguments, progress, cancellation, and output. With local providers (`local`, `whisper`, `local_whisper`), the Whisper model is downloaded on demand. With cloud providers (`openai`, `groq`, `deepgram`, `assemblyai`, `custom`), local model download is skipped; hosted providers require `--api-key`, while `--provider custom` requires `--api-url` (and allows anonymous endpoints without `--api-key`). `--max-offset-secs` overrides the default 120-second drift search radius. `--quiet` suppresses progress on stderr.

Successful runs write the synced SRT to `--output` and print a summary to stdout. Errors go to stderr and exit 1; invalid arguments exit 2. Ctrl+C cancels model download or analysis, does not write an output file, and exits 130. Existing output files are replaced only after a successful analysis. Suggested anchors are all applied before the output is written. Cancellation is checked immediately before the synchronous file save; once that save starts, it runs to completion.

```sh
cargo test -p vesta-autosync-cli
```

See the [command-by-command CLI guide](../../docs/CLI.md) for examples, misuse
corrections and the current GUI/library coverage boundaries.
