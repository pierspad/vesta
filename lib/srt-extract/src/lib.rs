use anyhow::Result;
use srt_parser::Subtitle;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,

    Summary,

    Debug,
}

impl OutputFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Self::Json),
            "summary" => Some(Self::Summary),
            "debug" => Some(Self::Debug),
            _ => None,
        }
    }
}

pub fn extract(subtitles: &HashMap<u32, Subtitle>, format: OutputFormat) -> Result<String> {
    match format {
        OutputFormat::Json => extract_to_json(subtitles),
        OutputFormat::Summary => Ok(extract_to_summary(subtitles)),
        OutputFormat::Debug => Ok(extract_to_debug(subtitles)),
    }
}

pub fn extract_to_json(subtitles: &HashMap<u32, Subtitle>) -> Result<String> {
    serde_json::to_string_pretty(subtitles)
        .map_err(|e| anyhow::anyhow!("Failed to serialize to JSON: {}", e))
}

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

pub fn extract_to_debug(subtitles: &HashMap<u32, Subtitle>) -> String {
    let mut sorted: Vec<_> = subtitles.values().collect();
    sorted.sort_by_key(|s| s.id);
    format!("{:#?}", sorted)
}

#[derive(Debug, Clone)]
pub struct SubtitleStats {
    pub total_count: usize,

    pub total_duration_seconds: f64,

    pub average_duration_seconds: f64,

    pub shortest_text_length: usize,

    pub longest_text_length: usize,

    pub average_text_length: f64,
}

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
        let duration = subtitle.end.total_milliseconds() - subtitle.start.total_milliseconds();
        total_duration += duration as f64 / 1000.0;

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
