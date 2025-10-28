use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use starknet_api::contract_class::ContractClass;

use crate::exe::err::Error;
use crate::gen;

/// Handles loading and conversion of contract classes
pub struct ContractLoader;

impl ContractLoader {
    /// Load and convert a contract class from the RPC result
    pub fn load_contract_class(
        class_result: gen::GetClassResult,
    ) -> Result<blockifier::execution::contract_class::RunnableCompiledClass, Error> {
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
                let sierra_version = contract_class.contract_class_version.parse()
                    .map_err(|_| Error::Custom("Failed to parse SierraVersion"))?;
                let casm_class = CasmContractClass::from_contract_class(
                    contract_class.into(),
                    true,
                    u32::MAX as usize
                )
                .map_err(|_| Error::Custom("Failed to convert Sierra program"))?;
                Ok(ContractClass::V1((casm_class, sierra_version)))
            }
            // TODO: add cairo 0 support
            //     deprecated_contract_class
            //     // let deprecated: blockifier::execution::contract_class::ContractClassV0 =
            //     //     deprecated_contract_class.try_into().map_err(|_| Error::Custom("Failed to convert DeprecatedContractClass"))?;
            //     // ContractClass::V0(deprecated_contract_class)
            // }
            _ => Err(Error::Custom("Failed to convert DeprecatedContractClass")),
        }
    }
}
