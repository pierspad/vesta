//! Comandi Tauri per la traduzione di sottotitoli.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use srt_parser::SrtParser;
use srt_translate::{TranslationProgress, TranslatorPool, translate_subtitles_tiered_cancellable};

use crate::state::AppTranslateState;

/// Una singola entry di un tier: provider + modello + key + opzioni.
/// È direttamente il tipo della libreria (contratto serde condiviso).
pub type TierEntryConfig = srt_translate::TierEntry;

/// Configurazione per la traduzione.
///
/// La traduzione è guidata interamente dai `tiers` (lista di priorità con
/// failover automatico). `tiers[0]` ha la priorità massima.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateConfig {
    pub input_path: String,
    pub output_path: String,
    pub target_lang: String,
    pub batch_size: usize,
    pub resume_overlap: Option<usize>,
    pub title_context: Option<String>,

    /// Tier in ordine di priorità: `tiers[0]` ha la priorità massima.
    pub tiers: Vec<Vec<TierEntryConfig>>,
}

/// Evento di progresso emesso al frontend
#[derive(Debug, Clone, Serialize)]
pub struct TranslateProgressEvent {
    pub message: String,
    pub current_batch: usize,
    pub total_batches: usize,
    pub percentage: f64,
    pub eta_seconds: Option<f64>,
}

/// Risultato della traduzione
#[derive(Debug, Clone, Serialize)]
pub struct TranslateResult {
    pub success: bool,
    pub message: String,
    pub output_path: Option<String>,
    pub translated_count: usize,
}

/// Carica un file SRT e ritorna info di base
#[tauri::command]
pub async fn load_srt_for_translate(path: String) -> Result<SrtFileInfo, String> {
    let mut subtitles = SrtParser::parse_file(&path)
        .map_err(|e| format!("Errore nel parsing del file SRT: {}", e))?;

    // Normalizza: riempi buchi nella numerazione con "[...]"
    SrtParser::normalize_subtitles(&mut subtitles);

    let mut sorted: Vec<_> = subtitles.values().collect();
    sorted.sort_by_key(|s| s.id);

    let first_text = sorted.first().map(|s| s.text.clone()).unwrap_or_default();
    let last_text = sorted.last().map(|s| s.text.clone()).unwrap_or_default();

    Ok(SrtFileInfo {
        path,
        subtitle_count: subtitles.len(),
        first_subtitle: first_text,
        last_subtitle: last_text,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct SrtFileInfo {
    pub path: String,
    pub subtitle_count: usize,
    pub first_subtitle: String,
    pub last_subtitle: String,
}

/// Avvia la traduzione
#[tauri::command]
pub async fn start_translation(
    app: AppHandle,
    state: State<'_, AppTranslateState>,
    config: TranslateConfig,
) -> Result<TranslateResult, String> {
    // Crea un nuovo cancellation token
    let cancellation_token = CancellationToken::new();

    // Controlla se già in traduzione e salva il token
    {
        let mut translate_state = state.lock().map_err(|e| e.to_string())?;
        if translate_state.is_translating {
            return Err("Traduzione già in corso".to_string());
        }
        translate_state.is_translating = true;
        translate_state.cancellation_token = Some(cancellation_token.clone());
    }

    // Esegui la traduzione
    let result = perform_translation(app.clone(), config, cancellation_token.clone()).await;

    // Reset flag traduzione e rimuovi token
    {
        if let Ok(mut translate_state) = state.lock() {
            translate_state.is_translating = false;
            translate_state.cancellation_token = None;
        }
    }

    result
}

async fn perform_translation(
    app: AppHandle,
    config: TranslateConfig,
    cancellation_token: CancellationToken,
) -> Result<TranslateResult, String> {
    // Carica i sottotitoli
    let mut subtitles = SrtParser::parse_file(&config.input_path)
        .map_err(|e| format!("Errore caricamento SRT: {}", e))?;

    // Normalizza: riempi buchi nella numerazione con "[...]"
    SrtParser::normalize_subtitles(&mut subtitles);

    let total_count = subtitles.len();

    // Costruisce il pool a tier: i default per provider e il filtro delle
    // entry inutilizzabili vivono in `srt_translate::pool`.
    let pool: TranslatorPool = srt_translate::build_pool(&config.tiers)?;

    if pool.is_empty() || pool.iter().all(|t| t.is_empty()) {
        return Err(
            "Nessun endpoint di traduzione configurato. Aggiungi almeno una key/tier nelle impostazioni."
                .to_string(),
        );
    }

    let output_path = PathBuf::from(&config.output_path);

    // Callback di progresso: `AppHandle` è già `Clone + Send + Sync` ed `emit`
    // è sincrono, quindi non serve alcun wrapping (niente Arc<Mutex<..>>, niente
    // tokio::spawn). La versione precedente spawnava un task per ogni evento e
    // usava `try_lock`, che sotto contesa perdeva silenziosamente gli eventi di
    // progresso invece di aspettare — qui l'evento viene sempre emesso.
    let on_progress = {
        let app = app.clone();
        move |progress: TranslationProgress| {
            let percentage = if progress.total_batches > 0 {
                (progress.current_batch as f64 / progress.total_batches as f64) * 100.0
            } else {
                0.0
            };

            let event = TranslateProgressEvent {
                message: progress.message,
                current_batch: progress.current_batch,
                total_batches: progress.total_batches,
                percentage,
                eta_seconds: progress.eta_seconds,
            };

            let _ = app.emit("translate-progress", event);
        }
    };

    // Esegui la traduzione a tier con failover automatico e supporto cancellazione.
    let translated: anyhow::Result<std::collections::HashMap<u32, srt_parser::Subtitle>> =
        translate_subtitles_tiered_cancellable(
            pool,
            subtitles,
            &config.target_lang,
            config.batch_size,
            config.resume_overlap.unwrap_or(2),
            config.title_context.as_deref(),
            &output_path,
            on_progress,
            cancellation_token,
        )
        .await;

    // Gestisci la cancellazione
    let translated: std::collections::HashMap<u32, srt_parser::Subtitle> = match translated {
        Ok(t) => t,
        Err(e) => {
            let error_str = e.to_string();
            if error_str.contains("cancelled") || error_str.contains("annullat") {
                // Emetti evento di cancellazione
                let _ = app.emit(
                    "translate-complete",
                    TranslateResult {
                        success: false,
                        message: "Traduzione annullata dall'utente".to_string(),
                        output_path: None,
                        translated_count: 0,
                    },
                );
                return Ok(TranslateResult {
                    success: false,
                    message: "Traduzione annullata".to_string(),
                    output_path: None,
                    translated_count: 0,
                });
            }
            return Err(format!("Errore traduzione: {}", e));
        }
    };

    // Un run che termina senza aver prodotto nemmeno un sottotitolo non è un
    // successo: il frontend lo mostra come errore/warning invece che in verde.
    let success = !translated.is_empty();

    // Emetti evento di completamento
    let _ = app.emit(
        "translate-complete",
        TranslateResult {
            success,
            message: format!(
                "Tradotti {} sottotitoli su {}",
                translated.len(),
                total_count
            ),
            output_path: success.then(|| config.output_path.clone()),
            translated_count: translated.len(),
        },
    );

    Ok(TranslateResult {
        success,
        message: format!(
            "Tradotti {} sottotitoli su {}",
            translated.len(),
            total_count
        ),
        output_path: success.then_some(config.output_path),
        translated_count: translated.len(),
    })
}

/// Annulla la traduzione in corso
#[tauri::command]
pub async fn cancel_translation(state: State<'_, AppTranslateState>) -> Result<bool, String> {
    let mut translate_state = state.lock().map_err(|e| e.to_string())?;

    // Cancella il token se presente - questo fermerà tutte le richieste in corso
    if let Some(ref token) = translate_state.cancellation_token {
        token.cancel();
    }

    translate_state.is_translating = false;
    translate_state.cancellation_token = None;

    Ok(true)
}

/// Rappresenta una coppia di sottotitoli (originale e tradotto)
#[derive(Debug, Clone, Serialize)]
pub struct SubtitlePair {
    pub id: u32,
    pub original: String,
    pub translated: String,
}

/// Legge gli ultimi N sottotitoli dal file di input e output
#[tauri::command]
pub async fn get_latest_translated_subtitles(
    input_path: String,
    output_path: String,
    count: usize,
) -> Result<Vec<SubtitlePair>, String> {
    use std::path::Path;

    // Verifica che il file di output esista
    if !Path::new(&output_path).exists() {
        return Ok(vec![]);
    }

    // Carica i sottotitoli originali
    let original_subs = SrtParser::parse_file(&input_path)
        .map_err(|e| format!("Errore lettura file originale: {}", e))?;

    // Carica i sottotitoli tradotti
    let translated_subs = SrtParser::parse_file(&output_path)
        .map_err(|e| format!("Errore lettura file tradotto: {}", e))?;

    // Ottieni gli ID ordinati dei sottotitoli tradotti
    let mut translated_ids: Vec<u32> = translated_subs.keys().cloned().collect();
    translated_ids.sort();

    // Prendi gli ultimi N
    let start_idx = if translated_ids.len() > count {
        translated_ids.len() - count
    } else {
        0
    };

    let latest_ids = &translated_ids[start_idx..];

    // Crea le coppie
    let mut pairs = Vec::new();
    for &id in latest_ids {
        let original_text = original_subs
            .get(&id)
            .map(|s| s.text.clone())
            .unwrap_or_else(|| "—".to_string());
        let translated_text = translated_subs
            .get(&id)
            .map(|s| s.text.clone())
            .unwrap_or_else(|| "—".to_string());

        pairs.push(SubtitlePair {
            id,
            original: original_text,
            translated: translated_text,
        });
    }

    Ok(pairs)
}
