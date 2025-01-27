use cosmwasm_schema::cw_serde;
use cw_storage_plus::{Map, Item};

pub const OPENING_R1_VOTES: Map<u32, Vec<u16>> = Map::new("opening_r1_votes");
// (aggragated_dependencies, voted_chains)
pub const OPENING_R2_VOTES: Map<u32, (Vec<u32>, Vec<u16>)> = Map::new("opening_r2_votes");
pub const READY_TXS: Item<Vec<u32>> = Item::new("ready_txs"); // txs ready to give instruction (bind with OPENING_R2_VOTES)

pub const CLOSED_R1_VOTES: Map<u32, bool> = Map::new("closed_r1_votes");
pub const CLOSED_R2_VOTES: Map<u32, bool> = Map::new("closed_r2_votes");


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
