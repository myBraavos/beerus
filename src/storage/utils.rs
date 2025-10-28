use crate::{client::State, gen::Felt, storage::storage_trait::StorageError};

pub fn parse_state_row(
    row: Option<(i64, String, String, String)>,
) -> Result<State, StorageError> {
    match row {
        Some((block_number, block_hash, root, prev_block_hash)) => {
            Ok(State::new(
                block_number as u64,
                Felt::try_new(&block_hash)
                    .map_err(|e| StorageError::Serde(e))?,
                Felt::try_new(&root).map_err(|e| StorageError::Serde(e))?,
                Felt::try_new(&prev_block_hash)
                    .map_err(|e| StorageError::Serde(e))?,
            ))
        }
        None => Err(StorageError::NotFound("state not found".to_string())),
    }
}
