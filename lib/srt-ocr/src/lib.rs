#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

//! Headless bitmap and hardcoded video subtitle extraction and OCR pipeline.

pub mod backend;
pub mod bitmap;
pub mod hardcoded;
pub mod local;
pub mod model;
pub mod pipeline;
pub mod vision;

pub use backend::OcrBackend;
pub use bitmap::BitmapSubtitleCue;
pub use hardcoded::{
    VideoFramePreview, VideoRoi, extract_hardcoded_subtitles, extract_video_preview_frame,
};
pub use local::LocalOcrBackend;
pub use model::OcrModelInfo;
pub use pipeline::{
    CompositeOcrBackend, HardcodedOcrConfig, OcrCallbacks, OcrConfig, OcrOutcome, OcrProgress,
    OcrSegment, build_backend_from_tiers, ocr_bitmap_track_to_srt, ocr_bitmap_track_with_backend,
    ocr_hardcoded_to_srt, ocr_hardcoded_with_backend, preview_ocr_bitmap_track,
    preview_ocr_hardcoded,
};
pub use vision::VisionLlmBackend;
