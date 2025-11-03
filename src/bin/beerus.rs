use std::{sync::Arc, time::Duration};

use beerus::{
    client::{state::GatewayState, Client, Http, State},
    config::ServerConfig,
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
    let beerus =
        Client::new(&config.client, http.clone(), storage.clone()).await?;

    {
        let period = Duration::from_secs(config.poll_secs);
        tokio::spawn(async move {
            let (
                mut tick,
                mut l1_sync_check,
                mut gateway_state,
                mut verified_state,
            ) = match prepare_main_loop(&beerus, period).await {
                Ok((tick, l1_sync_check, gateway_state, verified_state)) => {
                    (tick, l1_sync_check, gateway_state, verified_state)
                }
                Err(e) => {
                    tracing::error!(error=%e, "failed to prepare main loop, exiting sync task");
                    return;
                }
            };
            loop {
                tick.tick().await;
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

    let beerus = Client::new(&config.client, http, storage).await?;
    let server = beerus::rpc::Server::new(Arc::new(beerus));
    beerus::rpc::serve_on(server, &config.rpc_addr.to_string()).await.unwrap();
    tracing::info!("rpc server started");
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

async fn prepare_main_loop(
    beerus: &Client<Http, SqlStorageProvider>,
    period: Duration,
) -> eyre::Result<(tokio::time::Interval, Instant, GatewayState, State)> {
    // Find initial state to start syncing from
    let latest_stored_state = beerus.storage().read_latest_state().await;
    let (latest_stored_block, latest_stored_hash) = match latest_stored_state {
        Ok(state) => (state.block_number, Some(state.block_hash)),
        Err(_) => (0, None),
    };

    let l1_state = beerus.l1().get_l1_state().await?;
    let (from_block, prev_hash) = if l1_state.block_number > latest_stored_block
    {
        tracing::info!(
            "Staring the sync from L1 block {}",
            l1_state.block_number
        );
        beerus.storage().write_state(&l1_state).await?;
        beerus.store_latest_l1_range(&l1_state).await?;
        (l1_state.block_number + 1, Some(l1_state.block_hash))
    } else {
        tracing::info!("Staring the sync from block {}", latest_stored_block);
        (latest_stored_block + 1, latest_stored_hash)
    };
    let gateway_state = beerus.get_gateway_state(from_block).await?;
    let verified_state =
        beerus.get_verified_state(&gateway_state.block_hash, prev_hash).await?;

    // sync periods
    let tick = tokio::time::interval(period);
    let l1_sync_check = Instant::now();

    Ok((tick, l1_sync_check, gateway_state, verified_state))
}

async fn execute_sync(
    beerus: &Client<Http, SqlStorageProvider>,
    gateway_state: &mut GatewayState,
    verified_state: &mut State,
    l1_sync_check: &mut Instant,
    l1_poll_secs: u64,
) -> eyre::Result<()> {
    let update = beerus.get_latest_gateway_state().await?;
    // sync all intermediate blocks
    while update.block_number > gateway_state.block_number {
        *gateway_state =
            if gateway_state.block_number + 1 == update.block_number {
                update.clone()
            } else {
                beerus.get_gateway_state(gateway_state.block_number + 1).await?
            };
        *verified_state = with_retry(|| async {
            beerus
                .get_verified_state(
                    &gateway_state.block_hash,
                    Some(verified_state.block_hash.clone()),
                )
                .await
        })
        .await?;
    }

    // sync L1 state and verify stored L2 state
    if l1_sync_check.elapsed().as_secs() >= l1_poll_secs {
        *l1_sync_check = Instant::now();
        let l1_state = beerus.l1().get_l1_state().await?;
        let stored_state =
            beerus.storage().read_state(l1_state.block_number).await?;
        assert_eq!(
            stored_state.block_hash, l1_state.block_hash,
            "Stored L2 state does not match L1 state"
        );
        beerus.store_latest_l1_range(&l1_state).await?;
    }
    Ok(())
}
