use serde::Serialize;

/// Safe provider failure categories; bodies, prompts and credentials are excluded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderFailureKind {
    RateLimit,
    Authentication,
    Unavailable,
    InvalidRequest,
    InvalidResponse,
    EmptyResponse,
}

/// Consumers can downcast an anyhow error without parsing localized prose.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderError {
    pub provider: &'static str,
    pub status: u16,
    pub kind: ProviderFailureKind,
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let detail = match self.kind {
            ProviderFailureKind::RateLimit => "quota exhausted or rate limited",
            ProviderFailureKind::Authentication => "authentication or access denied",
            ProviderFailureKind::Unavailable => "provider unavailable",
            ProviderFailureKind::InvalidRequest => {
                "request rejected; check model and endpoint configuration"
            }
            ProviderFailureKind::InvalidResponse => "invalid API response",
            ProviderFailureKind::EmptyResponse => "empty API response",
        };
        write!(f, "{} API (HTTP {}): {detail}", self.provider, self.status)
    }
}
impl std::error::Error for ProviderError {}

pub(crate) fn provider_error(provider: &'static str, status: u16, body: &str) -> ProviderError {
    let body = body.to_ascii_lowercase();
    let quota = [
        "insufficient_quota",
        "resource_exhausted",
        "resource exhausted",
        "quota",
        "rate limit",
    ]
    .iter()
    .any(|term| body.contains(term));
    let kind = if status == 401 {
        ProviderFailureKind::Authentication
    } else if status == 429 || quota {
        ProviderFailureKind::RateLimit
    } else if status == 403 {
        ProviderFailureKind::Authentication
    } else if status >= 500 {
        ProviderFailureKind::Unavailable
    } else {
        ProviderFailureKind::InvalidRequest
    };
    ProviderError {
        provider,
        status,
        kind,
    }
}

/// Optional numeric-only retry telemetry. Libraries do not choose a log sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TranslationDiagnostic {
    pub status: u16,
    pub attempt: u32,
    pub max_retries: u32,
    pub retry_in_ms: u64,
}
