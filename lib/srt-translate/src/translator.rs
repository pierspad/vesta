use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

use crate::prompts::{
    build_batch_translation_prompt, build_context_enhanced_translation_prompt,
    build_single_translation_prompt,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ApiType {
    Local,

    Google,

    Groq,

    #[allow(dead_code)]
    OpenRouter,
}

#[derive(Clone)]
pub struct TranslatorConfig {
    pub base_url: String,

    pub model: String,

    pub api_key: Option<String>,

    pub api_type: ApiType,
}

impl Default for TranslatorConfig {
    fn default() -> Self {
        Self {
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            model: "gemini-2.0-flash".to_string(),
            api_key: None,
            api_type: ApiType::Google,
        }
    }
}

#[derive(Clone)]
pub struct Translator {
    config: TranslatorConfig,
    client: reqwest::Client,
}

impl Translator {
    pub fn new(config: TranslatorConfig) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { config, client }
    }

    pub async fn translate(
        &self,
        text: &str,
        target_lang: &str,
        context: Option<&str>,
    ) -> Result<String> {
        match self.config.api_type {
            ApiType::Google => self.translate_google(text, target_lang, context).await,
            ApiType::Local | ApiType::OpenRouter | ApiType::Groq => {
                self.translate_openai(text, target_lang, context).await
            }
        }
    }

    pub async fn translate_with_context(
        &self,
        text: &str,
        target_lang: &str,
        title_context: Option<&str>,
        surrounding_context: Option<&str>,
    ) -> Result<String> {
        match self.config.api_type {
            ApiType::Google => {
                self.translate_with_context_google(
                    text,
                    target_lang,
                    title_context,
                    surrounding_context,
                )
                .await
            }
            ApiType::Local | ApiType::OpenRouter | ApiType::Groq => {
                self.translate_with_context_openai(
                    text,
                    target_lang,
                    title_context,
                    surrounding_context,
                )
                .await
            }
        }
    }

    pub async fn translate_batch(
        &self,
        texts_with_ids: &[(u32, String)],
        target_lang: &str,
        context: Option<&str>,
    ) -> Result<HashMap<u32, String>> {
        match self.config.api_type {
            ApiType::Google => {
                self.translate_batch_google(texts_with_ids, target_lang, context)
                    .await
            }
            ApiType::Local | ApiType::OpenRouter | ApiType::Groq => {
                self.translate_batch_openai(texts_with_ids, target_lang, context)
                    .await
            }
        }
    }

    pub async fn generate_response(&self, prompt: &str) -> Result<String> {
        match self.config.api_type {
            ApiType::Google => self.call_google_api(prompt).await,
            ApiType::Local | ApiType::OpenRouter | ApiType::Groq => {
                self.call_openai_api(prompt).await
            }
        }
    }

    async fn translate_google(
        &self,
        text: &str,
        target_lang: &str,
        context: Option<&str>,
    ) -> Result<String> {
        let prompt = build_single_translation_prompt(text, target_lang, context);
        self.call_google_api(&prompt).await
    }

    async fn translate_batch_google(
        &self,
        texts_with_ids: &[(u32, String)],
        target_lang: &str,
        context: Option<&str>,
    ) -> Result<HashMap<u32, String>> {
        let prompt = build_batch_translation_prompt(texts_with_ids, target_lang, context);
        let result_text = self.call_google_api(&prompt).await?;

        let translations = parse_json_translations(&result_text, texts_with_ids.len())?;
        Ok(translations)
    }

    async fn translate_with_context_google(
        &self,
        text: &str,
        target_lang: &str,
        title_context: Option<&str>,
        surrounding_context: Option<&str>,
    ) -> Result<String> {
        let prompt = build_context_enhanced_translation_prompt(
            text,
            target_lang,
            title_context,
            surrounding_context,
        );
        self.call_google_api(&prompt).await
    }

    async fn call_google_api(&self, prompt: &str) -> Result<String> {
        #[derive(Serialize)]
        struct Part {
            text: String,
        }

        #[derive(Serialize)]
        struct Content {
            parts: Vec<Part>,
        }

        #[derive(Serialize)]
        struct GenerationConfig {
            temperature: f32,
        }

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct GeminiRequest {
            contents: Vec<Content>,
            generation_config: GenerationConfig,
        }

        #[derive(Deserialize)]
        struct GeminiPart {
            text: Option<String>,
        }

        #[derive(Deserialize)]
        struct GeminiContent {
            parts: Option<Vec<GeminiPart>>,
        }

        #[derive(Deserialize)]
        struct GeminiCandidate {
            content: Option<GeminiContent>,
        }

        #[derive(Deserialize)]
        struct GeminiError {
            message: Option<String>,
            code: Option<i32>,
        }

        #[derive(Deserialize)]
        struct GeminiResponse {
            candidates: Option<Vec<GeminiCandidate>>,
            error: Option<GeminiError>,
        }

        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Google API key is required"))?;

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.config.base_url.trim_end_matches('/'),
            self.config.model,
            api_key
        );

        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: GenerationConfig { temperature: 0.3 },
        };

        const MAX_RETRIES: u32 = 3;

        for attempt in 0..=MAX_RETRIES {
            let http_response = self
                .client
                .post(&url)
                .header("x-goog-api-key", api_key)
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
                .map_err(|e| {
                    eprintln!(
                        "[srt-translate] Google API request failed: {} (url: {})",
                        e,
                        &url[..url.find('?').unwrap_or(url.len())]
                    );
                    e
                })?;

            let status = http_response.status();
            let response_text = http_response.text().await?;

            if let Some(delay) = retry_backoff(status, &response_text, attempt, MAX_RETRIES) {
                sleep(delay).await;
                continue;
            }

            if !status.is_success() {
                eprintln!(
                    "[srt-translate] Google API error response (status {}): {}",
                    status,
                    &response_text[..response_text.len().min(500)]
                );
            }

            let response: GeminiResponse = serde_json::from_str(&response_text).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse Google API response (status {}): {}. Raw: {}",
                    status,
                    e,
                    &response_text[..response_text.len().min(500)]
                )
            })?;

            if let Some(ref error) = response.error {
                let error_msg = error
                    .message
                    .as_deref()
                    .unwrap_or("Unknown Google API error");
                let error_code = error.code.unwrap_or(0);
                anyhow::bail!("Google API error (code {}): {}", error_code, error_msg);
            }

            let text = response
                .candidates
                .and_then(|c| c.into_iter().next())
                .and_then(|c| c.content)
                .and_then(|c| c.parts)
                .and_then(|p| p.into_iter().next())
                .and_then(|p| p.text)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Google API response missing text content. Status: {}. Response: {}",
                        status,
                        &response_text[..response_text.len().min(500)]
                    )
                })?;

            return Ok(text.trim().trim_matches('"').to_string());
        }

        anyhow::bail!(
            "Google API rate limit exceeded after {} retries",
            MAX_RETRIES
        )
    }

    async fn translate_openai(
        &self,
        text: &str,
        target_lang: &str,
        context: Option<&str>,
    ) -> Result<String> {
        let prompt = build_single_translation_prompt(text, target_lang, context);
        let result = self.call_openai_api(&prompt).await?;
        Ok(result.trim().trim_matches('"').to_string())
    }

    async fn translate_batch_openai(
        &self,
        texts_with_ids: &[(u32, String)],
        target_lang: &str,
        context: Option<&str>,
    ) -> Result<HashMap<u32, String>> {
        let prompt = build_batch_translation_prompt(texts_with_ids, target_lang, context);
        let result_text = self.call_openai_api(&prompt).await?;

        let translations = parse_json_translations(&result_text, texts_with_ids.len())?;
        Ok(translations)
    }

    async fn translate_with_context_openai(
        &self,
        text: &str,
        target_lang: &str,
        title_context: Option<&str>,
        surrounding_context: Option<&str>,
    ) -> Result<String> {
        let prompt = build_context_enhanced_translation_prompt(
            text,
            target_lang,
            title_context,
            surrounding_context,
        );
        let result = self.call_openai_api(&prompt).await?;
        Ok(result.trim().trim_matches('"').to_string())
    }

    async fn call_openai_api(&self, prompt: &str) -> Result<String> {
        #[derive(Serialize, Deserialize)]
        struct Message {
            role: String,
            content: String,
        }

        #[derive(Serialize)]
        struct Request {
            model: String,
            messages: Vec<Message>,
            temperature: f32,
        }

        #[derive(Deserialize)]
        struct Choice {
            message: Message,
        }

        #[derive(Deserialize)]
        struct ApiError {
            message: Option<String>,
            error: Option<String>,
        }

        #[derive(Deserialize)]
        struct ResponseWithError {
            choices: Option<Vec<Choice>>,
            error: Option<ApiError>,
        }

        let url = format!(
            "{}/chat/completions",
            self.config.base_url.trim_end_matches('/')
        );

        let request = Request {
            model: self.config.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            temperature: 0.3,
        };

        const MAX_RETRIES: u32 = 3;

        for attempt in 0..=MAX_RETRIES {
            let mut req_builder = self.client.post(&url).json(&request);

            if let Some(api_key) = &self.config.api_key {
                req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
            }

            if self.config.api_type == ApiType::OpenRouter {
                req_builder = req_builder
                    .header("HTTP-Referer", "https://srt-tools.app")
                    .header("X-Title", "SRT Tools");
            }

            let http_response = req_builder.send().await?;
            let status = http_response.status();
            let response_text = http_response.text().await?;

            if let Some(delay) = retry_backoff(status, &response_text, attempt, MAX_RETRIES) {
                sleep(delay).await;
                continue;
            }

            if !status.is_success() {
                eprintln!(
                    "[srt-translate] API error response (status {}): {}",
                    status,
                    &response_text[..response_text.len().min(500)]
                );
            }

            let response: ResponseWithError =
                serde_json::from_str(&response_text).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to parse API response (status {}): {}. Raw: {}",
                        status,
                        e,
                        &response_text[..response_text.len().min(300)]
                    )
                })?;

            if let Some(ref api_error) = response.error {
                let error_msg = api_error
                    .message
                    .as_deref()
                    .or(api_error.error.as_deref())
                    .unwrap_or("Unknown API error");
                anyhow::bail!("API error: {}", error_msg);
            }

            let choices = response.choices.ok_or_else(|| {
                anyhow::anyhow!(
                    "API response missing 'choices'. Status: {}. Response: {}",
                    status,
                    &response_text[..response_text.len().min(300)]
                )
            })?;

            return Ok(choices
                .first()
                .map(|c| c.message.content.trim().to_string())
                .unwrap_or_default());
        }

        anyhow::bail!("API rate limit exceeded after {} retries", MAX_RETRIES)
    }
}

fn retry_backoff(
    status: reqwest::StatusCode,
    body: &str,
    attempt: u32,
    max_retries: u32,
) -> Option<Duration> {
    if attempt >= max_retries {
        return None;
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let delay = parse_retry_delay(body);
        eprintln!(
            "[srt-translate] Rate limited (429), retrying in {:.0}s (attempt {}/{})...",
            delay.as_secs_f64(),
            attempt + 1,
            max_retries
        );
        return Some(delay);
    }
    if status.is_server_error() {
        let delay = Duration::from_secs(2_u64.pow(attempt) * 5);
        eprintln!(
            "[srt-translate] Server error ({}), retrying in {:.0}s (attempt {}/{})...",
            status,
            delay.as_secs_f64(),
            attempt + 1,
            max_retries
        );
        return Some(delay);
    }
    None
}

fn parse_retry_delay(response_body: &str) -> Duration {
    if let Some(pos) = response_body.find("retry in ") {
        let after = &response_body[pos + 9..];

        let num_str: String = after
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        if let Ok(secs) = num_str.parse::<f64>() {
            let clamped = secs.clamp(1.0, 120.0);

            return Duration::from_secs_f64(clamped + 2.0);
        }
    }

    Duration::from_secs(60)
}

#[derive(Deserialize, Debug)]
struct TranslationItem {
    id: u32,
    text: String,
}

fn parse_json_translations(response: &str, expected_count: usize) -> Result<HashMap<u32, String>> {
    let cleaned = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let json_start = cleaned.find('[');
    let json_end = cleaned.rfind(']');

    let json_str = match (json_start, json_end) {
        (Some(start), Some(end)) if end > start => &cleaned[start..=end],
        _ => cleaned,
    };

    match serde_json::from_str::<Vec<TranslationItem>>(json_str) {
        Ok(items) => {
            let translations: HashMap<u32, String> =
                items.into_iter().map(|item| (item.id, item.text)).collect();

            if translations.len() != expected_count {
                anyhow::bail!(
                    "Batch translation incomplete: expected {} translations, got {}",
                    expected_count,
                    translations.len()
                );
            }

            Ok(translations)
        }
        Err(e) => {
            if let Some(translations) = try_legacy_parsing(cleaned, expected_count) {
                return Ok(translations);
            }

            anyhow::bail!(
                "Failed to parse JSON response: {}. Response was: {}",
                e,
                &response[..response.len().min(500)]
            )
        }
    }
}

fn try_legacy_parsing(text: &str, expected_count: usize) -> Option<HashMap<u32, String>> {
    let mut translations = HashMap::new();
    let mut current_id: Option<u32> = None;
    let mut current_translation = String::new();

    for line in text.lines() {
        let line_lower = line.to_lowercase();

        if line_lower.starts_with("id:") || line_lower.contains("id:") {
            if let Some(id) = current_id {
                translations.insert(id, current_translation.trim().to_string());
            }

            if let Some((id_part, trans_part)) = line.split_once('|') {
                let id_str: String = id_part.chars().filter(|c| c.is_ascii_digit()).collect();
                if let Ok(id) = id_str.parse::<u32>() {
                    current_id = Some(id);

                    let trans = trans_part
                        .trim()
                        .trim_start_matches(|c: char| !c.is_alphabetic() || c.is_ascii_uppercase())
                        .trim_start_matches("TRANSLATION:")
                        .trim_start_matches("translation:")
                        .trim_start_matches("Translation:")
                        .trim();
                    current_translation = trans.to_string();
                }
            }
        } else if current_id.is_some() && !line.trim().is_empty() {
            if !current_translation.is_empty() {
                current_translation.push('\n');
            }
            current_translation.push_str(line);
        }
    }

    if let Some(id) = current_id {
        translations.insert(id, current_translation.trim().to_string());
    }

    if translations.len() >= expected_count {
        Some(translations)
    } else {
        None
    }
}
