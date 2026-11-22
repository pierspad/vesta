# vesta-ocr

Headless bitmap and hardcoded video subtitle extraction and OCR pipeline used by Vesta.

It extracts embedded bitmap subtitle tracks (`hdmv_pgs_subtitle`, `dvd_subtitle`, `dvb_subtitle`, `xsub`) or burned-in pixel regions (`VideoRoi`) from media containers into cropped RGBA/PNG subtitle cues, recognizes text via pluggable OCR backends (`OcrBackend`, `LocalOcrBackend` using pure-Rust PaddleOCR v4 ONNX inference, `VisionLlmBackend`, and `CompositeOcrBackend`), and atomically writes standard `.srt` files with cancellation and progress callbacks.

## Usage

```bash
cargo test -p vesta-ocr
```

## Components

- `bitmap`: Pure-Rust Blu-ray PGS (`.sup`) parser and FFmpeg/FFprobe bitmap track extractor (`BitmapSubtitleCue`, `parse_pgs_sup`, `extract_bitmap_subtitles`).
- `hardcoded`: Normalized `VideoRoi` cropping, single-frame PNG preview (`VideoFramePreview`, `extract_video_preview_frame`), and FFmpeg frame-sampling + border-contrast deduplication (`extract_hardcoded_subtitles`).
- `local`: Pure-Rust ONNX protobuf parser, SVTR_LCNet tensor runtime, horizontal line segmentation, and CTC greedy decoder (`LocalOcrBackend`) for offline PaddleOCR v4 mobile models without external shared libraries.
- `backend` / `vision`: `OcrBackend` trait and `VisionLlmBackend` backed by `vesta-translate` multimodal Vision LLMs with tiered rate-limit failover.
- `pipeline`: End-to-end bounded-concurrency `ocr_bitmap_track_to_srt`, `ocr_bitmap_track_with_backend`, `preview_ocr_bitmap_track`, `ocr_hardcoded_to_srt`, `ocr_hardcoded_with_backend`, and `preview_ocr_hardcoded`, plus `CompositeOcrBackend` / `build_backend_from_tiers`.
- `model`: Local OCR model catalog (`OCR_MODELS`) and atomic downloads via `vesta-download`.
