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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn test_new() {
        let _limiter = RateLimiter::new(10);
        // Should create without panicking
        assert!(true);
    }

    #[test]
    fn test_new_with_zero_rate_limit() {
        // This should panic because NonZeroU32::new(0) returns None
        // and unwrap() will panic
        let result = std::panic::catch_unwind(|| {
            RateLimiter::new(0);
        });
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_wait_first_call_immediate() {
        let limiter = RateLimiter::new(100);
        let start = Instant::now();
        limiter.wait().await;
        let elapsed = start.elapsed();
        // First call should be immediate (or very fast)
        assert!(elapsed < Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_wait_rate_limiting() {
        let limiter = RateLimiter::new(2); // 2 requests per second
        let start = Instant::now();

        // First call should be immediate
        limiter.wait().await;
        let first_elapsed = start.elapsed();
        assert!(first_elapsed < Duration::from_millis(100));

        // Second call should also be immediate (within the same second)
        limiter.wait().await;
        let second_elapsed = start.elapsed();
        assert!(second_elapsed < Duration::from_millis(100));

        // Third call should be rate limited (should wait ~500ms for the next slot)
        limiter.wait().await;
        let third_elapsed = start.elapsed();
        // Should have waited approximately 500ms (half a second for 2 req/s)
        assert!(third_elapsed >= Duration::from_millis(400));
        assert!(third_elapsed < Duration::from_millis(700));
    }

    #[tokio::test]
    async fn test_wait_multiple_rapid_calls() {
        let limiter = RateLimiter::new(10); // 10 requests per second
        let start = Instant::now();

        // Make 11 rapid calls
        for _ in 0..11 {
            limiter.wait().await;
        }

        let elapsed = start.elapsed();
        // With 10 req/s, the first 10 calls should be immediate (token bucket allows burst)
        // The 11th call should be rate limited and wait for the next token
        // Allow some margin for timing variations
        assert!(elapsed >= Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_clone_shares_rate_limit() {
        let limiter1 = RateLimiter::new(2); // 2 requests per second
        let limiter2 = limiter1.clone();

        let start = Instant::now();

        // Use limiter1 twice
        limiter1.wait().await;
        limiter1.wait().await;

        // Use limiter2 - should be rate limited since they share the same underlying limiter
        limiter2.wait().await;
        let elapsed = start.elapsed();

        // The third call (via limiter2) should be rate limited
        assert!(elapsed >= Duration::from_millis(400));
        assert!(elapsed < Duration::from_millis(700));
    }

    #[tokio::test]
    async fn test_different_limiters_independent() {
        let limiter1 = RateLimiter::new(2); // 2 requests per second
        let limiter2 = RateLimiter::new(2); // 2 requests per second (different instance)

        let start = Instant::now();

        // Use limiter1 twice
        limiter1.wait().await;
        limiter1.wait().await;

        // Use limiter2 - should NOT be rate limited since it's a different instance
        limiter2.wait().await;
        let elapsed = start.elapsed();

        // Should be fast since limiter2 is independent
        assert!(elapsed < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_high_rate_limit() {
        let limiter = RateLimiter::new(1000); // 1000 requests per second
        let start = Instant::now();

        // Make 100 rapid calls
        for _ in 0..100 {
            limiter.wait().await;
        }

        let elapsed = start.elapsed();
        // With 1000 req/s, 100 calls should complete very quickly
        assert!(elapsed < Duration::from_millis(200));
    }
}
