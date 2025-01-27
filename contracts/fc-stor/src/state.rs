use cosmwasm_schema::cw_serde;
use cosmwasm_std::Timestamp;
use cw_storage_plus::{Item, Map};

use crate::msg::FcrossTx;

pub const CHAIN_ID: Item<u16> = Item::new("chain_id");

// mfs
// storage optimization enabled, we only record the full branches once
pub const MF_MAP: Item<Vec<Option<i64>>> = Item::new("mf_maps"); // 2^pending_len
// pub const MF_VOTE_MAP: Map<u32, bool> = Map::new("mf_vote_maps"); // (bind with MF_MAP)
pub const HISTORY_TXS_LIST: Item<Vec<u32>> = Item::new("history_txs_list"); // once executed, a tx is added to HISTORY_TXS_LIST, no matter pending or not

// pending txs
// the tx_id do not indicate their global sequence
pub const PENDING_TX_LIST: Item<Vec<(u32, bool)>> = Item::new("pending_tx_list"); // (tx_id, voted)

// waiting txs
pub const MAX_PENDING_LEN: u32 = 12; // <15
pub const WAITING_TX_LIST: Item<Vec<FcrossTx>> = Item::new("waiting_tx_list");


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