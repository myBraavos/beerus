use crate::exe::cache::{CachedState as ProxyCachedState, HasBlockHash};
use apollo_rpc_execution::{
    execution_utils::get_trace_constructor,
    objects::{PriceUnit, TransactionTrace},
    ExecutableTransactionInput, ExecutionError, ExecutionResult,
    DEPRECATED_CONTRACT_SIERRA_SIZE,
};
use blockifier::{
    blockifier_versioned_constants::VersionedConstants,
    bouncer::BouncerConfig,
    context::{ChainInfo, FeeTokenAddresses},
    state::{
        cached_state::{CommitmentStateDiff, MutRefState},
        state_api::{State as BlockifierState, StateReader},
    },
    transaction::transactions::ExecutableTransaction,
};
use blockifier::{
    context::BlockContext,
    transaction::{
        account_transaction::ExecutionFlags, objects::TransactionExecutionInfo,
    },
};
use blockifier::{
    state::cached_state::CachedState,
    transaction::transaction_execution::Transaction as BlockifierTransaction,
};
use serde::{Deserialize, Serialize};
use starknet_api::{
    block::{
        BlockInfo, BlockNumber, BlockTimestamp, GasPrice, GasPrices,
        StarknetVersion,
    },
    contract_class::{ClassInfo, SierraVersion},
    core::{ChainId, ClassHash, ContractAddress},
    hash::StarkHash,
    state::ThinStateDiff,
    transaction::{
        fields::Fee, DeclareTransaction, DeclareTransactionV0V1, Transaction,
        TransactionHash, TransactionOptions,
    },
    transaction_hash::get_transaction_hash,
};
use starknet_crypto::Felt;

const STRK_CONTRACT_ADDRESS: &str =
    "0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d";
const ETH_CONTRACT_ADDRESS: &str =
    "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7";
const SEQUENCER_ADDRESS: &str =
    "0x1176a1bd84444c89232ec27754698e5d2e7e1a7f1539f12027f28b23ec9f3d8";

struct TransactionExecutionOutput {
    execution_info: TransactionExecutionInfo,
    induced_state_diff: ThinStateDiff,
    price_unit: PriceUnit,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct FeeEstimation {
    pub l1_gas_consumed: Felt,
    pub l1_gas_price: GasPrice,
    pub l1_data_gas_consumed: Felt,
    pub l1_data_gas_price: GasPrice,
    pub l2_gas_consumed: Felt,
    pub l2_gas_price: GasPrice,
    pub overall_fee: Fee,
    pub unit: PriceUnit,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct TransactionSimulationOutput {
    pub transaction_trace: TransactionTrace,
    pub induced_state_diff: ThinStateDiff,
    pub fee_estimation: FeeEstimation,
}

#[allow(clippy::too_many_arguments)]
// TODO(Dan, Yair): consider box large elements (because of BadDeclareTransaction) or use ID
// instead.
fn execute_transactions<T: StateReader + BlockifierState + HasBlockHash>(
    txs: Vec<ExecutableTransactionInput>,
    chain_id: &ChainId,
    proxy_cached_state: ProxyCachedState<T>,
    gas_prices: &GasPrices,
    block_context_block_number: BlockNumber,
    block_context_timestamp: BlockTimestamp,
    charge_fee: bool,
    validate: bool,
) -> ExecutionResult<(Vec<TransactionExecutionOutput>, BlockContext)> {
    // The starknet state will be from right before the block in which the transactions should run.
    let mut cached_state = CachedState::new(proxy_cached_state);

    let block_context = create_block_context(
        block_context_block_number,
        block_context_timestamp,
        chain_id.clone(),
        gas_prices,
    )?;

    let (txs, tx_hashes) = calc_tx_hashes(txs, chain_id)?;

    let mut res = vec![];
    for (transaction_index, (tx, tx_hash)) in
        txs.into_iter().zip(tx_hashes.into_iter()).enumerate()
    {
        let price_unit = PriceUnit::Fri;
        let mut transactional_state =
            CachedState::create_transactional(&mut cached_state);
        let deprecated_declared_class_hash = match &tx {
            ExecutableTransactionInput::DeclareV0(
                DeclareTransactionV0V1 { class_hash, .. },
                _,
                _,
                _,
            ) => Some(*class_hash),
            ExecutableTransactionInput::DeclareV1(
                DeclareTransactionV0V1 { class_hash, .. },
                _,
                _,
                _,
            ) => Some(*class_hash),
            _ => None,
        };
        let blockifier_tx = to_blockifier_tx(
            tx,
            tx_hash,
            transaction_index,
            charge_fee,
            validate,
        )?;
        // TODO(Yoni): use the TransactionExecutor instead.
        let tx_execution_info_result =
            blockifier_tx.execute(&mut transactional_state, &block_context);
        let state_diff = induced_state_diff(
            &mut transactional_state,
            deprecated_declared_class_hash,
        )?;
        transactional_state.commit();
        let execution_info = tx_execution_info_result.map_err(|error| {
            ExecutionError::from((transaction_index, error))
        })?;
        res.push(TransactionExecutionOutput {
            execution_info,
            induced_state_diff: state_diff,
            price_unit,
        });
    }

    Ok((res, block_context))
}

#[allow(clippy::too_many_arguments)]
pub fn simulate_transactions<
    T: StateReader + BlockifierState + HasBlockHash,
>(
    txs: Vec<ExecutableTransactionInput>,
    chain_id: &ChainId,
    proxy_cached_state: ProxyCachedState<T>,
    gas_prices: &GasPrices,
    block_context_block_number: BlockNumber,
    block_context_timestamp: BlockTimestamp,
    charge_fee: bool,
    validate: bool,
) -> ExecutionResult<Vec<TransactionSimulationOutput>> {
    let trace_constructors =
        txs.iter().map(get_trace_constructor).collect::<Vec<_>>();
    let (execution_results, block_context) = execute_transactions(
        txs,
        chain_id,
        proxy_cached_state,
        gas_prices,
        block_context_block_number,
        block_context_timestamp,
        charge_fee,
        validate,
    )?;
    execution_results
        .into_iter()
        .zip(trace_constructors)
        .map(|(tx_execution_output, trace_constructor)| {
            let fee_estimation = tx_execution_output_to_fee_estimation(
                &tx_execution_output,
                &block_context,
            )?;
            match trace_constructor(tx_execution_output.execution_info) {
                Ok(transaction_trace) => Ok(TransactionSimulationOutput {
                    transaction_trace,
                    induced_state_diff: tx_execution_output.induced_state_diff,
                    fee_estimation,
                }),
                Err(e) => Err(e),
            }
        })
        .collect()
}

fn calc_tx_hashes(
    txs: Vec<ExecutableTransactionInput>,
    chain_id: &ChainId,
) -> ExecutionResult<(Vec<ExecutableTransactionInput>, Vec<TransactionHash>)> {
    Ok(txs
        .into_iter()
        .map(|tx| get_executable_tx_hash(tx, chain_id))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .unzip())
}

fn get_tx_hash(
    tx: &Transaction,
    chain_id: &ChainId,
    only_query: bool,
) -> ExecutionResult<TransactionHash> {
    get_transaction_hash(tx, chain_id, &TransactionOptions { only_query })
        .map_err(ExecutionError::TransactionHashCalculationFailed)
}

fn get_executable_tx_hash(
    tx_input: ExecutableTransactionInput,
    chain_id: &ChainId,
) -> ExecutionResult<(ExecutableTransactionInput, TransactionHash)> {
    let res = match tx_input.clone() {
        ExecutableTransactionInput::Invoke(tx, only_query) => {
            let as_transaction = Transaction::Invoke(tx);
            get_tx_hash(&as_transaction, chain_id, only_query)?
        }
        ExecutableTransactionInput::DeclareV0(
            tx,
            _class,
            _abi_length,
            only_query,
        ) => {
            let as_transaction =
                Transaction::Declare(DeclareTransaction::V0(tx));
            get_tx_hash(&as_transaction, chain_id, only_query)?
        }
        ExecutableTransactionInput::DeclareV1(
            tx,
            _class,
            _abi_length,
            only_query,
        ) => {
            let as_transaction =
                Transaction::Declare(DeclareTransaction::V1(tx));
            get_tx_hash(&as_transaction, chain_id, only_query)?
        }
        ExecutableTransactionInput::DeclareV2(
            tx,
            _class,
            _sierra_program_length,
            _abi_length,
            only_query,
            _sierra_version,
        ) => {
            let as_transaction =
                Transaction::Declare(DeclareTransaction::V2(tx));
            get_tx_hash(&as_transaction, chain_id, only_query)?
        }
        ExecutableTransactionInput::DeclareV3(
            tx,
            _class,
            _sierra_program_length,
            _abi_length,
            only_query,
            _sierra_version,
        ) => {
            let as_transaction =
                Transaction::Declare(DeclareTransaction::V3(tx));
            get_tx_hash(&as_transaction, chain_id, only_query)?
        }
        ExecutableTransactionInput::DeployAccount(tx, only_query) => {
            let as_transaction = Transaction::DeployAccount(tx);
            get_tx_hash(&as_transaction, chain_id, only_query)?
        }
        ExecutableTransactionInput::L1Handler(tx, _fee, only_query) => {
            let as_transaction = Transaction::L1Handler(tx);
            get_tx_hash(&as_transaction, chain_id, only_query)?
        }
    };
    Ok((tx_input, res))
}

fn create_block_context(
    block_context_number: BlockNumber,
    block_context_timestamp: BlockTimestamp,
    chain_id: ChainId,
    gas_prices: &GasPrices,
) -> ExecutionResult<BlockContext> {
    let block_number = block_context_number.unchecked_next();

    let block_info = BlockInfo {
        block_timestamp: block_context_timestamp,
        sequencer_address: ContractAddress::try_from(
            StarkHash::from_hex_unchecked(SEQUENCER_ADDRESS),
        )
        .map_err(|_| ExecutionError::ConfigContentError)?,
        use_kzg_da: true,
        block_number,
        gas_prices: gas_prices.clone(),
    };
    let chain_info = ChainInfo {
        chain_id,
        fee_token_addresses: FeeTokenAddresses {
            strk_fee_token_address: ContractAddress::try_from(
                StarkHash::from_hex_unchecked(STRK_CONTRACT_ADDRESS),
            )
            .map_err(|_| ExecutionError::ConfigContentError)?,
            eth_fee_token_address: ContractAddress::try_from(
                StarkHash::from_hex_unchecked(ETH_CONTRACT_ADDRESS),
            )
            .map_err(|_| ExecutionError::ConfigContentError)?,
        },
        is_l3: false,
    };
    let starknet_version = StarknetVersion::LATEST;
    let mut versioned_constants =
        VersionedConstants::get(&starknet_version)?.clone();
    versioned_constants.enable_casm_hash_migration = false;

    let block_context = BlockContext::new(
        block_info,
        chain_info,
        versioned_constants,
        BouncerConfig::max(),
    );

    Ok(block_context)
}
fn tx_execution_output_to_fee_estimation(
    tx_execution_output: &TransactionExecutionOutput,
    block_context: &BlockContext,
) -> ExecutionResult<FeeEstimation> {
    let gas_prices = &block_context.block_info().gas_prices;
    let (l1_gas_price, l1_data_gas_price, l2_gas_price) = (
        gas_prices.l1_gas_price(&tx_execution_output.price_unit.into()).get(),
        gas_prices
            .l1_data_gas_price(&tx_execution_output.price_unit.into())
            .get(),
        gas_prices.l2_gas_price(&tx_execution_output.price_unit.into()).get(),
    );

    let gas_vector = tx_execution_output.execution_info.receipt.gas;

    Ok(FeeEstimation {
        l1_gas_consumed: gas_vector.l1_gas.0.into(),
        l1_gas_price,
        l1_data_gas_consumed: gas_vector.l1_data_gas.0.into(),
        l1_data_gas_price,
        l2_gas_consumed: gas_vector.l2_gas.0.into(),
        l2_gas_price,
        overall_fee: tx_execution_output.execution_info.receipt.fee,
        unit: tx_execution_output.price_unit,
    })
}

/// ------------------------------------------------------------------------------------------------
/// copy of apollo_rpc_execution private code without modifications
/// ------------------------------------------------------------------------------------------------
fn to_blockifier_tx(
    tx: ExecutableTransactionInput,
    tx_hash: TransactionHash,
    transaction_index: usize,
    charge_fee: bool,
    validate: bool,
) -> ExecutionResult<BlockifierTransaction> {
    // TODO(yair): support only_query version bit (enable in the RPC v0.6 and use the correct
    // value).
    let strict_nonce_check = true;
    match tx {
        ExecutableTransactionInput::Invoke(invoke_tx, only_query) => {
            let execution_flags = ExecutionFlags {
                only_query,
                charge_fee,
                validate,
                strict_nonce_check,
            };
            BlockifierTransaction::from_api(
                Transaction::Invoke(invoke_tx),
                tx_hash,
                None,
                None,
                None,
                execution_flags,
            )
            .map_err(|err| ExecutionError::from((transaction_index, err)))
        }

        ExecutableTransactionInput::DeployAccount(
            deploy_acc_tx,
            only_query,
        ) => {
            let execution_flags = ExecutionFlags {
                only_query,
                charge_fee,
                validate,
                strict_nonce_check,
            };
            BlockifierTransaction::from_api(
                Transaction::DeployAccount(deploy_acc_tx),
                tx_hash,
                None,
                None,
                None,
                execution_flags,
            )
            .map_err(|err| ExecutionError::from((transaction_index, err)))
        }

        ExecutableTransactionInput::DeclareV0(
            declare_tx,
            deprecated_class,
            abi_length,
            only_query,
        ) => {
            let class_info = ClassInfo::new(
                &deprecated_class.into(),
                DEPRECATED_CONTRACT_SIERRA_SIZE,
                abi_length,
                SierraVersion::DEPRECATED,
            )
            .map_err(|err| {
                ExecutionError::BadDeclareTransaction {
                    tx: DeclareTransaction::V0(declare_tx.clone()).into(),
                    err,
                }
            })?;

            let execution_flags = ExecutionFlags {
                only_query,
                charge_fee,
                validate,
                strict_nonce_check,
            };
            BlockifierTransaction::from_api(
                Transaction::Declare(DeclareTransaction::V0(declare_tx)),
                tx_hash,
                Some(class_info),
                None,
                None,
                execution_flags,
            )
            .map_err(|err| ExecutionError::from((transaction_index, err)))
        }
        ExecutableTransactionInput::DeclareV1(
            declare_tx,
            deprecated_class,
            abi_length,
            only_query,
        ) => {
            let class_info = ClassInfo::new(
                &deprecated_class.into(),
                DEPRECATED_CONTRACT_SIERRA_SIZE,
                abi_length,
                SierraVersion::DEPRECATED,
            )
            .map_err(|err| {
                ExecutionError::BadDeclareTransaction {
                    tx: DeclareTransaction::V1(declare_tx.clone()).into(),
                    err,
                }
            })?;
            let execution_flags = ExecutionFlags {
                only_query,
                charge_fee,
                validate,
                strict_nonce_check,
            };
            BlockifierTransaction::from_api(
                Transaction::Declare(DeclareTransaction::V1(declare_tx)),
                tx_hash,
                Some(class_info),
                None,
                None,
                execution_flags,
            )
            .map_err(|err| ExecutionError::from((transaction_index, err)))
        }
        ExecutableTransactionInput::DeclareV2(
            declare_tx,
            compiled_class,
            sierra_program_length,
            abi_length,
            only_query,
            sierra_version,
        ) => {
            let class_info = ClassInfo::new(
                &(compiled_class, sierra_version.clone()).into(),
                sierra_program_length,
                abi_length,
                sierra_version,
            )
            .map_err(|err| {
                ExecutionError::BadDeclareTransaction {
                    tx: DeclareTransaction::V2(declare_tx.clone()).into(),
                    err,
                }
            })?;
            let execution_flags = ExecutionFlags {
                only_query,
                charge_fee,
                validate,
                strict_nonce_check,
            };
            BlockifierTransaction::from_api(
                Transaction::Declare(DeclareTransaction::V2(declare_tx)),
                tx_hash,
                Some(class_info),
                None,
                None,
                execution_flags,
            )
            .map_err(|err| ExecutionError::from((transaction_index, err)))
        }
        ExecutableTransactionInput::DeclareV3(
            declare_tx,
            compiled_class,
            sierra_program_length,
            abi_length,
            only_query,
            sierra_version,
        ) => {
            let class_info = ClassInfo::new(
                &(compiled_class, sierra_version.clone()).into(),
                sierra_program_length,
                abi_length,
                sierra_version,
            )
            .map_err(|err| {
                ExecutionError::BadDeclareTransaction {
                    tx: DeclareTransaction::V3(declare_tx.clone()).into(),
                    err,
                }
            })?;
            let execution_flags = ExecutionFlags {
                only_query,
                charge_fee,
                validate,
                strict_nonce_check,
            };
            BlockifierTransaction::from_api(
                Transaction::Declare(DeclareTransaction::V3(declare_tx)),
                tx_hash,
                Some(class_info),
                None,
                None,
                execution_flags,
            )
            .map_err(|err| ExecutionError::from((transaction_index, err)))
        }
        ExecutableTransactionInput::L1Handler(
            l1_handler_tx,
            paid_fee,
            only_query,
        ) => {
            let execution_flags = ExecutionFlags {
                only_query,
                charge_fee,
                validate,
                strict_nonce_check,
            };
            BlockifierTransaction::from_api(
                Transaction::L1Handler(l1_handler_tx),
                tx_hash,
                None,
                Some(paid_fee),
                None,
                execution_flags,
            )
            .map_err(|err| ExecutionError::from((transaction_index, err)))
        }
    }
}

fn induced_state_diff<T: StateReader + BlockifierState + HasBlockHash>(
    transactional_state: &mut CachedState<
        MutRefState<'_, CachedState<ProxyCachedState<T>>>,
    >,
    deprecated_declared_class_hash: Option<ClassHash>,
) -> ExecutionResult<ThinStateDiff> {
    let blockifier_state_diff = CommitmentStateDiff::from(
        transactional_state.to_state_diff()?.state_maps,
    );

    Ok(ThinStateDiff {
        deployed_contracts: blockifier_state_diff.address_to_class_hash,
        storage_diffs: blockifier_state_diff.storage_updates,
        class_hash_to_compiled_class_hash: blockifier_state_diff
            .class_hash_to_compiled_class_hash,
        deprecated_declared_classes: deprecated_declared_class_hash
            .map_or_else(Vec::new, |class_hash| vec![class_hash]),
        nonces: blockifier_state_diff.address_to_nonce,
    })
}

// need to:
// - add logic for the state
// + build block context
//  + seems like we don`t need pre_process_block
// - map params
// - integrate with context.rs simulateTransactions
//
// missing:
// - gas info for past blocks (can be provided in the pending data, should update this struct during sync)
