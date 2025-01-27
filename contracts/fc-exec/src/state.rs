use cosmwasm_schema::cw_serde;
use cosmwasm_std::Timestamp;
use cw_storage_plus::{Item, Map};

use crate::msg::FcrossTx;

// type StateType = i64;

pub const CHAIN_ID: Item<u16> = Item::new("chain_id");

// mfs
// storage optimization enabled, we only record the full branches once
pub const MF_MAP: Item<Vec<Option<i64>>> = Item::new("mf_maps"); // 2^pending_len

// history_list, {pending_list, secondary_pending_list}, waiting_list
// hostory txs, a tx is added to HISTORY_TXS_LIST once executed
pub const HISTORY_TXS_LIST: Item<Vec<u32>> = Item::new("history_txs_list");
// pending txs & secondary pending txs
pub const MAX_PENDING_LEN: usize = 10; // <15
pub const PENDING_TX_LIST: Item<Vec<(u32, bool)>> = Item::new("pending_tx_list"); // (tx_id, voted)
pub const MAX_SECONDARY_PENDING_LEN: usize = 8;
// all tx in here should have a determined status to vote, otherwise we should just put it in waiting_list 
pub const SECONDARY_PENDING_TX_LIST: Item<Vec<ScopeExecutedTx>> = Item::new("secondary_pending_tx_list");
// waiting txs
// TODO: give an individual space for the first elements in waiting_list to get better performance
// TODO: set maximum length to waiting_list to get better performance
pub const WAITING_TX_LIST: Item<Vec<FcrossTx>> = Item::new("waiting_tx_list");

// pub const CACHED_INSTRUCTIONS: Map<u32, bool> = Map::new("cached_instructions");


#[cw_serde]
pub struct ScopeExecutedTx {
    pub tx: FcrossTx,
    pub updated_scope: (i64, i64), // (lower_bound, upper_bound), latest merged scope up till now
    pub voted: bool,
    pub received_instruction: Option<bool>,
    // pub calculated_scope: Option<(i64, i64)>, // partial scope
    // pub received_instruction: Option<bool>,
}


// ibc relevant states
pub const MY_CHANNEL: Item<ChannelInfo> = Item::new("my_channel");

#[cw_serde]
pub struct ChannelInfo {
    pub channel_id: String,
    /// whether the channel is completely set up
    pub finalized: bool,
}

// log relevant states
pub const ERR_LOGS: Item<String> = Item::new("err_logs");
pub const TIME_LOGS: Map<u32, TimeInfo> = Map::new("time_logs");

#[cw_serde]
pub struct TimeInfo {
    pub start_time: Timestamp, // time the tx is on-chain (no matter executed or not)
    pub end_time: Option<Timestamp>, // time when tx apply the instruction
}