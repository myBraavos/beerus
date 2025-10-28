use std::sync::Arc;

use blockifier::{
    bouncer::BouncerConfig,
    context::{BlockContext, ChainInfo, FeeTokenAddresses, TransactionContext},
    transaction::objects::{
        CommonAccountFields, DeprecatedTransactionInfo, TransactionInfo,
    },
    blockifier_versioned_constants::VersionedConstants,
};
use starknet_api::{
    block::{
        BlockInfo, BlockNumber as StarknetBlockNumber, BlockTimestamp,
        GasPriceVector, GasPrices, NonzeroGasPrice,
    },
    core::{
        ChainId as BlockifierChainId, ContractAddress, Nonce,
    },
    hash::StarkHash,
    transaction::{
        fields::{Fee, TransactionSignature},
        TransactionHash, TransactionVersion,
    },
};
use starknet_types_core::felt::Felt as StarkFelt;

use crate::exe::err::Error;

/// Builder for creating execution contexts
pub struct ExecutionContextBuilder {
    block_number: Option<StarknetBlockNumber>,
    block_timestamp: Option<BlockTimestamp>,
    chain_id: BlockifierChainId,
    sequencer_address: Option<ContractAddress>,
    gas_prices: Option<GasPrices>,
    use_kzg_da: bool,
}

impl Default for ExecutionContextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionContextBuilder {
    /// Create a new builder with default values
    pub fn new() -> Self {
        Self {
            block_number: None,
            block_timestamp: None,
            chain_id: BlockifierChainId::Mainnet,
            sequencer_address: None,
            gas_prices: None,
            use_kzg_da: false,
        }
    }

    /// Set the block number
    pub fn block_number(mut self, block_number: StarknetBlockNumber) -> Self {
        self.block_number = Some(block_number);
        self
    }

    /// Set the block timestamp
    pub fn block_timestamp(mut self, block_timestamp: BlockTimestamp) -> Self {
        self.block_timestamp = Some(block_timestamp);
        self
    }

    /// Set the chain ID
    pub fn chain_id(mut self, chain_id: BlockifierChainId) -> Self {
        self.chain_id = chain_id;
        self
    }

    /// Set the sequencer address
    pub fn sequencer_address(mut self, sequencer_address: ContractAddress) -> Self {
        self.sequencer_address = Some(sequencer_address);
        self
    }

    /// Set gas prices
    pub fn gas_prices(mut self, gas_prices: GasPrices) -> Self {
        self.gas_prices = Some(gas_prices);
        self
    }

    /// Set KZG DA usage
    pub fn use_kzg_da(mut self, use_kzg_da: bool) -> Self {
        self.use_kzg_da = use_kzg_da;
        self
    }

    /// Build the execution context for query operations
    pub fn for_query(self) -> Result<TransactionContext, Error> {
        let block_info = BlockInfo {
            block_number: self.block_number.unwrap_or_default(),
            block_timestamp: self.block_timestamp.unwrap_or_default(),
            sequencer_address: self.sequencer_address.unwrap_or_default(),
            gas_prices: self.gas_prices.unwrap_or(GasPrices {
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
            }),
            use_kzg_da: self.use_kzg_da,
        };

        let chain_info = ChainInfo {
            chain_id: self.chain_id,
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

        Ok(TransactionContext { block_context: Arc::new(block_context), tx_info })
    }
}

/// Create a default execution context for query operations
pub fn create_query_context() -> Result<TransactionContext, Error> {
    ExecutionContextBuilder::new().for_query()
}
