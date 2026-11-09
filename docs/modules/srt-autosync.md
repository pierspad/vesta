# srt-autosync — automatic alignment via Whisper anchors

`lib/srt-autosync` aligns an out-of-sync SRT to a media file automatically —
the engine behind Vesta's "Auto Sync" button. The algorithm:

1. sample dialogue-dense positions across the media (20 s segments in quick mode, 35 s segments in precise mode; scaled up to 16–32 samples when GPU acceleration is active);
2. extract each audio stream (`-vn -sn -dn`) to 16 kHz mono WAV via FFmpeg, skipping silent windows;
3. transcribe every sample with a single reused `WhisperState` (`transcribe_full_with_state`, word timestamps on, WebRTC VAD pre-filtering, `no_context`, `single_temperature`, `suppress_nst`, and `split_on_word`) or a configured cloud STT endpoint;
4. fuzzy-match transcribed text against nearby subtitle lines (default `max_offset_secs = 120.0`, or a ±15 s guided radius when manual anchors are provided) and refine speech onset via full-phrase or unique 2–4 word prefix token timestamps;
5. keep only matches agreeing with the dominant global offset (density-based
   consensus verification);
6. dedupe per subtitle and space anchors apart across the timeline.

The result is a list of `AnchorSuggestion`s; feeding them to
[`srt-sync`](srt-sync.md)'s `SyncEngine` produces the re-timed SRT.

**Library API**

- `AutoSyncConfig` (`AutoSyncConfig::new(media_path, model_path)`, language hint, quick mode, `use_gpu`, `max_offset_secs`, `existing_anchors`, ffmpeg/ffprobe commands)
- `ExistingAnchor { subtitle_id, original_ms, corrected_ms }` — manual anchors used to guide local search and constrain consensus
- `run_auto_sync(&config, subtitles, on_progress, &cancel_token)` →
  `AutoSyncOutcome { suggestions, segments_analyzed, cancelled }`
- `run_auto_sync_with_cloud(&config, &cloud_endpoint, subtitles, on_progress, &cancel_token)` — cloud speech-to-text alignment without a local Whisper model
- Progress callback carries `message_key` + `params` so UIs can localize.

## Use as a binary

```bash
cargo build --release -p srt-autosync-cli
./target/release/srt-autosync movie.srt movie.mp4 -o movie.synced.srt \
  --model base --language en --quick
```

The model is downloaded on demand (Hugging Face, cached in
`~/.cache/whisper/`). Requires `ffmpeg`/`ffprobe` on PATH (or via `--ffmpeg` /
`--ffprobe`).

## Use as a Rust dependency

```toml
[dependencies]
srt-autosync   = { git = "https://github.com/pierspad/vesta" }
srt-sync       = { git = "https://github.com/pierspad/vesta" }
srt-transcribe = { git = "https://github.com/pierspad/vesta" }
anyhow          = "1"
tokio          = { version = "1", features = ["full"] }
tokio-util     = "0.7"
```

```rust
use srt_autosync::{run_auto_sync, AutoSyncConfig, SubtitleLine};
use srt_sync::SyncEngine;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut engine = SyncEngine::new("movie.srt")?;
    let subs: Vec<SubtitleLine> = engine.get_all_subtitles().iter()
        .map(|s| SubtitleLine { id: s.id, start_ms: s.start.milliseconds as i64, text: s.text.clone() })
        .collect();

    let mut config = AutoSyncConfig::new(
        "movie.mp4",
        srt_transcribe::model::model_file_path("base")?,
    );
    config.language = Some("en".into());
    config.quick = false;
    config.ffmpeg_cmd = "ffmpeg".into();
    config.ffprobe_cmd = "ffprobe".into();

    let outcome = run_auto_sync(&config, subs, None, &CancellationToken::new()).await?;
    if outcome.cancelled {
        anyhow::bail!("Auto-sync was cancelled; no output was written");
    }
    for s in &outcome.suggestions {
        engine.add_anchor(s.subtitle_id, s.corrected_time_ms, false)?;
    }
    engine.save_synced_file("movie.synced.srt")?;
    Ok(())
}
```

## Vendoring & Standalone Integration

Prefer a Cargo Git dependency so workspace metadata and the root `[patch.crates-io]`
entry for `whisper-rs-sys` resolve automatically. When vendoring into another
workspace, include `lib/srt-autosync/`, `lib/srt-transcribe/`, `core/srt-parser/`,
and `core/srt-download/` (plus `lib/srt-sync/` if applying anchors), and preserve
the workspace dependency and patch definitions described in
[`docs/library-integration.md`](../library-integration.md).
