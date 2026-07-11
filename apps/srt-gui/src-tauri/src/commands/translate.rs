//! Comandi Tauri per la traduzione di sottotitoli.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex as TokioMutex;
use tokio_util::sync::CancellationToken;

use srt_parser::SrtParser;
use srt_translate::{
    ApiType, PoolEntry, RateLimitConfig, TranslationProgress, Translator, TranslatorConfig,
    TranslatorPool, translate_subtitles_tiered_cancellable,
};

use crate::state::AppTranslateState;

/// Una singola entry di un tier: provider + modello + key + opzioni.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierEntryConfig {
    /// Provider id: "google" | "groq" | "openrouter" | "mistral" | "github" | "nvidia" | "local" | "custom".
    pub provider: String,
    /// Id del modello da chiamare.
    pub model: String,
    /// API key (assente per i provider locali).
    pub api_key: Option<String>,
    /// Base URL personalizzato (richiesto per "custom").
    pub api_url: Option<String>,
    /// Limite richieste/minuto desiderato (override del default per provider).
    pub rpm: Option<u32>,
    /// Budget opzionale di richieste per questo run.
    pub max_requests: Option<u32>,
}

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

/// Default per provider: (tipo API, base URL, RPM consigliato, modello di default).
fn provider_defaults(provider: &str) -> (ApiType, &'static str, u32, &'static str) {
    match provider.to_lowercase().as_str() {
        "google" | "gemini" => (
            ApiType::Google,
            "https://generativelanguage.googleapis.com/v1beta",
            15,
            "gemini-2.5-flash",
        ),
        "groq" => (
            ApiType::Groq,
            "https://api.groq.com/openai/v1",
            30,
            "llama-3.3-70b-versatile",
        ),
        "openrouter" => (
            ApiType::OpenRouter,
            "https://openrouter.ai/api/v1",
            20,
            "google/gemini-2.0-flash-001",
        ),
        "mistral" => (
            ApiType::Local,
            "https://api.mistral.ai/v1",
            30,
            "mistral-small-latest",
        ),
        "github" => (
            ApiType::Local,
            "https://models.github.ai/inference",
            10,
            "openai/gpt-4o-mini",
        ),
        "nvidia" => (
            ApiType::Local,
            "https://integrate.api.nvidia.com/v1",
            40,
            "meta/llama-3.3-70b-instruct",
        ),
        // local / custom / sconosciuti: OpenAI-compatible, nessun rate limit di default.
        _ => (ApiType::Local, "http://localhost:11434/v1", 0, "llama3.2"),
    }
}

/// Costruisce una `PoolEntry` da una entry di tier.
fn build_pool_entry(entry: &TierEntryConfig, tier_human: usize) -> PoolEntry {
    let (api_type, default_url, default_rpm, default_model) = provider_defaults(&entry.provider);

    let base_url = entry
        .api_url
        .clone()
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| default_url.to_string());

    let model = if entry.model.trim().is_empty() {
        default_model.to_string()
    } else {
        entry.model.clone()
    };

    let api_key = entry
        .api_key
        .clone()
        .filter(|k| !k.trim().is_empty());

    let translator = Translator::new(TranslatorConfig {
        api_type,
        api_key,
        base_url,
        model: model.clone(),
    });

    // Rate limiter: usa l'rpm dichiarato, altrimenti il default del provider.
    // rpm == 0 significa "nessun limite" (es. local).
    let rpm = entry.rpm.unwrap_or(default_rpm);
    let rate_limiter = if rpm > 0 {
        Some(RateLimitConfig::with_burst(rpm, 3).create_limiter())
    } else {
        None
    };

    PoolEntry {
        translator,
        rate_limiter,
        max_requests: entry.max_requests.filter(|n| *n > 0),
        label: format!("T{} · {} · {}", tier_human, entry.provider, model),
    }
}

/// True se il provider può funzionare senza API key (endpoint locali o custom).
fn provider_allows_missing_key(provider: &str) -> bool {
    matches!(provider.to_lowercase().as_str(), "local" | "custom")
}

/// Costruisce il pool a tier dalla configurazione. Le entry senza modello o,
/// per i provider remoti, senza key valida vengono scartate (il frontend le
/// filtra già: qui è defense-in-depth, così una config malformata non produce
/// batch che falliscono a runtime con "API key is required").
fn build_pool(config: &TranslateConfig) -> Result<TranslatorPool, String> {
    let pool: TranslatorPool = config
        .tiers
        .iter()
        .enumerate()
        .map(|(ti, tier)| {
            tier.iter()
                .filter(|e| !e.model.trim().is_empty())
                .filter(|e| {
                    provider_allows_missing_key(&e.provider)
                        || e.api_key.as_deref().is_some_and(|k| !k.trim().is_empty())
                })
                .map(|e| build_pool_entry(e, ti + 1))
                .collect::<Vec<PoolEntry>>()
        })
        .filter(|t| !t.is_empty())
        .collect();

    if pool.is_empty() {
        return Err(
            "Nessun tier configurato. Aggiungi almeno un endpoint nei Tier di precedenza."
                .to_string(),
        );
    }

    Ok(pool)
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

    // Costruisce il pool a tier. Se `config.tiers` è presente lo usa direttamente;
    // altrimenti ricade sui campi legacy creando un singolo tier (un'entry per key).
    let pool: TranslatorPool = build_pool(&config)?;

    if pool.is_empty() || pool.iter().all(|t| t.is_empty()) {
        return Err(
            "Nessun endpoint di traduzione configurato. Aggiungi almeno una key/tier nelle impostazioni."
                .to_string(),
        );
    }

    let output_path = PathBuf::from(&config.output_path);

    // Wrapper per il callback di progresso che emette eventi Tauri
    let app_handle = Arc::new(TokioMutex::new(app.clone()));
    
    let on_progress = {
        let app_handle = app_handle.clone();
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

            // Usa tokio spawn per emettere l'evento
            let app_handle = app_handle.clone();
            tokio::spawn(async move {
                if let Ok(app) = app_handle.try_lock() {
                    let _ = app.emit("translate-progress", event);
                }
            });
        }
    };

    // Esegui la traduzione a tier con failover automatico e supporto cancellazione.
    let translated: anyhow::Result<std::collections::HashMap<u32, srt_parser::Subtitle>> = translate_subtitles_tiered_cancellable(
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
                let _ = app.emit("translate-complete", TranslateResult {
                    success: false,
                    message: "Traduzione annullata dall'utente".to_string(),
                    output_path: None,
                    translated_count: 0,
                });
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
    let _ = app.emit("translate-complete", TranslateResult {
        success,
        message: format!("Tradotti {} sottotitoli su {}", translated.len(), total_count),
        output_path: success.then(|| config.output_path.clone()),
        translated_count: translated.len(),
    });

    Ok(TranslateResult {
        success,
        message: format!("Tradotti {} sottotitoli su {}", translated.len(), total_count),
        output_path: success.then_some(config.output_path),
        translated_count: translated.len(),
    })
}

/// Annulla la traduzione in corso
#[tauri::command]
pub async fn cancel_translation(
    state: State<'_, AppTranslateState>,
) -> Result<bool, String> {
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
        let original_text = original_subs.get(&id)
            .map(|s| s.text.clone())
            .unwrap_or_else(|| "—".to_string());
        let translated_text = translated_subs.get(&id)
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
