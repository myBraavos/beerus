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
    block::{BlockNumber, BlockTimestamp, GasPrices},
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
use std::sync::RwLock;

use crate::{
    client::{rate_limiter::RateLimiter, settings::Settings, State},
    exe::{
        cache,
        contract_loader::ContractLoader,
        err::Error,
        simulate::{simulate_transactions, TransactionSimulationOutput},
        utils::transform_trace_json,
    },
    gen::{self, blocking::Rpc},
};

use super::context::create_query_context;

fn wait_rate_limiter(rate_limiter: &RateLimiter) {
    // Try to get the current runtime handle
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        // We're in a runtime context. Check if we can use block_in_place
        // (which only works in multi-threaded runtimes)
        if handle.runtime_flavor()
            != tokio::runtime::RuntimeFlavor::CurrentThread
        {
            // Multi-threaded runtime: use block_in_place (safe here)
            tokio::task::block_in_place(|| {
                handle.block_on(rate_limiter.wait())
            });
        }
    }
}

fn should_verify_storage_sync(
    settings: &Arc<RwLock<Settings>>,
    contract_address: &str,
) -> bool {
    // Use blocking read lock - this works in both sync and async contexts
    // Settings reads are fast, so blocking is acceptable
    match settings.read() {
        Ok(guard) => guard.should_verify_storage(&contract_address.to_string()),
        Err(_) => {
            // Lock is poisoned (another thread panicked while holding the lock)
            // Return a safe default: verify storage if we can't read settings
            tracing::warn!("Settings lock is poisoned");
            true
        }
    }
}

/// Executes function calls on the Starknet state
pub struct CallExecutor<T: gen::client::blocking::HttpClient> {
    client: gen::client::blocking::Client<T>,
    state: State,
    rate_limiter: RateLimiter,
    settings: Arc<RwLock<Settings>>,
}

impl<T: gen::client::blocking::HttpClient + Clone> CallExecutor<T> {
    /// Create a new call executor
    pub fn new(
        client: gen::client::blocking::Client<T>,
        state: State,
        rate_limiter: RateLimiter,
        settings: Arc<RwLock<Settings>>,
    ) -> Self {
        Self { client, state, rate_limiter, settings }
    }

    /// Execute a function call
    pub fn call(
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

        let tx_context = Arc::new(create_query_context(
            self.state.block_number as u64,
            self.state.timestamp as u64,
        )?);

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
            rate_limiter: self.rate_limiter.clone(),
            settings: self.settings.clone(),
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

    pub fn simulate(
        &self,
        transactions: Vec<gen::BroadcastedTxn>,
        simulation_flags: Vec<gen::SimulationFlag>,
        gas_prices: &GasPrices,
    ) -> Result<Vec<gen::SimulatedTransaction>, Error> {
        let res =
            self.do_simulate(transactions, simulation_flags, gas_prices)?;
        // Convert TransactionSimulationOutput to gen::SimulatedTransaction
        // Use serde_json to convert, handling the enum structure properly
        let converted: Vec<gen::SimulatedTransaction> = res
            .into_iter()
            .map(|output| {
                // Convert transaction_trace by serializing and transforming the structure
                let trace_json = serde_json::to_value(&output.transaction_trace)
                    .map_err(|e| Error::IamGroot(iamgroot::jsonrpc::Error::new(
                        32101,
                        format!("Failed to serialize transaction trace: {e:?}"),
                    )))?;

                tracing::debug!("Original trace JSON: {}", serde_json::to_string(&trace_json).unwrap_or_default());

                let state_diff_json = serde_json::to_value(&output.induced_state_diff)
                    .map_err(|e| Error::IamGroot(iamgroot::jsonrpc::Error::new(
                        32101,
                        format!("Failed to serialize transaction trace: {e:?}"),
                    )))?;

                // Transform the JSON to match gen's expected structure
                // The apollo structure has execution_resources with a different format
                let transformed_json = transform_trace_json(trace_json, state_diff_json)?;

                tracing::debug!("Transformed trace JSON: {}", serde_json::to_string(&transformed_json).unwrap_or_default());

                let transaction_trace: gen::TransactionTrace = serde_json::from_value(transformed_json.clone())
                    .map_err(|e| {
                        tracing::error!("Failed to deserialize trace JSON: {e:?}");
                        tracing::error!("JSON was: {}", serde_json::to_string(&transformed_json).unwrap_or_default());
                        Error::IamGroot(iamgroot::jsonrpc::Error::new(
                            32101,
                            format!("Failed to convert transaction trace: {e:?}"),
                        ))
                    })?;

                // Convert fee_estimation using serde_json
                let fee_estimation = serde_json::to_value(&output.fee_estimation)
                    .and_then(serde_json::from_value)
                    .map_err(|e| Error::IamGroot(iamgroot::jsonrpc::Error::new(
                        32101,
                        format!("Failed to convert fee estimation: {e:?}"),
                    )))?;

                Ok(gen::SimulatedTransaction {
                    fee_estimation: Some(fee_estimation),
                    transaction_trace: Some(transaction_trace),
                })
            })
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(converted)
    }

    pub fn estimate_fee(
        &self,
        transactions: Vec<gen::BroadcastedTxn>,
        simulation_flags: Vec<gen::SimulationFlag>,
        gas_prices: &GasPrices,
    ) -> Result<Vec<gen::FeeEstimate>, Error> {
        let res =
            self.do_simulate(transactions, simulation_flags, gas_prices)?;
        let fee_estimates: Vec<gen::FeeEstimate> = res
            .into_iter()
            .map(|output| {
                serde_json::to_value(output.fee_estimation)
                    .map_err(|e| Error::IamGroot(iamgroot::jsonrpc::Error::new(
                        32101,
                        format!("Failed to serialize fee estimation: {e:?}"),
                    )))
                    .and_then(|value| {
                        serde_json::from_value(value).map_err(|e| Error::IamGroot(iamgroot::jsonrpc::Error::new(
                            32101,
                            format!("Failed to deserialize fee estimation: {e:?}"),
                        )))
                    })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(fee_estimates)
    }

    fn do_simulate(
        &self,
        transactions: Vec<gen::BroadcastedTxn>,
        simulation_flags: Vec<gen::SimulationFlag>,
        gas_prices: &GasPrices,
    ) -> Result<Vec<TransactionSimulationOutput>, Error> {
        let mut charge_fee = true;
        let mut validate = true;
        for flag in simulation_flags {
            match flag {
                gen::SimulationFlag::SkipFeeCharge => charge_fee = false,
                gen::SimulationFlag::SkipValidate => validate = false,
            }
        }

        let state_proxy: StateProxy<T> = StateProxy {
            client: self.client.clone(),
            state: self.state.clone(),
            rate_limiter: self.rate_limiter.clone(),
            settings: self.settings.clone(),
        };
        let state_proxy = cache::CachedState::new(state_proxy);

        // Convert BroadcastedTxn to ExecutableTransactionInput
        let executable_transactions: Vec<
            apollo_rpc_execution::ExecutableTransactionInput,
        > = transactions
            .into_iter()
            .map(|tx| tx.try_into())
            .collect::<Result<Vec<_>, _>>()?;

        let res = simulate_transactions(
            executable_transactions,
            &starknet_api::core::ChainId::Mainnet,
            state_proxy,
            gas_prices,
            BlockNumber(self.state.block_number as u64),
            BlockTimestamp::from(self.state.timestamp as u64),
            charge_fee,
            validate,
        )
        .map_err(Error::from)?;

        tracing::debug!("Simulation result: {:?}", res);
        Ok(res)
    }
}

/// State proxy that implements the blockifier state interface
struct StateProxy<T: gen::client::blocking::HttpClient> {
    client: gen::client::blocking::Client<T>,
    state: State,
    rate_limiter: RateLimiter,
    settings: Arc<RwLock<Settings>>,
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

        wait_rate_limiter(&self.rate_limiter);
        let ret = self
            .client
            .getStorageAt(address.clone(), key.clone(), block_id.clone())
            .map_err(Into::<Error>::into)?;
        tracing::debug!(?address, ?key, value=?ret, "get_storage_at");

        if ret.as_ref() == "0x0" {
            tracing::debug!("get_storage_at: skipping proof for zero value");
            return Ok(StarkFelt::try_from(ret)?);
        }

        if should_verify_storage_sync(&self.settings, &address.0.to_string()) {
            wait_rate_limiter(&self.rate_limiter);
            let proof = self
                .client
                .getProof(block_id, address.clone(), vec![key.clone()])
                .map_err(Into::<Error>::into)?;
            tracing::debug!("get_storage_at: proof received");

            let global_root = self.state.root.clone();
            let value = ret.clone();
            crate::proof::verify_proof(
                &proof,
                global_root,
                address,
                key,
                value,
            )
            .map_err(|e| {
                blockifier::state::errors::StateError::StateReadError(format!(
                    "Failed to verify merkle proof: {e:?}"
                ))
            })?;
            tracing::debug!("get_storage_at: proof verified");
        } else {
            tracing::debug!(
                "skipping storage verification for contract {}",
                &address.0.to_string()
            );
        }

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

        wait_rate_limiter(&self.rate_limiter);
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

        wait_rate_limiter(&self.rate_limiter);
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
        tracing::debug!(?class_hash, "get_compiled_class");

        let block_id = gen::BlockId::BlockHash {
            block_hash: gen::BlockHash(self.state.block_hash.clone()),
        };

        let class_hash: gen::Felt = gen::Felt::try_from(&class_hash.0)?;

        wait_rate_limiter(&self.rate_limiter);
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
        _contract_class: RunnableCompiledClass,
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
