use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::interpolator::{AnchorPoint, TimeMapper};
use crate::sampler::{AdaptiveSampler, SamplerStrategy};
use srt_parser::{SrtParser, Subtitle, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub srt_path: String,

    pub video_path: Option<String>,

    pub time_mapper: TimeMapper,

    pub checked_indices: Vec<u32>,

    pub sampler_strategy: SamplerStrategy,
}

#[derive(Debug)]
pub struct SyncEngine {
    subtitles: HashMap<u32, Subtitle>,

    sorted_ids: Vec<u32>,

    time_mapper: TimeMapper,

    sampler: AdaptiveSampler,

    srt_path: String,

    video_path: Option<String>,

    current_index: usize,
}

impl SyncEngine {
    pub fn new<P: AsRef<Path>>(srt_path: P) -> Result<Self> {
        let path_str = srt_path.as_ref().to_string_lossy().to_string();
        let mut subtitles =
            SrtParser::parse_file(&srt_path).context("Impossibile caricare il file SRT")?;

        SrtParser::normalize_subtitles(&mut subtitles);

        let mut sorted_ids: Vec<u32> = subtitles.keys().copied().collect();
        sorted_ids.sort();

        let total = subtitles.len();
        let sampler = AdaptiveSampler::new(total, SamplerStrategy::BinarySearch);

        let mut engine = Self {
            subtitles,
            sorted_ids,
            time_mapper: TimeMapper::new(),
            sampler,
            srt_path: path_str,
            video_path: None,
            current_index: 0,
        };

        engine.update_sampler_times();

        Ok(engine)
    }

    pub fn from_state(state: SyncState) -> Result<Self> {
        let mut subtitles = SrtParser::parse_file(&state.srt_path)
            .context("Impossibile caricare il file SRT dallo stato salvato")?;

        SrtParser::normalize_subtitles(&mut subtitles);

        let mut sorted_ids: Vec<u32> = subtitles.keys().copied().collect();
        sorted_ids.sort();

        let total = subtitles.len();
        let mut sampler = AdaptiveSampler::new(total, state.sampler_strategy);

        for idx in &state.checked_indices {
            sampler.mark_checked(*idx);
        }

        let mut engine = Self {
            subtitles,
            sorted_ids,
            time_mapper: state.time_mapper,
            sampler,
            srt_path: state.srt_path,
            video_path: state.video_path,
            current_index: 0,
        };

        engine.update_sampler_times();

        Ok(engine)
    }

    fn update_sampler_times(&mut self) {
        let times: Vec<i64> = self
            .sorted_ids
            .iter()
            .filter_map(|id| self.subtitles.get(id))
            .map(|sub| sub.start.milliseconds as i64)
            .collect();

        self.sampler.set_subtitle_times(times);
    }

    pub fn set_video_path<P: AsRef<Path>>(&mut self, path: P) {
        self.video_path = Some(path.as_ref().to_string_lossy().to_string());
    }

    pub fn get_video_path(&self) -> Option<&str> {
        self.video_path.as_deref()
    }

    pub fn export_state(&self) -> SyncState {
        SyncState {
            srt_path: self.srt_path.clone(),
            video_path: self.video_path.clone(),
            time_mapper: self.time_mapper.clone(),
            checked_indices: self.sampler.get_checked_indices().to_vec(),
            sampler_strategy: self.sampler.strategy(),
        }
    }

    pub fn total_subtitles(&self) -> usize {
        self.subtitles.len()
    }

    pub fn get_subtitle(&self, id: u32) -> Option<&Subtitle> {
        self.subtitles.get(&id)
    }

    pub fn get_synced_subtitle(&self, id: u32) -> Option<Subtitle> {
        self.subtitles.get(&id).map(|sub| {
            let start_offset = self
                .time_mapper
                .calculate_offset(sub.start.milliseconds as i64);
            let end_offset = self
                .time_mapper
                .calculate_offset(sub.end.milliseconds as i64);

            Subtitle {
                id: sub.id,
                start: Timestamp {
                    milliseconds: (sub.start.milliseconds as i64 + start_offset).max(0) as u64,
                },
                end: Timestamp {
                    milliseconds: (sub.end.milliseconds as i64 + end_offset).max(0) as u64,
                },
                text: sub.text.clone(),
            }
        })
    }

    #[inline]
    pub fn get_synced_info_for(&self, sub: &Subtitle) -> (u64, u64, i64) {
        let start_offset = self
            .time_mapper
            .calculate_offset(sub.start.milliseconds as i64);
        let end_offset = self
            .time_mapper
            .calculate_offset(sub.end.milliseconds as i64);
        let start = (sub.start.milliseconds as i64 + start_offset).max(0) as u64;
        let end = (sub.end.milliseconds as i64 + end_offset).max(0) as u64;
        (start, end, start_offset)
    }

    #[inline]
    pub fn get_synced_times(&self, id: u32) -> Option<(u64, u64)> {
        self.subtitles.get(&id).map(|sub| {
            let (start, end, _) = self.get_synced_info_for(sub);
            (start, end)
        })
    }

    pub fn get_all_subtitles(&self) -> Vec<&Subtitle> {
        self.sorted_ids
            .iter()
            .filter_map(|id| self.subtitles.get(id))
            .collect()
    }

    pub fn get_all_synced_subtitles(&self) -> Vec<Subtitle> {
        self.sorted_ids
            .iter()
            .filter_map(|id| self.get_synced_subtitle(*id))
            .collect()
    }

    pub fn find_subtitle_at_time(&self, video_time_ms: u64) -> Option<u32> {
        if self.sorted_ids.is_empty() {
            return None;
        }

        // Binary search: find the first subtitle that starts strictly after video_time_ms in O(log N)
        let idx = self.sorted_ids.partition_point(|&id| {
            self.get_synced_times(id)
                .is_some_and(|(start, _)| start <= video_time_ms)
        });

        // The subtitle containing video_time_ms, if any, started at or before video_time_ms (index idx - 1)
        if idx > 0 {
            let id = self.sorted_ids[idx - 1];
            if let Some((start, end)) = self.get_synced_times(id)
                && video_time_ms >= start
                && video_time_ms <= end
            {
                return Some(id);
            }
            // Check immediately preceding entries in case of small overlapping timestamps
            for i in (0..idx - 1).rev().take(3) {
                let id = self.sorted_ids[i];
                if let Some((start, end)) = self.get_synced_times(id) {
                    if video_time_ms >= start && video_time_ms <= end {
                        return Some(id);
                    }
                    if end < video_time_ms.saturating_sub(10_000) {
                        break;
                    }
                }
            }
        }

        None
    }

    pub fn find_nearest_subtitle(&self, video_time_ms: u64) -> Option<u32> {
        if self.sorted_ids.is_empty() {
            return None;
        }

        // Binary search for closest start time in O(log N)
        let idx = self.sorted_ids.partition_point(|&id| {
            self.get_synced_times(id)
                .is_some_and(|(start, _)| start < video_time_ms)
        });

        let mut nearest_id = None;
        let mut min_distance = i64::MAX;

        let start_window = idx.saturating_sub(2);
        let end_window = (idx + 3).min(self.sorted_ids.len());

        for i in start_window..end_window {
            let id = self.sorted_ids[i];
            if let Some((start, end)) = self.get_synced_times(id) {
                let center = (start + end) / 2;
                let distance = (center as i64 - video_time_ms as i64).abs();

                if distance < min_distance {
                    min_distance = distance;
                    nearest_id = Some(id);
                }
            }
        }

        nearest_id
    }

    pub fn add_anchor(
        &mut self,
        subtitle_id: u32,
        corrected_time_ms: i64,
        is_manual: bool,
    ) -> Result<()> {
        let subtitle = self
            .subtitles
            .get(&subtitle_id)
            .context(format!("Sottotitolo {} non trovato", subtitle_id))?;

        let anchor = if is_manual {
            AnchorPoint::new_manual(
                subtitle_id,
                subtitle.start.milliseconds as i64,
                corrected_time_ms,
            )
        } else {
            AnchorPoint::new(
                subtitle_id,
                subtitle.start.milliseconds as i64,
                corrected_time_ms,
            )
        };

        self.time_mapper.add_anchor(anchor);
        self.sampler.mark_checked(subtitle_id);

        Ok(())
    }

    pub fn remove_anchor(&mut self, subtitle_id: u32) -> bool {
        self.time_mapper.remove_anchor(subtitle_id)
    }

    pub fn get_current_offset(&self, subtitle_id: u32) -> Option<i64> {
        self.subtitles.get(&subtitle_id).map(|sub| {
            self.time_mapper
                .calculate_offset(sub.start.milliseconds as i64)
        })
    }

    pub fn get_average_offset(&self) -> f64 {
        let anchors = self.time_mapper.get_anchors();
        if anchors.is_empty() {
            return 0.0;
        }

        let sum: i64 = anchors.iter().map(|a| a.offset()).sum();
        sum as f64 / anchors.len() as f64
    }

    pub fn suggest_next_index(&self) -> Option<u32> {
        self.sampler.suggest_next(&self.time_mapper)
    }

    pub fn set_sampling_strategy(&mut self, strategy: SamplerStrategy) {
        self.sampler.set_strategy(strategy);
    }

    pub fn anchor_count(&self) -> usize {
        self.time_mapper.anchor_count()
    }

    pub fn get_anchors(&self) -> &[AnchorPoint] {
        self.time_mapper.get_anchors()
    }

    pub fn checked_count(&self) -> usize {
        self.sampler.checked_count()
    }

    pub fn completion_percentage(&self) -> f64 {
        if self.subtitles.is_empty() {
            return 100.0;
        }
        (self.sampler.checked_count() as f64 / self.subtitles.len() as f64) * 100.0
    }

    pub fn save_synced_file<P: AsRef<Path>>(&self, output_path: P) -> Result<()> {
        let synced: HashMap<u32, Subtitle> = self
            .sorted_ids
            .iter()
            .filter_map(|id| self.get_synced_subtitle(*id).map(|s| (*id, s)))
            .collect();

        SrtParser::save_file(output_path, &synced)
            .context("Impossibile salvare il file sincronizzato")?;

        Ok(())
    }

    pub fn save_session<P: AsRef<Path>>(&self, session_path: P) -> Result<()> {
        let state = self.export_state();
        let json =
            serde_json::to_string_pretty(&state).context("Impossibile serializzare lo stato")?;

        std::fs::write(session_path, json).context("Impossibile salvare la sessione")?;

        Ok(())
    }

    pub fn load_session<P: AsRef<Path>>(session_path: P) -> Result<Self> {
        let json = std::fs::read_to_string(&session_path)
            .context("Impossibile leggere il file di sessione")?;

        let state: SyncState =
            serde_json::from_str(&json).context("Impossibile deserializzare lo stato")?;

        Self::from_state(state)
    }

    pub fn reset(&mut self) {
        self.time_mapper.clear();
        self.sampler.reset();
        self.current_index = 0;
    }

    pub fn set_current_index(&mut self, index: usize) {
        if index < self.sorted_ids.len() {
            self.current_index = index;
        }
    }

    pub fn get_current_index(&self) -> usize {
        self.current_index
    }

    pub fn get_current_subtitle_id(&self) -> Option<u32> {
        self.sorted_ids.get(self.current_index).copied()
    }

    pub fn next_subtitle(&mut self) -> Option<u32> {
        if self.current_index + 1 < self.sorted_ids.len() {
            self.current_index += 1;
            self.sorted_ids.get(self.current_index).copied()
        } else {
            None
        }
    }

    pub fn previous_subtitle(&mut self) -> Option<u32> {
        if self.current_index > 0 {
            self.current_index -= 1;
            self.sorted_ids.get(self.current_index).copied()
        } else {
            None
        }
    }

    pub fn go_to_subtitle(&mut self, id: u32) -> bool {
        if let Some(pos) = self.sorted_ids.iter().position(|&x| x == id) {
            self.current_index = pos;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_roundtrip_preserves_sampling_strategy() {
        let directory = tempfile::tempdir().unwrap();
        let srt = directory.path().join("source.srt");
        std::fs::write(&srt, "1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
        let mut engine = SyncEngine::new(&srt).unwrap();
        engine.set_sampling_strategy(SamplerStrategy::UniformTime);
        let restored = SyncEngine::from_state(engine.export_state()).unwrap();
        assert_eq!(
            restored.export_state().sampler_strategy,
            SamplerStrategy::UniformTime
        );
    }

    #[test]
    fn test_find_subtitle_at_time() {
        let mut subs = HashMap::new();
        subs.insert(
            1,
            Subtitle {
                id: 1,
                start: Timestamp { milliseconds: 1000 },
                end: Timestamp { milliseconds: 3000 },
                text: "First".to_string(),
            },
        );
        subs.insert(
            2,
            Subtitle {
                id: 2,
                start: Timestamp { milliseconds: 4000 },
                end: Timestamp { milliseconds: 6000 },
                text: "Second".to_string(),
            },
        );
        subs.insert(
            3,
            Subtitle {
                id: 3,
                start: Timestamp { milliseconds: 7000 },
                end: Timestamp { milliseconds: 9000 },
                text: "Third".to_string(),
            },
        );

        let engine = SyncEngine {
            subtitles: subs,
            sorted_ids: vec![1, 2, 3],
            time_mapper: TimeMapper::new(),
            sampler: AdaptiveSampler::new(3, SamplerStrategy::BinarySearch),
            srt_path: "test.srt".to_string(),
            video_path: None,
            current_index: 0,
        };

        assert_eq!(engine.find_subtitle_at_time(500), None);
        assert_eq!(engine.find_subtitle_at_time(1000), Some(1));
        assert_eq!(engine.find_subtitle_at_time(2000), Some(1));
        assert_eq!(engine.find_subtitle_at_time(3000), Some(1));
        assert_eq!(engine.find_subtitle_at_time(3500), None);
        assert_eq!(engine.find_subtitle_at_time(5000), Some(2));
        assert_eq!(engine.find_subtitle_at_time(8500), Some(3));
        assert_eq!(engine.find_subtitle_at_time(10000), None);

        assert_eq!(engine.find_nearest_subtitle(500), Some(1));
        assert_eq!(engine.find_nearest_subtitle(3400), Some(1));
        assert_eq!(engine.find_nearest_subtitle(3700), Some(2));
        assert_eq!(engine.find_nearest_subtitle(8000), Some(3));
        assert_eq!(engine.find_nearest_subtitle(20000), Some(3));
    }

    #[test]
    fn test_anchor_offset() {
        let mut mapper = TimeMapper::new();
        mapper.add_anchor(AnchorPoint::new(1, 1000, 2000));

        assert_eq!(mapper.calculate_offset(1000), 1000);
    }
}
