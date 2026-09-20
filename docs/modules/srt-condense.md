# srt-condense

Dialogue audio extractor and silence trimmer for immersion listening.

## What it does

`srt-condense` creates "condensed audio" tracks from video and audio files by stitching together only dialogue portions while eliminating long silences and music breaks.

### Modes

1. **Subtitle-based (`CondenseMode::Subtitles`)**: Uses timestamp ranges from an existing `.srt` subtitle file with configurable start/end padding (`pad_ms`) and gap merging (`merge_gap_ms`).
2. **VAD-based (`CondenseMode::Vad`)**: Analyzes the audio stream directly with Voice Activity Detection (VAD) to locate speech without needing subtitles.

## Crate Info

- **Path**: `lib/srt-condense`
- **Dependencies**: `srt-parser`, `serde`, `tokio`, `tokio-util`

## Rust API Example

```rust
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use srt_condense::{CondenseConfig, CondenseMode, condense_audio};

let config = CondenseConfig {
    media_path: "episode01.mkv".to_string(),
    output_path: "episode01_condensed.mp3".to_string(),
    mode: CondenseMode::Subtitles { srt_path: "episode01.srt".to_string() },
    pad_ms: 150,
    merge_gap_ms: 1500,
    bitrate_kbps: 128,
    audio_track_index: None,
    n_threads: None,
};

let cancel = CancellationToken::new();
condense_audio(config, &cancel, Arc::new(|progress| {
    println!("{}: {:.1}%", progress.stage, progress.percentage);
})).await?;
```
