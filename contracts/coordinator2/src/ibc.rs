use cosmwasm_std::{ensure, entry_point, DepsMut, Env, StdError, StdResult, from_json};
use cosmwasm_std::{IbcBasicResponse, IbcChannelConnectMsg, IbcChannelCloseMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcPacketReceiveMsg, IbcReceiveResponse, IbcPacketAckMsg, IbcPacketTimeoutMsg, StdAck};

use crate::contract::exec::{add_r1_vote, add_r2_vote};
use crate::msg::Vote::{self, *};
use crate::state::{ChannelInfo, MY_CHANNELS, ERR_LOGS};
use crate::error::ContractError;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_open(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg
) -> StdResult<IbcChannelOpenResponse> {
    // use connection_id to differentiate chains
    let channel = msg.channel();
    ensure!(MY_CHANNELS.may_load(deps.storage, channel.connection_id.clone())?.is_none(), StdError::generic_err("connection already exists"));
    MY_CHANNELS.save(deps.storage, channel.connection_id.clone(), &ChannelInfo{
        channel_id: channel.endpoint.channel_id.clone(),
        finalized: false,
    })?;
    Ok(())
}
 
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();
    let mut channel_info = MY_CHANNELS.load(deps.storage, channel.connection_id.clone())?;
    ensure!(channel_info.channel_id==channel.endpoint.channel_id, StdError::generic_err("incosistent channel id"));
    ensure!(channel_info.finalized==false, StdError::generic_err("channel already established"));
    channel_info.finalized=true;
    MY_CHANNELS.save(deps.storage, channel.connection_id.clone(), &channel_info)?;

    Ok(IbcBasicResponse::new()
    .add_attribute("established_connection_id", channel.connection_id.clone()))
}
 
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcChannelCloseMsg,
) -> StdResult<IbcBasicResponse> {
    Err(StdError::generic_err("closing not allowed"))
}



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_receive(
    mut deps: DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,        
) -> Result<IbcReceiveResponse, ContractError> {
    let res = packet_receive_handler(&mut deps, env, msg);
    match res {
        Err(e) => {
            let mut logs = ERR_LOGS.load(deps.storage)?;
            logs.push_str(format!("\n{:?}", e).as_str());
            ERR_LOGS.save(deps.storage, &logs)?;
            Ok(IbcReceiveResponse::new().set_ack(StdAck::success(b"coordinator_force_success")))
        },
        Ok(reps) => Ok(reps)
    }
}

pub fn packet_receive_handler(
    deps: &mut DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,        
) -> Result<IbcReceiveResponse, ContractError> {
    let received_vote: Vote = from_json(&msg.packet.data)?;
    let (attrs, msgs) = match received_vote {
        Status(vt) => {
            add_r1_vote(deps, &env, &vt)?
        },
        Validity(vt) => {
            add_r2_vote(deps, &env, &vt)?
        },
    };

    let ack = StdAck::success(b"coordinator_success");
    Ok(IbcReceiveResponse::new().add_attributes(attrs).add_messages(msgs).set_ack(ack))
}

#[entry_point]
pub fn ibc_packet_ack(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketAckMsg,        
) -> StdResult<IbcBasicResponse> {
    Ok(IbcBasicResponse::default())
}

#[entry_point]
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> StdResult<IbcBasicResponse> {
    Ok(IbcBasicResponse::new().add_attribute("action", "ibc_packet_timeout"))
}