use blockifier::state::state_api::{
    State as BlockifierState, StateReader, StateResult,
};
use starknet_api::{
    core::{ClassHash, CompiledClassHash, ContractAddress, Nonce},
    state::StorageKey as StarknetStorageKey,
};
use starknet_types_core::felt::Felt as StarkFelt;

use crate::{
    client::State,
    exe::{cache, err::Error},
    gen::{self, blocking::Rpc},
};

/// State proxy that implements the blockifier state interface
pub struct StateProxy<T: gen::client::blocking::HttpClient> {
    pub client: gen::client::blocking::Client<T>,
    pub state: State,
}

impl<T: gen::client::blocking::HttpClient> cache::HasBlockHash
    for StateProxy<T>
{
    fn get_block_hash(&self) -> &gen::Felt {
        &self.state.block_hash
    }
}

impl<T: gen::client::blocking::HttpClient> StateReader for StateProxy<T> {
    fn get_storage_at(
        &self,
        contract_address: ContractAddress,
        storage_key: StarknetStorageKey,
    ) -> StateResult<StarkFelt> {
        tracing::debug!(?contract_address, ?storage_key, "get_storage_at");

        let felt: gen::Felt = gen::Felt::try_from(contract_address.0.key())?;
        let address = gen::Address(felt);

        let key = gen::StorageKey::try_new(&storage_key.0.to_string())
            .map_err(Into::<Error>::into)?;

        let block_id = gen::BlockId::BlockHash {
            block_hash: gen::BlockHash(self.state.block_hash.clone()),
        };

        let ret = self
            .client
            .getStorageAt(address.clone(), key.clone(), block_id.clone())
            .map_err(Into::<Error>::into)?;
        tracing::debug!(?address, ?key, value=?ret, "get_storage_at");

        if ret.as_ref() == "0x0" {
            tracing::debug!("get_storage_at: skipping proof for zero value");
            return Ok(StarkFelt::try_from(ret)?);
        }

        let proof = self
            .client
            .getProof(block_id, address.clone(), vec![key.clone()])
            .map_err(Into::<Error>::into)?;
        tracing::debug!("get_storage_at: proof received");

        let global_root = self.state.root.clone();
        let value = ret.clone();
        crate::proof::verify_proof(&proof, global_root, address, key, value)
            .map_err(|e| {
                blockifier::state::errors::StateError::StateReadError(format!(
                    "Failed to verify merkle proof: {e:?}"
                ))
            })?;
        tracing::debug!("get_storage_at: proof verified");

        Ok(StarkFelt::try_from(ret)?)
    }

    fn get_nonce_at(
        &self,
        contract_address: ContractAddress,
    ) -> StateResult<Nonce> {
        tracing::debug!(?contract_address, "get_nonce_at");

        let block_id = gen::BlockId::BlockHash {
            block_hash: gen::BlockHash(self.state.block_hash.clone()),
        };

        let felt: gen::Felt = gen::Felt::try_from(contract_address.0.key())?;
        let contract_address = gen::Address(felt);

        let ret = self
            .client
            .getNonce(block_id, contract_address)
            .map_err(Into::<Error>::into)?;

        Ok(Nonce(StarkFelt::try_from(ret)?))
    }

    fn get_class_hash_at(
        &self,
        contract_address: ContractAddress,
    ) -> StateResult<ClassHash> {
        tracing::debug!(?contract_address, "get_class_hash_at");

        let block_id = gen::BlockId::BlockHash {
            block_hash: gen::BlockHash(self.state.block_hash.clone()),
        };

        let felt: gen::Felt = gen::Felt::try_from(contract_address.0.key())?;
        let contract_address = gen::Address(felt);

        let ret = self
            .client
            .getClassHashAt(block_id, contract_address)
            .map_err(Into::<Error>::into)?;

        Ok(ClassHash(StarkFelt::try_from(ret)?))
    }

    fn get_compiled_class(
        &self,
        class_hash: ClassHash,
    ) -> Result<
        blockifier::execution::contract_class::RunnableCompiledClass,
        blockifier::state::errors::StateError,
    > {
        tracing::debug!(?class_hash, "get_compiled_class");

        let block_id = gen::BlockId::BlockHash {
            block_hash: gen::BlockHash(self.state.block_hash.clone()),
        };

        let class_hash: gen::Felt = gen::Felt::try_from(&class_hash.0)?;

        let ret = self
            .client
            .getClass(block_id, class_hash)
            .map_err(Into::<Error>::into)?;

        // Convert to blockifier's ContractClass via explicit variant conversion
        let contract_class = match ret {
            gen::GetClassResult::ContractClass(contract_class) => {
                let sierra_version =
                    contract_class.contract_class_version.parse().map_err(
                        |_| Error::Custom("Failed to parse SierraVersion"),
                    )?;
                let casm_class = cairo_lang_starknet_classes::casm_contract_class::CasmContractClass::from_contract_class(contract_class.try_into()?, true, u32::MAX as usize)
                    .map_err(|_| Error::Custom("Failed to convert Sierra program"))?;
                starknet_api::contract_class::ContractClass::V1((
                    casm_class,
                    sierra_version,
                ))
            }
            gen::GetClassResult::DeprecatedContractClass(
                deprecated_contract_class,
            ) => {
                let deprecated: starknet_api::deprecated_contract_class::ContractClass =
                    deprecated_contract_class.try_into()?;
                starknet_api::contract_class::ContractClass::V0(deprecated)
            }
        };
        let runnable_compiled_class =
            blockifier::execution::contract_class::RunnableCompiledClass::try_from(contract_class)?;

        Ok(runnable_compiled_class)
    }

    fn get_compiled_class_hash(
        &self,
        class_hash: ClassHash,
    ) -> StateResult<CompiledClassHash> {
        tracing::debug!(?class_hash, "get_compiled_class_hash");
        Err(blockifier::state::errors::StateError::UndeclaredClassHash(
            class_hash,
        ))
    }
}

impl<T: gen::client::blocking::HttpClient> BlockifierState for StateProxy<T> {
    fn set_storage_at(
        &mut self,
        contract_address: ContractAddress,
        key: StarknetStorageKey,
        value: StarkFelt,
    ) -> StateResult<()> {
        tracing::debug!(?contract_address, ?key, ?value, "set_storage_at");
        Ok(())
    }

    fn increment_nonce(
        &mut self,
        contract_address: ContractAddress,
    ) -> StateResult<()> {
        tracing::debug!(?contract_address, "increment_nonce");
        Ok(())
    }

    fn set_class_hash_at(
        &mut self,
        contract_address: ContractAddress,
        class_hash: ClassHash,
    ) -> StateResult<()> {
        tracing::debug!(?contract_address, ?class_hash, "set_class_hash_at");
        Ok(())
    }

    fn set_contract_class(
        &mut self,
        class_hash: ClassHash,
        _contract_class: blockifier::execution::contract_class::RunnableCompiledClass,
    ) -> StateResult<()> {
        tracing::debug!(?class_hash, "set_contract_class");
        Ok(())
    }

    fn set_compiled_class_hash(
        &mut self,
        class_hash: ClassHash,
        compiled_class_hash: CompiledClassHash,
    ) -> StateResult<()> {
        tracing::debug!(
            ?class_hash,
            ?compiled_class_hash,
            "set_compiled_class_hash"
        );
        Ok(())
    }
}
