use std::collections::HashMap;

use crate::AnchorSuggestion;
use crate::matching::MatchCandidate;

#[derive(Debug, Clone, Copy)]
pub struct LinearModel {
    pub slope: f64,
    pub intercept_ms: f64,
}

impl LinearModel {
    pub fn constant(offset_ms: i64) -> Self {
        Self {
            slope: 1.0,
            intercept_ms: offset_ms as f64,
        }
    }

    pub fn predict(&self, original_ms: i64) -> i64 {
        ((self.slope * original_ms as f64) + self.intercept_ms).round() as i64
    }

    pub fn residual(&self, candidate: &MatchCandidate) -> f64 {
        let predicted = self.predict(candidate.original_start_ms);
        (candidate.transcribed_start_ms - predicted).abs() as f64
    }
}

/// Find the dominant consensus model (either constant offset or linear drift) and inlier suggestions.
pub fn extract_consensus_anchors(
    candidates: &[MatchCandidate],
    min_anchor_spacing_ms: i64,
) -> (Option<LinearModel>, Vec<AnchorSuggestion>) {
    if candidates.is_empty() {
        return (None, Vec::new());
    }

    // 1. Estimate 1D dominant offset via score-weighted sliding window
    let mut best_1d_offset = 0i64;
    let mut best_1d_score = 0.0f64;

    for candidate in candidates {
        let center = candidate.offset_ms();
        let mut cluster_score = 0.0f64;
        for other in candidates {
            if (other.offset_ms() - center).abs() <= 2500 {
                cluster_score += other.score;
            }
        }
        if cluster_score > best_1d_score {
            best_1d_score = cluster_score;
            best_1d_offset = center;
        }
    }

    let model_1d = LinearModel::constant(best_1d_offset);

    // 2. RANSAC 2D linear drift search: t = slope * s + intercept
    let mut best_2d_model: Option<LinearModel> = None;
    let mut best_2d_score = 0.0f64;
    let mut best_2d_inliers_count = 0usize;

    // Only attempt 2D RANSAC if we have enough spread in candidate positions
    let n = candidates.len();
    if n >= 4 {
        for i in 0..n {
            for j in (i + 1)..n {
                let s_diff = candidates[j].original_start_ms - candidates[i].original_start_ms;
                // Candidates must be separated by at least 60 seconds
                if s_diff.abs() < 60_000 {
                    continue;
                }
                let t_diff =
                    candidates[j].transcribed_start_ms - candidates[i].transcribed_start_ms;
                let slope = t_diff as f64 / s_diff as f64;

                // Accept only realistic framerate conversion drift: slope within [0.94, 1.06]
                if !(0.94..=1.06).contains(&slope) {
                    continue;
                }

                let intercept = candidates[i].transcribed_start_ms as f64
                    - (slope * candidates[i].original_start_ms as f64);
                let candidate_model = LinearModel {
                    slope,
                    intercept_ms: intercept,
                };

                let mut inlier_count = 0usize;
                let mut inlier_score = 0.0f64;
                for m in candidates {
                    if candidate_model.residual(m) <= 1500.0 {
                        inlier_count += 1;
                        inlier_score += m.score;
                    }
                }

                if inlier_score > best_2d_score {
                    best_2d_score = inlier_score;
                    best_2d_model = Some(candidate_model);
                    best_2d_inliers_count = inlier_count;
                }
            }
        }
    }

    // 3. Choose between 1D constant offset and 2D drift
    let chosen_model = match best_2d_model {
        Some(m2d)
            if (m2d.slope - 1.0).abs() >= 0.001
                && best_2d_inliers_count >= 3
                && best_2d_score > best_1d_score * 1.15 =>
        {
            m2d
        }
        _ => model_1d,
    };

    // 4. Collect and verify inliers against chosen model
    let inliers: Vec<&MatchCandidate> = candidates
        .iter()
        .filter(|m| chosen_model.residual(m) <= 2000.0)
        .collect();

    // 5. Deduplicate by subtitle_id (keep highest score)
    let mut best_per_sub: HashMap<u32, &MatchCandidate> = HashMap::new();
    for m in inliers {
        let entry = best_per_sub.entry(m.subtitle_id).or_insert(m);
        if m.score > entry.score || (m.score == entry.score && m.similarity > entry.similarity) {
            *entry = m;
        }
    }

    let mut sorted_inliers: Vec<&MatchCandidate> = best_per_sub.into_values().collect();
    sorted_inliers.sort_unstable_by_key(|m| m.original_start_ms);

    // 6. Space anchors by min_anchor_spacing_ms
    let mut suggestions = Vec::new();
    let mut last_original_time: Option<i64> = None;

    for m in sorted_inliers {
        if last_original_time
            .is_none_or(|lt| m.original_start_ms.saturating_sub(lt) >= min_anchor_spacing_ms)
        {
            last_original_time = Some(m.original_start_ms);
            suggestions.push(AnchorSuggestion {
                subtitle_id: m.subtitle_id,
                original_start_ms: m.original_start_ms,
                corrected_time_ms: m.transcribed_start_ms,
                similarity: m.similarity,
                score: m.score,
            });
        }
    }

    (Some(chosen_model), suggestions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_consensus_constant_offset() {
        let mut candidates = Vec::new();
        // True offset = +56_000ms
        for i in 1..=10 {
            candidates.push(MatchCandidate {
                subtitle_id: i,
                original_start_ms: (i as i64) * 60_000,
                transcribed_start_ms: ((i as i64) * 60_000) + 56_000 + ((i % 2 * 20) as i64),
                similarity: 0.85,
                score: 0.80,
            });
        }
        // Add a couple outliers
        candidates.push(MatchCandidate {
            subtitle_id: 100,
            original_start_ms: 120_000,
            transcribed_start_ms: 122_640, // +2.64s outlier!
            similarity: 0.45,
            score: 0.42,
        });

        let (model, suggestions) = extract_consensus_anchors(&candidates, 30_000);
        assert!(model.is_some());
        let m = model.unwrap();
        assert!((m.intercept_ms - 56_000.0).abs() < 200.0);
        assert!((m.slope - 1.0).abs() < 0.01);
        // Outlier must NOT be present
        assert!(!suggestions.iter().any(|s| s.subtitle_id == 100));
        assert!(suggestions.len() >= 5);
    }

    #[test]
    fn test_extract_consensus_linear_drift() {
        let mut candidates = Vec::new();
        // Framerate drift: 23.976 -> 25 fps => slope = 25 / 23.976 ~ 1.04271
        let slope = 25.0 / 23.976;
        let intercept = 10_000.0;

        for i in 1..=10 {
            let orig = (i as i64) * 120_000;
            let trans = ((slope * orig as f64) + intercept).round() as i64;
            candidates.push(MatchCandidate {
                subtitle_id: i,
                original_start_ms: orig,
                transcribed_start_ms: trans,
                similarity: 0.88,
                score: 0.85,
            });
        }

        let (model, suggestions) = extract_consensus_anchors(&candidates, 30_000);
        assert!(model.is_some());
        let m = model.unwrap();
        assert!((m.slope - slope).abs() < 0.005);
        assert!((m.intercept_ms - intercept).abs() < 500.0);
        assert_eq!(suggestions.len(), 10);
    }
}
