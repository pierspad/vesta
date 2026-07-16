//! Motore di refinement batch sopra il pool LLM a tier condiviso.
//!
//! Riusa la stessa politica di scheduling di `srt-translate`
//! ([`TierScheduler`]: round-robin intra-tier, failover inter-tier su
//! rate-limit/quota, budget per entry, rate limiting per RPM) applicandola
//! alle chiamate di arricchimento note delle flashcard.
//!
//! Modalità batch: più card per richiesta con risposta JSON strutturata;
//! le card la cui risposta manca o non è parsabile vengono automaticamente
//! rimesse in coda come richieste singole (stesso fallback della vecchia
//! implementazione GUI, ma qui è headless e riusabile).

use std::collections::VecDeque;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use srt_translate::{
    TierScheduler, TranslatorPool, is_rate_limit_error, pool_concurrency,
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::{RefineCard, interpolate_prompt, strip_html};

/// Configurazione di un run di refinement.
#[derive(Debug, Clone, Deserialize)]
pub struct RefineRunConfig {
    /// Template per-card (placeholder `{{expression}}`, `{{meaning}}`, `{{notes}}`).
    pub prompt: String,
    /// Se `true`, invia le card in batch JSON di `batch_size` per richiesta.
    pub batch_mode: bool,
    /// Card per richiesta in modalità batch (default consigliato: 5).
    pub batch_size: usize,
}

/// Evento di progresso emesso durante il run (contratto serde col frontend).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RefineEvent {
    /// Note generate per una card.
    #[serde(rename_all = "camelCase")]
    CardDone { id: String, notes: String, done: usize, total: usize },
    /// Card fallita definitivamente (errore non di quota).
    #[serde(rename_all = "camelCase")]
    CardFailed { id: String, error: String },
    /// Messaggio informativo (endpoint in uso, fallback, ecc.).
    #[serde(rename_all = "camelCase")]
    Info { message: String },
}

/// Riepilogo del run.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefineRunSummary {
    pub done: usize,
    pub failed: usize,
    /// `true` se il run si è fermato perché ogni tier è esaurito.
    pub pool_exhausted: bool,
    pub cancelled: bool,
}

/// Un job in coda: una card singola o un batch di card.
enum Job {
    Single(RefineCard),
    Batch(Vec<RefineCard>),
}

/// Stato condiviso fra i worker.
struct Shared<F: FnMut(RefineEvent) + Send + 'static> {
    scheduler: Mutex<TierScheduler>,
    queue: Mutex<VecDeque<Job>>,
    on_event: Mutex<F>,
    summary: Mutex<RefineRunSummary>,
    total: usize,
}

/// Esegue il refinement di `cards` usando il pool a tier.
///
/// Per ogni card completata invoca `on_event(RefineEvent::CardDone { .. })`;
/// il chiamante applica gli aggiornamenti (GUI: evento Tauri; CLI: stampa).
/// Ritorna il riepilogo del run.
pub async fn refine_cards_tiered<F>(
    cards: Vec<RefineCard>,
    config: RefineRunConfig,
    pool: TranslatorPool,
    on_event: F,
    token: CancellationToken,
) -> Result<RefineRunSummary, String>
where
    F: FnMut(RefineEvent) + Send + 'static,
{
    if pool.is_empty() || pool.iter().all(|t| t.is_empty()) {
        return Err("Nessun endpoint LLM configurato (pool vuoto)".to_string());
    }
    if cards.is_empty() {
        return Ok(RefineRunSummary::default());
    }

    let total = cards.len();
    let batch_size = config.batch_size.max(1);
    let jobs: VecDeque<Job> = if config.batch_mode && batch_size > 1 {
        cards
            .chunks(batch_size)
            .map(|c| {
                if c.len() == 1 {
                    Job::Single(c[0].clone())
                } else {
                    Job::Batch(c.to_vec())
                }
            })
            .collect()
    } else {
        cards.into_iter().map(Job::Single).collect()
    };

    let concurrency = pool_concurrency(&pool);
    let shared = Arc::new(Shared {
        scheduler: Mutex::new(TierScheduler::new(&pool)),
        queue: Mutex::new(jobs),
        on_event: Mutex::new(on_event),
        summary: Mutex::new(RefineRunSummary::default()),
        total,
    });
    let pool = Arc::new(pool);
    let prompt = Arc::new(config.prompt);

    let workers: Vec<_> = (0..concurrency)
        .map(|_| {
            let shared = shared.clone();
            let pool = pool.clone();
            let prompt = prompt.clone();
            let token = token.clone();
            tokio::spawn(worker_loop(shared, pool, prompt, token))
        })
        .collect();

    for w in workers {
        let _ = w.await;
    }

    let mut summary = std::mem::take(&mut *shared.summary.lock().await);
    summary.cancelled = token.is_cancelled();
    Ok(summary)
}

async fn worker_loop<F>(
    shared: Arc<Shared<F>>,
    pool: Arc<TranslatorPool>,
    prompt: Arc<String>,
    token: CancellationToken,
) where
    F: FnMut(RefineEvent) + Send + 'static,
{
    loop {
        if token.is_cancelled() {
            return;
        }
        let Some(job) = shared.queue.lock().await.pop_front() else {
            return;
        };

        // Failover: prova entry diverse finché il job non riesce o il pool è esaurito.
        loop {
            if token.is_cancelled() {
                return;
            }

            let acquired = { shared.scheduler.lock().await.acquire() };
            let Some((ti, ei)) = acquired else {
                shared.summary.lock().await.pool_exhausted = true;
                shared.queue.lock().await.push_front(job);
                return;
            };
            let entry = pool[ti][ei].clone();

            if let Some(ref limiter) = entry.rate_limiter {
                tokio::select! {
                    _ = token.cancelled() => return,
                    _ = limiter.until_ready() => {}
                }
            }

            let request_prompt = match &job {
                Job::Single(card) => interpolate_prompt(&prompt, card),
                Job::Batch(cards) => build_batch_prompt(&prompt, cards),
            };

            let result = entry.translator.generate_response(&request_prompt).await;
            if token.is_cancelled() {
                return;
            }

            match result {
                Ok(response) => {
                    match &job {
                        Job::Single(card) => {
                            let notes = response.trim().to_string();
                            let done = {
                                let mut s = shared.summary.lock().await;
                                s.done += 1;
                                s.done
                            };
                            emit(&shared, RefineEvent::CardDone {
                                id: card.id.clone(),
                                notes,
                                done,
                                total: shared.total,
                            })
                            .await;
                        }
                        Job::Batch(cards) => {
                            let missing = apply_batch_response(&shared, cards, &response).await;
                            if !missing.is_empty() {
                                emit(&shared, RefineEvent::Info {
                                    message: format!(
                                        "Batch incompleto da {}: {} card rimesse in coda singolarmente",
                                        entry.label,
                                        missing.len()
                                    ),
                                })
                                .await;
                                let mut queue = shared.queue.lock().await;
                                for card in missing {
                                    queue.push_back(Job::Single(card));
                                }
                            }
                        }
                    }
                    break; // job gestito, passa al prossimo
                }
                Err(e) if is_rate_limit_error(&e) => {
                    // Entry esaurita: marca e riprova lo stesso job con la prossima.
                    shared.scheduler.lock().await.report_exhausted(ti, ei);
                    emit(&shared, RefineEvent::Info {
                        message: format!("{}: rate limit/quota raggiunti, failover", entry.label),
                    })
                    .await;
                }
                Err(e) => match &job {
                    Job::Single(card) => {
                        shared.summary.lock().await.failed += 1;
                        emit(&shared, RefineEvent::CardFailed {
                            id: card.id.clone(),
                            error: e.to_string(),
                        })
                        .await;
                        break;
                    }
                    Job::Batch(cards) => {
                        // Stesso fallback della vecchia GUI: il batch fallito
                        // viene riprocessato una card alla volta.
                        emit(&shared, RefineEvent::Info {
                            message: format!(
                                "Batch fallito via {} ({e}), fallback a richieste singole",
                                entry.label
                            ),
                        })
                        .await;
                        let mut queue = shared.queue.lock().await;
                        for card in cards.clone() {
                            queue.push_back(Job::Single(card));
                        }
                        break;
                    }
                },
            }
        }
    }
}

async fn emit<F>(shared: &Shared<F>, event: RefineEvent)
where
    F: FnMut(RefineEvent) + Send + 'static,
{
    (shared.on_event.lock().await)(event);
}

/// Costruisce il prompt batch: istruzione per-card + lista JSON delle card,
/// con risposta attesa in JSON `{"results": [{"id", "notes"}]}`.
fn build_batch_prompt(per_card_prompt: &str, cards: &[RefineCard]) -> String {
    #[derive(Serialize)]
    struct BatchCard<'a> {
        id: &'a str,
        expression: String,
        meaning: String,
    }
    let payload: Vec<BatchCard<'_>> = cards
        .iter()
        .map(|c| BatchCard {
            id: &c.id,
            expression: strip_html(&c.expression),
            meaning: strip_html(&c.meaning),
        })
        .collect();
    let json = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "[]".to_string());

    format!(
        r#"You are an AI assistant specialised in enriching and refining language-learning flashcards.
You will receive a list of flashcards in JSON format.
Your task is to generate detailed notes for EACH flashcard, following this instruction SCRUPULOUSLY:
"{per_card_prompt}"

---
FLASHCARDS TO PROCESS (JSON):
{json}
---

Reply EXCLUSIVELY with a valid JSON object structured exactly like the following example, without comments or extra explanations outside the JSON. Do not wrap the answer in markdown code fences (no ```json ... ```), return the raw JSON text only.

Expected response format:
{{
  "results": [
    {{
      "id": "id_from_list",
      "notes": "generated explanation/notes..."
    }}
  ]
}}
"#
    )
}

/// Applica una risposta batch: aggiorna le card trovate ed emette gli eventi.
/// Ritorna le card senza risposta (da riprocessare singolarmente).
async fn apply_batch_response<F>(
    shared: &Shared<F>,
    cards: &[RefineCard],
    response: &str,
) -> Vec<RefineCard>
where
    F: FnMut(RefineEvent) + Send + 'static,
{
    #[derive(Deserialize)]
    struct BatchResult {
        id: String,
        #[serde(default)]
        notes: String,
    }
    #[derive(Deserialize)]
    struct BatchResponse {
        results: Vec<BatchResult>,
    }

    let cleaned = strip_code_fences(response);
    let Ok(parsed) = serde_json::from_str::<BatchResponse>(cleaned) else {
        return cards.to_vec(); // risposta non parsabile: riprocessa tutto
    };

    let mut missing: Vec<RefineCard> = Vec::new();
    for card in cards {
        match parsed.results.iter().find(|r| r.id == card.id) {
            Some(r) if !r.notes.trim().is_empty() => {
                let done = {
                    let mut s = shared.summary.lock().await;
                    s.done += 1;
                    s.done
                };
                emit(shared, RefineEvent::CardDone {
                    id: card.id.clone(),
                    notes: r.notes.trim().to_string(),
                    done,
                    total: shared.total,
                })
                .await;
            }
            _ => missing.push(card.clone()),
        }
    }
    missing
}

/// Rimuove eventuali code fence markdown attorno a una risposta JSON.
fn strip_code_fences(response: &str) -> &str {
    let trimmed = response.trim();
    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed;
    };
    // Salta l'eventuale language tag sulla prima riga.
    let rest = rest.split_once('\n').map_or(rest, |(_, body)| body);
    rest.strip_suffix("```").unwrap_or(rest).trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_code_fences_handles_plain_and_fenced() {
        assert_eq!(strip_code_fences(r#"{"a":1}"#), r#"{"a":1}"#);
        assert_eq!(strip_code_fences("```json\n{\"a\":1}\n```"), r#"{"a":1}"#);
        assert_eq!(strip_code_fences("```\n{\"a\":1}\n```"), r#"{"a":1}"#);
    }

    #[test]
    fn strip_html_removes_tags_only() {
        assert_eq!(strip_html("plain"), "plain");
        assert_eq!(strip_html("a <b>bold</b> word"), "a bold word");
        assert_eq!(strip_html(r#"<img src="x.jpg">hi"#), "hi");
    }

    #[test]
    fn batch_prompt_contains_cards_and_instruction() {
        let cards = vec![RefineCard {
            id: "42".into(),
            expression: "<b>Hallo</b>".into(),
            meaning: "Ciao".into(),
            notes: String::new(),
        }];
        let p = build_batch_prompt("Explain the grammar", &cards);
        assert!(p.contains("Explain the grammar"));
        assert!(p.contains("\"expression\": \"Hallo\"")); // HTML stripped
        assert!(p.contains("\"id\": \"42\""));
    }
}
