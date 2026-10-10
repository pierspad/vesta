use std::collections::HashSet;

use super::types::*;

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

    let mut seen_subs1: HashSet<&str> = HashSet::new();
    let mut seen_subs2: HashSet<&str> = HashSet::new();

    let actor_filter: Option<HashSet<String>> = filters.actor_filter.as_ref().map(|a| {
        a.split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    });

    for line in lines.iter_mut() {
        if !line.active {
            continue;
        }

        let duration = line.subs1.end_ms - line.subs1.start_ms;

        if include_set.is_some() || exclude_set.is_some() {
            let text_lower = line.subs1.text.to_lowercase();

            if let Some(ref words) = include_set
                && !words.iter().any(|w| text_lower.contains(w))
            {
                line.active = false;
                continue;
            }

            if let Some(ref words) = exclude_set
                && words.iter().any(|w| text_lower.contains(w))
            {
                line.active = false;
                continue;
            }
        }

        if filters.exclude_duplicates_subs1 {
            let normalized = line.subs1.text.trim();
            if !seen_subs1.insert(normalized) {
                line.active = false;
                continue;
            }
        }

        if filters.exclude_duplicates_subs2
            && let Some(ref s2) = line.subs2
        {
            let normalized = s2.text.trim();
            if !seen_subs2.insert(normalized) {
                line.active = false;
                continue;
            }
        }

        if filters.min_chars.is_some() || filters.max_chars.is_some() {
            let char_count = line.subs1.text.chars().count();
            if let Some(min) = filters.min_chars
                && char_count < min
            {
                line.active = false;
                continue;
            }
            if let Some(max) = filters.max_chars
                && char_count > max
            {
                line.active = false;
                continue;
            }
        }

        if let Some(min) = filters.min_duration_ms
            && duration < min
        {
            line.active = false;
            continue;
        }
        if let Some(max) = filters.max_duration_ms
            && duration > max
        {
            line.active = false;
            continue;
        }

        if filters.exclude_styled && line.subs1.text.starts_with('{') {
            line.active = false;
            continue;
        }

        if let Some(ref actors) = actor_filter {
            let Some(ref actor) = line.subs1.actor else {
                line.active = false;
                continue;
            };
            let actor_lower = actor.to_lowercase();
            if !actors.contains(&actor_lower) {
                line.active = false;
                continue;
            }
        }

        if filters.only_cjk {
            let has_cjk = line.subs1.text.chars().any(|c| {
                matches!(c,
                    '\u{4E00}'..='\u{9FFF}' |
                    '\u{3400}'..='\u{4DBF}' |
                    '\u{3040}'..='\u{309F}' |
                    '\u{30A0}'..='\u{30FF}'
                )
            });
            if !has_cjk {
                line.active = false;
                continue;
            }
        }

        if filters.remove_no_match && line.subs2.is_none() {
            line.active = false;
        }
    }
}

pub(crate) fn combine_sentences(lines: &mut Vec<MatchedLine>, continuation_chars: &str) {
    if continuation_chars.is_empty() || lines.is_empty() {
        return;
    }

    let cont_chars: Vec<char> = continuation_chars.chars().collect();
    let mut write_idx = 0;

    for read_idx in 1..lines.len() {
        let ends_with_cont = lines[write_idx]
            .subs1
            .text
            .trim_end()
            .chars()
            .last()
            .map(|c| cont_chars.contains(&c))
            .unwrap_or(false);

        if ends_with_cont && lines[write_idx].active && lines[read_idx].active {
            let next_end = lines[read_idx].subs1.end_ms;
            let next_text = std::mem::take(&mut lines[read_idx].subs1.text);

            lines[write_idx].subs1.text.reserve(1 + next_text.len());
            lines[write_idx].subs1.text.push(' ');
            lines[write_idx].subs1.text.push_str(&next_text);
            lines[write_idx].subs1.end_ms = next_end;

            let next_s2 = lines[read_idx].subs2.take();
            if let (Some(s2), Some(next_s2)) = (&mut lines[write_idx].subs2, next_s2) {
                s2.text.reserve(1 + next_s2.text.len());
                s2.text.push(' ');
                s2.text.push_str(&next_s2.text);
                s2.end_ms = next_s2.end_ms;
            }
        } else {
            write_idx += 1;
            if write_idx != read_idx {
                lines.swap(write_idx, read_idx);
            }
        }
    }

    lines.truncate(write_idx + 1);

    for (j, m) in lines.iter_mut().enumerate() {
        m.index = j;
    }
}

pub(crate) fn compute_context(lines: &mut [MatchedLine], ctx: &ContextConfig) {
    if ctx.leading == 0 && ctx.trailing == 0 {
        return;
    }

    let gap_ms = (ctx.max_gap_seconds * 1000.0) as i64;
    let len = lines.len();

    for i in 0..len {
        let mut leading = Vec::new();
        let mut trailing = Vec::new();

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

pub(crate) fn apply_span(
    lines: &mut [MatchedLine],
    span_start: Option<i64>,
    span_end: Option<i64>,
) {
    for line in lines.iter_mut() {
        if let Some(start) = span_start
            && line.subs1.end_ms < start
        {
            line.active = false;
        }
        if let Some(end) = span_end
            && line.subs1.start_ms > end
        {
            line.active = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matched_line(idx: usize, start_ms: i64, end_ms: i64, text: &str) -> MatchedLine {
        MatchedLine {
            index: idx,
            subs1: SubEntry {
                id: idx as u32 + 1,
                start_ms,
                end_ms,
                text: text.to_string(),
                actor: None,
                style: None,
                active: true,
            },
            subs2: None,
            active: true,
            leading_context: Vec::new(),
            trailing_context: Vec::new(),
        }
    }

    #[test]
    fn test_apply_filters_cjk() {
        let mut lines = vec![
            matched_line(0, 1000, 2000, "English only"),
            matched_line(1, 2000, 3000, "こんにちは (Japanese)"),
            matched_line(2, 3000, 4000, "你好 (Chinese)"),
        ];

        let filters = SubtitleFilters {
            only_cjk: true,
            ..Default::default()
        };

        apply_filters(&mut lines, &filters);
        assert!(!lines[0].active);
        assert!(lines[1].active);
        assert!(lines[2].active);
    }

    #[test]
    fn test_apply_filters_include_exclude_words() {
        let mut lines = vec![
            matched_line(0, 1000, 2000, "I love rust programming"),
            matched_line(1, 2000, 3000, "I hate bugs in code"),
            matched_line(2, 3000, 4000, "Just regular dialogue"),
        ];

        let filters = SubtitleFilters {
            include_words: Some("rust, dialogue".to_string()),
            exclude_words: Some("bugs, bad".to_string()),
            ..Default::default()
        };

        apply_filters(&mut lines, &filters);
        assert!(lines[0].active); // contains "rust"
        assert!(!lines[1].active); // does not contain include_words
        assert!(lines[2].active); // contains "dialogue"
    }

    #[test]
    fn test_apply_filters_duplicates() {
        let mut lines = vec![
            matched_line(0, 1000, 2000, "Repeated text"),
            matched_line(1, 2000, 3000, "Repeated text"),
            matched_line(2, 3000, 4000, "Unique text"),
        ];

        let filters = SubtitleFilters {
            exclude_duplicates_subs1: true,
            ..Default::default()
        };

        apply_filters(&mut lines, &filters);
        assert!(lines[0].active);
        assert!(!lines[1].active); // Duplicate excluded
        assert!(lines[2].active);
    }

    #[test]
    fn test_combine_sentences_multiple() {
        let mut lines = vec![
            matched_line(0, 1000, 2000, "Wait for it..."),
            matched_line(1, 2100, 3000, "almost there..."),
            matched_line(2, 3100, 4000, "done!"),
            matched_line(3, 5000, 6000, "Next sentence."),
        ];

        combine_sentences(&mut lines, ".,-");
        // Lines 0, 1, 2 should merge into one line
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].subs1.text, "Wait for it... almost there... done!");
        assert_eq!(lines[0].subs1.start_ms, 1000);
        assert_eq!(lines[0].subs1.end_ms, 4000);
        assert_eq!(lines[1].subs1.text, "Next sentence.");
    }

    #[test]
    fn test_compute_context_with_gap_limit() {
        let mut lines = vec![
            matched_line(0, 1000, 2000, "First"),
            matched_line(1, 2500, 3500, "Second"), // gap from 0 is 500ms
            matched_line(2, 10000, 11000, "Third"), // gap from 1 is 6500ms (exceeds max_gap)
        ];

        let ctx = ContextConfig {
            leading: 2,
            trailing: 2,
            max_gap_seconds: 2.0,
        };

        compute_context(&mut lines, &ctx);
        // Line 1 should have leading context [0]
        assert_eq!(lines[1].leading_context, vec![0]);
        // Line 2 should NOT have leading context because gap is 6.5s > 2.0s
        assert_eq!(lines[2].leading_context, Vec::<usize>::new());
    }

    #[test]
    fn test_apply_span() {
        let mut lines = vec![
            matched_line(0, 1000, 2000, "Early"),
            matched_line(1, 4000, 5000, "Middle"),
            matched_line(2, 8000, 9000, "Late"),
        ];

        apply_span(&mut lines, Some(3000), Some(6000));
        assert!(!lines[0].active);
        assert!(lines[1].active);
        assert!(!lines[2].active);
    }
}
