use anyhow::{Context as _, Result};
use std::path::Path;

use super::types::*;

// ─── Subtitle Parsing ────────────────────────────────────────────────────────

/// Detect format from extension
pub(crate) fn detect_format(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "srt" => "srt",
        "ass" | "ssa" => "ass",
        "vtt" | "webvtt" => "vtt",
        "lrc" => "lrc",
        _ => "srt",
    }
}

/// Parse SRT file into SubEntry vec
pub(crate) fn parse_srt(content: &str) -> Result<Vec<SubEntry>> {
    let mut entries = Vec::new();
    // Normalize CRLF → LF so that block splitting works for Windows-encoded files
    let normalized = content.replace("\r\n", "\n");
    let blocks: Vec<&str> = normalized.split("\n\n").collect();

    for block in blocks {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }

        let lines: Vec<&str> = block.lines().collect();
        if lines.len() < 2 {
            continue;
        }

        let id: u32 = match lines[0].trim().parse() {
            Ok(id) => id,
            Err(_) => continue,
        };

        let timeline = lines[1];
        let parts: Vec<&str> = timeline.split(" --> ").collect();
        if parts.len() != 2 {
            continue;
        }

        let start_ms = parse_srt_timestamp(parts[0].trim())?;
        let end_ms = parse_srt_timestamp(parts[1].trim())?;

        let text = if lines.len() > 2 {
            lines[2..].join("\n").trim().to_string()
        } else {
            String::new()
        };

        if text.is_empty() {
            continue;
        }

        entries.push(SubEntry {
            id,
            start_ms,
            end_ms,
            text,
            actor: None,
            style: None,
            active: true,
        });
    }

    entries.sort_by_key(|e| e.start_ms);
    Ok(entries)
}

pub(crate) fn parse_srt_timestamp(s: &str) -> Result<i64> {
    let parts: Vec<&str> = s.split(&[':', ','][..]).collect();
    if parts.len() != 4 {
        anyhow::bail!("Invalid SRT timestamp: {}", s);
    }
    let h: i64 = parts[0].parse().context("Invalid hours")?;
    let m: i64 = parts[1].parse().context("Invalid minutes")?;
    let sec: i64 = parts[2].parse().context("Invalid seconds")?;
    let ms: i64 = parts[3].parse().context("Invalid milliseconds")?;
    Ok(h * 3600_000 + m * 60_000 + sec * 1000 + ms)
}

/// Parse ASS/SSA file
pub(crate) fn parse_ass(content: &str) -> Result<Vec<SubEntry>> {
    let mut entries = Vec::new();
    let mut in_events = false;
    let mut format_fields: Vec<String> = Vec::new();
    let mut id_counter: u32 = 1;

    for line in content.lines() {
        let line = line.trim();

        if line.eq_ignore_ascii_case("[Events]") {
            in_events = true;
            continue;
        }

        if line.starts_with('[') && in_events {
            break; // New section
        }

        if !in_events {
            continue;
        }

        if line.starts_with("Format:") {
            let fields_str = line.strip_prefix("Format:").unwrap_or("");
            format_fields = fields_str
                .split(',')
                .map(|f| f.trim().to_lowercase())
                .collect();
            continue;
        }

        if line.starts_with("Dialogue:") || line.starts_with("Comment:") {
            let is_comment = line.starts_with("Comment:");
            if is_comment {
                continue;
            }

            let data = line.splitn(2, ':').nth(1).unwrap_or("").trim();
            let parts: Vec<&str> = data.splitn(format_fields.len().max(1), ',').collect();

            let get_field = |name: &str| -> Option<String> {
                format_fields
                    .iter()
                    .position(|f| f == name)
                    .and_then(|i| parts.get(i))
                    .map(|s| s.trim().to_string())
            };

            let start_str = get_field("start").unwrap_or_default();
            let end_str = get_field("end").unwrap_or_default();
            let actor = get_field("name").or_else(|| get_field("actor"));
            let style = get_field("style");

            // Text is the last field and may contain commas
            let text_field_idx = format_fields.iter().position(|f| f == "text");
            let text = if let Some(idx) = text_field_idx {
                if idx < parts.len() {
                    // Rejoin everything from this index forward
                    parts[idx..].join(",").trim().to_string()
                } else {
                    String::new()
                }
            } else {
                parts
                    .last()
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default()
            };

            // Strip ASS formatting tags like {\b1}, {\an8}, etc.
            let text = strip_ass_tags(&text)
                .replace("\\N", "\n")
                .replace("\\n", "\n");

            if text.trim().is_empty() {
                continue;
            }

            let start_ms = parse_ass_timestamp(&start_str).unwrap_or(0);
            let end_ms = parse_ass_timestamp(&end_str).unwrap_or(0);

            entries.push(SubEntry {
                id: id_counter,
                start_ms,
                end_ms,
                text: text.trim().to_string(),
                actor,
                style,
                active: true,
            });
            id_counter += 1;
        }
    }

    entries.sort_by_key(|e| e.start_ms);
    Ok(entries)
}

pub(crate) fn parse_ass_timestamp(s: &str) -> Result<i64> {
    // Format: H:MM:SS.CC (centiseconds)
    let parts: Vec<&str> = s.split(&[':', '.'][..]).collect();
    if parts.len() != 4 {
        anyhow::bail!("Invalid ASS timestamp: {}", s);
    }
    let h: i64 = parts[0].parse().unwrap_or(0);
    let m: i64 = parts[1].parse().unwrap_or(0);
    let sec: i64 = parts[2].parse().unwrap_or(0);
    let cs: i64 = parts[3].parse().unwrap_or(0);
    Ok(h * 3600_000 + m * 60_000 + sec * 1000 + cs * 10)
}

pub(crate) fn strip_ass_tags(text: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' && chars.peek() == Some(&'\\') {
            in_tag = true;
            continue;
        }
        if in_tag {
            if c == '}' {
                in_tag = false;
            }
            continue;
        }
        result.push(c);
    }
    result
}

/// Parse WebVTT file
pub(crate) fn parse_vtt(content: &str) -> Result<Vec<SubEntry>> {
    let mut entries = Vec::new();
    let mut id_counter: u32 = 1;

    // Skip WEBVTT header
    let content = content.trim_start_matches('\u{feff}'); // BOM
    let blocks: Vec<&str> = content.split("\n\n").collect();

    for block in blocks {
        let block = block.trim();
        if block.is_empty() || block.starts_with("WEBVTT") || block.starts_with("NOTE") {
            continue;
        }

        let lines: Vec<&str> = block.lines().collect();
        let mut timeline_idx = 0;

        // Find timeline (contains " --> ")
        for (i, line) in lines.iter().enumerate() {
            if line.contains(" --> ") {
                timeline_idx = i;
                break;
            }
        }

        let parts: Vec<&str> = lines[timeline_idx].split(" --> ").collect();
        if parts.len() != 2 {
            continue;
        }

        // VTT timestamps can be MM:SS.mmm or HH:MM:SS.mmm
        let start_ms = parse_vtt_timestamp(parts[0].trim())?;
        let end_ms = parse_vtt_timestamp(parts[1].split_whitespace().next().unwrap_or("").trim())?;

        let text = if timeline_idx + 1 < lines.len() {
            lines[timeline_idx + 1..].join("\n").trim().to_string()
        } else {
            String::new()
        };

        // Strip VTT tags like <b>, <i>, <c.colorname>, etc.
        let text = strip_vtt_tags(&text);

        if text.is_empty() {
            continue;
        }

        entries.push(SubEntry {
            id: id_counter,
            start_ms,
            end_ms,
            text,
            actor: None,
            style: None,
            active: true,
        });
        id_counter += 1;
    }

    entries.sort_by_key(|e| e.start_ms);
    Ok(entries)
}

pub(crate) fn parse_vtt_timestamp(s: &str) -> Result<i64> {
    let parts: Vec<&str> = s.split(&[':', '.'][..]).collect();
    match parts.len() {
        // MM:SS.mmm
        3 => {
            let m: i64 = parts[0].parse().unwrap_or(0);
            let sec: i64 = parts[1].parse().unwrap_or(0);
            let ms: i64 = parts[2].parse().unwrap_or(0);
            Ok(m * 60_000 + sec * 1000 + ms)
        }
        // HH:MM:SS.mmm
        4 => {
            let h: i64 = parts[0].parse().unwrap_or(0);
            let m: i64 = parts[1].parse().unwrap_or(0);
            let sec: i64 = parts[2].parse().unwrap_or(0);
            let ms: i64 = parts[3].parse().unwrap_or(0);
            Ok(h * 3600_000 + m * 60_000 + sec * 1000 + ms)
        }
        _ => anyhow::bail!("Invalid VTT timestamp: {}", s),
    }
}

pub(crate) fn strip_vtt_tags(text: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in text.chars() {
        if c == '<' {
            in_tag = true;
            continue;
        }
        if c == '>' {
            in_tag = false;
            continue;
        }
        if !in_tag {
            result.push(c);
        }
    }
    result.trim().to_string()
}

/// Parse any supported subtitle file
pub(crate) fn parse_subtitle_file(path: &str) -> Result<(Vec<SubEntry>, &'static str)> {
    let content = std::fs::read_to_string(path)
        .or_else(|_| -> Result<String> {
            // Try common encodings
            let bytes = std::fs::read(path)?;
            // Try UTF-8 with BOM
            if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                Ok(String::from_utf8_lossy(&bytes[3..]).to_string())
            } else {
                // Try Latin-1 as fallback
                Ok(bytes.iter().map(|&b| b as char).collect())
            }
        })
        .context(format!("Cannot read file: {}", path))?;

    let format = detect_format(path);
    let entries = match format {
        "ass" => parse_ass(&content)?,
        "vtt" => parse_vtt(&content)?,
        _ => parse_srt(&content)?,
    };

    Ok((entries, format))
}
