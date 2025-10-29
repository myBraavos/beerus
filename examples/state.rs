use std::sync::Arc;

use beerus::client::{Client, Http};
use beerus::config::Config;
use beerus::storage::sql_storage_provider::SqlStorageProvider;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config {
        starknet_rpc: format!(
            "https://starknet-mainnet.public.blastapi.io/rpc/v0_9"
        ),
        gateway_url: format!("https://feeder.alpha-mainnet.starknet.io"),
        database_url: format!(
            "postgresql://myuser:mypassword@localhost:5432/beerus"
        ),
    };

    let http = Http::new();
    let storage =
        Arc::new(SqlStorageProvider::new(&config.database_url).await?);
    let beerus = Client::new(&config, http, storage).await?;

    let gateway_state = beerus.get_latest_gateway_state().await?;
    let state =
        beerus.get_verified_state(&gateway_state.block_hash, None).await?;
    tracing::info!("synced: {state:#?}");
    let state = beerus.storage().read_latest_state().await?;
    tracing::info!("read: {state:#?}");

    Ok(())
}
