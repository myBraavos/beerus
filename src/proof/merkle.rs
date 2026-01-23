use std::collections::HashMap;

use iamgroot::jsonrpc;
use starknet_crypto::Felt as FieldElement;

use crate::gen::{
    BinaryNode, BinaryNodeBinary, EdgeNode, EdgeNodeEdge, Felt, Node,
};
use crate::proof::hash::{
    calculate_binary_hash, calculate_edge_hash, calculate_expected_binary_hash,
};
use crate::proof::types::{Direction, ERROR_CODE_FIELD_ELEMENT, MERKLE_HEIGHT};
use crate::util::{felt_from_bits, felt_to_bits};

/// Index for efficient node lookup in merkle proofs
pub struct NodeIndex {
    nodes: Vec<Node>,
    index: HashMap<String, Option<usize>>, // key(hex) -> index in `nodes`
}

impl NodeIndex {
    pub fn new(nodes: Vec<Node>) -> Self {
        let mut idx: HashMap<String, Option<usize>> = HashMap::new();

        for (i, node) in nodes.iter().enumerate() {
            match node {
                Node::BinaryNode(b) => {
                    // index by left, right, and node_hash
                    idx.insert(format!("{:?}", b.node.left), Some(i));
                    idx.insert(format!("{:?}", b.node.right), Some(i));
                }
                Node::EdgeNode(e) => {
                    // index by child (from edge), and by node_hash
                    idx.insert(format!("{:?}", e.node.child), Some(i));
                }
            }
        }

        NodeIndex { nodes, index: idx }
    }

    /// Query returns zero-or-more matching nodes for a given hex key.
    pub fn query(&self, key: &Felt) -> Vec<&Node> {
        match self.index.get(&format!("{:?}", key)) {
            Some(Some(idx)) => vec![&self.nodes[*idx]],
            Some(None) | None => vec![],
        }
    }
}

/// Parse and verify a merkle proof
pub fn parse_proof(
    key: impl Into<String>,
    value: Felt,
    proof: &[Node],
) -> Result<Option<Felt>, jsonrpc::Error> {
    let key = FieldElement::from_hex(&key.into())
        .map_err(|_| create_field_element_error())?;
    let key = felt_to_bits(&key.to_bytes_be());

    if key.len() != MERKLE_HEIGHT {
        return Ok(None);
    }

    let value = FieldElement::from_hex(value.as_ref())
        .map_err(|_| create_field_element_error())?;

    let node_index = NodeIndex::new(proof.to_vec());
    // initialized to the value so if the last node
    // in the proof is a binary node we can still verify
    let (mut current_hash, mut path_len) = (value, 0);
    // reverse the proof in order to hash from the leaf towards the root
    // iterate until no more nodes are found
    let mut i = 0;

    loop {
        let nodes =
            node_index.query(&Felt::try_new(&format!("{:?}", current_hash))?);
        if nodes.is_empty() {
            // No more nodes found, break out of the loop
            break;
        }

        for node in nodes {
            match node {
                Node::EdgeNode(EdgeNode {
                    node: EdgeNodeEdge { child, path, length },
                    node_hash: _,
                }) => {
                    // calculate edge hash given by provider
                    let provided_hash = calculate_edge_hash(
                        child.as_ref(),
                        path.as_ref(),
                        *length as u64,
                    )?;

                    if i == 0 {
                        // mask storage key
                        let computed_hash = match felt_from_bits(
                            &key,
                            Some(MERKLE_HEIGHT - *length as usize),
                        ) {
                            Ok(masked_key) => calculate_edge_hash(
                                &format!("0x{:x}", value),
                                &format!("0x{:x}", masked_key),
                                *length as u64,
                            )?,
                            Err(_) => return Ok(None),
                        };
                        // verify computed hash against provided hash
                        if provided_hash != computed_hash {
                            return Ok(None);
                        };
                    }

                    // walk up the remaining path
                    path_len += *length;
                    current_hash = provided_hash;
                }
                Node::BinaryNode(BinaryNode {
                    node: BinaryNodeBinary { left, right },
                    node_hash: _,
                }) => {
                    path_len += 1;
                    let _left_fe = FieldElement::from_hex(left.as_ref())
                        .map_err(|_| create_field_element_error())?;
                    let _right_fe = FieldElement::from_hex(right.as_ref())
                        .map_err(|_| create_field_element_error())?;

                    // identify path direction for this node
                    let expected_hash = calculate_expected_binary_hash(
                        current_hash,
                        left.as_ref(),
                        right.as_ref(),
                        Direction::from(key[MERKLE_HEIGHT - path_len as usize]),
                    )?;

                    current_hash =
                        calculate_binary_hash(left.as_ref(), right.as_ref())?;
                    // verify calculated hash vs provided hash for the node
                    if current_hash != expected_hash {
                        return Ok(None);
                    };
                }
            };
        }
        i += 1; // increment counter for next iteration
    }

    Ok(Some(Felt::try_new(&format!("0x{:x}", current_hash))?))
}

fn create_field_element_error() -> jsonrpc::Error {
    jsonrpc::Error::new(
        ERROR_CODE_FIELD_ELEMENT,
        "Failed to create Field Element".to_string(),
    )
}
