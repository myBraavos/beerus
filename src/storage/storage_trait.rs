use async_trait::async_trait;
use thiserror::Error as ThisError;
use crate::client::State;

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
}

impl Storages {
    pub fn as_str(&self) -> &'static str {
        match self {
            Storages::State => "State",
        }
    }
}

#[async_trait]
pub trait StorageProviderTrait: Send + Sync
{
    async fn read_state(&self, block_number: u64) -> Result<State, StorageError>;
    async fn read_latest_state(&self) -> Result<State, StorageError>;
    async fn write_state(&self, state: &State) -> Result<(), StorageError>;
}
