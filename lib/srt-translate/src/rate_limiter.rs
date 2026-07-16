//! # Rate Limiter Module
//!
//! Implementa un rate limiter basato su token bucket per rispettare
//! i limiti RPM (Richieste Per Minuto) delle API.
//!
//! A differenza del semaforo che limita solo la concorrenza,
//! questo limiter garantisce un numero massimo di richieste per minuto.

use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter as GovRateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;

/// Tipo del rate limiter (non-keyed, in-memory, default clock)
pub type RateLimiter = GovRateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// Crea un nuovo rate limiter configurato per un certo numero di richieste al minuto
///
/// # Argomenti
/// * `rpm` - Richieste per minuto consentite
///
/// # Esempio
/// ```no_run
/// # async fn esempio() {
/// use srt_translate::create_rate_limiter;
///
/// let limiter = create_rate_limiter(15); // 15 RPM per Gemini free tier
/// limiter.until_ready().await; // Aspetta fino a quando è disponibile un token
/// // Fai la richiesta
/// # }
/// ```
pub fn create_rate_limiter(rpm: u32) -> Arc<RateLimiter> {
    // Converti RPM in quota
    // NonZeroU32::new restituisce None se l'input è 0, ma usiamo max(1, rpm)
    // quindi l'unwrap è sicuro (non può mai essere 0)
    let requests_per_minute = NonZeroU32::new(rpm.max(1)).expect("rpm.max(1) is always >= 1");

    // Crea una quota che permette 'rpm' richieste al minuto
    // allow_burst(1) significa che le richieste devono essere spaziate nel tempo,
    // evitando "raffiche" di richieste all'inizio del minuto.
    // Questo è ideale per traduzioni batch che devono rispettare i rate limit API.
    let quota = Quota::per_minute(requests_per_minute)
        .allow_burst(NonZeroU32::new(1).expect("1 is always >= 1"));

    Arc::new(GovRateLimiter::direct(quota))
}

/// Crea un rate limiter con burst (permette un numero iniziale di richieste rapide)
///
/// # Argomenti
/// * `rpm` - Richieste per minuto (refill rate)
/// * `burst` - Numero massimo di richieste in burst
pub fn create_rate_limiter_with_burst(rpm: u32, burst: u32) -> Arc<RateLimiter> {
    // Gli expect sono sicuri perché max(1) garantisce valori >= 1
    let requests_per_minute = NonZeroU32::new(rpm.max(1)).expect("rpm.max(1) is always >= 1");
    let burst_size = NonZeroU32::new(burst.max(1)).expect("burst.max(1) is always >= 1");

    let quota = Quota::per_minute(requests_per_minute).allow_burst(burst_size);

    Arc::new(GovRateLimiter::direct(quota))
}

/// Configurazione per il rate limiting di più provider
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Richieste per minuto consentite
    pub rpm: u32,
    /// Numero massimo di richieste in burst (opzionale)
    pub burst: Option<u32>,
}

impl RateLimitConfig {
    pub fn new(rpm: u32) -> Self {
        Self { rpm, burst: None }
    }

    pub fn with_burst(rpm: u32, burst: u32) -> Self {
        Self {
            rpm,
            burst: Some(burst),
        }
    }

    /// Crea il rate limiter dalla configurazione
    pub fn create_limiter(&self) -> Arc<RateLimiter> {
        match self.burst {
            Some(burst) => create_rate_limiter_with_burst(self.rpm, burst),
            None => create_rate_limiter(self.rpm),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_rate_limiter() {
        let limiter = create_rate_limiter(60);
        // Dovrebbe permettere almeno una richiesta immediata
        assert!(limiter.check().is_ok());
    }

    #[test]
    fn test_rate_limiter_with_burst() {
        let limiter = create_rate_limiter_with_burst(60, 5);
        // Dovrebbe permettere 5 richieste immediate (burst)
        for _ in 0..5 {
            assert!(limiter.check().is_ok());
        }
    }
}
