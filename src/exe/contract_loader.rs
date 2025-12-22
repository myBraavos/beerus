use std::collections::HashMap;
use std::io::prelude::*;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use flate2::read::GzDecoder;
use starknet_api::contract_class::ContractClass;

use crate::exe::err::Error;
use crate::gen;

/// Handles loading and conversion of contract classes
pub struct ContractLoader;

impl ContractLoader {
    /// Load and convert a contract class from the RPC result
    pub fn load_contract_class(
        class_result: gen::GetClassResult,
    ) -> Result<
        blockifier::execution::contract_class::RunnableCompiledClass,
        Error,
    > {
        let contract_class = Self::convert_to_contract_class(class_result)?;
        let runnable_compiled_class =
            blockifier::execution::contract_class::RunnableCompiledClass::try_from(contract_class)
                .map_err(|_| Error::Custom("Failed to convert to RunnableCompiledClass"))?;
        Ok(runnable_compiled_class)
    }

    /// Convert RPC GetClassResult to blockifier's ContractClass
    fn convert_to_contract_class(
        class_result: gen::GetClassResult,
    ) -> Result<ContractClass, Error> {
        match class_result {
            gen::GetClassResult::ContractClass(contract_class) => {
                let sierra_version =
                    contract_class.contract_class_version.parse().map_err(
                        |_| Error::Custom("Failed to parse SierraVersion"),
                    )?;
                let casm_class = CasmContractClass::from_contract_class(
                    contract_class.try_into()?,
                    true,
                    u32::MAX as usize,
                )
                .map_err(|_| {
                    Error::Custom("Failed to convert Sierra program")
                })?;
                Ok(ContractClass::V1((casm_class, sierra_version)))
            }
            gen::GetClassResult::DeprecatedContractClass(
                deprecated_contract_class,
            ) => {
                let deprecated: starknet_api::deprecated_contract_class::ContractClass =
                    deprecated_contract_class.try_into()?;
                Ok(ContractClass::V0(deprecated))
            }
        }
    }
}

/// Convert a single entry point from gen format to starknet_api format
fn convert_entry_point(
    ep: gen::DeprecatedCairoEntryPoint,
) -> Result<starknet_api::deprecated_contract_class::EntryPointV0, Error> {
    Ok(starknet_api::deprecated_contract_class::EntryPointV0 {
        selector: starknet_api::core::EntryPointSelector(
            ep.selector.try_into()?,
        ),
        offset: starknet_api::deprecated_contract_class::EntryPointOffset(
            usize::from_str_radix(
                ep.offset.as_ref().trim_start_matches("0x"),
                16,
            )
            .map_err(|e| Error::Program(format!("Invalid offset: {e}")))?,
        ),
    })
}

/// Convert a list of entry points for a specific type
fn convert_entry_points(
    entry_points: Vec<gen::DeprecatedCairoEntryPoint>,
) -> Result<Vec<starknet_api::deprecated_contract_class::EntryPointV0>, Error> {
    entry_points.into_iter().map(convert_entry_point).collect()
}

/// Decode and decompress a base64-encoded program
fn decode_program(program: &str) -> Result<String, Error> {
    let decoded = BASE64.decode(program)?;
    let mut gz = GzDecoder::new(&decoded[..]);
    let mut result = String::new();
    gz.read_to_string(&mut result)?;
    Ok(result)
}

impl TryFrom<gen::DeprecatedContractClass>
    for starknet_api::deprecated_contract_class::ContractClass
{
    type Error = Error;

    fn try_from(
        class: gen::DeprecatedContractClass,
    ) -> Result<Self, Self::Error> {
        // Convert the program from base64 string to the expected format
        let program_json = decode_program(class.program.as_ref())?;
        let program: starknet_api::deprecated_contract_class::Program =
            serde_json::from_str(&program_json)?;

        // Convert entry points using the helper function
        let mut entry_points_by_type = HashMap::new();

        if let Some(constructor) = class.entry_points_by_type.constructor {
            let converted = convert_entry_points(constructor)?;
            entry_points_by_type.insert(
                starknet_api::contract_class::EntryPointType::Constructor,
                converted,
            );
        }

        if let Some(external) = class.entry_points_by_type.external {
            let converted = convert_entry_points(external)?;
            entry_points_by_type.insert(
                starknet_api::contract_class::EntryPointType::External,
                converted,
            );
        }

        if let Some(l1_handler) = class.entry_points_by_type.l1_handler {
            let converted = convert_entry_points(l1_handler)?;
            entry_points_by_type.insert(
                starknet_api::contract_class::EntryPointType::L1Handler,
                converted,
            );
        }

        // Convert ABI if present
        let abi = if let Some(abi) = class.abi {
            // Convert gen::ContractAbiEntry to starknet_api::ContractClassAbiEntry
            let converted_abi: Result<Vec<_>, _> = abi
                .into_iter()
                .map(|entry| {
                    let json = serde_json::to_value(&entry)?;
                    serde_json::from_value(json).map_err(Error::Serde)
                })
                .collect();
            Some(converted_abi?)
        } else {
            None
        };

        Ok(starknet_api::deprecated_contract_class::ContractClass {
            abi,
            program,
            entry_points_by_type,
        })
    }
}
