use std::time::Duration;

use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};

const ALLOWED_HOSTS: &[&str] = &[
    "api.openai.com",
    "generativelanguage.googleapis.com",
    "api.anthropic.com",
    "openrouter.ai",
    "api.groq.com",
    "api.mistral.ai",
    "models.github.ai",
    "integrate.api.nvidia.com",
    "github.com",
    "api.github.com",
    "raw.githubusercontent.com",
];

fn is_host_allowed(host: &str) -> bool {
    host == "localhost" || host == "127.0.0.1" || ALLOWED_HOSTS.contains(&host)
}

pub type HeaderPairs = Vec<(String, String)>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpFetchRequest {
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub headers: HeaderPairs,
    #[serde(default)]
    pub body: Option<String>,

    #[serde(default = "default_redirect")]
    pub redirect: String,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_method() -> String {
    "GET".to_string()
}

fn default_redirect() -> String {
    "follow".to_string()
}

fn default_timeout_ms() -> u64 {
    15_000
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpFetchResponse {
    pub status: u16,
    pub url: String,
    pub headers: HeaderPairs,
    pub body: String,
}

#[tauri::command]
pub async fn http_fetch(req: HttpFetchRequest) -> Result<HttpFetchResponse, String> {
    let url = reqwest::Url::parse(&req.url).map_err(|e| format!("URL non valido: {e}"))?;

    match url.host_str() {
        Some(host) if is_host_allowed(host) => {}
        Some(host) => return Err(format!("Host non consentito: {host}")),
        None => return Err("URL senza host".to_string()),
    }

    let follow_redirects = req.redirect != "manual";

    let client = reqwest::Client::builder()
        .redirect(if follow_redirects {
            Policy::default()
        } else {
            Policy::none()
        })
        .timeout(Duration::from_millis(req.timeout_ms))
        .build()
        .map_err(|e| e.to_string())?;

    let method = req
        .method
        .parse::<reqwest::Method>()
        .map_err(|e| format!("Metodo HTTP non valido: {e}"))?;

    let mut builder = client.request(method, url);
    for (name, value) in &req.headers {
        builder = builder.header(name, value);
    }
    if let Some(body) = req.body {
        builder = builder.body(body);
    }

    let response = builder.send().await.map_err(|e| e.to_string())?;

    let status = response.status().as_u16();
    let final_url = response.url().to_string();
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.to_string(),
                value.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    let body = response.text().await.map_err(|e| e.to_string())?;

    Ok(HttpFetchResponse {
        status,
        url: final_url,
        headers,
        body,
    })
}
