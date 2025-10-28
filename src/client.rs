use eyre::Result;
use starknet_api::block_hash::block_hash_calculator::{
    calculate_block_commitments, calculate_block_hash,
};
use std::sync::Arc;

use crate::client::state::GatewayState;
use crate::config::Config;
use crate::feeder::GatewayClient;
use crate::gen::client::Client as StarknetClient;
use crate::gen::BlockHash;
use crate::gen::{gen, BlockId, BlockTag, Felt, FunctionCall, Rpc};
use crate::r#gen::BlockNumber;
use crate::storage::storage_trait::StorageProviderTrait;

pub mod http;
pub mod state;
pub mod utils;

pub use http::Http;
pub use state::State;
pub use utils::as_felt;

const MIN_RPC_SPEC_VERSION: &str = "0.8.1";
const MAX_STARKNET_VERSION: &str = "0.14.0";
const FIRST_SUPPORTED_BLOCK_NUMBER: i64 = 1_000_000;

/// Main client for interacting with Starknet
pub struct Client<
    T: gen::client::HttpClient
        + gen::client::blocking::HttpClient
        + Clone
        + 'static,
> {
    starknet: StarknetClient<T>,
    http: T,
    gateway: GatewayClient,
    storage: Arc<dyn StorageProviderTrait>,
}

impl<
        T: gen::client::HttpClient
            + gen::client::blocking::HttpClient
            + Clone
            + 'static,
    > Client<T>
{
    /// Create a new client with the given configuration and HTTP client
    pub async fn new(
        config: &Config,
        http: T,
        storage: Arc<dyn StorageProviderTrait>,
    ) -> Result<Self> {
        let starknet = StarknetClient::new(&config.starknet_rpc, http.clone());
        let rpc_spec_version = starknet.specVersion().await?;
        let version1 = semver::Version::parse(&rpc_spec_version)?;
        let version2 = semver::Version::parse(MIN_RPC_SPEC_VERSION)?;
        if version1 < version2 {
            eyre::bail!("RPC spec version mismatch: expected {MIN_RPC_SPEC_VERSION} but got {rpc_spec_version}");
        }
        let gateway = GatewayClient::new(&config.gateway_url)?;
        Ok(Self { starknet, http, gateway, storage })
    }

    /// Get the underlying Starknet client
    pub fn starknet(&self) -> &StarknetClient<T> {
        &self.starknet
    }

    /// Get the HTTP client
    pub fn http(&self) -> &T {
        &self.http
    }

    /// Get the storage provider
    pub fn storage(&self) -> &Arc<dyn StorageProviderTrait> {
        &self.storage
    }

    /// Execute a function call on the Starknet state
    pub fn execute(
        &self,
        request: FunctionCall,
        state: State,
    ) -> Result<Vec<Felt>> {
        let client = gen::client::blocking::Client::new(
            &self.starknet.url,
            self.http.clone(),
        );
        let call_info = crate::exe::call(client, request, state)?;
        call_info
            .execution
            .retdata
            .0
            .into_iter()
            .map(|felt| as_felt(&felt.to_bytes_be()))
            .collect()
    }

    // Get minimal state from feeder gateway
    pub async fn get_gateway_state_by_id(
        &self,
        block_id: BlockId,
    ) -> Result<GatewayState> {
        if let BlockId::BlockNumber { block_number } = block_id.clone() {
            // TODO: implement block hash verification for older blocks
            if block_number.0 < FIRST_SUPPORTED_BLOCK_NUMBER {
                eyre::bail!("Block number is too low: {block_number:?}, minimum supported: {FIRST_SUPPORTED_BLOCK_NUMBER}");
            }
        }
        self.gateway.get_state(block_id).await
    }

    pub async fn get_gateway_state(
        &self,
        block_number: i64,
    ) -> Result<GatewayState> {
        self.get_gateway_state_by_id(BlockId::BlockNumber {
            block_number: BlockNumber::try_new(block_number).unwrap(),
        })
        .await
    }

    pub async fn get_latest_gateway_state(&self) -> Result<GatewayState> {
        self.get_gateway_state_by_id(BlockId::BlockTag(BlockTag::Latest)).await
    }

    /// Get and verify block state from rpc
    pub async fn get_verified_state(
        &self,
        block_hash: &Felt,
        prev_block_hash: Option<Felt>,
    ) -> Result<State> {
        let block_id =
            BlockId::BlockHash { block_hash: BlockHash(block_hash.clone()) };
        let block = self.starknet.getBlockWithReceipts(block_id).await?;
        let gen::GetBlockWithReceiptsResult::BlockWithReceipts(block) = block
        else {
            eyre::bail!("Pending block received, which is not supported");
        };

        let parent_block_hash = block.block_header.parent_hash.0.clone();
        if let Some(prev_block_hash) = prev_block_hash {
            if parent_block_hash != prev_block_hash {
                eyre::bail!("Prev block hash mismatch: expected {prev_block_hash:?} but got {parent_block_hash:?}");
            }
        }

        let starknet_version =
            semver::Version::parse(&block.block_header.starknet_version)?;
        let max_starknet_version =
            semver::Version::parse(MAX_STARKNET_VERSION)?;
        if starknet_version > max_starknet_version {
            eyre::bail!("Unsupported starknet version: {starknet_version}, max supported: {MAX_STARKNET_VERSION}");
        }

        let block_header: starknet_api::block::BlockHeaderWithoutHash =
            block.block_header.clone().try_into()?;

        let state_update = self
            .starknet
            .getStateUpdate(gen::BlockId::BlockNumber {
                block_number: block.block_header.block_number.clone(),
            })
            .await?;
        let gen::GetStateUpdateResult::StateUpdate(state_update) = state_update
        else {
            eyre::bail!("Pending state received, which is not supported");
        };
        let transactions_data = block.block_body_with_receipts.transactions.into_iter().map(|transaction_and_receipt| {
            transaction_and_receipt.try_into().unwrap()
        }).collect::<Vec<starknet_api::block_hash::block_hash_calculator::TransactionHashingData>>();
        let block_commitments = calculate_block_commitments(
            &transactions_data,
            &state_update.state_diff.try_into()?,
            block_header.l1_da_mode,
            &block_header.starknet_version,
        );

        let calculated_block_hash =
            calculate_block_hash(block_header, block_commitments)?;
        tracing::debug!(calculated_block_hash=?calculated_block_hash, "calculated block hash");

        if calculated_block_hash.0
            != starknet_api::hash::StarkHash::from_hex_unchecked(
                block_hash.as_ref(),
            )
        {
            eyre::bail!("Block hash mismatch: expected {block_hash:?} but got {calculated_block_hash:?}");
        }

        let state = State::new(
            *block.block_header.block_number.as_ref(),
            block.block_header.block_hash.0,
            block.block_header.new_root,
        );
        self.storage().write_state(&state).await?;
        Ok(state)
    }

    /// DEPRECATED: Use get_verified_state instead
    /// Get the current state of the blockchain
    pub async fn get_state(&self) -> Result<State> {
        let block_id = BlockId::BlockTag(BlockTag::Latest);
        let block = self.starknet.getBlockWithReceipts(block_id).await?;
        let gen::GetBlockWithReceiptsResult::BlockWithReceipts(block) = block
        else {
            eyre::bail!("Pending block received, which is not supported");
        };
        Ok(State::new(
            *block.block_header.block_number.as_ref(),
            block.block_header.block_hash.0,
            block.block_header.new_root,
        ))
    }
}
