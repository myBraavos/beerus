use crate::gen::Felt;
use eyre::Result;

/// Convert bytes to a Felt value, handling leading zeros according to RPC spec
///
/// The RPC spec requires that FELT values don't have leading zeros in their hex representation
pub fn as_felt(bytes: &[u8]) -> Result<Felt> {
    // RPC spec FELT regex: leading zeroes are not allowed
    let hex = hex::encode(bytes);
    let hex = hex.chars().skip_while(|c| c == &'0').collect::<String>();
    let hex = format!("0x{hex}");
    let felt = Felt::try_new(&hex)?;
    Ok(felt)
}
