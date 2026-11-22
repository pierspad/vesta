use crate::diagnostics::{
    ProviderError, ProviderFailureKind, TranslationDiagnostic, provider_error,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

pub(crate) const DEFAULT_GOOGLE_MODEL: &str = "gemini-3.8-flash";

use crate::prompts::{
    build_batch_translation_prompt, build_context_enhanced_translation_prompt,
    build_single_translation_prompt,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ApiType {
    Local,

    Google,

    Groq,

    OpenRouter,

    Anthropic,
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
            model: DEFAULT_GOOGLE_MODEL.to_string(),
            api_key: None,
            api_type: ApiType::Google,
        }
    }
}

#[derive(Clone)]
pub struct Translator {
    config: TranslatorConfig,
    client: reqwest::Client,
    diagnostics: Option<Arc<dyn Fn(TranslationDiagnostic) + Send + Sync>>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct GeminiPart {
    pub text: Option<String>,
    #[serde(default)]
    pub thought: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct GeminiContent {
    pub parts: Option<Vec<GeminiPart>>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct GeminiCandidate {
    pub content: Option<GeminiContent>,
    #[serde(rename = "finishReason")]
    pub finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct GeminiError {
    pub message: Option<String>,
    pub code: Option<i32>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct GeminiResponse {
    pub candidates: Option<Vec<GeminiCandidate>>,
    pub error: Option<GeminiError>,
}

pub(crate) fn extract_gemini_text(
    response: GeminiResponse,
    status: u16,
) -> Result<String, ProviderError> {
    let first_candidate = response.candidates.and_then(|c| c.into_iter().next());

    let (finish_reason, text) = if let Some(candidate) = first_candidate {
        let finish_reason = candidate.finish_reason;
        let parts = candidate.content.and_then(|c| c.parts).unwrap_or_default();
        let non_thought_text = parts
            .into_iter()
            .filter(|p| p.thought != Some(true))
            .filter_map(|p| p.text)
            .collect::<Vec<_>>()
            .join("");
        (finish_reason, non_thought_text)
    } else {
        (None, String::new())
    };

    let trimmed = text.trim();
    if trimmed.is_empty() {
        if let Some(reason) = finish_reason {
            let upper = reason.to_ascii_uppercase();
            if upper.contains("SAFETY")
                || upper.contains("PROHIBITED_CONTENT")
                || upper.contains("BLOCKLIST")
                || upper.contains("RECITATION")
            {
                return Err(ProviderError {
                    provider: "Google",
                    status,
                    kind: ProviderFailureKind::InvalidRequest,
                });
            }
        }
        return Err(ProviderError {
            provider: "Google",
            status,
            kind: ProviderFailureKind::EmptyResponse,
        });
    }

    Ok(trimmed.to_string())
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct OpenAiMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum OpenAiContentPart {
    Text { text: String },
    ImageUrl { image_url: OpenAiImageUrl },
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub(crate) struct OpenAiImageUrl {
    pub url: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub(crate) enum OpenAiRequestContent {
    Text(String),
    Parts(Vec<OpenAiContentPart>),
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub(crate) struct OpenAiRequestMessage {
    pub role: String,
    pub content: OpenAiRequestContent,
}

#[derive(Serialize, Debug)]
pub(crate) struct OpenAiRequest {
    pub model: String,
    pub messages: Vec<OpenAiRequestMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

pub(crate) fn supports_custom_temperature(model: &str) -> bool {
    let model = model.to_ascii_lowercase();
    let name = match model.rfind('/') {
        Some(pos) => &model[pos + 1..],
        None => &model,
    };
    let modern_gpt = name == "gpt-5"
        || name.starts_with("gpt-5-")
        || name.starts_with("gpt-5.6")
        || name
            .strip_prefix("gpt-")
            .and_then(|version| version.split(['.', '-']).next())
            .and_then(|major| major.parse::<u32>().ok())
            .is_some_and(|major| major >= 6);
    !(modern_gpt || name.starts_with("o1") || name.starts_with("o3") || name.starts_with("o4"))
}

pub(crate) fn build_openai_request_with_image(
    model: &str,
    prompt: &str,
    image: Option<(&str, &str)>,
) -> OpenAiRequest {
    let temperature = if supports_custom_temperature(model) {
        Some(0.3)
    } else {
        None
    };
    let content = match image {
        Some((b64, mime)) => OpenAiRequestContent::Parts(vec![
            OpenAiContentPart::Text {
                text: prompt.to_string(),
            },
            OpenAiContentPart::ImageUrl {
                image_url: OpenAiImageUrl {
                    url: format!("data:{mime};base64,{b64}"),
                },
            },
        ]),
        None => OpenAiRequestContent::Text(prompt.to_string()),
    };
    OpenAiRequest {
        model: model.to_string(),
        messages: vec![OpenAiRequestMessage {
            role: "user".to_string(),
            content,
        }],
        temperature,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn build_openai_request(model: &str, prompt: &str) -> OpenAiRequest {
    build_openai_request_with_image(model, prompt, None)
}

#[derive(Serialize, Debug)]
pub(crate) struct GeminiInlineData {
    pub mime_type: String,
    pub data: String,
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub(crate) enum GeminiRequestPart {
    Text { text: String },
    InlineData { inline_data: GeminiInlineData },
}

#[derive(Serialize, Debug)]
pub(crate) struct GeminiRequestContent {
    pub parts: Vec<GeminiRequestPart>,
}

#[derive(Serialize, Debug)]
pub(crate) struct GeminiGenerationConfig {
    pub temperature: f32,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GeminiRequest {
    pub contents: Vec<GeminiRequestContent>,
    pub generation_config: GeminiGenerationConfig,
}

pub(crate) fn build_gemini_request_with_image(
    prompt: &str,
    image: Option<(&str, &str)>,
) -> GeminiRequest {
    let mut parts = vec![GeminiRequestPart::Text {
        text: prompt.to_string(),
    }];
    if let Some((b64, mime)) = image {
        parts.push(GeminiRequestPart::InlineData {
            inline_data: GeminiInlineData {
                mime_type: mime.to_string(),
                data: b64.to_string(),
            },
        });
    }
    GeminiRequest {
        contents: vec![GeminiRequestContent { parts }],
        generation_config: GeminiGenerationConfig { temperature: 0.3 },
    }
}

#[derive(Serialize, Debug)]
pub(crate) struct AnthropicImageSource {
    #[serde(rename = "type")]
    pub source_type: &'static str,
    pub media_type: String,
    pub data: String,
}

#[derive(Serialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum AnthropicContentPart {
    Image { source: AnthropicImageSource },
    Text { text: String },
}

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub(crate) enum AnthropicMessageContent {
    Text(String),
    Parts(Vec<AnthropicContentPart>),
}

#[derive(Serialize, Debug)]
pub(crate) struct AnthropicMessage {
    pub role: String,
    pub content: AnthropicMessageContent,
}

#[derive(Serialize, Debug)]
pub(crate) struct AnthropicRequest {
    pub model: String,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    pub messages: Vec<AnthropicMessage>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct AnthropicError {
    pub message: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct AnthropicContentBlock {
    #[serde(rename = "type")]
    pub block_type: Option<String>,
    pub text: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct AnthropicResponse {
    pub content: Option<Vec<AnthropicContentBlock>>,
    pub error: Option<AnthropicError>,
}

fn supports_anthropic_temperature(model: &str) -> bool {
    let model = model.to_ascii_lowercase();
    let name = model.rsplit('/').next().unwrap_or(&model);
    let Some(version) = name.strip_prefix("claude-") else {
        return true;
    };
    let mut numbers = version
        .split('-')
        .filter(|part| part.len() <= 2)
        .filter_map(|part| part.parse::<u32>().ok());
    let major = numbers.next().unwrap_or(0);
    let minor = numbers.next().unwrap_or(0);
    major < 4 || (major == 4 && minor < 7)
}

pub(crate) fn build_anthropic_request_with_image(
    model: &str,
    prompt: &str,
    image: Option<(&str, &str)>,
) -> AnthropicRequest {
    let content = match image {
        Some((b64, mime)) => AnthropicMessageContent::Parts(vec![
            AnthropicContentPart::Image {
                source: AnthropicImageSource {
                    source_type: "base64",
                    media_type: mime.to_string(),
                    data: b64.to_string(),
                },
            },
            AnthropicContentPart::Text {
                text: prompt.to_string(),
            },
        ]),
        None => AnthropicMessageContent::Text(prompt.to_string()),
    };
    AnthropicRequest {
        model: model.to_string(),
        max_tokens: 4096,
        temperature: supports_anthropic_temperature(model).then_some(0.3),
        messages: vec![AnthropicMessage {
            role: "user".to_string(),
            content,
        }],
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn build_anthropic_request(model: &str, prompt: &str) -> AnthropicRequest {
    build_anthropic_request_with_image(model, prompt, None)
}

pub(crate) fn extract_anthropic_text(
    response: AnthropicResponse,
    status: u16,
) -> Result<String, ProviderError> {
    if let Some(ref error) = response.error {
        let err_msg = error.message.as_deref().unwrap_or("Unknown API error");
        return Err(provider_error("Anthropic", status, err_msg));
    }

    let text = response
        .content
        .unwrap_or_default()
        .into_iter()
        .filter(|block| block.block_type.as_deref() != Some("thinking"))
        .filter_map(|block| block.text)
        .collect::<Vec<_>>()
        .join("");

    let trimmed = text.trim();
    if trimmed.trim_matches('"').trim().is_empty() {
        return Err(ProviderError {
            provider: "Anthropic",
            status,
            kind: ProviderFailureKind::EmptyResponse,
        });
    }

    Ok(trimmed.to_string())
}

fn api_redirect_policy() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= 10 {
            attempt.error("Too many API redirects")
        } else if attempt
            .previous()
            .first()
            .is_some_and(|original| original.origin() == attempt.url().origin())
        {
            attempt.follow()
        } else {
            attempt.stop()
        }
    })
}

impl Translator {
    /// Builds a translator and reports HTTP client configuration errors.
    pub fn try_new(config: TranslatorConfig) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .redirect(api_redirect_policy())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()?;
        Ok(Self {
            config,
            client,
            diagnostics: None,
        })
    }

    /// Builds a translator, falling back to reqwest defaults if custom timeout
    /// configuration fails. Prefer [`Self::try_new`] when errors can be handled.
    pub fn new(config: TranslatorConfig) -> Self {
        match Self::try_new(config.clone()) {
            Ok(translator) => translator,
            Err(error) => {
                eprintln!(
                    "Warning: could not configure translation HTTP client timeouts ({error}); using reqwest defaults"
                );
                Self {
                    config,
                    client: reqwest::Client::builder()
                        .redirect(api_redirect_policy())
                        .build()
                        .expect("Failed to initialize default translation HTTP client"),
                    diagnostics: None,
                }
            }
        }
    }

    /// Attach a fast, nonblocking observer. Cloned translators retain it.
    /// No request bodies, URLs or keys are included in diagnostic events.
    pub fn with_diagnostics(
        mut self,
        observer: impl Fn(TranslationDiagnostic) + Send + Sync + 'static,
    ) -> Self {
        self.diagnostics = Some(Arc::new(observer));
        self
    }

    fn report_retry(
        &self,
        status: reqwest::StatusCode,
        attempt: u32,
        max_retries: u32,
        delay: Duration,
    ) {
        if let Some(observer) = &self.diagnostics {
            observer(TranslationDiagnostic {
                status: status.as_u16(),
                attempt: attempt + 1,
                max_retries,
                retry_in_ms: delay.as_millis() as u64,
            });
        }
    }

    pub async fn translate(
        &self,
        text: &str,
        target_lang: &str,
        context: Option<&str>,
    ) -> Result<String> {
        let prompt = build_single_translation_prompt(text, target_lang, context);
        self.translation_response(&prompt).await
    }

    pub async fn translate_with_context(
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
        self.translation_response(&prompt).await
    }

    // Translation prompts expect a bare line; strip stray surrounding quotes consistently across providers.
    async fn translation_response(&self, prompt: &str) -> Result<String> {
        let text = self.generate_response(prompt).await?;
        Ok(text.trim().trim_matches('"').to_string())
    }

    pub async fn translate_batch(
        &self,
        texts_with_ids: &[(u32, String)],
        target_lang: &str,
        context: Option<&str>,
    ) -> Result<HashMap<u32, String>> {
        let prompt = build_batch_translation_prompt(texts_with_ids, target_lang, context);
        let response = self.generate_response(&prompt).await?;
        let translations = parse_json_translations(&response, texts_with_ids.len())?;
        anyhow::ensure!(
            texts_with_ids
                .iter()
                .all(|(id, _)| translations.contains_key(id)),
            "Batch translation returned IDs that do not match the requested subtitles"
        );
        Ok(translations)
    }

    pub async fn generate_response(&self, prompt: &str) -> Result<String> {
        self.generate_response_with_image(prompt, None).await
    }

    /// Sends a multimodal prompt with a base64-encoded image to the configured provider.
    pub async fn generate_vision_response(
        &self,
        prompt: &str,
        image_base64: &str,
        mime_type: &str,
    ) -> Result<String> {
        anyhow::ensure!(
            !image_base64.trim().is_empty(),
            "Image base64 payload must not be empty"
        );
        anyhow::ensure!(
            !mime_type.trim().is_empty(),
            "Image MIME type must not be empty"
        );
        self.generate_response_with_image(prompt, Some((image_base64, mime_type)))
            .await
    }

    async fn generate_response_with_image(
        &self,
        prompt: &str,
        image: Option<(&str, &str)>,
    ) -> Result<String> {
        match self.config.api_type {
            ApiType::Google => self.call_google_api_with_image(prompt, image).await,
            ApiType::Anthropic => self.call_anthropic_api_with_image(prompt, image).await,
            ApiType::Local | ApiType::OpenRouter | ApiType::Groq => {
                self.call_openai_api_with_image(prompt, image).await
            }
        }
    }

    async fn request_with_retries(
        &self,
        provider: &'static str,
        request: impl Fn() -> reqwest::RequestBuilder,
    ) -> Result<(reqwest::StatusCode, String)> {
        const MAX_RETRIES: u32 = 3;
        for attempt in 0..=MAX_RETRIES {
            let response = request()
                .send()
                .await
                .map_err(reqwest::Error::without_url)
                .with_context(|| format!("{provider} API transport failed"))?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            let text = response
                .text()
                .await
                .map_err(reqwest::Error::without_url)
                .with_context(|| format!("{provider} API response read failed"))?;
            if let Some(delay) =
                retry_backoff(status, &text, retry_after.as_deref(), attempt, MAX_RETRIES)
            {
                self.report_retry(status, attempt, MAX_RETRIES, delay);
                sleep(delay).await;
                continue;
            }
            if !status.is_success() {
                return Err(provider_error(provider, status.as_u16(), &text).into());
            }
            return Ok((status, text));
        }
        unreachable!("the final attempt returns a response or an error")
    }

    async fn call_google_api_with_image(
        &self,
        prompt: &str,
        image: Option<(&str, &str)>,
    ) -> Result<String> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Google API key is required"))?;

        let url = format!(
            "{}/models/{}:generateContent",
            self.config.base_url.trim_end_matches('/'),
            self.config.model
        );

        let request = build_gemini_request_with_image(prompt, image);

        let (status, response_text) = self
            .request_with_retries("Google", || {
                self.client
                    .post(&url)
                    .header("x-goog-api-key", api_key)
                    .header("Content-Type", "application/json")
                    .json(&request)
            })
            .await?;

        let response: GeminiResponse =
            serde_json::from_str(&response_text).map_err(|_| ProviderError {
                provider: "Google",
                status: status.as_u16(),
                kind: ProviderFailureKind::InvalidResponse,
            })?;

        if let Some(ref error) = response.error {
            if error.code == Some(429) {
                return Err(ProviderError {
                    provider: "Google",
                    status: status.as_u16(),
                    kind: ProviderFailureKind::RateLimit,
                }
                .into());
            }
            return Err(provider_error(
                "Google",
                status.as_u16(),
                error.message.as_deref().unwrap_or(""),
            )
            .into());
        }

        let text = extract_gemini_text(response, status.as_u16())?;
        Ok(text)
    }

    async fn call_openai_api_with_image(
        &self,
        prompt: &str,
        image: Option<(&str, &str)>,
    ) -> Result<String> {
        #[derive(Deserialize)]
        struct Choice {
            message: OpenAiMessage,
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

        let request = build_openai_request_with_image(&self.config.model, prompt, image);

        let (status, response_text) = self
            .request_with_retries("OpenAI-compatible", || {
                let mut builder = self.client.post(&url).json(&request);
                if let Some(api_key) = &self.config.api_key {
                    builder = builder.bearer_auth(api_key);
                }
                if self.config.api_type == ApiType::OpenRouter {
                    builder = builder
                        .header("HTTP-Referer", "https://srt-tools.app")
                        .header("X-Title", "SRT Tools");
                }
                builder
            })
            .await?;

        let response: ResponseWithError =
            serde_json::from_str(&response_text).map_err(|_| ProviderError {
                provider: "OpenAI-compatible",
                status: status.as_u16(),
                kind: ProviderFailureKind::InvalidResponse,
            })?;

        if let Some(ref api_error) = response.error {
            let error_msg = api_error
                .message
                .as_deref()
                .or(api_error.error.as_deref())
                .unwrap_or("Unknown API error");
            return Err(provider_error("OpenAI-compatible", status.as_u16(), error_msg).into());
        }

        let choices = response.choices.ok_or_else(|| ProviderError {
            provider: "OpenAI-compatible",
            status: status.as_u16(),
            kind: ProviderFailureKind::EmptyResponse,
        })?;

        let text = choices
            .first()
            .map(|c| c.message.content.trim().to_string())
            .unwrap_or_default();
        if text.trim_matches('"').trim().is_empty() {
            return Err(ProviderError {
                provider: "OpenAI-compatible",
                status: status.as_u16(),
                kind: ProviderFailureKind::EmptyResponse,
            }
            .into());
        }
        Ok(text)
    }

    async fn call_anthropic_api_with_image(
        &self,
        prompt: &str,
        image: Option<(&str, &str)>,
    ) -> Result<String> {
        let api_key = self
            .config
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Anthropic API key is required"))?;

        let url = format!("{}/messages", self.config.base_url.trim_end_matches('/'));
        let request = build_anthropic_request_with_image(&self.config.model, prompt, image);

        let (status, response_text) = self
            .request_with_retries("Anthropic", || {
                self.client
                    .post(&url)
                    .header("x-api-key", api_key)
                    .header("anthropic-version", "2023-06-01")
                    .header("Content-Type", "application/json")
                    .json(&request)
            })
            .await?;

        let response: AnthropicResponse =
            serde_json::from_str(&response_text).map_err(|_| ProviderError {
                provider: "Anthropic",
                status: status.as_u16(),
                kind: ProviderFailureKind::InvalidResponse,
            })?;

        extract_anthropic_text(response, status.as_u16()).map_err(Into::into)
    }
}

fn retry_backoff(
    status: reqwest::StatusCode,
    body: &str,
    retry_after: Option<&str>,
    attempt: u32,
    max_retries: u32,
) -> Option<Duration> {
    if attempt >= max_retries {
        return None;
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS && permanent_quota_error(body) {
        return None;
    }
    if (status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error())
        && let Some(delay) =
            retry_after.and_then(|value| parse_retry_after(value, std::time::SystemTime::now()))
    {
        // Fail over rather than retrying before a provider's longer deadline.
        return (delay <= Duration::from_secs(120)).then_some(delay);
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let delay = parse_retry_delay(body);
        return Some(delay);
    }
    if status.is_server_error() {
        let delay = Duration::from_secs(2_u64.pow(attempt) * 5);
        return Some(delay);
    }
    None
}

fn permanent_quota_error(body: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return false;
    };
    ["/error/code", "/error/type", "/error/details/error_code"]
        .into_iter()
        .any(|path| {
            matches!(
                value.pointer(path).and_then(serde_json::Value::as_str),
                Some(
                    "insufficient_quota"
                        | "credit_balance_exhausted"
                        | "organization_spend_limit_exceeded"
                        | "project_spend_limit_exceeded"
                        | "organization_usage_limit_exceeded"
                        | "enforced_spend_limit_reached"
                )
            )
        })
}

fn parse_retry_after(value: &str, now: std::time::SystemTime) -> Option<Duration> {
    let value = value.trim();
    let delay = if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) {
        Duration::from_secs(value.parse().ok()?)
    } else {
        httpdate::parse_http_date(value)
            .ok()?
            .duration_since(now)
            .unwrap_or_default()
    };
    Some(delay.max(Duration::from_secs(1)))
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

fn extract_json_array_candidates(cleaned: &str) -> Vec<&str> {
    let mut candidates = Vec::new();
    let bytes = cleaned.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            let mut depth = 0usize;
            let mut in_string = false;
            let mut escaped = false;
            let mut j = i;
            while j < bytes.len() {
                let b = bytes[j];
                if in_string {
                    if escaped {
                        escaped = false;
                    } else if b == b'\\' {
                        escaped = true;
                    } else if b == b'"' {
                        in_string = false;
                    }
                } else {
                    match b {
                        b'"' => in_string = true,
                        b'[' => depth += 1,
                        b']' => {
                            depth = depth.saturating_sub(1);
                            if depth == 0 {
                                candidates.push(&cleaned[i..=j]);
                                i = j;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                j += 1;
            }
        }
        i += 1;
    }
    candidates
}

fn parse_json_translations(response: &str, expected_count: usize) -> Result<HashMap<u32, String>> {
    let cleaned = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let candidates = extract_json_array_candidates(cleaned);
    let mut parsed_items: Option<Vec<TranslationItem>> = None;
    let mut last_err = None;
    for candidate in candidates.into_iter().chain(std::iter::once(cleaned)) {
        match serde_json::from_str::<Vec<TranslationItem>>(candidate) {
            Ok(items) => {
                parsed_items = Some(items);
                break;
            }
            Err(e) => {
                if last_err.is_none() {
                    last_err = Some(e);
                }
            }
        }
    }

    match parsed_items {
        Some(items) => {
            let mut translations = HashMap::new();
            for item in items {
                anyhow::ensure!(
                    !item.text.trim().is_empty(),
                    "Empty translation for subtitle {}",
                    item.id
                );
                anyhow::ensure!(
                    translations.insert(item.id, item.text).is_none(),
                    "Duplicate translation ID {}",
                    item.id
                );
            }

            if translations.len() != expected_count {
                anyhow::bail!(
                    "Batch translation incomplete: expected {} translations, got {}",
                    expected_count,
                    translations.len()
                );
            }

            Ok(translations)
        }
        None => {
            if let Some(translations) = try_legacy_parsing(cleaned, expected_count) {
                anyhow::ensure!(
                    translations.values().all(|text| !text.trim().is_empty()),
                    "Batch translation contains empty text"
                );
                return Ok(translations);
            }

            if let Some(e) = last_err {
                anyhow::bail!(
                    "Failed to parse translation JSON response at line {}, column {}",
                    e.line(),
                    e.column()
                )
            } else {
                anyhow::bail!("Failed to parse translation JSON response")
            }
        }
    }
}

fn try_legacy_parsing(text: &str, expected_count: usize) -> Option<HashMap<u32, String>> {
    let mut translations = HashMap::new();
    let mut current_id: Option<u32> = None;
    let mut current_translation = String::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        let is_legacy_header = trimmed
            .get(..3)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("id:"))
            || (trimmed.len() > 3
                && trimmed.as_bytes()[0..2].eq_ignore_ascii_case(b"id")
                && trimmed[2..].trim_start().starts_with(':'));

        if is_legacy_header && let Some((id_part, trans_part)) = trimmed.split_once('|') {
            if let Some(id) = current_id
                && translations
                    .insert(id, current_translation.trim().to_string())
                    .is_some()
            {
                return None;
            }

            let id_str: String = id_part.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(id) = id_str.parse::<u32>() {
                current_id = Some(id);

                let mut trans = trans_part.trim();
                if let Some((_, after_colon)) = trans.split_once(':')
                    && (trans.to_lowercase().starts_with("translation:")
                        || trans.to_lowercase().starts_with("trans:"))
                {
                    trans = after_colon.trim();
                }
                current_translation = trans.to_string();
            }
        } else if current_id.is_some() && !line.trim().is_empty() {
            if !current_translation.is_empty() {
                current_translation.push('\n');
            }
            current_translation.push_str(line);
        }
    }

    if let Some(id) = current_id
        && translations
            .insert(id, current_translation.trim().to_string())
            .is_some()
    {
        return None;
    }

    if translations.len() == expected_count {
        Some(translations)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_model_requests_omit_unsupported_sampling_parameters() {
        for model in [
            "gpt-5",
            "gpt-5-mini",
            "gpt-5.6-sol",
            "openai/gpt-6-luna",
            "gpt-6.2-sol",
        ] {
            let json = serde_json::to_value(build_openai_request(model, "prompt")).unwrap();
            assert!(json.get("temperature").is_none(), "{model}");
        }
        for model in [
            "claude-opus-4-7",
            "claude-opus-4-7-20260416",
            "claude-sonnet-5-5",
            "claude-haiku-5-5",
            "claude-fable-5-2",
        ] {
            let json = serde_json::to_value(build_anthropic_request(model, "prompt")).unwrap();
            assert!(json.get("temperature").is_none(), "{model}");
            assert_eq!(json["max_tokens"], 4096);
        }
        for model in [
            "claude-sonnet-4-6",
            "claude-haiku-4-5-20251001",
            "claude-3-5-haiku-20241022",
            "claude-opus-4-20250514",
        ] {
            let json = serde_json::to_value(build_anthropic_request(model, "prompt")).unwrap();
            assert!(
                (json["temperature"].as_f64().unwrap() - 0.3).abs() < 1e-6,
                "{model}"
            );
        }
    }

    #[test]
    fn permanent_billing_errors_do_not_schedule_retries() {
        use reqwest::StatusCode;
        for body in [
            r#"{"error":{"code":"credit_balance_exhausted"}}"#,
            r#"{"error":{"type":"insufficient_quota"}}"#,
            r#"{"error":{"code":"organization_spend_limit_exceeded"}}"#,
            r#"{"error":{"code":"project_spend_limit_exceeded"}}"#,
            r#"{"error":{"code":"organization_usage_limit_exceeded"}}"#,
            r#"{"error":{"details":{"error_code":"enforced_spend_limit_reached"}}}"#,
        ] {
            assert_eq!(
                retry_backoff(StatusCode::TOO_MANY_REQUESTS, body, Some("1"), 0, 3),
                None
            );
        }
        assert!(
            retry_backoff(
                StatusCode::TOO_MANY_REQUESTS,
                r#"{"error":{"type":"rate_limit_error"}}"#,
                None,
                0,
                3
            )
            .is_some()
        );
    }

    #[test]
    fn retry_after_headers_take_precedence_and_do_not_force_early_retries() {
        use reqwest::StatusCode;
        assert_eq!(
            retry_backoff(
                StatusCode::TOO_MANY_REQUESTS,
                "retry in 60s",
                Some("2"),
                0,
                3
            ),
            Some(Duration::from_secs(2))
        );
        assert_eq!(
            retry_backoff(StatusCode::SERVICE_UNAVAILABLE, "", Some("0"), 0, 3),
            Some(Duration::from_secs(1))
        );
        assert_eq!(
            retry_backoff(StatusCode::TOO_MANY_REQUESTS, "", Some("600"), 0, 3),
            None
        );
        assert_eq!(
            retry_backoff(StatusCode::UNAUTHORIZED, "", Some("2"), 0, 3),
            None
        );
        assert_eq!(
            retry_backoff(StatusCode::TOO_MANY_REQUESTS, "", Some("2"), 3, 3),
            None
        );
        assert_eq!(
            retry_backoff(
                StatusCode::TOO_MANY_REQUESTS,
                "retry in 2s",
                Some("invalid"),
                0,
                3
            ),
            Some(Duration::from_secs(4))
        );
        let now = std::time::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let future = httpdate::fmt_http_date(now + Duration::from_secs(30));
        let past = httpdate::fmt_http_date(now - Duration::from_secs(30));
        assert_eq!(
            parse_retry_after(&future, now),
            Some(Duration::from_secs(30))
        );
        assert_eq!(parse_retry_after(&past, now), Some(Duration::from_secs(1)));
        for invalid in ["", "-1", "NaN", "inf", "1.5"] {
            assert_eq!(parse_retry_after(invalid, now), None);
        }
    }

    #[test]
    fn fallible_constructor_keeps_translator_configuration() {
        let config = TranslatorConfig::default();
        let expected_model = config.model.clone();
        let translator = Translator::try_new(config).unwrap();
        assert_eq!(translator.config.model, expected_model);
        assert!(translator.diagnostics.is_none());
    }

    #[test]
    fn test_parse_json_translations_clean_json() {
        let json = r#"[{"id":1,"text":"Ciao mondo"},{"id":2,"text":"Come va?"}]"#;
        let map = parse_json_translations(json, 2).unwrap();
        assert_eq!(map.get(&1).unwrap(), "Ciao mondo");
        assert_eq!(map.get(&2).unwrap(), "Come va?");
    }

    #[test]
    fn test_parse_json_translations_markdown_fenced() {
        let text = r#"Here is the translation:
```json
[
  {"id": 10, "text": "Bonjour"},
  {"id": 20, "text": "Au revoir"}
]
```
Hope this helps!
"#;
        let map = parse_json_translations(text, 2).unwrap();
        assert_eq!(map.get(&10).unwrap(), "Bonjour");
        assert_eq!(map.get(&20).unwrap(), "Au revoir");
    }

    #[test]
    fn test_parse_json_translations_legacy_fallback() {
        let text = "ID: 1 | TRANSLATION: First line\nID: 2 | TRANSLATION: Second line";
        let map = parse_json_translations(text, 2).unwrap();
        assert_eq!(map.get(&1).unwrap(), "First line");
        assert_eq!(map.get(&2).unwrap(), "Second line");
    }

    #[test]
    fn test_parse_json_translations_mismatched_count_errors() {
        let json = r#"[{"id":1,"text":"Only one item"}]"#;
        assert!(parse_json_translations(json, 2).is_err());
    }

    #[test]
    fn test_gemini_thinking_model_filters_thought_parts() {
        let json = r#"{
            "candidates": [
                {
                    "content": {
                        "parts": [
                            {
                                "text": "Analyzing the nuance of the source phrase...\nI should output only the Italian translation.",
                                "thought": true
                            },
                            {
                                "text": "Ciao mondo"
                            }
                        ]
                    },
                    "finishReason": "STOP"
                }
            ]
        }"#;

        let response: GeminiResponse = serde_json::from_str(json).unwrap();
        let extracted = extract_gemini_text(response, 200).unwrap();
        assert_eq!(extracted, "Ciao mondo");
    }

    #[test]
    fn test_gemini_safety_block_without_text_reports_invalid_request() {
        let json = r#"{
            "candidates": [
                {
                    "content": {
                        "parts": [
                            {
                                "text": "Thinking...",
                                "thought": true
                            }
                        ]
                    },
                    "finishReason": "SAFETY"
                }
            ]
        }"#;

        let response: GeminiResponse = serde_json::from_str(json).unwrap();
        let err = extract_gemini_text(response, 200).unwrap_err();
        assert_eq!(err.kind, ProviderFailureKind::InvalidRequest);
    }

    #[test]
    fn test_openai_reasoning_models_omit_temperature_and_standard_include() {
        for reasoning_model in [
            "o1",
            "o1-mini",
            "o1-preview",
            "o3-mini",
            "o4-mini",
            "openai/o1-mini",
            "openai/o3-mini",
            "openai/o4-mini",
            "OPENAI/O1-MINI",
        ] {
            assert!(
                !supports_custom_temperature(reasoning_model),
                "Reasoning model {reasoning_model} should not support custom temperature"
            );
            let req = build_openai_request(reasoning_model, "Translate this text");
            let json = serde_json::to_string(&req).unwrap();
            assert!(
                !json.contains("\"temperature\""),
                "Model {reasoning_model} must omit temperature from request body: {json}"
            );
        }

        for standard_model in [
            "gpt-4o-mini",
            "gpt-4.1-mini",
            "gpt-5.1",
            "openai/gpt-4o-mini",
            "deepseek/deepseek-chat",
        ] {
            assert!(
                supports_custom_temperature(standard_model),
                "Standard model {standard_model} should support custom temperature"
            );
            let req = build_openai_request(standard_model, "Translate this text");
            let json = serde_json::to_string(&req).unwrap();
            assert!(
                json.contains("\"temperature\":0.3"),
                "Standard model {standard_model} must include temperature 0.3: {json}"
            );
        }
    }

    #[test]
    fn test_anthropic_request_serialization() {
        let req = build_anthropic_request("claude-haiku-4-5", "Translate hello");
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["model"], "claude-haiku-4-5");
        assert_eq!(value["max_tokens"], 4096);
        assert!((value["temperature"].as_f64().unwrap() - 0.3).abs() < 1e-6);
        assert_eq!(
            value["messages"],
            serde_json::json!([{"role": "user", "content": "Translate hello"}])
        );
    }

    #[test]
    fn test_anthropic_extract_filters_thinking_and_handles_errors() {
        let thinking_json = r#"{
            "content": [
                {"type": "thinking", "text": "Internal reasoning that must be ignored"},
                {"type": "text", "text": "  Ciao "},
                {"type": "text", "text": "mondo  "}
            ]
        }"#;
        let response: AnthropicResponse = serde_json::from_str(thinking_json).unwrap();
        assert_eq!(extract_anthropic_text(response, 200).unwrap(), "Ciao mondo");

        for empty_json in [
            r#"{"content":[]}"#,
            r#"{"content":[{"type":"thinking","text":"Only thought"}]}"#,
            r#"{"content":[{"type":"text","text":"   "}]}"#,
            r#"{"content":[{"type":"text","text":"\"\""}]}"#,
        ] {
            let empty_resp: AnthropicResponse = serde_json::from_str(empty_json).unwrap();
            let err = extract_anthropic_text(empty_resp, 200).unwrap_err();
            assert_eq!(err.provider, "Anthropic");
            assert_eq!(err.kind, ProviderFailureKind::EmptyResponse);
        }

        let err_json = r#"{
            "error": {
                "type": "rate_limit_error",
                "message": "Rate limit exceeded"
            }
        }"#;
        let err_resp: AnthropicResponse = serde_json::from_str(err_json).unwrap();
        let err = extract_anthropic_text(err_resp, 200).unwrap_err();
        assert_eq!(err.provider, "Anthropic");
        assert_eq!(err.kind, ProviderFailureKind::RateLimit);
    }

    #[tokio::test]
    async fn test_anthropic_requires_non_empty_api_key() {
        for api_key in [None, Some("   ".to_string())] {
            let translator = Translator::new(TranslatorConfig {
                base_url: "http://127.0.0.1:1/v1".to_string(),
                model: "claude-haiku-4-5".to_string(),
                api_key,
                api_type: ApiType::Anthropic,
            });
            let err = translator.generate_response("Hello").await.unwrap_err();
            assert!(err.to_string().contains("Anthropic API key is required"));
        }
    }

    #[tokio::test]
    async fn test_anthropic_http_headers_endpoint_and_response() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let mut captured = Vec::new();
            for response_body in [
                r#"{"content":[{"type":"thinking","text":"Reasoning..."},{"type":"text","text":"Ciao mondo"}]}"#,
                r#"{"content":[{"type":"text","text":"\"Buongiorno\""}]}"#,
            ] {
                let (mut stream, _) =
                    tokio::time::timeout(Duration::from_secs(5), listener.accept())
                        .await
                        .unwrap()
                        .unwrap();

                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                let header_end = loop {
                    let n = stream.read(&mut chunk).await.unwrap();
                    assert!(n > 0, "unexpected EOF reading request");
                    buf.extend_from_slice(&chunk[..n]);
                    if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        break pos + 4;
                    }
                };
                let headers_str = String::from_utf8_lossy(&buf[..header_end]).to_string();
                let content_length = headers_str
                    .lines()
                    .find_map(|line| {
                        let (name, val) = line.split_once(':')?;
                        name.trim()
                            .eq_ignore_ascii_case("content-length")
                            .then(|| val.trim().parse::<usize>().ok())?
                    })
                    .unwrap_or(0);
                while buf.len() < header_end + content_length {
                    let n = stream.read(&mut chunk).await.unwrap();
                    assert!(n > 0, "unexpected EOF reading request body");
                    buf.extend_from_slice(&chunk[..n]);
                }
                let body_str =
                    String::from_utf8_lossy(&buf[header_end..header_end + content_length])
                        .to_string();
                captured.push((headers_str, body_str));

                let http_resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                stream.write_all(http_resp.as_bytes()).await.unwrap();
            }
            captured
        });

        let translator = Translator::new(TranslatorConfig {
            base_url: format!("http://{addr}/v1/"),
            model: "claude-haiku-4-5".to_string(),
            api_key: Some("anthropic-secret-key".to_string()),
            api_type: ApiType::Anthropic,
        });

        let raw = translator.generate_response("Hello world").await.unwrap();
        assert_eq!(raw, "Ciao mondo");

        let translated = translator
            .translate("Good morning", "Italian", None)
            .await
            .unwrap();
        assert_eq!(translated, "Buongiorno");

        let captured = server_task.await.unwrap();
        assert_eq!(captured.len(), 2);
        let (headers, body) = &captured[0];
        let headers_lower = headers.to_ascii_lowercase();
        assert!(headers.starts_with("POST /v1/messages HTTP/"));
        assert!(headers_lower.contains("x-api-key: anthropic-secret-key\r\n"));
        assert!(headers_lower.contains("anthropic-version: 2023-06-01\r\n"));
        assert!(headers_lower.contains("content-type: application/json\r\n"));

        let body_json: serde_json::Value = serde_json::from_str(body).unwrap();
        assert_eq!(body_json["model"], "claude-haiku-4-5");
        assert_eq!(body_json["max_tokens"], 4096);
        assert_eq!(body_json["messages"][0]["role"], "user");
        assert_eq!(body_json["messages"][0]["content"], "Hello world");
    }

    #[test]
    fn test_vision_request_payloads_match_provider_contracts() {
        let openai = build_openai_request_with_image(
            "gpt-4o-mini",
            "Read subtitle",
            Some(("AAAA", "image/png")),
        );
        let openai_json = serde_json::to_value(&openai).unwrap();
        assert_eq!(openai_json["model"], "gpt-4o-mini");
        assert!((openai_json["temperature"].as_f64().unwrap() - 0.3).abs() < 1e-6);
        assert_eq!(
            openai_json["messages"],
            serde_json::json!([{
                "role": "user",
                "content": [
                    {"type": "text", "text": "Read subtitle"},
                    {"type": "image_url", "image_url": {"url": "data:image/png;base64,AAAA"}}
                ]
            }])
        );

        let openai_reasoning = build_openai_request_with_image(
            "o3-mini",
            "Read subtitle",
            Some(("AAAA", "image/png")),
        );
        let reasoning_json = serde_json::to_value(&openai_reasoning).unwrap();
        assert!(reasoning_json.get("temperature").is_none());

        let gemini = build_gemini_request_with_image("Read subtitle", Some(("BBBB", "image/png")));
        let gemini_json = serde_json::to_value(&gemini).unwrap();
        assert!(
            (gemini_json["generationConfig"]["temperature"]
                .as_f64()
                .unwrap()
                - 0.3)
                .abs()
                < 1e-6
        );
        assert_eq!(
            gemini_json["contents"],
            serde_json::json!([{
                "parts": [
                    {"text": "Read subtitle"},
                    {"inline_data": {"mime_type": "image/png", "data": "BBBB"}}
                ]
            }])
        );

        let anthropic = build_anthropic_request_with_image(
            "claude-haiku-4-5",
            "Read subtitle",
            Some(("CCCC", "image/png")),
        );
        let anthropic_json = serde_json::to_value(&anthropic).unwrap();
        assert_eq!(anthropic_json["model"], "claude-haiku-4-5");
        assert_eq!(anthropic_json["max_tokens"], 4096);
        assert!((anthropic_json["temperature"].as_f64().unwrap() - 0.3).abs() < 1e-6);
        assert_eq!(
            anthropic_json["messages"],
            serde_json::json!([{
                "role": "user",
                "content": [
                    {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "CCCC"}},
                    {"type": "text", "text": "Read subtitle"}
                ]
            }])
        );

        let anthropic_new = build_anthropic_request_with_image(
            "claude-opus-4-7",
            "Read subtitle",
            Some(("CCCC", "image/png")),
        );
        let anthropic_new_json = serde_json::to_value(&anthropic_new).unwrap();
        assert!(anthropic_new_json.get("temperature").is_none());
    }

    async fn spawn_single_response_mock_server(
        response_body: &'static str,
    ) -> (
        std::net::SocketAddr,
        tokio::task::JoinHandle<(String, String)>,
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let (mut stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
                .await
                .unwrap()
                .unwrap();

            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            let header_end = loop {
                let n = stream.read(&mut chunk).await.unwrap();
                assert!(n > 0, "unexpected EOF reading request");
                buf.extend_from_slice(&chunk[..n]);
                if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    break pos + 4;
                }
            };
            let headers_str = String::from_utf8_lossy(&buf[..header_end]).to_string();
            let content_length = headers_str
                .lines()
                .find_map(|line| {
                    let (name, val) = line.split_once(':')?;
                    name.trim()
                        .eq_ignore_ascii_case("content-length")
                        .then(|| val.trim().parse::<usize>().ok())?
                })
                .unwrap_or(0);
            while buf.len() < header_end + content_length {
                let n = stream.read(&mut chunk).await.unwrap();
                assert!(n > 0, "unexpected EOF reading request body");
                buf.extend_from_slice(&chunk[..n]);
            }
            let body_str =
                String::from_utf8_lossy(&buf[header_end..header_end + content_length]).to_string();

            let http_resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body
            );
            stream.write_all(http_resp.as_bytes()).await.unwrap();
            (headers_str, body_str)
        });
        (addr, handle)
    }

    #[tokio::test]
    async fn test_generate_vision_response_end_to_end_all_providers() {
        // 1. OpenAI-compatible (Local / OpenRouter / Groq)
        let (openai_addr, openai_srv) = spawn_single_response_mock_server(
            r#"{"choices":[{"message":{"role":"assistant","content":"Hello\nWorld"}}]}"#,
        )
        .await;
        let openai_translator = Translator::new(TranslatorConfig {
            base_url: format!("http://{openai_addr}/v1"),
            model: "gpt-4o-mini".to_string(),
            api_key: Some("openai-key".to_string()),
            api_type: ApiType::OpenRouter,
        });
        let openai_out = openai_translator
            .generate_vision_response("Extract text", "QUFBQQ==", "image/png")
            .await
            .unwrap();
        assert_eq!(openai_out, "Hello\nWorld");
        let (headers, body) = openai_srv.await.unwrap();
        assert!(headers.starts_with("POST /v1/chat/completions HTTP/"));
        assert!(
            headers
                .to_ascii_lowercase()
                .contains("authorization: bearer openai-key\r\n")
        );
        let body_json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(
            body_json["messages"][0]["content"][1]["image_url"]["url"],
            "data:image/png;base64,QUFBQQ=="
        );

        // 2. Google Gemini
        let (google_addr, google_srv) = spawn_single_response_mock_server(
            r#"{"candidates":[{"content":{"parts":[{"text":"Ciao mondo"}]},"finishReason":"STOP"}]}"#,
        )
        .await;
        let google_translator = Translator::new(TranslatorConfig {
            base_url: format!("http://{google_addr}/v1beta"),
            model: "gemini-2.5-flash".to_string(),
            api_key: Some("gemini-key".to_string()),
            api_type: ApiType::Google,
        });
        let google_out = google_translator
            .generate_vision_response("Extract text", "QkJCQg==", "image/png")
            .await
            .unwrap();
        assert_eq!(google_out, "Ciao mondo");
        let (headers, body) = google_srv.await.unwrap();
        assert!(headers.starts_with("POST /v1beta/models/gemini-2.5-flash:generateContent HTTP/"));
        assert!(
            headers
                .to_ascii_lowercase()
                .contains("x-goog-api-key: gemini-key\r\n")
        );
        let body_json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(
            body_json["contents"][0]["parts"][1]["inline_data"]["mime_type"],
            "image/png"
        );
        assert_eq!(
            body_json["contents"][0]["parts"][1]["inline_data"]["data"],
            "QkJCQg=="
        );

        // 3. Anthropic
        let (anthropic_addr, anthropic_srv) =
            spawn_single_response_mock_server(r#"{"content":[{"type":"text","text":"Bonjour"}]}"#)
                .await;
        let anthropic_translator = Translator::new(TranslatorConfig {
            base_url: format!("http://{anthropic_addr}/v1"),
            model: "claude-haiku-4-5".to_string(),
            api_key: Some("anthropic-key".to_string()),
            api_type: ApiType::Anthropic,
        });
        let anthropic_out = anthropic_translator
            .generate_vision_response("Extract text", "Q0NDQw==", "image/png")
            .await
            .unwrap();
        assert_eq!(anthropic_out, "Bonjour");
        let (headers, body) = anthropic_srv.await.unwrap();
        assert!(headers.starts_with("POST /v1/messages HTTP/"));
        assert!(
            headers
                .to_ascii_lowercase()
                .contains("x-api-key: anthropic-key\r\n")
        );
        let body_json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(body_json["messages"][0]["content"][0]["type"], "image");
        assert_eq!(
            body_json["messages"][0]["content"][0]["source"]["data"],
            "Q0NDQw=="
        );
        assert_eq!(
            body_json["messages"][0]["content"][1]["text"],
            "Extract text"
        );
    }

    #[test]
    fn batch_json_and_legacy_and_gemini_quotes_are_handled_safely() {
        // Quoted text preserved in extract_gemini_text
        let resp: GeminiResponse =
            serde_json::from_str(r#"{"candidates":[{"content":{"parts":[{"text":"\"Ciao\""}]}}]}"#)
                .unwrap();
        assert_eq!(extract_gemini_text(resp, 200).unwrap(), "\"Ciao\"");

        // Surrounding prose with brackets does not break JSON array extraction
        let wrapped = r#"Note [draft]:
[{"id":1,"text":"Uno"}]
Trailing [end]"#;
        let map = parse_json_translations(wrapped, 1).unwrap();
        assert_eq!(map.get(&1).map(String::as_str), Some("Uno"));

        // Dialogue containing "David:" or "said:" is not treated as a legacy record header
        let legacy = "ID: 1 | Trans: David: \"hello\"\nsaid: world\nID: 2 | Trans: Fine";
        let parsed = try_legacy_parsing(legacy, 2).unwrap();
        assert_eq!(
            parsed.get(&1).map(String::as_str),
            Some("David: \"hello\"\nsaid: world")
        );
        assert_eq!(parsed.get(&2).map(String::as_str), Some("Fine"));
    }
}
