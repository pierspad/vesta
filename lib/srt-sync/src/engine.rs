//! Motore principale di sincronizzazione.
//!
//! SyncEngine gestisce l'intero flusso di sincronizzazione, coordinando
//! il TimeMapper per l'interpolazione e l'AdaptiveSampler per i suggerimenti.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use srt_parser::{SrtParser, Subtitle, Timestamp};
use crate::interpolator::{AnchorPoint, TimeMapper};
use crate::sampler::{AdaptiveSampler, SamplerStrategy};

/// Stato corrente della sincronizzazione (serializzabile per salvataggio sessione)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    /// Percorso file SRT originale
    pub srt_path: String,
    /// Percorso file video associato
    pub video_path: Option<String>,
    /// Mapper temporale con le ancore
    pub time_mapper: TimeMapper,
    /// Indici già verificati
    pub checked_indices: Vec<u32>,
    /// Strategia di campionamento
    pub sampler_strategy: SamplerStrategy,
}

/// Motore di sincronizzazione principale
#[derive(Debug)]
pub struct SyncEngine {
    /// Sottotitoli originali
    subtitles: HashMap<u32, Subtitle>,
    /// Sottotitoli ordinati per accesso sequenziale
    sorted_ids: Vec<u32>,
    /// Mapper temporale
    time_mapper: TimeMapper,
    /// Sampler per suggerimenti
    sampler: AdaptiveSampler,
    /// Percorso file SRT
    srt_path: String,
    /// Percorso file video
    video_path: Option<String>,
    /// Indice corrente visualizzato
    current_index: usize,
}

impl SyncEngine {
    /// Crea un nuovo engine caricando un file SRT
    pub fn new<P: AsRef<Path>>(srt_path: P) -> Result<Self> {
        let path_str = srt_path.as_ref().to_string_lossy().to_string();
        let mut subtitles = SrtParser::parse_file(&srt_path)
            .context("Impossibile caricare il file SRT")?;

        // Normalizza: riempi buchi nella numerazione con "[...]"
        SrtParser::normalize_subtitles(&mut subtitles);

        let mut sorted_ids: Vec<u32> = subtitles.keys().copied().collect();
        sorted_ids.sort();

        let total = subtitles.len();
        let sampler = AdaptiveSampler::new(total, SamplerStrategy::BinarySearch);

        // Estrai i tempi per il sampler
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

    /// Crea un engine da uno stato salvato
    pub fn from_state(state: SyncState) -> Result<Self> {
        let mut subtitles = SrtParser::parse_file(&state.srt_path)
            .context("Impossibile caricare il file SRT dallo stato salvato")?;

        // Normalizza: riempi buchi nella numerazione con "[...]"
        SrtParser::normalize_subtitles(&mut subtitles);

        let mut sorted_ids: Vec<u32> = subtitles.keys().copied().collect();
        sorted_ids.sort();

        let total = subtitles.len();
        let mut sampler = AdaptiveSampler::new(total, state.sampler_strategy);
        
        // Ripristina gli indici controllati
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

    /// Aggiorna i tempi nel sampler
    fn update_sampler_times(&mut self) {
        let times: Vec<i64> = self.sorted_ids
            .iter()
            .filter_map(|id| self.subtitles.get(id))
            .map(|sub| sub.start.milliseconds as i64)
            .collect();
        
        self.sampler.set_subtitle_times(times);
    }

    /// Imposta il percorso del video
    pub fn set_video_path<P: AsRef<Path>>(&mut self, path: P) {
        self.video_path = Some(path.as_ref().to_string_lossy().to_string());
    }

    /// Ottiene il percorso del video
    pub fn get_video_path(&self) -> Option<&str> {
        self.video_path.as_deref()
    }

    /// Esporta lo stato corrente per salvataggio
    pub fn export_state(&self) -> SyncState {
        SyncState {
            srt_path: self.srt_path.clone(),
            video_path: self.video_path.clone(),
            time_mapper: self.time_mapper.clone(),
            checked_indices: self.sampler.get_checked_indices().to_vec(),
            sampler_strategy: SamplerStrategy::BinarySearch, // TODO: rendere configurabile
        }
    }

    /// Numero totale di sottotitoli
    pub fn total_subtitles(&self) -> usize {
        self.subtitles.len()
    }

    /// Ottiene un sottotitolo per ID (1-based)
    pub fn get_subtitle(&self, id: u32) -> Option<&Subtitle> {
        self.subtitles.get(&id)
    }

    /// Ottiene un sottotitolo sincronizzato (con offset applicato)
    pub fn get_synced_subtitle(&self, id: u32) -> Option<Subtitle> {
        self.subtitles.get(&id).map(|sub| {
            let start_offset = self.time_mapper.calculate_offset(sub.start.milliseconds as i64);
            let end_offset = self.time_mapper.calculate_offset(sub.end.milliseconds as i64);

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

    /// Ottiene tutti i sottotitoli originali ordinati
    pub fn get_all_subtitles(&self) -> Vec<&Subtitle> {
        self.sorted_ids
            .iter()
            .filter_map(|id| self.subtitles.get(id))
            .collect()
    }

    /// Ottiene tutti i sottotitoli sincronizzati ordinati
    pub fn get_all_synced_subtitles(&self) -> Vec<Subtitle> {
        self.sorted_ids
            .iter()
            .filter_map(|id| self.get_synced_subtitle(*id))
            .collect()
    }

    /// Trova il sottotitolo attivo per un dato tempo video (in ms)
    pub fn find_subtitle_at_time(&self, video_time_ms: u64) -> Option<u32> {
        for id in &self.sorted_ids {
            if let Some(synced) = self.get_synced_subtitle(*id) {
                if video_time_ms >= synced.start.milliseconds
                    && video_time_ms <= synced.end.milliseconds
                {
                    return Some(*id);
                }
            }
        }
        None
    }

    /// Trova il sottotitolo più vicino a un dato tempo video
    pub fn find_nearest_subtitle(&self, video_time_ms: u64) -> Option<u32> {
        let mut nearest_id = None;
        let mut min_distance = i64::MAX;

        for id in &self.sorted_ids {
            if let Some(synced) = self.get_synced_subtitle(*id) {
                // Distanza dal centro del sottotitolo
                let center = (synced.start.milliseconds + synced.end.milliseconds) / 2;
                let distance = (center as i64 - video_time_ms as i64).abs();

                if distance < min_distance {
                    min_distance = distance;
                    nearest_id = Some(*id);
                }
            }
        }

        nearest_id
    }

    /// Aggiunge un punto di ancoraggio
    /// 
    /// # Argomenti
    /// * `subtitle_id` - ID del sottotitolo (1-based)
    /// * `corrected_time_ms` - Tempo corretto dal video in millisecondi
    /// * `is_manual` - Specifica se l'ancora è forzata dall'utente (priorità massima)
    pub fn add_anchor(&mut self, subtitle_id: u32, corrected_time_ms: i64, is_manual: bool) -> Result<()> {
        let subtitle = self.subtitles.get(&subtitle_id)
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

    /// Rimuove un punto di ancoraggio
    pub fn remove_anchor(&mut self, subtitle_id: u32) -> bool {
        self.time_mapper.remove_anchor(subtitle_id)
    }

    /// Ottiene l'offset corrente per un sottotitolo
    pub fn get_current_offset(&self, subtitle_id: u32) -> Option<i64> {
        self.subtitles.get(&subtitle_id).map(|sub| {
            self.time_mapper.calculate_offset(sub.start.milliseconds as i64)
        })
    }

    /// Ottiene l'offset medio globale
    pub fn get_average_offset(&self) -> f64 {
        let anchors = self.time_mapper.get_anchors();
        if anchors.is_empty() {
            return 0.0;
        }

        let sum: i64 = anchors.iter().map(|a| a.offset()).sum();
        sum as f64 / anchors.len() as f64
    }

    /// Suggerisce il prossimo sottotitolo da controllare
    pub fn suggest_next_index(&self) -> Option<u32> {
        self.sampler.suggest_next(&self.time_mapper)
    }

    /// Cambia la strategia di campionamento
    pub fn set_sampling_strategy(&mut self, strategy: SamplerStrategy) {
        self.sampler.set_strategy(strategy);
    }

    /// Numero di ancore definite
    pub fn anchor_count(&self) -> usize {
        self.time_mapper.anchor_count()
    }

    /// Ottiene le ancore correnti
    pub fn get_anchors(&self) -> &[AnchorPoint] {
        self.time_mapper.get_anchors()
    }

    /// Numero di sottotitoli verificati
    pub fn checked_count(&self) -> usize {
        self.sampler.checked_count()
    }

    /// Percentuale di completamento
    pub fn completion_percentage(&self) -> f64 {
        if self.subtitles.is_empty() {
            return 100.0;
        }
        (self.sampler.checked_count() as f64 / self.subtitles.len() as f64) * 100.0
    }

    /// Salva i sottotitoli sincronizzati su file
    pub fn save_synced_file<P: AsRef<Path>>(&self, output_path: P) -> Result<()> {
        let synced: HashMap<u32, Subtitle> = self.sorted_ids
            .iter()
            .filter_map(|id| self.get_synced_subtitle(*id).map(|s| (*id, s)))
            .collect();

        SrtParser::save_file(output_path, &synced)
            .context("Impossibile salvare il file sincronizzato")?;

        Ok(())
    }

    /// Salva lo stato della sessione per ripresa futura
    pub fn save_session<P: AsRef<Path>>(&self, session_path: P) -> Result<()> {
        let state = self.export_state();
        let json = serde_json::to_string_pretty(&state)
            .context("Impossibile serializzare lo stato")?;
        
        std::fs::write(session_path, json)
            .context("Impossibile salvare la sessione")?;

        Ok(())
    }

    /// Carica una sessione salvata
    pub fn load_session<P: AsRef<Path>>(session_path: P) -> Result<Self> {
        let json = std::fs::read_to_string(&session_path)
            .context("Impossibile leggere il file di sessione")?;
        
        let state: SyncState = serde_json::from_str(&json)
            .context("Impossibile deserializzare lo stato")?;

        Self::from_state(state)
    }

    /// Resetta completamente la sincronizzazione
    pub fn reset(&mut self) {
        self.time_mapper.clear();
        self.sampler.reset();
        self.current_index = 0;
    }

    /// Imposta l'indice corrente
    pub fn set_current_index(&mut self, index: usize) {
        if index < self.sorted_ids.len() {
            self.current_index = index;
        }
    }

    /// Ottiene l'indice corrente
    pub fn get_current_index(&self) -> usize {
        self.current_index
    }

    /// Ottiene l'ID del sottotitolo corrente
    pub fn get_current_subtitle_id(&self) -> Option<u32> {
        self.sorted_ids.get(self.current_index).copied()
    }

    /// Va al prossimo sottotitolo
    pub fn next_subtitle(&mut self) -> Option<u32> {
        if self.current_index + 1 < self.sorted_ids.len() {
            self.current_index += 1;
            self.sorted_ids.get(self.current_index).copied()
        } else {
            None
        }
    }

    /// Va al sottotitolo precedente
    pub fn previous_subtitle(&mut self) -> Option<u32> {
        if self.current_index > 0 {
            self.current_index -= 1;
            self.sorted_ids.get(self.current_index).copied()
        } else {
            None
        }
    }

    /// Vai a un sottotitolo specifico per ID
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

    fn create_test_subtitles() -> HashMap<u32, Subtitle> {
        let mut subs = HashMap::new();
        
        subs.insert(1, Subtitle {
            id: 1,
            start: Timestamp { milliseconds: 1000 },
            end: Timestamp { milliseconds: 3000 },
            text: "First subtitle".to_string(),
        });
        
        subs.insert(2, Subtitle {
            id: 2,
            start: Timestamp { milliseconds: 5000 },
            end: Timestamp { milliseconds: 7000 },
            text: "Second subtitle".to_string(),
        });
        
        subs.insert(3, Subtitle {
            id: 3,
            start: Timestamp { milliseconds: 10000 },
            end: Timestamp { milliseconds: 12000 },
            text: "Third subtitle".to_string(),
        });

        subs
    }

    #[test]
    fn test_find_subtitle_at_time() {
        // Questo test richiederebbe un file SRT reale
        // Per ora è un placeholder
    }

    #[test]
    fn test_anchor_offset() {
        // Test di base per la logica degli offset
        let mut mapper = TimeMapper::new();
        mapper.add_anchor(AnchorPoint::new(1, 1000, 2000)); // +1 secondo
        
        assert_eq!(mapper.calculate_offset(1000), 1000);
    }
}
