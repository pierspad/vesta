# srt-autosync

Automatic subtitle-to-audio alignment engine (`AutoSyncConfig`, `run_auto_sync`, `run_auto_sync_with_cloud`).

Samples dialogue-dense audio segments across a media file, transcribes them with a reused local `WhisperState` (`transcribe_full_with_state`, WebRTC VAD pre-filtering, word-timestamp decoding flags) or a configured cloud STT endpoint, fuzzy-matches transcribed spans against subtitle text with token-level speech-onset refinement, and selects consensus synchronization anchors (`AnchorSuggestion`). Anchor interpolation and retiming are handled by [`srt-sync`](../srt-sync).

```bash
cargo test -p srt-autosync
```

See [`docs/modules/srt-autosync.md`](../../docs/modules/srt-autosync.md) for the full workflow and Rust example. License: GPL-3.0-only.
