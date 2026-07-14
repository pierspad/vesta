//! Costruzione del pool a tier a partire da una configurazione dichiarativa.
//!
//! Un [`TierEntry`] descrive un endpoint (provider + modello + key + opzioni);
//! [`build_pool`] trasforma una lista ordinata di tier (`tiers[0]` = priorità
//! massima) in un [`TranslatorPool`] pronto per
//! [`translate_subtitles_tiered_cancellable`](crate::translate_subtitles_tiered_cancellable),
//! applicando i default per provider (base URL, RPM, modello) e scartando le
//! entry inutilizzabili (senza modello o, per i provider remoti, senza key).
//!
//! Questo modulo è il punto unico dei default per provider: GUI, CLI e
//! embedder di terze parti condividono le stesse regole.

use serde::{Deserialize, Serialize};

use crate::rate_limiter::RateLimitConfig;
use crate::translator::{ApiType, Translator, TranslatorConfig};
use crate::{PoolEntry, TranslatorPool};

/// Una singola entry di un tier: provider + modello + key + opzioni.
///
/// I nomi dei campi sono parte del contratto serde con i frontend; tenerli
/// stabili.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierEntry {
    /// Provider id: "google" | "groq" | "openrouter" | "mistral" | "github" |
    /// "nvidia" | "local" | "custom".
    pub provider: String,
    /// Id del modello da chiamare (vuoto = default del provider).
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

/// Default per provider: (tipo API, base URL, RPM consigliato, modello di default).
pub fn provider_defaults(provider: &str) -> (ApiType, &'static str, u32, &'static str) {
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

/// True se il provider può funzionare senza API key (endpoint locali o custom).
pub fn provider_allows_missing_key(provider: &str) -> bool {
    matches!(provider.to_lowercase().as_str(), "local" | "custom")
}

/// Costruisce una [`PoolEntry`] da una entry di tier (`tier_human` è 1-based,
/// usato solo per l'etichetta leggibile nei log/progress).
pub fn build_pool_entry(entry: &TierEntry, tier_human: usize) -> PoolEntry {
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

    let api_key = entry.api_key.clone().filter(|k| !k.trim().is_empty());

    let translator = Translator::new(TranslatorConfig {
        api_type,
        api_key,
        base_url,
        model: model.clone(),
    });

    // Rate limiter: usa l'rpm dichiarato, altrimenti il default del provider.
    // rpm == 0 significa "nessun limite" (es. local).
    let rpm = entry.rpm.unwrap_or(default_rpm);
    let rate_limiter = (rpm > 0).then(|| RateLimitConfig::with_burst(rpm, 3).create_limiter());

    PoolEntry {
        translator,
        rate_limiter,
        max_requests: entry.max_requests.filter(|n| *n > 0),
        label: format!("T{} · {} · {}", tier_human, entry.provider, model),
    }
}

/// Costruisce il pool a tier dalla configurazione. Le entry senza modello o,
/// per i provider remoti, senza key valida vengono scartate (defense-in-depth:
/// una config malformata non deve produrre batch che falliscono a runtime con
/// "API key is required").
pub fn build_pool(tiers: &[Vec<TierEntry>]) -> Result<TranslatorPool, String> {
    let pool: TranslatorPool = tiers
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
