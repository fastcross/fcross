use cosmwasm_schema::{cw_serde, QueryResponses};

use crate::state::ScopeExecutedTx;

/* Initiate */
#[cw_serde]
pub struct InstantiateMsg {
    pub chain_id: u16,
    pub original_value: i64,
}

/* Execute */
#[cw_serde]
pub enum ExecuteMsg {
    ExecuteTxs { fcross_txs: Vec<FcrossTx> },
    FinalizeTx { instruction: Instruction },
}

#[cw_serde]
pub struct FcrossTx{
    pub tx_id: u32,
    pub operation: Operation,
}

#[cw_serde]
pub enum Operation {
    CreditBalance { amount: i64 },
    DebitBalance { amount: i64 },
}

#[cw_serde]
pub struct Instruction{
    pub tx_id: u32,
    pub commitment: bool,
}

/* Query */
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(MultifutureResp)]
    Multifuture {},
    #[returns(WaitingListResp)]
    WaitingList {},
    #[returns(AllListsResp)]
    AllLists {},
    #[returns(MyErrLogsResp)]
    MyErrLogs{},
    #[returns(MyTimeLogsResp)]
    MyTimeLogs{},
}

#[cw_serde]
pub struct MultifutureResp {
    pub futures: Vec<String>,
}

#[cw_serde]
pub struct WaitingListResp {
    pub waiting_tx_list: Vec<FcrossTx>,
}

#[cw_serde]
pub struct AllListsResp {
    pub history_list: Vec<u32>,
    pub pending_list: Vec<(u32, bool)>,
    pub secondary_pending_list: Vec<ScopeExecutedTx>,
    pub waiting_list: Vec<FcrossTx>,
}

#[cw_serde]
pub struct MyErrLogsResp {
    pub logs: String,
}

#[cw_serde]
pub struct MyTimeLogsResp {
    pub logs: Vec<String>,
}

// send from coordinator chain
#[cw_serde]
pub struct Vote{
    pub tx_id: u32,
    pub chain_id: u16,
    pub success: bool,
}

