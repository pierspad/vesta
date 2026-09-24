# srt-autosync

Automatic subtitle-to-audio alignment engine.

The engine samples a media file, transcribes the samples with Whisper, fuzzy-matches them against subtitle text, and proposes synchronization anchors. Anchor interpolation is handled by `srt-sync`.

```bash
cargo test -p srt-autosync
```

See [`../../docs/modules/srt-autosync.md`](../../docs/modules/srt-autosync.md) for the full workflow. License: GPL-3.0-only.
