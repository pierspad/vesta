use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter as GovRateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;

pub type RateLimiter = GovRateLimiter<NotKeyed, InMemoryState, DefaultClock>;

pub fn create_rate_limiter(rpm: u32) -> Arc<RateLimiter> {
    create_rate_limiter_with_burst(rpm, NonZeroU32::MIN.get())
}

pub fn create_rate_limiter_with_burst(rpm: u32, burst: u32) -> Arc<RateLimiter> {
    let requests_per_minute = NonZeroU32::new(rpm).unwrap_or(NonZeroU32::MIN);
    let burst_size = NonZeroU32::new(burst).unwrap_or(NonZeroU32::MIN);

    let quota = Quota::per_minute(requests_per_minute).allow_burst(burst_size);

    Arc::new(GovRateLimiter::direct(quota))
}

#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    pub rpm: u32,

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

        assert!(limiter.check().is_ok());
    }

    #[test]
    fn test_rate_limiter_with_burst() {
        let limiter = create_rate_limiter_with_burst(60, 5);

        for _ in 0..5 {
            assert!(limiter.check().is_ok());
        }
    }
}
