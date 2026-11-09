use crate::normalization::CleanText;
use crate::{ExistingAnchor, SubtitleLine};

#[derive(Debug, Clone)]
pub struct CandidateCue {
    pub subtitle_id: u32,
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: CleanText,
}

#[derive(Debug, Clone)]
pub struct MatchCandidate {
    pub subtitle_id: u32,
    pub original_start_ms: i64,
    pub transcribed_start_ms: i64,
    pub similarity: f64,
    pub score: f64,
}

impl MatchCandidate {
    pub fn offset_ms(&self) -> i64 {
        self.transcribed_start_ms - self.original_start_ms
    }
}

pub struct SubtitleIndex {
    pub candidates: Vec<CandidateCue>,
}

impl SubtitleIndex {
    pub fn build(subtitles: &[SubtitleLine]) -> Self {
        let mut candidates = Vec::new();

        // 1. Single cues
        for sub in subtitles {
            let clean = CleanText::new(&sub.text);
            if !clean.is_empty() {
                candidates.push(CandidateCue {
                    subtitle_id: sub.id,
                    start_ms: sub.start_ms,
                    end_ms: sub.start_ms + 4000,
                    text: clean,
                });
            }
        }

        // 2. Merged 2-cue windows
        for i in 0..subtitles.len().saturating_sub(1) {
            let s1 = &subtitles[i];
            let s2 = &subtitles[i + 1];
            if (s2.start_ms - s1.start_ms).abs() <= 6000 {
                let merged_text = format!("{} {}", s1.text, s2.text);
                let clean = CleanText::new(&merged_text);
                if !clean.is_empty() {
                    candidates.push(CandidateCue {
                        subtitle_id: s1.id,
                        start_ms: s1.start_ms,
                        end_ms: s2.start_ms + 4000,
                        text: clean,
                    });
                }
            }
        }

        // 3. Merged 3-cue windows
        for i in 0..subtitles.len().saturating_sub(2) {
            let s1 = &subtitles[i];
            let s2 = &subtitles[i + 1];
            let s3 = &subtitles[i + 2];
            if (s3.start_ms - s1.start_ms).abs() <= 10000 {
                let merged_text = format!("{} {} {}", s1.text, s2.text, s3.text);
                let clean = CleanText::new(&merged_text);
                if !clean.is_empty() {
                    candidates.push(CandidateCue {
                        subtitle_id: s1.id,
                        start_ms: s1.start_ms,
                        end_ms: s3.start_ms + 4000,
                        text: clean,
                    });
                }
            }
        }

        candidates.sort_unstable_by_key(|c| c.start_ms);
        Self { candidates }
    }

    /// Query candidate cues whose start_ms is within [min_time, max_time]
    pub fn find_in_range(&self, min_time: i64, max_time: i64) -> &[CandidateCue] {
        if min_time > max_time {
            return &[];
        }
        let start_idx = self.candidates.partition_point(|c| c.start_ms < min_time);
        let end_idx = self.candidates.partition_point(|c| c.start_ms <= max_time);

        if start_idx < end_idx {
            &self.candidates[start_idx..end_idx]
        } else {
            &[]
        }
    }

    /// Query candidate cues whose start_ms is within [center_ms - max_offset_ms, center_ms + max_offset_ms]
    pub fn find_in_window(&self, center_ms: i64, max_offset_ms: i64) -> &[CandidateCue] {
        self.find_in_range(center_ms - max_offset_ms, center_ms + max_offset_ms)
    }
}

/// Piecewise linear interpolation prior derived from existing fixed ground-truth anchors.
#[derive(Debug, Clone, Default)]
pub struct AnchorPrior {
    /// Anchors sorted by original_time_ms ascending
    anchors: Vec<ExistingAnchor>,
}

impl AnchorPrior {
    pub fn new(mut anchors: Vec<ExistingAnchor>) -> Self {
        anchors.sort_by_key(|a| a.original_time_ms);
        anchors.dedup_by_key(|a| a.original_time_ms);
        Self { anchors }
    }

    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }

    pub fn len(&self) -> usize {
        self.anchors.len()
    }

    pub fn anchors(&self) -> &[ExistingAnchor] {
        &self.anchors
    }

    /// Calculate the expected offset for a given original subtitle timestamp t.
    ///
    /// - If no anchors, returns None.
    /// - If 1 anchor, returns that anchor's offset.
    /// - If t <= first_anchor, returns first_anchor.offset().
    /// - If t >= last_anchor, returns last_anchor.offset().
    /// - Otherwise, linearly interpolates between surrounding anchors A1 and A2.
    pub fn expected_offset_at_orig(&self, t: i64) -> Option<i64> {
        if self.anchors.is_empty() {
            return None;
        }
        if self.anchors.len() == 1 {
            return Some(self.anchors[0].offset());
        }

        let first = &self.anchors[0];
        let last = &self.anchors[self.anchors.len() - 1];

        if t <= first.original_time_ms {
            return Some(first.offset());
        }
        if t >= last.original_time_ms {
            return Some(last.offset());
        }

        let idx = self.anchors.partition_point(|a| a.original_time_ms <= t);
        let a1 = &self.anchors[idx - 1];
        let a2 = &self.anchors[idx];

        let dt = a2.original_time_ms - a1.original_time_ms;
        if dt == 0 {
            return Some(a1.offset());
        }

        let alpha = (t - a1.original_time_ms) as f64 / (dt as f64);
        let interpolated_offset = a1.offset() as f64 + alpha * (a2.offset() - a1.offset()) as f64;
        Some(interpolated_offset.round() as i64)
    }

    /// Calculate the expected offset for a given target audio timestamp tau.
    ///
    /// - If no anchors, returns None.
    /// - If 1 anchor, returns that anchor's offset.
    /// - If tau <= first_anchor.target_time_ms, returns first_anchor.offset().
    /// - If tau >= last_anchor.target_time_ms, returns last_anchor.offset().
    /// - Otherwise, linearly interpolates between surrounding anchors A1 and A2.
    pub fn expected_offset_at_target(&self, tau: i64) -> Option<i64> {
        if self.anchors.is_empty() {
            return None;
        }
        if self.anchors.len() == 1 {
            return Some(self.anchors[0].offset());
        }

        let first = &self.anchors[0];
        let last = &self.anchors[self.anchors.len() - 1];

        if tau <= first.target_time_ms {
            return Some(first.offset());
        }
        if tau >= last.target_time_ms {
            return Some(last.offset());
        }

        let idx = self.anchors.partition_point(|a| a.target_time_ms <= tau);
        if idx == 0 {
            return Some(first.offset());
        }
        if idx >= self.anchors.len() {
            return Some(last.offset());
        }

        let a1 = &self.anchors[idx - 1];
        let a2 = &self.anchors[idx];

        let d_tau = a2.target_time_ms - a1.target_time_ms;
        if d_tau == 0 {
            return Some(a1.offset());
        }

        let alpha = (tau - a1.target_time_ms) as f64 / (d_tau as f64);
        let interpolated_offset = a1.offset() as f64 + alpha * (a2.offset() - a1.offset()) as f64;
        Some(interpolated_offset.round() as i64)
    }

    /// Calculate expected original subtitle start time for target audio timestamp tau.
    pub fn expected_orig_time(&self, tau: i64) -> Option<i64> {
        self.expected_offset_at_target(tau)
            .map(|off| tau.saturating_sub(off))
    }

    /// Calculate expected target audio timestamp for original subtitle timestamp t.
    pub fn expected_target_time(&self, t: i64) -> Option<i64> {
        self.expected_offset_at_orig(t)
            .map(|off| t.saturating_add(off))
    }

    /// Search bounds for original subtitle time corresponding to audio time tau,
    /// bounded by a local radius (e.g. 15s) and strictly constrained not to cross surrounding anchors.
    pub fn candidate_orig_bounds(&self, tau: i64, local_radius_ms: i64) -> (i64, i64) {
        if self.anchors.is_empty() {
            return (tau - local_radius_ms, tau + local_radius_ms);
        }

        let expected_orig = self.expected_orig_time(tau).unwrap_or(tau);
        let mut min_t = expected_orig - local_radius_ms;
        let mut max_t = expected_orig + local_radius_ms;

        // Clamp to surrounding anchors so candidates never jump across fixed anchors
        for a in &self.anchors {
            if a.target_time_ms < tau {
                if a.original_time_ms > min_t {
                    min_t = a.original_time_ms;
                }
            } else if a.target_time_ms > tau && a.original_time_ms < max_t {
                max_t = a.original_time_ms;
            }
        }

        (min_t, max_t)
    }
}

pub fn calculate_temporal_weight(time_diff_ms: i64, max_offset_ms: i64) -> f64 {
    let diff = time_diff_ms.abs();
    if diff <= 10_000 {
        1.0
    } else if diff >= max_offset_ms {
        0.70
    } else {
        let normalized = (diff - 10_000) as f64 / (max_offset_ms - 10_000).max(1) as f64;
        1.0 - (normalized * 0.30)
    }
}

/// Compute temporal weight based on deviation from the anchor-guided expected offset.
pub fn calculate_guided_temporal_weight(
    actual_offset_ms: i64,
    expected_offset_ms: i64,
    local_radius_ms: i64,
) -> f64 {
    let deviation = (actual_offset_ms - expected_offset_ms).abs();
    if deviation <= 2_000 {
        1.0
    } else if deviation >= local_radius_ms {
        0.65
    } else {
        let normalized = (deviation - 2_000) as f64 / (local_radius_ms - 2_000).max(1) as f64;
        1.0 - (normalized * 0.35)
    }
}

/// Snap only an unambiguous contiguous phrase to measured Whisper token timing.
/// Without reliable timing, retain the segment onset rather than inventing one.
pub fn refine_speech_onset(
    tseg_start_ms: i64,
    tseg_end_ms: i64,
    tokens: &[srt_transcribe::transcribe::TimedToken],
    cand_clean: &CleanText,
) -> i64 {
    let mut bytes = Vec::new();
    let mut onsets = Vec::new();
    for token in tokens {
        bytes.extend_from_slice(&token.bytes);
        onsets.extend(std::iter::repeat_n(token.start_ms, token.bytes.len()));
    }
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return tseg_start_ms;
    };
    let mut words = Vec::new();
    let mut position = 0;
    for raw_word in text.split_whitespace() {
        let offset = text[position..].find(raw_word).unwrap() + position;
        position = offset + raw_word.len();
        for word in CleanText::new(raw_word).tokens {
            words.push((word, onsets[offset]));
        }
    }
    let count = cand_clean.tokens.len();
    if count < 2 || words.len() < 2 {
        return tseg_start_ms;
    }

    // Try exact contiguous match first, then fall back to a unique leading prefix
    // (4 down to 2 words) when Whisper paraphrases or drops trailing words in a cue.
    let mut candidate_lengths = Vec::with_capacity(4);
    if words.len() >= count {
        candidate_lengths.push(count);
    }
    for prefix_len in (2..=count.saturating_sub(1).min(4)).rev() {
        if words.len() >= prefix_len && !candidate_lengths.contains(&prefix_len) {
            candidate_lengths.push(prefix_len);
        }
    }

    for len in candidate_lengths {
        let target_slice = &cand_clean.tokens[..len];
        let mut matches = words
            .windows(len)
            .filter(|window| window.iter().map(|(word, _)| word).eq(target_slice.iter()));
        if let Some(found) = matches.next()
            && matches.next().is_none()
        {
            let onset = found[0].1;
            if (tseg_start_ms..tseg_end_ms).contains(&onset) {
                return onset;
            }
        }
    }
    tseg_start_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subtitle_index_find_in_window() {
        let subs = vec![
            SubtitleLine {
                id: 1,
                start_ms: 10_000,
                text: "First line".into(),
            },
            SubtitleLine {
                id: 2,
                start_ms: 20_000,
                text: "Second line".into(),
            },
            SubtitleLine {
                id: 3,
                start_ms: 70_000,
                text: "Third line".into(),
            },
        ];
        let index = SubtitleIndex::build(&subs);

        // Window of 60s around 15_000 -> covers 10_000, 20_000, 70_000
        let in_win = index.find_in_window(15_000, 60_000);
        assert!(!in_win.is_empty());
        assert!(in_win.iter().any(|c| c.subtitle_id == 3));

        // Window of 5s around 15_000 -> covers only 10_000 to 20_000
        let small_win = index.find_in_window(15_000, 6_000);
        assert!(!small_win.iter().any(|c| c.subtitle_id == 3));
    }

    #[test]
    fn test_anchor_prior_piecewise_interpolation() {
        let anchors = vec![
            ExistingAnchor::new(10_000, 15_000),   // offset +5000ms at t=10s
            ExistingAnchor::new(100_000, 115_000), // offset +15000ms at t=100s
        ];
        let prior = AnchorPrior::new(anchors);

        // Before first anchor: clamps to first anchor's offset (+5000ms)
        assert_eq!(prior.expected_offset_at_orig(5_000), Some(5_000));
        assert_eq!(prior.expected_offset_at_target(8_000), Some(5_000));

        // After last anchor: clamps to last anchor's offset (+15000ms)
        assert_eq!(prior.expected_offset_at_orig(150_000), Some(15_000));
        assert_eq!(prior.expected_offset_at_target(200_000), Some(15_000));

        // Midpoint: t = 55_000 (halfway between 10_000 and 100_000) -> offset = 10_000ms
        assert_eq!(prior.expected_offset_at_orig(55_000), Some(10_000));

        // Target midpoint: tau = 65_000 (halfway between 15_000 and 115_000) -> offset = 10_000ms
        assert_eq!(prior.expected_offset_at_target(65_000), Some(10_000));
        assert_eq!(prior.expected_orig_time(65_000), Some(55_000));
        assert_eq!(prior.expected_target_time(55_000), Some(65_000));
    }

    #[test]
    fn test_anchor_prior_candidate_orig_bounds_never_crosses_anchors() {
        let anchors = vec![
            ExistingAnchor::new(20_000, 25_000),
            ExistingAnchor::new(80_000, 85_000),
        ];
        let prior = AnchorPrior::new(anchors);

        // At audio tau = 55_000, expected orig is 50_000. Radius 15_000 gives [35_000, 65_000].
        // This is strictly inside [20_000, 80_000].
        let (min_t, max_t) = prior.candidate_orig_bounds(55_000, 15_000);
        assert!(min_t >= 20_000);
        assert!(max_t <= 80_000);

        // At audio tau = 28_000, expected orig is 23_000. Radius 15_000 would reach down to 8_000,
        // but it MUST be clamped at the preceding anchor's orig_time (20_000)!
        let (min_t_clamped, _) = prior.candidate_orig_bounds(28_000, 15_000);
        assert_eq!(min_t_clamped, 20_000);
    }

    #[test]
    fn test_refine_speech_onset_uses_measured_timing() {
        use srt_transcribe::transcribe::TimedToken;
        let tokens = vec![
            TimedToken {
                bytes: b"good morning and".to_vec(),
                start_ms: 10_000,
            },
            TimedToken {
                bytes: b" wel".to_vec(),
                start_ms: 12_900,
            },
            TimedToken {
                bytes: b"come to the show".to_vec(),
                start_ms: 13_000,
            },
        ];
        let phrase = CleanText::new("welcome to the show");
        assert_eq!(
            refine_speech_onset(10_000, 14_000, &tokens, &phrase),
            12_900
        );
        assert_eq!(refine_speech_onset(10_000, 14_000, &[], &phrase), 10_000);
        assert_eq!(
            refine_speech_onset(10_000, 14_000, &tokens, &CleanText::new("missing words")),
            10_000
        );
    }

    #[test]
    fn speech_onset_falls_back_for_ambiguous_or_invalid_tokens() {
        use srt_transcribe::transcribe::TimedToken;
        let phrase = CleanText::new("hello world");
        for tokens in [
            vec![TimedToken {
                bytes: b"hello world hello world".to_vec(),
                start_ms: 11_000,
            }],
            vec![TimedToken {
                bytes: b"hello world".to_vec(),
                start_ms: 20_000,
            }],
            vec![TimedToken {
                bytes: vec![0xff],
                start_ms: 11_000,
            }],
        ] {
            assert_eq!(
                refine_speech_onset(10_000, 14_000, &tokens, &phrase),
                10_000
            );
        }
        // Whisper can split a UTF-8 code point across tokens. Reassemble bytes
        // before normalization and retain the onset of the first byte.
        let raw = " café demain".as_bytes();
        let tokens = vec![
            TimedToken {
                bytes: b"bonjour".to_vec(),
                start_ms: 10_000,
            },
            TimedToken {
                bytes: raw[..5].to_vec(),
                start_ms: 12_000,
            },
            TimedToken {
                bytes: raw[5..].to_vec(),
                start_ms: 12_500,
            },
        ];
        assert_eq!(
            refine_speech_onset(10_000, 14_000, &tokens, &CleanText::new("café demain")),
            12_000
        );
    }

    #[test]
    fn test_guided_temporal_weight() {
        // Deviation <= 2000ms gets weight 1.0
        assert_eq!(calculate_guided_temporal_weight(5_000, 5_500, 15_000), 1.0);
        assert_eq!(calculate_guided_temporal_weight(5_000, 7_000, 15_000), 1.0);

        // Deviation >= local_radius gets weight 0.65
        assert_eq!(
            calculate_guided_temporal_weight(5_000, 20_000, 15_000),
            0.65
        );
        assert_eq!(
            calculate_guided_temporal_weight(5_000, 30_000, 15_000),
            0.65
        );

        // Midpoint deviation
        let mid = calculate_guided_temporal_weight(5_000, 13_500, 15_000);
        assert!(mid > 0.65 && mid < 1.0);
    }

    #[test]
    fn refine_speech_onset_falls_back_to_unique_prefix_when_trailing_words_differ() {
        use srt_transcribe::transcribe::TimedToken;
        let tokens = vec![
            TimedToken {
                bytes: b"noise".to_vec(),
                start_ms: 10_000,
            },
            TimedToken {
                bytes: b" commander".to_vec(),
                start_ms: 11_420,
            },
            TimedToken {
                bytes: b" shepard".to_vec(),
                start_ms: 11_700,
            },
            TimedToken {
                bytes: b" report".to_vec(),
                start_ms: 12_050,
            },
            TimedToken {
                bytes: b" immediately".to_vec(),
                start_ms: 12_400,
            },
        ];
        // Subtitle has different trailing words ("to the bridge"), but shares a unique 3-word prefix.
        let phrase = CleanText::new("Commander Shepard report to the bridge");
        assert_eq!(
            refine_speech_onset(10_000, 14_000, &tokens, &phrase),
            11_420
        );
    }
}
