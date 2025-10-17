use beerus::client::{Client, Http};
use beerus::config::Config;
use beerus::gen::{Address, Felt, FunctionCall};
use eyre::{Result};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config {
        starknet_rpc: format!(
            "https://starknet-mainnet.public.blastapi.io/rpc/v0_9"
        ),
        data_dir: "tmp".to_owned(),
    };

    let http = Http::new();
    let beerus = Client::new(&config, http).await?;

    let calldata = FunctionCall {
        contract_address: Address(Felt::try_new(
            "0x060e91c92fdad9e7245b9bb4e143b880e4e9354d0b95c5c2d33dc347dded3bf0",
        )?),
        entry_point_selector: Felt::try_new(
            "0x35a73cd311a05d46deda634c5ee045db92f811b4e74bca4437fcb5302b7af33",
        )?,
        calldata: vec![Felt::try_new("0x03884d24eebb32c1cc8f3a03781a6963ec85bcc3fa69fab9189fb4b359bf4259")?],
    };
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

    let state = beerus.get_state().await?;
    let res = beerus.execute(calldata, state)?;
    tracing::info!("{res:#?}");

    Ok(())
}
