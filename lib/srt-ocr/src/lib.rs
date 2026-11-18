#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

//! Headless bitmap subtitle (PGS, VobSub, DVB, XSUB) extraction and OCR pipeline.

pub mod backend;
pub mod bitmap;
pub mod model;
pub mod pipeline;
pub mod vision;

pub use backend::OcrBackend;
pub use bitmap::BitmapSubtitleCue;
pub use model::OcrModelInfo;
pub use pipeline::{
    OcrCallbacks, OcrConfig, OcrOutcome, OcrProgress, OcrSegment, ocr_bitmap_track_to_srt,
    ocr_bitmap_track_with_backend, preview_ocr_bitmap_track,
};
pub use vision::VisionLlmBackend;
