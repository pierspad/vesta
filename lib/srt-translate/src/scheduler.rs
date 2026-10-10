use crate::{RateLimiter, Translator};
use std::sync::Arc;

// =====================================================================================
//  TIERED SCHEDULER
//
//  Un "pool" è una lista ordinata di tier. Ogni tier contiene una o più `PoolEntry`
//  (provider + modello + API key, ciascuna con il proprio rate limiter e budget).
//
//  Politica di esecuzione:
//   • All'interno di un tier le entry vengono usate in round-robin (carico bilanciato).

#[derive(Clone)]
pub struct PoolEntry {
    pub translator: Translator,

    pub rate_limiter: Option<Arc<RateLimiter>>,

    pub max_requests: Option<u32>,

    pub label: String,
}

pub type TranslatorPool = Vec<Vec<PoolEntry>>;

struct EntryRuntime {
    exhausted: bool,
    remaining: Option<u32>,
}

struct TierRuntime {
    entries: Vec<EntryRuntime>,
    cursor: usize,
}

pub struct TierScheduler {
    tiers: Vec<TierRuntime>,
    active: usize,
}

impl TierScheduler {
    pub fn new(pool: &TranslatorPool) -> Self {
        let tiers = pool
            .iter()
            .map(|entries| TierRuntime {
                entries: entries
                    .iter()
                    .map(|e| EntryRuntime {
                        exhausted: false,
                        remaining: e.max_requests,
                    })
                    .collect(),
                cursor: 0,
            })
            .collect();
        Self { tiers, active: 0 }
    }

    pub fn acquire(&mut self) -> Option<(usize, usize)> {
        while self.active < self.tiers.len() {
            let active = self.active;
            let tier = &mut self.tiers[active];
            let n = tier.entries.len();
            if n > 0 {
                for off in 0..n {
                    let i = (tier.cursor + off) % n;
                    let entry = &mut tier.entries[i];
                    if entry.exhausted {
                        continue;
                    }
                    if let Some(0) = entry.remaining {
                        entry.exhausted = true;
                        continue;
                    }
                    if let Some(r) = entry.remaining.as_mut() {
                        *r -= 1;
                    }
                    tier.cursor = (i + 1) % n;
                    return Some((active, i));
                }
            }

            self.active += 1;
        }
        None
    }

    pub fn report_exhausted(&mut self, tier: usize, idx: usize) {
        if let Some(t) = self.tiers.get_mut(tier)
            && let Some(e) = t.entries.get_mut(idx)
        {
            e.exhausted = true;
        }
    }

    pub fn active_tier_human(&self) -> usize {
        self.active + 1
    }
}

pub fn is_rate_limit_error(error: &anyhow::Error) -> bool {
    let s = error.to_string().to_lowercase();
    s.contains("429")
        || s.contains("rate limit")
        || s.contains("rate-limit")
        || s.contains("quota")
        || s.contains("resource_exhausted")
        || s.contains("resource exhausted")
        || s.contains("too many requests")
        || s.contains("limit exceeded")
        || s.contains("insufficient_quota")
}

pub fn pool_concurrency(pool: &TranslatorPool) -> usize {
    pool.iter().map(|t| t.len()).max().unwrap_or(1).clamp(1, 16)
}
