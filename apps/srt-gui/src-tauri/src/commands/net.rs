use std::sync::LazyLock;
use std::time::Duration;

use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};

static CLIENT_NO_REDIRECT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .redirect(Policy::none())
        .build()
        .unwrap_or_default()
});

static CLIENT_FOLLOW_CUSTOM: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .redirect(Policy::custom(|attempt| {
            if attempt.previous().len() >= 10 {
                attempt.error("Too many redirects")
            } else if matches!(attempt.url().scheme(), "http" | "https") {
                attempt.follow()
            } else {
                attempt.error("Redirect host not allowed")
            }
        }))
        .build()
        .unwrap_or_default()
});

static CLIENT_FOLLOW: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .redirect(Policy::custom(|attempt| {
            if attempt.previous().len() >= 10 {
                attempt.error("Too many redirects")
            } else if matches!(attempt.url().scheme(), "http" | "https")
                && attempt.url().host_str().is_some_and(is_host_allowed)
            {
                attempt.follow()
            } else {
                attempt.error("Redirect host not allowed")
            }
        }))
        .build()
        .unwrap_or_default()
});

fn get_client(follow_redirects: bool, allow_custom_host: bool) -> &'static reqwest::Client {
    if !follow_redirects {
        &CLIENT_NO_REDIRECT
    } else if allow_custom_host {
        &CLIENT_FOLLOW_CUSTOM
    } else {
        &CLIENT_FOLLOW
    }
}

const ALLOWED_HOSTS: &[&str] = &[
    "api.openai.com",
    "generativelanguage.googleapis.com",
    "api.anthropic.com",
    "openrouter.ai",
    "api.groq.com",
    "api.mistral.ai",
    "integrate.api.nvidia.com",
    "api.deepgram.com",
    "api.assemblyai.com",
    "github.com",
    "api.github.com",
    "raw.githubusercontent.com",
];

fn is_host_allowed(host: &str) -> bool {
    if host == "localhost" || ALLOWED_HOSTS.contains(&host) {
        return true;
    }
    // Local network hosts/IPs: allow both loopback and LAN inference servers
    // (e.g., LM Studio or Ollama on another machine on the same LAN) without
    // requiring a static IP allowlist.
    if let Ok(ip) = host.trim_matches(['[', ']']).parse::<std::net::IpAddr>() {
        return ip.is_loopback() || is_private_lan_ip(&ip);
    }
    host.ends_with(".local")
}

fn is_private_lan_ip(ip: &std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => v4.is_private() || v4.is_link_local(),
        std::net::IpAddr::V6(v6) => {
            // fc00::/7 (ULA) and fe80::/10 (link-local).
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

    /// Set by the frontend when the URL comes from an explicit user configuration
    /// ("custom"/"local" provider in Settings, or an API key's custom `apiUrl`)
    /// rather than a hardcoded host. The scheme remains restricted to http/https.
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

// This IPC endpoint serves text metadata, not model or release asset downloads.
const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

async fn bounded_response_text(mut response: reqwest::Response) -> Result<String, String> {
    let too_large = || format!("HTTP response exceeds the {MAX_RESPONSE_BYTES}-byte limit");
    if response
        .content_length()
        .is_some_and(|size| size > MAX_RESPONSE_BYTES as u64)
    {
        return Err(too_large());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if chunk.len() > MAX_RESPONSE_BYTES - bytes.len() {
            return Err(too_large());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
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
    let url = reqwest::Url::parse(&req.url).map_err(|e| format!("Invalid URL: {e}"))?;

    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!("Disallowed URL scheme: {}", url.scheme()));
    }

    match url.host_str() {
        Some(host) if is_host_allowed(host) || req.allow_custom_host => {}
        Some(host) => return Err(format!("Disallowed host: {host}")),
        None => return Err("URL is missing a host".to_string()),
    }

    let follow_redirects = req.redirect != "manual";
    let client = get_client(follow_redirects, req.allow_custom_host);

    let method = req
        .method
        .parse::<reqwest::Method>()
        .map_err(|e| format!("Invalid HTTP method: {e}"))?;

    let mut builder = client
        .request(method, url)
        .timeout(Duration::from_millis(req.timeout_ms));
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
    let body = bounded_response_text(response).await?;

    Ok(HttpFetchResponse {
        status,
        url: final_url,
        headers,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn local_response(header: String, body: Vec<u8>) -> reqwest::Response {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 4096];
            let _ = stream.read(&mut request).await;
            if stream.write_all(header.as_bytes()).await.is_ok() {
                let _ = stream.write_all(&body).await;
            }
        });
        reqwest::Client::new()
            .get(format!("http://{address}"))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn bounds_fixed_length_chunked_and_close_delimited_responses() {
        let size = MAX_RESPONSE_BYTES;
        let response = local_response(
            format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", size + 1),
            Vec::new(),
        )
        .await;
        assert!(
            bounded_response_text(response)
                .await
                .unwrap_err()
                .contains("exceeds")
        );

        for chunked in [false, true] {
            let body = vec![b'x'; size + 1];
            let (header, body) = if chunked {
                let mut encoded = format!("{:x}\r\n", body.len()).into_bytes();
                encoded.extend(body);
                encoded.extend(b"\r\n0\r\n\r\n");
                (
                    "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n",
                    encoded,
                )
            } else {
                ("HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n", body)
            };
            let response = local_response(header.into(), body).await;
            assert!(
                bounded_response_text(response)
                    .await
                    .unwrap_err()
                    .contains("exceeds")
            );
        }

        let response = local_response(
            format!("HTTP/1.1 200 OK\r\nContent-Length: {size}\r\n\r\n"),
            vec![b'x'; size],
        )
        .await;
        assert_eq!(bounded_response_text(response).await.unwrap().len(), size);
    }

    #[tokio::test]
    async fn preserves_utf8_text_and_reports_truncated_transport() {
        let text = "日本語 café";
        let response = local_response(
            format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", text.len()),
            text.as_bytes().to_vec(),
        )
        .await;
        assert_eq!(bounded_response_text(response).await.unwrap(), text);
        let response = local_response(
            "HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n".into(),
            b"short".to_vec(),
        )
        .await;
        assert!(bounded_response_text(response).await.is_err());
    }

    #[test]
    fn permits_loopback_and_lan_hosts_but_not_arbitrary_public_hosts() {
        for host in [
            "localhost",
            "127.0.0.1",
            "[::1]",
            "192.168.1.2",
            "[fd00::1]",
            "ollama.local",
            "api.openai.com",
            "api.deepgram.com",
            "api.assemblyai.com",
            "github.com",
            "api.github.com",
            "raw.githubusercontent.com",
        ] {
            assert!(is_host_allowed(host), "{host}");
        }
        for host in [
            "example.com",
            "8.8.8.8",
            "api.openai.com.example.com",
            "models.github.ai",
        ] {
            assert!(!is_host_allowed(host), "{host}");
        }
    }

    #[test]
    fn allows_deepgram_and_assemblyai_hosts() {
        assert!(is_host_allowed("api.deepgram.com"));
        assert!(is_host_allowed("api.assemblyai.com"));
    }
}
