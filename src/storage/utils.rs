use crate::{client::State, gen::Felt, storage::storage_trait::StorageError};

pub fn parse_state_row(
    row: Option<(i64, String, String)>,
) -> Result<State, StorageError> {
    match row {
        Some((block_number, block_hash, root)) => Ok(State::new(
            block_number,
            Felt::try_new(&block_hash).map_err(StorageError::Serde)?,
            Felt::try_new(&root).map_err(StorageError::Serde)?,
        )),
        None => Err(StorageError::NotFound("state not found".to_string())),
    }
}
