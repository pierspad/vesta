# srt-sync

Subtitle synchronization primitives based on anchor points and interpolation.

The crate contains the reusable timing engine, media matching helpers, playback preparation, and subtitle sampling logic used by the GUI and `srt-autosync`.

```bash
cargo test -p srt-sync
```

See [`../../docs/modules/srt-sync.md`](../../docs/modules/srt-sync.md) for usage details. License: GPL-3.0.
