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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Storages {
    State,
    L1Range,
}

impl Storages {
    pub fn as_str(&self) -> &'static str {
        match self {
            Storages::State => "State",
            Storages::L1Range => "L1Range",
        }
    }
}

#[async_trait]
pub trait StorageProviderTrait: Send + Sync + Clone + 'static {
    // l2 state
    async fn read_state(&self, block_number: i64) -> Result<State>;
    async fn read_state_by_hash(&self, block_hash: &Felt) -> Result<State>;
    async fn read_latest_state(&self) -> Result<State>;
    async fn write_state(&self, state: &State) -> Result<()>;

    // l1 range
    async fn read_l1_range(&self, block_number: i64) -> Result<L1Range>;
    async fn write_l1_range(&self, l1_range: &L1Range) -> Result<()>;
}
