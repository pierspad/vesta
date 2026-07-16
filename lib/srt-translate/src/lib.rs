mod language_info;
pub mod pool;
mod prompts;
mod rate_limiter;
mod translator;

use anyhow::Result;
use srt_parser::{SrtParser, Subtitle};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

pub use pool::{
    TierEntry, build_pool, build_pool_entry, provider_allows_missing_key, provider_defaults,
};
pub use rate_limiter::{
    RateLimitConfig, RateLimiter, create_rate_limiter, create_rate_limiter_with_burst,
};
pub use translator::{ApiType, Translator, TranslatorConfig};

#[derive(Debug, Clone)]
pub struct TranslationProgress {
    pub message: String,

    pub eta_seconds: Option<f64>,

    pub current_batch: usize,

    pub total_batches: usize,

    pub batch_start: usize,

    pub batch_end: usize,
}

#[derive(Debug, Clone)]
pub struct BatchResult {
    pub success: bool,

    pub batch_number: usize,

    pub error: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub async fn translate_subtitles_async<F>(
    translators: Vec<Translator>,
    subtitles: HashMap<u32, Subtitle>,
    target_lang: &str,
    batch_size: usize,
    resume_overlap: usize,
    title_context: Option<&str>,
    output_path: &std::path::Path,
    on_progress: F,
) -> Result<HashMap<u32, Subtitle>>
where
    F: FnMut(TranslationProgress) + Send + 'static,
{
    // Delega alla nuova funzione senza rate limiters (usa solo semaforo)
    translate_subtitles_with_rate_limit(
        translators,
        None, // Nessun rate limiter, usa solo semaforo
        subtitles,
        target_lang,
        batch_size,
        resume_overlap,
        title_context,
        output_path,
        on_progress,
    )
    .await
}

/// Traduce tutti i sottotitoli usando multiple API keys in parallelo con rate limiting RPM
///
/// Questa versione implementa un vero rate limiter basato su token bucket che rispetta
/// i limiti RPM (Richieste Per Minuto) delle API, non solo la concorrenza.
///
/// # Argomenti
///
/// * `translators` - Vector di Translator pre-configurati (uno per API provider)
/// * `rate_limiters` - Optional: Vector di RateLimiter (uno per provider), se None usa solo semaforo
/// * `subtitles` - HashMap dei sottotitoli da tradurre
/// * `target_lang` - Codice lingua target (es: "it", "en", "es")
/// * `batch_size` - Numero di sottotitoli da tradurre insieme
/// * `title_context` - Contesto opzionale (es: titolo del film)
/// * `output_path` - Percorso del file di output per salvataggio incrementale
/// * `on_progress` - Callback invocato ad ogni aggiornamento di progresso
#[allow(clippy::too_many_arguments)]
pub async fn translate_subtitles_with_rate_limit<F>(
    translators: Vec<Translator>,
    rate_limiters: Option<Vec<std::sync::Arc<RateLimiter>>>,
    subtitles: HashMap<u32, Subtitle>,
    target_lang: &str,
    batch_size: usize,
    resume_overlap: usize,
    title_context: Option<&str>,
    output_path: &std::path::Path,
    on_progress: F,
) -> Result<HashMap<u32, Subtitle>>
where
    F: FnMut(TranslationProgress) + Send + 'static,
{
    let cancellation_token = CancellationToken::new();
    translate_subtitles_with_rate_limit_cancellable(
        translators,
        rate_limiters,
        subtitles,
        target_lang,
        batch_size,
        resume_overlap,
        title_context,
        output_path,
        on_progress,
        cancellation_token,
    )
    .await
}

pub fn verify_translation_completeness(
    original: &HashMap<u32, Subtitle>,
    translated: &HashMap<u32, Subtitle>,
) -> Vec<u32> {
    get_missing_or_incorrect_subtitle_ids(original, translated)
}

pub fn get_missing_or_incorrect_subtitle_ids(
    original: &HashMap<u32, Subtitle>,
    translated: &HashMap<u32, Subtitle>,
) -> Vec<u32> {
    original
        .iter()
        .filter(|(id, original_sub)| {
            if !translated.contains_key(id) {
                return true;
            }

            if let Some(translated_sub) = translated.get(id) {
                let original_lines = original_sub.text.lines().count();
                let translated_lines = translated_sub.text.lines().count();
                return original_lines != translated_lines;
            }

            false
        })
        .map(|(id, _)| *id)
        .collect()
}

#[deprecated(note = "Use get_missing_or_incorrect_subtitle_ids instead")]
pub fn get_missing_subtitle_ids(
    original: &HashMap<u32, Subtitle>,
    translated: &HashMap<u32, Subtitle>,
) -> Vec<u32> {
    original
        .keys()
        .filter(|id| !translated.contains_key(id))
        .copied()
        .collect()
}

pub async fn repair_translation<F>(
    translators: Vec<Translator>,
    original: &HashMap<u32, Subtitle>,
    translated: &mut HashMap<u32, Subtitle>,
    missing_ids: Vec<u32>,
    target_lang: &str,
    title_context: Option<&str>,
    on_progress: F,
) -> Result<()>
where
    F: FnMut(TranslationProgress) + Send + 'static,
{
    // Delega alla versione con rate limiter senza rate limiters
    repair_translation_with_rate_limit(
        translators,
        None,
        original,
        translated,
        missing_ids,
        target_lang,
        title_context,
        on_progress,
    )
    .await
}

/// Ripara una traduzione incompleta con supporto per rate limiting RPM
///
/// Versione avanzata che supporta rate limiters per rispettare i limiti RPM delle API.
#[allow(clippy::too_many_arguments)]
pub async fn repair_translation_with_rate_limit<F>(
    translators: Vec<Translator>,
    rate_limiters: Option<Vec<std::sync::Arc<RateLimiter>>>,
    original: &HashMap<u32, Subtitle>,
    translated: &mut HashMap<u32, Subtitle>,
    missing_ids: Vec<u32>,
    target_lang: &str,
    title_context: Option<&str>,
    on_progress: F,
) -> Result<()>
where
    F: FnMut(TranslationProgress) + Send + 'static,
{
    use std::sync::Arc;
    use tokio::sync::Mutex;

    if missing_ids.is_empty() {
        return Ok(());
    }

    let total = missing_ids.len();
    let translators_len = translators.len();

    let progress_callback = Arc::new(Mutex::new(on_progress));
    let repaired = Arc::new(Mutex::new(HashMap::new()));

    let semaphore = Arc::new(Semaphore::new(translators_len));

    let rate_limiters: Option<Vec<Arc<RateLimiter>>> = rate_limiters;

    {
        let mut callback = progress_callback.lock().await;
        callback(TranslationProgress {
            message: format!(
                "Found {} missing subtitles, repairing with {} workers...",
                total, translators_len
            ),
            eta_seconds: None,
            current_batch: 0,
            total_batches: total,
            batch_start: 0,
            batch_end: 0,
        });
    }

    let timing_stats = Arc::new(Mutex::new((0.0_f64, 0_usize)));
    let start_time = Instant::now();

    let mut handles = vec![];

    for (idx, id) in missing_ids.iter().enumerate() {
        if let Some(subtitle) = original.get(id) {
            let translator_idx = idx % translators_len;
            let translator = translators[translator_idx].clone();

            let semaphore = semaphore.clone();

            let rate_limiter = rate_limiters
                .as_ref()
                .map(|limiters| limiters[translator_idx % limiters.len()].clone());
            let id = *id;
            let subtitle = subtitle.clone();
            let target_lang = target_lang.to_string();
            let title_context = title_context.map(|s| s.to_string());
            let progress_callback = progress_callback.clone();
            let repaired = repaired.clone();
            let timing_stats = timing_stats.clone();

            let context_text = build_repair_context(id, original, translated);

            let handle = tokio::spawn(async move {
                if let Some(ref limiter) = rate_limiter {
                    limiter.until_ready().await;
                }

                let _permit = semaphore
                    .acquire()
                    .await
                    .expect("Semaphore should never be closed during repair");

                let task_start = Instant::now();

                let eta = {
                    let stats = timing_stats.lock().await;
                    let (total_duration, completed) = *stats;
                    if completed > 0 {
                        let avg_duration = total_duration / completed as f64;
                        let remaining = total.saturating_sub(completed);
                        Some(avg_duration * remaining as f64)
                    } else {
                        None
                    }
                };

                {
                    let completed = timing_stats.lock().await.1;
                    let mut callback = progress_callback.lock().await;
                    callback(TranslationProgress {
                        message: format!(
                            "Repairing subtitle {} ({}/{}) [worker {}]",
                            id,
                            idx + 1,
                            total,
                            translator_idx + 1
                        ),
                        eta_seconds: eta,
                        current_batch: completed,
                        total_batches: total,
                        batch_start: idx + 1,
                        batch_end: total,
                    });
                }

                match translator
                    .translate_with_context(
                        &subtitle.text,
                        &target_lang,
                        title_context.as_deref(),
                        context_text.as_deref(),
                    )
                    .await
                {
                    Ok(translation) => {
                        let mut new_subtitle = subtitle.clone();
                        new_subtitle.text = translation;
                        repaired.lock().await.insert(id, new_subtitle);

                        let duration = task_start.elapsed().as_secs_f64();
                        let mut stats = timing_stats.lock().await;
                        stats.0 += duration;
                        stats.1 += 1;
                    }
                    Err(e) => {
                        let completed = timing_stats.lock().await.1;
                        let mut callback = progress_callback.lock().await;
                        callback(TranslationProgress {
                            message: format!("Failed to repair subtitle {}: {}", id, e),
                            eta_seconds: None,
                            current_batch: completed,
                            total_batches: total,
                            batch_start: idx + 1,
                            batch_end: total,
                        });

                        repaired.lock().await.insert(id, subtitle.clone());
                    }
                }
            });

            handles.push(handle);
        }
    }

    for handle in handles {
        let _ = handle.await;
    }

    let repaired_subs = repaired.lock().await;
    for (id, subtitle) in repaired_subs.iter() {
        translated.insert(*id, subtitle.clone());
    }

    let total_time = start_time.elapsed().as_secs_f64();
    let mut callback = progress_callback.lock().await;
    callback(TranslationProgress {
        message: format!(
            "Repair completed! Fixed {} subtitles in {:.1}s ✓",
            total, total_time
        ),
        eta_seconds: Some(0.0),
        current_batch: total,
        total_batches: total,
        batch_start: total,
        batch_end: total,
    });

    Ok(())
}

fn build_repair_context(
    missing_id: u32,
    original: &HashMap<u32, Subtitle>,
    translated: &HashMap<u32, Subtitle>,
) -> Option<String> {
    let mut context_parts = Vec::new();

    for offset in (1..=2).rev() {
        if let Some(prev_id) = missing_id.checked_sub(offset)
            && let (Some(orig), Some(trans)) = (original.get(&prev_id), translated.get(&prev_id))
        {
            context_parts.push(format!(
                "[{}] Original: {}\nTranslated: {}",
                prev_id, orig.text, trans.text
            ));
        }
    }

    for offset in 1..=2 {
        let next_id = missing_id + offset;
        if let (Some(orig), Some(trans)) = (original.get(&next_id), translated.get(&next_id)) {
            context_parts.push(format!(
                "[{}] Original: {}\nTranslated: {}",
                next_id, orig.text, trans.text
            ));
        }
    }

    if context_parts.is_empty() {
        None
    } else {
        Some(format!(
            "Context from surrounding subtitles:\n\n{}",
            context_parts.join("\n\n")
        ))
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn translate_subtitles_with_rate_limit_cancellable<F>(
    translators: Vec<Translator>,
    rate_limiters: Option<Vec<std::sync::Arc<RateLimiter>>>,
    subtitles: HashMap<u32, Subtitle>,
    target_lang: &str,
    batch_size: usize,
    resume_overlap: usize,
    title_context: Option<&str>,
    output_path: &std::path::Path,
    on_progress: F,
    cancellation_token: CancellationToken,
) -> Result<HashMap<u32, Subtitle>>
where
    F: FnMut(TranslationProgress) + Send + 'static,
{
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let total = subtitles.len();
    let total_batches = total.div_ceil(batch_size);

    // Wrapper thread-safe per il callback
    let progress_callback = Arc::new(Mutex::new(on_progress));

    // Risultati condivisi
    let translated = Arc::new(Mutex::new(HashMap::new()));

    // Timing stats condivisi
    let timing_stats = Arc::new(Mutex::new((0.0_f64, 0_usize)));

    // Ordina sottotitoli per ID
    let mut sorted: Vec<_> = subtitles.into_iter().collect();
    sorted.sort_by_key(|(id, _)| *id);
    let subtitles_map: HashMap<u32, Subtitle> = sorted.iter().cloned().collect();

    // Controlla se esiste un file di output e determina da dove riprendere
    // (stessa politica della versione tiered: prefisso completo → salta i batch
    // fatti; pochi buchi → solo repair mirato; molti buchi → ritraduzione in
    // batch con numerazione corretta).
    let (skip_batches, start_idx) = if output_path.exists() {
        match SrtParser::parse_file(output_path) {
            Ok(existing_translations) => {
                let existing_count = existing_translations.len();
                if existing_count > 0 {
                    *translated.lock().await = existing_translations.clone();
                    let missing_count = get_missing_or_incorrect_subtitle_ids(
                        &subtitles_map,
                        &existing_translations,
                    )
                    .len();
                    if missing_count == 0 {
                        let calc_start_idx = existing_count.saturating_sub(resume_overlap);
                        let skip_b = calc_start_idx / batch_size;
                        (skip_b, calc_start_idx)
                    } else if missing_count <= batch_size * 2 {
                        (0, sorted.len())
                    } else {
                        (0, 0)
                    }
                } else {
                    (0, 0)
                }
            }
            Err(_) => (0, 0),
        }
    } else {
        (0, 0)
    };

    // Prepara i batch da processare
    let remaining = if start_idx < sorted.len() {
        &sorted[start_idx..]
    } else {
        &[]
    };
    let batches_to_process: Vec<_> = remaining
        .chunks(batch_size)
        .enumerate()
        .map(|(idx, chunk)| (idx + skip_batches, chunk.to_vec()))
        .collect();

    let total_workers = translators.len();
    let semaphore = Arc::new(Semaphore::new(total_workers));
    let rate_limiters: Option<Vec<Arc<RateLimiter>>> = rate_limiters;

    let mut handles = vec![];

    for (batch_idx, batch_data) in batches_to_process {
        // Controlla cancellazione prima di iniziare un nuovo batch
        if cancellation_token.is_cancelled() {
            break;
        }

        let translator_idx = batch_idx % translators.len();
        let translator = translators[translator_idx].clone();

        let semaphore = semaphore.clone();
        let rate_limiter = rate_limiters
            .as_ref()
            .map(|limiters| limiters[translator_idx % limiters.len()].clone());
        let translated = translated.clone();
        let progress_callback = progress_callback.clone();
        let timing_stats = timing_stats.clone();
        let output_path = output_path.to_path_buf();
        let target_lang = target_lang.to_string();
        let title_context = title_context.map(|s| s.to_string());
        let token = cancellation_token.clone();

        let handle = tokio::spawn(async move {
            // Controlla cancellazione
            if token.is_cancelled() {
                return;
            }

            // Rate limiting
            if let Some(ref limiter) = rate_limiter {
                tokio::select! {
                    _ = token.cancelled() => return,
                    _ = limiter.until_ready() => {}
                }
            }

            // Acquisisci permit
            let _permit = match semaphore.acquire().await {
                Ok(p) => p,
                Err(_) => return,
            };

            // Controlla cancellazione dopo il permit
            if token.is_cancelled() {
                return;
            }

            let batch_start_time = Instant::now();
            let batch_start = batch_idx * batch_size + 1;
            let batch_end = (batch_start + batch_data.len() - 1).min(total);

            let eta = {
                let stats = timing_stats.lock().await;
                let (total_duration, completed) = *stats;
                if completed > 0 {
                    let avg_duration = total_duration / completed as f64;
                    let remaining = total_batches.saturating_sub(completed);
                    Some(avg_duration * remaining as f64)
                } else {
                    None
                }
            };

            {
                let completed = timing_stats.lock().await.1;
                let mut callback = progress_callback.lock().await;
                callback(TranslationProgress {
                    message: format!(
                        "Starting batch [{}-{}]/{} (worker {})...",
                        batch_start,
                        batch_end,
                        total,
                        translator_idx + 1
                    ),
                    eta_seconds: eta,
                    current_batch: completed,
                    total_batches,
                    batch_start,
                    batch_end,
                });
            }

            let texts_with_ids: Vec<(u32, String)> = batch_data
                .iter()
                .map(|(id, subtitle)| (*id, subtitle.text.clone()))
                .collect();

            let result = translator
                .translate_batch(&texts_with_ids, &target_lang, title_context.as_deref())
                .await;

            // Controlla cancellazione dopo la traduzione
            if token.is_cancelled() {
                return;
            }

            match result {
                Ok(translations) => {
                    let completed_after = timing_stats.lock().await.1 + 1;
                    {
                        let mut callback = progress_callback.lock().await;
                        callback(TranslationProgress {
                            message: format!("Batch [{}-{}] completed ✓", batch_start, batch_end),
                            eta_seconds: eta,
                            current_batch: completed_after,
                            total_batches,
                            batch_start,
                            batch_end,
                        });
                    }

                    {
                        let mut trans_map = translated.lock().await;
                        for (id, subtitle) in &batch_data {
                            if let Some(translation) = translations.get(id) {
                                let mut new_subtitle = subtitle.clone();
                                new_subtitle.text = translation.clone();
                                trans_map.insert(*id, new_subtitle);
                            } else {
                                trans_map.insert(*id, subtitle.clone());
                            }
                        }
                    }

                    let batch_duration = batch_start_time.elapsed().as_secs_f64();
                    {
                        let mut stats = timing_stats.lock().await;
                        stats.0 += batch_duration;
                        stats.1 += 1;
                    }

                    {
                        let trans_map = translated.lock().await;
                        let _ = SrtParser::save_file(&output_path, &trans_map);
                    }
                }
                Err(e) => {
                    let completed = timing_stats.lock().await.1;
                    let mut callback = progress_callback.lock().await;
                    callback(TranslationProgress {
                        message: format!("Batch [{}-{}] error: {} ✗", batch_start, batch_end, e),
                        eta_seconds: None,
                        current_batch: completed,
                        total_batches,
                        batch_start,
                        batch_end,
                    });
                }
            }
        });

        handles.push(handle);
    }

    // Attendi completamento o cancellazione
    for handle in handles {
        let _ = handle.await;
    }

    // Controlla se è stato cancellato
    if cancellation_token.is_cancelled() {
        anyhow::bail!("Translation cancelled by user");
    }

    // Verifica integrità e repair
    let trans_map = translated.lock().await;
    let missing_ids = get_missing_or_incorrect_subtitle_ids(&subtitles_map, &trans_map);
    drop(trans_map);

    if !missing_ids.is_empty() && !cancellation_token.is_cancelled() {
        let mut callback = progress_callback.lock().await;
        callback(TranslationProgress {
            message: format!(
                "Repairing {} missing/incorrect subtitles...",
                missing_ids.len()
            ),
            eta_seconds: None,
            current_batch: total_batches,
            total_batches,
            batch_start: 0,
            batch_end: 0,
        });
        drop(callback);

        repair_missing_subtitles_cancellable(
            &translators[0],
            &missing_ids,
            &subtitles_map,
            &translated,
            target_lang,
            title_context,
            output_path,
            progress_callback.clone(),
            &cancellation_token,
        )
        .await?;
    }

    let result = translated.lock().await.clone();
    Ok(result)
}

/// Ripara i sottotitoli mancanti con supporto cancellazione
#[allow(clippy::too_many_arguments)]
async fn repair_missing_subtitles_cancellable(
    translator: &Translator,
    missing_ids: &[u32],
    original_subtitles: &HashMap<u32, Subtitle>,
    translated: &Arc<tokio::sync::Mutex<HashMap<u32, Subtitle>>>,
    target_lang: &str,
    title_context: Option<&str>,
    output_path: &std::path::Path,
    progress_callback: Arc<tokio::sync::Mutex<impl FnMut(TranslationProgress) + Send>>,
    cancellation_token: &CancellationToken,
) -> Result<()> {
    let total = missing_ids.len();

    for (idx, &id) in missing_ids.iter().enumerate() {
        if cancellation_token.is_cancelled() {
            anyhow::bail!("Repair cancelled by user");
        }

        if let Some(original) = original_subtitles.get(&id) {
            let trans_map = translated.lock().await;
            let context = build_repair_context(id, original_subtitles, &trans_map);
            drop(trans_map);

            let result = translator
                .translate_with_context(
                    &original.text,
                    target_lang,
                    title_context,
                    context.as_deref(),
                )
                .await;

            match result {
                Ok(translation) => {
                    let mut new_subtitle = original.clone();
                    new_subtitle.text = translation;

                    let mut trans_map = translated.lock().await;
                    trans_map.insert(id, new_subtitle);
                    let _ = SrtParser::save_file(output_path, &trans_map);
                }
                Err(e) => {
                    let mut callback = progress_callback.lock().await;
                    callback(TranslationProgress {
                        message: format!("Repair failed for subtitle {}: {}", id, e),
                        eta_seconds: None,
                        current_batch: idx,
                        total_batches: total,
                        batch_start: id as usize,
                        batch_end: id as usize,
                    });
                }
            }
        }
    }

    Ok(())
}

// =====================================================================================
//  TIERED SCHEDULER
//
//  Un "pool" è una lista ordinata di tier. Ogni tier contiene una o più `PoolEntry`
//  (provider + modello + API key, ciascuna con il proprio rate limiter e budget).
//
//  Politica di esecuzione:
//   • All'interno di un tier le entry vengono usate in round-robin (carico bilanciato).

#[derive(Clone)]
pub struct PoolEntry {
    pub translator: Translator,

    pub rate_limiter: Option<Arc<RateLimiter>>,

    pub max_requests: Option<u32>,

    pub label: String,
}

pub type TranslatorPool = Vec<Vec<PoolEntry>>;

struct EntryRuntime {
    exhausted: bool,
    remaining: Option<u32>,
}

struct TierRuntime {
    entries: Vec<EntryRuntime>,
    cursor: usize,
}

pub struct TierScheduler {
    tiers: Vec<TierRuntime>,
    active: usize,
}

impl TierScheduler {
    pub fn new(pool: &TranslatorPool) -> Self {
        let tiers = pool
            .iter()
            .map(|entries| TierRuntime {
                entries: entries
                    .iter()
                    .map(|e| EntryRuntime {
                        exhausted: false,
                        remaining: e.max_requests,
                    })
                    .collect(),
                cursor: 0,
            })
            .collect();
        Self { tiers, active: 0 }
    }

    pub fn acquire(&mut self) -> Option<(usize, usize)> {
        while self.active < self.tiers.len() {
            let active = self.active;
            let tier = &mut self.tiers[active];
            let n = tier.entries.len();
            if n > 0 {
                for off in 0..n {
                    let i = (tier.cursor + off) % n;
                    let entry = &mut tier.entries[i];
                    if entry.exhausted {
                        continue;
                    }
                    if let Some(0) = entry.remaining {
                        entry.exhausted = true;
                        continue;
                    }
                    if let Some(r) = entry.remaining.as_mut() {
                        *r -= 1;
                    }
                    tier.cursor = (i + 1) % n;
                    return Some((active, i));
                }
            }

            self.active += 1;
        }
        None
    }

    pub fn report_exhausted(&mut self, tier: usize, idx: usize) {
        if let Some(t) = self.tiers.get_mut(tier)
            && let Some(e) = t.entries.get_mut(idx)
        {
            e.exhausted = true;
        }
    }

    pub fn active_tier_human(&self) -> usize {
        self.active + 1
    }
}

pub fn is_rate_limit_error(error: &anyhow::Error) -> bool {
    let s = error.to_string().to_lowercase();
    s.contains("429")
        || s.contains("rate limit")
        || s.contains("rate-limit")
        || s.contains("quota")
        || s.contains("resource_exhausted")
        || s.contains("resource exhausted")
        || s.contains("too many requests")
        || s.contains("limit exceeded")
        || s.contains("insufficient_quota")
}

pub fn pool_concurrency(pool: &TranslatorPool) -> usize {
    pool.iter().map(|t| t.len()).max().unwrap_or(1).clamp(1, 16)
}

#[allow(clippy::too_many_arguments)]
pub async fn translate_subtitles_tiered_cancellable<F>(
    pool: TranslatorPool,
    subtitles: HashMap<u32, Subtitle>,
    target_lang: &str,
    batch_size: usize,
    resume_overlap: usize,
    title_context: Option<&str>,
    output_path: &std::path::Path,
    on_progress: F,
    cancellation_token: CancellationToken,
) -> Result<HashMap<u32, Subtitle>>
where
    F: FnMut(TranslationProgress) + Send + 'static,
{
    use tokio::sync::Mutex;

    if pool.is_empty() || pool.iter().all(|t| t.is_empty()) {
        anyhow::bail!("No translation endpoints configured (empty pool)");
    }

    let total = subtitles.len();
    let total_batches = total.div_ceil(batch_size);

    let progress_callback = Arc::new(Mutex::new(on_progress));
    let translated = Arc::new(Mutex::new(HashMap::new()));
    let timing_stats = Arc::new(Mutex::new((0.0_f64, 0_usize)));

    // Ordina i sottotitoli per ID.
    let mut sorted: Vec<_> = subtitles.into_iter().collect();
    sorted.sort_by_key(|(id, _)| *id);
    let subtitles_map: HashMap<u32, Subtitle> = sorted.iter().cloned().collect();

    // Ripresa: se esiste già un output, riparti dal punto giusto.
    //  • prefisso completo           → salta i batch già fatti (con overlap)
    //  • pochi buchi sparsi          → salta la fase batch: il repair mirato
    //                                  (uno a uno, con contesto) li sistema
    //  • molti buchi                 → ritraduci in batch dall'inizio con

    let (skip_batches, start_idx) = if output_path.exists() {
        match SrtParser::parse_file(output_path) {
            Ok(existing) => {
                let existing_count = existing.len();
                if existing_count > 0 {
                    *translated.lock().await = existing.clone();
                    let missing =
                        get_missing_or_incorrect_subtitle_ids(&subtitles_map, &existing).len();
                    if missing == 0 {
                        let calc_start_idx = existing_count.saturating_sub(resume_overlap);
                        (calc_start_idx / batch_size, calc_start_idx)
                    } else if missing <= batch_size * 2 {
                        (0, sorted.len())
                    } else {
                        (0, 0)
                    }
                } else {
                    (0, 0)
                }
            }
            Err(_) => (0, 0),
        }
    } else {
        (0, 0)
    };

    let remaining = if start_idx < sorted.len() {
        &sorted[start_idx..]
    } else {
        &[]
    };
    let batches_to_process: VecDeque<(usize, Vec<(u32, Subtitle)>)> = remaining
        .chunks(batch_size)
        .enumerate()
        .map(|(idx, chunk)| (idx + skip_batches, chunk.to_vec()))
        .collect();

    let pool = Arc::new(pool);
    let scheduler = Arc::new(Mutex::new(TierScheduler::new(&pool)));
    let queue = Arc::new(Mutex::new(batches_to_process));
    let exhausted_flag = Arc::new(Mutex::new(false));
    let concurrency = pool_concurrency(&pool);

    let mut workers = Vec::new();

    for _ in 0..concurrency {
        let pool = pool.clone();
        let scheduler = scheduler.clone();
        let queue = queue.clone();
        let translated = translated.clone();
        let progress_callback = progress_callback.clone();
        let timing_stats = timing_stats.clone();
        let exhausted_flag = exhausted_flag.clone();
        let output_path = output_path.to_path_buf();
        let target_lang = target_lang.to_string();
        let title_context = title_context.map(|s| s.to_string());
        let token = cancellation_token.clone();

        let worker = tokio::spawn(async move {
            loop {
                if token.is_cancelled() {
                    return;
                }

                let next = { queue.lock().await.pop_front() };
                let Some((batch_idx, batch_data)) = next else {
                    return;
                };

                let batch_start = batch_idx * batch_size + 1;
                let batch_end = (batch_start + batch_data.len() - 1).min(total);

                let texts_with_ids: Vec<(u32, String)> = batch_data
                    .iter()
                    .map(|(id, s)| (*id, s.text.clone()))
                    .collect();

                loop {
                    if token.is_cancelled() {
                        return;
                    }

                    let acquired = { scheduler.lock().await.acquire() };
                    let Some((ti, ei)) = acquired else {
                        *exhausted_flag.lock().await = true;
                        queue
                            .lock()
                            .await
                            .push_front((batch_idx, batch_data.clone()));
                        return;
                    };

                    let entry = pool[ti][ei].clone();

                    if let Some(ref limiter) = entry.rate_limiter {
                        tokio::select! {
                            _ = token.cancelled() => return,
                            _ = limiter.until_ready() => {}
                        }
                    }

                    let eta = {
                        let (total_dur, completed) = *timing_stats.lock().await;
                        if completed > 0 {
                            let avg = total_dur / completed as f64;
                            Some(avg * total_batches.saturating_sub(completed) as f64)
                        } else {
                            None
                        }
                    };

                    {
                        let completed = timing_stats.lock().await.1;
                        let mut cb = progress_callback.lock().await;
                        cb(TranslationProgress {
                            message: format!(
                                "Batch [{}-{}]/{} via {}...",
                                batch_start, batch_end, total, entry.label
                            ),
                            eta_seconds: eta,
                            current_batch: completed,
                            total_batches,
                            batch_start,
                            batch_end,
                        });
                    }

                    let batch_start_time = Instant::now();
                    let result = entry
                        .translator
                        .translate_batch(&texts_with_ids, &target_lang, title_context.as_deref())
                        .await;

                    if token.is_cancelled() {
                        return;
                    }

                    match result {
                        Ok(translations) => {
                            {
                                let mut map = translated.lock().await;
                                for (id, subtitle) in &batch_data {
                                    let mut new_sub = subtitle.clone();
                                    if let Some(tr) = translations.get(id) {
                                        new_sub.text = tr.clone();
                                    }
                                    map.insert(*id, new_sub);
                                }
                                let _ = SrtParser::save_file(&output_path, &map);
                            }

                            let dur = batch_start_time.elapsed().as_secs_f64();
                            let completed_after = {
                                let mut stats = timing_stats.lock().await;
                                stats.0 += dur;
                                stats.1 += 1;
                                stats.1
                            };

                            let mut cb = progress_callback.lock().await;
                            cb(TranslationProgress {
                                message: format!(
                                    "Batch [{}-{}] completed ✓",
                                    batch_start, batch_end
                                ),
                                eta_seconds: eta,
                                current_batch: completed_after,
                                total_batches,
                                batch_start,
                                batch_end,
                            });
                            break;
                        }
                        Err(e) if is_rate_limit_error(&e) => {
                            scheduler.lock().await.report_exhausted(ti, ei);
                            let tier_now = scheduler.lock().await.active_tier_human();
                            let completed = timing_stats.lock().await.1;
                            let mut cb = progress_callback.lock().await;
                            cb(TranslationProgress {
                                message: format!(
                                    "{} rate-limited — failing over (tier {}) ↻",
                                    entry.label, tier_now
                                ),
                                eta_seconds: None,
                                current_batch: completed,
                                total_batches,
                                batch_start,
                                batch_end,
                            });
                        }
                        Err(e) => {
                            let completed = timing_stats.lock().await.1;
                            let mut cb = progress_callback.lock().await;
                            cb(TranslationProgress {
                                message: format!(
                                    "Batch [{}-{}] error via {}: {} ✗",
                                    batch_start, batch_end, entry.label, e
                                ),
                                eta_seconds: None,
                                current_batch: completed,
                                total_batches,
                                batch_start,
                                batch_end,
                            });
                            break;
                        }
                    }
                }
            }
        });

        workers.push(worker);
    }

    for w in workers {
        let _ = w.await;
    }

    if cancellation_token.is_cancelled() {
        anyhow::bail!("Translation cancelled by user");
    }

    if *exhausted_flag.lock().await {
        let completed = timing_stats.lock().await.1;
        let mut cb = progress_callback.lock().await;
        cb(TranslationProgress {
            message: "All tiers exhausted — some subtitles may be untranslated. Re-run to resume."
                .to_string(),
            eta_seconds: None,
            current_batch: completed,
            total_batches,
            batch_start: 0,
            batch_end: 0,
        });
    }

    let missing_ids = {
        let map = translated.lock().await;
        get_missing_or_incorrect_subtitle_ids(&subtitles_map, &map)
    };

    if !missing_ids.is_empty() && !cancellation_token.is_cancelled() {
        {
            let mut cb = progress_callback.lock().await;
            cb(TranslationProgress {
                message: format!(
                    "Repairing {} missing/incorrect subtitles...",
                    missing_ids.len()
                ),
                eta_seconds: None,
                current_batch: total_batches,
                total_batches,
                batch_start: 0,
                batch_end: 0,
            });
        }

        repair_missing_tiered(
            pool.clone(),
            &missing_ids,
            &subtitles_map,
            &translated,
            target_lang,
            title_context.map(|s| s.to_string()),
            output_path,
            progress_callback.clone(),
            &cancellation_token,
        )
        .await?;
    }

    let result = translated.lock().await.clone();
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
async fn repair_missing_tiered(
    pool: Arc<TranslatorPool>,
    missing_ids: &[u32],
    original_subtitles: &HashMap<u32, Subtitle>,
    translated: &Arc<tokio::sync::Mutex<HashMap<u32, Subtitle>>>,
    target_lang: &str,
    title_context: Option<String>,
    output_path: &std::path::Path,
    progress_callback: Arc<tokio::sync::Mutex<impl FnMut(TranslationProgress) + Send>>,
    cancellation_token: &CancellationToken,
) -> Result<()> {
    use tokio::sync::Mutex;

    let scheduler = Arc::new(Mutex::new(TierScheduler::new(&pool)));
    let total = missing_ids.len();

    for (idx, &id) in missing_ids.iter().enumerate() {
        if cancellation_token.is_cancelled() {
            anyhow::bail!("Repair cancelled by user");
        }

        let Some(original) = original_subtitles.get(&id) else {
            continue;
        };

        let context = {
            let map = translated.lock().await;
            build_repair_context(id, original_subtitles, &map)
        };

        loop {
            if cancellation_token.is_cancelled() {
                anyhow::bail!("Repair cancelled by user");
            }

            let acquired = { scheduler.lock().await.acquire() };
            let Some((ti, ei)) = acquired else {
                break;
            };
            let entry = pool[ti][ei].clone();

            if let Some(ref limiter) = entry.rate_limiter {
                tokio::select! {
                    _ = cancellation_token.cancelled() => anyhow::bail!("Repair cancelled by user"),
                    _ = limiter.until_ready() => {}
                }
            }

            match entry
                .translator
                .translate_with_context(
                    &original.text,
                    target_lang,
                    title_context.as_deref(),
                    context.as_deref(),
                )
                .await
            {
                Ok(translation) => {
                    let mut new_sub = original.clone();
                    new_sub.text = translation;
                    let mut map = translated.lock().await;
                    map.insert(id, new_sub);
                    let _ = SrtParser::save_file(output_path, &map);
                    break;
                }
                Err(e) if is_rate_limit_error(&e) => {
                    scheduler.lock().await.report_exhausted(ti, ei);
                }
                Err(e) => {
                    let mut cb = progress_callback.lock().await;
                    cb(TranslationProgress {
                        message: format!("Repair failed for subtitle {}: {}", id, e),
                        eta_seconds: None,
                        current_batch: idx,
                        total_batches: total,
                        batch_start: id as usize,
                        batch_end: id as usize,
                    });
                    break;
                }
            }
        }
    }

    Ok(())
}
