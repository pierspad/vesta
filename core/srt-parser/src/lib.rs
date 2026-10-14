use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub mod encoding;
pub mod persistence;

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
        let trimmed = s.trim();
        let bytes = trimmed.as_bytes();
        // Fast path for standard fixed length format "00:00:00,000" or "00:00:00.000" (12 bytes)
        if bytes.len() == 12
            && bytes[2] == b':'
            && bytes[5] == b':'
            && (bytes[8] == b',' || bytes[8] == b'.')
            && bytes[0].is_ascii_digit()
            && bytes[1].is_ascii_digit()
            && bytes[3].is_ascii_digit()
            && bytes[4].is_ascii_digit()
            && bytes[6].is_ascii_digit()
            && bytes[7].is_ascii_digit()
            && bytes[9].is_ascii_digit()
            && bytes[10].is_ascii_digit()
            && bytes[11].is_ascii_digit()
        {
            let hours = ((bytes[0] - b'0') as u32 * 10) + (bytes[1] - b'0') as u32;
            let minutes = ((bytes[3] - b'0') as u32 * 10) + (bytes[4] - b'0') as u32;
            let seconds = ((bytes[6] - b'0') as u32 * 10) + (bytes[7] - b'0') as u32;
            let millis = ((bytes[9] - b'0') as u32 * 100)
                + ((bytes[10] - b'0') as u32 * 10)
                + (bytes[11] - b'0') as u32;
            return Ok(Self::new(hours, minutes, seconds, millis));
        }

        let mut parts = trimmed.split([':', ',', '.']);
        let hours: u32 = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Formato timestamp invalido: {}", s))?
            .parse()
            .context("Ore invalide")?;
        let minutes: u32 = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Formato timestamp invalido: {}", s))?
            .parse()
            .context("Minuti invalidi")?;
        let seconds: u32 = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Formato timestamp invalido: {}", s))?
            .parse()
            .context("Secondi invalidi")?;
        let millis: u32 = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("Formato timestamp invalido: {}", s))?
            .parse()
            .context("Millisecondi invalidi")?;

        if parts.next().is_some() {
            anyhow::bail!("Formato timestamp invalido: {}", s);
        }

        Ok(Self::new(hours, minutes, seconds, millis))
    }

    /// Scrive il timestamp direttamente in un buffer stringa
    pub fn write_srt_to(&self, out: &mut String) {
        use std::fmt::Write;
        let total_seconds = self.milliseconds / 1000;
        let millis = self.milliseconds % 1000;
        let seconds = total_seconds % 60;
        let total_minutes = total_seconds / 60;
        let minutes = total_minutes % 60;
        let hours = total_minutes / 60;

        let _ = write!(out, "{hours:02}:{minutes:02}:{seconds:02},{millis:03}");
    }

    /// Converte in formato SRT
    pub fn to_srt_string(&self) -> String {
        let mut s = String::with_capacity(12);
        self.write_srt_to(&mut s);
        s
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
    /// Scrive il blocco sottotitolo direttamente in un buffer stringa
    pub fn write_srt_to(&self, out: &mut String) {
        use std::fmt::Write;
        let _ = writeln!(out, "{}", self.id);
        self.start.write_srt_to(out);
        out.push_str(" --> ");
        self.end.write_srt_to(out);
        let _ = writeln!(out, "\n{}", self.text);
    }

    /// Converte il sottotitolo in formato SRT
    pub fn to_srt_string(&self) -> String {
        let mut s = String::with_capacity(32 + self.text.len());
        self.write_srt_to(&mut s);
        s
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
        let trimmed = content.trim_start_matches('\u{feff}');
        let mut current_block_lines: Vec<&str> = Vec::with_capacity(4);

        for line in trimmed.lines() {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                if !current_block_lines.is_empty() {
                    let subtitle = Self::parse_block_lines(&current_block_lines)?;
                    subtitles.insert(subtitle.id, subtitle);
                    current_block_lines.clear();
                }
            } else {
                current_block_lines.push(line);
            }
        }

        if !current_block_lines.is_empty() {
            let subtitle = Self::parse_block_lines(&current_block_lines)?;
            subtitles.insert(subtitle.id, subtitle);
        }

        Ok(subtitles)
    }

    /// Parse un singolo blocco di sottotitolo
    pub fn parse_block(block: &str) -> Result<Subtitle> {
        let lines: Vec<&str> = block.lines().collect();
        Self::parse_block_lines(&lines)
    }

    fn parse_block_lines(lines: &[&str]) -> Result<Subtitle> {
        let [id_line, timeline, text_lines @ ..] = lines else {
            anyhow::bail!("Blocco sottotitolo invalido");
        };

        // Parse ID
        let id: u32 = id_line.trim().parse().context("ID invalido")?;

        // Parse timestamps
        let Some((start_str, end_str)) = timeline.split_once(" --> ") else {
            anyhow::bail!("Timeline invalida: {}", timeline);
        };

        let start = Timestamp::from_srt_string(start_str.trim())?;
        let end = Timestamp::from_srt_string(end_str.trim())?;

        // Parse testo (può essere multi-linea, può essere vuoto)
        let text = if !text_lines.is_empty() {
            let mut t = String::with_capacity(text_lines.len() * 32);
            for (i, line) in text_lines.iter().enumerate() {
                if i > 0 {
                    t.push('\n');
                }
                t.push_str(line);
            }
            let trimmed = t.trim();
            if trimmed.is_empty() {
                "[...]".to_string()
            } else {
                trimmed.to_string()
            }
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

    /// Numero massimo di ID placeholder che siamo disposti a generare in un colpo
    /// solo. Un file malformato con un ID abnorme (es: 999999999) non deve poter
    /// far allocare/iterare l'app fino all'OOM: oltre questa soglia rinunciamo alla
    /// normalizzazione anziché tentare di riempire il buco.
    const MAX_NORMALIZE_GAP: u32 = 50_000;

    /// Normalizza i sottotitoli riempiendo buchi nella numerazione con "[...]".
    /// Se mancano ID (es: 1, 3, 5 oppure il file inizia da 2), vengono creati
    /// sottotitoli placeholder con testo "[...]" per riempire ogni lacuna.
    ///
    /// Se il gap tra il numero di sottotitoli reali e `max_id` supera
    /// [`MAX_NORMALIZE_GAP`] (indice di un file corrotto), la normalizzazione
    /// viene saltata per evitare un'allocazione/iterazione spropositata.
    pub fn normalize_subtitles(subtitles: &mut HashMap<u32, Subtitle>) {
        if subtitles.is_empty() {
            return;
        }

        let mut existing_ids: Vec<u32> = subtitles.keys().copied().collect();
        existing_ids.sort_unstable();

        let max_id = *existing_ids.last().unwrap();
        let gap = max_id as u64 - subtitles.len() as u64;
        if gap > Self::MAX_NORMALIZE_GAP as u64 {
            return;
        }

        let mut placeholders = Vec::new();

        // 1. Buchi iniziali se il primo ID > 1
        let first_id = existing_ids[0];
        if first_id > 1 {
            let next_start = subtitles
                .get(&first_id)
                .map(|s| s.start.milliseconds)
                .unwrap_or(1000);
            for id in 1..first_id {
                placeholders.push((id, 0, next_start));
            }
        }

        // 2. Buchi intermedi tra elementi consecutivi
        for window in existing_ids.windows(2) {
            let curr_id = window[0];
            let next_id = window[1];
            if next_id > curr_id + 1 {
                let prev_end = subtitles
                    .get(&curr_id)
                    .map(|s| s.end.milliseconds)
                    .unwrap_or(0);
                let next_start = subtitles
                    .get(&next_id)
                    .map(|s| s.start.milliseconds)
                    .unwrap_or(prev_end + 1000);
                for id in curr_id + 1..next_id {
                    placeholders.push((id, prev_end, next_start));
                }
            }
        }

        for (id, prev_end, next_start) in placeholders {
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
        sorted_subs.sort_unstable_by_key(|s| s.id);

        let mut content = String::with_capacity(sorted_subs.len() * 64);
        for (i, sub) in sorted_subs.iter().enumerate() {
            sub.write_srt_to(&mut content);
            if i < sorted_subs.len() - 1 {
                content.push('\n');
            }
        }

        persistence::atomic_write(path, content.as_bytes())
            .context("Impossibile scrivere il file")?;
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

    #[test]
    fn test_normalize_subtitles_fills_gaps() {
        let mut subs = HashMap::new();
        subs.insert(
            1,
            Subtitle {
                id: 1,
                start: Timestamp::from_milliseconds(1000),
                end: Timestamp::from_milliseconds(2000),
                text: "First".to_string(),
            },
        );
        subs.insert(
            3,
            Subtitle {
                id: 3,
                start: Timestamp::from_milliseconds(5000),
                end: Timestamp::from_milliseconds(6000),
                text: "Third".to_string(),
            },
        );

        SrtParser::normalize_subtitles(&mut subs);

        assert_eq!(subs.len(), 3);
        assert!(subs.contains_key(&1));
        assert!(subs.contains_key(&2));
        assert!(subs.contains_key(&3));

        let placeholder = subs.get(&2).unwrap();
        assert_eq!(placeholder.text, "[...]");
        assert_eq!(placeholder.start.milliseconds, 2000);
        assert_eq!(placeholder.end.milliseconds, 5000);
    }

    #[test]
    fn test_normalize_subtitles_gap_safety_skips_huge_gap() {
        let mut subs = HashMap::new();
        subs.insert(
            1,
            Subtitle {
                id: 1,
                start: Timestamp::from_milliseconds(1000),
                end: Timestamp::from_milliseconds(2000),
                text: "First".to_string(),
            },
        );
        subs.insert(
            100_000,
            Subtitle {
                id: 100_000,
                start: Timestamp::from_milliseconds(5000),
                end: Timestamp::from_milliseconds(6000),
                text: "Far away".to_string(),
            },
        );

        // Gap is > MAX_NORMALIZE_GAP (50_000). Must NOT allocate 99_998 placeholders.
        SrtParser::normalize_subtitles(&mut subs);
        assert_eq!(subs.len(), 2);
    }

    #[test]
    fn test_invalid_timestamp_returns_err() {
        assert!(Timestamp::from_srt_string("invalid").is_err());
        assert!(Timestamp::from_srt_string("00:00:20").is_err());
        assert!(Timestamp::from_srt_string("00:aa:20,000").is_err());
        assert!(Timestamp::from_srt_string("00:00:20,000:00").is_err());
    }

    #[test]
    fn test_subtitle_to_srt_string_roundtrip() {
        let sub = Subtitle {
            id: 42,
            start: Timestamp::new(1, 23, 45, 678),
            end: Timestamp::new(1, 23, 50, 123),
            text: "Hello\nWorld!".to_string(),
        };

        let srt_str = sub.to_srt_string();
        assert!(srt_str.contains("42"));
        assert!(srt_str.contains("01:23:45,678 --> 01:23:50,123"));
        assert!(srt_str.contains("Hello\nWorld!"));
    }
}
