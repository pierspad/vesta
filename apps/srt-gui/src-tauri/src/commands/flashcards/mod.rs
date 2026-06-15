//! Tauri adapter over the standalone [`srt_flashcards`] engine.
//!
//! All flashcard logic (parsing, matching, filtering, ffmpeg extraction, TSV/APKG
//! export) lives in the GUI-agnostic `srt-flashcards` library crate. This module
//! is a thin layer that exposes that engine as Tauri commands: it resolves the
//! bundled ffmpeg binary, forwards the engine's progress callback to the
//! frontend as `flashcard-progress` events, and wires cancellation to app state.

pub mod commands;
pub mod media;

/// Backwards-compatible alias so existing paths such as
/// `commands::flashcards::types::FlashcardConfig` keep resolving (used by the
/// `--benchmark` CLI mode in `main.rs` and by the integration tests).
pub mod types {
    pub use srt_flashcards::*;
}

pub use commands::*;
#[allow(unused_imports)]
pub use srt_flashcards::*;
