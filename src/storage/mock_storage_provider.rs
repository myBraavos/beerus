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
    l1_ranges: Arc<Mutex<Vec<L1Range>>>
}

impl MockStorageProvider {
    pub fn new() -> Self {
        Self {
            l1_ranges: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_initial_range(initial_range: L1Range) -> Self {
        Self {
            l1_ranges: Arc::new(Mutex::new(vec![initial_range])),
        }
    }

    pub fn get_l1_ranges(&self) -> Arc<Mutex<Vec<L1Range>>> {
        self.l1_ranges.clone()
    }
}

#[async_trait]
impl StorageProviderTrait for MockStorageProvider {
    async fn read_state(&self, _block_number: i64) -> Result<State> {
        panic!("Not implemented");
    }
    async fn read_state_after(&self, _block_number: i64) -> Result<State> {
        panic!("Not implemented");
    }
    async fn read_states_by_range(
        &self,
        _start_block: i64,
        _end_block: i64,
    ) -> Result<Vec<State>> {
        panic!("Not implemented");
    }
    async fn read_state_by_hash(&self, _block_hash: &Felt) -> Result<State> {
        panic!("Not implemented");
    }
    async fn read_latest_state(&self) -> Result<State> {
        panic!("Not implemented");
    }

    async fn write_state(&self, _state: &State) -> Result<()> {
        Ok(())
    }

    async fn read_l1_range(&self, _block_number: i64) -> Result<L1Range> {
        panic!("Not implemented");
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
        panic!("Not implemented");
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
