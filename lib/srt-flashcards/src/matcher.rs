use super::types::*;

pub(crate) fn get_overlap(a_start: i64, a_end: i64, b_start: i64, b_end: i64) -> i64 {
    let overlap_start = a_start.max(b_start);
    let overlap_end = a_end.min(b_end);
    overlap_end.saturating_sub(overlap_start).max(0)
}

const LONG_CUE_THRESHOLD_MS: i64 = 30_000;

pub(crate) fn match_subtitles(subs1: &[SubEntry], subs2: &[SubEntry]) -> Vec<MatchedLine> {
    let mut matched: Vec<MatchedLine> = Vec::with_capacity(subs1.len());

    // Separate short and long cues in subs2.
    // If a very long cue (e.g. 1-hour sign or OP) starts at 0, building prefix_max_end
    // over the whole slice would cause partition_point to always return 0, degrading
    // matching to O(N * M).
    let mut short_cues = Vec::with_capacity(subs2.len());
    let mut long_cues = Vec::new();

    for (orig_idx, cue) in subs2.iter().enumerate() {
        if cue.end_ms.saturating_sub(cue.start_ms) <= LONG_CUE_THRESHOLD_MS {
            short_cues.push((orig_idx, cue));
        } else {
            long_cues.push((orig_idx, cue));
        }
    }

    let mut prefix_max_end = Vec::with_capacity(short_cues.len());
    let mut maximum = i64::MIN;
    for &(_, cue) in &short_cues {
        maximum = maximum.max(cue.end_ms);
        prefix_max_end.push(maximum);
    }

    for (i, s1) in subs1.iter().enumerate() {
        let mut best_match: Option<&SubEntry> = None;
        let mut best_overlap: i64 = 0;
        let mut best_orig_idx: usize = usize::MAX;

        let earliest = s1.start_ms.saturating_sub(5000);
        let latest = s1.end_ms.saturating_add(5000);

        let search_start = prefix_max_end.partition_point(|end| *end < earliest);
        for &(orig_idx, s2) in &short_cues[search_start..] {
            let overlap = get_overlap(s1.start_ms, s1.end_ms, s2.start_ms, s2.end_ms);
            if overlap > best_overlap
                || (overlap == best_overlap && overlap > 0 && orig_idx < best_orig_idx)
            {
                best_overlap = overlap;
                best_match = Some(s2);
                best_orig_idx = orig_idx;
            }

            if s2.start_ms > latest {
                break;
            }
        }

        for &(orig_idx, s2) in &long_cues {
            let overlap = get_overlap(s1.start_ms, s1.end_ms, s2.start_ms, s2.end_ms);
            if overlap > best_overlap
                || (overlap == best_overlap && overlap > 0 && orig_idx < best_orig_idx)
            {
                best_overlap = overlap;
                best_match = Some(s2);
                best_orig_idx = orig_idx;
            }

            if s2.start_ms > latest {
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
    if matched.is_empty() {
        return;
    }

    let mut write_idx = 0;
    for read_idx in 1..matched.len() {
        let same_s2 = match (&matched[write_idx].subs2, &matched[read_idx].subs2) {
            (Some(a), Some(b)) => a.id == b.id,
            _ => false,
        };

        if same_s2 {
            let next_end = matched[read_idx].subs1.end_ms;
            let next_text = std::mem::take(&mut matched[read_idx].subs1.text);
            matched[write_idx].subs1.text.reserve(1 + next_text.len());
            matched[write_idx].subs1.text.push(' ');
            matched[write_idx].subs1.text.push_str(&next_text);
            matched[write_idx].subs1.end_ms = matched[write_idx].subs1.end_ms.max(next_end);
        } else {
            write_idx += 1;
            if write_idx != read_idx {
                matched.swap(write_idx, read_idx);
            }
        }
    }

    matched.truncate(write_idx + 1);

    for (j, m) in matched.iter_mut().enumerate() {
        m.index = j;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sub(id: u32, start_ms: i64, end_ms: i64, text: &str) -> SubEntry {
        SubEntry {
            id,
            start_ms,
            end_ms,
            text: text.to_string(),
            actor: None,
            style: None,
            active: true,
        }
    }

    #[test]
    fn test_get_overlap() {
        assert_eq!(
            get_overlap(i64::MIN, i64::MIN + 1, i64::MAX - 1, i64::MAX),
            0
        );
        // Disjoint
        assert_eq!(get_overlap(1000, 2000, 3000, 4000), 0);
        assert_eq!(get_overlap(3000, 4000, 1000, 2000), 0);
        // Partial overlap
        assert_eq!(get_overlap(1000, 3000, 2000, 4000), 1000);
        assert_eq!(get_overlap(2000, 4000, 1000, 3000), 1000);
        // Fully contained
        assert_eq!(get_overlap(1000, 4000, 2000, 3000), 1000);
        assert_eq!(get_overlap(2000, 3000, 1000, 4000), 1000);
        // Touching boundaries (0ms overlap)
        assert_eq!(get_overlap(1000, 2000, 2000, 3000), 0);
    }

    #[test]
    fn combining_nested_target_cues_preserves_the_outer_end() {
        let left = [sub(1, 0, 20_000, "outer"), sub(2, 1_000, 2_000, "inner")];
        let right = [sub(10, 0, 20_000, "translation")];
        let matched = match_subtitles(&left, &right);
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].subs1.end_ms, 20_000);
        assert_eq!(matched[0].subs1.text, "outer inner");
    }

    #[test]
    fn test_match_subtitles_basic() {
        let s1 = vec![sub(1, 1000, 3000, "Hello"), sub(2, 4000, 6000, "World")];
        let s2 = vec![sub(10, 1050, 2950, "Ciao"), sub(20, 3950, 6050, "Mondo")];

        let matched = match_subtitles(&s1, &s2);
        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0].subs1.text, "Hello");
        assert_eq!(matched[0].subs2.as_ref().unwrap().text, "Ciao");
        assert_eq!(matched[1].subs1.text, "World");
        assert_eq!(matched[1].subs2.as_ref().unwrap().text, "Mondo");
    }

    #[test]
    fn long_earlier_cue_is_not_skipped_by_nonmonotonic_end_times() {
        let left = vec![sub(1, 10_000, 11_000, "left")];
        let right = vec![sub(10, 0, 20_000, "long"), sub(11, 1_000, 2_000, "short")];
        let matched = match_subtitles(&left, &right);
        assert_eq!(matched[0].subs2.as_ref().map(|cue| cue.id), Some(10));
    }

    #[test]
    fn matcher_matches_brute_force_reference_for_nested_cues() {
        let left = vec![sub(1, 8_000, 9_000, "a"), sub(2, 40_000, 41_000, "b")];
        let right = vec![
            sub(10, 0, 30_000, "nested-long"),
            sub(11, 1_000, 2_000, "short"),
            sub(12, 7_500, 8_500, "overlap"),
            sub(13, 38_000, 42_000, "later"),
        ];
        let actual = match_subtitles(&left, &right);
        for (i, cue) in left.iter().enumerate() {
            let expected = right
                .iter()
                .filter(|candidate| {
                    candidate.start_ms <= cue.end_ms.saturating_add(5000)
                        && candidate.end_ms >= cue.start_ms.saturating_sub(5000)
                })
                .map(|candidate| {
                    (
                        candidate,
                        get_overlap(
                            cue.start_ms,
                            cue.end_ms,
                            candidate.start_ms,
                            candidate.end_ms,
                        ),
                    )
                })
                .filter(|(_, overlap)| *overlap > 0)
                .fold(None, |best, (candidate, overlap)| match best {
                    Some((_, best_overlap)) if best_overlap >= overlap => best,
                    _ => Some((candidate.id, overlap)),
                })
                .map(|(candidate_id, _)| candidate_id);
            assert_eq!(
                actual[i].subs2.as_ref().map(|candidate| candidate.id),
                expected
            );
        }
    }

    #[test]
    fn test_combine_consecutive_repeats() {
        // Two consecutive s1 matching the same s2 (id 10)
        let s1 = vec![
            sub(1, 1000, 2000, "How"),
            sub(2, 2100, 3000, "are you?"),
            sub(3, 4000, 5000, "I am fine"),
        ];
        let s2 = vec![
            sub(10, 1000, 3000, "Come stai?"),
            sub(20, 4000, 5000, "Sto bene"),
        ];

        let matched = match_subtitles(&s1, &s2);
        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0].subs1.text, "How are you?");
        assert_eq!(matched[0].subs1.start_ms, 1000);
        assert_eq!(matched[0].subs1.end_ms, 3000);
        assert_eq!(matched[0].subs2.as_ref().unwrap().text, "Come stai?");

        assert_eq!(matched[1].subs1.text, "I am fine");
        assert_eq!(matched[1].subs2.as_ref().unwrap().text, "Sto bene");
    }

    fn naive_match_subtitles(subs1: &[SubEntry], subs2: &[SubEntry]) -> Vec<MatchedLine> {
        let mut matched: Vec<MatchedLine> = Vec::with_capacity(subs1.len());
        for (i, s1) in subs1.iter().enumerate() {
            let mut best_match: Option<&SubEntry> = None;
            let mut best_overlap: i64 = 0;
            for s2 in subs2 {
                let overlap = get_overlap(s1.start_ms, s1.end_ms, s2.start_ms, s2.end_ms);
                if overlap > best_overlap {
                    best_overlap = overlap;
                    best_match = Some(s2);
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

    #[test]
    fn long_sign_cue_at_index_zero_matches_naive_and_runs_fast() {
        // Sign cue lasting 1 hour at index 0 (duration 3_600_000 > 30_000)
        let mut subs2 = vec![sub(0, 0, 3_600_000, "1-hour sign/title")];
        // Dialogue cues spanning past the 1-hour sign cue
        for i in 1..=500 {
            let start = i * 8000;
            subs2.push(sub(
                i as u32,
                start,
                start + 2500,
                &format!("Target dialogue {i}"),
            ));
        }

        // subs1 with matching dialogue cues (some within 1-hour, some after 1-hour)
        let mut subs1 = Vec::new();
        for i in 1..=500 {
            let start = i * 8000 + 100;
            subs1.push(sub(
                1000 + i as u32,
                start,
                start + 2000,
                &format!("Native dialogue {i}"),
            ));
        }

        let start_time = std::time::Instant::now();
        let actual = match_subtitles(&subs1, &subs2);
        let elapsed = start_time.elapsed();
        // 500 cues should match in well under 100ms with partitioning
        assert!(
            elapsed.as_millis() < 500,
            "matcher took too long: {:?}",
            elapsed
        );

        // Compare against naive exhaustive matching
        let expected = naive_match_subtitles(&subs1, &subs2);
        assert_eq!(actual.len(), expected.len());
        for (a, e) in actual.iter().zip(&expected) {
            assert_eq!(a.index, e.index);
            assert_eq!(a.subs1.text, e.subs1.text);
            assert_eq!(a.subs1.start_ms, e.subs1.start_ms);
            assert_eq!(a.subs1.end_ms, e.subs1.end_ms);
            assert_eq!(
                a.subs2.as_ref().map(|s| s.id),
                e.subs2.as_ref().map(|s| s.id)
            );
        }
    }
}
