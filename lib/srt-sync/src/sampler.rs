use crate::interpolator::TimeMapper;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SamplerStrategy {
    #[default]
    BinarySearch,

    MaxUncertainty,

    UniformTime,

    Sequential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveSampler {
    strategy: SamplerStrategy,

    total_subtitles: usize,

    checked_indices: Vec<u32>,

    subtitle_times_ms: Vec<i64>,
}

impl AdaptiveSampler {
    pub fn new(total_subtitles: usize, strategy: SamplerStrategy) -> Self {
        Self {
            strategy,
            total_subtitles,
            checked_indices: Vec::new(),
            subtitle_times_ms: Vec::new(),
        }
    }

    pub fn set_subtitle_times(&mut self, times_ms: Vec<i64>) {
        self.subtitle_times_ms = times_ms;
    }

    pub fn set_strategy(&mut self, strategy: SamplerStrategy) {
        self.strategy = strategy;
    }

    pub fn mark_checked(&mut self, index: u32) {
        if !self.checked_indices.contains(&index) {
            self.checked_indices.push(index);
            self.checked_indices.sort();
        }
    }

    pub fn is_checked(&self, index: u32) -> bool {
        self.checked_indices.contains(&index)
    }

    pub fn checked_count(&self) -> usize {
        self.checked_indices.len()
    }

    pub fn suggest_next(&self, time_mapper: &TimeMapper) -> Option<u32> {
        if self.checked_indices.len() >= self.total_subtitles {
            return None;
        }

        match self.strategy {
            SamplerStrategy::BinarySearch => self.suggest_binary_search(),
            SamplerStrategy::MaxUncertainty => self.suggest_max_uncertainty(time_mapper),
            SamplerStrategy::UniformTime => self.suggest_uniform_time(),
            SamplerStrategy::Sequential => self.suggest_sequential(),
        }
    }

    fn suggest_binary_search(&self) -> Option<u32> {
        if self.checked_indices.is_empty() {
            return Some((self.total_subtitles / 2) as u32 + 1);
        }

        let mut largest_gap = 0;
        let mut gap_start = 0u32;
        let mut gap_end = 0u32;

        let first_checked = self.checked_indices[0];
        if first_checked > 1 {
            let gap = first_checked - 1;
            if gap > largest_gap {
                largest_gap = gap;
                gap_start = 1;
                gap_end = first_checked;
            }
        }

        for i in 0..self.checked_indices.len() - 1 {
            let current = self.checked_indices[i];
            let next = self.checked_indices[i + 1];
            let gap = next - current - 1;

            if gap > largest_gap {
                largest_gap = gap;
                gap_start = current + 1;
                gap_end = next;
            }
        }

        let last_checked = *self.checked_indices.last().unwrap();
        let total = self.total_subtitles as u32;
        if last_checked < total {
            let gap = total - last_checked;
            if gap > largest_gap {
                largest_gap = gap;
                gap_start = last_checked + 1;
                gap_end = total + 1;
            }
        }

        if largest_gap > 0 {
            Some((gap_start + gap_end) / 2)
        } else {
            None
        }
    }

    fn suggest_max_uncertainty(&self, time_mapper: &TimeMapper) -> Option<u32> {
        if self.subtitle_times_ms.is_empty() {
            return self.suggest_binary_search();
        }

        let mut max_error = -1.0_f64;
        let mut best_index = None;

        for i in 1..=self.total_subtitles {
            let idx = i as u32;
            if self.checked_indices.contains(&idx) {
                continue;
            }

            if let Some(time_ms) = self.subtitle_times_ms.get(i - 1) {
                if let Some(error) = time_mapper.estimate_error_at(*time_ms) {
                    if error > max_error {
                        max_error = error;
                        best_index = Some(idx);
                    }
                } else {
                    if max_error < 0.0 {
                        best_index = Some(idx);
                    }
                }
            }
        }

        best_index.or_else(|| self.suggest_binary_search())
    }

    fn suggest_uniform_time(&self) -> Option<u32> {
        if self.subtitle_times_ms.is_empty() {
            return self.suggest_binary_search();
        }

        let first_time = *self.subtitle_times_ms.first()?;
        let last_time = *self.subtitle_times_ms.last()?;
        let duration = last_time - first_time;

        if duration <= 0 {
            return self.suggest_sequential();
        }

        let checked_times: Vec<i64> = self
            .checked_indices
            .iter()
            .filter_map(|&idx| self.subtitle_times_ms.get(idx as usize - 1).copied())
            .collect();

        let mut best_index = None;
        let mut max_min_distance = -1_i64;

        for i in 1..=self.total_subtitles {
            let idx = i as u32;
            if self.checked_indices.contains(&idx) {
                continue;
            }

            if let Some(&time_ms) = self.subtitle_times_ms.get(i - 1) {
                let min_distance = checked_times
                    .iter()
                    .map(|&t| (t - time_ms).abs())
                    .min()
                    .unwrap_or(i64::MAX);

                if min_distance > max_min_distance {
                    max_min_distance = min_distance;
                    best_index = Some(idx);
                }
            }
        }

        best_index
    }

    fn suggest_sequential(&self) -> Option<u32> {
        for i in 1..=self.total_subtitles {
            let idx = i as u32;
            if !self.checked_indices.contains(&idx) {
                return Some(idx);
            }
        }
        None
    }

    pub fn reset(&mut self) {
        self.checked_indices.clear();
    }

    pub fn get_checked_indices(&self) -> &[u32] {
        &self.checked_indices
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_search_initial() {
        let sampler = AdaptiveSampler::new(100, SamplerStrategy::BinarySearch);

        assert_eq!(sampler.suggest_next(&TimeMapper::new()), Some(51));
    }

    #[test]
    fn test_binary_search_splits() {
        let mut sampler = AdaptiveSampler::new(100, SamplerStrategy::BinarySearch);

        sampler.mark_checked(50);
        let next = sampler.suggest_next(&TimeMapper::new());

        assert!(next.is_some());

        let suggested = next.unwrap();
        assert!(suggested != 50);
    }

    #[test]
    fn test_sequential() {
        let mut sampler = AdaptiveSampler::new(10, SamplerStrategy::Sequential);

        assert_eq!(sampler.suggest_next(&TimeMapper::new()), Some(1));
        sampler.mark_checked(1);
        assert_eq!(sampler.suggest_next(&TimeMapper::new()), Some(2));
        sampler.mark_checked(2);
        assert_eq!(sampler.suggest_next(&TimeMapper::new()), Some(3));
    }

    #[test]
    fn test_all_checked() {
        let mut sampler = AdaptiveSampler::new(3, SamplerStrategy::Sequential);

        sampler.mark_checked(1);
        sampler.mark_checked(2);
        sampler.mark_checked(3);

        assert_eq!(sampler.suggest_next(&TimeMapper::new()), None);
    }
}
