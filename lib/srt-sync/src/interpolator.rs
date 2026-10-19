use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnchorPoint {
    pub subtitle_index: u32,

    pub original_time_ms: i64,

    pub corrected_time_ms: i64,

    #[serde(default)]
    pub is_manual: bool,
}

impl AnchorPoint {
    pub fn new(subtitle_index: u32, original_time_ms: i64, corrected_time_ms: i64) -> Self {
        Self {
            subtitle_index,
            original_time_ms,
            corrected_time_ms,
            is_manual: false,
        }
    }

    pub fn new_manual(subtitle_index: u32, original_time_ms: i64, corrected_time_ms: i64) -> Self {
        Self {
            subtitle_index,
            original_time_ms,
            corrected_time_ms,
            is_manual: true,
        }
    }

    /// Offset in milliseconds, saturated when the difference exceeds `i64`.
    pub fn offset(&self) -> i64 {
        self.corrected_time_ms.saturating_sub(self.original_time_ms)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimeMapper {
    anchors: Vec<AnchorPoint>,
}

impl TimeMapper {
    pub fn new() -> Self {
        Self {
            anchors: Vec::new(),
        }
    }

    pub fn add_anchor(&mut self, anchor: AnchorPoint) {
        if let Some(existing) = self
            .anchors
            .iter()
            .find(|a| a.subtitle_index == anchor.subtitle_index)
            && existing.is_manual
            && !anchor.is_manual
        {
            return;
        }

        self.anchors
            .retain(|a| a.subtitle_index != anchor.subtitle_index);

        self.anchors.push(anchor);

        self.anchors.sort_by_key(|a| a.original_time_ms);

        self.filter_inconsistent_auto_anchors();

        self.enforce_manual_priority();
    }

    fn enforce_manual_priority(&mut self) {
        if self.anchors.is_empty() {
            return;
        }

        const MANUAL_EXCLUSION_WINDOW_MS: i64 = 90_000;

        let mut manual_original_times: Vec<i64> = self
            .anchors
            .iter()
            .filter(|a| a.is_manual)
            .map(|a| a.original_time_ms)
            .collect();

        if manual_original_times.is_empty() {
            return;
        }

        manual_original_times.sort_unstable();

        self.anchors.retain(|anchor| {
            if anchor.is_manual {
                return true;
            }

            if manual_original_times
                .iter()
                .any(|m| anchor.original_time_ms.abs_diff(*m) <= MANUAL_EXCLUSION_WINDOW_MS as u64)
            {
                return false;
            }

            for pair in manual_original_times.windows(2) {
                let left = pair[0];
                let right = pair[1];
                if anchor.original_time_ms > left && anchor.original_time_ms < right {
                    return false;
                }
            }

            true
        });
    }

    fn filter_inconsistent_auto_anchors(&mut self) {
        if self.anchors.len() < 2 {
            return;
        }

        let mut valid_anchors = Vec::new();

        for anchor in self.anchors.drain(..) {
            if anchor.is_manual {
                valid_anchors.push(anchor);
            } else {
                if let Some(last_valid) = valid_anchors.last()
                    && anchor.corrected_time_ms < last_valid.corrected_time_ms
                {
                    continue;
                }
                valid_anchors.push(anchor);
            }
        }

        let mut final_anchors = Vec::new();
        let mut next_valid: Option<AnchorPoint> = None;

        for anchor in valid_anchors.into_iter().rev() {
            if anchor.is_manual {
                final_anchors.push(anchor);
                next_valid = Some(anchor);
            } else {
                if let Some(next_v) = next_valid
                    && anchor.corrected_time_ms > next_v.corrected_time_ms
                {
                    continue;
                }
                final_anchors.push(anchor);
                next_valid = Some(anchor);
            }
        }

        final_anchors.reverse();
        self.anchors = final_anchors;
    }

    pub fn remove_anchor(&mut self, subtitle_index: u32) -> bool {
        let len_before = self.anchors.len();
        self.anchors.retain(|a| a.subtitle_index != subtitle_index);
        self.anchors.len() < len_before
    }

    pub fn get_anchors(&self) -> &[AnchorPoint] {
        &self.anchors
    }

    pub fn anchor_count(&self) -> usize {
        self.anchors.len()
    }

    pub fn has_anchors(&self) -> bool {
        !self.anchors.is_empty()
    }

    pub fn calculate_offset(&self, original_time_ms: i64) -> i64 {
        match self.anchors.len() {
            0 => 0,
            1 => self.anchors[0].offset(),
            _ => self.interpolate_offset(original_time_ms),
        }
    }

    /// Apply the interpolated offset, saturating at the signed timestamp bounds.
    pub fn map_time(&self, original_time_ms: i64) -> i64 {
        original_time_ms.saturating_add(self.calculate_offset(original_time_ms))
    }

    fn interpolate_offset(&self, original_time_ms: i64) -> i64 {
        let first = &self.anchors[0];
        let last = &self.anchors[self.anchors.len() - 1];

        if original_time_ms <= first.original_time_ms {
            return first.offset();
        }

        if original_time_ms >= last.original_time_ms {
            return last.offset();
        }

        // Binary search for the enclosing anchor interval in O(log N)
        let idx = self
            .anchors
            .partition_point(|a| a.original_time_ms < original_time_ms);
        let anchor_before = &self.anchors[idx - 1];
        let anchor_after = &self.anchors[idx];

        let span = (i128::from(anchor_after.original_time_ms)
            - i128::from(anchor_before.original_time_ms)) as f64;
        if span <= 0.0 {
            return anchor_before.offset();
        }

        let elapsed =
            (i128::from(original_time_ms) - i128::from(anchor_before.original_time_ms)) as f64;
        let t = elapsed / span;
        let offset_before = anchor_before.offset() as f64;
        let offset_after = anchor_after.offset() as f64;

        (offset_before + t * (offset_after - offset_before)).round() as i64
    }

    pub fn estimate_error_at(&self, original_time_ms: i64) -> Option<f64> {
        if self.anchors.len() < 2 {
            return None;
        }

        let idx = self
            .anchors
            .partition_point(|a| a.original_time_ms < original_time_ms);
        let mut min_distance = self.anchors[idx.min(self.anchors.len() - 1)]
            .original_time_ms
            .abs_diff(original_time_ms);
        if idx > 0 {
            let prev_dist = self.anchors[idx - 1]
                .original_time_ms
                .abs_diff(original_time_ms);
            if prev_dist < min_distance {
                min_distance = prev_dist;
            }
        }

        Some(min_distance as f64 / 1000.0)
    }

    pub fn clear(&mut self) {
        self.anchors.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_anchor() {
        let mut mapper = TimeMapper::new();
        mapper.add_anchor(AnchorPoint::new(1, 10000, 12000));

        assert_eq!(mapper.calculate_offset(5000), 2000);
        assert_eq!(mapper.calculate_offset(10000), 2000);
        assert_eq!(mapper.calculate_offset(20000), 2000);
    }

    #[test]
    fn test_linear_interpolation() {
        let mut mapper = TimeMapper::new();
        mapper.add_anchor(AnchorPoint::new(1, 0, 0));
        mapper.add_anchor(AnchorPoint::new(10, 10000, 12000));

        assert_eq!(mapper.calculate_offset(5000), 1000);

        assert_eq!(mapper.calculate_offset(7500), 1500);
    }

    #[test]
    fn test_extrapolation() {
        let mut mapper = TimeMapper::new();
        mapper.add_anchor(AnchorPoint::new(1, 5000, 6000));
        mapper.add_anchor(AnchorPoint::new(10, 10000, 12000));

        assert_eq!(mapper.calculate_offset(0), 1000);

        assert_eq!(mapper.calculate_offset(20000), 2000);
    }

    #[test]
    fn test_remove_anchor() {
        let mut mapper = TimeMapper::new();
        mapper.add_anchor(AnchorPoint::new(1, 0, 0));
        mapper.add_anchor(AnchorPoint::new(2, 1000, 2000));

        assert_eq!(mapper.anchor_count(), 2);

        assert!(mapper.remove_anchor(1));
        assert_eq!(mapper.anchor_count(), 1);

        assert!(!mapper.remove_anchor(99));
    }

    #[test]
    fn extreme_anchor_offsets_and_mapping_saturate() {
        let forward = AnchorPoint::new(1, i64::MIN, i64::MAX);
        assert_eq!(forward.offset(), i64::MAX);
        let reverse = AnchorPoint::new(1, i64::MAX, i64::MIN);
        assert_eq!(reverse.offset(), i64::MIN);

        let mut mapper = TimeMapper::new();
        mapper.add_anchor(AnchorPoint::new(1, i64::MAX - 1, i64::MAX));
        assert_eq!(mapper.map_time(i64::MAX), i64::MAX);
        let mut negative = TimeMapper::new();
        negative.add_anchor(AnchorPoint::new(2, i64::MIN + 1, i64::MIN));
        assert_eq!(negative.map_time(i64::MIN), i64::MIN);
    }

    #[test]
    fn wide_anchor_interval_interpolates_without_integer_overflow() {
        let mut mapper = TimeMapper::new();
        mapper.add_anchor(AnchorPoint::new(1, i64::MIN, i64::MIN));
        mapper.add_anchor(AnchorPoint::new(2, i64::MAX, i64::MAX));
        assert_eq!(mapper.calculate_offset(0), 0);
        assert_eq!(mapper.map_time(1234), 1234);
    }

    #[test]
    fn estimate_error_at_extremes_and_manual_priority_do_not_overflow() {
        let mut mapper = TimeMapper::new();
        mapper.add_anchor(AnchorPoint::new(1, i64::MIN, i64::MIN));
        mapper.add_anchor(AnchorPoint::new(2, i64::MAX, i64::MAX));
        let distance = mapper.estimate_error_at(i64::MAX).unwrap();
        assert_eq!(distance, 0.0);
        assert_eq!(
            mapper.estimate_error_at(0).unwrap(),
            i64::MAX as f64 / 1000.0
        );

        let mut priority = TimeMapper::new();
        priority.add_anchor(AnchorPoint::new(10, i64::MIN, i64::MIN));
        priority.add_anchor(AnchorPoint::new_manual(1, i64::MAX, i64::MAX));
        assert!(priority.get_anchors().iter().any(|anchor| anchor.is_manual));
        assert!(
            priority
                .get_anchors()
                .iter()
                .any(|anchor| anchor.subtitle_index == 10)
        );

        let mut near = TimeMapper::new();
        near.add_anchor(AnchorPoint::new(11, i64::MIN + 90_000, i64::MIN + 90_000));
        near.add_anchor(AnchorPoint::new_manual(12, i64::MIN, i64::MIN));
        assert_eq!(near.anchor_count(), 1);
        assert!(near.get_anchors()[0].is_manual);
    }
}
