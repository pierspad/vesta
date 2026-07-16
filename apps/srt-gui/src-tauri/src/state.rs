use std::sync::Mutex;

use srt_sync::SyncEngine;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub struct SyncState {
    pub engine: Option<SyncEngine>,
    pub is_auto_syncing: bool,
    pub auto_sync_cancellation_token: Option<CancellationToken>,
}

pub type AppSyncState = Mutex<SyncState>;

#[derive(Default)]
pub struct TranslateState {
    pub is_translating: bool,
    pub cancellation_token: Option<CancellationToken>,
}

pub type AppTranslateState = Mutex<TranslateState>;

#[derive(Default)]
pub struct FlashcardState {
    pub is_processing: bool,
    pub cancellation_token: Option<CancellationToken>,
}

pub type AppFlashcardState = Mutex<FlashcardState>;

#[derive(Default)]
pub struct TranscribeState {
    pub is_transcribing: bool,
    pub cancellation_token: Option<CancellationToken>,
}

pub type AppTranscribeState = Mutex<TranscribeState>;

#[derive(Default)]
pub struct RefineState {
    pub is_refining: bool,
    pub cancellation_token: Option<CancellationToken>,
}

pub type AppRefineState = Mutex<RefineState>;

#[derive(Default)]
pub struct CondenseState {
    pub is_running: bool,
    pub cancellation_token: Option<CancellationToken>,
}

pub type AppCondenseState = Mutex<CondenseState>;
