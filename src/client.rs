use eyre::Result;
use starknet_api::block_hash::block_hash_calculator::{
    calculate_block_commitments, calculate_block_hash,
};
use std::sync::Arc;

use crate::client::l1_range::L1Range;
use crate::client::state::GatewayState;
use crate::client::utils::{approximate_l1_block, find_l1_sub_range};
use crate::config::Config;
use crate::eth::core_contract::L1CoreContract;
use crate::feeder::GatewayClient;
use crate::gen::client::Client as StarknetClient;
use crate::gen::BlockHash;
use crate::gen::{gen, BlockId, BlockTag, Felt, FunctionCall, Rpc};
use crate::r#gen::BlockNumber;
use crate::storage::storage_trait::StorageProviderTrait;

pub mod http;
pub mod l1_range;
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
    S: StorageProviderTrait,
> {
    starknet: StarknetClient<T>,
    http: T,
    gateway: GatewayClient,
    storage: Arc<S>,
    l1_core_contract: L1CoreContract,
}

impl<
        T: gen::client::HttpClient
            + gen::client::blocking::HttpClient
            + Clone
            + 'static,
        S: StorageProviderTrait,
    > Client<T, S>
{
    /// Create a new client with the given configuration and HTTP client
    pub async fn new(
        config: &Config,
        http: T,
        storage: Arc<S>,
    ) -> Result<Self> {
        let starknet = StarknetClient::new(&config.starknet_rpc, http.clone());
        let rpc_spec_version = starknet.specVersion().await?;
        let version1 = semver::Version::parse(&rpc_spec_version)?;
        let version2 = semver::Version::parse(MIN_RPC_SPEC_VERSION)?;
        if version1 < version2 {
            eyre::bail!("RPC spec version mismatch: expected {MIN_RPC_SPEC_VERSION} but got {rpc_spec_version}");
        }
        let gateway = GatewayClient::new(&config.gateway_url)?;
        let l1_core_contract = L1CoreContract::new(&config.eth_rpc);
        Ok(Self { starknet, http, gateway, storage, l1_core_contract })
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
    pub fn storage(&self) -> &Arc<S> {
        &self.storage
    }

    /// Get the L1 core contract
    pub fn l1(&self) -> &L1CoreContract {
        &self.l1_core_contract
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
        // start with getting block receipt from the Starknet RPC
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

        // then get state update from the Starknet RPC
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

        // then calculate block commitments
        let block_commitments = calculate_block_commitments(
            &transactions_data,
            &state_update.state_diff.try_into()?,
            block_header.l1_da_mode,
            &block_header.starknet_version,
        );

        // then calculate block hash
        let calculated_block_hash =
            calculate_block_hash(block_header, block_commitments)?;
        tracing::debug!(calculated_block_hash=?calculated_block_hash, "calculated block hash");

        // it should match the provided hash
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

    pub async fn get_state_at(&self, block_id: BlockId) -> Result<State> {
        match block_id {
            BlockId::BlockTag(_) => {
                // for the latest and pending states use the latest verified state from the storage
                self.storage().read_latest_state().await
            }
            BlockId::BlockHash { block_hash } => {
                // try to find the state in the storage
                if let Ok(state) =
                    self.storage().read_state_by_hash(&block_hash.0).await
                {
                    return Ok(state);
                }

                // if not found, fetch block number from the gateway
                let gateway_state = self
                    .get_gateway_state_by_id(BlockId::BlockHash { block_hash })
                    .await?;

                // and use L1 to validate the state
                self.sync_state_using_l1(gateway_state.block_number).await
            }
            BlockId::BlockNumber { block_number } => {
                // try to find the state in the storage
                if let Ok(state) =
                    self.storage().read_state(block_number.0).await
                {
                    return Ok(state);
                }

                // if not found, use L1 to validate the state
                self.sync_state_using_l1(block_number.0).await
            }
        }
    }

    async fn sync_state_using_l1(&self, block_number: i64) -> Result<State> {
        // get L1 range from storage
        let l1_range = self.storage().read_l1_range(block_number).await?;
        tracing::debug!(?l1_range, "L1 range from storage");

        // search for the block range in L1 events that contains the target block number
        let (start_state, end_state) =
            self.get_minimal_l1_range(l1_range, block_number).await?;
        tracing::debug!(?start_state, ?end_state, "found minimal L1 range");

        // verify all blocks in the minimal range received from L1
        self.storage().write_state(&start_state).await?;
        let mut found_state = start_state.clone();
        let mut prev_block_hash = start_state.block_hash;
        if let Some(end_state) = end_state {
            // no end_state means that state for exact l2 block found on l1, so we need to verify only if end_state is present
            tracing::debug!(
                "verifying states from {} to {}",
                start_state.block_number,
                end_state.block_number
            );
            for l2_block_number in
                (start_state.block_number + 1)..=end_state.block_number
            {
                tracing::debug!("verifying state at {l2_block_number}");
                let gateway_state =
                    self.get_gateway_state(l2_block_number).await?;
                let state = self
                    .get_verified_state(
                        &gateway_state.block_hash,
                        Some(prev_block_hash),
                    )
                    .await?;
                prev_block_hash = gateway_state.block_hash;
                if state.block_number == block_number {
                    found_state = state;
                }
            }
        };

        Ok(found_state)
    }

    async fn get_minimal_l1_range(
        &self,
        mut l1_range: L1Range,
        block_number: i64,
    ) -> Result<(State, Option<State>)> {
        // check if the block number is at the start or end of the L1 range
        if block_number == l1_range.l2_start {
            let state = self.l1().get_state_on_block(l1_range.l1_start).await?;
            if let Some(state) = state {
                return Ok((state, None));
            }
            tracing::warn!("State update not found for block {block_number}, using L1 range start state");
        } else if block_number == l1_range.l2_end {
            let state = self.l1().get_state_on_block(l1_range.l1_end).await?;
            if let Some(state) = state {
                return Ok((state, None));
            }
            tracing::warn!("State update not found for block {block_number}, using L1 range end state");
        }

        // if not, find the smallest L1 range that contains the block number
        let mut new_l1_ranges: Vec<L1Range> = vec![];
        let mut is_target_below_range = false; // 'true' means that blocks in received range are above the target block number, so we need to search below

        const MAX_L2_RANGE_SIZE: i64 = 500;
        while l1_range.l2_end - l1_range.l2_start < MAX_L2_RANGE_SIZE {
            let mut l1_block_start =
                approximate_l1_block(&l1_range, block_number)? as u64;
            tracing::debug!(
                "L1 range loop, starting from block: {l1_block_start}"
            );
            let l1_initial_start = l1_block_start;
            let mut l1_block_end = l1_range.next_end(l1_block_start);
            let mut found_sub_range: Option<L1Range> = None;
            while !is_target_below_range
                && l1_block_end <= l1_range.l1_end as u64
            {
                let states = self
                    .l1()
                    .get_l1_state_updates(l1_block_start, l1_block_end)
                    .await?;
                tracing::debug!("loop above {l1_block_start}-{l1_block_end}, states: {states:#?}");
                if !states.is_empty() {
                    let (sub_range, is_below) = find_l1_sub_range(
                        l1_range.clone(),
                        &states,
                        block_number,
                        &mut new_l1_ranges,
                    )?;
                    found_sub_range = sub_range;
                    is_target_below_range = is_below;
                    break;
                }
                l1_block_start = l1_block_end + 1;
                l1_block_end = l1_range.next_end(l1_block_end);
            }
            if found_sub_range.is_none() {
                l1_block_end = l1_initial_start;
                l1_block_start = l1_range.prev_start(l1_initial_start);
                // TODO: refactor duplicated code
                while is_target_below_range
                    && l1_block_start >= l1_range.l1_start as u64
                {
                    let states = self
                        .l1()
                        .get_l1_state_updates(l1_block_start, l1_block_end)
                        .await?;
                    tracing::debug!("loop below {l1_block_start}-{l1_block_end}, states: {states:#?}");
                    if !states.is_empty() {
                        let (sub_range, is_below) = find_l1_sub_range(
                            l1_range.clone(),
                            &states,
                            block_number,
                            &mut new_l1_ranges,
                        )?;
                        found_sub_range = sub_range;
                        is_target_below_range = is_below;
                        break;
                    }
                    l1_block_end = l1_block_start - 1;
                    l1_block_start = l1_range.prev_start(l1_block_start);
                }
            }
            l1_range = found_sub_range.ok_or(eyre::eyre!(
                "State not found for block {block_number}"
            ))?;
        }

        // store updated L1 ranges
        for l1_range in new_l1_ranges {
            self.storage().write_l1_range(&l1_range).await?;
        }

        // TODO: use already fetched states if available
        let start_state = self
            .l1()
            .get_state_on_block(l1_range.l1_start)
            .await?
            .ok_or(eyre::eyre!("State not found"))?;
        let end_state: Option<State> = if l1_range.l2_end == l1_range.l2_start {
            // the exact block was found in L1 commitment, no need to verify the range
            None
        } else {
            self.l1().get_state_on_block(l1_range.l1_end).await?
        };

        Ok((start_state, end_state))
    }
}
