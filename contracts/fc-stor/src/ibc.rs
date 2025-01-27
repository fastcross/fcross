use cosmwasm_std::{ensure, entry_point, DepsMut, Env, StdError, StdResult, from_json};
use cosmwasm_std::{IbcBasicResponse, IbcChannelConnectMsg, IbcChannelCloseMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcPacketReceiveMsg, IbcReceiveResponse, IbcPacketAckMsg, IbcPacketTimeoutMsg, StdAck};

use crate::msg::Instruction;
use crate::state::{ChannelInfo, MY_CHANNEL, ERR_LOGS};
use crate::contract::exec::finalize_tx;
use crate::error::ContractError;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_open(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg
) -> StdResult<IbcChannelOpenResponse> {
    // handle msg OpenInit/OpenTry
    let channel = msg.channel();
    // only support one channel per contract
    ensure!(MY_CHANNEL.may_load(deps.storage)?.is_none(), StdError::generic_err("channel already exists"));
    MY_CHANNEL.save(deps.storage, &ChannelInfo{
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
    let mut channel_info = MY_CHANNEL.load(deps.storage)?;
    ensure!(channel_info.channel_id==channel.endpoint.channel_id, StdError::generic_err("incosistent channel id"));
    ensure!(channel_info.finalized==false, StdError::generic_err("channel already established"));
    channel_info.finalized=true;
    MY_CHANNEL.save(deps.storage, &channel_info)?;
    Ok(IbcBasicResponse::new()
    .add_attribute("established_channel", channel_info.channel_id))
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
            Ok(IbcReceiveResponse::new().set_ack(StdAck::success(b"mf_force_success")))
        },
        Ok(reps) => Ok(reps)
    }
}

pub fn packet_receive_handler(
    deps: &mut DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,        
) -> Result<IbcReceiveResponse, ContractError> {
    let instruction: Instruction = from_json(&msg.packet.data)?;
    let (attrs, msgs) = finalize_tx(deps, &env, &instruction)?;
    let ack = StdAck::success(b"mf_success");
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