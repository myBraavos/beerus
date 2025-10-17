use cairo_lang_starknet_classes::contract_class::ContractClass as CairoContractClass;
use serde_json::Value;

use crate::gen::ContractClass as GenContractClass;

/// Trait for converting from generated types to Cairo types
pub trait ToCairo {
    type Output;
    type Error;

    fn to_cairo(self) -> Result<Self::Output, Self::Error>;
}

impl ToCairo for GenContractClass {
    type Output = CairoContractClass;
    type Error = ConversionError;

    fn to_cairo(self) -> Result<Self::Output, Self::Error> {
        // Convert to JSON first, then deserialize to CairoContractClass
        // This approach avoids dependency issues and handles type conversions automatically
        let mut json = serde_json::to_value(&self)?;

        // Handle ABI conversion if present
        if let Some(abi_str) = self.abi {
            let abi: Value = serde_json::from_str(&abi_str)?;
            json["abi"] = abi;
        }

        // Deserialize to CairoContractClass
        let contract_class: CairoContractClass = serde_json::from_value(json)?;

        Ok(contract_class)
    }
}

// Implement From trait for compatibility with existing code
impl From<GenContractClass> for CairoContractClass {
    fn from(gen_class: GenContractClass) -> Self {
        gen_class.to_cairo().expect("Failed to convert ContractClass")
    }
}

/// Errors that can occur during conversion
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
