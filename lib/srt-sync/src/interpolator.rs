//! Modulo per l'interpolazione temporale tra ancore.
//!
//! Implementa un TimeMapper che usa interpolazione lineare per calcolare
//! l'offset corretto per qualsiasi timestamp basandosi sui punti di ancoraggio definiti.

use serde::{Deserialize, Serialize};

/// Un punto di ancoraggio che mappa un tempo originale a un tempo corretto
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnchorPoint {
    /// Indice del sottotitolo (1-based come negli SRT)
    pub subtitle_index: u32,
    /// Tempo originale del sottotitolo in millisecondi
    pub original_time_ms: i64,
    /// Tempo corretto (sincronizzato con il video) in millisecondi
    pub corrected_time_ms: i64,
    /// Indica se l'ancora è stata aggiunta manualmente (true) o tramite auto-sync (false)
    #[serde(default)]
    pub is_manual: bool,
}

impl AnchorPoint {
    /// Crea un nuovo punto di ancoraggio
    pub fn new(subtitle_index: u32, original_time_ms: i64, corrected_time_ms: i64) -> Self {
        Self {
            subtitle_index,
            original_time_ms,
            corrected_time_ms,
            is_manual: false, // Per retrocompatibilità, le ancore esistenti non marcate si presumono automatiche. Ma aspetta...
        }
    }

    /// Crea un punto di ancoraggio manuale
    pub fn new_manual(subtitle_index: u32, original_time_ms: i64, corrected_time_ms: i64) -> Self {
        Self {
            subtitle_index,
            original_time_ms,
            corrected_time_ms,
            is_manual: true,
        }
    }
    /// Calcola l'offset (differenza tra tempo corretto e originale)
    pub fn offset(&self) -> i64 {
        self.corrected_time_ms - self.original_time_ms
    }
}

/// Mapper temporale che usa interpolazione lineare tra ancore
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TimeMapper {
    /// Lista ordinata di punti di ancoraggio
    anchors: Vec<AnchorPoint>,
}

impl TimeMapper {
    /// Crea un nuovo TimeMapper vuoto
    pub fn new() -> Self {
        Self {
            anchors: Vec::new(),
        }
    }

    /// Aggiunge un punto di ancoraggio e mantiene l'ordinamento
    pub fn add_anchor(&mut self, anchor: AnchorPoint) {
        // Se esiste già un'ancora manuale, e la nuova non è manuale, ignoriamo la nuova.
        // Le ancore manuali sono la "verità assoluta" e non possono essere sovrascritte da processi automatici come il whisper sync.
        if let Some(existing) = self.anchors.iter().find(|a| a.subtitle_index == anchor.subtitle_index) {
            if existing.is_manual && !anchor.is_manual {
                return;
            }
        }

        // Rimuovi eventuali ancore esistenti per lo stesso indice
        self.anchors.retain(|a| a.subtitle_index != anchor.subtitle_index);
        
        // Aggiungi la nuova ancora
        self.anchors.push(anchor);
        
        // Ordina per tempo originale in modo che l'interpolazione funzioni correttamente
        self.anchors.sort_by_key(|a| a.original_time_ms);
        
        // Filtriamo le ancore automatiche ("whisper sync") che causano incongruenze temporali
        // (es. "viaggio nel tempo" all'indietro tra le ancore manuali o incongruenza di ordinamento temporale stretto).
        self.filter_inconsistent_auto_anchors();

        // Le ancore manuali sono l'unica fonte "ground truth":
        // le ancore automatiche devono restare solo come suggerimento e non bloccare
        // la propagazione dell'interpolazione attorno ai punti fissati dall'utente.
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

            // 1) Rimuovi ancore automatiche troppo vicine a una manuale.
            if manual_original_times
                .iter()
                .any(|m| (anchor.original_time_ms - *m).abs() <= MANUAL_EXCLUSION_WINDOW_MS)
            {
                return false;
            }

            // 2) Se l'ancora automatica cade tra due manuali, la rimuoviamo:
            // tra due punti certi vogliamo solo interpolazione guidata dall'utente.
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

    /// Rimuove le ancore automatiche che rompono l'ordine logico del tempo (tempo corretto non monotono crescente).
    /// Le ancore manuali vengono sempre preservate (verità assoluta). Se un'ancora automatica inverte l'ordine del tempo
    /// rispetto alle restanti ancore (specialmente quelle manuali), viene rimossa.
    fn filter_inconsistent_auto_anchors(&mut self) {
        if self.anchors.len() < 2 {
            return;
        }
        
        let mut valid_anchors = Vec::new();
        
        for anchor in self.anchors.drain(..) {
            if anchor.is_manual {
                // Le ancore manuali sono sacre
                valid_anchors.push(anchor);
            } else {
                // Controlla se questa ancora automatica ("whisper sync") rompe la monotonicità del tempo corretto 
                // rispetto all'ultima ancora considerata valida
                if let Some(last_valid) = valid_anchors.last() {
                    // Poiché le ancore sono ordinate per original_time_ms crescente,
                    // ci aspettiamo che anche i corrected_time_ms siano (grosso modo) crescenti
                    // affinchè la sincronizzazione sia percepita ottimalmente.
                    if anchor.corrected_time_ms < last_valid.corrected_time_ms {
                        // Se l'ultima era manuale, sicuramente scartiamo questa automatica.
                        // Se l'ultima era automatica, scartiamo la nuova per mantenere l'ordine.
                        continue;
                    }
                }
                valid_anchors.push(anchor);
            }
        }
        
        // A questo punto potrebbero esserci sovrapposizioni o cali di tempo all'indietro da un'ancora automatica 
        // verso una successiva ancora manuale. Ripetiamo all'indietro per assicurarci
        // che le automatiche non eccedano le manuali future.
        let mut final_anchors = Vec::new();
        let mut next_valid: Option<AnchorPoint> = None;
        
        for anchor in valid_anchors.into_iter().rev() {
            if anchor.is_manual {
                final_anchors.push(anchor);
                next_valid = Some(anchor);
            } else {
                if let Some(next_v) = next_valid {
                    // anchor.corrected_time_ms deve essere minore (o uguale) di next_v.corrected_time_ms
                    if anchor.corrected_time_ms > next_v.corrected_time_ms {
                        // L'ancora automatica sta spingendo il tempo oltre una futura ancora ritenuta valida, la rimuoviamo
                        continue;
                    }
                }
                final_anchors.push(anchor);
                next_valid = Some(anchor);
            }
        }
        
        // Final anchors è al contrario rispetto ad original_time_ms
        final_anchors.reverse();
        self.anchors = final_anchors;
    }

    /// Rimuove un punto di ancoraggio per indice sottotitolo
    pub fn remove_anchor(&mut self, subtitle_index: u32) -> bool {
        let len_before = self.anchors.len();
        self.anchors.retain(|a| a.subtitle_index != subtitle_index);
        self.anchors.len() < len_before
    }

    /// Ottiene tutti i punti di ancoraggio
    pub fn get_anchors(&self) -> &[AnchorPoint] {
        &self.anchors
    }

    /// Numero di ancore definite
    pub fn anchor_count(&self) -> usize {
        self.anchors.len()
    }

    /// Controlla se ci sono ancore
    pub fn has_anchors(&self) -> bool {
        !self.anchors.is_empty()
    }

    /// Calcola l'offset per un dato tempo originale usando interpolazione lineare
    /// 
    /// Strategia:
    /// - Se non ci sono ancore: offset = 0
    /// - Se c'è una sola ancora: usa offset costante
    /// - Se il tempo è prima della prima ancora: usa offset della prima ancora
    /// - Se il tempo è dopo l'ultima ancora: usa offset dell'ultima ancora
    /// - Altrimenti: interpola linearmente tra le due ancore adiacenti
    pub fn calculate_offset(&self, original_time_ms: i64) -> i64 {
        match self.anchors.len() {
            0 => 0,
            1 => self.anchors[0].offset(),
            _ => self.interpolate_offset(original_time_ms),
        }
    }

    /// Mappa un tempo originale al tempo corretto
    pub fn map_time(&self, original_time_ms: i64) -> i64 {
        original_time_ms + self.calculate_offset(original_time_ms)
    }

    /// Interpolazione lineare tra ancore
    fn interpolate_offset(&self, original_time_ms: i64) -> i64 {
        // Trova le ancore adiacenti
        let first = &self.anchors[0];
        let last = &self.anchors[self.anchors.len() - 1];

        // Prima della prima ancora
        if original_time_ms <= first.original_time_ms {
            return first.offset();
        }

        // Dopo l'ultima ancora
        if original_time_ms >= last.original_time_ms {
            return last.offset();
        }

        // Trova le due ancore tra cui interpolare
        for i in 0..self.anchors.len() - 1 {
            let anchor_before = &self.anchors[i];
            let anchor_after = &self.anchors[i + 1];

            if original_time_ms >= anchor_before.original_time_ms
                && original_time_ms <= anchor_after.original_time_ms
            {
                // Interpolazione lineare
                let t = (original_time_ms - anchor_before.original_time_ms) as f64
                    / (anchor_after.original_time_ms - anchor_before.original_time_ms) as f64;

                let offset_before = anchor_before.offset() as f64;
                let offset_after = anchor_after.offset() as f64;

                return (offset_before + t * (offset_after - offset_before)).round() as i64;
            }
        }

        // Fallback (non dovrebbe mai arrivarci)
        0
    }

    /// Calcola l'errore di sincronizzazione stimato per un dato punto
    /// Ritorna None se non ci sono abbastanza ancore per stimare l'errore
    pub fn estimate_error_at(&self, original_time_ms: i64) -> Option<f64> {
        if self.anchors.len() < 2 {
            return None;
        }

        // Trova la distanza dal punto di ancoraggio più vicino
        let min_distance = self.anchors
            .iter()
            .map(|a| (a.original_time_ms - original_time_ms).abs())
            .min()
            .unwrap_or(0);

        // Stima dell'errore basata sulla distanza (euristica)
        // Più lontano siamo da un'ancora, maggiore è l'incertezza
        Some(min_distance as f64 / 1000.0) // Ritorna in secondi
    }

    /// Svuota tutte le ancore
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
        mapper.add_anchor(AnchorPoint::new(1, 10000, 12000)); // +2 secondi

        assert_eq!(mapper.calculate_offset(5000), 2000);
        assert_eq!(mapper.calculate_offset(10000), 2000);
        assert_eq!(mapper.calculate_offset(20000), 2000);
    }

    #[test]
    fn test_linear_interpolation() {
        let mut mapper = TimeMapper::new();
        mapper.add_anchor(AnchorPoint::new(1, 0, 0));        // offset 0 a t=0
        mapper.add_anchor(AnchorPoint::new(10, 10000, 12000)); // offset +2s a t=10s

        // A metà strada, offset dovrebbe essere +1s
        assert_eq!(mapper.calculate_offset(5000), 1000);
        
        // A 3/4, offset dovrebbe essere +1.5s
        assert_eq!(mapper.calculate_offset(7500), 1500);
    }

    #[test]
    fn test_extrapolation() {
        let mut mapper = TimeMapper::new();
        mapper.add_anchor(AnchorPoint::new(1, 5000, 6000));   // offset +1s a t=5s
        mapper.add_anchor(AnchorPoint::new(10, 10000, 12000)); // offset +2s a t=10s

        // Prima della prima ancora: usa offset della prima
        assert_eq!(mapper.calculate_offset(0), 1000);
        
        // Dopo l'ultima ancora: usa offset dell'ultima
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
        
        assert!(!mapper.remove_anchor(99)); // Non esiste
    }
}
