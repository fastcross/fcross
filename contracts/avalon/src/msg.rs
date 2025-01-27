use cosmwasm_schema::{cw_serde, QueryResponses};

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
    PrepareTx { instruction: StatusInstruction },
    FinalizeTx { instruction: ValidityInstruction },
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

/* Query */
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(HistoryTxsResp)]
    HistoryTxs {},
    #[returns(PendingTxsResp)]
    PendingTxs {},
    #[returns(WaitingTxsResp)]
    WaitingTxs {},
    #[returns(MyErrLogsResp)]
    MyErrLogs{},
    #[returns(MyTimeLogsResp)]
    MyTimeLogs{},
}

#[cw_serde]
pub struct HistoryTxsResp {
    pub history: Vec<(u32, bool)>,
}

#[cw_serde]
pub struct PendingTxsResp {
    pub pending_txs: Vec<(FcrossTx, Option<i64>, bool, bool)>,
}


#[cw_serde]
pub struct MultifutureResp {
    pub futures: Vec<String>,
}

#[cw_serde]
pub struct AllMfsResp {
    pub mfs: Vec<(u32, String)>,
}

#[cw_serde]
pub struct WaitingTxsResp {
    pub waiting_txs: Vec<FcrossTx>,
}

#[cw_serde]
pub struct MyErrLogsResp {
    pub logs: String,
}

#[cw_serde]
pub struct MyTimeLogsResp {
    pub logs: Vec<String>,
}

// received from the coordinator chain
#[cw_serde]
pub enum Instruction {
    Status(StatusInstruction),
    Validity(ValidityInstruction),
}

#[cw_serde]
pub struct StatusInstruction{
    pub tx_id: u32,
    pub advancement: bool,
}

#[cw_serde]
pub struct ValidityInstruction{
    pub tx_id: u32,
    pub commitment: bool,
}

// send to the coordinator chain
#[cw_serde]
pub enum Vote{
    Status(StatusVote),
    Validity(ValidityVote),
}

#[cw_serde]
pub struct StatusVote{
    pub tx_id: u32,
    pub chain_id: u16,
    pub status: bool,
}

#[cw_serde]
pub struct ValidityVote{
    pub tx_id: u32,
    pub chain_id: u16,
    pub dependencies: Vec<u32>,
}


