# srt-ocr

Headless bitmap subtitle extraction and OCR pipeline used by Vesta.

It extracts embedded bitmap subtitle tracks (`hdmv_pgs_subtitle`, `dvd_subtitle`, `dvb_subtitle`, `xsub`) from media containers into cropped RGBA/PNG subtitle cues, recognizes text via pluggable OCR backends (`OcrBackend`, `VisionLlmBackend`), and atomically writes standard `.srt` files with cancellation and progress callbacks.

## Usage

```bash
cargo test -p srt-ocr
```

## Components

- `bitmap`: Pure-Rust Blu-ray PGS (`.sup`) parser and FFmpeg/FFprobe bitmap track extractor (`BitmapSubtitleCue`, `parse_pgs_sup`, `extract_bitmap_subtitles`).
- `backend` / `vision`: `OcrBackend` trait and `VisionLlmBackend` backed by `srt-translate` multimodal Vision LLMs with tiered rate-limit failover.
- `pipeline`: End-to-end bounded-concurrency `ocr_bitmap_track_to_srt`, `ocr_bitmap_track_with_backend`, and `preview_ocr_bitmap_track`.
- `model`: Local OCR model catalog (`OCR_MODELS`) and atomic downloads via `srt-download`.
