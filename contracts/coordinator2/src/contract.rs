use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, IbcMsg, IbcTimeout
};
use crate::error::ContractError;
use crate::msg::{InstantiateMsg, QueryMsg, ExecuteMsg};
use crate::state::{CHAIN_NUM, ERR_LOGS, READY_TXS};

// pub const CHAIN_NUM: &str = env!("CHAIN_NUM");

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    READY_TXS.save(deps.storage, &Vec::new())?;
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
        AllClosedVotes{} => to_json_binary(&query::all_closed_votes(deps)?),
        MyLogs{} => to_json_binary(&query::my_logs(deps)?),
    }
}

mod query {
    use crate::{msg::{AllClosedVotesResp, ClosedVotesResp, MyLogsResp, OpeningVotesResp}, state::{CLOSED_R1_VOTES, CLOSED_R2_VOTES, OPENING_R1_VOTES, OPENING_R2_VOTES}};

    use super::*;

    pub fn opening_votes(deps: Deps) -> StdResult<OpeningVotesResp> {
        let r1_votes = OPENING_R1_VOTES
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<StdResult<Vec<(u32, Vec<u16>)>>>()?;

        let r2_votes = OPENING_R2_VOTES
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<StdResult<Vec<(u32, (Vec<u32>, Vec<u16>))>>>()?;
        
        Ok(OpeningVotesResp{ r1_votes, r2_votes})
    }

    pub fn closed_votes(deps: Deps) -> StdResult<ClosedVotesResp> {
        let votes = CLOSED_R1_VOTES
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<StdResult<Vec<(u32, bool)>>>()?;

        Ok(ClosedVotesResp{ votes })
    }

    pub fn all_closed_votes(deps: Deps) -> StdResult<AllClosedVotesResp> {
        let r1_votes = CLOSED_R1_VOTES
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<StdResult<Vec<(u32, bool)>>>()?;

        let r2_votes = CLOSED_R2_VOTES
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<StdResult<Vec<(u32, bool)>>>()?;

        Ok(AllClosedVotesResp{ r1_votes, r2_votes})
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
        AddStatusVote { vote } => {
            let (attrs, msgs) = exec::add_r1_vote(&mut deps, &env, &vote)?;
            Ok(Response::new().add_attributes(attrs).add_messages(msgs))
        },
        AddValidityVote { vote } => {
            let (attrs, msgs) = exec::add_r2_vote(&mut deps, &env, &vote)?;
            Ok(Response::new().add_attributes(attrs).add_messages(msgs))
        },
    }
}

pub mod exec {
    use crate::msg::{Instruction, StatusInstruction, StatusVote, ValidityInstruction, ValidityVote};
    use crate::error::ContractError;
    use crate::state::{CHAIN_NUM, CLOSED_R1_VOTES, CLOSED_R2_VOTES, MY_CHANNELS, OPENING_R1_VOTES, OPENING_R2_VOTES, READY_TXS};
    use super::*;

    pub fn create_instruction_msg(my_instruction: Instruction, deps: Deps, env: &Env) -> StdResult<Vec<IbcMsg>>{
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

    pub fn add_r1_vote(
        deps: &mut DepsMut,
        env: &Env,
        new_vote: &StatusVote,
    ) -> Result<(Vec<(String, String)>, Vec<IbcMsg>), ContractError> {
        let mut attrs: Vec<(String, String)> = vec![("new_vote".to_string(), format!("{}-{}", new_vote.tx_id, new_vote.chain_id))];
        let mut msgs: Vec<IbcMsg> = Vec::new();
        let tx_id = new_vote.tx_id;

        // pre-check: open and unvoted
        let mut voted_chains = match OPENING_R1_VOTES.may_load(deps.storage, tx_id)? {
            None=> {
                // 2 possibilities: 1) uncreated (the first received vote) 2) has been closed (casued by early abort, doing nothing)
                match CLOSED_R1_VOTES.may_load(deps.storage, tx_id)? {
                    None=> Vec::new(),
                    Some(_)=>{
                        return Ok((attrs, msgs));
                    },
                }
                
            },
            Some(voted_chains) => voted_chains,
        };
        if voted_chains.contains(&new_vote.chain_id){
            return Err(ContractError::AlreadyVoted { tx_id, chain_id: new_vote.chain_id })
        }

        // add vote
        match new_vote.status {
            false => {
                // close the r1 vote
                OPENING_R1_VOTES.remove(deps.storage, tx_id);
                CLOSED_R1_VOTES.save(deps.storage, tx_id, &false)?;
                msgs.extend(create_instruction_msg(Instruction::Status(StatusInstruction{ tx_id, advancement: false}), deps.as_ref(), env)?);
                attrs.push(("instruction".to_string(), "aborted".to_string()));
            }
            true => {
                voted_chains.push(new_vote.chain_id);
                let chain_num = CHAIN_NUM.load(deps.storage)?;
                if (voted_chains.len() as u16) == chain_num{
                    // instruct to advance
                    OPENING_R1_VOTES.remove(deps.storage, new_vote.tx_id);
                    CLOSED_R1_VOTES.save(deps.storage, tx_id, &true)?;
                    msgs.extend(create_instruction_msg(Instruction::Status(StatusInstruction{ tx_id, advancement: true}), deps.as_ref(), env)?);
                    attrs.push(("instruction".to_string(), "committed".to_string()));
                } else {
                    OPENING_R1_VOTES.save(deps.storage, tx_id, &voted_chains)?;
                }
            }
        }
        Ok((attrs, msgs))
    }

    pub fn add_r2_vote(
        deps: &mut DepsMut,
        env: &Env,
        new_vote: &ValidityVote,
    ) -> Result<(Vec<(String, String)>, Vec<IbcMsg>), ContractError> {
        let mut attrs: Vec<(String, String)> = vec![("new_vote".to_string(), format!("{}-{}", new_vote.tx_id, new_vote.chain_id))];
        let mut msgs: Vec<IbcMsg> = Vec::new();
        let tx_id = new_vote.tx_id;

        // check: 1) r1 vote of the tx must be closed and positive
        let r1_vote_result = CLOSED_R1_VOTES.may_load(deps.storage, tx_id)?;
        match r1_vote_result {
            None => {
                return Err(ContractError::StatusVoteUnfinished(tx_id))
            },
            Some(advanced) => {
                match advanced {
                    false => {
                        return Err(ContractError::ValidityVoteNotAllowed(tx_id))
                    },
                    true => {},
                }
            },
        }
        // 2) should not be closed (r2 vote cannot be early aborted before receive all the r2 votes)
        let r2_vote_result = CLOSED_R2_VOTES.may_load(deps.storage, tx_id)?;
        if r2_vote_result.is_some() {
            return Err(ContractError::TxClosed(tx_id))
        }
        // 3) should not be already voted
        let mut existing_votes = match OPENING_R2_VOTES.may_load(deps.storage, tx_id)? {
            // if not found, created a new one
            None=> { (Vec::new(), Vec::new()) },
            Some(vs) => vs,
        };
        if existing_votes.1.contains(&new_vote.chain_id){
            return Err(ContractError::AlreadyVoted { tx_id, chain_id: new_vote.chain_id })
        }

        // insert new vote into existing votes
        existing_votes.1.push(new_vote.chain_id);
        let new_dep = new_vote.dependencies.clone().into_iter().filter(|tx_id| !existing_votes.0.contains(tx_id)).collect::<Vec<u32>>();
        existing_votes.0.extend(new_dep);

        // save the updated r2 votes
        OPENING_R2_VOTES.save(deps.storage, tx_id, &existing_votes)?;
        // if gather up all r2 votes, add the tx to ready list
        let chain_num = CHAIN_NUM.load(deps.storage)?;
        if existing_votes.1.len() as u16 == chain_num {
            let mut ready_txs = READY_TXS.load(deps.storage)?;
            ready_txs.push(tx_id);
            READY_TXS.save(deps.storage, &ready_txs)?;

            // check if we can give r2 instructions for all ready txs
            msgs.extend(check_ready_vote(deps, env)?);
        }

        Ok((attrs, msgs))
    }

    pub fn check_ready_vote(deps: &mut DepsMut, env: &Env) -> Result<Vec<IbcMsg>, ContractError>{
        let mut msgs = Vec::new();
        let mut ready_ids = READY_TXS.load(deps.storage)?;
        // check each ready tx if we can close it
        for tx_id in ready_ids.clone(){
            let dependencies =OPENING_R2_VOTES.load(deps.storage, tx_id)?.0;
            let mut uncertainty = false;
            let mut commitment = true;
            // for ech dep (tx), we check if its r1 vote success 
            for d in dependencies{
                match CLOSED_R1_VOTES.may_load(deps.storage, d)? {
                    None => {
                        uncertainty = true;
                        break;
                    },
                    Some(success) => {
                        match success {
                            false => {
                                commitment = false;
                                break;
                            },
                            true => {},
                        }
                    },
                };
            }
            // check finish, submit instruction for the tx
            match uncertainty {
                // uncertainty, no instruction added
                true => {},
                // negative or positive r2 instruction added
                false => {
                    msgs.extend(create_instruction_msg(Instruction::Validity(ValidityInstruction{ tx_id, commitment}), deps.as_ref(), env)?);
                    // clean opening r2 vote and ready txs & add to closed r2 vote
                    OPENING_R2_VOTES.remove(deps.storage, tx_id);
                    ready_ids.retain(|i| *i!= tx_id); // save ready_ids later
                    CLOSED_R2_VOTES.save(deps.storage, tx_id, &commitment)?;
                },
            }
        }

        // update ready txs
        READY_TXS.save(deps.storage, &ready_ids)?;
        Ok(msgs)
    }
}