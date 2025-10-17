use beerus::client::{Client, Http};
use beerus::config::Config;
use eyre::{Result};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config {
        starknet_rpc: format!(
            "https://starknet-mainnet.public.blastapi.io/rpc/v0_7"
        ),
        data_dir: "tmp".to_owned(),
    };

    let http = Http::new();
    let beerus = Client::new(&config, http).await?;

    let state = beerus.get_state().await?;
    tracing::info!("{state:#?}");

    Ok(())
}
