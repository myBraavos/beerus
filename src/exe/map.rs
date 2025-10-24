use cairo_lang_starknet_classes::contract_class::ContractClass as CairoContractClass;
use starknet_api::{
    contract_class::ContractClass,
    deprecated_contract_class::ContractClass as DeprecatedContractClass,
};
use starknet_types_core::felt::Felt as StarkFelt;

use super::*;

/// Convert a single entry point from gen format to starknet_api format
fn convert_entry_point(
    ep: gen::DeprecatedCairoEntryPoint,
) -> Result<starknet_api::deprecated_contract_class::EntryPointV0, Error> {
    Ok(starknet_api::deprecated_contract_class::EntryPointV0 {
        selector: starknet_api::core::EntryPointSelector(ep.selector.try_into()?),
        offset: starknet_api::deprecated_contract_class::EntryPointOffset(
            ep.offset
                .as_ref()
                .parse::<usize>()
                .map_err(|e| Error::Program(format!("Invalid offset: {e}")))?
        ),
    })
}

/// Convert a list of entry points for a specific type
fn convert_entry_points(
    entry_points: Vec<gen::DeprecatedCairoEntryPoint>,
) -> Result<Vec<starknet_api::deprecated_contract_class::EntryPointV0>, Error> {
    entry_points
        .into_iter()
        .map(convert_entry_point)
        .collect()
}

/// Convert deprecated contract class from gen format to starknet_api format
fn convert_deprecated_contract_class(
    class: gen::DeprecatedContractClass,
) -> Result<DeprecatedContractClass, Error> {
    // Convert the program from base64 string to the expected format
    let program = decode_program(class.program.as_ref())?;

    // Convert entry points using the helper function
    let mut entry_points_by_type = std::collections::HashMap::new();

    if let Some(constructor) = class.entry_points_by_type.constructor {
        let converted = convert_entry_points(constructor)?;
        entry_points_by_type.insert(starknet_api::contract_class::EntryPointType::Constructor, converted);
    }

    if let Some(external) = class.entry_points_by_type.external {
        let converted = convert_entry_points(external)?;
        entry_points_by_type.insert(starknet_api::contract_class::EntryPointType::External, converted);
    }

    if let Some(l1_handler) = class.entry_points_by_type.l1_handler {
        let converted = convert_entry_points(l1_handler)?;
        entry_points_by_type.insert(starknet_api::contract_class::EntryPointType::L1Handler, converted);
    }

    // Convert the program
    let program: starknet_api::deprecated_contract_class::Program = serde_json::from_str(&program)?;

    Ok(DeprecatedContractClass {
        abi: None, // We'll skip ABI conversion for now
        program,
        entry_points_by_type,
    })
}

/// Decode and decompress a base64-encoded program
fn decode_program(program: &str) -> Result<String, Error> {
    let program = decode_base64(program)?;
    let program = decompress(&program)?;
    Ok(program)
}

/// Decode base64 string to bytes
fn decode_base64(input: &str) -> Result<Vec<u8>, Error> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    let result = BASE64.decode(input)?;
    Ok(result)
}

/// Decompress gzipped data
fn decompress(input: &[u8]) -> Result<String, Error> {
    use flate2::read::GzDecoder;
    use std::io::prelude::*;
    let mut gz = GzDecoder::new(input);
    let mut result = String::new();
    gz.read_to_string(&mut result)?;
    Ok(result)
}

/// Convert gen::Felt to StarkFelt
impl TryFrom<gen::Felt> for StarkFelt {
    type Error = Error;
    fn try_from(felt: gen::Felt) -> Result<Self, Self::Error> {
        let felt = felt.as_ref().as_str();
        let felt = StarkFelt::from_hex_unchecked(felt);
        Ok(felt)
    }
}

/// Convert StarkFelt to gen::Felt (by reference)
impl TryFrom<&StarkFelt> for gen::Felt {
    type Error = Error;
    fn try_from(felt: &StarkFelt) -> Result<Self, Self::Error> {
        let hex = hex::encode(felt.to_bytes_be());
        let hex = {
            // drop leading zeroes in order to match the regex
            let hex = hex.trim_start_matches('0');
            let hex = if hex.is_empty() { "0" } else { hex };
            format!("0x{hex}")
        };
        let felt = gen::Felt::try_new(&hex)?;
        Ok(felt)
    }
}

/// Convert StarkFelt to gen::Felt (by value)
impl TryFrom<StarkFelt> for gen::Felt {
    type Error = Error;
    fn try_from(felt: StarkFelt) -> Result<Self, Self::Error> {
        let felt = &felt;
        felt.try_into()
    }
}

/// Convert gen::GetClassResult to ContractClass
impl TryFrom<gen::GetClassResult> for ContractClass {
    type Error = Error;

    fn try_from(value: gen::GetClassResult) -> Result<Self, Self::Error> {
        Ok(match value {
            gen::GetClassResult::ContractClass(ref class) => {
                let mut json = serde_json::to_value(&value)?;
                if let Some(abi) = class.abi.as_ref() {
                    let abi: serde_json::Value = serde_json::to_value(abi)?;
                    json["abi"] = abi;
                }
                let contract_class: CairoContractClass =
                    serde_json::from_value(json)?;
                // Serialize and deserialize to handle version mismatch
                let casm_contract_class =
                    cairo_lang_starknet_classes::casm_contract_class::CasmContractClass::from_contract_class(
                        contract_class,
                        /*add_pythonic_hints=*/ false,
                        /*max_bytecode_size=*/ u16::MAX as usize,
                    )?;

                // Serialize to JSON and deserialize back to handle version mismatch
                let json = serde_json::to_string(&casm_contract_class)?;
                let casm_contract_class: starknet_api::contract_class::ContractClass = serde_json::from_str(&json)?;

                casm_contract_class
            }
            gen::GetClassResult::DeprecatedContractClass(class) => {
                let converted_class = convert_deprecated_contract_class(class)?;
                ContractClass::V0(converted_class)
            }
        })
    }
}
