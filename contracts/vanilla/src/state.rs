use cosmwasm_schema::cw_serde;
use cosmwasm_std::Timestamp;
use cw_storage_plus::{Item, Map};

use crate::msg::FcrossTx;

pub const CHAIN_ID: Item<u16> = Item::new("chain_id");

pub const CURRENT_STATE: Item<i64> = Item::new("current_state");

pub const HISTORY_TXS_LIST: Item<Vec<u32>> = Item::new("history_txs_list"); // after a tx is finalized, adding to history (while in Fcross, tx is added instantly after execution)
pub const PENDING_TX: Item<Option<(FcrossTx, i64)>> = Item::new("pending_tx"); // (pending_tx, new_value)
pub const WAITING_TX_LIST: Item<Vec<FcrossTx>> = Item::new("waiting_tx_list");

pub const CACHED_INSTRUCTIONS: Map<u32, bool> = Map::new("cached_instructions");


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