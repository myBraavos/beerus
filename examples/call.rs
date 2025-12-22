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

    let calldata = FunctionCall {
        contract_address: Address(Felt::try_new(
            "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
        )?),
        entry_point_selector: Felt::try_new(
            "0x361458367e696363fbcc70777d07ebbd2394e89fd0adcaf147faccd1d294d60",
        )?,
        calldata: vec![],
    };

    // vesu
    // let calldata = FunctionCall {
    //     contract_address: Address(Felt::try_new(
    //         "0x060e91c92fdad9e7245b9bb4e143b880e4e9354d0b95c5c2d33dc347dded3bf0",
    //     )?),
    //     entry_point_selector: Felt::try_new(
    //         "0x35a73cd311a05d46deda634c5ee045db92f811b4e74bca4437fcb5302b7af33",
    //     )?,
    //     calldata: vec![Felt::try_new("0x03884d24eebb32c1cc8f3a03781a6963ec85bcc3fa69fab9189fb4b359bf4259")?],
    // };

    // multicall
    // let calldata = FunctionCall {
    //     contract_address: Address(Felt::try_new(
    //         "0x4754444977069c834be932a6dd2119c1d42d935ffce9d6af4e5d5d6ff8cc449",
    //     )?),
    //     entry_point_selector: Felt::try_new(
    //         "0x24c7ee658acc0eb4da5d128b6f216a0156f1bcd4e92f63e949b495a3be3772f",
    //     )?,
    //     calldata: vec![
    //         Felt::try_new("0xC")?,

    //         Felt::try_new("0x6d507cf5c751a6569d3a10447aee58f9b1410bb6a7d9c52d22875cd5377b29")?,
    //         Felt::try_new("0x361458367e696363fbcc70777d07ebbd2394e89fd0adcaf147faccd1d294d60")?,
    //         Felt::try_new("0x0")?,
    //         Felt::try_new("0x6d507cf5c751a6569d3a10447aee58f9b1410bb6a7d9c52d22875cd5377b29")?,
    //         Felt::try_new("0x216b05c387bab9ac31918a3e61672f4618601f3c598a2f3f2710f37053e1ea4")?,
    //         Felt::try_new("0x0")?,
    //         Felt::try_new("0x6d507cf5c751a6569d3a10447aee58f9b1410bb6a7d9c52d22875cd5377b29")?,
    //         Felt::try_new("0x4c4fb1ab068f6039d5780c68dd0fa2f8742cceb3426d19667778ca7f3518a9")?,
    //         Felt::try_new("0x0")?,
    //         Felt::try_new("0x6d507cf5c751a6569d3a10447aee58f9b1410bb6a7d9c52d22875cd5377b29")?,
    //         Felt::try_new("0x2e4263afad30923c891518314c3c95dbe830a16874e8abc5777a9a20b54c76e")?,
    //         Felt::try_new("0x1")?,
    //         Felt::try_new("0x3884d24eebb32c1cc8f3a03781a6963ec85bcc3fa69fab9189fb4b359bf4259")?,
    //         Felt::try_new("0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7")?,
    //         Felt::try_new("0x361458367e696363fbcc70777d07ebbd2394e89fd0adcaf147faccd1d294d60")?,
    //         Felt::try_new("0x0")?,
    //         Felt::try_new("0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7")?,
    //         Felt::try_new("0x216b05c387bab9ac31918a3e61672f4618601f3c598a2f3f2710f37053e1ea4")?,
    //         Felt::try_new("0x0")?,
    //         Felt::try_new("0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7")?,
    //         Felt::try_new("0x4c4fb1ab068f6039d5780c68dd0fa2f8742cceb3426d19667778ca7f3518a9")?,
    //         Felt::try_new("0x0")?,
    //         Felt::try_new("0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7")?,
    //         Felt::try_new("0x2e4263afad30923c891518314c3c95dbe830a16874e8abc5777a9a20b54c76e")?,
    //         Felt::try_new("0x1")?,
    //         Felt::try_new("0x3884d24eebb32c1cc8f3a03781a6963ec85bcc3fa69fab9189fb4b359bf4259")?,
    //         Felt::try_new("0x57912720381af14b0e5c87aa4718ed5e527eab60b3801ebf702ab09139e38b")?,
    //         Felt::try_new("0x361458367e696363fbcc70777d07ebbd2394e89fd0adcaf147faccd1d294d60")?,
    //         Felt::try_new("0x0")?,
    //         Felt::try_new("0x57912720381af14b0e5c87aa4718ed5e527eab60b3801ebf702ab09139e38b")?,
    //         Felt::try_new("0x216b05c387bab9ac31918a3e61672f4618601f3c598a2f3f2710f37053e1ea4")?,
    //         Felt::try_new("0x0")?,
    //         Felt::try_new("0x57912720381af14b0e5c87aa4718ed5e527eab60b3801ebf702ab09139e38b")?,
    //         Felt::try_new("0x4c4fb1ab068f6039d5780c68dd0fa2f8742cceb3426d19667778ca7f3518a9")?,
    //         Felt::try_new("0x0")?,
    //         Felt::try_new("0x57912720381af14b0e5c87aa4718ed5e527eab60b3801ebf702ab09139e38b")?,
    //         Felt::try_new("0x2e4263afad30923c891518314c3c95dbe830a16874e8abc5777a9a20b54c76e")?,
    //         Felt::try_new("0x1")?,
    //         Felt::try_new("0x3884d24eebb32c1cc8f3a03781a6963ec85bcc3fa69fab9189fb4b359bf4259")?
    //     ],
    // };
    // cairo 0
    // let calldata = FunctionCall {
    //     contract_address: Address(Felt::try_new(
    //         "0x041fd22b238fa21cfcf5dd45a8548974d8263b3a531a60388411c5e230f97023",
    //     )?),
    //     entry_point_selector: Felt::try_new(
    //         "0x22db07240efd55449143fc6832df9627bb9e91ae79dd25445baafc8a6106a9a",
    //     )?,
    //     calldata: vec![
    //         Felt::try_new("0x6B1BFF9")?,
    //         Felt::try_new("0x0")?,
    //         Felt::try_new("0x220C11")?,
    //         Felt::try_new("0x0")?,
    //         Felt::try_new("0x7D83")?,
    //         Felt::try_new("0x0")?,
    //     ],
    // };

    let state = beerus.verify_and_update_state(
        &Felt::try_new("0x7256dde30ae68f43f3def9ce2a4433dd3de11b630d4f84336891bad8fe4127e")?,
        Some(Felt::try_new("0x6084bda2cd3247aa11364404f7918001e82a7567cfe0b949fa6a7f3d4b4099f")?),
    ).await?;
    let res = beerus.execute(calldata, state)?;
    tracing::info!("{res:#?}");

    Ok(())
}
