use serde::{Deserialize, Serialize};

use crate::rate_limiter::RateLimitConfig;
use crate::translator::{ApiType, Translator, TranslatorConfig};
use crate::{PoolEntry, TranslatorPool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierEntry {
    pub provider: String,

    pub model: String,

    pub api_key: Option<String>,

    pub api_url: Option<String>,

    pub rpm: Option<u32>,

    pub max_requests: Option<u32>,
}

pub fn provider_defaults(provider: &str) -> (ApiType, &'static str, u32, &'static str) {
    match provider.to_lowercase().as_str() {
        "openai" => (
            ApiType::Local,
            "https://api.openai.com/v1",
            0,
            "gpt-4o-mini",
        ),
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
            "openai/gpt-oss-20b",
        ),
        "openrouter" => (
            ApiType::OpenRouter,
            "https://openrouter.ai/api/v1",
            20,
            "google/gemini-2.5-flash",
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
            "deepseek-ai/deepseek-v4.1-flash",
        ),

        _ => (ApiType::Local, "http://localhost:11434/v1", 0, "llama3.2"),
    }
}

pub fn provider_allows_missing_key(provider: &str) -> bool {
    matches!(provider.to_lowercase().as_str(), "local" | "custom")
}

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

    let rpm = entry.rpm.unwrap_or(default_rpm);
    let rate_limiter = (rpm > 0).then(|| RateLimitConfig::with_burst(rpm, 3).create_limiter());

    PoolEntry {
        translator,
        rate_limiter,
        max_requests: entry.max_requests.filter(|n| *n > 0),
        label: format!("T{} · {} · {}", tier_human, entry.provider, model),
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_defaults() {
        let (api_type, url, _, _) = provider_defaults("OPENAI");
        assert_eq!(api_type, ApiType::Local);
        assert_eq!(url, "https://api.openai.com/v1");
        let (api_type, url, rpm, model) = provider_defaults("google");
        assert_eq!(api_type, ApiType::Google);
        assert_eq!(url, "https://generativelanguage.googleapis.com/v1beta");
        assert_eq!(rpm, 15);
        assert_eq!(model, "gemini-2.5-flash");

        let (api_type, url, rpm, model) = provider_defaults("groq");
        assert_eq!(api_type, ApiType::Groq);
        assert_eq!(url, "https://api.groq.com/openai/v1");
        assert_eq!(rpm, 30);
        assert_eq!(model, "openai/gpt-oss-20b");

        let (api_type, url, rpm, model) = provider_defaults("unknown_local");
        assert_eq!(api_type, ApiType::Local);
        assert_eq!(url, "http://localhost:11434/v1");
        assert_eq!(rpm, 0);
        assert_eq!(model, "llama3.2");
    }

    #[test]
    fn test_provider_allows_missing_key() {
        assert!(provider_allows_missing_key("local"));
        assert!(provider_allows_missing_key("LOCAL"));
        assert!(provider_allows_missing_key("custom"));
        assert!(!provider_allows_missing_key("google"));
        assert!(!provider_allows_missing_key("openai"));
    }

    #[test]
    fn test_build_pool_success() {
        let tiers = vec![
            vec![TierEntry {
                provider: "google".to_string(),
                model: "gemini-2.5-flash".to_string(),
                api_key: Some("test-key".to_string()),
                api_url: None,
                rpm: Some(10),
                max_requests: Some(50),
            }],
            vec![TierEntry {
                provider: "local".to_string(),
                model: "qwen2.5".to_string(),
                api_key: None, // allowed for local
                api_url: Some("http://localhost:11434/v1".to_string()),
                rpm: None,
                max_requests: None,
            }],
        ];

        let pool = build_pool(&tiers).unwrap();
        assert_eq!(pool.len(), 2);
        assert_eq!(pool[0].len(), 1);
        assert!(pool[0][0].label.contains("T1 · google · gemini-2.5-flash"));
        assert!(pool[0][0].rate_limiter.is_some());
        assert_eq!(pool[0][0].max_requests, Some(50));

        assert_eq!(pool[1].len(), 1);
        assert!(pool[1][0].label.contains("T2 · local · qwen2.5"));
    }

    #[test]
    fn test_build_pool_empty_or_invalid_returns_err() {
        let empty_tiers: Vec<Vec<TierEntry>> = vec![];
        assert!(build_pool(&empty_tiers).is_err());

        // Remote provider with missing key gets dropped, leaving pool empty -> error
        let invalid_tiers = vec![vec![TierEntry {
            provider: "google".to_string(),
            model: "gemini-2.5-flash".to_string(),
            api_key: None, // Missing key for google -> dropped
            api_url: None,
            rpm: None,
            max_requests: None,
        }]];
        assert!(build_pool(&invalid_tiers).is_err());
    }
}
