use std::sync::Arc;

use blockifier::{
    execution::{
        call_info::CallInfo,
        common_hints::ExecutionMode,
        contract_class::RunnableCompiledClass,
        entry_point::{
            CallEntryPoint, CallType, EntryPointExecutionContext,
            SierraGasRevertTracker,
        },
    },
    state::state_api::{State as BlockifierState, StateReader, StateResult},
};
use starknet_api::{
    contract_class::EntryPointType,
    core::{
        ClassHash, CompiledClassHash, ContractAddress, EntryPointSelector,
        Nonce,
    },
    execution_resources::GasAmount,
    state::StorageKey as StarknetStorageKey,
    transaction::fields::Calldata,
};
use starknet_types_core::felt::Felt as StarkFelt;

use crate::{
    client::State,
    exe::{cache, contract_loader::ContractLoader, err::Error},
    gen::{self, blocking::Rpc},
};

use super::context::create_query_context;

/// Executes function calls on the Starknet state
pub struct CallExecutor<T: gen::client::blocking::HttpClient> {
    client: gen::client::blocking::Client<T>,
    state: State,
}

impl<T: gen::client::blocking::HttpClient + Clone> CallExecutor<T> {
    /// Create a new call executor
    pub fn new(client: gen::client::blocking::Client<T>, state: State) -> Self {
        Self { client, state }
    }

    /// Execute a function call
    pub fn execute(
        &self,
        function_call: gen::FunctionCall,
    ) -> Result<CallInfo, Error> {
        let gen::FunctionCall {
            calldata,
            contract_address,
            entry_point_selector,
        } = function_call;

        let calldata: Result<Vec<StarkFelt>, _> =
            calldata.into_iter().map(StarkFelt::try_from).collect();

        let contract_address: StarkFelt =
            StarkFelt::try_from(contract_address.0)?;
        let entry_point_selector: StarkFelt =
            StarkFelt::try_from(entry_point_selector)?;

        let tx_context = Arc::new(create_query_context()?);
        let limit_steps_by_resources = false;
        let mut context = EntryPointExecutionContext::new(
            tx_context.clone(),
            ExecutionMode::Execute,
            limit_steps_by_resources,
            SierraGasRevertTracker::new(GasAmount::MAX),
        );

        let call_entry_point = CallEntryPoint {
            class_hash: None,
            code_address: None,
            entry_point_type: EntryPointType::External,
            entry_point_selector: EntryPointSelector(entry_point_selector),
            calldata: Calldata(Arc::new(calldata?)),
            storage_address: ContractAddress(contract_address.try_into()?),
            caller_address: ContractAddress::default(),
            call_type: CallType::Call,
            initial_gas: u64::MAX,
        };

        let state_proxy: StateProxy<T> = StateProxy {
            client: self.client.clone(),
            state: self.state.clone(),
        };

        tracing::debug!("State information:");
        tracing::debug!("  Block number: {}", state_proxy.state.block_number);
        tracing::debug!("  Block hash: {:?}", state_proxy.state.block_hash);
        tracing::debug!("  Root: {:?}", state_proxy.state.root);

        let mut state_proxy = cache::CachedState::new(state_proxy);

        let mut resources = Default::default();
        let call_info = call_entry_point.execute(
            &mut state_proxy,
            &mut context,
            &mut resources,
        )?;

        tracing::debug!(?call_info, "call completed");
        Ok(call_info)
    }
}

/// State proxy that implements the blockifier state interface
struct StateProxy<T: gen::client::blocking::HttpClient> {
    client: gen::client::blocking::Client<T>,
    state: State,
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
        tracing::info!(?contract_address, ?storage_key, "get_storage_at");

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
        tracing::info!(?address, ?key, value=?ret, "get_storage_at");

        if ret.as_ref() == "0x0" {
            tracing::info!("get_storage_at: skipping proof for zero value");
            return Ok(StarkFelt::try_from(ret)?);
        }

        let proof = self
            .client
            .getProof(block_id, address.clone(), vec![key.clone()])
            .map_err(Into::<Error>::into)?;
        tracing::info!("get_storage_at: proof received");

        let global_root = self.state.root.clone();
        let value = ret.clone();
        crate::proof::verify_proof(&proof, global_root, address, key, value)
            .map_err(|e| {
                blockifier::state::errors::StateError::StateReadError(format!(
                    "Failed to verify merkle proof: {e:?}"
                ))
            })?;
        tracing::info!("get_storage_at: proof verified");

        Ok(StarkFelt::try_from(ret)?)
    }

    fn get_nonce_at(
        &self,
        contract_address: ContractAddress,
    ) -> StateResult<Nonce> {
        tracing::info!(?contract_address, "get_nonce_at");

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
        tracing::info!(?contract_address, "get_class_hash_at");

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
    ) -> Result<RunnableCompiledClass, blockifier::state::errors::StateError>
    {
        tracing::info!(?class_hash, "get_compiled_class");

        let block_id = gen::BlockId::BlockHash {
            block_hash: gen::BlockHash(self.state.block_hash.clone()),
        };

        let class_hash: gen::Felt = gen::Felt::try_from(&class_hash.0)?;

        let ret = self
            .client
            .getClass(block_id, class_hash)
            .map_err(Into::<Error>::into)?;

        ContractLoader::load_contract_class(ret).map_err(Into::into)
    }

    fn get_compiled_class_hash(
        &self,
        class_hash: ClassHash,
    ) -> StateResult<CompiledClassHash> {
        tracing::info!(?class_hash, "get_compiled_class_hash");
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
        tracing::info!(?contract_address, ?key, ?value, "set_storage_at");
        Ok(())
    }

    fn increment_nonce(
        &mut self,
        contract_address: ContractAddress,
    ) -> StateResult<()> {
        tracing::info!(?contract_address, "increment_nonce");
        Ok(())
    }

    fn set_class_hash_at(
        &mut self,
        contract_address: ContractAddress,
        class_hash: ClassHash,
    ) -> StateResult<()> {
        tracing::info!(?contract_address, ?class_hash, "set_class_hash_at");
        Ok(())
    }

    fn set_contract_class(
        &mut self,
        class_hash: ClassHash,
        _contract_class: RunnableCompiledClass,
    ) -> StateResult<()> {
        tracing::info!(?class_hash, "set_contract_class");
        Ok(())
    }

    fn set_compiled_class_hash(
        &mut self,
        class_hash: ClassHash,
        compiled_class_hash: CompiledClassHash,
    ) -> StateResult<()> {
        tracing::info!(
            ?class_hash,
            ?compiled_class_hash,
            "set_compiled_class_hash"
        );
        Ok(())
    }
}
