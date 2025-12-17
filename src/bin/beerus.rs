use std::{sync::Arc, time::Duration};

use beerus::{
    background_loader::{
        async_blocker::AsyncBlocker, loader::BackgroundLoader,
    },
    client::{state::GatewayState, Client, Http, State},
    config::ServerConfig,
    gen::{BlockId, BlockNumber},
    storage::{
        sql_storage_provider::SqlStorageProvider,
        storage_trait::StorageProviderTrait,
    },
    util::with_retry,
};
use tokio::time::Instant;
use validator::Validate;

#[cfg(not(tarpaulin_include))] // exclude from code-coverage report
#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();

    let config = get_config().await?;

    let http = Http::new();
    let storage =
        Arc::new(SqlStorageProvider::new(&config.client.database_url).await?);
    let async_blocker = Arc::new(AsyncBlocker::new());
    let beerus = Arc::new(Client::new(&config.client, http, storage).await?);
    let server =
        beerus::rpc::Server::new(beerus.clone(), async_blocker.clone());
    let background_loader =
        BackgroundLoader::new(beerus.clone(), async_blocker.clone());

    {
        let period = Duration::from_secs(config.poll_secs);
        tokio::spawn(async move {
            let (
                mut tick,
                mut l1_sync_check,
                mut gateway_state,
                mut verified_state,
            ) = match prepare_main_loop(&beerus, async_blocker.clone(), period)
                .await
            {
                Ok((tick, l1_sync_check, gateway_state, verified_state)) => {
                    (tick, l1_sync_check, gateway_state, verified_state)
                }
                Err(e) => {
                    panic!(
                        "failed to prepare main loop, exiting sync task: {e:?}"
                    );
                }
            };
            loop {
                tick.tick().await;
                let _guard = async_blocker.block_tasks();
                match execute_sync(
                    &beerus,
                    &mut gateway_state,
                    &mut verified_state,
                    &mut l1_sync_check,
                    config.l1_poll_secs,
                )
                .await
                {
                    Ok(_) => (),
                    Err(e) => {
                        tracing::error!(error=%e, "failed to execute sync");
                    }
                }
            }
        });
    }

    if !config.client.disable_background_loader {
        tokio::spawn(async move {
            background_loader.run().await;
        });
    }

    beerus::rpc::serve_on(server, &config.rpc_addr.to_string()).await.unwrap();
    Ok(())
}

#[cfg(not(tarpaulin_include))] // exclude from code-coverage report
async fn get_config() -> eyre::Result<ServerConfig> {
    let args: Vec<String> = std::env::args().collect();
    let config = if args.len() > 2 && args[1] == "-c" {
        // Handle -c flag followed by config file path
        ServerConfig::from_file(&args[2])?
    } else if args.len() > 1 {
        // Handle config file path as first argument (without -c flag)
        ServerConfig::from_file(&args[1])?
    } else {
        // Fall back to environment variables
        ServerConfig::from_env()?
    };
    config.validate()?;
    Ok(config)
}

/// Prepares the main synchronization loop for the Beerus client.
///
/// This async function determines the correct starting point for syncing
/// by checking the storage for the latest stored L2 state and the latest L1 state.
/// It then chooses either to continue from the latest stored block, or, if the L1 state is ahead,
/// updates storage to the L1 state and starts from there. It then fetches the required
/// Starknet gateway state and verifies the state at the determined block.
///
/// # Arguments
/// * `beerus` - The Beerus client instance used for network and storage access.
/// * `period` - The duration between sync ticks, controlling periodic sync operations.
///
/// # Returns
/// Returns a tuple containing:
/// - `tokio::time::Interval`: Timer for driving the sync loop.
/// - `Instant`: Timestamp representing when the last L1 sync was performed (initially set to now).
/// - `GatewayState`: The initial Starknet gateway state from which to begin syncing.
/// - `State`: The corresponding verified Starknet state at the starting block.
///
/// # Errors
/// This function returns an error if there is a failure in fetching required state from storage
/// or the network.
//
/// # Detailed Steps
/// 1. Try to read the latest state from storage. If it exists, extract the block number and hash;
///    otherwise, default to block 0 and a missing hash.
/// 2. Fetch the most recent L1 state from Ethereum.
/// 3. If the L1 state is ahead of our storage, write it to storage and use it as our starting point.
/// 4. Else, continue syncing from the latest block in local storage.
/// 5. Fetch the initial Starknet gateway and verified states to seed the sync loop.
/// 6. Return the timers and state as a tuple for driving the sync task.
async fn prepare_main_loop(
    beerus: &Client<Http, SqlStorageProvider>,
    async_blocker: Arc<AsyncBlocker>,
    period: Duration,
) -> eyre::Result<(tokio::time::Interval, Instant, GatewayState, State)> {
    let _guard = async_blocker.block_tasks();

    // Attempt to find the last synced L2 state in persistent storage.
    let latest_stored_state = beerus.storage().read_latest_state().await;
    let latest_stored_block = match &latest_stored_state {
        Ok(state) => state.block_number,
        Err(_) => 0,
    };

    let l1_state = beerus.l1().get_l1_state().await?;
    // Decide sync starting point: if L1 head is ahead of L2, start from there, else use last L2.
    let mut verified_state = if l1_state.block_number > latest_stored_block {
        let state =
            beerus.verify_and_update_state(&l1_state.block_hash, None).await?;
        beerus.store_latest_l1_range(&l1_state).await?;
        state
    } else {
        latest_stored_state?
    };

    // Sync missing blocks
    let gateway_state = beerus.get_latest_gateway_state().await?;
    if gateway_state.block_number > verified_state.block_number {
        beerus
            .verify_state_range(
                verified_state.clone(),
                gateway_state.clone().into(),
                None,
            )
            .await?;
        verified_state = beerus
            .get_state_at(BlockId::BlockNumber {
                block_number: BlockNumber::try_new(gateway_state.block_number)
                    .unwrap(),
            })
            .await?;
    }
    tracing::info!(
        "Starting the sync from block {}",
        verified_state.block_number
    );

    // Prepare interval timer for sync period, and note when the last L1 sync was checked.
    let tick = tokio::time::interval(period);
    let l1_sync_check = Instant::now();

    Ok((tick, l1_sync_check, gateway_state, verified_state))
}

/// Synchronize the local client state with the latest Starknet state and L1 state.
///
/// This function performs both L2 and L1 synchronization:
/// - L2 synchronization: iteratively updates `gateway_state` and `verified_state`
///   until they are caught up with the most recent Starknet gateway block. Each
///   intermediate block is sequentially synchronized and verified.
/// - L1 synchronization: at a given polling interval (`l1_poll_secs`), fetches the latest
///   L1 state and ensures that the stored L2 state matches the verified L1 state. Updates
///   the record of the latest stored L1 range accordingly.
///
/// # Arguments
/// * `beerus` - The initialized Beerus client containing all context for storage and network access.
/// * `gateway_state` - Mutable reference to the last known Starknet gateway state being tracked.
///
/// * `verified_state` - Mutable reference to the last known verified Starknet on-chain state.
///
/// * `l1_sync_check` - Mutable reference to the timestamp of the last L1 sync check.
/// * `l1_poll_secs` - Number of seconds between L1 state verification rounds.
///
/// # Errors
/// Returns an `eyre::Error` if any step in the synchronization process fails.
///
/// # Panics
/// Panics if the stored L2 block hash does not match the newly fetched L1 state.
async fn execute_sync(
    beerus: &Client<Http, SqlStorageProvider>,
    gateway_state: &mut GatewayState,
    verified_state: &mut State,
    l1_sync_check: &mut Instant,
    l1_poll_secs: u64,
) -> eyre::Result<()> {
    // Fetch the most recent gateway state from the Starknet feeder
    let update = beerus.get_latest_gateway_state().await?;

    // Synchronize all missing blocks between the current gateway_state and the latest update.
    while update.block_number > gateway_state.block_number {
        // If we're one block behind, use the update directly; otherwise, fetch the next block in sequence.
        *gateway_state =
            if gateway_state.block_number + 1 == update.block_number {
                update.clone()
            } else {
                beerus.get_gateway_state(gateway_state.block_number + 1).await?
            };
        // Attempt to update the verified state for each intermediate block,
        // retrying if necessary using with_retry for resilience.
        *verified_state = with_retry(|| async {
            beerus
                .verify_and_update_state(
                    &gateway_state.block_hash,
                    Some(verified_state.block_hash.clone()),
                )
                .await
        })
        .await?;
    }

    // Sync L1 state and verify stored L2 state
    if l1_sync_check.elapsed().as_secs() >= l1_poll_secs {
        *l1_sync_check = Instant::now();
        let l1_state = beerus.l1().get_l1_state().await?;
        let stored_state =
            beerus.storage().read_state(l1_state.block_number).await?;

        assert_eq!(
            stored_state.block_hash, l1_state.block_hash,
            "The state committed on L1 does not match the state the light client processed at block {}. L1 hash: {}, L2 hash: {}",
            l1_state.block_number, l1_state.block_hash, stored_state.block_hash
        );

        beerus.store_latest_l1_range(&l1_state).await?;
    }
    Ok(())
}
