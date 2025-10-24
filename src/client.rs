use eyre::Result;

use crate::config::Config;
use crate::gen::client::Client as StarknetClient;
use crate::gen::{gen, Felt, FunctionCall, Rpc, BlockId, BlockTag};

pub mod http;
pub mod state;
pub mod utils;

pub use http::Http;
pub use state::State;
pub use utils::as_felt;

const RPC_SPEC_VERSION: &str = "0.9.0";

/// Main client for interacting with Starknet
pub struct Client<
    T: gen::client::HttpClient
        + gen::client::blocking::HttpClient
        + Clone
        + 'static,
> {
    starknet: StarknetClient<T>,
    http: T,
}

impl<
        T: gen::client::HttpClient
            + gen::client::blocking::HttpClient
            + Clone
            + 'static,
    > Client<T>
{
    /// Create a new client with the given configuration and HTTP client
    pub async fn new(config: &Config, http: T) -> Result<Self> {
        let starknet = StarknetClient::new(&config.starknet_rpc, http.clone());
        let rpc_spec_version = starknet.specVersion().await?;
        if rpc_spec_version != RPC_SPEC_VERSION {
            eyre::bail!("RPC spec version mismatch: expected {RPC_SPEC_VERSION} but got {rpc_spec_version}");
        }
        Ok(Self { starknet, http })
    }

    /// Get the underlying Starknet client
    pub fn starknet(&self) -> &StarknetClient<T> {
        &self.starknet
    }

    /// Get the HTTP client
    pub fn http(&self) -> &T {
        &self.http
    }

    /// Execute a function call on the Starknet state
    pub fn execute(
        &self,
        request: FunctionCall,
        state: State,
    ) -> Result<Vec<Felt>> {
        let client = gen::client::blocking::Client::new(
            &self.starknet.url,
            self.http.clone(),
        );
        let call_info = crate::exe::call(client, request, state)?;
        call_info
            .execution
            .retdata
            .0
            .into_iter()
            .map(|felt| as_felt(&felt.to_bytes_be()))
            .collect()
    }

    /// Get the current state of the blockchain
    pub async fn get_state(&self) -> Result<State> {
        let block_id = BlockId::BlockTag(BlockTag::Latest);
        let block = self.starknet.getBlockWithTxHashes(block_id).await?;
        let gen::GetBlockWithTxHashesResult::BlockWithTxHashes(block) = block
        else {
            eyre::bail!("Pending block received, which is not supported");
        };
        Ok(State::new(
            *block.block_header.block_number.as_ref() as u64,
            block.block_header.block_hash.0,
            block.block_header.new_root,
        ))
    }
}
