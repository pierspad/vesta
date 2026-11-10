# srt-sync

Subtitle synchronization primitives based on anchor points and interpolation.

`SyncEngine` performs piecewise-linear timestamp interpolation across manual and automatic anchors (`add_anchor`, `remove_anchor`, `group_anchor_edits`), maintains a bounded 100-step undo/redo history (`undo`, `redo`), serializes session state (`save_session`, `load_session`), and writes retimed SRT files atomically (`save_synced_file`). Companion modules provide subtitle/media file discovery (`matching`), checkpoint candidate selection (`AdaptiveSampler`), and audio preview caching (`playback`). Manual retiming is also available through the [srt-sync CLI](../../cli/srt-sync-cli/README.md).

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
