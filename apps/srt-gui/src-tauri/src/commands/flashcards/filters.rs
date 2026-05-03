use std::collections::HashSet;

use super::types::*;

// ─── Filters ─────────────────────────────────────────────────────────────────

pub(crate) fn apply_filters(lines: &mut [MatchedLine], filters: &SubtitleFilters) {
    let include_set: Option<Vec<String>> = filters.include_words.as_ref().map(|w| {
        w.split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    });

    let exclude_set: Option<Vec<String>> = filters.exclude_words.as_ref().map(|w| {
        w.split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    });

    // Collect texts for duplicate detection
    let mut seen_subs1: HashSet<String> = HashSet::new();
    let mut seen_subs2: HashSet<String> = HashSet::new();

    // Actor filter
    let actor_filter: Option<Vec<String>> = filters.actor_filter.as_ref().map(|a| {
        a.split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    });

    for line in lines.iter_mut() {
        if !line.active {
            continue;
        }

        let text_lower = line.subs1.text.to_lowercase();
        let duration = line.subs1.end_ms - line.subs1.start_ms;

        // Include words filter
        if let Some(ref words) = include_set {
            if !words.iter().any(|w| text_lower.contains(w)) {
                line.active = false;
                continue;
            }
        }

        // Exclude words filter
        if let Some(ref words) = exclude_set {
            if words.iter().any(|w| text_lower.contains(w)) {
                line.active = false;
                continue;
            }
        }

        // Exclude duplicates subs1
        if filters.exclude_duplicates_subs1 {
            let normalized = line.subs1.text.trim().to_string();
            if seen_subs1.contains(&normalized) {
                line.active = false;
                continue;
            }
            seen_subs1.insert(normalized);
        }

        // Exclude duplicates subs2
        if filters.exclude_duplicates_subs2 {
            if let Some(ref s2) = line.subs2 {
                let normalized = s2.text.trim().to_string();
                if seen_subs2.contains(&normalized) {
                    line.active = false;
                    continue;
                }
                seen_subs2.insert(normalized);
            }
        }

        // Min/max character length
        if let Some(min) = filters.min_chars {
            if line.subs1.text.chars().count() < min {
                line.active = false;
                continue;
            }
        }
        if let Some(max) = filters.max_chars {
            if line.subs1.text.chars().count() > max {
                line.active = false;
                continue;
            }
        }

        // Min/max duration
        if let Some(min) = filters.min_duration_ms {
            if duration < min {
                line.active = false;
                continue;
            }
        }
        if let Some(max) = filters.max_duration_ms {
            if duration > max {
                line.active = false;
                continue;
            }
        }

        // Exclude styled lines (ASS)
        if filters.exclude_styled {
            if line.subs1.text.starts_with('{') {
                line.active = false;
                continue;
            }
        }

        // Actor filter (ASS only)
        if let Some(ref actors) = actor_filter {
            if let Some(ref actor) = line.subs1.actor {
                if !actors.iter().any(|a| a == &actor.to_lowercase()) {
                    line.active = false;
                    continue;
                }
            } else {
                // No actor info, filter out if actor filter is set
                line.active = false;
                continue;
            }
        }

        // Only CJK characters
        if filters.only_cjk {
            let has_cjk = line.subs1.text.chars().any(|c| {
                matches!(c,
                    '\u{4E00}'..='\u{9FFF}' |  // CJK Unified Ideographs
                    '\u{3400}'..='\u{4DBF}' |  // CJK Extension A
                    '\u{3040}'..='\u{309F}' |  // Hiragana
                    '\u{30A0}'..='\u{30FF}'    // Katakana
                )
            });
            if !has_cjk {
                line.active = false;
                continue;
            }
        }

        // Remove lines with no subs2 match
        if filters.remove_no_match && line.subs2.is_none() {
            line.active = false;
        }
    }
}

// ─── Sentence Combining ─────────────────────────────────────────────────────

pub(crate) fn combine_sentences(lines: &mut Vec<MatchedLine>, continuation_chars: &str) {
    if continuation_chars.is_empty() {
        return;
    }

    let cont_chars: Vec<char> = continuation_chars.chars().collect();
    let mut i = 0;

    while i + 1 < lines.len() {
        let ends_with_cont = lines[i]
            .subs1
            .text
            .trim_end()
            .chars()
            .last()
            .map(|c| cont_chars.contains(&c))
            .unwrap_or(false);

        if ends_with_cont && lines[i].active && lines[i + 1].active {
            let next_text = lines[i + 1].subs1.text.clone();
            let next_end = lines[i + 1].subs1.end_ms;
            let next_s2 = lines[i + 1].subs2.clone();

            lines[i].subs1.text = format!("{} {}", lines[i].subs1.text, next_text);
            lines[i].subs1.end_ms = next_end;

            // Combine subs2 too if both present
            if let (Some(ref mut s2), Some(next_s2)) = (&mut lines[i].subs2, next_s2) {
                s2.text = format!("{} {}", s2.text, next_s2.text);
                s2.end_ms = next_s2.end_ms;
            }

            lines.remove(i + 1);
            // Reindex
            for (j, m) in lines.iter_mut().enumerate() {
                m.index = j;
            }
            // Don't advance i - might need to combine more
        } else {
            i += 1;
        }
    }
}

// ─── Context Lines ───────────────────────────────────────────────────────────

pub(crate) fn compute_context(lines: &mut Vec<MatchedLine>, ctx: &ContextConfig) {
    if ctx.leading == 0 && ctx.trailing == 0 {
        return;
    }

    let gap_ms = (ctx.max_gap_seconds * 1000.0) as i64;
    let len = lines.len();

    for i in 0..len {
        let mut leading = Vec::new();
        let mut trailing = Vec::new();

        // Leading context
        for j in 1..=ctx.leading {
            if i < j {
                break;
            }
            let prev_idx = i - j;
            let gap = lines[i].subs1.start_ms - lines[prev_idx].subs1.end_ms;
            if gap_ms > 0 && gap > gap_ms {
                break;
            }
            leading.push(prev_idx);
        }
        leading.reverse();

        // Trailing context
        for j in 1..=ctx.trailing {
            let next_idx = i + j;
            if next_idx >= len {
                break;
            }
            let gap = lines[next_idx].subs1.start_ms - lines[i].subs1.end_ms;
            if gap_ms > 0 && gap > gap_ms {
                break;
            }
            trailing.push(next_idx);
        }

        lines[i].leading_context = leading;
        lines[i].trailing_context = trailing;
    }
}

// ─── Span Filter ─────────────────────────────────────────────────────────────

pub(crate) fn apply_span(
    lines: &mut [MatchedLine],
    span_start: Option<i64>,
    span_end: Option<i64>,
) {
    for line in lines.iter_mut() {
        if let Some(start) = span_start {
            if line.subs1.end_ms < start {
                line.active = false;
            }
        }
        if let Some(end) = span_end {
            if line.subs1.start_ms > end {
                line.active = false;
            }
        }
    }
}
