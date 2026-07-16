# srt-autosync — automatic alignment via Whisper anchors

`lib/srt-autosync` aligns an out-of-sync SRT to a media file automatically —
the engine behind Vesta's "Auto Sync" button. The algorithm:

1. sample N positions across the media (quick: 12×20 s, precise: 24×40 s);
2. extract each to 16 kHz mono WAV via FFmpeg, skipping silent windows;
3. transcribe every sample with Whisper (word timestamps on);
4. fuzzy-match transcribed text against nearby subtitle lines (±45 s window,
   temporal weighting);
5. keep only matches agreeing with the dominant global offset (density-based
   "geometric verification");
6. dedupe per subtitle and space anchors ≥ 30 s apart.

The result is a list of `AnchorSuggestion`s; feeding them to
[`srt-sync`](srt-sync.md)'s `SyncEngine` produces the re-timed SRT.

**Library API**

- `AutoSyncConfig` (media path, ggml model path, language hint, quick mode,
  ffmpeg/ffprobe commands)
- `run_auto_sync(&config, subtitles, on_progress, &cancel_token)` →
  `AutoSyncOutcome { suggestions, segments_analyzed, cancelled }`
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

    let config = AutoSyncConfig {
        media_path: "movie.mp4".into(),
        model_path: srt_transcribe::model::model_file_path("base")?,
        language: Some("en".into()),
        quick: false,
        ffmpeg_cmd: "ffmpeg".into(),
        ffprobe_cmd: "ffprobe".into(),
    };

    let outcome = run_auto_sync(&config, subs, None, &CancellationToken::new()).await?;
    for s in &outcome.suggestions {
        engine.add_anchor(s.subtitle_id, s.corrected_time_ms, false).ok();
    }
    engine.save_synced_file("movie.synced.srt")?;
    Ok(())
}
```

## Extract it standalone

Copy `lib/srt-autosync/` + `lib/srt-transcribe/` (+ `lib/srt-sync/` and
`core/srt-parser/` if you apply the anchors). External deps: `whisper-rs`
(compiles whisper.cpp — needs cmake and a C++ toolchain), `tokio`,
`tokio-util`, `tempfile`, `anyhow`, `serde`. FFmpeg is a runtime requirement,
not a build one.
