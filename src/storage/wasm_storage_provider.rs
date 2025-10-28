use crate::{
    client::State,
    storage::storage_trait::{StorageError, StorageProviderTrait},
};
use async_trait::async_trait;

pub struct WasmStorageProvider {}

#[async_trait]
impl StorageProviderTrait for WasmStorageProvider {
    async fn read_state(
        &self,
        _block_number: u64,
    ) -> Result<State, StorageError> {
        panic!("Not implemented");
    }
    async fn read_latest_state(&self) -> Result<State, StorageError> {
        panic!("Not implemented");
    }

    async fn write_state(&self, _state: &State) -> Result<(), StorageError> {
        panic!("Not implemented");
    }
}
