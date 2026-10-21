# srt-autosync-cli

Headless command-line interface for Whisper-assisted subtitle synchronization.

```bash
cargo build --release -p srt-autosync-cli
./target/release/srt-autosync --help
```

The alignment engine is provided by `srt-autosync`; this package handles CLI arguments, progress, cancellation, and output. The Whisper model is downloaded on demand. `--quiet` suppresses progress on stderr.

Successful runs write the synced SRT to `--output` and print a summary to stdout. Errors go to stderr and exit 1; invalid arguments exit 2. Ctrl+C cancels model download or analysis, does not write an output file, and exits 130. Existing output files are replaced only after a successful analysis. Suggested anchors are all applied before the output is written. Cancellation is checked immediately before the synchronous file save; once that save starts, it runs to completion.

```sh
cargo test -p srt-autosync-cli
```

See the [command-by-command CLI guide](../../docs/CLI.md) for examples, misuse
corrections and the current GUI/library coverage boundaries.
