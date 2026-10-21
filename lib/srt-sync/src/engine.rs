use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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

    original_ids: HashSet<u32>,

    sorted_ids: Vec<u32>,

    time_mapper: TimeMapper,

    sampler: AdaptiveSampler,

    srt_path: String,

    video_path: Option<String>,

    current_index: usize,

    // Time order is independent of subtitle IDs and can change after anchors.
    time_index: Vec<(u64, u64, u32)>,
    prefix_max_end: Vec<u64>,
    center_index: Vec<(u64, u32)>,
}

impl SyncEngine {
    pub fn new<P: AsRef<Path>>(srt_path: P) -> Result<Self> {
        let path_str = srt_path.as_ref().to_string_lossy().to_string();
        let subtitles =
            SrtParser::parse_file(&srt_path).context("Impossibile caricare il file SRT")?;

        Ok(Self::from_subtitles(
            subtitles,
            SyncState {
                srt_path: path_str,
                video_path: None,
                time_mapper: TimeMapper::new(),
                checked_indices: Vec::new(),
                sampler_strategy: SamplerStrategy::BinarySearch,
            },
        ))
    }

    pub fn from_state(state: SyncState) -> Result<Self> {
        let subtitles = SrtParser::parse_file(&state.srt_path)
            .context("Impossibile caricare il file SRT dallo stato salvato")?;
        Ok(Self::from_subtitles(subtitles, state))
    }

    fn from_subtitles(mut subtitles: HashMap<u32, Subtitle>, state: SyncState) -> Self {
        let original_ids = subtitles.keys().copied().collect();
        SrtParser::normalize_subtitles(&mut subtitles);
        let mut sorted_ids: Vec<_> = subtitles.keys().copied().collect();
        sorted_ids.sort_unstable();
        let mut sampler = AdaptiveSampler::new(subtitles.len(), state.sampler_strategy);
        for idx in state.checked_indices {
            sampler.mark_checked(idx);
        }
        let mut engine = Self {
            subtitles,
            original_ids,
            sorted_ids,
            time_mapper: state.time_mapper,
            sampler,
            srt_path: state.srt_path,
            video_path: state.video_path,
            current_index: 0,
            time_index: Vec::new(),
            prefix_max_end: Vec::new(),
            center_index: Vec::new(),
        };
        engine.update_sampler_times();
        engine.rebuild_time_index();
        engine
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
            let (start, end, _) = self.get_synced_info_for(sub);
            Subtitle {
                id: sub.id,
                start: Timestamp {
                    milliseconds: start,
                },
                end: Timestamp { milliseconds: end },
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
        let start = (sub.start.milliseconds as i64)
            .saturating_add(start_offset)
            .max(0) as u64;
        let end = (sub.end.milliseconds as i64)
            .saturating_add(end_offset)
            .max(0) as u64;
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

    fn rebuild_time_index(&mut self) {
        let mut intervals: Vec<_> = self
            .sorted_ids
            .iter()
            .filter_map(|&id| {
                self.get_synced_times(id)
                    .map(|(start, end)| (start, end, id))
            })
            .collect();
        intervals.sort_unstable_by_key(|&(start, _, id)| (start, id));
        let mut max_end = 0;
        self.prefix_max_end = intervals
            .iter()
            .map(|&(_, end, _)| {
                max_end = max_end.max(end);
                max_end
            })
            .collect();
        self.center_index = intervals
            .iter()
            .map(|&(start, end, id)| (((start as u128 + end as u128) / 2) as u64, id))
            .collect();
        self.center_index.sort_unstable();
        self.time_index = intervals;
    }

    /// Return the containing subtitle with the latest start (highest ID on ties).
    /// Prefix maxima preserve long overlaps without an arbitrary search window.
    pub fn find_subtitle_at_time(&self, video_time_ms: u64) -> Option<u32> {
        let end = self
            .time_index
            .partition_point(|&(start, _, _)| start <= video_time_ms);
        for i in (0..end).rev() {
            if self.prefix_max_end[i] < video_time_ms {
                break;
            }
            let (_, end, id) = self.time_index[i];
            if end >= video_time_ms {
                return Some(id);
            }
        }
        None
    }

    /// Return the closest subtitle center, breaking equal distances by ID.
    pub fn find_nearest_subtitle(&self, video_time_ms: u64) -> Option<u32> {
        let next = self
            .center_index
            .partition_point(|&(center, _)| center < video_time_ms);
        let before = next
            .checked_sub(1)
            .and_then(|i| self.center_index.get(i))
            .map(|&(center, _)| {
                // Multiple subtitles may share the preceding center; select its first ID.
                self.center_index[self
                    .center_index
                    .partition_point(|&(value, _)| value < center)]
            });
        before
            .into_iter()
            .chain(self.center_index.get(next).copied())
            .min_by_key(|&(center, id)| (center.abs_diff(video_time_ms), id))
            .map(|(_, id)| id)
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
        self.rebuild_time_index();
        self.sampler.mark_checked(subtitle_id);

        Ok(())
    }

    pub fn remove_anchor(&mut self, subtitle_id: u32) -> bool {
        let removed = self.time_mapper.remove_anchor(subtitle_id);
        if removed {
            self.rebuild_time_index();
        }
        removed
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
            .filter(|id| self.original_ids.contains(id))
            .filter_map(|id| self.get_synced_subtitle(*id).map(|s| (*id, s)))
            .collect();

        // The SRT parser accepts u32 hours. Reject unrepresentable mapped times
        // before opening the output rather than publishing a file we cannot read.
        for subtitle in synced.values() {
            for timestamp in [subtitle.start, subtitle.end] {
                anyhow::ensure!(
                    timestamp.milliseconds / 3_600_000 <= u32::MAX as u64,
                    "Subtitle {} exceeds the supported SRT timestamp range",
                    subtitle.id
                );
            }
        }

        SrtParser::save_file(output_path, &synced)
            .context("Impossibile salvare il file sincronizzato")?;

        Ok(())
    }

    pub fn save_session<P: AsRef<Path>>(&self, session_path: P) -> Result<()> {
        let state = self.export_state();
        let json =
            serde_json::to_string_pretty(&state).context("Impossibile serializzare lo stato")?;

        srt_parser::persistence::atomic_write(session_path, json.as_bytes())
            .context("Impossibile salvare la sessione")?;

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
        self.rebuild_time_index();
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
        if let Ok(pos) = self.sorted_ids.binary_search(&id) {
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
    fn extreme_corrected_time_saturates_instead_of_overflowing() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.srt");
        std::fs::write(&source, "1\n00:00:01,000 --> 00:00:02,000\nHello\n").unwrap();
        let mut engine = SyncEngine::new(source).unwrap();
        engine.add_anchor(1, i64::MAX, true).unwrap();
        let synced = engine.get_synced_subtitle(1).unwrap();
        assert_eq!(synced.start.milliseconds, i64::MAX as u64);
        assert_eq!(synced.end.milliseconds, i64::MAX as u64);
        assert_eq!(
            engine.get_synced_times(1),
            Some((i64::MAX as u64, i64::MAX as u64))
        );
        let output = directory.path().join("existing.srt");
        std::fs::write(&output, "existing output").unwrap();
        assert!(engine.save_synced_file(&output).is_err());
        assert_eq!(std::fs::read_to_string(output).unwrap(), "existing output");
    }

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
    fn time_indexes_match_exhaustive_search_with_overlaps_and_ties() {
        let subtitles = (1..=80_u32)
            .map(|id| {
                let start = u64::from((id * 37) % 40) * 100;
                let end = start + u64::from((id * 19) % 25) * 100;
                (
                    id,
                    Subtitle {
                        id,
                        start: Timestamp {
                            milliseconds: start,
                        },
                        end: Timestamp { milliseconds: end },
                        text: id.to_string(),
                    },
                )
            })
            .collect();
        let mut engine = SyncEngine::from_subtitles(
            subtitles,
            SyncState {
                srt_path: String::new(),
                video_path: None,
                time_mapper: TimeMapper::new(),
                checked_indices: vec![],
                sampler_strategy: SamplerStrategy::BinarySearch,
            },
        );
        for shifted in [false, true] {
            if shifted {
                engine.add_anchor(1, 2_000, true).unwrap();
            }
            for time in (0..8_000).step_by(37) {
                let intervals: Vec<_> = engine
                    .sorted_ids
                    .iter()
                    .map(|&id| {
                        let (start, end) = engine.get_synced_times(id).unwrap();
                        (start, end, id)
                    })
                    .collect();
                let expected = intervals
                    .iter()
                    .filter(|&&(start, end, _)| start <= time && time <= end)
                    .max_by_key(|&&(start, _, id)| (start, id))
                    .map(|&(_, _, id)| id);
                assert_eq!(engine.find_subtitle_at_time(time), expected);
                let nearest = intervals
                    .iter()
                    .min_by_key(|&&(start, end, id)| {
                        let center = ((start as u128 + end as u128) / 2) as u64;
                        (center.abs_diff(time), id)
                    })
                    .map(|&(_, _, id)| id);
                assert_eq!(engine.find_nearest_subtitle(time), nearest);
            }
        }
    }

    #[test]
    fn temporal_search_handles_unsorted_ids_long_overlaps_and_anchor_changes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("unsorted.srt");
        std::fs::write(&path, "1\n00:00:00,000 --> 00:01:40,000\nLong\n\n2\n00:00:50,000 --> 00:00:51,000\nLater\n\n3\n00:00:01,000 --> 00:00:02,000\nEarly\n\n4\n00:00:03,000 --> 00:00:04,000\nFour\n\n5\n00:00:05,000 --> 00:00:06,000\nFive\n\n6\n00:00:07,000 --> 00:00:08,000\nSix\n").unwrap();
        let mut engine = SyncEngine::new(&path).unwrap();
        assert_eq!(engine.find_subtitle_at_time(1_500), Some(3));
        assert_eq!(engine.find_subtitle_at_time(40_000), Some(1));
        assert_eq!(engine.find_nearest_subtitle(49_900), Some(1));
        assert!(engine.go_to_subtitle(6));
        assert_eq!(engine.get_current_subtitle_id(), Some(6));
        assert!(!engine.go_to_subtitle(7));
        engine.add_anchor(1, 10_000, true).unwrap();
        assert_eq!(engine.find_subtitle_at_time(1_500), None);
        assert_eq!(engine.find_subtitle_at_time(11_500), Some(3));
        assert_eq!(engine.find_nearest_subtitle(59_900), Some(1));
        let restored = SyncEngine::from_state(engine.export_state()).unwrap();
        assert_eq!(restored.find_subtitle_at_time(11_500), Some(3));
        assert!(engine.remove_anchor(1));
        assert_eq!(engine.find_subtitle_at_time(1_500), Some(3));
        engine.add_anchor(1, 10_000, true).unwrap();
        engine.reset();
        assert_eq!(engine.find_subtitle_at_time(1_500), Some(3));
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

        let mut engine = SyncEngine {
            subtitles: subs.clone(),
            original_ids: subs.keys().copied().collect(),
            sorted_ids: vec![1, 2, 3],
            time_mapper: TimeMapper::new(),
            sampler: AdaptiveSampler::new(3, SamplerStrategy::BinarySearch),
            srt_path: "test.srt".to_string(),
            video_path: None,
            current_index: 0,
            time_index: Vec::new(),
            prefix_max_end: Vec::new(),
            center_index: Vec::new(),
        };

        engine.rebuild_time_index();

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

    #[test]
    fn normalized_placeholders_are_excluded_from_saved_file() {
        let directory = tempfile::tempdir().unwrap();
        let srt = directory.path().join("source.srt");
        // Gaps: 1 and 3 are present, 2 is missing and will be filled with [...] by normalization
        std::fs::write(
            &srt,
            "1\n00:00:01,000 --> 00:00:02,000\nHello\n\n3\n00:00:05,000 --> 00:00:06,000\nWorld\n",
        )
        .unwrap();

        let engine = SyncEngine::new(&srt).unwrap();
        // The normalized engine internally contains placeholder ID 2
        assert_eq!(engine.subtitles.len(), 3);
        assert!(engine.subtitles.contains_key(&2));
        assert_eq!(engine.subtitles.get(&2).unwrap().text, "[...]");

        let output = directory.path().join("synced.srt");
        engine.save_synced_file(&output).unwrap();

        let saved = SrtParser::parse_file(&output).unwrap();
        assert_eq!(saved.len(), 2);
        assert!(saved.contains_key(&1));
        assert!(!saved.contains_key(&2));
        assert!(saved.contains_key(&3));
    }
}
