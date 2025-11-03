use eyre::Result;
use futures::stream::{StreamExt, TryStreamExt};
use std::sync::Arc;

use crate::client::block_hash::validate_block_hash;
use crate::client::l1_range::L1Range;
use crate::client::rate_limiter::RateLimiter;
use crate::client::state::GatewayState;
use crate::client::utils::{approximate_l1_block, find_l1_sub_range};
use crate::config::Config;
use crate::eth::core_contract::L1CoreContract;
use crate::feeder::GatewayClient;
use crate::gen::client::Client as StarknetClient;
use crate::gen::{gen, BlockId, BlockTag, Felt, FunctionCall, Rpc};
use crate::gen::{BlockHash, BlockNumber, BlockWithReceipts, StateUpdate};
use crate::storage::storage_trait::StorageProviderTrait;

pub mod block_hash;
pub mod http;
pub mod l1_range;
pub mod rate_limiter;
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
    config: Config,
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
        Ok(Self {
            starknet,
            http,
            gateway,
            storage,
            l1_core_contract,
            config: config.clone(),
        })
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
    /// Retrieves and verifies the state for a given Starknet block hash.
    ///
    /// This method fetches the block and its receipts using the Starknet RPC, checks parent hash correctness
    /// (if provided), verifies that the block's Starknet version is supported, fetches the corresponding state update,
    /// and validates the relationship between the block and state update, including the block hash.
    /// Upon successful validation, it constructs a minimal [`State`] record, persists it, and returns it.
    ///
    /// # Arguments
    /// * `block_hash` - Hash of the block whose state should be fetched and verified.
    /// * `prev_block_hash` - (Optional) Hash of the previous block; if provided, this method asserts the parent
    ///   block hash of the fetched block matches this value, ensuring continuity.
    ///
    /// # Errors
    /// Returns an error if any RPC or data conversion fails, the parent hash does not match (if provided),
    /// the Starknet version is unsupported, or validation of the block hash relationship fails.
    pub async fn get_verified_state(
        &self,
        block_hash: &Felt,
        prev_block_hash: Option<Felt>,
    ) -> Result<State> {
        // Step 1: Retrieve block with receipts from the Starknet RPC using the given block hash.
        let block_id = BlockId::BlockHash { block_hash: BlockHash(block_hash.clone()) };
        let block: BlockWithReceipts =
            self.starknet.getBlockWithReceipts(block_id).await?.try_into()?;

        // Step 2: Validate the parent block hash if `prev_block_hash` is supplied.
        let parent_block_hash = block.block_header.parent_hash.0.clone();
        if let Some(prev_block_hash) = prev_block_hash {
            if parent_block_hash != prev_block_hash {
                eyre::bail!("Prev block hash mismatch: expected {prev_block_hash:?} but got {parent_block_hash:?}");
            }
        }

        // Step 3: Ensure Starknet protocol version isn't above max supported.
        let starknet_version = semver::Version::parse(&block.block_header.starknet_version)?;
        let max_starknet_version = semver::Version::parse(MAX_STARKNET_VERSION)?;
        if starknet_version > max_starknet_version {
            eyre::bail!("Unsupported starknet version: {starknet_version}, max supported: {MAX_STARKNET_VERSION}");
        }

        // Step 4: Fetch state update for block by number.
        let state_update = self
            .starknet
            .getStateUpdate(gen::BlockId::BlockNumber {
                block_number: block.block_header.block_number.clone(),
            })
            .await?
            .try_into()?;

        // Step 5: Validate relationship between block and state update.
        validate_block_hash(&block, &state_update, block_hash)?;

        // Step 6: Construct local minimal state and persist it.
        let state = State::new(
            *block.block_header.block_number.as_ref(),
            block.block_header.block_hash.0,
            block.block_header.new_root,
        );
        self.storage().write_state(&state).await?;
        Ok(state)
    }

    /// Verifies and persists a range of Starknet state blocks by:
    /// 1. Sequentially fetching each block and its state update between two given states (exclusive of start, inclusive of end).
    /// 2. Validating each block's hash and parent hash, and the relationship between state update and block.
    /// 3. Ensuring the chain is contiguous (block.prev_hash == prev_block.hash).
    /// 4. Finally, persisting verified states for each block in the range to storage.
    ///
    /// # Arguments
    /// * `start_state` - The starting state (block will NOT be included for validation or storage).
    /// * `end_state` - The ending state (block and state will be fetched and validated).
    ///
    /// # Errors
    /// Returns an error if any RPC or data conversion fails, block hash validation fails,
    /// or if chain consistency is not preserved.
    pub async fn verify_state_range(
        &self,
        start_state: State,
        end_state: State,
    ) -> Result<()> {
        // Step 1: Collect all block IDs to verify (exclusive range)
        tracing::debug!(?start_state, ?end_state, "verify_state_range");
        let block_ids: Vec<BlockId> = (start_state.block_number + 1
            ..end_state.block_number + 1)
            .map(|block_number| BlockId::BlockNumber {
                block_number: BlockNumber::try_new(block_number).unwrap(),
            })
            .collect();

        // Step 2: Fetch each block and corresponding state update in parallel, using a rate limiter
        let rate_limiter = RateLimiter::new(self.config.batch_size);

        let responses: Vec<(BlockWithReceipts, StateUpdate)> =
            futures::stream::iter(block_ids)
                .map(|block_id| {
                    let starknet = self.starknet.clone();
                    let rate_limiter = rate_limiter.new_instance();
                    async move {
                        // Wait for ratelimit slot
                        rate_limiter.wait().await;

                        tracing::debug!("requesting block {:?}", block_id);

                        // Fetch block with receipts
                        let block: BlockWithReceipts = starknet
                            .getBlockWithReceipts(block_id.clone())
                            .await?
                            .try_into()?;
                        // Fetch state update for the same block
                        let state_update: StateUpdate = starknet
                            .getStateUpdate(block_id)
                            .await?
                            .try_into()?;
                        Ok::<(BlockWithReceipts, StateUpdate), eyre::Error>((
                            block,
                            state_update,
                        ))
                    }
                })
                .buffer_unordered(self.config.batch_size)
                .try_collect()
                .await?;

        // Step 3: For each fetched (block,state_update), validate block hash in parallel
        let mut results: Vec<BlockWithReceipts> =
            futures::stream::iter(responses)
                .map(|(block, state_update)| async move {
                    tracing::debug!(
                        "validating block hash for block {}",
                        block.block_header.block_number.0
                    );
                    let block1 = block.clone();
                    // Validate in a blocking thread since it may be CPU-heavy
                    let _ = tokio::task::spawn_blocking(move || {
                        validate_block_hash(
                            &block,
                            &state_update,
                            &block.block_header.block_hash.0,
                        )?;
                        Ok::<(), eyre::Error>(())
                    })
                    .await?;
                    Ok::<BlockWithReceipts, eyre::Error>(block1)
                })
                .buffer_unordered(100)
                .try_collect()
                .await?;

        // Step 4: Sort results by block number (to guarantee sequential checking)
        results.sort_by_key(|block| block.block_header.block_number.0);

        // Step 5: Verify parent hashes form a contiguous chain
        let mut prev_block_hash = start_state.block_hash.clone();
        for block in &results {
            if block.block_header.parent_hash.0 != prev_block_hash {
                eyre::bail!("Prev block hash mismatch: expected {prev_block_hash:?} but got {:?}", block.block_header.parent_hash.0.as_ref());
            }
            prev_block_hash = block.block_header.block_hash.0.clone();
        }

        // Step 6: Verify last block hash matches the ending state block hash
        let Some(last_block) = results.last() else {
            eyre::bail!("No blocks received");
        };
        if last_block.block_header.block_hash.0 != end_state.block_hash {
            eyre::bail!(
                "End block hash mismatch: expected {:?} but got {:?}",
                end_state.block_hash,
                last_block.block_header.block_hash.0.as_ref()
            );
        }

        // Step 7: Store all verified states to storage
        for block in results {
            let state = State::new(
                *block.block_header.block_number.as_ref(),
                block.block_header.block_hash.0,
                block.block_header.new_root,
            );
            self.storage().write_state(&state).await?;
        }

        tracing::debug!("range verified");
        Ok(())
    }

    /// Retrieves the state for a given block identifier (block number, block hash, or block tag).
    ///
    /// This function attempts to resolve the block identity and fetch the corresponding verified
    /// [`State`] from local storage or by performing on-demand verification using L1 proofs.
    ///
    /// # Arguments
    /// * `block_id` - A [`BlockId`] specifying the block by number, hash, or tag (e.g., latest, pending).
    ///
    /// # Returns
    /// Returns the [`State`] at the requested block, verified either from persistent storage or
    /// via L1 validation.
    ///
    /// # Behavior
    /// - For [`BlockId::BlockTag`] (e.g., latest, pending): fetches the latest verified state from storage.
    /// - For [`BlockId::BlockHash`]: attempts to fetch by hash from storage; if not found, queries the gateway
    ///   to resolve the corresponding block number, then validates that state using L1 verification logic.
    /// - For [`BlockId::BlockNumber`]: attempts to fetch by number from storage; if not found, validates it using L1.
    pub async fn get_state_at(&self, block_id: BlockId) -> Result<State> {
        match block_id {
            BlockId::BlockTag(_) => {
                // For the latest and pending states, use the latest verified state from storage.
                self.storage().read_latest_state().await
            }
            BlockId::BlockHash { block_hash } => {
                // Try to find the state in storage by block hash.
                if let Ok(state) =
                    self.storage().read_state_by_hash(&block_hash.0).await
                {
                    return Ok(state);
                }

                // If not found, fetch block number from the gateway using the block hash.
                let gateway_state = self
                    .get_gateway_state_by_id(BlockId::BlockHash { block_hash })
                    .await?;

                // Use L1 (Ethereum) to validate and reconstruct the state at that block number.
                self.sync_state_using_l1(gateway_state.block_number).await
            }
            BlockId::BlockNumber { block_number } => {
                // Try to find the state in storage by block number.
                if let Ok(state) =
                    self.storage().read_state(block_number.0).await
                {
                    return Ok(state);
                }

                // If not found, use L1 to validate and reconstruct the state at the given block number.
                self.sync_state_using_l1(block_number.0).await
            }
        }
    }

    /// Synchronizes a specific Starknet L2 state using L1 (Ethereum) event proofs.
    ///
    /// This function reconstructs or validates the Starknet block state at a given L2 block number
    /// by leveraging L1 stored events with state updates and on-chain proofs.
    ///
    /// # Workflow
    /// 1. Queries the database for the most recent L1 range relevant to the target `block_number`.
    /// 2. Identifies the minimal L1 range (from within L1 logs/events) required to verify or reconstruct
    ///    the desired L2 block state.
    /// 3. Writes the starting state of this range to local storage.
    /// 4. If further verification is required (i.e., the target state isn't the range boundary),
    ///    performs stepwise verification of state transitions within the range, storing intermediate states.
    /// 5. Returns the final verified state for the given block number from persistent storage.
    ///
    /// # Arguments
    /// * `block_number` - The L2 Starknet block number to synchronize.
    ///
    /// # Returns
    /// Returns the fully verified [`State`] for the requested L2 block.
    ///
    /// # Errors
    /// Returns an error if storage access, L1 event retrieval, or state verification fails.
    async fn sync_state_using_l1(&self, block_number: i64) -> Result<State> {
        // Retrieve the corresponding L1 range from storage; this describes which L1 blocks
        // encapsulate the L2 state transitions relevant to `block_number`.
        let l1_range = self.storage().read_l1_range(block_number).await?;
        tracing::debug!(?l1_range, "L1 range from storage");

        // Identify the smallest necessary L1 range and its start/end verified state.
        let (start_state, end_state) =
            self.get_minimal_l1_range(l1_range, block_number).await?;
        tracing::debug!(?start_state, ?end_state, "found minimal L1 range");

        // Persist the starting state from the minimal range (may be sufficient if block is on a range boundary).
        self.storage().write_state(&start_state).await?;

        // If an end state is present, it means our exact target state was not at a range boundary
        // and we must verify all intermediate transitions from start_state to end_state.
        if let Some(end_state) = end_state {
            tracing::debug!(
                "verifying states from {} to {}",
                start_state.block_number,
                end_state.block_number
            );
            // This function verifies all L2 blocks between start_state and end_state,
            // storing them to persistent storage, including the target state.
            self.verify_state_range(start_state.clone(), end_state).await?;
        };

        // Finally, retrieve and return the requested, now-verified state from storage.
        self.storage().read_state(block_number).await
    }

    /// Finds the minimal L1 range necessary to verify the state for a given L2 block number.
    ///
    /// This function attempts to narrow down the provided L1 range to be as small as possible,
    /// such that it either directly contains or tightly bounds the requested L2 block number.
    /// It first checks if the block number is at the start or end of the L1 range, and tries to
    /// retrieve a pre-committed state. If more narrowing is required (range is too wide),
    /// it iteratively bisects the L1 range by querying for state updates above or below the target
    /// until a sufficiently narrow range is found. All discovered sub-ranges are persisted in storage.
    ///
    /// # Arguments
    /// * `l1_range` - Initial broad L1 range, likely from storage.
    /// * `block_number` - The target L2 block number to verify.
    ///
    /// # Returns
    /// Returns (`State`, `Option<State>`), where:
    /// - The first is always the starting state (for target, or range start).
    /// - The second is an optional end state (if the L2 block is between state updates)
    ///
    /// # Errors
    /// Returns an error if no suitable state can be found or data retrieval fails.
    async fn get_minimal_l1_range(
        &self,
        mut l1_range: L1Range,
        block_number: i64,
    ) -> Result<(State, Option<State>)> {
        // Check if the block number coincides with the start or end of the L1 range.
        // In that case, fetch and return the corresponding state immediately.
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

        // Otherwise, iteratively narrow the L1 range until it is suitably small.
        // This ensures we only process the minimal number of blocks necessary for verification.
        let mut new_l1_ranges: Vec<L1Range> = vec![];
        // Indicates whether the target block number is below the current sub-range.
        let mut is_target_below_range = false;

        const MAX_L2_RANGE_SIZE: i64 = 500;
        while l1_range.l2_end - l1_range.l2_start > MAX_L2_RANGE_SIZE {
            // Determine an approximate L1 block start close to the block of interest.
            let mut l1_block_start =
                approximate_l1_block(&l1_range, block_number)? as u64;
            tracing::debug!(
                "L1 range loop, starting from block: {l1_block_start}"
            );
            let l1_initial_start = l1_block_start;
            let mut l1_block_end =
                l1_range.next_end(l1_block_start, self.config.l1_range_blocks);
            let mut found_sub_range: Option<L1Range> = None;

            // Search for sub-ranges above the current block (progressing upward).
            while !is_target_below_range
                && l1_block_end <= l1_range.l1_end as u64
            {
                // Query for all L1 state updates within [l1_block_start, l1_block_end]
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
                // Move to the next range window above.
                l1_block_start = l1_block_end + 1;
                l1_block_end = l1_range
                    .next_end(l1_block_end, self.config.l1_range_blocks);
            }

            // If not found above, search "below" by stepping downwards.
            if found_sub_range.is_none() {
                l1_block_end = l1_initial_start;
                l1_block_start = l1_range
                    .prev_start(l1_initial_start, self.config.l1_range_blocks);
                while is_target_below_range
                    && l1_block_start >= l1_range.l1_start as u64
                {
                    // Look for state updates in window [l1_block_start, l1_block_end]
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
                    // Step the window further downward
                    l1_block_end = l1_block_start - 1;
                    l1_block_start = l1_range.prev_start(
                        l1_block_start,
                        self.config.l1_range_blocks,
                    );
                }
            }

            // At this point we should have found a sub-range, otherwise we received invalid data from L1.
            l1_range = found_sub_range.ok_or(eyre::eyre!(
                "L1 range not found for block {block_number}"
            ))?;
        }

        // Persist all newly discovered L1 sub-ranges for future efficiency.
        for l1_range in new_l1_ranges {
            self.storage().write_l1_range(&l1_range).await?;
        }

        // TODO: Could use already fetched states for efficiency
        // Fetch the starting state for the minimal range. This will always be present, otherwise we received invalid data from L1.
        let start_state = self
            .l1()
            .get_state_on_block(l1_range.l1_start)
            .await?
            .ok_or(eyre::eyre!("State not found"))?;
        let end_state: Option<State> = if l1_range.l2_end == l1_range.l2_start {
            // The exact block was found in L1 commitment, no need to verify the range
            None
        } else {
            self.l1().get_state_on_block(l1_range.l1_end).await?
        };

        Ok((start_state, end_state))
    }

    /// Stores the latest L1 range in persistent storage, updating it if the provided L1 state is ahead.
    ///
    /// This function ensures that the storage's record of the "latest" L1 range
    /// (which tracks the mapping between L1 and L2 block commitments)
    /// remains accurate and up-to-date. It is typically called after detecting that the L1 state
    /// has advanced beyond the current stored L1 range.
    ///
    /// # Arguments
    /// * `l1_state` - The newly observed L1 state for which we want to ensure the corresponding L1 range is stored.
    ///
    /// # Returns
    /// Returns `Ok(())` if the latest L1 range is persisted or was already up-to-date, or an error otherwise.
    ///
    /// # Process
    /// 1. Reads the latest L1 range from storage.
    /// 2. Checks if the provided state is already within the latest range; if so, returns immediately.
    /// 3. Otherwise, begins searching for the new L1 range by querying for state updates in descending blocks.
    /// 4. Iteratively halves the search window downward until it locates an event containing the appropriate state update.
    /// 5. Updates and persists the new range in storage.
    pub async fn store_latest_l1_range(&self, l1_state: &State) -> Result<()> {
        // Get the latest L1 range stored in persistent storage.
        let latest_l1_range = self.storage().read_latest_l1_range().await?;
        // If the provided state is already within the latest available L1 range, no update is necessary.
        if l1_state.block_number <= latest_l1_range.l2_end {
            return Ok(());
        }
        tracing::debug!(
            "updating latest L1 range from {} to {}",
            latest_l1_range.l2_end,
            l1_state.block_number
        );

        // Start searching for a new L1 range that includes the new state update.
        // Begin at the latest L1 block number seen.
        let mut end_block = self.l1().get_latest_block_number().await?;
        let mut l1_range = L1Range::new(
            latest_l1_range.l1_end,
            end_block as i64,
            latest_l1_range.l2_end,
            l1_state.block_number,
        );
        // Determine the start block for the search window.
        let mut start_block =
            l1_range.prev_start(end_block, self.config.l1_range_blocks);

        // Iteratively look for the most recent state update event by descending search windows.
        while end_block > start_block {
            // Fetch state update events within the current search window on L1.
            let states =
                self.l1().get_l1_state_updates(start_block, end_block).await?;
            match states.last() {
                Some((state, l1_block_number)) => {
                    // Found a new state update event. Update the end boundaries of the new L1 range.
                    l1_range.l1_end = *l1_block_number as i64;
                    l1_range.l2_end = state.block_number;
                    break;
                }
                None => {
                    // No events in current window. Step the search window downward.
                    end_block = start_block - 1;
                    start_block = l1_range
                        .prev_start(start_block, self.config.l1_range_blocks);
                }
            }
        }

        // Store the new L1 range in persistent storage for future reference.
        tracing::debug!(?l1_range, "new L1 range");
        self.storage().write_l1_range(&l1_range).await?;

        Ok(())
    }
}
