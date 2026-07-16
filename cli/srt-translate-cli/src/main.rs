//! # srt-translate CLI
//!
//! Interfaccia a riga di comando per la traduzione di sottotitoli SRT.
//! Questo è il "guscio" che gestisce l'I/O utente e delega la logica
//! di business alla libreria `srt-translate` — inclusi il pool a tier
//! e lo scheduler di failover, condivisi con l'app desktop (vedi
//! `srt_translate::{build_pool, translate_subtitles_tiered_cancellable}`).

use anyhow::Result;
use clap::Parser;
use regex::Regex;
use serde::Deserialize;
use srt_parser::SrtParser;
use srt_translate::{TierEntry, TranslationProgress, translate_subtitles_tiered_cancellable};
use std::path::PathBuf;
use std::sync::LazyLock;
use tokio_util::sync::CancellationToken;

/// Configurazione caricata da config.toml
#[derive(Debug, Deserialize)]
struct Config {
    /// Tier di precedenza: il primo esaurito (rate-limit/quota) fa
    /// scattare il failover automatico verso il tier successivo.
    #[serde(default)]
    tiers: Vec<TierConfig>,
    translation: TranslationConfig,
    #[serde(default)]
    output: OutputConfig,
}

#[derive(Debug, Deserialize)]
struct TierConfig {
    entries: Vec<TierEntry>,
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

#[derive(Parser)]
#[command(name = "srt-translate")]
#[command(about = "Translate SRT files using LLM APIs with tiered multi-provider failover", long_about = None)]
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

    // Il pool a tier (default per provider, filtro delle entry inutilizzabili,
    // rate limiter, concorrenza) è tutto delegato a `srt_translate::build_pool`:
    // la stessa funzione usata dall'app desktop, così CLI e GUI condividono
    // un'unica implementazione del failover invece di due.
    let tiers: Vec<Vec<TierEntry>> = config.tiers.into_iter().map(|t| t.entries).collect();
    let pool = srt_translate::build_pool(&tiers).map_err(|e| anyhow::anyhow!(e))?;

    println!("🔧 Configuration:");
    println!("  📄 Config file: {}", cli.config.display());
    for (tier_idx, tier) in pool.iter().enumerate() {
        println!("  🏷️  Tier {}:", tier_idx + 1);
        for entry in tier {
            println!("     - {}", entry.label);
        }
    }
    println!(
        "  📦 Batch size: {} subtitles per request",
        config.translation.batch_size
    );
    println!();

    // Parse del file SRT
    println!("📖 Reading file: {}", cli.input.display());
    let original_subtitles = SrtParser::parse_file(&cli.input)?;
    println!("✅ Found {} subtitles", original_subtitles.len());

    // Estrai il titolo del film/show dal nome file per contesto
    let title_context = cli
        .input
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.replace(['_', '.'], " "));

    if let Some(ref title) = title_context {
        println!("🎬 Detected title: {}", title);
    }

    // Determina il percorso di output
    let output_path = cli.output.unwrap_or_else(|| {
        let parent = cli
            .input
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));

        // Estrai il nome file senza estensione
        let input_filename = cli
            .input
            .file_stem()
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
        let filename = config
            .output
            .filename_pattern
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
            println!("  {} (ETA: {}m {}s)", progress.message, minutes, seconds);
        } else if !progress.message.is_empty() {
            println!("  ℹ️  {}", progress.message);
        }
    };

    // Un'unica chiamata gestisce batching, rate limiting, failover a tier e
    // repair automatico delle righe mancanti/incoerenti: tutta questa logica
    // vive in `srt_translate` (vedi `translate_subtitles_tiered_cancellable`),
    // non più duplicata qui nel CLI.
    let translated = translate_subtitles_tiered_cancellable(
        pool,
        original_subtitles.clone(),
        language,
        config.translation.batch_size,
        config.translation.resume_overlap.unwrap_or(2),
        title_context.as_deref(),
        &output_path,
        cli_progress_handler,
        CancellationToken::new(),
    )
    .await?;

    println!();

    let missing_ids =
        srt_translate::verify_translation_completeness(&original_subtitles, &translated);
    if missing_ids.is_empty() {
        println!(
            "✅ All {} subtitles present with correct line counts!",
            original_subtitles.len()
        );
    } else {
        println!(
            "⚠️  {} subtitles still have issues after repair — you may need to review them manually.",
            missing_ids.len()
        );
    }

    println!();
    println!("💾 Translation complete: {}", output_path.display());
    println!("✨ Done!");

    Ok(())
}

fn check_missing_subtitles(original_path: &PathBuf, translated_path: &PathBuf) -> Result<()> {
    println!("🔍 Checking for missing subtitles and line count discrepancies...");
    println!();

    // Parse del file originale
    println!("📖 Reading original file: {}", original_path.display());
    let original_subtitles = SrtParser::parse_file(original_path)?;
    println!(
        "✅ Found {} subtitles in original",
        original_subtitles.len()
    );
    println!();

    // Verifica se il file tradotto esiste
    if !translated_path.exists() {
        println!(
            "❌ Translation file not found: {}",
            translated_path.display()
        );
        println!(
            "   All {} subtitles need to be translated.",
            original_subtitles.len()
        );
        return Ok(());
    }

    println!("📖 Reading translation file: {}", translated_path.display());
    let translated_subtitles = SrtParser::parse_file(translated_path)?;
    println!(
        "✅ Found {} subtitles in translation",
        translated_subtitles.len()
    );
    println!();

    // Verifica completezza
    let missing_ids =
        srt_translate::verify_translation_completeness(&original_subtitles, &translated_subtitles);

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
        println!(
            "   All {} subtitles are present with correct line counts.",
            original_subtitles.len()
        );
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
            println!(
                "⚠️  Found {} subtitles with line count discrepancies:",
                line_discrepancies.len()
            );
            println!();
            println!("Subtitle ID | Original Lines | Translated Lines");
            println!("------------|----------------|------------------");

            // Mostra i primi 20 e se ce ne sono di più, mostra un sommario
            let show_count = line_discrepancies.len().min(20);
            for (id, orig_lines, trans_lines) in &line_discrepancies[..show_count] {
                println!(
                    "     {:6} |         {:6} |           {:6}",
                    id, orig_lines, trans_lines
                );
            }

            if line_discrepancies.len() > 20 {
                println!("     ...and {} more", line_discrepancies.len() - 20);
            }
            println!();
            println!(
                "💡 These subtitles may have been incorrectly translated (missing or extra lines)."
            );
        }

        println!();
        println!("💡 Tip: Re-run the translation to fix these issues:");
        println!(
            "   srt-translate -i {} -o {}",
            original_path.display(),
            translated_path.display()
        );
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
