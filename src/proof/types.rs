/// Constants used in proof verification
pub const MERKLE_HEIGHT: usize = 251;
pub const CONTRACT_STATE_HASH_VERSION: &str = "0x0";

/// Error codes for proof verification
pub const ERROR_CODE_PROOF_INVALID: i64 = -32700;
pub const ERROR_CODE_FIELD_ELEMENT: i64 = -32701;

/// Direction in the merkle tree
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
}

impl From<bool> for Direction {
    fn from(flag: bool) -> Self {
        if flag {
            Self::Right
        } else {
            Self::Left
        }
    }
}
