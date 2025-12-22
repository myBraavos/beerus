use beerus::client::{Client, Http};
use beerus::config::Config;
use beerus::gen::{Address, Felt};
use beerus::r#gen::{
    BroadcastedInvokeTxn, BroadcastedTxn, DaMode, InvokeTxn, InvokeTxnV3,
    InvokeTxnV3Type, InvokeTxnV3Version, ResourceBounds, ResourceBoundsMapping,
    SimulationFlag, U128, U64,
};
use beerus::storage::sql_storage_provider::SqlStorageProvider;
use eyre::Result;
use std::sync::Arc;

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

    let state = beerus.verify_and_update_state(
        &Felt::try_new("0x4f5fd556dd1fb7ece5c6ae031221f6c96321763cf5308de77bc0c1c4333d9ed")?,
        Some(Felt::try_new("0x6acfe9f4b8087025be737085e2417746482b4258a635ed680b2ba17098cefe0")?),
    ).await?;

    let client = beerus::gen::client::blocking::Client::new(
        &beerus.starknet().await.url,
        beerus.http().clone(),
    );

    let simulation_flags =
        vec![SimulationFlag::SkipValidate, SimulationFlag::SkipFeeCharge];

    let transactions = vec![BroadcastedTxn::BroadcastedInvokeTxn(BroadcastedInvokeTxn(InvokeTxn::InvokeTxnV3(InvokeTxnV3 {
        account_deployment_data: vec![],
        calldata: vec![
            Felt::try_new("0x1")?,
            Felt::try_new("0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d")?,
            Felt::try_new("0x83afd3f4caedc6eebf44246fe54e38c95e3179a5ec9ea81740eca5b482d12e")?,
            Felt::try_new("0x3")?,
            Felt::try_new("0x3e9321c77973bdfb918026b1c5ca451e4545176b9564d1677d0767977815342")?,
            Felt::try_new("0x2386f26fc10000")?,
            Felt::try_new("0x0")?,
        ],
        fee_data_availability_mode: DaMode::L1,
        nonce: Felt::try_new("0x5")?,
        nonce_data_availability_mode: DaMode::L1,
        paymaster_data: vec![],
        r#type: InvokeTxnV3Type::Invoke,
        resource_bounds: ResourceBoundsMapping{
            l1_gas: ResourceBounds{
                max_amount: U64::try_new("0x0")?,
                max_price_per_unit: U128::try_new("0x0")?,
            },
            l2_gas: ResourceBounds{
                max_amount: U64::try_new("0x0")?,
                max_price_per_unit: U128::try_new("0x0")?,
            },
            l1_data_gas: ResourceBounds{
                max_amount: U64::try_new("0x0")?,
                max_price_per_unit: U128::try_new("0x0")?,
            },
        },
        sender_address: Address(Felt::try_new("0x7ceab46cb84c112f1aa2b78f5a688d96a17060e462584bc783fb4f4f89f9405")?),
        signature: vec![
            Felt::try_new("0x1")?,
            Felt::try_new("0x0")?,
            Felt::try_new("0x0")?,
        ],
        tip: U64::try_new("0x0")?,
        version: InvokeTxnV3Version::V0x100000000000000000000000000000003,
    })))];

    let res = beerus::exe::simulate(
        client,
        transactions,
        simulation_flags,
        state,
        &beerus.gas_prices(),
        beerus.rate_limiter(),
        beerus.settings(),
    )?;

    tracing::info!("{res:#?}");

    Ok(())
}
