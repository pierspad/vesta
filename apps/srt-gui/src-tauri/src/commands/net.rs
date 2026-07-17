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
    if host == "localhost" || ALLOWED_HOSTS.contains(&host) {
        return true;
    }
    // Host/IP di rete locale: coprono sia il loopback sia i server di
    // inferenza in LAN (es. LM Studio/Ollama su un'altra macchina della
    // stessa rete), senza dover mantenere un elenco statico di IP.
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return ip.is_loopback() || is_private_lan_ip(&ip);
    }
    host.ends_with(".local")
}

fn is_private_lan_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => v4.is_private() || v4.is_link_local(),
        std::net::IpAddr::V6(v6) => {
            // fc00::/7 (ULA) e fe80::/10 (link-local).
            let seg0 = v6.segments()[0];
            (0xfc00..=0xfdff).contains(&seg0) || (seg0 & 0xffc0) == 0xfe80
        }
    }
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

    /// Il frontend lo imposta quando l'URL proviene da una configurazione
    /// esplicita dell'utente (provider "custom"/"local" in Settings, o
    /// `apiUrl` di una API key), non da un host cablato nel codice. In quel
    /// caso l'host non deve necessariamente stare nella whitelist statica:
    /// è l'utente stesso ad aver scelto quell'endpoint, esattamente come
    /// quando incolla la sua API key. Lo schema resta comunque limitato a
    /// http/https.
    #[serde(default)]
    pub allow_custom_host: bool,
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

    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!("Schema non consentito: {}", url.scheme()));
    }

    match url.host_str() {
        Some(host) if is_host_allowed(host) || req.allow_custom_host => {}
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
