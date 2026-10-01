# srt-transcribe

Media-to-SRT transcription pipeline used by Vesta and `srt-transcribe-cli`.

It supports local `whisper.cpp`, optional Silero VAD, cloud speech-to-text providers, progress callbacks, cancellation, and GPU backends selected at compile time.

```bash
cargo test -p srt-transcribe
# Optional acceleration, for example:
cargo test -p srt-transcribe --features vulkan
```

See [`../../docs/modules/srt-transcribe.md`](../../docs/modules/srt-transcribe.md) for configuration and API examples.

Cloud transcription selects one provider rather than the translation tier pool.
OpenAI `whisper-1` supports segment timestamps; GPT-4o models use JSON with
coarse timing spanning the audio chunk. English translation requires a compatible
Whisper model. AssemblyAI polling is bounded to 15 minutes; local cancellation
interrupts the waiting request but does not delete the remote job.

`cargo test -p srt-transcribe` includes loopback HTTP contract tests for OpenAI,
Groq, Deepgram and AssemblyAI; these consume no API credits and do not certify
live account/model availability.
