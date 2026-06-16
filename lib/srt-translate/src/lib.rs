//! # srt-translate-lib
//!
//! Libreria core per la traduzione di sottotitoli SRT usando LLM (locali o remoti).
//! 
//! Questa libreria implementa il pattern di Inversione del Controllo (IoC),
//! permettendo al chiamante di definire come gestire gli aggiornamenti di progresso
//! tramite callback personalizzati.

mod translator;
mod language_info;
mod prompts;
mod rate_limiter;

use anyhow::Result;
use srt_parser::{SrtParser, Subtitle};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

// Re-export dei tipi pubblici
pub use translator::{Translator, TranslatorConfig, ApiType};
pub use rate_limiter::{RateLimiter, RateLimitConfig, create_rate_limiter, create_rate_limiter_with_burst};

/// Dati di progresso della traduzione passati al callback
#[derive(Debug, Clone)]
pub struct TranslationProgress {
    /// Messaggio descrittivo dello stato corrente
    pub message: String,
    /// Tempo stimato rimanente in secondi (None se non disponibile)
    pub eta_seconds: Option<f64>,
    /// Numero batch corrente
    pub current_batch: usize,
    /// Numero totale di batch
    pub total_batches: usize,
    /// Numero del primo sottotitolo del batch corrente
    pub batch_start: usize,
    /// Numero dell'ultimo sottotitolo del batch corrente
    pub batch_end: usize,
}

/// Risultato della traduzione batch-by-batch
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// Batch completato con successo
    pub success: bool,
    /// Numero batch
    pub batch_number: usize,
    /// Errore eventuale
    pub error: Option<String>,
}

/// Traduce tutti i sottotitoli usando multiple API keys in parallelo con rate limiting
///
/// # Argomenti
///
/// * `translators` - Vector di Translator pre-configurati (uno per API provider)
/// * `subtitles` - HashMap dei sottotitoli da tradurre
/// * `target_lang` - Codice lingua target (es: "it", "en", "es")
/// * `batch_size` - Numero di sottotitoli da tradurre insieme
/// * `title_context` - Contesto opzionale (es: titolo del film)
/// * `output_path` - Percorso del file di output per salvataggio incrementale
/// * `on_progress` - Callback invocato ad ogni aggiornamento di progresso
///
/// **Nota**: Questa funzione usa solo il semaforo per limitare la concorrenza.
/// Per un vero rate limiting basato su RPM, usa `translate_subtitles_with_rate_limit`.
// API di traduzione pubblica a strati: ogni variante aggiunge una capacità (rate
// limiting, cancellazione) e delega alla successiva, quindi la lista di parametri
// è intrinseca al design, non complessità incidentale.
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
    ).await
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
    ).await
}

/// Salva i sottotitoli tradotti su file
fn save_translated_subtitles(
    subtitles: &HashMap<u32, Subtitle>,
    path: &std::path::Path,
) -> Result<()> {
    use std::io::Write;

    let mut sorted: Vec<_> = subtitles.iter().collect();
    sorted.sort_by_key(|(id, _)| *id);

    let mut file = std::fs::File::create(path)?;

    for (id, subtitle) in sorted {
        writeln!(file, "{}", id)?;
        writeln!(
            file,
            "{} --> {}",
            subtitle.start.to_srt_string(), 
            subtitle.end.to_srt_string()
        )?;
        writeln!(file, "{}", subtitle.text)?;
        writeln!(file)?;
    }

    Ok(())
}

/// Verifica che tutti i sottotitoli dell'originale siano presenti nella traduzione
/// Restituisce gli ID mancanti
pub fn verify_translation_completeness(
    original: &HashMap<u32, Subtitle>,
    translated: &HashMap<u32, Subtitle>,
) -> Vec<u32> {
    get_missing_or_incorrect_subtitle_ids(original, translated)
}

/// Funzione modulare che identifica gli ID dei sottotitoli mancanti o con discrepanze nella traduzione
/// 
/// Questa funzione confronta l'originale con la traduzione e restituisce un vettore
/// contenente gli ID dei sottotitoli che:
/// - Sono presenti nell'originale ma assenti nella traduzione
/// - Hanno un numero di linee diverso tra originale e traduzione
///
/// # Argomenti
///
/// * `original` - HashMap dei sottotitoli originali
/// * `translated` - HashMap dei sottotitoli tradotti
///
/// # Restituisce
///
/// Un vettore di ID (u32) dei sottotitoli che devono essere corretti/tradotti
pub fn get_missing_or_incorrect_subtitle_ids(
    original: &HashMap<u32, Subtitle>,
    translated: &HashMap<u32, Subtitle>,
) -> Vec<u32> {
    original
        .iter()
        .filter(|(id, original_sub)| {
            // Sottotitolo mancante
            if !translated.contains_key(id) {
                return true;
            }
            
            // Sottotitolo presente ma con numero di linee diverso
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

/// Funzione legacy che identifica solo gli ID dei sottotitoli completamente mancanti
/// (senza verificare il numero di linee)
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

/// Ripara una traduzione incompleta traducendo i sottotitoli mancanti in parallelo
/// con contesto migliorato (sottotitoli prima e dopo)
/// 
/// Utilizza tutti i translators disponibili con un semaforo per massimizzare
/// l'efficienza del parallelismo
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
    ).await
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
    
    // Wrapper thread-safe per il callback e i risultati
    let progress_callback = Arc::new(Mutex::new(on_progress));
    let repaired = Arc::new(Mutex::new(HashMap::new()));
    
    // Crea un semaforo per controllare il parallelismo (come nella traduzione principale)
    let semaphore = Arc::new(Semaphore::new(translators_len));
    
    // Wrappa i rate limiters
    let rate_limiters: Option<Vec<Arc<RateLimiter>>> = rate_limiters;

    {
        let mut callback = progress_callback.lock().await;
        callback(TranslationProgress {
            message: format!("Found {} missing subtitles, repairing with {} workers...", total, translators_len),
            eta_seconds: None,
            current_batch: 0,
            total_batches: total,
            batch_start: 0,
            batch_end: 0,
        });
    }

    // Timing stats per ETA
    let timing_stats = Arc::new(Mutex::new((0.0_f64, 0_usize))); // (total_duration, completed)
    let start_time = Instant::now();

    // Crea tasks per ogni sottotitolo mancante
    let mut handles = vec![];
    
    for (idx, id) in missing_ids.iter().enumerate() {
        if let Some(subtitle) = original.get(id) {
            // Seleziona translator in round-robin per bilanciare il carico
            let translator_idx = idx % translators_len;
            let translator = translators[translator_idx].clone();
            
            let semaphore = semaphore.clone();
            // Clona il rate limiter per questo provider (se disponibile)
            let rate_limiter = rate_limiters.as_ref().map(|limiters| {
                limiters[translator_idx % limiters.len()].clone()
            });
            let id = *id;
            let subtitle = subtitle.clone();
            let target_lang = target_lang.to_string();
            let title_context = title_context.map(|s| s.to_string());
            let progress_callback = progress_callback.clone();
            let repaired = repaired.clone();
            let timing_stats = timing_stats.clone();
            
            // Costruisce il contesto: sottotitoli prima e dopo
            let context_text = build_repair_context(id, original, translated);

            let handle = tokio::spawn(async move {
                // Prima: aspetta il rate limiter RPM (se configurato)
                if let Some(ref limiter) = rate_limiter {
                    limiter.until_ready().await;
                }
                
                // Poi: acquisisce permit dal semaforo per limitare il parallelismo
                // Questo non fallisce a meno che il semaforo non sia chiuso
                let _permit = semaphore.acquire().await
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

                // Notifica inizio
                {
                    let completed = timing_stats.lock().await.1;
                    let mut callback = progress_callback.lock().await;
                    callback(TranslationProgress {
                        message: format!("Repairing subtitle {} ({}/{}) [worker {}]", id, idx + 1, total, translator_idx + 1),
                        eta_seconds: eta,
                        current_batch: completed,
                        total_batches: total,
                        batch_start: idx + 1,
                        batch_end: total,
                    });
                }

                // Usa un prompt speciale con contesto
                match translator.translate_with_context(
                    &subtitle.text, 
                    &target_lang, 
                    title_context.as_deref(),
                    context_text.as_deref()
                ).await {
                    Ok(translation) => {
                        let mut new_subtitle = subtitle.clone();
                        new_subtitle.text = translation;
                        repaired.lock().await.insert(id, new_subtitle);
                        
                        // Aggiorna timing stats
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
                        // In caso di errore, inseriamo l'originale
                        repaired.lock().await.insert(id, subtitle.clone());
                    }
                }
            });

            handles.push(handle);
        }
    }

    // Attendi completamento di tutti i task
    for handle in handles {
        let _ = handle.await;
    }

    // Applica le riparazioni
    let repaired_subs = repaired.lock().await;
    for (id, subtitle) in repaired_subs.iter() {
        translated.insert(*id, subtitle.clone());
    }

    let total_time = start_time.elapsed().as_secs_f64();
    let mut callback = progress_callback.lock().await;
    callback(TranslationProgress {
        message: format!("Repair completed! Fixed {} subtitles in {:.1}s ✓", total, total_time),
        eta_seconds: Some(0.0),
        current_batch: total,
        total_batches: total,
        batch_start: total,
        batch_end: total,
    });

    Ok(())
}

/// Costruisce il contesto per il repair: sottotitoli prima e dopo quello mancante
fn build_repair_context(
    missing_id: u32,
    original: &HashMap<u32, Subtitle>,
    translated: &HashMap<u32, Subtitle>,
) -> Option<String> {
    let mut context_parts = Vec::new();
    
    // Cerca 2 sottotitoli prima
    for offset in (1..=2).rev() {
        if let Some(prev_id) = missing_id.checked_sub(offset) {
            if let (Some(orig), Some(trans)) = (original.get(&prev_id), translated.get(&prev_id)) {
                context_parts.push(format!(
                    "[{}] Original: {}\nTranslated: {}",
                    prev_id, orig.text, trans.text
                ));
            }
        }
    }
    
    // Cerca 2 sottotitoli dopo
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

/// Traduce tutti i sottotitoli con supporto per cancellazione
///
/// Questa versione permette di cancellare la traduzione in corso tramite un CancellationToken.
///
/// # Argomenti
///
/// * `translators` - Vector di Translator pre-configurati
/// * `rate_limiters` - Optional: Vector di RateLimiter
/// * `subtitles` - HashMap dei sottotitoli da tradurre
/// * `target_lang` - Codice lingua target
/// * `batch_size` - Numero di sottotitoli da tradurre insieme
/// * `title_context` - Contesto opzionale
/// * `output_path` - Percorso del file di output
/// * `on_progress` - Callback per il progresso
/// * `cancellation_token` - Token per cancellare la traduzione
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
    let (skip_batches, start_idx) = if output_path.exists() {
        match SrtParser::parse_file(output_path) {
            Ok(existing_translations) => {
                let existing_count = existing_translations.len();
                if existing_count > 0 {
                    *translated.lock().await = existing_translations.clone();
                    let missing_count = get_missing_or_incorrect_subtitle_ids(&subtitles_map, &existing_translations).len();
                    if missing_count > 0 {
                        (total_batches, 0)
                    } else {
                        let calc_start_idx = existing_count.saturating_sub(resume_overlap);
                        let skip_b = calc_start_idx / batch_size;
                        (skip_b, calc_start_idx)
                    }
                } else { (0, 0) }
            }
            Err(_) => (0, 0)
        }
    } else { (0, 0) };

    // Prepara i batch da processare
    let remaining = if start_idx < sorted.len() { &sorted[start_idx..] } else { &[] };
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
        let rate_limiter = rate_limiters.as_ref().map(|limiters| {
            limiters[translator_idx % limiters.len()].clone()
        });
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
                } else { None }
            };

            {
                let completed = timing_stats.lock().await.1;
                let mut callback = progress_callback.lock().await;
                callback(TranslationProgress {
                    message: format!("Starting batch [{}-{}]/{} (worker {})...", 
                        batch_start, batch_end, total, translator_idx + 1),
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
                        let _ = save_translated_subtitles(&trans_map, &output_path);
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
            message: format!("Repairing {} missing/incorrect subtitles...", missing_ids.len()),
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
        ).await?;
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
                .translate_with_context(&original.text, target_lang, title_context, context.as_deref())
                .await;

            match result {
                Ok(translation) => {
                    let mut new_subtitle = original.clone();
                    new_subtitle.text = translation;
                    
                    let mut trans_map = translated.lock().await;
                    trans_map.insert(id, new_subtitle);
                    let _ = save_translated_subtitles(&trans_map, output_path);
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
