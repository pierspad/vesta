use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub mod encoding;

/// Rappresenta un timestamp SRT in millisecondi
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timestamp {
    pub milliseconds: u64,
}

impl Timestamp {
    /// Crea un timestamp da ore, minuti, secondi e millisecondi
    pub fn new(hours: u32, minutes: u32, seconds: u32, millis: u32) -> Self {
        let total_ms =
            (hours as u64 * 3600 + minutes as u64 * 60 + seconds as u64) * 1000 + millis as u64;
        Self {
            milliseconds: total_ms,
        }
    }

    /// Crea un timestamp direttamente dai millisecondi
    pub fn from_milliseconds(milliseconds: u32) -> Self {
        Self {
            milliseconds: milliseconds as u64,
        }
    }

    /// Restituisce il totale dei millisecondi
    pub fn total_milliseconds(&self) -> u64 {
        self.milliseconds
    }

    /// Parse da formato SRT (00:00:20,000)
    pub fn from_srt_string(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(&[':', ',', '.'][..]).collect();
        let &[hours, minutes, seconds, millis] = parts.as_slice() else {
            anyhow::bail!("Formato timestamp invalido: {}", s);
        };

        let hours: u32 = hours.parse().context("Ore invalide")?;
        let minutes: u32 = minutes.parse().context("Minuti invalidi")?;
        let seconds: u32 = seconds.parse().context("Secondi invalidi")?;
        let millis: u32 = millis.parse().context("Millisecondi invalidi")?;

        Ok(Self::new(hours, minutes, seconds, millis))
    }

    /// Converte in formato SRT
    pub fn to_srt_string(&self) -> String {
        let total_seconds = self.milliseconds / 1000;
        let millis = self.milliseconds % 1000;
        let seconds = total_seconds % 60;
        let total_minutes = total_seconds / 60;
        let minutes = total_minutes % 60;
        let hours = total_minutes / 60;

        format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, millis)
    }
}

/// Rappresenta un singolo sottotitolo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtitle {
    pub id: u32,
    pub start: Timestamp,
    pub end: Timestamp,
    pub text: String,
}

impl Subtitle {
    /// Converte il sottotitolo in formato SRT
    pub fn to_srt_string(&self) -> String {
        format!(
            "{}\n{} --> {}\n{}\n",
            self.id,
            self.start.to_srt_string(),
            self.end.to_srt_string(),
            self.text
        )
    }
}

/// Parser per file SRT
pub struct SrtParser;

impl SrtParser {
    /// Parse un file SRT e ritorna una HashMap con id -> sottotitolo.
    ///
    /// L'encoding del file è rilevato automaticamente (BOM, UTF-8/16,
    /// code page legacy): vedi [`encoding::read_text_auto`].
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<HashMap<u32, Subtitle>> {
        let content = encoding::read_text_auto(path)?;
        Self::parse_string(&content)
    }

    /// Parse una stringa SRT
    pub fn parse_string(content: &str) -> Result<HashMap<u32, Subtitle>> {
        let mut subtitles = HashMap::new();
        // Tollera un BOM residuo e normalizza i line ending prima dello split
        let normalized = content.trim_start_matches('\u{feff}').replace("\r\n", "\n");
        let blocks: Vec<&str> = normalized.split("\n\n").collect();

        for block in blocks {
            let block = block.trim();
            if block.is_empty() {
                continue;
            }

            let subtitle = Self::parse_block(block)?;
            subtitles.insert(subtitle.id, subtitle);
        }

        Ok(subtitles)
    }

    /// Parse un singolo blocco di sottotitolo
    fn parse_block(block: &str) -> Result<Subtitle> {
        let lines: Vec<&str> = block.lines().collect();
        let [id_line, timeline, text_lines @ ..] = lines.as_slice() else {
            anyhow::bail!("Blocco sottotitolo invalido");
        };

        // Parse ID
        let id: u32 = id_line.trim().parse().context("ID invalido")?;

        // Parse timestamps
        let parts: Vec<&str> = timeline.split(" --> ").collect();
        let &[start_str, end_str] = parts.as_slice() else {
            anyhow::bail!("Timeline invalida: {}", timeline);
        };

        let start = Timestamp::from_srt_string(start_str.trim())?;
        let end = Timestamp::from_srt_string(end_str.trim())?;

        // Parse testo (può essere multi-linea, può essere vuoto)
        let text = if !text_lines.is_empty() {
            let t = text_lines.join("\n").trim().to_string();
            if t.is_empty() { "[...]".to_string() } else { t }
        } else {
            "[...]".to_string()
        };

        Ok(Subtitle {
            id,
            start,
            end,
            text,
        })
    }

    /// Normalizza i sottotitoli riempiendo buchi nella numerazione con "[...]".
    /// Se mancano ID (es: 1, 3, 5 oppure il file inizia da 2), vengono creati
    /// sottotitoli placeholder con testo "[...]" per riempire ogni lacuna.
    pub fn normalize_subtitles(subtitles: &mut HashMap<u32, Subtitle>) {
        if subtitles.is_empty() {
            return;
        }

        let max_id = *subtitles.keys().max().unwrap();

        // Raccogli gli ID mancanti e i tempi da interpolare
        let missing: Vec<u32> = (1..=max_id)
            .filter(|id| !subtitles.contains_key(id))
            .collect();

        for id in missing {
            // Cerca il sottotitolo precedente e successivo per interpolare i tempi
            let prev_end = (1..id)
                .rev()
                .find_map(|prev_id| subtitles.get(&prev_id).map(|s| s.end.milliseconds))
                .unwrap_or(0);
            let next_start = (id + 1..=max_id)
                .find_map(|next_id| subtitles.get(&next_id).map(|s| s.start.milliseconds))
                .unwrap_or(prev_end + 1000);

            subtitles.insert(
                id,
                Subtitle {
                    id,
                    start: Timestamp {
                        milliseconds: prev_end,
                    },
                    end: Timestamp {
                        milliseconds: next_start,
                    },
                    text: "[...]".to_string(),
                },
            );
        }
    }

    /// Salva i sottotitoli in un file SRT
    pub fn save_file<P: AsRef<Path>>(path: P, subtitles: &HashMap<u32, Subtitle>) -> Result<()> {
        let mut sorted_subs: Vec<_> = subtitles.values().collect();
        sorted_subs.sort_by_key(|s| s.id);

        let mut content = String::new();
        for (i, sub) in sorted_subs.iter().enumerate() {
            content.push_str(&sub.to_srt_string());
            if i < sorted_subs.len() - 1 {
                content.push('\n');
            }
        }

        fs::write(path, content).context("Impossibile scrivere il file")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_parsing() {
        let ts = Timestamp::from_srt_string("00:00:20,000").unwrap();
        assert_eq!(ts.milliseconds, 20000);
        assert_eq!(ts.to_srt_string(), "00:00:20,000");
    }

    #[test]
    fn test_timestamp_dot_parsing() {
        let ts = Timestamp::from_srt_string("00:00:20.500").unwrap();
        assert_eq!(ts.milliseconds, 20500);
        assert_eq!(ts.to_srt_string(), "00:00:20,500");
    }

    #[test]
    fn test_subtitle_parsing() {
        let content = r#"1
00:00:20,000 --> 00:00:24,400
Ciao mondo!

2
00:00:24,600 --> 00:00:27,800
Come stai?"#;

        let subs = SrtParser::parse_string(content).unwrap();
        assert_eq!(subs.len(), 2);
        assert_eq!(subs.get(&1).unwrap().text, "Ciao mondo!");
    }

    #[test]
    fn test_crlf_subtitle_parsing() {
        let content = "1\r\n00:00:20,000 --> 00:00:24,400\r\nCiao mondo!\r\n\r\n2\r\n00:00:24,600 --> 00:00:27,800\r\nCome stai?";
        let subs = SrtParser::parse_string(content).unwrap();
        assert_eq!(subs.len(), 2);
        assert_eq!(subs.get(&1).unwrap().text, "Ciao mondo!");
        assert_eq!(subs.get(&2).unwrap().text, "Come stai?");
    }
}
