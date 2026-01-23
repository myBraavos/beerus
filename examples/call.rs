use beerus::client::{Client, Http};
use beerus::config::Config;
use beerus::gen::{Address, Felt, FunctionCall};
use beerus::storage::sql_storage_provider::SqlStorageProvider;
use eyre::Result;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config {
        eth_rpc: format!("https://eth-mainnet.public.blastapi.io"),
        starknet_rpc: format!(
            "https://starknet-mainnet.public.blastapi.io/rpc/v0_10"
        ),
        gateway_url: format!("https://feeder.alpha-mainnet.starknet.io"),
        database_url: format!(
            "postgresql://postgres:postgres@localhost:5432/beerus"
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

    let calldata = FunctionCall {
        contract_address: Address(Felt::try_new(
            "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
        )?),
        entry_point_selector: Felt::try_new(
            "0x361458367e696363fbcc70777d07ebbd2394e89fd0adcaf147faccd1d294d60",
        )?,
        calldata: vec![],
    };

    let state = beerus.verify_and_update_state(
        &Felt::try_new("0x7256dde30ae68f43f3def9ce2a4433dd3de11b630d4f84336891bad8fe4127e")?,
        Some(Felt::try_new("0x6084bda2cd3247aa11364404f7918001e82a7567cfe0b949fa6a7f3d4b4099f")?),
    ).await?;
    let res = beerus.execute(calldata, state)?;
    tracing::info!("{res:#?}");

    Ok(())
}
