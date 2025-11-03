use crate::{
    client::{l1_range::L1Range, State},
    gen::Felt,
    storage::storage_trait::StorageError,
};
use eyre::Result;

pub fn parse_state_row(row: Option<(i64, String, String)>) -> Result<State> {
    match row {
        Some((block_number, block_hash, root)) => Ok(State::new(
            block_number,
            Felt::try_new(&block_hash).map_err(StorageError::Serde)?,
            Felt::try_new(&root).map_err(StorageError::Serde)?,
        )),
        None => {
            Err(StorageError::NotFound("state not found".to_string()).into())
        }
    }
}

pub fn parse_l1_range_row(
    row: Option<(i64, i64, i64, i64)>,
) -> Result<L1Range> {
    match row {
        Some((l1_start, l1_end, l2_start, l2_end)) => {
            Ok(L1Range::new(l1_start, l1_end, l2_start, l2_end))
        }
        None => {
            Err(StorageError::NotFound("l1 range not found".to_string()).into())
        }
    }
}
