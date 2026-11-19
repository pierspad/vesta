use std::sync::LazyLock;
use std::time::Duration;

use reqwest::redirect::Policy;
use serde::{Deserialize, Serialize};

static CLIENT_NO_REDIRECT: LazyLock<Result<reqwest::Client, reqwest::Error>> =
    LazyLock::new(|| reqwest::Client::builder().redirect(Policy::none()).build());

static CLIENT_FOLLOW_CUSTOM: LazyLock<Result<reqwest::Client, reqwest::Error>> =
    LazyLock::new(|| {
        reqwest::Client::builder()
            .redirect(Policy::custom(|attempt| {
                if attempt.previous().len() >= 10 {
                    attempt.error("Too many redirects")
                } else if matches!(attempt.url().scheme(), "http" | "https")
                    && !redirect_crosses_into_local_network(
                        attempt.previous().first(),
                        attempt.url(),
                    )
                {
                    attempt.follow()
                } else {
                    attempt.error("Redirect host not allowed")
                }
            }))
            .build()
    });

static CLIENT_FOLLOW: LazyLock<Result<reqwest::Client, reqwest::Error>> = LazyLock::new(|| {
    reqwest::Client::builder()
        .redirect(Policy::custom(|attempt| {
            if attempt.previous().len() >= 10 {
                attempt.error("Too many redirects")
            } else if matches!(attempt.url().scheme(), "http" | "https")
                && attempt.url().host_str().is_some_and(is_host_allowed)
                && !redirect_crosses_into_local_network(attempt.previous().first(), attempt.url())
            {
                attempt.follow()
            } else {
                attempt.error("Redirect host not allowed")
            }
        }))
        .build()
});

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

static CLIENT_FOLLOW_AUTHENTICATED: LazyLock<Result<reqwest::Client, reqwest::Error>> =
    LazyLock::new(|| {
        reqwest::Client::builder()
            .redirect(api_redirect_policy())
            .build()
    });

fn get_client(
    follow_redirects: bool,
    allow_custom_host: bool,
    authenticated: bool,
) -> Result<&'static reqwest::Client, String> {
    let client = if !follow_redirects {
        &CLIENT_NO_REDIRECT
    } else if authenticated {
        &CLIENT_FOLLOW_AUTHENTICATED
    } else if allow_custom_host {
        &CLIENT_FOLLOW_CUSTOM
    } else {
        &CLIENT_FOLLOW
    };
    client
        .as_ref()
        .map_err(|_| "Failed to configure HTTP client".into())
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
        std::net::IpAddr::V4(v4) => v4.is_private(),
        std::net::IpAddr::V6(v6) => {
            // fc00::/7 (ULA); explicitly exclude link-local fe80::/10 metadata range.
            let seg0 = v6.segments()[0];
            (0xfc00..=0xfdff).contains(&seg0)
        }
    }
}

fn is_local_or_private_host(host: &str) -> bool {
    let trimmed = host.trim_matches(['[', ']']);
    if trimmed.eq_ignore_ascii_case("localhost") || trimmed.ends_with(".local") {
        return true;
    }
    if let Ok(ip) = trimmed.parse::<std::net::IpAddr>() {
        let is_link_local = match ip {
            std::net::IpAddr::V4(v4) => v4.is_link_local(),
            std::net::IpAddr::V6(v6) => (v6.segments()[0] & 0xffc0) == 0xfe80,
        };
        return ip.is_loopback() || ip.is_unspecified() || is_link_local || is_private_lan_ip(&ip);
    }
    false
}

fn redirect_crosses_into_local_network(
    initial_url: Option<&reqwest::Url>,
    next_url: &reqwest::Url,
) -> bool {
    let Some(initial_host) = initial_url.and_then(|u| u.host_str()) else {
        return false;
    };
    let Some(next_host) = next_url.host_str() else {
        return true;
    };
    !is_local_or_private_host(initial_host) && is_local_or_private_host(next_host)
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
    let authenticated = !url.username().is_empty()
        || url.password().is_some()
        || req.headers.iter().any(|(name, _)| {
            !matches!(
                name.to_ascii_lowercase().as_str(),
                "accept" | "accept-language" | "content-type" | "user-agent"
            )
        });
    let client = get_client(follow_redirects, req.allow_custom_host, authenticated)?;

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

    #[tokio::test]
    async fn authenticated_metadata_redirects_do_not_forward_custom_keys_to_another_origin() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let destination = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let target = destination.local_addr().unwrap();
        let origin = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = origin.local_addr().unwrap();
        let task = tokio::spawn(async move {
            for _ in 0..2 {
                let (mut stream, _) = origin.accept().await.unwrap();
                let mut bytes = [0u8; 4096];
                assert!(stream.read(&mut bytes).await.unwrap() > 0);
                stream.write_all(format!("HTTP/1.1 307 Redirect\r\nLocation: http://{target}/models\r\nContent-Length: 0\r\n\r\n").as_bytes()).await.unwrap();
            }
        });
        for name in ["x-goog-api-key", "x-custom-secret"] {
            let response = http_fetch(HttpFetchRequest {
                url: format!("http://{address}/models"),
                method: "GET".into(),
                headers: vec![(name.into(), "private-key".into())],
                body: None,
                redirect: "follow".into(),
                timeout_ms: 2000,
                allow_custom_host: true,
            })
            .await
            .unwrap();
            assert_eq!(response.status, 307);
        }
        task.await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(100), destination.accept())
                .await
                .is_err()
        );
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

    #[test]
    fn blocks_link_local_metadata_ips_and_cross_boundary_redirects() {
        for blocked in ["169.254.169.254", "169.254.0.1", "[fe80::1]"] {
            assert!(!is_host_allowed(blocked), "{blocked} must be blocked");
        }
        let public_origin = reqwest::Url::parse("https://api.openai.com/v1/models").unwrap();
        let local_target = reqwest::Url::parse("http://127.0.0.1:8080/secret").unwrap();
        let lan_target = reqwest::Url::parse("http://192.168.1.1/admin").unwrap();
        let meta_target = reqwest::Url::parse("http://169.254.169.254/latest").unwrap();
        let public_target = reqwest::Url::parse("https://openrouter.ai/api/v1").unwrap();
        assert!(redirect_crosses_into_local_network(
            Some(&public_origin),
            &local_target
        ));
        assert!(redirect_crosses_into_local_network(
            Some(&public_origin),
            &lan_target
        ));
        assert!(redirect_crosses_into_local_network(
            Some(&public_origin),
            &meta_target
        ));
        assert!(!redirect_crosses_into_local_network(
            Some(&public_origin),
            &public_target
        ));
        assert!(!redirect_crosses_into_local_network(
            Some(&local_target),
            &local_target
        ));
    }
}
