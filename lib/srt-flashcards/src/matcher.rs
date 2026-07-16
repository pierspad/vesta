use super::types::*;

pub(crate) fn get_overlap(a_start: i64, a_end: i64, b_start: i64, b_end: i64) -> i64 {
    let overlap_start = a_start.max(b_start);
    let overlap_end = a_end.min(b_end);
    (overlap_end - overlap_start).max(0)
}

pub(crate) fn match_subtitles(subs1: &[SubEntry], subs2: &[SubEntry]) -> Vec<MatchedLine> {
    let mut matched: Vec<MatchedLine> = Vec::with_capacity(subs1.len());

    for (i, s1) in subs1.iter().enumerate() {
        let mut best_match: Option<&SubEntry> = None;
        let mut best_overlap: i64 = 0;

        let search_start = subs2.partition_point(|s2| s2.end_ms < s1.start_ms.saturating_sub(5000));

        for s2 in &subs2[search_start..] {
            let overlap = get_overlap(s1.start_ms, s1.end_ms, s2.start_ms, s2.end_ms);
            if overlap > best_overlap {
                best_overlap = overlap;
                best_match = Some(s2);
            }

            if s2.start_ms > s1.end_ms + 5000 {
                break;
            }
        }

        matched.push(MatchedLine {
            index: i,
            subs1: s1.clone(),
            subs2: best_match.cloned(),
            active: s1.active,
            leading_context: Vec::new(),
            trailing_context: Vec::new(),
        });
    }

    combine_consecutive_repeats(&mut matched);

    matched
}

pub(crate) fn combine_consecutive_repeats(matched: &mut Vec<MatchedLine>) {
    let mut i = 0;
    while i + 1 < matched.len() {
        let same_s2 = match (&matched[i].subs2, &matched[i + 1].subs2) {
            (Some(a), Some(b)) => a.id == b.id,
            _ => false,
        };

        if same_s2 {
            let next_text = matched[i + 1].subs1.text.clone();
            let next_end = matched[i + 1].subs1.end_ms;
            matched[i].subs1.text = format!("{} {}", matched[i].subs1.text, next_text);
            matched[i].subs1.end_ms = next_end;
            matched.remove(i + 1);

            for (j, m) in matched.iter_mut().enumerate() {
                m.index = j;
            }
        } else {
            i += 1;
        }
    }
}
