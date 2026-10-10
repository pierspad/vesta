use std::sync::Mutex;

use srt_sync::SyncEngine;
use tokio_util::sync::CancellationToken;

// The token's presence is the running state: cancellation requests stop work,
// while only the owning guard releases the slot after that work has ended.
#[derive(Default)]
pub struct OperationState {
    token: Option<CancellationToken>,
}

impl OperationState {
    pub fn cancel(&self) -> bool {
        if let Some(token) = &self.token {
            token.cancel();
            true
        } else {
            false
        }
    }
}

pub trait HasOperation {
    fn operation(&mut self) -> &mut OperationState;
}

pub struct OperationGuard<'a, T: HasOperation> {
    state: &'a Mutex<T>,
    token: CancellationToken,
}

impl<'a, T: HasOperation> OperationGuard<'a, T> {
    pub fn begin(state: &'a Mutex<T>, busy_message: &str) -> Result<Self, String> {
        Self::with_token(state, busy_message, CancellationToken::new())
    }

    pub fn with_token(
        state: &'a Mutex<T>,
        busy_message: &str,
        token: CancellationToken,
    ) -> Result<Self, String> {
        let mut current = state.lock().map_err(|e| e.to_string())?;
        let operation = current.operation();
        if operation.token.is_some() {
            return Err(busy_message.to_owned());
        }
        operation.token = Some(token.clone());
        Ok(Self { state, token })
    }

    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }
}

impl<T: HasOperation> Drop for OperationGuard<'_, T> {
    fn drop(&mut self) {
        self.token.cancel();
        // Recover only to release resources; ordinary access still reports poison.
        let mut current = self.state.lock().unwrap_or_else(|error| error.into_inner());
        current.operation().token = None;
    }
}

#[derive(Default)]
pub struct SyncState {
    pub engine: Option<SyncEngine>,
    pub operation: OperationState,
}

impl HasOperation for SyncState {
    fn operation(&mut self) -> &mut OperationState {
        &mut self.operation
    }
}

pub type AppSyncState = Mutex<SyncState>;

// Keep distinct types for Tauri's type-based state registry.
macro_rules! operation_state {
    ($state:ident, $app_state:ident) => {
        #[derive(Default)]
        pub struct $state {
            pub operation: OperationState,
        }
        impl HasOperation for $state {
            fn operation(&mut self) -> &mut OperationState {
                &mut self.operation
            }
        }
        pub type $app_state = Mutex<$state>;
    };
}
operation_state!(TranslateState, AppTranslateState);
operation_state!(FlashcardState, AppFlashcardState);
operation_state!(TranscribeState, AppTranscribeState);
operation_state!(RefineState, AppRefineState);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_does_not_allow_overlap_until_the_owner_exits() {
        let state = AppTranscribeState::default();
        let guard = OperationGuard::begin(&state, "busy").unwrap();
        let token = guard.token();
        assert!(state.lock().unwrap().operation.cancel());
        assert!(token.is_cancelled());
        assert!(OperationGuard::begin(&state, "busy").is_err());
        drop(guard);
        let next = OperationGuard::begin(&state, "busy").unwrap();
        assert!(!next.token().is_cancelled());
    }

    #[test]
    fn shared_token_cancels_both_slots_and_failed_acquisition_releases_the_first() {
        let transcribe = AppTranscribeState::default();
        let sync = AppSyncState::default();
        let first = OperationGuard::begin(&transcribe, "busy").unwrap();
        let second = OperationGuard::with_token(&sync, "busy", first.token()).unwrap();
        assert!(transcribe.lock().unwrap().operation.cancel());
        assert!(second.token().is_cancelled());
        drop(first);
        drop(second);
        assert!(!sync.lock().unwrap().operation.cancel());
        let _occupied = OperationGuard::begin(&sync, "busy").unwrap();
        let attempt = || -> Result<(), String> {
            let reserved = OperationGuard::begin(&transcribe, "busy")?;
            let _sync = OperationGuard::with_token(&sync, "busy", reserved.token())?;
            Ok(())
        };
        assert!(attempt().is_err());
        assert!(OperationGuard::begin(&transcribe, "busy").is_ok());
    }

    #[tokio::test]
    async fn aborting_the_command_cancels_and_releases_its_slot() {
        let state = std::sync::Arc::new(AppTranslateState::default());
        let task_state = state.clone();
        let (send, ready) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let guard = OperationGuard::begin(&task_state, "busy").unwrap();
            send.send(guard.token()).unwrap();
            std::future::pending::<()>().await;
        });
        let token = ready.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(token.is_cancelled());
        assert!(OperationGuard::begin(&state, "busy").is_ok());
    }
}
