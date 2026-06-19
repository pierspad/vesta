//! # Cloud transcription
//!
//! Trascrizione audio tramite provider cloud. Ogni provider ha un formato di
//! risposta diverso; questo modulo li **normalizza** tutti in
//! [`TranscribedSegment`] (start_ms / end_ms / text), così il resto della
//! pipeline (post-processing + scrittura SRT) resta identico al percorso locale.
//!
//! Provider supportati:
//!  - `groq`, `openai`, `custom` → endpoint OpenAI-compatible `/audio/transcriptions`
//!    (o `/audio/translations` se si traduce in inglese), `response_format=verbose_json`.
//!  - `deepgram` → `/v1/listen` (body audio grezzo), usa le `utterances`.
//!  - `assemblyai` → upload asincrono + creazione transcript + polling, parole→segmenti.

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::transcribe::TranscribedSegment;

/// Configurazione di un provider cloud per la trascrizione.
#[derive(Debug, Clone)]
pub struct CloudConfig {
    /// Id provider: "groq" | "openai" | "deepgram" | "assemblyai" | "custom".
    pub provider: String,
    pub api_key: String,
    /// Base URL opzionale (override del default del provider; richiesto per "custom").
    pub api_url: Option<String>,
    pub model: String,
    /// Lingua sorgente (None = autodetect).
    pub language: Option<String>,
    /// Tradurre in inglese (supportato solo dai provider OpenAI-compatible).
    pub translate_to_english: bool,
}

impl CloudConfig {
    fn base_url(&self) -> String {
        if let Some(u) = self.api_url.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
            return u.trim_end_matches('/').to_string();
        }
        default_base_url(&self.provider).to_string()
    }
}

/// HTTP client preconfigurato per le richieste cloud (timeout generoso per upload
/// e polling). Esposto così il chiamante non deve dipendere direttamente da reqwest.
pub fn default_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(900))
        .build()
        .unwrap_or_default()
}

fn default_base_url(provider: &str) -> &'static str {
    match provider.to_lowercase().as_str() {
        "groq" => "https://api.groq.com/openai/v1",
        "openai" => "https://api.openai.com/v1",
        "deepgram" => "https://api.deepgram.com/v1",
        "assemblyai" => "https://api.assemblyai.com/v2",
        _ => "",
    }
}

/// Trascrive un singolo chunk audio (WAV) restituendo segmenti con timestamp
/// **relativi al chunk** (l'offset lo aggiunge il chiamante).
pub async fn transcribe_chunk(
    client: &reqwest::Client,
    cfg: &CloudConfig,
    audio: Vec<u8>,
    file_name: &str,
) -> Result<Vec<TranscribedSegment>> {
    match cfg.provider.to_lowercase().as_str() {
        "deepgram" => deepgram(client, cfg, audio).await,
        "assemblyai" => assemblyai(client, cfg, audio).await,
        // groq / openai / custom / qualunque endpoint OpenAI-compatible
        _ => openai_compatible(client, cfg, audio, file_name).await,
    }
}

// ─── OpenAI-compatible (Groq, OpenAI, custom) ──────────────────────────────────

#[derive(Deserialize)]
struct OpenAiSegment {
    start: f64,
    end: f64,
    text: String,
}

#[derive(Deserialize)]
struct OpenAiVerbose {
    #[serde(default)]
    segments: Vec<OpenAiSegment>,
    #[serde(default)]
    text: String,
    #[serde(default)]
    duration: Option<f64>,
}

async fn openai_compatible(
    client: &reqwest::Client,
    cfg: &CloudConfig,
    audio: Vec<u8>,
    file_name: &str,
) -> Result<Vec<TranscribedSegment>> {
    let endpoint = if cfg.translate_to_english {
        format!("{}/audio/translations", cfg.base_url())
    } else {
        format!("{}/audio/transcriptions", cfg.base_url())
    };

    let part = reqwest::multipart::Part::bytes(audio)
        .file_name(file_name.to_string())
        .mime_str("audio/wav")?;

    let mut form = reqwest::multipart::Form::new()
        .text("model", cfg.model.clone())
        .text("response_format", "verbose_json")
        .part("file", part);

    // timestamp_granularities[]=segment (ignorato da chi non lo supporta)
    form = form.text("timestamp_granularities[]", "segment");

    // language valido solo per /audio/transcriptions
    if !cfg.translate_to_english {
        if let Some(lang) = cfg.language.as_ref().filter(|l| l.as_str() != "auto") {
            form = form.text("language", lang.clone());
        }
    }

    let resp = client
        .post(&endpoint)
        .bearer_auth(&cfg.api_key)
        .multipart(form)
        .send()
        .await
        .context("Cloud transcription request failed")?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        anyhow::bail!("Transcription API error ({}): {}", status, truncate(&body, 400));
    }

    let parsed: OpenAiVerbose = serde_json::from_str(&body)
        .with_context(|| format!("Failed to parse transcription response: {}", truncate(&body, 300)))?;

    if !parsed.segments.is_empty() {
        return Ok(parsed
            .segments
            .into_iter()
            .filter(|s| !s.text.trim().is_empty())
            .map(|s| TranscribedSegment {
                start_ms: (s.start * 1000.0) as i64,
                end_ms: (s.end * 1000.0) as i64,
                text: s.text.trim().to_string(),
            })
            .collect());
    }

    // Nessun segmento (es. modelli che restituiscono solo testo): un unico blocco.
    let text = parsed.text.trim().to_string();
    if text.is_empty() {
        return Ok(vec![]);
    }
    let end_ms = (parsed.duration.unwrap_or(0.0) * 1000.0) as i64;
    Ok(vec![TranscribedSegment { start_ms: 0, end_ms, text }])
}

// ─── Deepgram ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct DgResponse {
    results: Option<DgResults>,
}
#[derive(Deserialize)]
struct DgResults {
    #[serde(default)]
    utterances: Vec<DgUtterance>,
    #[serde(default)]
    channels: Vec<DgChannel>,
}
#[derive(Deserialize)]
struct DgUtterance {
    start: f64,
    end: f64,
    transcript: String,
}
#[derive(Deserialize)]
struct DgChannel {
    #[serde(default)]
    alternatives: Vec<DgAlternative>,
}
#[derive(Deserialize)]
struct DgAlternative {
    #[serde(default)]
    transcript: String,
    #[serde(default)]
    words: Vec<DgWord>,
}
#[derive(Deserialize)]
struct DgWord {
    start: f64,
    end: f64,
    #[serde(default)]
    word: String,
    #[serde(default)]
    punctuated_word: Option<String>,
}

async fn deepgram(
    client: &reqwest::Client,
    cfg: &CloudConfig,
    audio: Vec<u8>,
) -> Result<Vec<TranscribedSegment>> {
    let model = if cfg.model.trim().is_empty() { "nova-3" } else { cfg.model.trim() };
    let mut url = format!(
        "{}/listen?model={}&smart_format=true&punctuate=true&utterances=true",
        cfg.base_url(),
        model
    );
    match cfg.language.as_ref().filter(|l| l.as_str() != "auto") {
        Some(lang) => url.push_str(&format!("&language={}", lang)),
        None => url.push_str("&detect_language=true"),
    }

    let resp = client
        .post(&url)
        .header("Authorization", format!("Token {}", cfg.api_key))
        .header("Content-Type", "audio/wav")
        .body(audio)
        .send()
        .await
        .context("Deepgram request failed")?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        anyhow::bail!("Deepgram API error ({}): {}", status, truncate(&body, 400));
    }

    let parsed: DgResponse = serde_json::from_str(&body)
        .with_context(|| format!("Failed to parse Deepgram response: {}", truncate(&body, 300)))?;
    let results = parsed.results.context("Deepgram response missing results")?;

    // Preferisci le utterances (già segmentate).
    if !results.utterances.is_empty() {
        return Ok(results
            .utterances
            .into_iter()
            .filter(|u| !u.transcript.trim().is_empty())
            .map(|u| TranscribedSegment {
                start_ms: (u.start * 1000.0) as i64,
                end_ms: (u.end * 1000.0) as i64,
                text: u.transcript.trim().to_string(),
            })
            .collect());
    }

    // Fallback: ricostruisci dai word-level timestamps.
    if let Some(alt) = results.channels.into_iter().next().and_then(|c| c.alternatives.into_iter().next()) {
        if !alt.words.is_empty() {
            let words: Vec<Word> = alt
                .words
                .into_iter()
                .map(|w| Word {
                    start_ms: (w.start * 1000.0) as i64,
                    end_ms: (w.end * 1000.0) as i64,
                    text: w.punctuated_word.unwrap_or(w.word),
                })
                .collect();
            return Ok(segments_from_words(words, 80));
        }
        let text = alt.transcript.trim().to_string();
        if !text.is_empty() {
            return Ok(vec![TranscribedSegment { start_ms: 0, end_ms: 0, text }]);
        }
    }

    Ok(vec![])
}

// ─── AssemblyAI ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AaiUpload {
    upload_url: String,
}
#[derive(Deserialize)]
struct AaiTranscript {
    id: Option<String>,
    status: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    words: Vec<AaiWord>,
    #[serde(default)]
    error: Option<String>,
}
#[derive(Deserialize)]
struct AaiWord {
    start: i64,
    end: i64,
    text: String,
}

async fn assemblyai(
    client: &reqwest::Client,
    cfg: &CloudConfig,
    audio: Vec<u8>,
) -> Result<Vec<TranscribedSegment>> {
    let base = cfg.base_url();

    // 1) Upload del file audio.
    let up: AaiUpload = client
        .post(format!("{}/upload", base))
        .header("Authorization", &cfg.api_key)
        .header("Content-Type", "application/octet-stream")
        .body(audio)
        .send()
        .await
        .context("AssemblyAI upload failed")?
        .error_for_status()
        .context("AssemblyAI upload returned an error")?
        .json()
        .await
        .context("Failed to parse AssemblyAI upload response")?;

    // 2) Creazione del transcript.
    let mut create = serde_json::json!({
        "audio_url": up.upload_url,
        "punctuate": true,
        "format_text": true,
    });
    match cfg.language.as_ref().filter(|l| l.as_str() != "auto") {
        Some(lang) => create["language_code"] = serde_json::Value::String(lang.clone()),
        None => create["language_detection"] = serde_json::Value::Bool(true),
    }

    let created: AaiTranscript = client
        .post(format!("{}/transcript", base))
        .header("Authorization", &cfg.api_key)
        .json(&create)
        .send()
        .await
        .context("AssemblyAI create transcript failed")?
        .error_for_status()
        .context("AssemblyAI create transcript returned an error")?
        .json()
        .await
        .context("Failed to parse AssemblyAI create response")?;

    let id = created.id.context("AssemblyAI transcript id missing")?;

    // 3) Polling fino a completamento.
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        let poll: AaiTranscript = client
            .get(format!("{}/transcript/{}", base, id))
            .header("Authorization", &cfg.api_key)
            .send()
            .await
            .context("AssemblyAI poll failed")?
            .error_for_status()
            .context("AssemblyAI poll returned an error")?
            .json()
            .await
            .context("Failed to parse AssemblyAI poll response")?;

        match poll.status.as_str() {
            "completed" => {
                if !poll.words.is_empty() {
                    let words: Vec<Word> = poll
                        .words
                        .into_iter()
                        .map(|w| Word { start_ms: w.start, end_ms: w.end, text: w.text })
                        .collect();
                    return Ok(segments_from_words(words, 80));
                }
                let text = poll.text.unwrap_or_default().trim().to_string();
                return Ok(if text.is_empty() {
                    vec![]
                } else {
                    vec![TranscribedSegment { start_ms: 0, end_ms: 0, text }]
                });
            }
            "error" => anyhow::bail!(
                "AssemblyAI transcription error: {}",
                poll.error.unwrap_or_else(|| "unknown".to_string())
            ),
            _ => {} // queued / processing → continua il polling
        }
    }
}

// ─── Word → Segment grouping ───────────────────────────────────────────────────

struct Word {
    start_ms: i64,
    end_ms: i64,
    text: String,
}

/// Raggruppa parole con timestamp in segmenti, spezzando alla punteggiatura di
/// fine frase oppure al superamento di `max_chars`.
fn segments_from_words(words: Vec<Word>, max_chars: usize) -> Vec<TranscribedSegment> {
    let mut segments = Vec::new();
    let mut cur_text = String::new();
    let mut cur_start: Option<i64> = None;
    let mut cur_end = 0i64;

    for w in words {
        let token = w.text.trim();
        if token.is_empty() {
            continue;
        }
        if cur_start.is_none() {
            cur_start = Some(w.start_ms);
        }
        if !cur_text.is_empty() {
            cur_text.push(' ');
        }
        cur_text.push_str(token);
        cur_end = w.end_ms;

        let ends_sentence = token.ends_with('.')
            || token.ends_with('!')
            || token.ends_with('?')
            || token.ends_with('…');

        if ends_sentence || cur_text.len() >= max_chars {
            segments.push(TranscribedSegment {
                start_ms: cur_start.unwrap_or(0),
                end_ms: cur_end,
                text: cur_text.trim().to_string(),
            });
            cur_text.clear();
            cur_start = None;
        }
    }

    if !cur_text.trim().is_empty() {
        segments.push(TranscribedSegment {
            start_ms: cur_start.unwrap_or(0),
            end_ms: cur_end,
            text: cur_text.trim().to_string(),
        });
    }

    segments
}

fn truncate(s: &str, n: usize) -> &str {
    if s.len() <= n {
        s
    } else {
        &s[..s.char_indices().nth(n).map(|(i, _)| i).unwrap_or(s.len())]
    }
}
