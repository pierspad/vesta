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

No async, no external tools: pure logic over `srt-parser` types, fully
serializable state (`SyncState`) for session persistence.

## Use as a binary

There is no interactive CLI: anchor placement is either a GUI affair or an
automated one. For the automated path use the
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

Copy `lib/srt-sync/` + `core/srt-parser/`. External deps: `anyhow`, `serde`,
`serde_json` only.
