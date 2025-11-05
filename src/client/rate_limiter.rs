use governor::clock::DefaultClock;
use governor::state::InMemoryState;
use governor::{state::NotKeyed, Quota, RateLimiter as GovernorRateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;

pub struct RateLimiter {
    limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self { limiter: self.limiter.clone() }
    }
}

impl RateLimiter {
    pub fn new(rate_limit: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(rate_limit).unwrap());
        let limiter = Arc::new(GovernorRateLimiter::<
            NotKeyed,
            InMemoryState,
            DefaultClock,
        >::direct(quota));
        Self { limiter }
    }

    pub async fn wait(&self) {
        self.limiter.until_ready().await;
    }
}
