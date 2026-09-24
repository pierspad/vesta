# srt-autosync-cli

Headless command-line interface for Whisper-assisted subtitle synchronization.

```bash
cargo build --release -p srt-autosync-cli
./target/release/srt-autosync --help
```

The alignment engine is provided by `srt-autosync`; this package only handles CLI arguments and output.
