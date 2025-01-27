use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, IbcMsg, MessageInfo, Response, StdResult
};
use crate::error::ContractError;
use crate::msg::{InstantiateMsg, QueryMsg, ExecuteMsg};
use crate::state::{CHAIN_ID, ERR_LOGS, HISTORY_TXS_LIST, MF_MAP, MF_VOTE_MAP, PENDING_TX_LIST, WAITING_TX_LIST};

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    CHAIN_ID.save(deps.storage, &msg.chain_id)?;
    MF_MAP.save(deps.storage, 0, &vec![Some(msg.original_value)])?;
    PENDING_TX_LIST.save(deps.storage, &Vec::new())?;
    HISTORY_TXS_LIST.save(deps.storage, &vec![0])?;
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
        Multifuture{ tx_id } => to_json_binary(&query::multifuture(deps, tx_id)?),
        AllMfs{} => to_json_binary(&query::all_mfs(deps)?),
        WaitingList {} => to_json_binary(&query::waiting_list(deps)?),
        MyErrLogs{} => to_json_binary(&query::my_err_logs(deps)?),
        MyTimeLogs{} => to_json_binary(&query::my_time_logs(deps)?),
    }
}

mod query {
    use exec::format_mf;

    use crate::{msg::{AllMfsResp, MultifutureResp, MyErrLogsResp, MyTimeLogsResp, WaitingListResp}, state::{TIME_LOGS, WAITING_TX_LIST}};

    use super::*;

    pub fn multifuture(deps: Deps, tx_id: u32) -> StdResult<MultifutureResp> {
        let mf = MF_MAP
        .load(deps.storage, tx_id)?
        .into_iter()
        .map(|i|{
            match i {
                None=>"None".to_string(),
                Some(v)=>v.to_string(),
            }
        })
        .collect::<Vec<String>>();
        Ok(MultifutureResp{futures: mf})
    }

    pub fn all_mfs(deps: Deps) -> StdResult<AllMfsResp> {
        let history = HISTORY_TXS_LIST.load(deps.storage)?;
        let mfs = history
        .into_iter()
        .map(|id| {
            let mf = MF_MAP.load(deps.storage, id)?;
                Ok((id, format_mf(&mf)))
        })
        .collect::<StdResult<Vec<(u32, String)>>>()?;
        Ok(AllMfsResp{ mfs })
    }

    pub fn waiting_list(deps: Deps) -> StdResult<WaitingListResp> {
        let waiting_tx_list = WAITING_TX_LIST.load(deps.storage)?;
        Ok(WaitingListResp{ waiting_tx_list })
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
                let (a, m) = exec::execute_tx(&mut deps, &env, tx)?;
                attrs.extend(a);
                msgs.extend(m);
                Ok(())
            })
            .collect::<Result<Vec<()>, ContractError>>()?;
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
    use cosmwasm_std::{CosmosMsg, WasmMsg};

    use crate::{error::ContractError, msg::{FcrossTx, Instruction}, state::{TimeInfo, MAX_PENDING_LEN, MY_CHANNEL, TIME_LOGS, WAITING_TX_LIST}, utils};

    use super::*;
    use crate::msg::{Operation, Vote};

    #[derive(Debug, Clone, Copy)]
    pub enum ExecutionStatus {
        Success,
        Failure,
        Uncertainty,
    }

    pub fn check_execution_stautus(values: &Vec<Option<i64>>, trim_inherited: bool) -> ExecutionStatus{
        let values = match trim_inherited {
            false => values,
            true => {
                &values[values.len()/2..]
            },
        };
        let has_some = values.iter().any(|v| v.is_some());
        let has_none = values.iter().any(|v| v.is_none());
        match (has_some, has_none) {
            (true, false) => ExecutionStatus::Success,
            (false, true) => ExecutionStatus::Failure,
            (true, true) => ExecutionStatus::Uncertainty,
            // if the mf vec is empty then this branch may be triggered, there is something wrong with that
            _ => unreachable!(),
        }
    }

    pub fn give_vote(tx_id: u32, chain_id: u16, status: ExecutionStatus, channel_id: String, env: &Env) -> StdResult<IbcMsg>{
        // must eliminate case ExecutionStatus::Uncertainty before entering the function
        let my_vote = Vote{
            tx_id,
            chain_id,
            success: match status {
                ExecutionStatus::Success=>true,
                ExecutionStatus::Failure=>false,
                _=>unreachable!(),
            },
        };
        let msg = IbcMsg::SendPacket {
            channel_id,
            data: to_json_binary(&my_vote)?,
            timeout: utils::get_timeout(env),
        };
        Ok(msg)
    }

    pub fn format_mf(mf: &Vec<Option<i64>>) -> String{
        mf
        .iter()
        .map(|item| {
            match item {
                Some(i)=>i.to_string(),
                None=>"None".to_string()
            }
        })
        .collect::<Vec<String>>()
        .join(",")
    }

    pub fn execute_tx(
        deps: &mut DepsMut,
        env: &Env,
        tx: FcrossTx,
    ) -> Result<(Vec<(String, String)>, Vec<CosmosMsg>), ContractError> {
        let mut attrs: Vec<(String, String)> = Vec::new();
        let mut msgs: Vec<CosmosMsg> = Vec::new();

        // pre-execution check
        // should not contain the same tx_id
        let history = HISTORY_TXS_LIST.load(deps.storage)?;
        if history.contains(&tx.tx_id){
            return Err(ContractError::ExecutionTxIdAlreadyExist { sent_id: tx.tx_id })
        }
        // if overflow append in the waiting list and return
        let mut pending = PENDING_TX_LIST.load(deps.storage)?;
        let pending_len = pending.len() as u32;
        if pending_len>=MAX_PENDING_LEN{
            // add to waiting list and return
            let mut waiting = WAITING_TX_LIST.load(deps.storage)?;
            waiting.push(tx.clone());
            WAITING_TX_LIST.save(deps.storage, &waiting)?;
            attrs.extend(vec![("lenth_overflow".to_string(), pending_len.to_string()), ("cached_tx".to_string(), format!("{:?}", tx))]);
            return Ok((attrs, msgs))
        }
        
        // log the start time of tx
        TIME_LOGS.save(deps.storage, tx.tx_id, &TimeInfo{
            start_time: env.block.time,
            end_time: None,
        })?;
        
        // execution
        let mut old_values = MF_MAP.load(deps.storage, history[history.len()-1])?;
        let new_values = old_values
        .iter()
        .map(|&item|{
            match item{
                None => None,
                Some(old_value) => {
                    match tx.operation {
                        Operation::DebitBalance { amount } => {
                            if old_value>=amount{
                                Some(old_value-amount)
                            } else {
                                None
                            }
                        },
                        Operation::CreditBalance { amount } => {
                            Some(old_value+amount)
                        },
                    }
                }
            }
        })
        .collect::<Vec<Option<i64>>>();

        // check if we can give instant voting
        let status = check_execution_stautus(&new_values, false);

        // post execution update
        old_values.extend(new_values);
        MF_MAP.save(deps.storage, tx.tx_id, &old_values)?;
        MF_VOTE_MAP.save(deps.storage, tx.tx_id, match status {
            ExecutionStatus::Success | ExecutionStatus::Failure => &true,
            ExecutionStatus::Uncertainty => &false,
        })?;
        pending.push(tx.tx_id);
        PENDING_TX_LIST.save(deps.storage, &pending)?;
        let mut history = HISTORY_TXS_LIST.load(deps.storage)?;
        history.push(tx.tx_id);
        HISTORY_TXS_LIST.save(deps.storage, &history)?;

        // response
        match status {
            ExecutionStatus::Success | ExecutionStatus::Failure => {
                let msg = give_vote(tx.tx_id, CHAIN_ID.load(deps.storage)?, status, MY_CHANNEL.load(deps.storage)?.channel_id, env)?;
                attrs.push(("voted".to_string(), "true".to_string()));
                msgs.push(CosmosMsg::Ibc(msg));
            },
            ExecutionStatus::Uncertainty => {
                attrs.push(("voted".to_string(), "false".to_string()));
            },
        };
        // attrs.extend(vec![("executed_tx".to_string(), tx.tx_id.to_string()), (format!("new_mf_{}", tx.tx_id), format_mf(&old_values))]);
        attrs.extend(vec![("executed_tx".to_string(), tx.tx_id.to_string()), (format!("new_mf_{}_len", tx.tx_id), old_values.len().to_string())]);
        Ok((attrs, msgs))
    }

    pub fn finalize_tx(
        deps: &mut DepsMut,
        env: &Env,
        instr: &Instruction,
    ) -> Result<(Vec<(String, String)>, Vec<CosmosMsg>), ContractError> {
        let mut msgs: Vec<CosmosMsg> = Vec::new();
        let mut attrs: Vec<(String, String)> = Vec::new();

        // pre-finalization check
        let mut pending = PENDING_TX_LIST.load(deps.storage)?;
        let pos = match pending.iter().position(|&x| {x==instr.tx_id}) {
            Some(i) => i,
            None => return Err(ContractError::FinalizationTxNotFound { sent_id: instr.tx_id, expected_id: pending }) 
        };

        // log the end time of tx
        let mut tl = TIME_LOGS.load(deps.storage, instr.tx_id)?;
        tl.end_time = Some(env.block.time);
        TIME_LOGS.save(deps.storage, instr.tx_id, &tl)?;

        // finalization
        let n = match instr.commitment {
            true => 1,
            false => 0,
        };
        let chain_id = CHAIN_ID.load(deps.storage)?;
        let channel_id = MY_CHANNEL.load(deps.storage)?.channel_id;
        let history = HISTORY_TXS_LIST.load(deps.storage)?;
        let partial_history = history.split_at(history.iter().position(|&x| x==instr.tx_id).unwrap()).1;
        for &i in partial_history {
            // update mfs
            let old_mf = MF_MAP.load(deps.storage, i)?;
            let updated_mf = old_mf
            .iter()
            .enumerate()
            .filter(|(j,_)|{(j >> pos) & 1 == n})
            .map(|(_, &v)| v)
            .collect::<Vec<Option<i64>>>();
            MF_MAP.save(deps.storage, i, &updated_mf)?;
            // add attrs for debug, however, this cost too much gas in ibc messages
            // attrs.push((format!("updated_mf_{}", i), format!("before_{},after_{}", format_mf(&old_mf),format_mf(&updated_mf))));
            attrs.push((format!("updated_mf_{}", i), format!("len_{}_to_{}", old_mf.len().to_string(), updated_mf.len().to_string())));

            // vote check
            let voted = MF_VOTE_MAP.load(deps.storage, i)?;
            // part of the pending tx are unvoted
            if !voted {
                // check execution status
                let status = check_execution_stautus(&updated_mf, true);
                match status {
                    ExecutionStatus::Success | ExecutionStatus::Failure => {
                        let msg = give_vote(i, chain_id, status, channel_id.clone(), env)?;
                        msgs.push(CosmosMsg::Ibc(msg));
                        attrs.push((format!("newly_voted_tx_{}", i), match status {
                            ExecutionStatus::Success=> "success".to_string(),
                            ExecutionStatus::Failure=> "failure".to_string(),
                            _=>unreachable!(),
                        }));
                        MF_VOTE_MAP.save(deps.storage, i, &true)?;
                    },
                    ExecutionStatus::Uncertainty => {},
                }
            }
        }

        // execute a new waiting tx
        // TODO: considering turn it into a sub-message that reply on error
        let mut waiting_txs = WAITING_TX_LIST.load(deps.storage)?;
        if waiting_txs.len()!=0{
            let next_tx = waiting_txs.remove(0);
            let tx_msg = to_json_binary(&ExecuteMsg::ExecuteTxs {
                fcross_txs: vec![next_tx],
            })?;
            msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: tx_msg,
                funds: vec![],
            }));
            // update waiting_tx_list
            WAITING_TX_LIST.save(deps.storage, &waiting_txs)?;
        }
        
        // post-finalization update
        pending.remove(pos);
        PENDING_TX_LIST.save(deps.storage, &pending)?;

        // resp
        attrs.extend(vec![("finalized_tx".to_string(), instr.tx_id.to_string()), ("committed".to_string(), instr.commitment.to_string())]);
        Ok((attrs, msgs))
    }
}