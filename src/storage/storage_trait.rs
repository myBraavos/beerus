use crate::{
    client::{l1_range::L1Range, State},
    gen::Felt,
};
use async_trait::async_trait;
use eyre::Result;
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum StorageError {
    #[error("SQL error: {0}")]
    SqlError(#[from] sqlx::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] iamgroot::jsonrpc::Error),
    #[error("not found: {0}")]
    NotFound(String),
}

#[async_trait]
pub trait StorageProviderTrait: Send + Sync + Clone + 'static {
    // l2 state
    async fn read_state(&self, block_number: i64) -> Result<State>;
    async fn read_state_after(&self, block_number: i64) -> Result<State>;
    async fn read_states_by_range(
        &self,
        start_block: i64,
        end_block: i64,
    ) -> Result<Vec<State>>;
    async fn read_state_by_hash(&self, block_hash: &Felt) -> Result<State>;
    async fn read_latest_state(&self) -> Result<State>;
    async fn write_state(&self, state: &State) -> Result<()>;

    // l1 range
    async fn read_l1_range(&self, block_number: i64) -> Result<L1Range>;
    async fn read_latest_l1_range(&self) -> Result<L1Range>;
    async fn find_big_range(
        &self,
        start_block: i64,
        range_size: i64,
    ) -> Result<L1Range>;
    async fn write_l1_range(&self, l1_range: &L1Range) -> Result<()>;
    async fn write_l1_ranges(&self, l1_ranges: &[L1Range]) -> Result<()>;
}
