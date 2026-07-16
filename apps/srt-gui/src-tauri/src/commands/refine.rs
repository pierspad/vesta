//! Comandi Tauri per il refinement delle flashcard.
//!
//! Adapter sottile sopra [`srt_refine`]: caricamento/salvataggio TSV-APKG e
//! motore LLM (pool a tier condiviso con la traduzione: round-robin,
//! failover, rate limiting, budget) vivono nella libreria headless; qui
//! restano solo le firme `#[tauri::command]` e l'emissione degli eventi.

use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use srt_refine::{RefineEvent, RefineRunConfig, RefineRunSummary};
pub use srt_refine::{RefineCard, RefineUpdate};

use crate::commands::translate::TierEntryConfig;
use crate::state::AppRefineState;

/// Load flashcards from a TSV or APKG file
#[tauri::command]
pub async fn refine_load_file(path: String) -> Result<Vec<RefineCard>, String> {
    // Blocking I/O (zip + sqlite): keep it off the async runtime.
    tokio::task::spawn_blocking(move || srt_refine::load_cards(&path))
        .await
        .map_err(|e| format!("Task refine fallito: {e}"))?
}

/// Save refined flashcards back to a TSV or APKG file
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

/// Genera le note per una singola card usando il pool a tier (failover incluso).
#[tauri::command]
pub async fn refine_card_llm_tiered(
    card: RefineCard,
    prompt: String,
    tiers: Vec<Vec<TierEntryConfig>>,
) -> Result<String, String> {
    let pool = srt_translate::build_pool(&tiers)?;
    let card_id = card.id.clone();
    let result: std::sync::Arc<std::sync::Mutex<Option<String>>> = Default::default();
    let result_cb = result.clone();

    let summary = srt_refine::refine_cards_tiered(
        vec![card],
        RefineRunConfig { prompt, batch_mode: false, batch_size: 1 },
        pool,
        move |event| {
            if let RefineEvent::CardDone { id, notes, .. } = event
                && id == card_id
            {
                *result_cb.lock().unwrap() = Some(notes);
            }
        },
        CancellationToken::new(),
    )
    .await?;

    let notes = result.lock().unwrap().take();
    notes.ok_or_else(|| {
        if summary.pool_exhausted {
            "Tutti i tier LLM sono esauriti (rate limit/quota)".to_string()
        } else {
            "Nessuna risposta generata".to_string()
        }
    })
}

/// Avvia il refinement AI di più card usando il pool a tier.
///
/// Il progresso viene emesso come eventi Tauri `refine-progress`
/// (payload: [`RefineEvent`]); il frontend applica gli aggiornamenti.
#[tauri::command]
pub async fn refine_cards_llm_tiered(
    app: AppHandle,
    cards: Vec<RefineCard>,
    prompt: String,
    tiers: Vec<Vec<TierEntryConfig>>,
    batch_mode: bool,
    state: State<'_, AppRefineState>,
) -> Result<RefineRunSummary, String> {
    let pool = srt_translate::build_pool(&tiers)?;

    let cancellation_token = CancellationToken::new();
    {
        let mut refine_state = state.lock().map_err(|e| e.to_string())?;
        if refine_state.is_refining {
            return Err("Refinement già in corso".to_string());
        }
        refine_state.is_refining = true;
        refine_state.cancellation_token = Some(cancellation_token.clone());
    }

    let on_event = {
        let app = app.clone();
        move |event: RefineEvent| {
            let _ = app.emit("refine-progress", &event);
        }
    };

    let result = srt_refine::refine_cards_tiered(
        cards,
        RefineRunConfig { prompt, batch_mode, batch_size: 5 },
        pool,
        on_event,
        cancellation_token,
    )
    .await;

    if let Ok(mut refine_state) = state.lock() {
        refine_state.is_refining = false;
        refine_state.cancellation_token = None;
    }

    result
}

/// Cancella il refinement AI in corso.
#[tauri::command]
pub async fn refine_cancel(state: State<'_, AppRefineState>) -> Result<bool, String> {
    let refine_state = state.lock().map_err(|e| e.to_string())?;
    if let Some(token) = &refine_state.cancellation_token {
        token.cancel();
        Ok(true)
    } else {
        Ok(false)
    }
}
