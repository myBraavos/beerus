use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

struct RateLimiterState {
    count: usize,
    window_start: Instant,
}

pub struct RateLimiter {
    batch_size: usize,
    state: Arc<Mutex<RateLimiterState>>,
}

impl RateLimiter {
    pub fn new(batch_size: usize) -> Self {
        Self {
            batch_size,
            state: Arc::new(Mutex::new(RateLimiterState {
                count: 0,
                window_start: Instant::now(),
            })),
        }
    }

    pub fn new_instance(&self) -> Self {
        Self { batch_size: self.batch_size, state: self.state.clone() }
    }

    pub async fn wait(&self) {
        // Wait for permission to start (rate limited to BATCH_SIZE per second)
        loop {
            let mut state = self.state.lock().await;
            let now = Instant::now();

            // If a new second has started, reset the counter
            if now.duration_since(state.window_start) >= Duration::from_secs(1)
            {
                state.count = 0;
                state.window_start = now;
            }

            // Check if we can start a new request in this second
            if state.count < self.batch_size {
                state.count += 1;
                break; // Got permission, proceed
            }

            // Need to wait until next second - calculate sleep duration
            let sleep_until = state.window_start + Duration::from_secs(1);
            let sleep_duration = sleep_until.saturating_duration_since(now);
            drop(state);

            if sleep_duration > Duration::ZERO {
                tokio::time::sleep(sleep_duration).await;
            }
        }
    }
}
