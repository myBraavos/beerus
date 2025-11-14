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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::storage_trait::StorageError;

    #[test]
    fn test_parse_state_row_success() {
        let block_number = 12345;
        let block_hash = "0x1";
        let root = "0x2";
        let row =
            Some((block_number, block_hash.to_string(), root.to_string()));

        let result = parse_state_row(row).unwrap();
        assert_eq!(result.block_number, block_number);
        assert_eq!(result.block_hash.as_ref(), block_hash);
        assert_eq!(result.root.as_ref(), root);
    }

    #[test]
    fn test_parse_state_row_none() {
        let row = None;
        let result = parse_state_row(row);

        assert!(result.is_err());
        let error = result.unwrap_err();
        if let Ok(storage_error) = error.downcast::<StorageError>() {
            match storage_error {
                StorageError::NotFound(msg) => {
                    assert_eq!(msg, "state not found");
                }
                _ => panic!("Expected NotFound error"),
            }
        } else {
            panic!("Expected StorageError");
        }
    }

    #[test]
    fn test_parse_state_row_invalid_block_hash() {
        let block_number = 12345;
        let invalid_hash = "invalid_hash";
        let root = "0x2";
        let row =
            Some((block_number, invalid_hash.to_string(), root.to_string()));

        let result = parse_state_row(row);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.downcast_ref::<StorageError>().is_some());
    }

    #[test]
    fn test_parse_state_row_invalid_root() {
        let block_number = 12345;
        let block_hash = "0x1";
        let invalid_root = "invalid_root";
        let row = Some((
            block_number,
            block_hash.to_string(),
            invalid_root.to_string(),
        ));

        let result = parse_state_row(row);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.downcast_ref::<StorageError>().is_some());
    }

    #[test]
    fn test_parse_state_row_with_zero_values() {
        let block_number = 0;
        let block_hash = "0x0";
        let root = "0x0";
        let row =
            Some((block_number, block_hash.to_string(), root.to_string()));

        let result = parse_state_row(row).unwrap();
        assert_eq!(result.block_number, block_number);
        assert_eq!(result.block_hash.as_ref(), block_hash);
        assert_eq!(result.root.as_ref(), root);
    }

    #[test]
    fn test_parse_state_row_with_large_values() {
        let block_number = i64::MAX;
        // Valid Felt value (max 63 hex chars after 0x, first char must be 1-9 or a-f)
        let block_hash =
            "0x7ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        let root = "0x1a2b3c4d5e6f";
        let row =
            Some((block_number, block_hash.to_string(), root.to_string()));

        let result = parse_state_row(row).unwrap();
        assert_eq!(result.block_number, block_number);
        assert_eq!(result.block_hash.as_ref(), block_hash);
        assert_eq!(result.root.as_ref(), root);
    }

    #[test]
    fn test_parse_l1_range_row_success() {
        let l1_start = 100;
        let l1_end = 200;
        let l2_start = 1000;
        let l2_end = 2000;
        let row = Some((l1_start, l1_end, l2_start, l2_end));

        let result = parse_l1_range_row(row).unwrap();
        assert_eq!(result.l1_start, l1_start);
        assert_eq!(result.l1_end, l1_end);
        assert_eq!(result.l2_start, l2_start);
        assert_eq!(result.l2_end, l2_end);
    }

    #[test]
    fn test_parse_l1_range_row_none() {
        let row = None;
        let result = parse_l1_range_row(row);

        assert!(result.is_err());
        let error = result.unwrap_err();
        if let Ok(storage_error) = error.downcast::<StorageError>() {
            match storage_error {
                StorageError::NotFound(msg) => {
                    assert_eq!(msg, "l1 range not found");
                }
                _ => panic!("Expected NotFound error"),
            }
        } else {
            panic!("Expected StorageError");
        }
    }

    #[test]
    fn test_parse_l1_range_row_with_zero_values() {
        let l1_start = 0;
        let l1_end = 0;
        let l2_start = 0;
        let l2_end = 0;
        let row = Some((l1_start, l1_end, l2_start, l2_end));

        let result = parse_l1_range_row(row).unwrap();
        assert_eq!(result.l1_start, l1_start);
        assert_eq!(result.l1_end, l1_end);
        assert_eq!(result.l2_start, l2_start);
        assert_eq!(result.l2_end, l2_end);
    }

    #[test]
    fn test_parse_l1_range_row_with_negative_values() {
        let l1_start = -100;
        let l1_end = -50;
        let l2_start = -1000;
        let l2_end = -500;
        let row = Some((l1_start, l1_end, l2_start, l2_end));

        let result = parse_l1_range_row(row).unwrap();
        assert_eq!(result.l1_start, l1_start);
        assert_eq!(result.l1_end, l1_end);
        assert_eq!(result.l2_start, l2_start);
        assert_eq!(result.l2_end, l2_end);
    }

    #[test]
    fn test_parse_l1_range_row_with_large_values() {
        let l1_start = i64::MAX;
        let l1_end = i64::MAX;
        let l2_start = i64::MAX;
        let l2_end = i64::MAX;
        let row = Some((l1_start, l1_end, l2_start, l2_end));

        let result = parse_l1_range_row(row).unwrap();
        assert_eq!(result.l1_start, l1_start);
        assert_eq!(result.l1_end, l1_end);
        assert_eq!(result.l2_start, l2_start);
        assert_eq!(result.l2_end, l2_end);
    }
}
