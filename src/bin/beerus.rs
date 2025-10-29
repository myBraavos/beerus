use std::{sync::Arc, time::Duration};

use beerus::{
    client::{Client, Http},
    config::ServerConfig,
    storage::sql_storage_provider::SqlStorageProvider,
};
use validator::Validate;

#[cfg(not(tarpaulin_include))] // exclude from code-coverage report
#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();

    let config = get_config().await?;

    let http = Http::new();
    let storage =
        Arc::new(SqlStorageProvider::new(&config.client.database_url).await?);
    let beerus = Client::new(&config.client, http, storage).await?;

    {
        let period = Duration::from_secs(config.poll_secs);
        tokio::spawn(async move {
            // Find initial state to start syncing from
            let latest_stored_state =
                beerus.storage().read_latest_state().await;
            let (latest_stored_block, latest_stored_hash) =
                match latest_stored_state {
                    Ok(state) => (state.block_number, Some(state.block_hash)),
                    Err(_) => (0, None),
                };
            let mut tick = tokio::time::interval(period);
            // FIXME: handle all 'unwrap's
            let l1_state = beerus.l1().get_l1_state().await.unwrap();
            let (from_block, prev_hash) =
                if l1_state.block_number > latest_stored_block {
                    tracing::info!(
                        "Staring the sync from L1 block {}",
                        l1_state.block_number
                    );
                    beerus.storage().write_state(&l1_state).await.unwrap();
                    (l1_state.block_number + 1, Some(l1_state.block_hash))
                } else {
                    tracing::info!(
                        "Staring the sync from block {}",
                        latest_stored_block
                    );
                    (latest_stored_block + 1, latest_stored_hash)
                };
            let mut gateway_state =
                beerus.get_gateway_state(from_block).await.unwrap();
            let mut verified_state = beerus
                .get_verified_state(&gateway_state.block_hash, prev_hash)
                .await
                .unwrap();
            loop {
                tick.tick().await;
                match beerus.get_latest_gateway_state().await {
                    Ok(update) => {
                        // sync all intermediate blocks
                        while update.block_number - 1
                            > gateway_state.block_number
                        {
                            gateway_state = beerus
                                .get_gateway_state(
                                    gateway_state.block_number + 1,
                                )
                                .await
                                .unwrap();
                            verified_state = beerus
                                .get_verified_state(
                                    &gateway_state.block_hash,
                                    Some(verified_state.block_hash),
                                )
                                .await
                                .unwrap();
                        }
                        if update.block_number != gateway_state.block_number {
                            gateway_state = update.clone();
                            // FIXME: block may be not available
                            verified_state = beerus
                                .get_verified_state(
                                    &gateway_state.block_hash,
                                    Some(verified_state.block_hash),
                                )
                                .await
                                .unwrap();
                        }
                    }
                    Err(e) => {
                        tracing::error!(error=%e, "state update failed");
                    }
                }
            }
        });
    }

    // FIXME: should use the same client as the one used for syncing?
    let http = Http::new();
    let storage =
        Arc::new(SqlStorageProvider::new(&config.client.database_url).await?);
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
