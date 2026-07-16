//! # srt-sync-lib
//!
//! Libreria per la sincronizzazione manuale di sottotitoli SRT con video.
//!
//! Questa libreria implementa un sistema di ancore (anchor points) per mappare
//! i tempi originali dei sottotitoli ai tempi corretti del video, usando
//! interpolazione lineare tra i punti di ancoraggio.

mod engine;
mod interpolator;
pub mod matching;
pub mod playback;
mod sampler;

pub use engine::{SyncEngine, SyncState};
pub use interpolator::{AnchorPoint, TimeMapper};
pub use matching::{
    suggest_companion_subtitle_for_srt, suggest_media_for_srt, suggest_subtitles_for_media,
};
pub use playback::{is_natively_playable, transcode_for_playback};
pub use sampler::{AdaptiveSampler, SamplerStrategy};
