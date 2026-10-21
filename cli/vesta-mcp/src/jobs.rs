use crate::paths::Paths;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;
use srt_flashcards::{FlashcardConfig, MediaTools};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, MutexGuard},
};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const HISTORY_LIMIT: usize = 64;

#[derive(Clone, Copy, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
}
impl Status {
    fn active(self) -> bool {
        matches!(self, Self::Running | Self::Cancelling)
    }
}

#[derive(Clone, Serialize, JsonSchema)]
pub struct JobSnapshot {
    pub job_id: String,
    pub status: Status,
    pub output_dir: String,
    pub progress: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<String>,
}
struct Entry {
    snapshot: JobSnapshot,
    cancel: CancellationToken,
}
#[derive(Default)]
struct State {
    entries: BTreeMap<String, Entry>,
    handles: Vec<JoinHandle<()>>,
    shutting_down: bool,
}

/// One generation at a time, with bounded in-memory history and cooperative shutdown.
#[derive(Clone, Default)]
pub struct Jobs {
    state: Arc<Mutex<State>>,
}
impl Jobs {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn start(&self, mut config: FlashcardConfig, paths: &Paths) -> Result<JobSnapshot, String> {
        let mut state = self.lock();
        if state.shutting_down {
            return Err("Server is shutting down".into());
        }
        if state.entries.values().any(|e| e.snapshot.status.active()) {
            return Err("A generation is already active; wait for it or cancel it first".into());
        }
        if state.entries.len() >= HISTORY_LIMIT
            && let Some(id) = state.entries.keys().next().cloned()
        {
            state.entries.remove(&id);
        }
        let output = paths.new_output()?;
        let id = output
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("Invalid output directory")?
            .to_owned();
        config.output_dir = output
            .to_str()
            .ok_or("Output root must be valid UTF-8")?
            .to_owned();
        let snapshot = JobSnapshot {
            job_id: id.clone(),
            status: Status::Running,
            output_dir: config.output_dir.clone(),
            progress: None,
            result: None,
            error: None,
        };
        let cancel = CancellationToken::new();
        state.entries.insert(
            id.clone(),
            Entry {
                snapshot: snapshot.clone(),
                cancel: cancel.clone(),
            },
        );
        state.handles.retain(|handle| !handle.is_finished());
        let jobs = self.clone();
        state.handles.push(tokio::spawn(async move {
            let progress_jobs = jobs.clone();
            let progress_id = id.clone();
            let worker_cancel = cancel.clone();
            // Supervise the engine so a task panic becomes an observable failed job.
            let outcome = tokio::spawn(async move {
                let progress = move |event| {
                    if let Some(entry) = progress_jobs.lock().entries.get_mut(&progress_id) {
                        entry.snapshot.progress = serde_json::to_value(event).ok();
                    }
                };
                srt_flashcards::generate(config, MediaTools::default(), worker_cancel, &progress)
                    .await
            })
            .await;
            let mut state = jobs.lock();
            if let Some(entry) = state.entries.get_mut(&id) {
                match outcome {
                    Ok(Ok(result)) if result.success => {
                        entry.snapshot.status = Status::Succeeded;
                        entry.snapshot.result = serde_json::to_value(result).ok();
                    }
                    other => {
                        entry.snapshot.status = if cancel.is_cancelled() {
                            Status::Cancelled
                        } else {
                            Status::Failed
                        };
                        entry.snapshot.error = Some(match other {
                            Ok(Ok(result)) => result.message,
                            Ok(Err(error)) => error,
                            Err(error) => format!("Generation task failed: {error}"),
                        });
                    }
                }
            }
        }));
        Ok(snapshot)
    }

    pub fn get(&self, id: &str) -> Result<JobSnapshot, String> {
        self.lock()
            .entries
            .get(id)
            .map(|entry| entry.snapshot.clone())
            .ok_or_else(|| "Unknown job ID (history lasts only for this server session)".into())
    }

    pub fn cancel(&self, id: &str) -> Result<JobSnapshot, String> {
        let mut state = self.lock();
        let entry = state.entries.get_mut(id).ok_or("Unknown job ID")?;
        if entry.snapshot.status.active() {
            entry.cancel.cancel();
            entry.snapshot.status = Status::Cancelling;
        }
        Ok(entry.snapshot.clone())
    }

    pub async fn shutdown(&self) {
        let handles = {
            let mut state = self.lock();
            state.shutting_down = true;
            for entry in state.entries.values() {
                entry.cancel.cancel();
            }
            std::mem::take(&mut state.handles)
        };
        for handle in handles {
            let _ = handle.await;
        }
    }
}
