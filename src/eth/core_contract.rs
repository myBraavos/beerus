use alloy::{
    primitives::{address, Address},
    providers::{Provider, ProviderBuilder},
    rpc::types::Filter,
    sol,
};
use eyre::Result;

use crate::client::State;

const CORE_CONTRACT_ADDRESS: Address =
    address!("0xc662c410c0ecf747543f5ba90660f6abebd9c8c4");

sol!(
    #[sol(rpc)]
    contract StarknetCore {
        function stateRoot() external view returns (uint256);
        function stateBlockNumber() external view returns (int256);
        function stateBlockHash() external view returns (uint256);
        event LogStateUpdate(uint256 globalRoot, int256 blockNumber, uint256 blockHash);
    }
);

pub struct L1CoreContract {
    rpc_url: String,
}

impl L1CoreContract {
    pub fn new(rpc_url: &str) -> Self {
        Self { rpc_url: rpc_url.to_string() }
    }

    pub async fn get_l1_state(&self) -> Result<State> {
        let provider =
            ProviderBuilder::new().connect_http(self.rpc_url.parse().unwrap());

        let contract = StarknetCore::new(CORE_CONTRACT_ADDRESS, &provider);

        let multicall = provider
            .multicall()
            .add(contract.stateRoot())
            .add(contract.stateBlockNumber())
            .add(contract.stateBlockHash());

        let (root, block_number, block_hash) = multicall.aggregate().await?;

        Ok(State::new(
            block_number.as_i64(),
            block_hash.try_into()?,
            root.try_into()?,
        ))
    }

    pub async fn get_l1_state_updates(
        &self,
        start_block: u64,
        end_block: u64,
    ) -> Result<()> {
        let provider =
            ProviderBuilder::new().connect_http(self.rpc_url.parse().unwrap());

        // let contract = StarknetCore::new(CORE_CONTRACT_ADDRESS, &provider);

        let filter = Filter::new()
            .address(CORE_CONTRACT_ADDRESS)
            .from_block(start_block)
            .to_block(end_block)
            .event("LogStateUpdate(uint256,int256,uint256)");

        let logs = provider.get_logs(&filter).await?;

        // TODO: parse and return response
        println!("logs: {:?}", logs);

        Ok(())
    }
}
