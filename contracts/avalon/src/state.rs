use cosmwasm_schema::cw_serde;
use cosmwasm_std::Timestamp;
use cw_storage_plus::{Item, Map};

use crate::msg::FcrossTx;

pub const CHAIN_ID: Item<u16> = Item::new("chain_id");

pub const CURRENT_STATE: Item<i64> = Item::new("current_state");

pub const HISTORY_TX_LIST: Item<Vec<(u32, bool)>> = Item::new("history_tx_list"); // (tx_id, committed/aborted)
// (0.pending_tx, 1.new_value, 2.received_r1_instruction, 3.received_r2_instruction)
pub const PENDING_TX_LIST: Item<Vec<(FcrossTx, Option<i64>, bool, bool)>> = Item::new("pending_tx_list");
pub const BATCH_NUM: usize = 5; // for simplicity (to prevent endless abort), we execute txs 
pub const WAITING_TX_LIST: Item<Vec<FcrossTx>> = Item::new("waiting_tx_list");

// todo: how do I do about this variable
pub const CACHED_STATUS_INSTRUCTIONS: Map<u32, bool> = Map::new("cached_status_instructions");
// pub const CACHED_VALIDITY_INSTRUCTIONS: Map<u32, bool> = Map::new("cached_validity_instructions");


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
    pub start_time: Timestamp,
    pub end_time: Option<Timestamp>,
}