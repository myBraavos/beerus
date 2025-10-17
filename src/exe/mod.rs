use std::sync::Arc;

use blockifier::{
    bouncer::BouncerConfig,
    context::{BlockContext, ChainInfo, FeeTokenAddresses, TransactionContext},
    execution::{
        call_info::CallInfo,
        common_hints::ExecutionMode,
        contract_class::RunnableCompiledClass,
        entry_point::{
            CallEntryPoint, CallType, EntryPointExecutionContext,
            SierraGasRevertTracker,
        },
    },
    state::{
        errors::StateError,
        state_api::{State as BlockifierState, StateReader, StateResult},
    },
    transaction::objects::{
        CommonAccountFields, DeprecatedTransactionInfo, TransactionInfo,
    },
    versioned_constants::VersionedConstants,
};
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use starknet_api::{
    block::{
        BlockInfo, BlockNumber as StarknetBlockNumber, BlockTimestamp,
        GasPriceVector, GasPrices, NonzeroGasPrice,
    },
    contract_class::{ContractClass, EntryPointType},
    core::{
        ChainId as BlockifierChainId, ClassHash, CompiledClassHash,
        ContractAddress, EntryPointSelector, Nonce,
    },
    execution_resources::GasAmount,
    hash::StarkHash,
    state::StorageKey as StarknetStorageKey,
    transaction::{
        fields::{Calldata, Fee, TransactionSignature},
        TransactionHash, TransactionVersion,
    },
};
use starknet_types_core::felt::Felt as StarkFelt;

use crate::{
    client::State,
    gen::{self, blocking::Rpc},
};

pub mod cache;
pub mod err;
pub mod map;

use err::Error;

pub fn call<T: gen::client::blocking::HttpClient>(
    client: gen::client::blocking::Client<T>,
    function_call: gen::FunctionCall,
    state: State,
) -> Result<CallInfo, Error> {
    let gen::FunctionCall { calldata, contract_address, entry_point_selector } =
        function_call;

    let calldata: Result<Vec<StarkFelt>, _> =
        calldata.into_iter().map(|felt| felt.try_into()).collect();

    let contract_address: StarkFelt = contract_address.0.try_into()?;

    let entry_point_selector: StarkFelt = entry_point_selector.try_into()?;

    let block_info = BlockInfo {
        block_number: StarknetBlockNumber::default(),
        block_timestamp: BlockTimestamp::default(),
        sequencer_address: ContractAddress::default(),
        gas_prices: GasPrices {
            eth_gas_prices: GasPriceVector {
                l1_gas_price: NonzeroGasPrice::MIN,
                l1_data_gas_price: NonzeroGasPrice::MIN,
                l2_gas_price: NonzeroGasPrice::MIN,
            },
            strk_gas_prices: GasPriceVector {
                l1_gas_price: NonzeroGasPrice::MIN,
                l1_data_gas_price: NonzeroGasPrice::MIN,
                l2_gas_price: NonzeroGasPrice::MIN,
            },
        },
        use_kzg_da: false,
    };

    let chain_info = ChainInfo {
        chain_id: BlockifierChainId::Mainnet,
        fee_token_addresses: FeeTokenAddresses {
            strk_fee_token_address: ContractAddress::default(),
            eth_fee_token_address: ContractAddress::default(),
        },
    };

    let versioned_constants = VersionedConstants::latest_constants().to_owned();

    let bouncer_config = BouncerConfig::default();

    let block_context = BlockContext::new(
        block_info,
        chain_info,
        versioned_constants,
        bouncer_config,
    );

    let tx_info = TransactionInfo::Deprecated(DeprecatedTransactionInfo {
        common_fields: CommonAccountFields {
            transaction_hash: TransactionHash::default(),
            version: TransactionVersion(StarkFelt::ONE),
            signature: TransactionSignature(vec![
                StarkHash::ZERO,
                StarkHash::ZERO,
            ]),
            nonce: Nonce(StarkHash::ZERO),
            sender_address: ContractAddress::default(),
            only_query: true,
        },
        max_fee: Fee::default(),
    });

    let tx_context = Arc::new(TransactionContext { block_context, tx_info });
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

    let state_proxy: StateProxy<T> = StateProxy { client, state };

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

        let felt: gen::Felt = contract_address.0.key().try_into()?;
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
            return Ok(ret.try_into()?);
        }

        let proof = self
            .client
            .getProof(block_id, address.clone(), vec![key.clone()])
            .map_err(Into::<Error>::into)?;
        tracing::info!("get_storage_at: proof received");

        let global_root = self.state.root.clone();
        let value = ret.clone();
        proof.verify(global_root, address, key, value).map_err(|e| {
            StateError::StateReadError(format!(
                "Failed to verify merkle proof: {e:?}"
            ))
        })?;
        tracing::info!("get_storage_at: proof verified");

        Ok(ret.try_into()?)
    }

    fn get_nonce_at(
        &self,
        contract_address: ContractAddress,
    ) -> StateResult<Nonce> {
        tracing::info!(?contract_address, "get_nonce_at");

        let block_id = gen::BlockId::BlockHash {
            block_hash: gen::BlockHash(self.state.block_hash.clone()),
        };

        let felt: gen::Felt = contract_address.0.key().try_into()?;
        let contract_address = gen::Address(felt);

        let ret = self
            .client
            .getNonce(block_id, contract_address)
            .map_err(Into::<Error>::into)?;

        Ok(Nonce(ret.try_into()?))
    }

    fn get_class_hash_at(
        &self,
        contract_address: ContractAddress,
    ) -> StateResult<ClassHash> {
        tracing::info!(?contract_address, "get_class_hash_at");

        let block_id = gen::BlockId::BlockHash {
            block_hash: gen::BlockHash(self.state.block_hash.clone()),
        };

        let felt: gen::Felt = contract_address.0.key().try_into()?;
        let contract_address = gen::Address(felt);

        let ret = self
            .client
            .getClassHashAt(block_id, contract_address)
            .map_err(Into::<Error>::into)?;

        Ok(ClassHash(ret.try_into()?))
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

        let class_hash: gen::Felt = class_hash.0.try_into()?;

        let ret = self
            .client
            .getClass(block_id, class_hash)
            .map_err(Into::<Error>::into)?;

        // Convert to blockifier's ContractClass via explicit variant conversion
        let contract_class = match ret {
            gen::GetClassResult::ContractClass(contract_class) => {
                let sierra_version = contract_class.contract_class_version.parse()
                    .map_err(|_| Error::Custom("Failed to parse SierraVersion"))?;
                let casm_class = CasmContractClass::from_contract_class(contract_class.into(), true, u32::MAX as usize)
                    .map_err(|_| Error::Custom("Failed to convert Sierra program"))?;
                ContractClass::V1((casm_class, sierra_version))
            }
            // TODO: add cairo 0 support
            //     deprecated_contract_class
            //     // let deprecated: blockifier::execution::contract_class::ContractClassV0 =
            //     //     deprecated_contract_class.try_into().map_err(|_| Error::Custom("Failed to convert DeprecatedContractClass"))?;
            //     // ContractClass::V0(deprecated_contract_class)
            // }
            _ => return Err(Error::Custom("Failed to convert DeprecatedContractClass").into()),
        };
        let runnable_compiled_class =
            RunnableCompiledClass::try_from(contract_class)?;

        Ok(runnable_compiled_class)
    }

    fn get_compiled_class_hash(
        &self,
        class_hash: ClassHash,
    ) -> StateResult<CompiledClassHash> {
        tracing::info!(?class_hash, "get_compiled_class_hash");
        Err(StateError::UndeclaredClassHash(class_hash))
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
        _contract_class: blockifier::execution::contract_class::RunnableCompiledClass,
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
