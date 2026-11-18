# srt-ocr — bitmap and hardcoded subtitle OCR

`lib/srt-ocr` extracts embedded bitmap subtitle tracks (`hdmv_pgs_subtitle` / PGS,
`dvd_subtitle` / VobSub, `dvb_subtitle` / DVB, `xsub`) and burned-in (hardcoded)
video regions (`VideoRoi`) into RGBA/PNG cue images, recognizes their text
through a pluggable `OcrBackend` (`LocalOcrBackend` with pure-Rust PaddleOCR v4
ONNX inference, `VisionLlmBackend` powered by [`srt-translate`](srt-translate.md),
and `CompositeOcrBackend` for tiered failover), and atomically publishes standard
`.srt` files.

**Library API** (see `lib/srt-ocr/src/lib.rs`, `src/bitmap.rs`, `src/hardcoded.rs`, `src/local.rs`, `src/vision.rs`, `src/pipeline.rs`, and `src/model.rs`)

- `bitmap::parse_pgs_sup(bytes, limit)` → `Result<Vec<BitmapSubtitleCue>>` (pure-Rust Blu-ray `.sup` decoder)
- `bitmap::extract_bitmap_subtitles(ffmpeg, ffprobe, input, stream_index, limit, cancel)` → `Result<Vec<BitmapSubtitleCue>>`
- `hardcoded::{VideoRoi, VideoFramePreview, extract_video_preview_frame, extract_hardcoded_subtitles}` (normalized `0.0..=1.0` video ROI cropping, preview PNG extraction, FFmpeg `fps` sampling, and `32x12` spatial IoU cue deduplication)
- `local::LocalOcrBackend` (pure-Rust ONNX protobuf parser, SVTR_LCNet tensor runtime, horizontal line segmentation, and CTC greedy decoder; zero C/C++ shared library dependencies)
- `OcrBackend` trait, `VisionLlmBackend`, `CompositeOcrBackend`, and `build_backend_from_tiers(&tiers)`
- `ocr_bitmap_track_to_srt(&config, callbacks, &cancel)` / `ocr_bitmap_track_with_backend(...)` / `preview_ocr_bitmap_track(&config, &cancel)`
- `ocr_hardcoded_to_srt(&config, callbacks, &cancel)` / `ocr_hardcoded_with_backend(...)` / `preview_ocr_hardcoded(&config, &cancel)`
- `model::{list_models, model_file_path, download_model, uninstall_model, validate_model_id, get_models_dir}` (`OcrModelInfo`, `OCR_MODELS`, `~/.cache/vesta/ocr/`)

## Use as a binary

```bash
cargo build --release -p srt-ocr-cli
./target/release/srt-ocr models
./target/release/srt-ocr download paddleocr-v4-mobile-multi
./target/release/srt-ocr run --input movie.mkv --track 2 --output movie.en.srt --provider local --model llava
./target/release/srt-ocr run --input movie.mp4 --roi 0.08,0.74,0.84,0.20 --fps 2.5 --output movie.hardcoded.srt --provider paddle
./target/release/srt-ocr run --input movie.mkv --track 2 --output movie.en.srt --provider google --model gemini-2.5-flash --api-key "$GEMINI_API_KEY"
```

See the [CLI guide](../../cli/srt-ocr-cli/README.md) and
[`docs/CLI.md`](../CLI.md#ocr-run-models-download-remove) for `--json` output,
provider rules, `--roi` / `--fps`, and exit codes.

## Use as a Rust dependency

```toml
[dependencies]
srt-ocr   = { git = "https://github.com/pierspad/vesta" }
srt-translate = { git = "https://github.com/pierspad/vesta" }
anyhow    = "1"
tokio     = { version = "1", features = ["full"] }
tokio-util = "0.7"
```

```rust
use srt_ocr::{ocr_bitmap_track_to_srt, OcrCallbacks, OcrConfig};
use srt_translate::TierEntry;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = OcrConfig {
        input_path: "movie.mkv".into(),
        output_path: "movie.en.srt".into(),
        track_index: 2,
        language: Some("en".into()),
        tiers: vec![vec![TierEntry {
            provider: "local".into(),
            model: "llava".into(),
            api_key: None,
            api_url: None,
            rpm: None,
            max_requests: None,
        }]],
        preview_limit: None,
        ffmpeg_path: "ffmpeg".into(),
        ffprobe_path: "ffprobe".into(),
    };

    let outcome = ocr_bitmap_track_to_srt(&config, OcrCallbacks::default(), &CancellationToken::new()).await?;
    println!("Wrote {} cues to {}", outcome.subtitle_count, outcome.output_path);
    Ok(())
}
```

## Vendoring & Standalone Integration

Prefer the Git dependency above. When vendoring into another workspace, include
`lib/srt-ocr/`, `lib/srt-extract/`, `lib/srt-translate/`, `core/srt-parser/`,
and `core/srt-download/`, and supply the workspace metadata and dependencies
(`png`, `base64`, `reqwest` with `rustls`, `serde`, `serde_json`, `tokio`,
`tokio-util`, `tempfile`, `dirs`); see [`docs/library-integration.md`](../library-integration.md).
