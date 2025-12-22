use std::time::Duration;

use bitvec::prelude::{BitSlice, BitVec, Msb0};
use bitvec::view::BitView;
use eyre::{eyre, Result};
use starknet_crypto::Felt as FieldElement;
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::Retry;

use crate::r#gen::BlockId;

pub const FIRST_SUPPORTED_BLOCK_NUMBER: i64 = 1_000_000;

pub fn felt_to_bits(felt: &[u8; 32]) -> BitVec<u8, Msb0> {
    felt.view_bits::<Msb0>()[5..].to_bitvec()
}

pub fn felt_from_bits(
    bits: &BitSlice<u8, Msb0>,
    mask: Option<usize>,
) -> Result<FieldElement> {
    if bits.len() != 251 {
        return Err(eyre!("expecting 251 bits"));
    }

    let mask = match mask {
        Some(x) => {
            if x > 251 {
                return Err(eyre!("Mask cannot be bigger than 251"));
            }
            x
        }
        None => 0,
    };

    let mut bytes = [0u8; 32];
    bytes.view_bits_mut::<Msb0>()[5 + mask..].copy_from_bitslice(&bits[mask..]);

    Ok(FieldElement::from_bytes_be(&bytes))
}

pub fn is_block_tag(block_id: &BlockId) -> bool {
    matches!(block_id, BlockId::BlockTag(_))
}

pub fn is_not_verifiable(block_id: &BlockId) -> bool {
    if let BlockId::BlockNumber { block_number } = block_id {
        // FIXME: add verification for older blocks
        block_number.0 <= FIRST_SUPPORTED_BLOCK_NUMBER
    } else {
        false
    }
}

pub async fn with_retry<T, F, Fut>(action: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    const RETRIES: usize = 3;
    const DELAY: u64 = 500; // 0.5 seconds
    const MAX_DELAY: Duration = Duration::from_secs(3);
    let retry_strategy = ExponentialBackoff::from_millis(DELAY)
        .max_delay(MAX_DELAY)
        .take(RETRIES);

    Retry::spawn(retry_strategy, action).await
}

#[cfg(test)]
mod tests {
    use bitvec::{order::Msb0, slice::BitSlice};
    use starknet_crypto::Felt as FieldElement;

    use super::{felt_from_bits, felt_to_bits, is_block_tag, with_retry};
    use crate::r#gen::{BlockHash, BlockId, BlockNumber, BlockTag, Felt};

    #[test]
    fn test_felt_from_bits_invalid() {
        let mut slice = [0u8; 32];
        let bit_slice = BitSlice::<u8, Msb0>::from_slice_mut(&mut slice);
        bit_slice.set(250, true);
        assert!(felt_from_bits(&bit_slice[..250], None).is_err(),);
    }

    #[test]
    fn test_felt_to_bits_three() {
        let val = FieldElement::THREE;
        let mut slice = [0u8; 32];
        let bit_slice = BitSlice::<u8, Msb0>::from_slice_mut(&mut slice);
        bit_slice.set(249, true);
        bit_slice.set(250, true);
        assert_eq!(felt_to_bits(&val.to_bytes_be()), &bit_slice[..251]);
    }

    #[test]
    fn test_felt_to_bits_fourteen() {
        let val = FieldElement::from_dec_str("14").unwrap();
        let mut slice = [0u8; 32];
        let bit_slice = BitSlice::<u8, Msb0>::from_slice_mut(&mut slice);
        bit_slice.set(247, true);
        bit_slice.set(248, true);
        bit_slice.set(249, true);
        assert_eq!(felt_to_bits(&val.to_bytes_be()), &bit_slice[..251]);
    }

    #[test]
    fn test_felt_from_bits_one() {
        let mut slice = [0u8; 32];
        let bit_slice = BitSlice::<u8, Msb0>::from_slice_mut(&mut slice);
        bit_slice.set(250, true);
        assert_eq!(
            felt_from_bits(&bit_slice[..251], None).unwrap(),
            FieldElement::ONE
        );
    }

    #[test]
    fn test_felt_from_bits_seven() {
        let mut slice = [0u8; 32];
        let bit_slice = BitSlice::<u8, Msb0>::from_slice_mut(&mut slice);
        bit_slice.set(248, true);
        bit_slice.set(249, true);
        bit_slice.set(250, true);
        assert_eq!(
            felt_from_bits(&bit_slice[..251], None).unwrap(),
            FieldElement::from_dec_str("7").unwrap()
        );
    }

    #[test]
    fn test_felt_from_bits_mask() {
        let mut slice = [0u8; 32];
        let bit_slice = BitSlice::<u8, Msb0>::from_slice_mut(&mut slice);
        bit_slice.set(0, true);
        bit_slice.set(250, true);
        assert_eq!(
            felt_from_bits(&bit_slice[..251], None).unwrap(),
            FieldElement::from_dec_str(
                "1809251394333065553493296640760748560207343510400633813116524750123642650625"
            )
            .unwrap()
        );
        assert_eq!(
            felt_from_bits(&bit_slice[..251], Some(1)).unwrap(),
            FieldElement::from_dec_str("1").unwrap()
        );
    }

    #[test]
    fn test_felt_from_bits_wrong_mask_value() {
        let mut slice = [0u8; 32];
        let bit_slice = BitSlice::<u8, Msb0>::from_slice_mut(&mut slice);
        assert!(felt_from_bits(&bit_slice[..251], Some(252)).is_err());
    }

    #[tokio::test]
    async fn test_with_retry_success_on_first_attempt() {
        let result = with_retry(|| async { Ok::<i32, eyre::Error>(42) }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_retry_success_after_retries() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        // Atomic counter so we can increment across tasks/calls safely
        let counter = Arc::new(AtomicUsize::new(0));
        let result = {
            let counter = counter.clone();
            with_retry(move || {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    if counter.load(Ordering::SeqCst) < 2 {
                        Err(eyre::eyre!("Temporary failure"))
                    } else {
                        Ok::<i32, eyre::Error>(42)
                    }
                }
            })
            .await
        };
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_with_retry_failure_after_all_retries() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        // Atomic counter so we can increment across tasks/calls safely
        let counter = Arc::new(AtomicUsize::new(0));

        let result = {
            let counter = counter.clone();
            with_retry(move || {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, eyre::Error>(eyre::eyre!("Persistent failure"))
                }
            })
            .await
        };

        assert!(result.is_err());
        // initial attempt + 3 retries = 4 attempts
        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn test_is_block_tag_with_block_tag_latest() {
        let block_id = BlockId::BlockTag(BlockTag::Latest);
        assert!(is_block_tag(&block_id));
    }

    #[test]
    fn test_is_block_tag_with_block_tag_pending() {
        let block_id = BlockId::BlockTag(BlockTag::Pending);
        assert!(is_block_tag(&block_id));
    }

    #[test]
    fn test_is_block_tag_with_block_tag_pre_confirmed() {
        let block_id = BlockId::BlockTag(BlockTag::PreConfirmed);
        assert!(is_block_tag(&block_id));
    }

    #[test]
    fn test_is_block_tag_with_block_hash() {
        let block_hash = BlockHash(Felt::zero());
        let block_id = BlockId::BlockHash { block_hash };
        assert!(!is_block_tag(&block_id));
    }

    #[test]
    fn test_is_block_tag_with_block_number() {
        let block_number = BlockNumber(123);
        let block_id = BlockId::BlockNumber { block_number };
        assert!(!is_block_tag(&block_id));
    }
}
