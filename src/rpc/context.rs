use std::sync::Arc;

use iamgroot::jsonrpc;

use crate::r#gen::{
    GetBlockWithReceiptsResult, GetTransactionByBlockIdAndIndexIndex,
    TxnReceiptWithBlockInfo,
};
use crate::{
    client::{Client, State as ClientState},
    exe,
    gen::{
        self, AddDeclareTransactionResult, AddDeployAccountTransactionResult,
        AddInvokeTransactionResult, Address, BlockHashAndNumberResult, BlockId,
        BlockNumber, BlockTransactionTrace, BroadcastedDeclareTxn,
        BroadcastedDeployAccountTxn, BroadcastedInvokeTxn, BroadcastedTxn,
        ChainId, EventsChunk, FeeEstimate, Felt, FunctionCall,
        GetBlockTransactionCountResult, GetBlockWithTxHashesResult,
        GetBlockWithTxsResult, GetClassAtResult, GetClassResult,
        GetEventsFilter, GetProofResult, GetStateUpdateResult,
        GetTransactionByBlockIdAndIndexResult, GetTransactionByHashResult,
        GetTransactionStatusResult, MsgFromL1, SimulatedTransaction,
        SimulationFlag, SimulationFlagForEstimateFee, StorageKey,
        SyncingResult, TransactionTrace, TxGatewayStatus, TxnHash,
    },
};

/// RPC context containing the client and current state
#[derive(Clone)]
pub struct Context {
    pub client: Arc<Client<crate::client::http::Http>>,
    pub state: Arc<tokio::sync::RwLock<ClientState>>,
}

impl Context {
    /// Create a new RPC context
    pub fn new(client: Arc<Client<crate::client::http::Http>>) -> Self {
        Self {
            client,
            state: Arc::new(tokio::sync::RwLock::new(ClientState::default())),
        }
    }

    /// Get the current state
    pub async fn get_state(&self) -> ClientState {
        self.state.read().await.clone()
    }

    /// Update the current state
    pub async fn update_state(&self, new_state: ClientState) {
        *self.state.write().await = new_state;
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl gen::Rpc for Context {
    async fn getProof(
        &self,
        block_id: BlockId,
        contract_address: Address,
        keys: Vec<StorageKey>,
    ) -> Result<GetProofResult, jsonrpc::Error> {
        self.client.starknet().getProof(block_id, contract_address, keys).await
    }

    async fn getTxStatus(
        &self,
        transaction_hash: TxnHash,
    ) -> Result<TxGatewayStatus, jsonrpc::Error> {
        self.client.starknet().getTxStatus(transaction_hash).await
    }

    async fn version(&self) -> Result<String, jsonrpc::Error> {
        self.client.starknet().version().await
    }

    async fn getStateRoot(&self) -> Result<Felt, jsonrpc::Error> {
        self.client.starknet().getStateRoot().await
    }

    async fn addDeclareTransaction(
        &self,
        declare_transaction: BroadcastedDeclareTxn,
    ) -> Result<AddDeclareTransactionResult, jsonrpc::Error> {
        self.client.starknet().addDeclareTransaction(declare_transaction).await
    }

    async fn addDeployAccountTransaction(
        &self,
        deploy_account_transaction: BroadcastedDeployAccountTxn,
    ) -> Result<AddDeployAccountTransactionResult, jsonrpc::Error> {
        self.client
            .starknet()
            .addDeployAccountTransaction(deploy_account_transaction)
            .await
    }

    async fn addInvokeTransaction(
        &self,
        invoke_transaction: BroadcastedInvokeTxn,
    ) -> Result<AddInvokeTransactionResult, jsonrpc::Error> {
        self.client.starknet().addInvokeTransaction(invoke_transaction).await
    }

    async fn blockHashAndNumber(
        &self,
    ) -> Result<BlockHashAndNumberResult, jsonrpc::Error> {
        self.client.starknet().blockHashAndNumber().await
    }

    async fn blockNumber(&self) -> Result<BlockNumber, jsonrpc::Error> {
        self.client.starknet().blockNumber().await
    }

    async fn call(
        &self,
        request: FunctionCall,
        _block_id: BlockId, // TODO: use block_id to make a call on specific state
    ) -> Result<Vec<Felt>, jsonrpc::Error> {
        let state = self
            .client
            .get_state()
            .await
            .map_err(|e| jsonrpc::Error::new(-32602, e.to_string()))?;
        let client = gen::client::blocking::Client::new(
            &self.client.starknet().url,
            self.client.http().clone(),
        );
        let call_info = exe::call(client, request, state)
            .map_err(|e| jsonrpc::Error::new(-32602, e.to_string()))?;

        Ok(call_info
            .execution
            .retdata
            .0
            .into_iter()
            .map(|felt| Felt::try_new(&format!("0x{:x}", felt)).unwrap())
            .collect())
    }

    async fn chainId(&self) -> Result<ChainId, jsonrpc::Error> {
        self.client.starknet().chainId().await
    }

    async fn estimateFee(
        &self,
        request: Vec<BroadcastedTxn>,
        simulation_flags: Vec<SimulationFlagForEstimateFee>,
        block_id: BlockId,
    ) -> Result<Vec<FeeEstimate>, jsonrpc::Error> {
        self.client
            .starknet()
            .estimateFee(request, simulation_flags, block_id)
            .await
    }

    async fn estimateMessageFee(
        &self,
        message: MsgFromL1,
        block_id: BlockId,
    ) -> Result<FeeEstimate, jsonrpc::Error> {
        self.client.starknet().estimateMessageFee(message, block_id).await
    }

    async fn getBlockTransactionCount(
        &self,
        block_id: BlockId,
    ) -> Result<GetBlockTransactionCountResult, jsonrpc::Error> {
        self.client.starknet().getBlockTransactionCount(block_id).await
    }

    async fn getBlockWithReceipts(
        &self,
        block_id: BlockId,
    ) -> Result<GetBlockWithReceiptsResult, jsonrpc::Error> {
        self.client.starknet().getBlockWithReceipts(block_id).await
    }

    async fn getBlockWithTxHashes(
        &self,
        block_id: BlockId,
    ) -> Result<GetBlockWithTxHashesResult, jsonrpc::Error> {
        self.client.starknet().getBlockWithTxHashes(block_id).await
    }

    async fn getBlockWithTxs(
        &self,
        block_id: BlockId,
    ) -> Result<GetBlockWithTxsResult, jsonrpc::Error> {
        self.client.starknet().getBlockWithTxs(block_id).await
    }

    async fn getClass(
        &self,
        block_id: BlockId,
        class_hash: Felt,
    ) -> Result<GetClassResult, jsonrpc::Error> {
        self.client.starknet().getClass(block_id, class_hash).await
    }

    async fn getClassAt(
        &self,
        block_id: BlockId,
        contract_address: Address,
    ) -> Result<GetClassAtResult, jsonrpc::Error> {
        self.client.starknet().getClassAt(block_id, contract_address).await
    }

    async fn getClassHashAt(
        &self,
        block_id: BlockId,
        contract_address: Address,
    ) -> Result<Felt, jsonrpc::Error> {
        self.client.starknet().getClassHashAt(block_id, contract_address).await
    }

    async fn getEvents(
        &self,
        filter: GetEventsFilter,
    ) -> Result<EventsChunk, jsonrpc::Error> {
        self.client.starknet().getEvents(filter).await
    }

    async fn getNonce(
        &self,
        block_id: BlockId,
        contract_address: Address,
    ) -> Result<Felt, jsonrpc::Error> {
        self.client.starknet().getNonce(block_id, contract_address).await
    }

    async fn getStateUpdate(
        &self,
        block_id: BlockId,
    ) -> Result<GetStateUpdateResult, jsonrpc::Error> {
        self.client.starknet().getStateUpdate(block_id).await
    }

    async fn getStorageAt(
        &self,
        contract_address: Address,
        key: StorageKey,
        block_id: BlockId,
    ) -> Result<Felt, jsonrpc::Error> {
        self.client
            .starknet()
            .getStorageAt(contract_address, key, block_id)
            .await
    }

    async fn getTransactionByHash(
        &self,
        transaction_hash: TxnHash,
    ) -> Result<GetTransactionByHashResult, jsonrpc::Error> {
        self.client.starknet().getTransactionByHash(transaction_hash).await
    }

    async fn getTransactionByBlockIdAndIndex(
        &self,
        block_id: BlockId,
        index: GetTransactionByBlockIdAndIndexIndex,
    ) -> Result<GetTransactionByBlockIdAndIndexResult, jsonrpc::Error> {
        self.client
            .starknet()
            .getTransactionByBlockIdAndIndex(block_id, index)
            .await
    }

    async fn getTransactionReceipt(
        &self,
        transaction_hash: TxnHash,
    ) -> Result<TxnReceiptWithBlockInfo, jsonrpc::Error> {
        self.client.starknet().getTransactionReceipt(transaction_hash).await
    }

    async fn getTransactionStatus(
        &self,
        transaction_hash: TxnHash,
    ) -> Result<GetTransactionStatusResult, jsonrpc::Error> {
        self.client.starknet().getTransactionStatus(transaction_hash).await
    }

    async fn simulateTransactions(
        &self,
        block_id: BlockId,
        transactions: Vec<BroadcastedTxn>,
        simulation_flags: Vec<SimulationFlag>,
    ) -> Result<Vec<SimulatedTransaction>, jsonrpc::Error> {
        self.client
            .starknet()
            .simulateTransactions(block_id, transactions, simulation_flags)
            .await
    }

    async fn specVersion(&self) -> Result<String, jsonrpc::Error> {
        self.client.starknet().specVersion().await
    }

    async fn syncing(&self) -> Result<SyncingResult, jsonrpc::Error> {
        self.client.starknet().syncing().await
    }

    async fn traceBlockTransactions(
        &self,
        block_id: BlockId,
    ) -> Result<Vec<BlockTransactionTrace>, jsonrpc::Error> {
        self.client.starknet().traceBlockTransactions(block_id).await
    }

    async fn traceTransaction(
        &self,
        transaction_hash: TxnHash,
    ) -> Result<TransactionTrace, jsonrpc::Error> {
        self.client.starknet().traceTransaction(transaction_hash).await
    }
}
