use crate::SubtitleLine;
use crate::normalization::CleanText;

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

    /// Query candidate cues whose start_ms is within [center_ms - max_offset_ms, center_ms + max_offset_ms]
    pub fn find_in_window(&self, center_ms: i64, max_offset_ms: i64) -> &[CandidateCue] {
        let min_time = center_ms - max_offset_ms;
        let max_time = center_ms + max_offset_ms;

        let start_idx = self.candidates.partition_point(|c| c.start_ms < min_time);
        let end_idx = self.candidates.partition_point(|c| c.start_ms <= max_time);

        if start_idx < end_idx {
            &self.candidates[start_idx..end_idx]
        } else {
            &[]
        }
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
}
