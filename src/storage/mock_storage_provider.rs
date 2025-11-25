use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    client::{l1_range::L1Range, State},
    gen::Felt,
    storage::storage_trait::StorageProviderTrait,
};
use async_trait::async_trait;
use eyre::Result;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct MockStorageProvider {
    l1_ranges: Arc<Mutex<Vec<L1Range>>>,
    states_by_number: Arc<Mutex<HashMap<i64, State>>>,
    states_by_hash: Arc<Mutex<HashMap<String, State>>>,
    latest_state: Arc<Mutex<Option<State>>>,
    big_range: Arc<Mutex<Option<L1Range>>>,
    states_by_range: Arc<Mutex<HashMap<(i64, i64), Vec<State>>>>,
}

impl MockStorageProvider {
    pub fn new() -> Self {
        Self {
            l1_ranges: Arc::new(Mutex::new(Vec::new())),
            states_by_number: Arc::new(Mutex::new(HashMap::new())),
            states_by_hash: Arc::new(Mutex::new(HashMap::new())),
            latest_state: Arc::new(Mutex::new(None)),
            big_range: Arc::new(Mutex::new(None)),
            states_by_range: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_initial_range(initial_range: L1Range) -> Self {
        Self {
            l1_ranges: Arc::new(Mutex::new(vec![initial_range])),
            states_by_number: Arc::new(Mutex::new(HashMap::new())),
            states_by_hash: Arc::new(Mutex::new(HashMap::new())),
            latest_state: Arc::new(Mutex::new(None)),
            big_range: Arc::new(Mutex::new(None)),
            states_by_range: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_state(self, state: State) -> Self {
        {
            let mut by_number = self
                .states_by_number
                .try_lock()
                .expect("Should be able to lock");
            by_number.insert(state.block_number, state.clone());
        }

        {
            let mut by_hash =
                self.states_by_hash.try_lock().expect("Should be able to lock");
            by_hash.insert(state.block_hash.as_ref().clone(), state.clone());
        }

        {
            let mut latest =
                self.latest_state.try_lock().expect("Should be able to lock");
            *latest = Some(state);
        }
        self
    }

    pub fn with_latest_state(self, state: State) -> Self {
        {
            let mut latest =
                self.latest_state.try_lock().expect("Should be able to lock");
            *latest = Some(state);
        }
        self
    }

    pub fn with_l1_range(self, l1_range: L1Range) -> Self {
        {
            let mut ranges =
                self.l1_ranges.try_lock().expect("Should be able to lock");
            ranges.push(l1_range);
        }
        self
    }

    pub fn get_l1_ranges(&self) -> Arc<Mutex<Vec<L1Range>>> {
        self.l1_ranges.clone()
    }

    pub fn with_big_range(self, big_range: L1Range) -> Self {
        {
            let mut br = self.big_range.try_lock().expect("Should be able to lock");
            *br = Some(big_range);
        }
        self
    }

    pub fn with_states_by_range(self, start_block: i64, end_block: i64, states: Vec<State>) -> Self {
        {
            let mut by_range = self.states_by_range.try_lock().expect("Should be able to lock");
            by_range.insert((start_block, end_block), states);
        }
        self
    }
}

impl Default for MockStorageProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageProviderTrait for MockStorageProvider {
    async fn read_state(&self, block_number: i64) -> Result<State> {
        let states = self.states_by_number.lock().await;
        states.get(&block_number).cloned().ok_or_else(|| {
            eyre::eyre!("State not found for block {}", block_number)
        })
    }

    async fn read_state_after(&self, block_number: i64) -> Result<State> {
        let states = self.states_by_number.lock().await;
        // Find the first state with block_number > the given block_number
        states
            .iter()
            .filter(|(bn, _)| **bn > block_number)
            .min_by_key(|(bn, _)| *bn)
            .map(|(_, state)| state.clone())
            .ok_or_else(|| {
                eyre::eyre!("No state found after block {}", block_number)
            })
    }

    async fn read_states_by_range(
        &self,
        start_block: i64,
        end_block: i64,
    ) -> Result<Vec<State>> {
        let by_range = self.states_by_range.lock().await;
        by_range
            .get(&(start_block, end_block))
            .cloned()
            .ok_or_else(|| {
                eyre::eyre!("States not found for range {} to {}", start_block, end_block)
            })
    }

    async fn read_state_by_hash(&self, block_hash: &Felt) -> Result<State> {
        let states = self.states_by_hash.lock().await;
        states
            .get(block_hash.as_ref())
            .cloned()
            .ok_or_else(|| eyre::eyre!("State not found for hash"))
    }

    async fn read_latest_state(&self) -> Result<State> {
        let latest = self.latest_state.lock().await;
        latest.clone().ok_or_else(|| eyre::eyre!("No latest state"))
    }

    async fn write_state(&self, state: &State) -> Result<()> {
        {
            let mut by_number = self.states_by_number.lock().await;
            by_number.insert(state.block_number, state.clone());
        }

        {
            let mut by_hash = self.states_by_hash.lock().await;
            by_hash.insert(state.block_hash.as_ref().clone(), state.clone());
        }
        Ok(())
    }

    async fn read_l1_range(&self, block_number: i64) -> Result<L1Range> {
        let ranges = self.l1_ranges.lock().await;
        // Find a range that contains the block number
        ranges
            .iter()
            .find(|r| block_number >= r.l2_start && block_number <= r.l2_end)
            .cloned()
            .ok_or_else(|| {
                eyre::eyre!("No L1 range found for block {}", block_number)
            })
    }

    async fn read_latest_l1_range(&self) -> Result<L1Range> {
        let ranges = self.l1_ranges.lock().await;
        ranges.last().cloned().ok_or_else(|| eyre::eyre!("No ranges set"))
    }

    async fn find_big_range(
        &self,
        _start_block: i64,
        _range_size: i64,
    ) -> Result<L1Range> {
        let big_range = self.big_range.lock().await;
        big_range.clone().ok_or_else(|| eyre::eyre!("No big range found"))
    }

    async fn write_l1_range(&self, l1_range: &L1Range) -> Result<()> {
        let mut ranges = self.l1_ranges.lock().await;
        ranges.push(l1_range.clone());
        Ok(())
    }

    async fn write_l1_ranges(&self, _l1_ranges: &[L1Range]) -> Result<()> {
        Ok(())
    }
}
