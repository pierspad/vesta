//! Comandi Tauri per il refinement delle flashcard.
//!
//! Adapter sottile sopra [`srt_refine`]: il caricamento/salvataggio TSV-APKG
//! e la chiamata LLM vivono nella libreria headless (riusabile fuori dalla
//! GUI); qui restano solo le firme `#[tauri::command]`.

pub use srt_refine::{RefineCard, RefineLlmConfig, RefineUpdate};

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

#[tauri::command]
pub async fn refine_card_llm_with_config(
    card: RefineCard,
    prompt: String,
    config: RefineLlmConfig,
) -> Result<String, String> {
    srt_refine::refine_card_llm(&card, &prompt, config).await
}
