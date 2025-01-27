use cosmwasm_schema::cw_serde;
use cw_storage_plus::{Map, Item};

// CHAIN_NUM="my_secret_key" cargo build
// pub static CHAIN_NUM: usize = env!("CHAIN_NUM").parse().unwrap();
// pub const CHAIN_NUM: usize = 3;

pub const OPENING_VOTES: Map<u32, Vec<u16>> = Map::new("opening_votes");
// bool: success?
pub const CLOSED_VOTES: Map<u32, bool> = Map::new("closed_votes");

pub const CHAIN_NUM: Item<u16> = Item::new("chain_num");

// ibc relevant, use connection_id to differentiate chains
pub const MY_CHANNELS: Map<String, ChannelInfo> = Map::new("my_channels");

#[cw_serde]
pub struct ChannelInfo {
    pub channel_id: String,
    /// whether the channel is completely set up
    pub finalized: bool,
}

pub const ERR_LOGS: Item<String> = Item::new("err_logs");
