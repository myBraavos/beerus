use iamgroot::jsonrpc;

use crate::gen::{Address, ContractLeafData, Felt, GetProofResult, StorageKey};
use crate::proof::hash::{
    calculate_contract_state_hash, calculate_global_root,
};
use crate::proof::merkle::parse_proof;
use crate::proof::types::ERROR_CODE_PROOF_INVALID;

/// Parser for proof verification
pub struct ProofParser;

impl ProofParser {
    /// Verify storage proofs
    pub fn verify_storage_proofs(
        proof: &GetProofResult,
        contract_leaf: &ContractLeafData,
        storage_key: StorageKey,
        storage_value: Felt,
    ) -> Result<(), jsonrpc::Error> {
        let expected_storage_root = &contract_leaf.storage_root;
        let storage_proofs = proof
            .contracts_storage_proofs
            .first()
            .ok_or(create_proof_error("No storage proof found"))?;

        match parse_proof(storage_key.as_ref(), storage_value, storage_proofs)? {
            Some(computed_storage_root) if computed_storage_root.as_ref() == expected_storage_root.as_ref() => {
                Ok(())
            }
            Some(computed_storage_root) => {
                Err(create_proof_error(&format!(
                    "Storage proof invalid:\nexpected root -> {}\ncomputed root -> {}\n",
                    expected_storage_root.as_ref(), computed_storage_root.as_ref()
                )))
            }
            None => Err(create_proof_error(&format!(
                "Storage proof invalid for root -> {}\n",
                expected_storage_root.as_ref()
            ))),
        }
    }

    /// Verify contracts proof
    pub fn verify_contracts_proof(
        proof: &GetProofResult,
        contract_leaf: &ContractLeafData,
        expected_global_root: Felt,
        contract_address: Address,
    ) -> Result<(), jsonrpc::Error> {
        let contract_state_hash = calculate_contract_state_hash(
            contract_leaf.class_hash.as_ref(),
            contract_leaf.storage_root.as_ref(),
            contract_leaf.nonce.as_ref(),
        )?;

        match parse_proof(
            contract_address.0.as_ref(),
            contract_state_hash,
            &proof.contracts_proof.nodes,
        )? {
            Some(computed_storage_commitment) => {
                let class_commitment = &proof.global_roots.classes_tree_root;
                let computed_global_root = calculate_global_root(
                    class_commitment,
                    computed_storage_commitment.clone(),
                )?;

                let expected_state_commitment =
                    &proof.global_roots.contracts_tree_root;
                if expected_state_commitment.as_ref()
                    == computed_storage_commitment.as_ref()
                    && expected_global_root.as_ref()
                        == computed_global_root.as_ref()
                {
                    Ok(())
                } else {
                    Err(create_proof_error(&format!(
                        "Contract proof invalid:\nexpected state commitment -> {}\ncomputed storage commitment -> {}\nexpected global root -> {}\ncomputed global root -> {}",
                        expected_state_commitment.as_ref(), computed_storage_commitment.as_ref(), expected_global_root.as_ref(), computed_global_root.as_ref()
                    )))
                }
            }
            None => Err(create_proof_error(&format!(
                "Could not parse global root for expected root: {}",
                expected_global_root.as_ref()
            ))),
        }
    }
}

fn create_proof_error(message: &str) -> jsonrpc::Error {
    jsonrpc::Error::new(ERROR_CODE_PROOF_INVALID, message.to_string())
}
