# srt-sync CLI

Manual, headless retiming through `srt-sync::SyncEngine`. No Tauri, FFmpeg or
Whisper installation is needed to run this command.

```sh
cargo run -p srt-sync-cli -- --input episode.srt --output episode.synced.srt --anchor 1:1500
```

An anchor is a subtitle ID and its corrected **start time in milliseconds**.
One anchor applies a constant shift. Repeat `--anchor` to correct drift with
the same interpolation/extrapolation policy as the desktop engine. Negative
resulting times clamp to zero. Anchor IDs must be positive and unique; corrected
times must be non-negative and increase with distinct original anchor times.
All anchors are validated before saving. Existing destination files are replaced
on success; use a separate output path to preserve the source.
Mapped timestamps beyond the shared SRT parser's supported hour range are
rejected before opening the destination.

The shared engine fills small gaps in subtitle IDs with `[...]` placeholders,
just as in the GUI. This CLI preserves that behavior. stdout stays empty;
completion and contextual errors go to stderr. Invalid arguments exit 2;
processing/I/O failures exit 1.

```sh
cargo test -p srt-sync-cli
```

GPL-3.0-only. For embedding, use `srt_sync::SyncEngine` directly rather than
spawning this executable. See [the engine guide](../../docs/modules/srt-sync.md).

See the [command-by-command CLI guide](../../docs/CLI.md) for examples, misuse
corrections and the current GUI/library coverage boundaries.
