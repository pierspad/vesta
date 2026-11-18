//! Vision LLM OCR backend powered by `srt-translate`.

use crate::backend::OcrBackend;
use crate::bitmap::BitmapSubtitleCue;
use anyhow::{Context as _, Result};
use base64::Engine as _;
use srt_translate::{
    ProviderError, ProviderFailureKind, TierEntry, TierScheduler, TranslationDiagnostic,
    TranslatorPool, build_pool, is_rate_limit_error, pool_concurrency,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// OCR backend that delegates subtitle bitmap recognition to multimodal Vision LLMs
/// via [`srt_translate`]'s tiered failover scheduler.
pub struct VisionLlmBackend {
    pool: Arc<TranslatorPool>,
    scheduler: Arc<Mutex<TierScheduler>>,
}

impl VisionLlmBackend {
    /// Creates a new Vision LLM OCR backend from priority-ordered provider tiers.
    pub fn new(tiers: &[Vec<TierEntry>]) -> Result<Self> {
        Self::build(tiers, None)
    }

    /// Creates a new Vision LLM OCR backend with a non-blocking retry diagnostic observer.
    pub fn with_diagnostics(
        tiers: &[Vec<TierEntry>],
        observer: Arc<dyn Fn(TranslationDiagnostic) + Send + Sync>,
    ) -> Result<Self> {
        Self::build(tiers, Some(observer))
    }

    fn build(
        tiers: &[Vec<TierEntry>],
        observer: Option<Arc<dyn Fn(TranslationDiagnostic) + Send + Sync>>,
    ) -> Result<Self> {
        let mut pool = build_pool(tiers).map_err(anyhow::Error::msg)?;
        if let Some(observer) = observer {
            for tier in &mut pool {
                for entry in tier {
                    let obs = observer.clone();
                    entry.translator = entry
                        .translator
                        .clone()
                        .with_diagnostics(move |diag| obs(diag));
                }
            }
        }
        let scheduler = TierScheduler::new(&pool);
        Ok(Self {
            pool: Arc::new(pool),
            scheduler: Arc::new(Mutex::new(scheduler)),
        })
    }

    /// Returns the recommended bounded concurrency for this provider pool.
    pub fn concurrency(&self) -> usize {
        pool_concurrency(&self.pool)
    }

    async fn recognize_cue_inner(
        &self,
        cue: &BitmapSubtitleCue,
        language_hint: Option<&str>,
        cancel: &CancellationToken,
    ) -> Result<String> {
        anyhow::ensure!(!cancel.is_cancelled(), "OCR cancelled by user");

        let b64 = base64::engine::general_purpose::STANDARD.encode(&cue.png_bytes);
        let prompt = build_ocr_prompt(language_hint);

        let mut retries: u32 = 0;
        loop {
            anyhow::ensure!(!cancel.is_cancelled(), "OCR cancelled by user");

            let acquired = { self.scheduler.lock().await.acquire() };
            let Some((ti, ei)) = acquired else {
                anyhow::bail!("All Vision LLM tiers are exhausted or rate-limited");
            };

            let entry = self
                .pool
                .get(ti)
                .and_then(|tier| tier.get(ei))
                .cloned()
                .context("Invalid scheduler pool index")?;

            if let Some(ref limiter) = entry.rate_limiter {
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => anyhow::bail!("OCR cancelled by user"),
                    _ = limiter.until_ready() => {}
                }
            }

            let result = tokio::select! {
                biased;
                _ = cancel.cancelled() => anyhow::bail!("OCR cancelled by user"),
                res = entry.translator.generate_vision_response(&prompt, &b64, "image/png") => res,
            };

            match result {
                Ok(raw) => return Ok(clean_ocr_response(&raw)),
                Err(err)
                    if err
                        .downcast_ref::<ProviderError>()
                        .is_some_and(|pe| pe.kind == ProviderFailureKind::EmptyResponse) =>
                {
                    return Ok(String::new());
                }
                Err(err) if is_rate_limit_error(&err) => {
                    self.scheduler.lock().await.report_exhausted(ti, ei);
                    retries = 0;
                }
                Err(err) => {
                    if retries < 2 {
                        retries += 1;
                        let delay = std::time::Duration::from_millis(500 * u64::from(retries));
                        tokio::select! {
                            biased;
                            _ = cancel.cancelled() => anyhow::bail!("OCR cancelled by user"),
                            _ = tokio::time::sleep(delay) => {}
                        }
                    } else {
                        return Err(err);
                    }
                }
            }
        }
    }
}

impl OcrBackend for VisionLlmBackend {
    fn recognize_cue<'a>(
        &'a self,
        cue: &'a BitmapSubtitleCue,
        language_hint: Option<&'a str>,
        cancel: &'a CancellationToken,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(self.recognize_cue_inner(cue, language_hint, cancel))
    }

    fn recommended_concurrency(&self) -> usize {
        self.concurrency()
    }
}

fn build_ocr_prompt(language_hint: Option<&str>) -> String {
    let mut prompt = String::from(
        "Extract the exact subtitle text visible in this image.\n\
         Preserve intentional line breaks as newlines.\n\
         Do not translate, paraphrase, explain, add speaker labels, timestamps, or markdown formatting.\n\
         If the image contains no legible subtitle text, respond with exactly: <EMPTY>",
    );
    if let Some(lang) = language_hint.map(str::trim).filter(|l| {
        !l.is_empty() && !l.eq_ignore_ascii_case("und") && !l.eq_ignore_ascii_case("auto")
    }) {
        prompt.push_str(&format!("\nLanguage hint: {lang}."));
    }
    prompt
}

fn clean_ocr_response(raw: &str) -> String {
    let mut text = raw.trim();
    if let Some(after_open) = text.strip_prefix("```") {
        // Drop optional fence language identifier on the first line (e.g. ```text\n).
        let after_header = match after_open.find('\n') {
            Some(pos) => &after_open[pos + 1..],
            None => after_open,
        };
        if let Some(inner) = after_header.strip_suffix("```") {
            text = inner.trim();
        }
    }

    if text.is_empty() || text.eq_ignore_ascii_case("<EMPTY>") {
        return String::new();
    }

    text.replace("\r\n", "\n")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn sample_cue() -> BitmapSubtitleCue {
        BitmapSubtitleCue {
            id: 1,
            start_ms: 1000,
            end_ms: 2500,
            png_bytes: b"\x89PNG\r\n\x1a\nfake".to_vec(),
            width: 32,
            height: 16,
        }
    }

    async fn spawn_http_responder(
        responses: Vec<(&'static str, &'static str)>,
    ) -> (std::net::SocketAddr, tokio::task::JoinHandle<Vec<String>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let mut bodies = Vec::new();
            for (status_line, body) in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                let header_end = loop {
                    let n = stream.read(&mut chunk).await.unwrap();
                    assert!(n > 0);
                    buf.extend_from_slice(&chunk[..n]);
                    if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        break pos + 4;
                    }
                };
                let headers = String::from_utf8_lossy(&buf[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (k, v) = line.split_once(':')?;
                        k.trim()
                            .eq_ignore_ascii_case("content-length")
                            .then(|| v.trim().parse::<usize>().ok())?
                    })
                    .unwrap_or(0);
                while buf.len() < header_end + content_length {
                    let n = stream.read(&mut chunk).await.unwrap();
                    assert!(n > 0);
                    buf.extend_from_slice(&chunk[..n]);
                }
                bodies.push(
                    String::from_utf8_lossy(&buf[header_end..header_end + content_length])
                        .to_string(),
                );
                let http = format!(
                    "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(http.as_bytes()).await.unwrap();
            }
            bodies
        });
        (addr, handle)
    }

    #[test]
    fn clean_ocr_response_strips_code_fences_and_handles_empty() {
        assert_eq!(clean_ocr_response("<EMPTY>"), "");
        assert_eq!(clean_ocr_response("  <empty>  "), "");
        assert_eq!(clean_ocr_response("   "), "");
        assert_eq!(
            clean_ocr_response("```text\r\nLine one\r\nLine two\r\n```"),
            "Line one\nLine two"
        );
        assert_eq!(clean_ocr_response("```\nHello!\n```"), "Hello!");
        assert_eq!(clean_ocr_response("Hello\nWorld"), "Hello\nWorld");
    }

    #[tokio::test]
    async fn vision_backend_fails_over_on_rate_limit_and_handles_empty_response() {
        // Tier 1 returns permanent quota 429; Tier 2 returns recognized text, then empty response.
        let (t1_addr, t1_srv) = spawn_http_responder(vec![(
            "429 Too Many Requests",
            r#"{"error":{"code":"insufficient_quota","message":"quota exhausted"}}"#,
        )])
        .await;
        let (t2_addr, t2_srv) = spawn_http_responder(vec![
            (
                "200 OK",
                r#"{"choices":[{"message":{"role":"assistant","content":"```\nLine 1\nLine 2\n```"}}]}"#,
            ),
            (
                "200 OK",
                r#"{"choices":[{"message":{"role":"assistant","content":"<EMPTY>"}}]}"#,
            ),
            (
                "200 OK",
                r#"{"choices":[{"message":{"role":"assistant","content":""}}]}"#,
            ),
        ])
        .await;

        let tiers = vec![
            vec![TierEntry {
                provider: "local".to_string(),
                model: "vision-t1".to_string(),
                api_key: None,
                api_url: Some(format!("http://{t1_addr}/v1")),
                rpm: Some(0),
                max_requests: None,
            }],
            vec![TierEntry {
                provider: "local".to_string(),
                model: "vision-t2".to_string(),
                api_key: None,
                api_url: Some(format!("http://{t2_addr}/v1")),
                rpm: Some(0),
                max_requests: None,
            }],
        ];

        let diagnostics = Arc::new(Mutex::new(Vec::new()));
        let diag_clone = diagnostics.clone();
        let backend = VisionLlmBackend::with_diagnostics(
            &tiers,
            Arc::new(move |diag| {
                if let Ok(mut guard) = diag_clone.try_lock() {
                    guard.push(diag);
                }
            }),
        )
        .unwrap();

        let cue = sample_cue();
        let cancel = CancellationToken::new();

        let first = backend
            .recognize_cue(&cue, Some("en"), &cancel)
            .await
            .unwrap();
        assert_eq!(first, "Line 1\nLine 2");

        let second = backend.recognize_cue(&cue, None, &cancel).await.unwrap();
        assert_eq!(second, "");

        let third = backend.recognize_cue(&cue, None, &cancel).await.unwrap();
        assert_eq!(third, "");

        let t1_reqs = t1_srv.await.unwrap();
        assert_eq!(t1_reqs.len(), 1);
        let t2_reqs = t2_srv.await.unwrap();
        assert_eq!(t2_reqs.len(), 3);
        assert!(t2_reqs[0].contains("Language hint: en."));
    }
}
