use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

pub use srt_refine::{RefineCard, RefineUpdate};
use srt_refine::{RefineEvent, RefineRunConfig, RefineRunSummary};

use crate::commands::translate::TierEntryConfig;
use crate::state::{AppRefineState, OperationGuard};

fn begin_refinement(
    state: &AppRefineState,
) -> Result<
    (
        OperationGuard<'_, crate::state::RefineState>,
        CancellationToken,
    ),
    String,
> {
    let guard = OperationGuard::begin(state, "ERR_ALREADY_RUNNING")?;
    let token = guard.token();
    Ok((guard, token))
}

#[tauri::command]
pub async fn refine_load_file(path: String) -> Result<Vec<RefineCard>, String> {
    tokio::task::spawn_blocking(move || srt_refine::load_cards(&path))
        .await
        .map_err(|e| format!("Task refine fallito: {e}"))?
}

#[tauri::command]
pub async fn refine_save_file(
    input_path: String,
    output_path: String,
    updates: Vec<RefineUpdate>,
) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || srt_refine::save_cards(&input_path, &output_path, updates))
        .await
        .map_err(|e| format!("Task refine fallito: {e}"))?
        .map(|_| true)
}

#[tauri::command]
pub async fn refine_card_llm_tiered(
    card: RefineCard,
    prompt: String,
    tiers: Vec<Vec<TierEntryConfig>>,
    state: State<'_, AppRefineState>,
) -> Result<String, String> {
    let pool = srt_translate::build_pool(&tiers)?;
    let (_guard, token) = begin_refinement(&state)?;
    let card_id = card.id.clone();
    let result: std::sync::Arc<std::sync::Mutex<Option<String>>> = Default::default();
    let result_cb = result.clone();

    let summary = srt_refine::refine_cards_tiered(
        vec![card],
        RefineRunConfig {
            prompt,
            batch_mode: false,
            batch_size: 1,
        },
        pool,
        move |event| {
            if let RefineEvent::CardDone { id, notes, .. } = event
                && id == card_id
            {
                *result_cb.lock().unwrap() = Some(notes);
            }
        },
        token,
    )
    .await?;

    let notes = result.lock().unwrap().take();
    notes.ok_or_else(|| {
        if summary.cancelled {
            "ERR_CANCELLED".to_string()
        } else if summary.pool_exhausted {
            "Tutti i tier LLM sono esauriti (rate limit/quota)".to_string()
        } else {
            "Nessuna risposta generata".to_string()
        }
    })
}

#[tauri::command]
pub async fn refine_cards_llm_tiered(
    app: AppHandle,
    cards: Vec<RefineCard>,
    prompt: String,
    tiers: Vec<Vec<TierEntryConfig>>,
    batch_mode: bool,
    run_id: String,
    state: State<'_, AppRefineState>,
) -> Result<RefineRunSummary, String> {
    let pool = srt_translate::build_pool(&tiers)?;

    let (_guard, cancellation_token) = begin_refinement(&state)?;

    let on_event = {
        let app = app.clone();
        move |event: RefineEvent| {
            let _ = app.emit(
                "refine-progress",
                serde_json::json!({ "runId": run_id, "event": event }),
            );
        }
    };

    srt_refine::refine_cards_tiered(
        cards,
        RefineRunConfig {
            prompt,
            batch_mode,
            batch_size: 5,
        },
        pool,
        on_event,
        cancellation_token,
    )
    .await
}

/// Cancella il refinement AI in corso.
#[tauri::command]
pub async fn refine_cancel(state: State<'_, AppRefineState>) -> Result<bool, String> {
    Ok(state.lock().map_err(|e| e.to_string())?.operation.cancel())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_guard_rejects_overlap_and_cancels_on_drop() {
        let state = AppRefineState::default();
        let (guard, token) = begin_refinement(&state).unwrap();
        assert!(begin_refinement(&state).is_err());
        assert!(state.lock().unwrap().operation.cancel());
        drop(guard);
        assert!(token.is_cancelled());
        assert!(!state.lock().unwrap().operation.cancel());
        assert!(begin_refinement(&state).is_ok());
    }
}
