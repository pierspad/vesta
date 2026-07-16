//! # srt-translate CLI
//!
//! Interfaccia a riga di comando per la traduzione di sottotitoli SRT.
//! Questo è il "guscio" che gestisce l'I/O utente e delega la logica
//! di business alla libreria `srt-translate`.

use anyhow::Result;
use clap::Parser;
use regex::Regex;
use serde::Deserialize;
use srt_parser::SrtParser;
use std::path::PathBuf;
use std::io::{self, Write};
use std::sync::{Arc, LazyLock};
use srt_translate::{
    translate_subtitles_with_rate_limit, 
    repair_translation_with_rate_limit,
    TranslationProgress,
    verify_translation_completeness,
    Translator,
    TranslatorConfig,
    ApiType,
    create_rate_limiter,
    RateLimiter,
};

/// Configurazione caricata da config.toml
#[derive(Debug, Deserialize)]
struct Config {
    api: ApiConfig,
    translation: TranslationConfig,
    #[serde(default)]
    output: OutputConfig,
}

#[derive(Debug, Deserialize)]
struct ApiConfig {
    providers: Vec<ProviderConfig>,
}

#[derive(Debug, Deserialize, Clone)]
struct ProviderConfig {
    provider: String,  // "gemini", "openai", "local"
    api_key: String,
    model: String,
    rpm_limit: usize,
    #[serde(default)]
    workers_per_key: Option<usize>,  // Ora opzionale, calcolato automaticamente se None
}

#[derive(Debug, Deserialize)]
struct TranslationConfig {
    batch_size: usize,
    #[serde(default)]
    resume_overlap: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
struct OutputConfig {
    #[serde(default = "default_filename_pattern")]
    filename_pattern: String,
}

fn default_filename_pattern() -> String {
    "{input_name}.{language}.srt".to_string()
}

/// Pattern per i placeholder ${VAR_NAME}, compilato una sola volta al primo
/// utilizzo invece che ad ogni chiamata di `expand_env_vars`.
static ENV_VAR_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").expect("Regex pattern is a compile-time constant")
});

/// Espande le variabili d'ambiente nel formato ${VAR_NAME}
fn expand_env_vars(content: &str) -> Result<String> {
    let mut result = content.to_string();
    let mut missing_vars = Vec::new();

    for cap in ENV_VAR_PATTERN.captures_iter(content) {
        let var_name = &cap[1];
        let placeholder = &cap[0];
        
        match std::env::var(var_name) {
            Ok(value) => {
                result = result.replace(placeholder, &value);
            }
            Err(_) => {
                missing_vars.push(var_name.to_string());
            }
        }
    }
    
    if !missing_vars.is_empty() {
        return Err(anyhow::anyhow!(
            "Missing required environment variables: {}\n\nPlease set them in your .env file.",
            missing_vars.join(", ")
        ));
    }
    
    Ok(result)
}

/// Calcola automaticamente il numero di workers per provider in base al RPM
/// 
/// Per task I/O bound (come le chiamate API), il numero di workers non è 
/// limitato dalla CPU ma dalla latenza di rete e dal rate limit dell'API.
/// 
/// Strategia:
/// - Con rate limiters abilitati, possiamo avere più workers in attesa
/// - Il rate limiter garantisce di non superare l'RPM
/// - Più workers = migliore parallelizzazione delle richieste
fn calculate_workers_per_provider(rpm_limit: usize) -> usize {
    // Per task I/O bound, usiamo un numero di workers basato sul RPM
    // Più RPM = più workers possibili
    // Formula: circa 1 worker ogni 5-10 RPM, con min=2 e max=20
    (rpm_limit / 8).clamp(2, 20)
}

/// Calcola workers legacy basato su CPU (per retrocompatibilità)
#[deprecated(note = "Use calculate_workers_per_provider which is optimized for I/O bound tasks")]
#[allow(dead_code)]
fn calculate_workers_per_key_legacy(num_providers: usize) -> usize {
    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    
    // Lascia sempre almeno 1 CPU libera
    let available_cpus = if num_cpus > 1 { num_cpus - 1 } else { 1 };

    // Distribuisci le CPU disponibili tra i provider (checked_div: num_providers può essere 0)
    available_cpus.checked_div(num_providers).unwrap_or(1).max(1)
}

#[derive(Parser)]
#[command(name = "srt-translate")]
#[command(about = "Translate SRT files using LLM APIs with multi-account support", long_about = None)]
struct Cli {
    /// Path to the SRT file to translate
    #[arg(short, long)]
    input: PathBuf,

    /// Target language (supported with examples: en, it, es, fr, de, pt, ru, ja, zh, ar)
    #[arg(short, long)]
    language: Option<String>,

    /// Show list of supported languages
    #[arg(long)]
    language_list: bool,

    /// Check which subtitles are missing or have line count discrepancies in a translated file
    /// Usage: srt-translate -i original.srt --check-missing translated.srt
    #[arg(long, value_name = "TRANSLATED_FILE")]
    check_missing: Option<PathBuf>,

    /// Path to config file (default: config.toml in current directory)
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Output file (optional, will use pattern from config if not specified)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Carica le variabili d'ambiente dal file .env
    // Ignora l'errore se il file .env non esiste (non è obbligatorio)
    let _ = dotenvy::dotenv();
    
    let cli = Cli::parse();
    
    // Se richiesta la lista delle lingue
    if cli.language_list {
        print_language_list();
        return Ok(());
    }

    // Se richiesta la verifica dei sottotitoli mancanti
    if let Some(translated_file) = &cli.check_missing {
        return check_missing_subtitles(&cli.input, translated_file);
    }

    // Validazione: language è obbligatorio per la traduzione
    let language = cli.language.as_ref()
        .ok_or_else(|| anyhow::anyhow!("Language is required. Use -l/--language <code> or see --language-list for available languages"))?;

    // Carica il file di configurazione
    let mut config_content = std::fs::read_to_string(&cli.config)
        .map_err(|e| anyhow::anyhow!(
            "Failed to read config file '{}': {}\n\nPlease create a config.toml file or specify --config path.",
            cli.config.display(),
            e
        ))?;
    
    // Sostituisci i placeholder ${VAR_NAME} con i valori dalle variabili d'ambiente
    config_content = expand_env_vars(&config_content)?;
    
    let config: Config = toml::from_str(&config_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse config.toml: {}", e))?;

    // Validazione
    if config.api.providers.is_empty() {
        return Err(anyhow::anyhow!("No API providers found in config.toml. Please add at least one [[api.providers]] section."));
    }
    
    // Applica il calcolo automatico dei workers basato sull'RPM (I/O bound optimization)
    let providers_with_workers: Vec<ProviderConfig> = config.api.providers
        .into_iter()
        .map(|mut p| {
            if p.workers_per_key.is_none() {
                // Calcola workers in base al RPM del provider (ottimizzato per I/O bound)
                p.workers_per_key = Some(calculate_workers_per_provider(p.rpm_limit));
            }
            p
        })
        .collect();

    // Statistiche
    let total_workers: usize = providers_with_workers.iter()
        .map(|p| p.workers_per_key.unwrap_or(1))
        .sum();
    
    let num_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    
    println!("🔧 Configuration:");
    println!("  💻 System CPUs: {} (not a bottleneck for I/O bound tasks)", num_cpus);
    println!("  📄 Config file: {}", cli.config.display());
    println!("  🔑 API providers: {}", providers_with_workers.len());
    for (idx, provider) in providers_with_workers.iter().enumerate() {
        let workers = provider.workers_per_key.unwrap_or(1);
        let auto_note = if provider.workers_per_key.is_none() { 
            " [auto-calculated from RPM]" 
        } else { 
            "" 
        };
        println!("     [{idx}] {} - {} ({} RPM, {} workers{})", 
            provider.provider, provider.model, provider.rpm_limit, workers, auto_note);
    }
    println!("  📦 Batch size: {} subtitles per request", config.translation.batch_size);
    println!("  🚀 Total concurrent workers: {}", total_workers);
    println!();

    // Parse del file SRT
    println!("📖 Reading file: {}", cli.input.display());
    let original_subtitles = SrtParser::parse_file(&cli.input)?;
    println!("✅ Found {} subtitles", original_subtitles.len());

    // Estrai il titolo del film/show dal nome file per contesto
    let title_context = cli.input
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.replace(['_', '.'], " "));

    if let Some(ref title) = title_context {
        println!("🎬 Detected title: {}", title);
    }

    // Determina il percorso di output
    let output_path = cli.output.unwrap_or_else(|| {
        let parent = cli.input.parent().unwrap_or_else(|| std::path::Path::new("."));
        
        // Estrai il nome file senza estensione
        let input_filename = cli.input.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("subtitles");
        
        // Rimuovi eventuale codice lingua esistente
        let base_name = if let Some(last_dot_idx) = input_filename.rfind('.') {
            let potential_lang = &input_filename[last_dot_idx + 1..];
            if potential_lang.len() == 2 && potential_lang.chars().all(|c| c.is_ascii_lowercase()) {
                input_filename[..last_dot_idx].to_string()
            } else {
                input_filename.to_string()
            }
        } else {
            input_filename.to_string()
        };
        
        // Applica il pattern dal config
        let filename = config.output.filename_pattern
            .replace("{input_name}", &base_name)
            .replace("{language}", language);
        
        parent.join(filename)
    });

    println!(" Translating to '{}'...", language);
    println!();

    // Callback per il progresso
    let cli_progress_handler = |progress: TranslationProgress| {
        if let Some(eta) = progress.eta_seconds {
            let minutes = (eta / 60.0) as u32;
            let seconds = (eta % 60.0) as u32;
            
            if progress.message.contains("Starting batch") {
                print!("  {} (ETA: {}m {}s)...\r", progress.message, minutes, seconds);
                // flush può fallire se stdout è una pipe chiusa - ignoriamo l'errore
                let _ = io::stdout().flush();
            } else if progress.message.contains("completed") {
                // Mostra il completamento con l'ETA
                println!("  {} (ETA: {}m {}s remaining)", progress.message, minutes, seconds);
            } else {
                println!("  {} (ETA: {}m {}s)", progress.message, minutes, seconds);
            }
        } else if progress.message.contains("✓") || progress.message.contains("✗") {
            println!("  {}", progress.message);
        } else if !progress.message.is_empty() {
            println!("  ℹ️  {}", progress.message);
        }
    };

    // Crea i Translator e RateLimiters da tutti i provider configurati
    let mut translators: Vec<Translator> = Vec::new();
    let mut rate_limiters: Vec<Arc<RateLimiter>> = Vec::new();
    
    for provider in &providers_with_workers {
        // Determina tipo API dal provider - Local o Google (OpenRouter disabilitato)
        let api_type = match provider.provider.as_str() {
            "local" => ApiType::Local,
            "google" | "gemini" => ApiType::Google,
            "groq" => ApiType::Groq,
            // OpenRouter e altri sono disabilitati
            _ => {
                eprintln!("⚠️ Provider '{}' non supportato. Usa 'local', 'groq' o 'google'.", provider.provider);
                continue;
            }
        };

        // Determina base URL
        let base_url = match api_type {
            ApiType::Local => "http://localhost:1234/v1",
            ApiType::Google => "https://generativelanguage.googleapis.com/v1beta",
            ApiType::OpenRouter => "https://openrouter.ai/api/v1", // Non usato
            ApiType::Groq => "https://api.groq.com/openai/v1",
        };

        let workers = provider.workers_per_key.unwrap_or(1);
        
        // Crea un rate limiter condiviso per questo provider (rispetta RPM)
        // Il rate limiter è condiviso tra tutti i workers dello stesso provider
        let rate_limiter = create_rate_limiter(provider.rpm_limit as u32);
        
        // Crea N translators per questo provider (workers_per_key)
        for _ in 0..workers {
            translators.push(Translator::new(TranslatorConfig {
                base_url: base_url.to_string(),
                model: provider.model.clone(),
                api_key: Some(provider.api_key.clone()),
                api_type: api_type.clone(),
            }));
            // Ogni worker dello stesso provider condivide lo stesso rate limiter
            rate_limiters.push(rate_limiter.clone());
        }
    }

    let mut translated = translate_subtitles_with_rate_limit(
        translators.clone(),
        Some(rate_limiters.clone()),
        original_subtitles.clone(),
        language,
        config.translation.batch_size,
        config.translation.resume_overlap.unwrap_or(2),
        title_context.as_deref(),
        &output_path,
        cli_progress_handler,
    )
    .await?;

    println!();
    
    // Loop di verifica e riparazione automatica
    // Continua fino a quando non ci sono più discrepanze
    let max_repair_attempts = 5;
    let mut repair_attempt = 0;
    
    loop {
        repair_attempt += 1;
        
        // Verifica completezza della traduzione
        println!("🔍 Verifying translation completeness...");
        let missing_ids = verify_translation_completeness(&original_subtitles, &translated);
        
        if missing_ids.is_empty() {
            println!("✅ All {} subtitles present with correct line counts - no repair needed!", original_subtitles.len());
            break;
        }
        
        if repair_attempt > max_repair_attempts {
            println!("⚠️  Maximum repair attempts ({}) reached.", max_repair_attempts);
            println!("⚠️  Still found {} subtitles with issues.", missing_ids.len());
            println!("💡 You may need to review these subtitles manually.");
            break;
        }
        
        println!("⚠️  Found {} subtitles with issues!", missing_ids.len());
        println!("🔧 Starting automatic repair (attempt {}/{}) with {} workers...", 
                 repair_attempt, max_repair_attempts, translators.len());
        println!();
        
        // Callback per il repair
        let repair_progress = |progress: TranslationProgress| {
            if progress.message.contains("Repairing") {
                print!("  {} \r", progress.message);
                io::stdout().flush().unwrap_or(());
            } else {
                println!("  {}", progress.message);
            }
        };
        
        // Gestione errori migliorata: se il repair fallisce, salva il file parziale
        // e continua al prossimo tentativo invece di propagare l'errore fatalmente
        let repair_result = repair_translation_with_rate_limit(
            translators.clone(),
            Some(rate_limiters.clone()),
            &original_subtitles,
            &mut translated,
            missing_ids.clone(),
            language,
            title_context.as_deref(),
            repair_progress,
        )
        .await;
        
        if let Err(e) = repair_result {
            eprintln!("⚠️  Repair attempt {} failed: {}", repair_attempt, e);
            
            // Salva comunque il file parziale per non perdere il lavoro fatto
            save_partial_translation(&output_path, &translated)?;
            println!("💾 Partial translation saved to: {}", output_path.display());
            
            // Se l'errore è fatale (es. Quota Exceeded), aspetta un po' prima di riprovare
            if e.to_string().contains("Quota") || e.to_string().contains("429") || e.to_string().contains("rate") {
                println!("⏳ Rate limit detected, waiting 60 seconds before retry...");
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            } else {
                // Per altri errori, breve pausa per far "raffreddare" l'API
                println!("⏳ Waiting 5 seconds before retry...");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
            
            continue; // Riprova il loop di repair
        }
        
        // Salva la versione riparata dopo ogni iterazione
        SrtParser::save_file(&output_path, &translated)?;

        println!();
        println!("✅ Repair cycle {} completed, re-verifying...", repair_attempt);
        println!();
    }
    
    println!();
    println!("💾 Translation complete: {}", output_path.display());
    println!("✨ Done!");

    Ok(())
}

/// Salva il file di traduzione parziale (usato in caso di errore nel repair)
/// Questo evita di perdere il lavoro già fatto se la traduzione viene interrotta
fn save_partial_translation(
    output_path: &PathBuf,
    translated: &std::collections::HashMap<u32, srt_parser::Subtitle>,
) -> Result<()> {
    SrtParser::save_file(output_path, translated)
}

fn check_missing_subtitles(original_path: &PathBuf, translated_path: &PathBuf) -> Result<()> {
    println!("🔍 Checking for missing subtitles and line count discrepancies...");
    println!();

    // Parse del file originale
    println!("📖 Reading original file: {}", original_path.display());
    let original_subtitles = SrtParser::parse_file(original_path)?;
    println!("✅ Found {} subtitles in original", original_subtitles.len());
    println!();

    // Verifica se il file tradotto esiste
    if !translated_path.exists() {
        println!("❌ Translation file not found: {}", translated_path.display());
        println!("   All {} subtitles need to be translated.", original_subtitles.len());
        return Ok(());
    }

    println!("📖 Reading translation file: {}", translated_path.display());
    let translated_subtitles = SrtParser::parse_file(translated_path)?;
    println!("✅ Found {} subtitles in translation", translated_subtitles.len());
    println!();

    // Verifica completezza
    let missing_ids = verify_translation_completeness(&original_subtitles, &translated_subtitles);
    
    // Verifica discrepanze nel numero di righe
    let mut line_discrepancies = Vec::new();
    for (id, original_sub) in &original_subtitles {
        if let Some(translated_sub) = translated_subtitles.get(id) {
            let original_lines = original_sub.text.lines().count();
            let translated_lines = translated_sub.text.lines().count();
            
            if original_lines != translated_lines {
                line_discrepancies.push((*id, original_lines, translated_lines));
            }
        }
    }
    
    let has_issues = !missing_ids.is_empty() || !line_discrepancies.is_empty();
    
    if !has_issues {
        println!("✅ Translation is complete and accurate!");
        println!("   All {} subtitles are present with correct line counts.", original_subtitles.len());
    } else {
        if !missing_ids.is_empty() {
            println!("⚠️  Found {} missing subtitles:", missing_ids.len());
            println!();
            
            // Raggruppa gli ID mancanti in intervalli per una visualizzazione più pulita
            let ranges = group_ids_into_ranges(&missing_ids);
            
            println!("Missing subtitle IDs:");
            for range in ranges {
                if range.0 == range.1 {
                    println!("  - {}", range.0);
                } else {
                    println!("  - {} to {}", range.0, range.1);
                }
            }
            println!();
        }
        
        if !line_discrepancies.is_empty() {
            println!("⚠️  Found {} subtitles with line count discrepancies:", line_discrepancies.len());
            println!();
            println!("Subtitle ID | Original Lines | Translated Lines");
            println!("------------|----------------|------------------");
            
            // Mostra i primi 20 e se ce ne sono di più, mostra un sommario
            let show_count = line_discrepancies.len().min(20);
            for (id, orig_lines, trans_lines) in &line_discrepancies[..show_count] {
                println!("     {:6} |         {:6} |           {:6}", id, orig_lines, trans_lines);
            }
            
            if line_discrepancies.len() > 20 {
                println!("     ...and {} more", line_discrepancies.len() - 20);
            }
            println!();
            println!("💡 These subtitles may have been incorrectly translated (missing or extra lines).");
        }
        
        println!();
        println!("💡 Tip: Re-run the translation to fix these issues:");
        println!("   srt-translate -i {} -o {}", original_path.display(), translated_path.display());
    }

    Ok(())
}

/// Raggruppa una lista di ID in intervalli consecutivi per una visualizzazione compatta
fn group_ids_into_ranges(ids: &[u32]) -> Vec<(u32, u32)> {
    if ids.is_empty() {
        return Vec::new();
    }
    
    let mut sorted_ids = ids.to_vec();
    sorted_ids.sort_unstable();
    
    let mut ranges = Vec::new();
    let mut range_start = sorted_ids[0];
    let mut range_end = sorted_ids[0];
    
    for &id in &sorted_ids[1..] {
        if id == range_end + 1 {
            range_end = id;
        } else {
            ranges.push((range_start, range_end));
            range_start = id;
            range_end = id;
        }
    }
    ranges.push((range_start, range_end));
    
    ranges
}

fn print_language_list() {
    println!("🌍 Supported languages with optimized few-shot examples:");
    println!();
    println!("  Code    | Language");
    println!("  --------|------------------");
    println!("  en      | English");
    println!("  it      | Italian");
    println!("  es      | Spanish");
    println!("  fr      | French");
    println!("  de      | German");
    println!("  pt      | Portuguese");
    println!("  ru      | Russian");
    println!("  ja      | Japanese");
    println!("  zh      | Chinese (Simplified)");
    println!("  ar      | Arabic");
    println!();
    println!("Note: Other language codes may work but won't have optimized examples.");
    println!("Usage: srt-translate -i file.srt -l <code>");
}
