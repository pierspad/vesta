# srt-sync

Subtitle synchronization primitives based on anchor points and interpolation.

The crate contains the reusable timing engine, media matching helpers, playback preparation, and subtitle sampling logic used by the GUI and `srt-autosync`.

Manual retiming is also available through the [srt-sync CLI](../../cli/srt-sync-cli/README.md), using the same engine without desktop dependencies.

Playback preparation (`srt-sync::playback`) passes natively playable audio
formats (`mp3`, `wav`, `ogg`, `m4a`, `aac`, `opus`, `flac`) through directly.
Other containers are converted via FFmpeg into cached 16 kHz mono PCM WAV
(`pcm_s16le`, `{hash}.wav`), reusing an existing cache entry when its timestamp
is at least as new as the source. Outputs are published atomically from a
`.partial.wav` sibling after successful preparation; requests are serialized
within the process and each FFmpeg invocation has a five-minute timeout.
Preview conversion is separate from flashcard export audio settings and has no
cancellation token.

```bash
cargo test -p srt-sync
```

See [`../../docs/modules/srt-sync.md`](../../docs/modules/srt-sync.md) for usage details. License: GPL-3.0-only.
