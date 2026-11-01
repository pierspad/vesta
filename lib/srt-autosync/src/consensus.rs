use std::collections::HashMap;

use crate::matching::{AnchorPrior, MatchCandidate};
use crate::{AnchorSuggestion, ExistingAnchor};

#[derive(Debug, Clone, Copy, PartialEq)]
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

/// Minimum spacing required from existing fixed manual anchors (15 seconds).
const MIN_DISTANCE_FROM_EXISTING_MS: i64 = 15_000;

/// Find dominant consensus model and inlier suggestions without prior anchors.
pub fn extract_consensus_anchors(
    candidates: &[MatchCandidate],
    min_anchor_spacing_ms: i64,
) -> (Option<LinearModel>, Vec<AnchorSuggestion>) {
    extract_consensus_anchors_guided(candidates, min_anchor_spacing_ms, &[])
}

/// Find dominant consensus model and inlier suggestions, guided and constrained by
/// existing fixed ground-truth anchors.
pub fn extract_consensus_anchors_guided(
    candidates: &[MatchCandidate],
    min_anchor_spacing_ms: i64,
    existing_anchors: &[ExistingAnchor],
) -> (Option<LinearModel>, Vec<AnchorSuggestion>) {
    if candidates.is_empty() {
        return (None, Vec::new());
    }

    if existing_anchors.is_empty() {
        return extract_unconstrained_consensus(candidates, min_anchor_spacing_ms);
    }

    extract_constrained_consensus(candidates, min_anchor_spacing_ms, existing_anchors)
}

fn extract_unconstrained_consensus(
    candidates: &[MatchCandidate],
    min_anchor_spacing_ms: i64,
) -> (Option<LinearModel>, Vec<AnchorSuggestion>) {
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

    let n = candidates.len();
    if n >= 4 {
        for i in 0..n {
            for j in (i + 1)..n {
                let s_diff = candidates[j].original_start_ms - candidates[i].original_start_ms;
                if s_diff.abs() < 60_000 {
                    continue;
                }
                let t_diff =
                    candidates[j].transcribed_start_ms - candidates[i].transcribed_start_ms;
                let slope = t_diff as f64 / s_diff as f64;

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

fn extract_constrained_consensus(
    candidates: &[MatchCandidate],
    min_anchor_spacing_ms: i64,
    existing_anchors: &[ExistingAnchor],
) -> (Option<LinearModel>, Vec<AnchorSuggestion>) {
    let prior = AnchorPrior::new(existing_anchors.to_vec());

    // Evaluate inliers against the piecewise linear prior
    let inliers: Vec<&MatchCandidate> = candidates
        .iter()
        .filter(|m| {
            if let Some(expected_target) = prior.expected_target_time(m.original_start_ms) {
                (m.transcribed_start_ms - expected_target).abs() <= 2000
            } else {
                false
            }
        })
        .collect();

    // Deduplicate by subtitle_id (highest score)
    let mut best_per_sub: HashMap<u32, &MatchCandidate> = HashMap::new();
    for m in inliers {
        let entry = best_per_sub.entry(m.subtitle_id).or_insert(m);
        if m.score > entry.score || (m.score == entry.score && m.similarity > entry.similarity) {
            *entry = m;
        }
    }

    let mut sorted_inliers: Vec<&MatchCandidate> = best_per_sub.into_values().collect();
    sorted_inliers.sort_unstable_by_key(|m| m.original_start_ms);

    // Filter against Immutable Anchor Constraints:
    // 1. Maintain at least 15s distance from all existing user anchors.
    // 2. Monotonicity constraint: strictly maintain chronological ordering with all user anchors.
    let mut valid_candidates = Vec::new();
    for c in sorted_inliers {
        let too_close = existing_anchors.iter().any(|a| {
            (a.original_time_ms - c.original_start_ms).abs() < MIN_DISTANCE_FROM_EXISTING_MS
                || (a.target_time_ms - c.transcribed_start_ms).abs() < MIN_DISTANCE_FROM_EXISTING_MS
        });
        if too_close {
            continue;
        }

        let violates_monotonicity = existing_anchors.iter().any(|a| {
            (c.original_start_ms < a.original_time_ms && c.transcribed_start_ms >= a.target_time_ms)
                || (c.original_start_ms > a.original_time_ms
                    && c.transcribed_start_ms <= a.target_time_ms)
        });
        if violates_monotonicity {
            continue;
        }

        valid_candidates.push(c);
    }

    // Space suggestions by min_anchor_spacing_ms while maintaining strict mutual monotonicity
    let mut suggestions = Vec::new();
    let mut last_cand: Option<&MatchCandidate> = None;

    for c in valid_candidates {
        if let Some(last) = last_cand {
            if c.original_start_ms.saturating_sub(last.original_start_ms) < min_anchor_spacing_ms {
                continue;
            }
            if c.transcribed_start_ms <= last.transcribed_start_ms {
                continue;
            }
        }
        last_cand = Some(c);
        suggestions.push(AnchorSuggestion {
            subtitle_id: c.subtitle_id,
            original_start_ms: c.original_start_ms,
            corrected_time_ms: c.transcribed_start_ms,
            similarity: c.similarity,
            score: c.score,
        });
    }

    // Reference linear model
    let model = if let Some(first) = existing_anchors.first() {
        if existing_anchors.len() == 1 {
            Some(LinearModel::constant(first.offset()))
        } else {
            let last = existing_anchors.last().unwrap();
            let s_diff = last.original_time_ms - first.original_time_ms;
            let t_diff = last.target_time_ms - first.target_time_ms;
            if s_diff > 0 {
                let slope = t_diff as f64 / s_diff as f64;
                let intercept =
                    first.target_time_ms as f64 - (slope * first.original_time_ms as f64);
                Some(LinearModel {
                    slope,
                    intercept_ms: intercept,
                })
            } else {
                Some(LinearModel::constant(first.offset()))
            }
        }
    } else {
        None
    };

    (model, suggestions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_consensus_constant_offset() {
        let mut candidates = Vec::new();
        for i in 1..=10 {
            candidates.push(MatchCandidate {
                subtitle_id: i,
                original_start_ms: (i as i64) * 60_000,
                transcribed_start_ms: ((i as i64) * 60_000) + 56_000 + ((i % 2 * 20) as i64),
                similarity: 0.85,
                score: 0.80,
            });
        }
        candidates.push(MatchCandidate {
            subtitle_id: 100,
            original_start_ms: 120_000,
            transcribed_start_ms: 122_640,
            similarity: 0.45,
            score: 0.42,
        });

        let (model, suggestions) = extract_consensus_anchors(&candidates, 30_000);
        assert!(model.is_some());
        let m = model.unwrap();
        assert!((m.intercept_ms - 56_000.0).abs() < 200.0);
        assert!((m.slope - 1.0).abs() < 0.01);
        assert!(!suggestions.iter().any(|s| s.subtitle_id == 100));
        assert!(suggestions.len() >= 5);
    }

    #[test]
    fn test_extract_consensus_linear_drift() {
        let mut candidates = Vec::new();
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

    #[test]
    fn test_extract_consensus_with_existing_anchors_respects_constraints() {
        let existing = vec![
            ExistingAnchor::new(60_000, 65_000),   // Anchor 1: offset +5000ms
            ExistingAnchor::new(300_000, 315_000), // Anchor 2: offset +15000ms
        ];

        let candidates = vec![
            // 1. Candidate within 10s of existing anchor 1 (should be filtered by MIN_DISTANCE_FROM_EXISTING_MS)
            MatchCandidate {
                subtitle_id: 1,
                original_start_ms: 68_000,
                transcribed_start_ms: 73_500,
                similarity: 0.9,
                score: 0.9,
            },
            // 2. Inlier at t=120_000. Piecewise expected offset:
            // alpha = (120 - 60) / (300 - 60) = 60 / 240 = 0.25
            // expected offset = 5000 + 0.25 * (15000 - 5000) = 7500ms -> target = 127_500
            MatchCandidate {
                subtitle_id: 2,
                original_start_ms: 120_000,
                transcribed_start_ms: 127_500,
                similarity: 0.95,
                score: 0.92,
            },
            // 3. Inlier at t=210_000.
            // alpha = (210 - 60) / 240 = 150 / 240 = 0.625
            // expected offset = 5000 + 0.625 * 10000 = 11250ms -> target = 221_250
            MatchCandidate {
                subtitle_id: 3,
                original_start_ms: 210_000,
                transcribed_start_ms: 221_250,
                similarity: 0.93,
                score: 0.90,
            },
            // 4. Candidate violating monotonicity: orig is after Anchor 1, but transcribed is before Anchor 1
            MatchCandidate {
                subtitle_id: 4,
                original_start_ms: 90_000,
                transcribed_start_ms: 60_000,
                similarity: 0.99,
                score: 0.99,
            },
        ];

        let (model, suggestions) = extract_consensus_anchors_guided(&candidates, 30_000, &existing);
        assert!(model.is_some());

        // Suggestions should only contain candidate 2 and candidate 3
        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].subtitle_id, 2);
        assert_eq!(suggestions[1].subtitle_id, 3);

        // Verify distance constraint: none of suggestions is within 15s of existing anchors
        for s in &suggestions {
            for a in &existing {
                assert!((s.original_start_ms - a.original_time_ms).abs() >= 15_000);
                assert!((s.corrected_time_ms - a.target_time_ms).abs() >= 15_000);
            }
        }

        // Verify strict monotonicity:
        // Anchor 1 (60s, 65s) < Suggestion 2 (120s, 127.5s) < Suggestion 3 (210s, 221.25s) < Anchor 2 (300s, 315s)
        assert!(suggestions[0].original_start_ms > existing[0].original_time_ms);
        assert!(suggestions[0].corrected_time_ms > existing[0].target_time_ms);
        assert!(suggestions[1].original_start_ms > suggestions[0].original_start_ms);
        assert!(suggestions[1].corrected_time_ms > suggestions[0].corrected_time_ms);
        assert!(suggestions[1].original_start_ms < existing[1].original_time_ms);
        assert!(suggestions[1].corrected_time_ms < existing[1].target_time_ms);
    }
}
