# srt-transcribe

Media-to-SRT transcription pipeline used by Vesta and `srt-transcribe-cli`.

It supports local `whisper.cpp`, optional Silero VAD, cloud speech-to-text providers, progress callbacks, cancellation, and GPU backends selected at compile time.

```bash
cargo test -p srt-transcribe
# Optional acceleration, for example:
cargo test -p srt-transcribe --features vulkan
```

See [`../../docs/modules/srt-transcribe.md`](../../docs/modules/srt-transcribe.md) for configuration and API examples.
