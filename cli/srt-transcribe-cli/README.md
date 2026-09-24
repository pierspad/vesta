# srt-transcribe-cli

Headless command-line interface for converting audio or video into SRT subtitles.

```bash
cargo build --release -p srt-transcribe-cli
./target/release/srt-transcribe --help
```

GPU feature flags are forwarded to `srt-transcribe`: `vulkan`, `cuda`, `rocm`, and `sycl`.
