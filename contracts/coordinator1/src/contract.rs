use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, IbcMsg, IbcTimeout
};
use crate::error::ContractError;
use crate::msg::{InstantiateMsg, QueryMsg, ExecuteMsg};
use crate::state::{CHAIN_NUM, ERR_LOGS};

// pub const CHAIN_NUM: &str = env!("CHAIN_NUM");

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    ERR_LOGS.save(deps.storage, &"start:".to_string())?;

    CHAIN_NUM.save(deps.storage, &msg.chain_num)?;
    Ok(Response::new()
    .add_attribute("method", "instantiate")
    .add_attribute("chain_num", msg.chain_num.to_string()))
}

/* QUERY */
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
        OpeningVotes{} => to_json_binary(&query::opening_votes(deps)?),
        ClosedVotes{} => to_json_binary(&query::closed_votes(deps)?),
        MyLogs{} => to_json_binary(&query::my_logs(deps)?),
    }
}

mod query {
    use crate::{msg::{ClosedVotesResp, MyLogsResp, OpeningVotesResp}, state::{CLOSED_VOTES, OPENING_VOTES}};

    use super::*;

    pub fn opening_votes(deps: Deps) -> StdResult<OpeningVotesResp> {
        let votes = OPENING_VOTES
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<StdResult<Vec<(u32, Vec<u16>)>>>()?;

        Ok(OpeningVotesResp{ votes })
    }

    pub fn closed_votes(deps: Deps) -> StdResult<ClosedVotesResp> {
        let votes = CLOSED_VOTES
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<StdResult<Vec<(u32, bool)>>>()?;

        Ok(ClosedVotesResp{ votes })
    }

    pub fn my_logs(deps: Deps) -> StdResult<MyLogsResp> {
        let logs = ERR_LOGS.load(deps.storage)?;
        Ok(MyLogsResp{logs})
    }
}

/* EXECUTION */
pub fn execute(
    mut deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    use ExecuteMsg::*;

    match msg {
        AddVote { vote } => {
            let (attrs, msgs) = exec::add_vote(&mut deps, &env, &vote)?;
            Ok(Response::new().add_attributes(attrs).add_messages(msgs))
        },
    }
}

pub mod exec {
    use crate::msg::Instruction;
    use crate::{error::ContractError, msg::Vote};
    use crate::state::{CHAIN_NUM, CLOSED_VOTES, MY_CHANNELS, OPENING_VOTES};
    use super::*;

    pub fn create_instruction(tx_id: u32, commitment: bool, deps: Deps, env: &Env) -> StdResult<Vec<IbcMsg>>{
        let my_instruction = Instruction{
            tx_id,
            commitment,
        };
        let data = to_json_binary(&my_instruction)?;
        MY_CHANNELS.range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| {
            let (_,v) = item?;
            Ok(IbcMsg::SendPacket {
                channel_id: v.channel_id,
                data: data.clone(),
                timeout: IbcTimeout::with_timestamp(env.block.time.plus_seconds(3600000)),
            })
        })
        .collect::<StdResult<Vec<IbcMsg>>>()
    }

    pub fn add_vote(
        deps: &mut DepsMut,
        env: &Env,
        new_vote: &Vote,
    ) -> Result<(Vec<(String, String)>, Vec<IbcMsg>), ContractError> {
        // pre-check: open and unvoted
        let mut voted_chains = match OPENING_VOTES.may_load(deps.storage, new_vote.tx_id)? {
            None=> {
                // 2 possibilities: 1) uncreated (receive the first vote) 2) has been closed
                match CLOSED_VOTES.may_load(deps.storage, new_vote.tx_id)? {
                    None=> Vec::new(),
                    Some(_)=>{
                        return Err(ContractError::AlreadyClosed { tx_id: new_vote.tx_id });
                    },
                }
                
            },
            Some(voted_chains) => voted_chains,
        };
        if voted_chains.contains(&new_vote.chain_id){
            return Err(ContractError::AlreadyVoted { tx_id: new_vote.tx_id, chain_id: new_vote.chain_id })
        }

        // add vote
        let mut attrs: Vec<(String, String)> = vec![("new_vote".to_string(), format!("{}-{}", new_vote.tx_id, new_vote.chain_id))];
        let mut msgs: Vec<IbcMsg> = Vec::new();
        match new_vote.success {
            false => {
                // fast abort
                OPENING_VOTES.remove(deps.storage, new_vote.tx_id);
                CLOSED_VOTES.save(deps.storage, new_vote.tx_id, &false)?;
                msgs.extend(create_instruction(new_vote.tx_id, false, deps.as_ref(), env)?);
                attrs.push(("instruction".to_string(), "aborted".to_string()));
            }
            true => {
                voted_chains.push(new_vote.chain_id);
                let chain_num = CHAIN_NUM.load(deps.storage)?;
                if (voted_chains.len() as u16) == chain_num{
                    // instruct to commit
                    OPENING_VOTES.remove(deps.storage, new_vote.tx_id);
                    CLOSED_VOTES.save(deps.storage, new_vote.tx_id, &true)?;
                    msgs.extend(create_instruction(new_vote.tx_id, true, deps.as_ref(), env)?);
                    attrs.push(("instruction".to_string(), "committed".to_string()));
                } else {
                    OPENING_VOTES.save(deps.storage, new_vote.tx_id, &voted_chains)?;
                }
            }
        }
        Ok((attrs, msgs))
    }
}