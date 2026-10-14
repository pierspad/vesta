# srt-transcribe-cli

Headless command-line interface for converting audio or video into SRT subtitles.

```bash
cargo build --release -p srt-transcribe-cli
./target/release/srt-transcribe --help
```

GPU feature flags are forwarded to `srt-transcribe`: `vulkan`, `cuda`, `rocm`, and `sycl`.

With `run --json`, stdout contains one JSON object per progress update followed
by exactly one terminal object. Success is `{"ok":true,"outcome":...}`;
errors are `{"ok":false,"cancelled":false,"error":"..."}` and exit 1.
Ctrl+C emits the terminal object with `cancelled:true` and exits 130. JSON mode
keeps stderr quiet so subprocess callers can parse stdout without filtering
progress or cancellation messages. Argument parsing errors still use clap's
usual stderr output and exit 2.

Cancellation is checked before publishing the SRT. Once synchronous publication
has started, it completes atomically and the command may report success even if
Ctrl+C arrives during that final write. A failed write preserves the previous file.
