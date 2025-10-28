use serde::{Deserialize, Serialize};

use crate::gen::Felt;

/// Minimal state from feeder gateway
#[derive(Debug, Clone)]
pub struct GatewayState {
    pub block_number: i64,
    pub block_hash: Felt,
}

/// Represents the current state of the Starknet blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub block_number: i64,
    pub block_hash: Felt,
    pub root: Felt,
}

impl State {
    /// Create a new state with the given values
    pub fn new(block_number: i64, block_hash: Felt, root: Felt) -> Self {
        Self {
            block_number,
            block_hash,
            root,
        }
    }
}

impl PartialEq<State> for State {
    fn eq(&self, other: &State) -> bool {
        self.block_number == other.block_number
            && self.root.as_ref() == other.root.as_ref()
            && self.block_hash.as_ref() == other.block_hash.as_ref()
    }
}

impl Eq for State {}

impl Default for State {
    fn default() -> Self {
        Self {
            block_number: 0,
            block_hash: Felt::try_new("0x0").unwrap(),
            root: Felt::try_new("0x0").unwrap(),
        }
    }
}
