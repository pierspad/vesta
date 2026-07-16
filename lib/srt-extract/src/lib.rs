//! # srt-extract
//!
//! Libreria per l'estrazione e la formattazione di dati da file SRT.
//!
//! Questa libreria fornisce funzionalità per:
//! - Estrarre sottotitoli in vari formati (JSON, summary, debug)
//! - Analizzare statistiche sui sottotitoli
//! - Filtrare e trasformare sottotitoli
//!
//! ## Esempio
//!
//! ```rust,no_run
//! use srt_extract::{extract, OutputFormat};
//! use srt_parser::SrtParser;
//!
//! # fn main() -> anyhow::Result<()> {
//! let subtitles = SrtParser::parse_file("movie.srt")?;
//!
//! // Estrai come JSON
//! let json = extract(&subtitles, OutputFormat::Json)?;
//!
//! // Estrai come summary
//! let summary = extract(&subtitles, OutputFormat::Summary)?;
//! # Ok(())
//! # }
//! ```

use anyhow::Result;
use srt_parser::Subtitle;
use std::collections::HashMap;

/// Formati di output supportati
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Output in formato JSON
    Json,
    /// Summary testuale con informazioni essenziali
    Summary,
    /// Debug format con tutti i dettagli
    Debug,
}

impl OutputFormat {
    /// Crea OutputFormat da stringa
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "summary" => Some(Self::Summary),
            "debug" => Some(Self::Debug),
            _ => None,
        }
    }
}

/// Estrae i sottotitoli nel formato specificato
///
/// # Argomenti
///
/// * `subtitles` - HashMap dei sottotitoli da estrarre
/// * `format` - Formato di output desiderato
///
/// # Restituisce
///
/// Stringa formattata secondo il formato richiesto
pub fn extract(subtitles: &HashMap<u32, Subtitle>, format: OutputFormat) -> Result<String> {
    match format {
        OutputFormat::Json => extract_to_json(subtitles),
        OutputFormat::Summary => Ok(extract_to_summary(subtitles)),
        OutputFormat::Debug => Ok(extract_to_debug(subtitles)),
    }
}

/// Estrae i sottotitoli in formato JSON
///
/// # Argomenti
///
/// * `subtitles` - HashMap dei sottotitoli
///
/// # Restituisce
///
/// Stringa JSON formattata
pub fn extract_to_json(subtitles: &HashMap<u32, Subtitle>) -> Result<String> {
    serde_json::to_string_pretty(subtitles)
        .map_err(|e| anyhow::anyhow!("Failed to serialize to JSON: {}", e))
}

/// Estrae i sottotitoli in formato summary
///
/// Crea un riepilogo testuale con:
/// - Numero totale di sottotitoli
/// - Lista di sottotitoli con ID, timestamp e preview del testo
///
/// # Argomenti
///
/// * `subtitles` - HashMap dei sottotitoli
///
/// # Restituisce
///
/// Stringa formattata come summary
pub fn extract_to_summary(subtitles: &HashMap<u32, Subtitle>) -> String {
    let mut lines = vec![format!("Total subtitles: {}\n", subtitles.len())];
    let mut sorted: Vec<_> = subtitles.values().collect();
    sorted.sort_by_key(|s| s.id);

    for sub in sorted {
        lines.push(format!(
            "ID {}: {} --> {} | {}",
            sub.id,
            sub.start.to_srt_string(),
            sub.end.to_srt_string(),
            sub.text.chars().take(50).collect::<String>()
        ));
    }
    lines.join("\n")
}

/// Estrae i sottotitoli in formato debug
///
/// Mostra tutti i dettagli di ogni sottotitolo in formato debug Rust
///
/// # Argomenti
///
/// * `subtitles` - HashMap dei sottotitoli
///
/// # Restituisce
///
/// Stringa formattata in debug format
pub fn extract_to_debug(subtitles: &HashMap<u32, Subtitle>) -> String {
    let mut sorted: Vec<_> = subtitles.values().collect();
    sorted.sort_by_key(|s| s.id);
    format!("{:#?}", sorted)
}

/// Statistiche sui sottotitoli
#[derive(Debug, Clone)]
pub struct SubtitleStats {
    /// Numero totale di sottotitoli
    pub total_count: usize,
    /// Durata totale in secondi
    pub total_duration_seconds: f64,
    /// Durata media di un sottotitolo in secondi
    pub average_duration_seconds: f64,
    /// Sottotitolo più corto (in caratteri)
    pub shortest_text_length: usize,
    /// Sottotitolo più lungo (in caratteri)
    pub longest_text_length: usize,
    /// Lunghezza media del testo
    pub average_text_length: f64,
}

/// Calcola statistiche sui sottotitoli
///
/// # Argomenti
///
/// * `subtitles` - HashMap dei sottotitoli
///
/// # Restituisce
///
/// Struttura SubtitleStats con le statistiche
pub fn calculate_stats(subtitles: &HashMap<u32, Subtitle>) -> SubtitleStats {
    let total_count = subtitles.len();

    if total_count == 0 {
        return SubtitleStats {
            total_count: 0,
            total_duration_seconds: 0.0,
            average_duration_seconds: 0.0,
            shortest_text_length: 0,
            longest_text_length: 0,
            average_text_length: 0.0,
        };
    }

    let mut total_duration = 0.0;
    let mut total_text_length = 0;
    let mut shortest_text_length = usize::MAX;
    let mut longest_text_length = 0;

    for subtitle in subtitles.values() {
        // Calcola durata
        let duration = subtitle.end.total_milliseconds() - subtitle.start.total_milliseconds();
        total_duration += duration as f64 / 1000.0; // Converti in secondi

        // Calcola lunghezze testo
        let text_len = subtitle.text.len();
        total_text_length += text_len;
        shortest_text_length = shortest_text_length.min(text_len);
        longest_text_length = longest_text_length.max(text_len);
    }

    SubtitleStats {
        total_count,
        total_duration_seconds: total_duration,
        average_duration_seconds: total_duration / total_count as f64,
        shortest_text_length,
        longest_text_length,
        average_text_length: total_text_length as f64 / total_count as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use srt_parser::{Subtitle, Timestamp};

    fn create_test_subtitle(id: u32, text: &str, start_ms: u32, end_ms: u32) -> Subtitle {
        Subtitle {
            id,
            start: Timestamp::from_milliseconds(start_ms),
            end: Timestamp::from_milliseconds(end_ms),
            text: text.to_string(),
        }
    }

    #[test]
    fn test_extract_to_json() {
        let mut subs = HashMap::new();
        subs.insert(1, create_test_subtitle(1, "Hello", 0, 1000));

        let json = extract_to_json(&subs).unwrap();
        assert!(json.contains("Hello"));
    }

    #[test]
    fn test_calculate_stats() {
        let mut subs = HashMap::new();
        subs.insert(1, create_test_subtitle(1, "Hello", 0, 2000));
        subs.insert(2, create_test_subtitle(2, "World!", 2000, 4000));

        let stats = calculate_stats(&subs);
        assert_eq!(stats.total_count, 2);
        assert_eq!(stats.total_duration_seconds, 4.0);
        assert_eq!(stats.average_duration_seconds, 2.0);
    }
}
