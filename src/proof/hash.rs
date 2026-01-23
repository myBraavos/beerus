use iamgroot::jsonrpc;
use starknet_crypto::{
    pedersen_hash, poseidon_hash_many, Felt as FieldElement,
};

use crate::gen::Felt;
use crate::proof::types::{
    CONTRACT_STATE_HASH_VERSION, ERROR_CODE_FIELD_ELEMENT,
};

/// Calculate contract state hash
pub fn calculate_contract_state_hash(
    class_hash: &str,
    storage_root: &str,
    nonce: &str,
) -> Result<Felt, jsonrpc::Error> {
    let class_hash_fe = FieldElement::from_hex(class_hash)
        .map_err(|_| create_field_element_error())?;
    let storage_root_fe = FieldElement::from_hex(storage_root)
        .map_err(|_| create_field_element_error())?;
    let nonce_fe = FieldElement::from_hex(nonce)
        .map_err(|_| create_field_element_error())?;

    // The contract state hash is defined as H(H(H(hash, root), nonce), CONTRACT_STATE_HASH_VERSION)
    let hash = pedersen_hash(&class_hash_fe, &storage_root_fe);
    let hash = pedersen_hash(&hash, &nonce_fe);
    let hash = pedersen_hash(
        &hash,
        &FieldElement::from_hex(CONTRACT_STATE_HASH_VERSION)
            .map_err(|_| create_field_element_error())?,
    );

    Felt::try_new(&format!("0x{:x}", hash))
        .map_err(|_| create_field_element_error())
}

/// Calculate global root
pub fn calculate_global_root(
    class_commitment: &Felt,
    storage_commitment: Felt,
) -> Result<Felt, jsonrpc::Error> {
    let global_state_ver =
        FieldElement::from_bytes_be_slice(b"STARKNET_STATE_V0");
    let storage_commitment_fe =
        FieldElement::from_hex(storage_commitment.as_ref())
            .map_err(|_| create_field_element_error())?;
    let class_commitment_fe = FieldElement::from_hex(class_commitment.as_ref())
        .map_err(|_| create_field_element_error())?;

    let hash = poseidon_hash_many(&[
        global_state_ver,
        storage_commitment_fe,
        class_commitment_fe,
    ]);

    Felt::try_new(&format!("0x{:x}", hash))
        .map_err(|_| create_field_element_error())
}

/// Calculate edge hash
pub fn calculate_edge_hash(
    child: &str,
    path: &str,
    length: u64,
) -> Result<FieldElement, jsonrpc::Error> {
    let child_fe = FieldElement::from_hex(child)
        .map_err(|_| create_field_element_error())?;
    let path_fe = FieldElement::from_hex(path)
        .map_err(|_| create_field_element_error())?;

    Ok(pedersen_hash(&child_fe, &path_fe) + FieldElement::from(length))
}

/// Calculate binary node hash
pub fn calculate_binary_hash(
    left: &str,
    right: &str,
) -> Result<FieldElement, jsonrpc::Error> {
    let left_fe = FieldElement::from_hex(left)
        .map_err(|_| create_field_element_error())?;
    let right_fe = FieldElement::from_hex(right)
        .map_err(|_| create_field_element_error())?;

    Ok(pedersen_hash(&left_fe, &right_fe))
}

/// Calculate expected hash for binary node based on direction
pub fn calculate_expected_binary_hash(
    hold: FieldElement,
    left: &str,
    right: &str,
    direction: crate::proof::types::Direction,
) -> Result<FieldElement, jsonrpc::Error> {
    let left_fe = FieldElement::from_hex(left)
        .map_err(|_| create_field_element_error())?;
    let right_fe = FieldElement::from_hex(right)
        .map_err(|_| create_field_element_error())?;

    Ok(match direction {
        crate::proof::types::Direction::Left => pedersen_hash(&hold, &right_fe),
        crate::proof::types::Direction::Right => pedersen_hash(&left_fe, &hold),
    })
}

fn create_field_element_error() -> jsonrpc::Error {
    jsonrpc::Error::new(
        ERROR_CODE_FIELD_ELEMENT,
        "Failed to create Field Element".to_string(),
    )
}
