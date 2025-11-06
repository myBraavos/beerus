use eyre::Result;
use starknet_api::block_hash::block_hash_calculator::{
    calculate_block_commitments, calculate_block_hash,
};

use crate::gen::{BlockWithReceipts, Felt, StateUpdate};

pub fn validate_block_hash(
    block: &BlockWithReceipts,
    state_update: &StateUpdate,
    block_hash: &Felt,
) -> Result<()> {
    let block_header: starknet_api::block::BlockHeaderWithoutHash =
        block.block_header.clone().try_into()?;

    let transactions_data = block.block_body_with_receipts.transactions.clone().into_iter().map(|transaction_and_receipt| {
        transaction_and_receipt.try_into()
    }).collect::<Result<Vec<starknet_api::block_hash::block_hash_calculator::TransactionHashingData>, crate::exe::err::Error>>()?;

    // then calculate block commitments
    let block_commitments = calculate_block_commitments(
        &transactions_data,
        &state_update.state_diff.clone().try_into()?,
        block_header.l1_da_mode,
        &block_header.starknet_version,
    );

    // then calculate block hash
    let calculated_block_hash =
        calculate_block_hash(block_header, block_commitments)?;
    tracing::debug!(calculated_block_hash=?calculated_block_hash, "calculated block hash");

    // it should match the provided hash
    if calculated_block_hash.0
        != starknet_api::hash::StarkHash::from_hex_unchecked(
            block_hash.as_ref(),
        )
    {
        eyre::bail!("Block hash mismatch: expected {block_hash:?} but got {calculated_block_hash:?}");
    }
    Ok(())
}
