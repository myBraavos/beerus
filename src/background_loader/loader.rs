use crate::background_loader::async_blocker::AsyncBlocker;
use crate::client::l1_range::L1Range;
use crate::client::state::L1State;
use crate::client::Client;
use crate::client::FIRST_SUPPORTED_BLOCK_NUMBER;
use crate::storage::storage_trait::StorageProviderTrait;
use eyre::Result;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_SUB_RANGE_SIZE: i64 = 10_000;
const LOADER_INTERVAL: Duration = Duration::from_secs(10);
const TIME_PER_BLOCK: i64 = 5; // seconds
const BLOCKS_PER_DAY: i64 = 24 * 60 * 60 / TIME_PER_BLOCK;
const BLOCKS_PER_MONTH: i64 = BLOCKS_PER_DAY * 31;
const BLOCKS_BUFFER: i64 = 3 * 60 / TIME_PER_BLOCK; // 3 minute buffer

#[derive(PartialEq, Eq)]
enum PreloadStatus {
    NoUpdates,
    InProgress,
}

pub struct BackgroundLoader<S: StorageProviderTrait> {
    client: Arc<Client<crate::client::http::Http, S>>,
    async_blocker: Arc<AsyncBlocker>,
}

impl<S: StorageProviderTrait> BackgroundLoader<S> {
    pub fn new(
        client: Arc<Client<crate::client::http::Http, S>>,
        async_blocker: Arc<AsyncBlocker>,
    ) -> Self {
        Self { client, async_blocker }
    }

    pub async fn run(&self) {
        let mut tick = tokio::time::interval(LOADER_INTERVAL);
        loop {
            tick.tick().await;
            self.async_blocker.wait_for_unlock().await;
            // Retry in case of error
            let mut preload_result = match self.preload_l1().await {
                Ok(status) => status,
                Err(e) => {
                    tracing::debug!(error=%e, "failed to preload l1");
                    PreloadStatus::InProgress
                }
            };

            if preload_result == PreloadStatus::NoUpdates {
                self.async_blocker.wait_for_unlock().await;
                preload_result = match self.preload_l2().await {
                    Ok(status) => status,
                    Err(e) => {
                        tracing::debug!(error=%e, "failed to preload l2");
                        PreloadStatus::InProgress
                    }
                };
            }

            if preload_result == PreloadStatus::NoUpdates {
                // Historical data is up-to-date
                break;
            }
        }
    }

    async fn preload_l1(&self) -> Result<PreloadStatus> {
        let latest_stored_state =
            self.client.storage().read_latest_state().await?;
        let start_block = std::cmp::max(
            latest_stored_state.block_number - BLOCKS_PER_MONTH,
            FIRST_SUPPORTED_BLOCK_NUMBER,
        );
        let l1_range = match self
            .client
            .storage()
            .find_big_range(start_block, MAX_SUB_RANGE_SIZE)
            .await
        {
            Ok(l1_range) => l1_range,
            Err(_) => {
                // No big range found, we can stop
                return Ok(PreloadStatus::NoUpdates);
            }
        };

        let mut new_l1_ranges: Vec<L1Range> = vec![];
        let mut prev_state: Option<(L1State, u64)> = None;
        let mut l1_block_start = l1_range.l1_start as u64;
        let mut l1_block_end = l1_range
            .next_end(l1_block_start, self.client.config().l1_range_blocks);

        // Search for sub-ranges above the current block (progressing upward).
        while l1_block_start <= l1_range.l1_end as u64 {
            self.async_blocker.wait_for_unlock().await;
            // Query for all L1 state updates within [l1_block_start, l1_block_end]
            let states = self
                .client
                .l1()
                .get_l1_state_updates(l1_block_start, l1_block_end)
                .await?;

            // Collect and store new ranges and states
            for (state, l1_block_number) in states {
                if let Some((prev_state, prev_l1_block_number)) = prev_state {
                    new_l1_ranges.push(L1Range::new(
                        prev_l1_block_number as i64,
                        l1_block_number as i64,
                        prev_state.block_number,
                        state.block_number,
                    ));
                }
                prev_state = Some((state, l1_block_number));
            }

            // Move to the next range window above.
            l1_block_start = l1_block_end + 1;
            l1_block_end = l1_range
                .next_end(l1_block_end, self.client.config().l1_range_blocks);
        }

        self.client.storage().write_l1_ranges(&new_l1_ranges).await?;

        Ok(PreloadStatus::InProgress)
    }

    async fn preload_l2(&self) -> Result<PreloadStatus> {
        // Calculate blocks since midnight UTC
        let now_utc =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
                as i64;
        const SECONDS_PER_DAY: i64 = 24 * 60 * 60;
        let seconds_since_midnight = now_utc % SECONDS_PER_DAY;
        let blocks_since_midnight = seconds_since_midnight / TIME_PER_BLOCK;

        // Calculate approximate midnight block
        let latest_stored_state =
            self.client.storage().read_latest_state().await?;
        let midnight_block =
            latest_stored_state.block_number - blocks_since_midnight;
        let mut midnight_block_start = midnight_block - BLOCKS_BUFFER;
        let mut midnight_block_end =
            midnight_block + BLOCKS_PER_DAY + BLOCKS_BUFFER;
        let earliest_block = midnight_block_start - BLOCKS_PER_MONTH;

        // Verify all midnight blocks over the last month
        while midnight_block_end > earliest_block {
            self.async_blocker.wait_for_unlock().await;
            let stored_states = self
                .client
                .storage()
                .read_states_by_range(midnight_block_start, midnight_block_end)
                .await;

            if stored_states.is_err() {
                // states not found, need to sync
                let l1_range = self
                    .client
                    .storage()
                    .read_l1_range(midnight_block_end)
                    .await?;
                let gateway_state =
                    self.client.get_gateway_state(l1_range.l2_start).await?;
                let start_state = self
                    .client
                    .get_verified_state(&gateway_state.block_hash, None)
                    .await?;
                let end_state =
                    self.client.get_gateway_state(l1_range.l2_end).await?;
                self.client
                    .verify_state_range(
                        start_state,
                        end_state.into(),
                        Some(self.async_blocker.clone()),
                    )
                    .await?;
            }

            midnight_block_start -= BLOCKS_PER_DAY;
            midnight_block_end -= BLOCKS_PER_DAY;
        }

        Ok(PreloadStatus::InProgress)
    }
}

// TODO: add tests
