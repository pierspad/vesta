//! Comando HTTP generico per il frontend, appoggiato al `reqwest` del workspace.
//!
//! Prima queste chiamate (controllo aggiornamenti su GitHub, discovery dei
//! modelli LLM) passavano da `@tauri-apps/plugin-http`. Quel plugin porta con
//! sé un secondo stack `reqwest` indipendente da quello già linkato da
//! `srt-translate` (workspace, v0.13): essendo una major diversa (v0.12),
//! cargo non può unificarlo con quello esistente, e l'intero albero
//! hyper/h2/rustls/tower viene duplicato nel lockfile. Esponendo un unico
//! comando Tauri che riusa il client del workspace, quel secondo stack sparisce.
//!
//! L'allowlist qui sotto rispecchia esattamente quella che prima viveva in
//! `capabilities/default.json` sotto `http:allow-fetch`: la sicurezza (niente
//! SSRF verso host arbitrari dal webview) resta quindi invariata, solo
//! applicata lato Rust invece che dal plugin.

use std::time::Duration;

use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};

/// Host verso cui questo comando può effettuare richieste: deve coprire
/// l'`apiUrl` di default di ogni provider `enabled: true` in `models.ts`,
/// altrimenti il pulsante "scopri modelli" fallisce in silenzio per quel
/// provider pur essendo selezionabile in UI (la traduzione vera e propria
/// non passa da qui: usa il reqwest di `srt-translate` lato Rust, non
/// soggetto a questa allowlist).
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

/// Coppie header (nome, valore); un `Vec` invece di una `HashMap` per
/// preservare l'ordine e ammettere header ripetuti, come fa la Fetch API.
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
    /// "follow" (default, come il browser) oppure "manual" per leggere
    /// l'header `Location` senza seguire il redirect.
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

/// Esegue una richiesta HTTP per conto del frontend, verso un host della
/// allowlist. Sostituisce `@tauri-apps/plugin-http`'s `fetch` — vedi il
/// commento in testa al modulo.
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

    // Con `Policy::none()` reqwest non segue il redirect ma restituisce
    // comunque normalmente la risposta 3xx (con l'header `Location`
    // leggibile), esattamente come `fetch(url, { redirect: "manual" })`.
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
