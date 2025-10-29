use crate::r#gen::Felt;
use alloy_primitives::Uint;

impl TryFrom<Uint<256, 4>> for Felt {
    type Error = iamgroot::jsonrpc::Error;
    fn try_from(value: Uint<256, 4>) -> Result<Self, Self::Error> {
        Felt::try_new(&format!("{:#x}", value))
    }
}
