//! Pluggable OCR backend abstraction.

use crate::bitmap::BitmapSubtitleCue;
use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use tokio_util::sync::CancellationToken;

/// Asynchronous OCR engine abstraction for recognizing bitmap subtitle cues.
pub trait OcrBackend: Send + Sync {
    /// Recognizes subtitle text from a rendered bitmap cue image.
    ///
    /// Implementations should return an empty string (`""`) when the cue contains
    /// no legible subtitle text and honor `cancel` cooperatively.
    fn recognize_cue<'a>(
        &'a self,
        cue: &'a BitmapSubtitleCue,
        language_hint: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>>;
}
