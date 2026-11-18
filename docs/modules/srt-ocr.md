# srt-ocr — bitmap subtitle extraction and OCR

`lib/srt-ocr` extracts embedded bitmap subtitle tracks (`hdmv_pgs_subtitle` / PGS,
`dvd_subtitle` / VobSub, `dvb_subtitle` / DVB, `xsub`) from media containers
into RGBA/PNG cue images, recognizes their text through a pluggable
`OcrBackend` (`VisionLlmBackend` powered by [`srt-translate`](srt-translate.md)
in Phase 1), and atomically publishes standard `.srt` files.

**Library API** (see `lib/srt-ocr/src/lib.rs`, `src/bitmap.rs`, `src/vision.rs`, `src/pipeline.rs`, and `src/model.rs`)

- `bitmap::parse_pgs_sup(bytes, limit)` → `Result<Vec<BitmapSubtitleCue>>` (pure-Rust Blu-ray `.sup` decoder)
- `bitmap::extract_bitmap_subtitles(ffmpeg, ffprobe, input, stream_index, limit, cancel)` → `Result<Vec<BitmapSubtitleCue>>`
- `OcrBackend` trait + `VisionLlmBackend::new(&tiers)` / `VisionLlmBackend::with_diagnostics(&tiers, observer)`
- `ocr_bitmap_track_to_srt(&config, callbacks, &cancel)` → `Result<OcrOutcome>`
- `ocr_bitmap_track_with_backend(&config, backend, callbacks, &cancel)` → `Result<OcrOutcome>`
- `preview_ocr_bitmap_track(&config, &cancel)` → `Result<String>` (recognizes up to `preview_limit`, default 15 cues, in memory)
- `model::{list_models, model_file_path, download_model, uninstall_model, validate_model_id, get_models_dir}` (`OcrModelInfo`, `OCR_MODELS`, `~/.cache/vesta/ocr/`)

## Use as a binary

```bash
cargo build --release -p srt-ocr-cli
./target/release/srt-ocr models
./target/release/srt-ocr run --input movie.mkv --track 2 --output movie.en.srt --provider local --model llava
./target/release/srt-ocr run --input movie.mkv --track 2 --output movie.en.srt --provider google --model gemini-2.5-flash --api-key "$GEMINI_API_KEY"
```

See the [CLI guide](../../cli/srt-ocr-cli/README.md) and
[`docs/CLI.md`](../CLI.md#ocr-run-models-download-remove) for `--json` output,
provider rules, and exit codes.

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
