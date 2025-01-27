use cosmwasm_schema::{cw_serde, QueryResponses};

/* Initiate */
#[cw_serde]
pub struct InstantiateMsg {
    pub chain_num: u16,
}

/* Execute */
#[cw_serde]
pub enum ExecuteMsg {
    AddVote { vote: Vote },
}

#[cw_serde]
pub struct Vote{
    pub tx_id: u32,
    pub chain_id: u16,
    pub success: bool,
}

/* Query */
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(OpeningVotesResp)]
    OpeningVotes {},
    #[returns(ClosedVotesResp)]
    ClosedVotes {},
    #[returns(MyLogsResp)]
    MyLogs{},
}

#[cw_serde]
pub struct OpeningVotesResp {
    pub votes: Vec<(u32, Vec<u16>)>,
}

#[cw_serde]
pub struct ClosedVotesResp {
    pub votes: Vec<(u32, bool)>,
}

#[cw_serde]
pub struct MyLogsResp {
    pub logs: String,
}

#[cw_serde]
pub struct Instruction{
    pub tx_id: u32,
    pub commitment: bool,
}