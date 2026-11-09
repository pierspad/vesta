# srt-autosync

Automatic subtitle-to-audio alignment engine (`AutoSyncConfig`, `run_auto_sync`, `run_auto_sync_with_cloud`).

Samples audio segments across a media file, transcribes them with local `whisper.cpp` or a configured cloud STT endpoint, fuzzy-matches transcribed spans against subtitle text, and selects consensus synchronization anchors (`AnchorSuggestion`). Anchor interpolation and retiming are handled by [`srt-sync`](../srt-sync).

```bash
cargo test -p srt-autosync
```

See [`docs/modules/srt-autosync.md`](../../docs/modules/srt-autosync.md) for the full workflow and Rust example. License: GPL-3.0-only.
