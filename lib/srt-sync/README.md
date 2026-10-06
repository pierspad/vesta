# srt-sync

Subtitle synchronization primitives based on anchor points and interpolation.

The crate contains the reusable timing engine, media matching helpers, playback preparation, and subtitle sampling logic used by the GUI and `srt-autosync`.

Playback preparation passes native formats through, otherwise reuses a fresh
nonempty cache or tries an OGG audio copy before Opus/Vorbis conversion. Outputs
are published after successful preparation; requests are serialized within the
process and each FFmpeg invocation has a five-minute timeout. Preview conversion
is separate from flashcard export audio settings and has no cancellation token.

```bash
cargo test -p srt-sync
```

See [`../../docs/modules/srt-sync.md`](../../docs/modules/srt-sync.md) for usage details. License: GPL-3.0.
