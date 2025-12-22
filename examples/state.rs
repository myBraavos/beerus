use std::sync::Arc;

use beerus::client::{Client, Http};
use beerus::config::Config;
use beerus::storage::sql_storage_provider::SqlStorageProvider;
use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config {
        eth_rpc: format!("https://eth-mainnet.public.blastapi.io"),
        starknet_rpc: format!(
            "https://starknet-mainnet.public.blastapi.io/rpc/v0_9"
        ),
        gateway_url: format!("https://feeder.alpha-mainnet.starknet.io"),
        database_url: format!(
            "postgresql://{}:{}@localhost:5432/beerus",
            std::env::var("POSTGRES_USER")
                .unwrap_or_else(|_| "postgres".to_string()),
            std::env::var("POSTGRES_PASSWORD")
                .unwrap_or_else(|_| "postgres".to_string()),
        ),
        l2_rate_limit: 10,
        l1_range_blocks: 9,
        disable_background_loader: true,
        validate_historical_blocks: true,
    };

    let http = Http::new();
    let storage =
        Arc::new(SqlStorageProvider::new(&config.database_url).await?);
    let beerus = Client::new(&config, http, storage).await?;

    let gateway_state = beerus.get_latest_gateway_state().await?;
    let state =
        beerus.verify_and_update_state(&gateway_state.block_hash, None).await?;
    tracing::info!("synced: {state:#?}");
    Ok(())
}
