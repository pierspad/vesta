# srt-sync — anchor-based subtitle re-timing

`lib/srt-sync` is the deterministic re-timing engine behind Vesta's Sync tab:
you (or an algorithm) place **anchor points** — "subtitle #123 is actually
spoken at 00:41:07.300" — and the engine re-times every subtitle by
piecewise-linear interpolation between anchors. Manual anchors take precedence
over automatic ones.

**Library API highlights**

- `SyncEngine::new(srt_path)` / `from_state` / `load_session` / `save_session`
- `add_anchor(subtitle_id, corrected_time_ms, is_manual)` / `remove_anchor`
- `get_synced_subtitle(id)`, `get_all_synced_subtitles()`,
  `find_subtitle_at_time(ms)`, `get_average_offset()`
- `save_synced_file(output_path)` — writes the re-timed SRT
- `AdaptiveSampler` / `SamplerStrategy` — which subtitles to review next
- `matching` module — suggest media/subtitle file pairings by name similarity

The retiming engine is synchronous pure logic over `srt-parser` types, with
serializable state (`SyncState`) for session persistence. The separate `playback`
module invokes FFmpeg to prepare non-native containers and cache audio for the
desktop player; it is not part of anchor interpolation.

## Use as a binary

The standalone CLI applies explicit manual anchors without the GUI:

```sh
cargo run -p srt-sync-cli -- --input movie.srt --output movie.synced.srt \
  --anchor 1:2500 --anchor 123:2467300
```

Each anchor specifies a subtitle ID and its corrected start in milliseconds.
The CLI validates IDs and time order before writing. One anchor shifts the whole
file; multiple anchors interpolate drift. See the
[CLI guide](../../cli/srt-sync-cli/README.md) for validation and output semantics.
For automatic anchor discovery from media use the
[`srt-autosync`](srt-autosync.md) CLI, which drives this engine end-to-end
(`SRT + media → anchors → synced SRT`).

## Use as a Rust dependency

```toml
[dependencies]
srt-sync = { git = "https://github.com/pierspad/vesta" }
```

```rust
use srt_sync::SyncEngine;

fn main() -> anyhow::Result<()> {
    let mut engine = SyncEngine::new("movie.srt")?;
    // Dialogue at subtitle #1 actually starts 2.5 s later:
    engine.add_anchor(1, 2_500, true)?;
    // …and by the last line the drift has grown to 4 s:
    engine.add_anchor(engine.total_subtitles() as u32, 5_400_000 + 4_000, true)?;
    engine.save_synced_file("movie.synced.srt")?;
    Ok(())
}
```

## Extract it standalone

Copying directories alone is insufficient because manifests inherit workspace
dependencies, edition and Rust version. Keep the root workspace and select the
package with `cargo build -p srt-sync-cli`, or rewrite the inherited fields and
the parser dependency for your own workspace. The parser also needs
`chardetng` and `encoding_rs` for charset detection.
