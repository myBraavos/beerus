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
    pub timestamp: i64,
    pub block_hash: Felt,
    pub root: Felt,
}

impl State {
    /// Create a new state with the given values
    pub fn new(
        block_number: i64,
        timestamp: i64,
        block_hash: Felt,
        root: Felt,
    ) -> Self {
        Self { block_number, timestamp, block_hash, root }
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
            timestamp: 0,
            block_hash: Felt::zero().clone(),
            root: Felt::zero().clone(),
        }
    }
}

impl From<GatewayState> for State {
    fn from(gateway_state: GatewayState) -> Self {
        Self {
            block_number: gateway_state.block_number,
            timestamp: 0,
            block_hash: gateway_state.block_hash,
            root: Felt::zero().clone(),
        }
    }
}

impl From<L1State> for State {
    fn from(l1_state: L1State) -> Self {
        Self {
            block_number: l1_state.block_number,
            timestamp: 0,
            block_hash: l1_state.block_hash,
            root: l1_state.root,
        }
    }
}

/// Represents the current state of the Starknet blockchain without timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L1State {
    pub block_number: i64,
    pub block_hash: Felt,
    pub root: Felt,
}

impl L1State {
    /// Create a new state with the given values
    pub fn new(block_number: i64, block_hash: Felt, root: Felt) -> Self {
        Self { block_number, block_hash, root }
    }
}

impl From<State> for L1State {
    fn from(state: State) -> Self {
        Self {
            block_number: state.block_number,
            block_hash: state.block_hash,
            root: state.root,
        }
    }
}
