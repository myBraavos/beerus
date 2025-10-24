//! # Proof Verification Module
//!
//! This module provides functionality for verifying Merkle proofs in the Starknet ecosystem.
//! It handles the verification of storage proofs, contract proofs, and global state proofs
//! using various cryptographic hash functions and Merkle tree operations.
//!
//! ## Key Components
//!
//! - **ProofParser**: Main parser for proof verification logic
//! - **Hash Functions**: Pedersen and Poseidon hash calculations
//! - **Merkle Operations**: Tree traversal and root calculations
//! - **Type Definitions**: Constants and error codes for proof verification
//!
//! ## Proof Types
//!
//! - **Storage Proofs**: Verify storage values at specific keys
//! - **Contract Proofs**: Verify contract state and class information
//! - **Global Proofs**: Verify the global state root

pub mod types;
pub mod hash;
pub mod merkle;
pub mod parser;

use iamgroot::jsonrpc;

use crate::gen::{
    Address, Felt, GetProofResult, StorageKey,
};

/// Verify a complete proof
///
/// This function performs comprehensive proof verification by checking both
/// storage proofs and contract proofs against the provided global root.
///
/// # Arguments
///
/// * `proof` - The complete proof data from the RPC response
/// * `global_root` - The expected global state root
/// * `contract_address` - The address of the contract being verified
/// * `key` - The storage key being verified
/// * `value` - The expected storage value
///
/// # Returns
///
/// Returns `Ok(())` if the proof is valid, or an error if verification fails.
///
/// # Errors
///
/// This function can return various proof verification errors including:
/// - Missing contract leaf data
/// - Invalid storage proofs
/// - Invalid contract proofs
/// - Hash calculation errors
pub fn verify_proof(
    proof: &GetProofResult,
    global_root: Felt,
    contract_address: Address,
    key: StorageKey,
    value: Felt,
) -> Result<(), jsonrpc::Error> {
    let contract_leaf = proof.contracts_proof.contract_leaves_data.first().ok_or(
        jsonrpc::Error::new(-32700, "No contract leaf data found".to_string()),
    )?;

    parser::ProofParser::verify_storage_proofs(proof, contract_leaf, key, value)?;
    parser::ProofParser::verify_contracts_proof(proof, contract_leaf, global_root, contract_address)
}

impl GetProofResult {
    /// Verify this proof against the given parameters
    ///
    /// This is a convenience method that calls the global `verify_proof` function
    /// with the current proof instance.
    ///
    /// # Arguments
    ///
    /// * `global_root` - The expected global state root
    /// * `contract_address` - The address of the contract being verified
    /// * `key` - The storage key being verified
    /// * `value` - The expected storage value
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the proof is valid, or an error if verification fails.
    pub fn verify(
        &self,
        global_root: Felt,
        contract_address: Address,
        key: StorageKey,
        value: Felt,
    ) -> Result<(), jsonrpc::Error> {
        verify_proof(self, global_root, contract_address, key, value)
    }
}

#[cfg(test)]
mod tests {
    use crate::gen::{
        Address, Felt, GetProofResult, Node, StorageKey, ContractLeafData, ProofData, GlobalRoots
    };

    #[test]
    fn valid_one_level_parse_proof() {
        let key = "0x0341c1bdfd89f69748aa00b5742b03adbffd79b8e80cab5c50d91cd8c2a79be1".to_string();
        let value =
            Felt::try_new("0x47616d65206f66204c69666520546f6b656e").unwrap();
        let edge_node_string = r#"[{
            "node": {
                "child": "0x47616d65206f66204c69666520546f6b656e",
                "path": "0x3dfd89f69748aa00b5742b03adbffd79b8e80cab5c50d91cd8c2a79be1",
                "length": 231
            },
            "node_hash": "0x0"
        }]"#;
        let proof: Vec<Node> = serde_json::from_str(edge_node_string).unwrap();
        let ret_val = crate::proof::merkle::parse_proof(key, value, &proof).unwrap();

        assert!(ret_val.is_some());
        let ret_val = ret_val.unwrap();
        assert_eq!(
            ret_val.as_ref(),
            "0x1e224db31dfb3e1b8c95670a12f1903d4a32ac7bb83f4b209029e14155bbca9"
        );
    }

    #[test]
    fn valid_five_level_parse_proof() {
        let key = "0x0341c1bdfd89f69748aa00b5742b03adbffd79b8e80cab5c50d91cd8c2a79be1".to_string();
        let value =
            Felt::try_new("0x47616d65206f66204c69666520546f6b656e").unwrap();
        let proof_string = r#"[
        {
            "node": {
                "left": "0x46e82293b0564764a071f1aa4488aa7577b1b5bb2e898321f8536d5593d371d",
                "right": "0x58adcf6ea8b96992aa316e2f092f2480ca406c3630fe97573046a32900745b5"
            },
            "node_hash": "0x0"
        },
        {
            "node": {
                "left": "0x716e211c75f4c0e14dbe46c361812b0129abd061b63faf91ad5569bf22b785c",
                "right": "0x3729d9699d4410223e413f3b3aa91a043d94242f888188036e6ea25b6962041"
            },
            "node_hash": "0x0"
        },
        {
            "node": {
                "child": "0x6281e42b5941ae1a77ea03836aad1190097f72e1a1ed534fae2e00b4118f504",
                "path": "0x1",
                "length": 1
            },
            "node_hash": "0x0"
        },
        {
            "node": {
                "left": "0x3e3800516f62800ef6491b1cb1915b3353026ea6a6afcf35e8d4c54e35b04ea",
                "right": "0x1e224db31dfb3e1b8c95670a12f1903d4a32ac7bb83f4b209029e14155bbca9"
            },
            "node_hash": "0x0"
        },
        {
            "node": {
                "child": "0x47616d65206f66204c69666520546f6b656e",
                "path": "0x3dfd89f69748aa00b5742b03adbffd79b8e80cab5c50d91cd8c2a79be1",
                "length": 231
            },
            "node_hash": "0x0"
        }]"#;
        let proof: Vec<Node> = serde_json::from_str(proof_string).unwrap();
        let ret_val = crate::proof::merkle::parse_proof(key, value, &proof).unwrap();

        assert!(ret_val.is_some());
        let ret_val = ret_val.unwrap();
        assert_eq!(
            ret_val.as_ref(),
            "0x6cc50a732b4256f7b642348e19bd1a8bee7ac76bed3fcee3bc34309538c00c6"
        );
    }

    #[test]
    fn invalid_one_level_parse_proof() {
        let key = "0xabc".to_string();
        let value = Felt::try_new("0xdef").unwrap();
        let proof: Vec<Node> = serde_json::from_str(
            r#"[{
            "node": {
                "child": "0xfaa",
                "path": "0xbad",
                "length": 1
            },
            "node_hash": "0x0"
        }]"#,
        )
        .unwrap();
        assert!(crate::proof::merkle::parse_proof(key, value, &proof)
            .unwrap()
            .is_some()); // new logic returns last value instead on none
    }

    #[test]
    fn invalid_one_level_proof_last_key_byte_2_instead_of_1() {
        let key = "0x0341c1bdfd89f69748aa00b5742b03adbffd79b8e80cab5c50d91cd8c2a79be2".to_string();
        let value =
            Felt::try_new("0x47616d65206f66204c69666520546f6b656e").unwrap();
        let edge_node_string = r#"[{
            "node": {
                "child": "0x47616d65206f66204c69666520546f6b656e",
                "path": "0x3dfd89f69748aa00b5742b03adbffd79b8e80cab5c50d91cd8c2a79be1",
                "length": 231
            },
            "node_hash": "0x0"
        }]"#;
        let proof: Vec<Node> = serde_json::from_str(edge_node_string).unwrap();
        assert!(crate::proof::merkle::parse_proof(key, value, &proof)
            .unwrap()
            .is_none());
    }

    #[test]
    fn invalid_five_level_proof_len_7_instead_of_1() {
        let key = "0x0341c1bdfd89f69748aa00b5742b03adbffd79b8e80cab5c50d91cd8c2a79be1".to_string();
        let value =
            Felt::try_new("0x47616d65206f66204c69666520546f6b656e").unwrap();
        let proof_string = r#"[
        {
            "node": {
                "left": "0x46e82293b0564764a071f1aa4488aa7577b1b5bb2e898321f8536d5593d371d",
                "right": "0x58adcf6ea8b96992aa316e2f092f2480ca406c3630fe97573046a32900745b5"
            },
            "node_hash": "0x0"
        },
        {
            "node": {
                "left": "0x716e211c75f4c0e14dbe46c361812b0129abd061b63faf91ad5569bf22b785c",
                "right": "0x3729d9699d4410223e413f3b3aa91a043d94242f888188036e6ea25b6962041"
            },
            "node_hash": "0x0"
        },
        {
            "node": {
                "child": "0x6281e42b5941ae1a77ea03836aad1190097f72e1a1ed534fae2e00b4118f504",
                "path": "0x1",
                "length": 7
            },
            "node_hash": "0x0"
        },
        {
            "node": {
                "left": "0x3e3800516f62800ef6491b1cb1915b3353026ea6a6afcf35e8d4c54e35b04ea",
                "right": "0x1e224db31dfb3e1b8c95670a12f1903d4a32ac7bb83f4b209029e14155bbca9"
            },
            "node_hash": "0x0"
        },
        {
            "node": {
                "child": "0x47616d65206f66204c69666520546f6b656e",
                "path": "0x3dfd89f69748aa00b5742b03adbffd79b8e80cab5c50d91cd8c2a79be1",
                "length": 231
            },
            "node_hash": "0x0"
        }]"#;
        let proof: Vec<Node> = serde_json::from_str(proof_string).unwrap();
        assert!(crate::proof::merkle::parse_proof(key, value, &proof)
            .unwrap()
            .is_some()); // new logic returns last value instead on none
    }

    #[test]
    fn valid_one_level_verify_storage_proof() {
        let key = StorageKey::try_new(
            "0x0341c1bdfd89f69748aa00b5742b03adbffd79b8e80cab5c50d91cd8c2a79be1",
        ).unwrap();
        let value =
            Felt::try_new("0x47616d65206f66204c69666520546f6b656e").unwrap();
        let edge_node_string = r#"[{
            "node": {
                "child": "0x47616d65206f66204c69666520546f6b656e",
                "path": "0x3dfd89f69748aa00b5742b03adbffd79b8e80cab5c50d91cd8c2a79be1",
                "length": 231
            },
            "node_hash": "0x0"
        }]"#;

        let storage_proof = GetProofResult {
            classes_proof: vec![],
            contracts_proof: ProofData {
                nodes: vec![],
                contract_leaves_data: vec![ContractLeafData {
                    class_hash: Felt::try_new("0x0").unwrap(),
                    nonce: Felt::try_new("0x0").unwrap(),
                    storage_root: Felt::try_new(
                        "0x1e224db31dfb3e1b8c95670a12f1903d4a32ac7bb83f4b209029e14155bbca9",
                    )
                    .unwrap(),
                }],
            },
            contracts_storage_proofs: vec![serde_json::from_str(edge_node_string).unwrap()],
            global_roots: GlobalRoots {
                block_hash: Felt::try_new("0x0").unwrap(),
                classes_tree_root: Felt::try_new("0x0").unwrap(),
                contracts_tree_root: Felt::try_new("0x0").unwrap(),
            },
        };
        let contract_leaf = &storage_proof.contracts_proof.contract_leaves_data[0];

        assert!(crate::proof::parser::ProofParser::verify_storage_proofs(
            &storage_proof, contract_leaf, key, value
        ).is_ok());
    }

    #[test]
    fn invalid_one_level_verify_storage_proof() {
        let key = StorageKey::try_new("0x0341c1bdfd89f69748aa00b5742b03adbffd79b8e80cab5c50d91cd8c2a79be1").unwrap();
        let value = Felt::try_new("0xdef").unwrap();
        let edge_node_string = r#"[{
            "node": {
                "child": "0xbad",
                "path": "0xfaa",
                "length": 231
            },
            "node_hash": "0x0"
        }]"#;

        let storage_proof = GetProofResult {
            classes_proof: vec![],
            contracts_proof: ProofData {
                nodes: vec![],
                contract_leaves_data: vec![ContractLeafData {
                    class_hash: Felt::try_new("0x0").unwrap(),
                    nonce: Felt::try_new("0x0").unwrap(),
                    storage_root: Felt::try_new("0x42").unwrap(),
                }],
            },
            contracts_storage_proofs: vec![serde_json::from_str(
                edge_node_string,
            )
            .unwrap()],
            global_roots: GlobalRoots {
                block_hash: Felt::try_new("0x0").unwrap(),
                classes_tree_root: Felt::try_new("0x0").unwrap(),
                contracts_tree_root: Felt::try_new("0x0").unwrap(),
            },
        };
        let contract_leaf = &storage_proof.contracts_proof.contract_leaves_data[0];

        assert!(crate::proof::parser::ProofParser::verify_storage_proofs(
            &storage_proof, contract_leaf, key, value
        ).is_err());
    }

    #[test]
    fn contract_state_hash_is_valid() {
        let contract_leaf = ContractLeafData {
            class_hash: Felt::try_new("0x123").unwrap(),
            storage_root: Felt::try_new("0xabc").unwrap(),
            nonce: Felt::try_new("0xdef").unwrap(),
        };

        let expected =
            "0x30a3c317f49a18c65bb5d22c87172f3f60101d54425457a66237474dd2d66db";
        assert_eq!(
            crate::proof::hash::calculate_contract_state_hash(
                contract_leaf.class_hash.as_ref(),
                contract_leaf.storage_root.as_ref(),
                contract_leaf.nonce.as_ref()
            )
                .unwrap()
                .as_ref(),
            Felt::try_new(expected).unwrap().as_ref()
        );
    }

    #[test]
    fn calculate_global_root_is_valid() {
        let expected =
            "0x42e26eb87a82c4b4130cb6bfbd33be7788436aa66f787ede4aef9456b58939";
        assert_eq!(
            crate::proof::hash::calculate_global_root(
                &Felt::try_new("0xabc").unwrap(),
                Felt::try_new("0xdef").unwrap()
            )
            .unwrap()
            .as_ref(),
            Felt::try_new(expected).unwrap().as_ref()
        );
    }

    #[test]
    fn valid_verify_contracts_proof() {
        let edge_node_string = r#"[{
            "node": {
                "child": "0x538a7653ef22e217f93066ac54784c0159a5e1e37d808f83c82d1b42d57457d",
                "path": "0x4a03bb9e744479e3298f54705a35966ab04140d3d8dd797c1f6dc49d0",
                "length": 229
            },
            "node_hash": "0x0"
        }]"#;
        let storage_proof = GetProofResult {
            classes_proof: vec![],
            contracts_proof: ProofData {
                nodes: serde_json::from_str(edge_node_string).unwrap(),
                contract_leaves_data: vec![ContractLeafData {
                    class_hash: Felt::try_new(
                        "0x4e635d495504b31ec191cbfc3d99b5d109bfcae4d0d9e16f4909a43b2e24c07",
                    )
                    .unwrap(),
                    storage_root: Felt::try_new(
                        "0x5826149cbab3f8538d346301869ba2742a159d1542463ce19a60a927b826a2f",
                    )
                    .unwrap(),
                    nonce: Felt::try_new("0x0").unwrap(),
                }],
            },
            contracts_storage_proofs: vec![vec![]],
            global_roots: GlobalRoots {
                block_hash: Felt::try_new("0x0").unwrap(),
                classes_tree_root: Felt::try_new("0x0").unwrap(),
                contracts_tree_root: Felt::try_new("0x2416e88b266ce76a4864b68e8169e0c08d37a48764f8780a522f203556fdc0c")
                    .unwrap(),
            },
        };

        let global_root = Felt::try_new(
            "0x1e2a7a7ee40c1d897c8c0a9515720ea02c8075ee9e00db277f5f8c3e4edcb54",
        )
        .unwrap();
        let contract_address = Address(Felt::try_new("0x6a05844a03bb9e744479e3298f54705a35966ab04140d3d8dd797c1f6dc49d0")
                .unwrap());
        let contract_leaf = &storage_proof.contracts_proof.contract_leaves_data[0];
        assert!(crate::proof::parser::ProofParser::verify_contracts_proof(
            &storage_proof, contract_leaf, global_root, contract_address
        ).is_ok());
    }

    #[test]
    fn invalid_verify_contracts_proof() {
        let invalid_storage_proof = GetProofResult {
            classes_proof: vec![],
            contracts_proof: ProofData {
                nodes: vec![],
                contract_leaves_data: vec![ContractLeafData {
                    class_hash: Felt::try_new("0x0").unwrap(),
                    nonce: Felt::try_new("0x0").unwrap(),
                    storage_root: Felt::try_new("0x0").unwrap(),
                }],
            },
            contracts_storage_proofs: vec![vec![]],
            global_roots: GlobalRoots {
                block_hash: Felt::try_new("0x0").unwrap(),
                classes_tree_root: Felt::try_new("0x0").unwrap(),
                contracts_tree_root: Felt::try_new("0x0").unwrap(),
            },
        };
        let global_root = Felt::try_new("0x0").unwrap();
        let contract_address = Address(Felt::try_new("0x0").unwrap());
        let contract_leaf = &invalid_storage_proof.contracts_proof.contract_leaves_data[0];
        assert!(crate::proof::parser::ProofParser::verify_contracts_proof(
            &invalid_storage_proof, contract_leaf, global_root, contract_address
        ).is_err());
    }
}
