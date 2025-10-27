use beerus::client::{Client, Http};
use beerus::config::Config;
use beerus::gen::{BlockId, BlockTag};
use eyre::{Result};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config {
        starknet_rpc: format!(
            "https://starknet-mainnet.public.blastapi.io/rpc/v0_9"
        ),
        gateway_url: format!(
            "https://feeder.alpha-mainnet.starknet.io"
        ),
        data_dir: "tmp".to_owned(),
    };

    let http = Http::new();
    let beerus = Client::new(&config, http).await?;

    let gateway_state = beerus.get_gateway_state(BlockId::BlockTag(BlockTag::Latest)).await?;
    let state = beerus.get_verified_state(
        &beerus::r#gen::BlockHash(gateway_state.block_hash),
        None,
    ).await?;
    tracing::info!("{state:#?}");

    Ok(())
}
