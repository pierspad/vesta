//! Gestione dello stato globale dell'applicazione.

use std::sync::Mutex;

use srt_sync::SyncEngine;
use tokio_util::sync::CancellationToken;

/// Stato per la sincronizzazione sottotitoli
#[derive(Default)]
pub struct SyncState {
    pub engine: Option<SyncEngine>,
    pub is_auto_syncing: bool,
    pub auto_sync_cancellation_token: Option<CancellationToken>,
}

/// Wrapper thread-safe per lo stato di sincronizzazione
pub type AppSyncState = Mutex<SyncState>;

/// Stato per la traduzione (run in corso + cancellazione).
/// La configurazione (tier, key, modelli) viaggia per-run dentro
/// `TranslateConfig`: non viene mai persistita nel backend.
#[derive(Default)]
pub struct TranslateState {
    pub is_translating: bool,
    pub cancellation_token: Option<CancellationToken>,
}

/// Wrapper thread-safe per lo stato di traduzione
pub type AppTranslateState = Mutex<TranslateState>;

/// Stato per la generazione di flashcard
#[derive(Default)]
pub struct FlashcardState {
    pub is_processing: bool,
    pub cancellation_token: Option<CancellationToken>,
}

/// Wrapper thread-safe per lo stato flashcard
pub type AppFlashcardState = Mutex<FlashcardState>;

/// Stato per la trascrizione Whisper
#[derive(Default)]
pub struct TranscribeState {
    pub is_transcribing: bool,
    pub cancellation_token: Option<CancellationToken>,
}

/// Wrapper thread-safe per lo stato trascrizione
pub type AppTranscribeState = Mutex<TranscribeState>;
