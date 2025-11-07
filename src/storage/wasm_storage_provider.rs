use crate::{
    client::{l1_range::L1Range, State},
    gen::Felt,
    storage::storage_trait::StorageProviderTrait,
};
use async_trait::async_trait;
use eyre::Result;

#[derive(Clone)]
pub struct WasmStorageProvider {}

#[async_trait]
impl StorageProviderTrait for WasmStorageProvider {
    async fn read_state(&self, _block_number: i64) -> Result<State> {
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
        panic!("Not implemented");
    }

    async fn read_l1_range(&self, _block_number: i64) -> Result<L1Range> {
        panic!("Not implemented");
    }
    async fn read_latest_l1_range(&self) -> Result<L1Range> {
        panic!("Not implemented");
    }
    async fn find_big_range(
        &self,
        _start_block: i64,
        _range_size: i64,
    ) -> Result<L1Range> {
        panic!("Not implemented");
    }
    async fn write_l1_range(&self, _l1_range: &L1Range) -> Result<()> {
        panic!("Not implemented");
    }
    async fn write_l1_ranges(&self, _l1_ranges: &[L1Range]) -> Result<()> {
        panic!("Not implemented");
    }
}
