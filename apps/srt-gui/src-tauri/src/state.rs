//! Gestione dello stato globale dell'applicazione.

use std::sync::Mutex;

use srt_sync::SyncEngine;
use tokio_util::sync::CancellationToken;

/// Stato per la sincronizzazione sottotitoli
pub struct SyncState {
    pub engine: Option<SyncEngine>,
    pub is_auto_syncing: bool,
    pub auto_sync_cancellation_token: Option<CancellationToken>,
}

impl Default for SyncState {
    fn default() -> Self {
        Self { 
            engine: None,
            is_auto_syncing: false,
            auto_sync_cancellation_token: None,
        }
    }
}

/// Wrapper thread-safe per lo stato di sincronizzazione
pub type AppSyncState = Mutex<SyncState>;

/// Stato per la traduzione (configurazione)
pub struct TranslateState {
    pub api_key: Option<String>,
    pub api_type: Option<String>,
    pub is_translating: bool,
    pub cancellation_token: Option<CancellationToken>,
}

impl Default for TranslateState {
    fn default() -> Self {
        Self {
            api_key: None,
            api_type: None,
            is_translating: false,
            cancellation_token: None,
        }
    }
}

/// Wrapper thread-safe per lo stato di traduzione
pub type AppTranslateState = Mutex<TranslateState>;

/// Stato per la generazione di flashcard
pub struct FlashcardState {
    pub is_processing: bool,
    pub cancellation_token: Option<CancellationToken>,
}

impl Default for FlashcardState {
    fn default() -> Self {
        Self {
            is_processing: false,
            cancellation_token: None,
        }
    }
}

/// Wrapper thread-safe per lo stato flashcard
pub type AppFlashcardState = Mutex<FlashcardState>;

/// Stato per la trascrizione Whisper
pub struct TranscribeState {
    pub is_transcribing: bool,
    pub cancellation_token: Option<CancellationToken>,
}

impl Default for TranscribeState {
    fn default() -> Self {
        Self {
            is_transcribing: false,
            cancellation_token: None,
        }
    }
}

/// Wrapper thread-safe per lo stato trascrizione
pub type AppTranscribeState = Mutex<TranscribeState>;
