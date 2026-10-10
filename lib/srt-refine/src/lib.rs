use serde::{Deserialize, Serialize};
use srt_translate::{ApiType, Translator, TranslatorConfig};

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
    let api_type = match config.api_type.to_lowercase().as_str() {
        "local" => ApiType::Local,
        "google" | "gemini" => ApiType::Google,
        "groq" => ApiType::Groq,
        "custom" => ApiType::Local,
        _ => return Err(format!("Tipo API non supportato: {}", config.api_type)),
    };

    let base_url = config.api_url.unwrap_or_else(|| {
        match api_type {
            ApiType::Local => "http://localhost:11434/v1",
            ApiType::Google => "https://generativelanguage.googleapis.com/v1beta",
            ApiType::Groq => "https://api.groq.com/openai/v1",
            ApiType::OpenRouter => "https://openrouter.ai/api/v1",
        }
        .to_string()
    });

    let model = config.model.unwrap_or_else(|| {
        match api_type {
            ApiType::Local => "llama3.2",
            ApiType::Google => "gemini-2.0-flash",
            ApiType::Groq => "llama-3.3-70b-versatile",
            ApiType::OpenRouter => "google/gemini-2.0-flash-001",
        }
        .to_string()
    });

    let api_key = match &config.api_key {
        None => {
            if api_type == ApiType::Local {
                None
            } else {
                return Err("Chiave API mancante".to_string());
            }
        }
        Some(k) if k.is_empty() => {
            if api_type == ApiType::Local {
                None
            } else {
                return Err("Chiave API mancante".to_string());
            }
        }
        Some(_) => config.api_key.clone(),
    };

    let translator = Translator::new(TranslatorConfig {
        api_type,
        api_key,
        base_url,
        model,
    });

    translator
        .generate_response(&interpolate_prompt(prompt, card))
        .await
        .map_err(|e| format!("Errore chiamata LLM: {e}"))
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
