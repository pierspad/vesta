use serde::{Deserialize, Serialize};
use srt_translate::{ApiType, Translator, TranslatorConfig, provider_defaults};

mod cards;
pub use cards::{analyze_tsv_columns, load_cards, save_cards};
pub mod engine;
pub use engine::{RefineEvent, RefineRunConfig, RefineRunSummary, refine_cards_tiered};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineCard {
    pub id: String,

    pub expression: String,

    pub meaning: String,

    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineUpdate {
    pub id: String,
    pub notes: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RefineLlmConfig {
    pub api_type: String,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub model: Option<String>,
}

pub async fn refine_card_llm(
    card: &RefineCard,
    prompt: &str,
    config: RefineLlmConfig,
) -> Result<String, String> {
    let translator = Translator::try_new(resolve_llm_config(config)?)
        .map_err(|e| format!("Errore configurazione client HTTP: {e}"))?;

    translator
        .generate_response(&interpolate_prompt(prompt, card))
        .await
        .map_err(|e| format!("Errore chiamata LLM: {e}"))
}

fn resolve_llm_config(config: RefineLlmConfig) -> Result<TranslatorConfig, String> {
    let provider = config.api_type.to_lowercase();
    if !matches!(
        provider.as_str(),
        "local" | "custom" | "google" | "gemini" | "groq" | "openrouter"
    ) {
        return Err(format!("Tipo API non supportato: {}", config.api_type));
    }
    let (api_type, default_url, _, default_model) = provider_defaults(&provider);
    let api_key = config.api_key.filter(|key| !key.trim().is_empty());
    if api_type != ApiType::Local && api_key.is_none() {
        return Err("Chiave API mancante".to_string());
    }
    Ok(TranslatorConfig {
        api_type,
        api_key,
        base_url: config.api_url.unwrap_or_else(|| default_url.to_string()),
        model: config.model.unwrap_or_else(|| default_model.to_string()),
    })
}

pub fn interpolate_prompt(template: &str, card: &RefineCard) -> String {
    let expression = strip_html(&card.expression);
    let meaning = strip_html(&card.meaning);
    template
        .replace("{{expression}}", &expression)
        .replace("{{front}}", &expression)
        .replace("{{meaning}}", &meaning)
        .replace("{{back}}", &meaning)
        .replace("{{notes}}", &card.notes)
}

pub fn strip_html(text: &str) -> std::borrow::Cow<'_, str> {
    if !text.contains('<') {
        return std::borrow::Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    std::borrow::Cow::Owned(out)
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn legacy_configuration_uses_shared_defaults_and_preserves_overrides() {
        for provider in ["google", "gemini", "groq", "openrouter", "local", "custom"] {
            let config = resolve_llm_config(RefineLlmConfig {
                api_type: provider.into(),
                api_key: Some("key".into()),
                api_url: None,
                model: None,
            })
            .unwrap();
            let (api_type, url, _, model) = provider_defaults(provider);
            assert_eq!(config.api_type, api_type);
            assert_eq!(config.base_url, url);
            assert_eq!(config.model, model);
        }
        let custom = resolve_llm_config(RefineLlmConfig {
            api_type: "custom".into(),
            api_key: Some("  ".into()),
            api_url: Some("http://localhost:8080/v1".into()),
            model: Some("custom-model".into()),
        })
        .unwrap();
        assert!(custom.api_key.is_none());
        assert_eq!(custom.base_url, "http://localhost:8080/v1");
        assert_eq!(custom.model, "custom-model");
        for provider in ["google", "groq", "openrouter", "unsupported"] {
            assert!(
                resolve_llm_config(RefineLlmConfig {
                    api_type: provider.into(),
                    api_key: Some("  ".into()),
                    api_url: None,
                    model: None,
                })
                .is_err()
            );
        }
    }
}
