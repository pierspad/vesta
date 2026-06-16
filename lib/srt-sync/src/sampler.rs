//! Modulo per il campionamento adattivo.
//!
//! Implementa strategie per suggerire quale sottotitolo controllare successivamente
//! per massimizzare l'efficienza della sincronizzazione.

use serde::{Deserialize, Serialize};
use crate::interpolator::TimeMapper;

/// Strategia di campionamento per suggerire il prossimo sottotitolo da controllare
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SamplerStrategy {
    /// Ricerca binaria: divide sempre l'intervallo a metà
    #[default]
    BinarySearch,
    /// Priorità ai segmenti con maggiore incertezza stimata
    MaxUncertainty,
    /// Campionamento uniforme in base alla durata temporale
    UniformTime,
    /// Campionamento sequenziale (prossimo sottotitolo)
    Sequential,
}

/// Sampler adattivo che suggerisce il prossimo sottotitolo da sincronizzare
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveSampler {
    /// Strategia corrente
    strategy: SamplerStrategy,
    /// Numero totale di sottotitoli
    total_subtitles: usize,
    /// Indici già controllati (set di indici confermati)
    checked_indices: Vec<u32>,
    /// Tempi originali dei sottotitoli in ms (indice 0 = sottotitolo 1)
    subtitle_times_ms: Vec<i64>,
}

impl AdaptiveSampler {
    /// Crea un nuovo sampler
    pub fn new(total_subtitles: usize, strategy: SamplerStrategy) -> Self {
        Self {
            strategy,
            total_subtitles,
            checked_indices: Vec::new(),
            subtitle_times_ms: Vec::new(),
        }
    }

    /// Imposta i tempi dei sottotitoli per strategie basate sul tempo
    pub fn set_subtitle_times(&mut self, times_ms: Vec<i64>) {
        self.subtitle_times_ms = times_ms;
    }

    /// Cambia la strategia di campionamento
    pub fn set_strategy(&mut self, strategy: SamplerStrategy) {
        self.strategy = strategy;
    }

    /// Marca un indice come controllato
    pub fn mark_checked(&mut self, index: u32) {
        if !self.checked_indices.contains(&index) {
            self.checked_indices.push(index);
            self.checked_indices.sort();
        }
    }

    /// Verifica se un indice è stato controllato
    pub fn is_checked(&self, index: u32) -> bool {
        self.checked_indices.contains(&index)
    }

    /// Numero di indici controllati
    pub fn checked_count(&self) -> usize {
        self.checked_indices.len()
    }

    /// Suggerisce il prossimo indice da controllare
    /// Ritorna None se tutti sono stati controllati o non c'è un suggerimento valido
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

    /// Strategia ricerca binaria: trova il punto medio del segmento più grande non controllato
    fn suggest_binary_search(&self) -> Option<u32> {
        if self.checked_indices.is_empty() {
            // Inizia dal centro
            return Some((self.total_subtitles / 2) as u32 + 1);
        }

        // Trova il segmento più grande tra indici controllati
        let mut largest_gap = 0;
        let mut gap_start = 0u32;
        let mut gap_end = 0u32;

        // Gap prima del primo controllato
        let first_checked = self.checked_indices[0];
        if first_checked > 1 {
            let gap = first_checked - 1;
            if gap > largest_gap {
                largest_gap = gap;
                gap_start = 1;
                gap_end = first_checked;
            }
        }

        // Gap tra elementi controllati
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

        // Gap dopo l'ultimo controllato
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
            // Ritorna il punto medio del gap
            Some((gap_start + gap_end) / 2)
        } else {
            None
        }
    }

    /// Strategia massima incertezza: trova il punto con maggiore errore stimato
    fn suggest_max_uncertainty(&self, time_mapper: &TimeMapper) -> Option<u32> {
        if self.subtitle_times_ms.is_empty() {
            // Fallback a binary search se non abbiamo i tempi
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
                    // Se non possiamo stimare l'errore, questo punto ha alta priorità
                    if max_error < 0.0 {
                        best_index = Some(idx);
                    }
                }
            }
        }

        best_index.or_else(|| self.suggest_binary_search())
    }

    /// Strategia uniforme nel tempo: campiona uniformemente sulla timeline
    fn suggest_uniform_time(&self) -> Option<u32> {
        if self.subtitle_times_ms.is_empty() {
            return self.suggest_binary_search();
        }

        // Calcola intervalli temporali uniformi
        let first_time = *self.subtitle_times_ms.first()?;
        let last_time = *self.subtitle_times_ms.last()?;
        let duration = last_time - first_time;

        if duration <= 0 {
            return self.suggest_sequential();
        }

        // Trova il tempo target non ancora controllato che è più lontano da un controllato
        let checked_times: Vec<i64> = self.checked_indices
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

    /// Strategia sequenziale: prossimo sottotitolo non controllato
    fn suggest_sequential(&self) -> Option<u32> {
        for i in 1..=self.total_subtitles {
            let idx = i as u32;
            if !self.checked_indices.contains(&idx) {
                return Some(idx);
            }
        }
        None
    }

    /// Resetta lo stato del sampler
    pub fn reset(&mut self) {
        self.checked_indices.clear();
    }

    /// Ottiene gli indici controllati
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
        // Dovrebbe suggerire il centro
        assert_eq!(sampler.suggest_next(&TimeMapper::new()), Some(51));
    }

    #[test]
    fn test_binary_search_splits() {
        let mut sampler = AdaptiveSampler::new(100, SamplerStrategy::BinarySearch);
        
        sampler.mark_checked(50);
        let next = sampler.suggest_next(&TimeMapper::new());
        // Dovrebbe suggerire il centro di uno dei due gap
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
