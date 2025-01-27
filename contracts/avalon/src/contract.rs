use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, IbcMsg, MessageInfo, Response, StdResult
};
use crate::error::ContractError;
use crate::msg::{InstantiateMsg, QueryMsg, ExecuteMsg};
use crate::state::{CHAIN_ID, CURRENT_STATE, ERR_LOGS, HISTORY_TX_LIST, PENDING_TX_LIST, WAITING_TX_LIST};

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    CHAIN_ID.save(deps.storage, &msg.chain_id)?;
    CURRENT_STATE.save(deps.storage, &msg.original_value)?;
    HISTORY_TX_LIST.save(deps.storage, &Vec::new())?;
    PENDING_TX_LIST.save(deps.storage, &Vec::new())?;
    WAITING_TX_LIST.save(deps.storage, &Vec::new())?;
    ERR_LOGS.save(deps.storage, &"start:".to_string())?;

    Ok(Response::new()
    .add_attribute("method", "instantiate")
    .add_attribute("initiated_chain", msg.chain_id.to_string())
    .add_attribute("original_value", msg.original_value.to_string()))
}

/* QUERY */
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;

    match msg {
        // TODO: query interface
        HistoryTxs{} => to_json_binary(&query::history_txs(deps)?),
        PendingTxs{} => to_json_binary(&query::pending_txs(deps)?),
        WaitingTxs{} => to_json_binary(&query::waiting_txs(deps)?),
        MyErrLogs{} => to_json_binary(&query::my_err_logs(deps)?),
        MyTimeLogs{} => to_json_binary(&query::my_time_logs(deps)?),
    }
}

mod query {
    use crate::{msg::{HistoryTxsResp, MyErrLogsResp, MyTimeLogsResp, PendingTxsResp, WaitingTxsResp}, state::{TIME_LOGS, WAITING_TX_LIST}};

    use super::*;

    pub fn history_txs(deps: Deps) -> StdResult<HistoryTxsResp> {
        let history = HISTORY_TX_LIST.load(deps.storage)?;
        Ok(HistoryTxsResp{ history })
    }

    pub fn pending_txs(deps: Deps) -> StdResult<PendingTxsResp> {
        let pending_txs = PENDING_TX_LIST.load(deps.storage)?;
        Ok(PendingTxsResp { pending_txs })
    }

    pub fn waiting_txs(deps: Deps) -> StdResult<WaitingTxsResp> {
        let waiting_txs = WAITING_TX_LIST.load(deps.storage)?;
        Ok(WaitingTxsResp{ waiting_txs })
    }

    pub fn my_err_logs(deps: Deps) -> StdResult<MyErrLogsResp> {
        let logs = ERR_LOGS.load(deps.storage)?;
        Ok(MyErrLogsResp{logs})
    }

    pub fn my_time_logs(deps: Deps) -> StdResult<MyTimeLogsResp> {
        let logs = TIME_LOGS.range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| {
            let (k, v) = item?;
            let log_str= format!("{},{},{}", k, v.start_time.nanos(), match v.end_time {
                None => "None".to_string(),
                Some(t) => t.nanos().to_string(),
            });
            Ok(log_str)
        })
        .collect::<StdResult<Vec<String>>>()?;
        Ok(MyTimeLogsResp{ logs })
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
        ExecuteTxs{ fcross_txs } => {
            // todo: classify every tx with different events
            let mut attrs: Vec<(String, String)> = Vec::new();
            let mut msgs: Vec<CosmosMsg> = Vec::new();
            fcross_txs
            .into_iter()
            .map(|tx|{
                let (a, m) = exec::add_tx(&mut deps, &env, tx)?;
                attrs.extend(a);
                msgs.extend(m);
                Ok(())
            })
            .collect::<Result<Vec<()>, ContractError>>()?;
            Ok(Response::new().add_attributes(attrs).add_messages(msgs))
        },
        PrepareTx { instruction } => {
            let (attrs, msgs) = exec::prepare_tx(&mut deps, &env, &instruction)?;
            Ok(Response::new().add_attributes(attrs).add_messages(msgs))
        },
        // todo
        FinalizeTx{ instruction } => {
            let (attrs, msgs) = exec::finalize_tx(&mut deps, &env, &instruction)?;
            Ok(Response::new().add_attributes(attrs).add_messages(msgs))
        },
    }
}

pub mod exec {
    use cosmwasm_std::{CosmosMsg, IbcTimeout, WasmMsg};

    use crate::{error::ContractError, msg::{FcrossTx, StatusInstruction, StatusVote, ValidityInstruction, ValidityVote, Vote}, state::{TimeInfo, BATCH_NUM, CACHED_STATUS_INSTRUCTIONS, CURRENT_STATE, MY_CHANNEL, PENDING_TX_LIST, TIME_LOGS, WAITING_TX_LIST}};

    use super::*;
    use crate::msg::Operation;

    pub fn give_status_vote(tx_id: u32, chain_id: u16, status: bool, channel_id: String, env: &Env) -> StdResult<IbcMsg>{
        let my_vote = Vote::Status(StatusVote{
            tx_id,
            chain_id,
            status,
        });
        let msg = IbcMsg::SendPacket {
            channel_id,
            data: to_json_binary(&my_vote)?,
            timeout: IbcTimeout::with_timestamp(env.block.time.plus_seconds(3600000)),
        };
        Ok(msg)
    }

    pub fn give_validity_vote(tx_id: u32, chain_id: u16, dependencies: Vec<u32>, channel_id: String, env: &Env) -> StdResult<IbcMsg>{
        let my_vote = Vote::Validity(ValidityVote{
            tx_id,
            chain_id,
            dependencies,
        });
        let msg = IbcMsg::SendPacket {
            channel_id,
            data: to_json_binary(&my_vote)?,
            timeout: IbcTimeout::with_timestamp(env.block.time.plus_seconds(3600000)),
        };
        Ok(msg)
    }

    pub fn new_preparation_request(deps: &mut DepsMut, env: &Env, tx_id: u32) -> StdResult<Option<CosmosMsg>>{
        // check if instruction is already received and cached
        let instr = CACHED_STATUS_INSTRUCTIONS.may_load(deps.storage, tx_id)?;
        match instr {
            None => {
                Ok(None)
            },
            Some(advancement) => {
                CACHED_STATUS_INSTRUCTIONS.remove(deps.storage, tx_id);
                let tx_msg = to_json_binary(&ExecuteMsg::PrepareTx { instruction: StatusInstruction{
                    tx_id,
                    advancement,
                }})?;
                Ok(Some(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: env.contract.address.to_string(),
                    msg: tx_msg,
                    funds: vec![],
                })))
            },
        }
    }

    pub fn try_pull_new_batch(deps: &mut DepsMut) -> StdResult<Option<Vec<FcrossTx>>>{
        // pull new batch
        let mut waiting_txs = WAITING_TX_LIST.load(deps.storage)?;
        match waiting_txs.len()>=BATCH_NUM {
            false => {
                return Ok(None)
            },
            true => {
                // drain waiting list
                let new_batch = waiting_txs.drain(..BATCH_NUM).collect();
                WAITING_TX_LIST.save(deps.storage, &waiting_txs)?;
                return Ok(Some(new_batch))
            },
        }
    }

    pub fn try_finish_batch(deps: &mut DepsMut, env: &Env) -> Result<Option<(Vec<(String, String)>, Vec<CosmosMsg>)>, ContractError>{
        let pending_txs = PENDING_TX_LIST.load(deps.storage)?;

        // check if finished
        match pending_txs.iter().all(|tx| {tx.1.is_none() || (tx.1.is_some() && tx.2 && tx.3)}) {
            true => {
                let mut attrs: Vec<(String, String)> = Vec::new();
                let mut msgs: Vec<CosmosMsg> = Vec::new();
                attrs.push(("finished_batch".to_string(), true.to_string()));
                attrs.push(("batch_info".to_string(), pending_txs.iter().map(|tx| tx.0.tx_id.to_string()).collect::<Vec<_>>().join("-")));
            
                // add pending txs to history & clean pending txs
                // let batch = PENDING_TX_LIST.load(deps.storage)?;
                let mut history: Vec<(u32, bool)> = HISTORY_TX_LIST.load(deps.storage)?;
                history.extend(pending_txs.iter().map(|tx| (tx.0.tx_id, tx.1.is_some())));
                HISTORY_TX_LIST.save(deps.storage, &history)?;
                PENDING_TX_LIST.save(deps.storage, &Vec::new())?;
                // apply state transition
                let mut current_state = CURRENT_STATE.load(deps.storage)?;
                pending_txs.iter().for_each(|tx| {
                    if tx.1.is_some() {
                        current_state = tx.1.unwrap()
                    }
                });
                CURRENT_STATE.save(deps.storage, &current_state)?;
                // log the end time of tx
                // todo: actually it is not the exactly end time
                pending_txs.iter().map(|tx| {
                    let mut tl = TIME_LOGS.load(deps.storage, tx.0.tx_id)?;
                    tl.end_time = Some(env.block.time);
                    TIME_LOGS.save(deps.storage, tx.0.tx_id, &tl)?;
                    Ok(())
                })
                .collect::<StdResult<Vec<()>>>()?;

            
                let new_batch = try_pull_new_batch(deps)?;
                match new_batch {
                    None => {},
                    Some(new_batch) => {
                        let (attr, msg) = execute_batch(deps, env, new_batch)?;
                        attrs.extend(attr);
                        msgs.extend(msg);
                    },
                }
                Ok(Some((attrs, msgs)))
            },
            false => {
                Ok(None)
            },
        }
    }

    pub fn add_tx(
        deps: &mut DepsMut,
        env: &Env,
        tx: FcrossTx,
    ) -> Result<(Vec<(String, String)>, Vec<CosmosMsg>), ContractError> {
        let mut attrs: Vec<(String, String)> = Vec::new();
        let mut msgs: Vec<CosmosMsg> = Vec::new();
        let tx_id = tx.tx_id;

        // note: we eliminate check of the same tx_id for simplicity, the client should control the difference of tx_id itself

        // add to waiting list
        let mut waiting_txs = WAITING_TX_LIST.load(deps.storage)?;
        waiting_txs.push(tx);
        attrs.push(("added_tx".to_string(), tx_id.to_string()));
        if waiting_txs.len()<BATCH_NUM{
            WAITING_TX_LIST.save(deps.storage, &waiting_txs)?;
            return Ok((attrs, msgs))
        }

        // check pending list avaliable, if so, execute the batch
        let pending_txs = PENDING_TX_LIST.load(deps.storage)?;
        let pending_txs_len = pending_txs.len();
        if pending_txs_len==0 {
            let new_batch = waiting_txs.drain(..BATCH_NUM).collect::<Vec<FcrossTx>>();
            WAITING_TX_LIST.save(deps.storage, &waiting_txs)?;
            // pending list is set by the function itself
            let (attr, msg) = execute_batch(deps, env, new_batch)?;
            attrs.extend(attr);
            msgs.extend(msg);
            Ok((attrs, msgs))
        } else if pending_txs_len==BATCH_NUM {
            WAITING_TX_LIST.save(deps.storage, &waiting_txs)?;
            Ok((attrs, msgs))
        } else {
            Err(ContractError::UnexpectedPendingTxsNumber(pending_txs.len()))
        }
    }

    // this function is not entrypoint (so no check is needed), entrypoints only contain: add_tx, prepare_tx, finalize_tx
    pub fn execute_batch(
        deps: &mut DepsMut,
        env: &Env,
        batch: Vec<FcrossTx>,
    ) -> Result<(Vec<(String, String)>, Vec<CosmosMsg>), ContractError> {
        let mut attrs: Vec<(String, String)> = Vec::new();
        let mut msgs: Vec<CosmosMsg> = Vec::new();
        
        // execute
        let current_value = CURRENT_STATE.load(deps.storage)?;
        let mut latest_value = Some(current_value);
        let pending_txs = batch.clone().into_iter().map(|tx| {
            // execute
            latest_value = match tx.operation {
                Operation::CreditBalance { amount } => {
                    latest_value.map(|v| v+amount)
                },
                Operation::DebitBalance { amount } => {
                    match latest_value {
                        None => None,
                        Some(v) => {
                            match v>=amount {
                                true => { Some(v-amount) },
                                false => { None },
                            }
                        }
                    }
                },
            };
            (tx, latest_value, false, false)
        })
        .collect::<Vec<_>>();

        // add r1 vote
        let chain_id = CHAIN_ID.load(deps.storage)?;
        let channel_id = MY_CHANNEL.load(deps.storage)?.channel_id;
        pending_txs.iter().map(|tx|{
            // log the satrt time of tx
            TIME_LOGS.save(deps.storage, tx.0.tx_id, &TimeInfo{
                start_time: env.block.time,
                end_time: None,
            })?;

            let msg = give_status_vote(tx.0.tx_id, chain_id, tx.1.is_some(), channel_id.clone(), env)?;
            msgs.push(CosmosMsg::Ibc(msg));
            Ok(())
        })
        .collect::<StdResult<Vec<()>>>()?;

        // update pending txs
        PENDING_TX_LIST.save(deps.storage, &pending_txs)?;

        // in case all txs are aborted, we can finish the batch
        match try_finish_batch(deps, env)? {
            Some((attr, msg)) => {
                attrs.extend(attr);
                msgs.extend(msg);
                return Ok((attrs, msgs))
            },
           None => {},
        }
        
        // if some status instructions already received, apply it instantly
        pending_txs.iter().filter(|tx| tx.1.is_some()).map(|tx| {
            match new_preparation_request(deps, env, tx.0.tx_id)? {
                None => { Ok(()) },
                Some(new_finalization_msg) => {
                    msgs.push(new_finalization_msg);
                    Ok(())
                }
            }
        })
        .collect::<StdResult<Vec<()>>>()?;

        // response
        attrs.push(("executed_batch".to_string(), batch.iter().map(|i| i.tx_id.to_string()).collect::<Vec<String>>().join("-")));
        Ok((attrs, msgs))
    }

    // on receiving StatusInstruction
    pub fn prepare_tx(
        deps: &mut DepsMut,
        env: &Env,
        instr: &StatusInstruction,
    ) -> Result<(Vec<(String, String)>, Vec<CosmosMsg>), ContractError> {
        let mut msgs: Vec<CosmosMsg> = Vec::new();
        let mut attrs: Vec<(String, String)> = Vec::new();
        let tx_id = instr.tx_id;

        // check 1) tx in pending list 2) haven't received r1 instruction yet (strict)
        let mut pending_txs = PENDING_TX_LIST.load(deps.storage)?;
        let p = pending_txs
        .iter()
        .position(|i| i.0.tx_id==tx_id);
        if p.is_none() {
            let history = HISTORY_TX_LIST.load(deps.storage)?.iter().map(|i| i.0).collect::<Vec<u32>>();
            match history.contains(&tx_id) {
                // already early aborted
                true => {
                    attrs.push(("already_early_aborted".to_string(), tx_id.to_string()));
                },
                // not executed
                false => {
                    CACHED_STATUS_INSTRUCTIONS.save(deps.storage, tx_id, &instr.advancement)?;
                    attrs.push(("cached_r1_instruction".to_string(), format!("{}-{}", tx_id, instr.advancement)));
                },
            }
            return Ok((attrs, msgs))
        }
        let p = p.unwrap();
        if pending_txs[p].2 {
            return Err(ContractError::TxAlreadyPrepared(tx_id))
        }

        // prepare the tx
        pending_txs[p].2 = true;
        match instr.advancement {
            false => {
                // abort the tx and all the following ones
                for i in p..BATCH_NUM{
                    pending_txs[i].1 = None;
                }
            },
            true => {
                // give r2 vote
                let msg = give_validity_vote(tx_id, CHAIN_ID.load(deps.storage)?, pending_txs.iter().take(p).map(|i| i.0.tx_id).collect::<Vec<u32>>(), MY_CHANNEL.load(deps.storage)?.channel_id, env)?;
                msgs.push(CosmosMsg::Ibc(msg));
            },
        }

        // update pending list
        PENDING_TX_LIST.save(deps.storage, &pending_txs)?;

        // in case all txs are aborted, we can finish the batch
        match try_finish_batch(deps, env)? {
            Some((attr, msg)) => {
                attrs.extend(attr);
                msgs.extend(msg);
                return Ok((attrs, msgs))
            },
           None => {},
        }

        return Ok((attrs, msgs))
    }

    pub fn finalize_tx(
        deps: &mut DepsMut,
        env: &Env,
        instr: &ValidityInstruction,
    ) -> Result<(Vec<(String, String)>, Vec<CosmosMsg>), ContractError> {
        let mut msgs: Vec<CosmosMsg> = Vec::new();
        let mut attrs: Vec<(String, String)> = Vec::new();
        let tx_id = instr.tx_id;

        // check: 1) tx in pending list 2) have received r1 instruction, and 3) haven't received r2 instruction yet
        // (actually, the received r1 instruction has to be positive, but we cannot validate that since the new_value may be None even if the r1 instruction is positive)
        let mut pending_txs = PENDING_TX_LIST.load(deps.storage)?;
        let p = pending_txs
        .iter()
        .position(|i| i.0.tx_id==tx_id);
        if p.is_none() {
            let history = HISTORY_TX_LIST.load(deps.storage)?.iter().map(|i| i.0).collect::<Vec<u32>>();
            match history.contains(&tx_id) {
                // already aborted
                true => {
                    attrs.push(("already_aborted".to_string(), tx_id.to_string()));
                },
                // unexpected instruction of future tx
                false => {
                    return Err(ContractError::UnexpectedValidityInstruction{
                        received_tx_id: tx_id,
                        received_commitment: instr.commitment,
                        expected_tx_ids: pending_txs.iter().map(|tx| {tx.0.tx_id}).collect::<Vec<u32>>(),
                    })
                },
            }
            return Ok((attrs, msgs))
        }
        let p = p.unwrap();
        if !pending_txs[p].2 {
            return Err(ContractError::TxUnprepared(tx_id))
        }
        if pending_txs[p].3 {
            return Err(ContractError::TxAlreadyFinalized(tx_id))
        }

        // finalize the tx
        pending_txs[p].3 = true;
        match instr.commitment {
            false => {
                // abort the tx
                pending_txs[p].1 = None;
            },
            true => {},
        }

        // update pending list
        PENDING_TX_LIST.save(deps.storage, &pending_txs)?;

        // in case all txs are aborted or committed, we can finish the batch
        match try_finish_batch(deps, env)? {
            Some((attr, msg)) => {
                attrs.extend(attr);
                msgs.extend(msg);
                return Ok((attrs, msgs))
            },
           None => {},
        }

        return Ok((attrs, msgs))
    }
}