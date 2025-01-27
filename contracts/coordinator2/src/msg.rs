use cosmwasm_schema::{cw_serde, QueryResponses};

/* Initiate */
#[cw_serde]
pub struct InstantiateMsg {
    pub chain_num: u16,
}

/* Execute */
#[cw_serde]
pub enum ExecuteMsg {
    AddStatusVote { vote: StatusVote },
    AddValidityVote { vote: ValidityVote },
}

/* Query */
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(OpeningVotesResp)]
    OpeningVotes {},
    #[returns(ClosedVotesResp)]
    ClosedVotes {},
    #[returns(AllClosedVotesResp)]
    AllClosedVotes {},
    #[returns(MyLogsResp)]
    MyLogs{},
}

#[cw_serde]
pub struct OpeningVotesResp {
    pub r1_votes: Vec<(u32, Vec<u16>)>,
    pub r2_votes: Vec<(u32, (Vec<u32>, Vec<u16>))>,
}

#[cw_serde]
pub struct ClosedVotesResp {
    pub votes: Vec<(u32, bool)>,
}

#[cw_serde]
pub struct AllClosedVotesResp {
    pub r1_votes: Vec<(u32, bool)>,
    pub r2_votes: Vec<(u32, bool)>,
}

#[cw_serde]
pub struct MyLogsResp {
    pub logs: String,
}

// received from the application chain
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

// send to the application chain
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