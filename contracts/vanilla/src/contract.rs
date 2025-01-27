use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, IbcMsg, MessageInfo, Response, StdResult
};
use crate::error::ContractError;
use crate::msg::{InstantiateMsg, QueryMsg, ExecuteMsg};
use crate::state::{CHAIN_ID, CURRENT_STATE, ERR_LOGS, HISTORY_TXS_LIST, PENDING_TX, WAITING_TX_LIST};

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    CHAIN_ID.save(deps.storage, &msg.chain_id)?;
    CURRENT_STATE.save(deps.storage, &msg.original_value)?;
    HISTORY_TXS_LIST.save(deps.storage, &vec![0])?;
    PENDING_TX.save(deps.storage, &None)?;
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
        WaitingList{} => to_json_binary(&query::waiting_list(deps)?),
        MyErrLogs{} => to_json_binary(&query::my_err_logs(deps)?),
        MyTimeLogs{} => to_json_binary(&query::my_time_logs(deps)?),
    }
}

mod query {
    use crate::{msg::{HistoryTxsResp, MyErrLogsResp, MyTimeLogsResp, WaitingListResp}, state::{TIME_LOGS, WAITING_TX_LIST}};

    use super::*;

    pub fn history_txs(deps: Deps) -> StdResult<HistoryTxsResp> {
        let history = HISTORY_TXS_LIST.load(deps.storage)?;
        Ok(HistoryTxsResp{ history })
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
    use cosmwasm_std::{CosmosMsg, IbcTimeout, WasmMsg};

    use crate::{error::ContractError, msg::{FcrossTx, Instruction}, state::{TimeInfo, CACHED_INSTRUCTIONS, CURRENT_STATE, MY_CHANNEL, PENDING_TX, TIME_LOGS, WAITING_TX_LIST}};

    use super::*;
    use crate::msg::{Operation, Vote};

    pub fn give_vote(tx_id: u32, chain_id: u16, status: bool, channel_id: String, env: &Env) -> StdResult<IbcMsg>{
        let my_vote = Vote{
            tx_id,
            chain_id,
            success: status,
        };
        let msg = IbcMsg::SendPacket {
            channel_id,
            data: to_json_binary(&my_vote)?,
            timeout: IbcTimeout::with_timestamp(env.block.time.plus_seconds(3600000)),
        };
        Ok(msg)
    }

    // pull new execution request from waiting list if exist
    // TODO: considering turn it into a sub-message that reply on error
    pub fn pull_execution_request(deps: &mut DepsMut, env: &Env) -> StdResult<Option<CosmosMsg>>{
        let mut waiting_txs = WAITING_TX_LIST.load(deps.storage)?;
        if waiting_txs.len()==0{
            return Ok(None)
        }
        let next_tx = waiting_txs.remove(0);
        WAITING_TX_LIST.save(deps.storage, &waiting_txs)?;

        let tx_msg = to_json_binary(&ExecuteMsg::ExecuteTxs {
            fcross_txs: vec![next_tx],
        })?;
        Ok(Some(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: tx_msg,
            funds: vec![],
        })))
    }

    pub fn new_finalization_request(deps: &mut DepsMut, env: &Env, tx_id: u32) -> StdResult<Option<CosmosMsg>>{
        // check if instruction is already received and cached
        let commitment = CACHED_INSTRUCTIONS.may_load(deps.storage, tx_id)?;
        match commitment {
            None => {
                Ok(None)
            },
            Some(commitment) => {
                CACHED_INSTRUCTIONS.remove(deps.storage, tx_id);
                let tx_msg = to_json_binary(&ExecuteMsg::FinalizeTx { instruction: Instruction{
                    tx_id,
                    commitment,
                }})?;
                Ok(Some(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: env.contract.address.to_string(),
                    msg: tx_msg,
                    funds: vec![],
                })))
            },
        }
    }

    pub fn execute_tx(
        deps: &mut DepsMut,
        env: &Env,
        tx: FcrossTx,
    ) -> Result<(Vec<(String, String)>, Vec<CosmosMsg>), ContractError> {
        let mut attrs: Vec<(String, String)> = Vec::new();
        let mut msgs: Vec<CosmosMsg> = Vec::new();
        let tx_id = tx.tx_id;

        // pre-execution check
        // should not contain the same tx_id
        let history = HISTORY_TXS_LIST.load(deps.storage)?;
        if history.contains(&tx_id){
            return Err(ContractError::ExecutionTxIdAlreadyExist { sent_id: tx_id })
        }
        // if already have pending tx, add to waiting_list and return
        let pending_tx = PENDING_TX.load(deps.storage)?;
        if pending_tx.is_some(){
            // add to waiting list and return
            let mut waiting = WAITING_TX_LIST.load(deps.storage)?;
            waiting.push(tx);
            WAITING_TX_LIST.save(deps.storage, &waiting)?;
            attrs.extend(vec![("state_locked".to_string(), pending_tx.unwrap().0.tx_id.to_string()), ("cached_tx".to_string(), tx_id.to_string())]);
            return Ok((attrs, msgs))
        }

        // log the start time of tx
        TIME_LOGS.save(deps.storage, tx_id, &TimeInfo{
            start_time: env.block.time,
            end_time: None,
        })?;

        // execute
        let current_value = CURRENT_STATE.load(deps.storage)?;
        let new_value = match tx.operation {
            Operation::CreditBalance { amount } => { Some(current_value+amount) },
            Operation::DebitBalance { amount } => {
                if current_value>=amount{
                    Some(current_value-amount)
                } else {
                    None
                }
            },
        };

        // add vote
        let vote_msg = give_vote(tx_id, CHAIN_ID.load(deps.storage)?, new_value.is_some(), MY_CHANNEL.load(deps.storage)?.channel_id, env)?;
        msgs.push(CosmosMsg::Ibc(vote_msg));

        // early abort
        if new_value.is_none() {
            // add to history
            let mut history = HISTORY_TXS_LIST.load(deps.storage)?;
            history.push(tx_id);
            HISTORY_TXS_LIST.save(deps.storage, &history)?;
            // log the end time of tx
            TIME_LOGS.save(deps.storage, tx_id, &TimeInfo{
                start_time: env.block.time,
                end_time: Some(env.block.time),
            })?;

            // pull next execution tx
            match pull_execution_request(deps, env)? {
                None => {},
                Some(new_execution_msg) => {
                    msgs.push(new_execution_msg);
                }, 
            }

            attrs.push(("early_abort".to_string(), tx_id.to_string()));
            return Ok((attrs, msgs))
        }

        // add to pending_tx
        let new_value = new_value.unwrap();
        PENDING_TX.save(deps.storage, &Some((tx, new_value)))?;

        // if instruction already received, finalize it instantly
        match new_finalization_request(deps, env, tx_id)? {
            None => {},
            Some(new_finalization_msg) => {
                msgs.push(new_finalization_msg);
            }
        }

        // response
        attrs.push(("executed_tx".to_string(), tx_id.to_string()));
        Ok((attrs, msgs))
    }

    // TODO: should be able to handle unordered instruction
    pub fn finalize_tx(
        deps: &mut DepsMut,
        env: &Env,
        instr: &Instruction,
    ) -> Result<(Vec<(String, String)>, Vec<CosmosMsg>), ContractError> {
        let mut msgs: Vec<CosmosMsg> = Vec::new();
        let mut attrs: Vec<(String, String)> = Vec::new();
        let tx_id = instr.tx_id;

        // pre-finalization check, check tx_id match
        let pending = PENDING_TX.load(deps.storage)?;
        if pending.is_none() || pending.clone().unwrap().0.tx_id != tx_id{
            // mismatch may oocur due to 1) early abort/old instruction 2) low execution rate/future instruction
            let history = HISTORY_TXS_LIST.load(deps.storage)?;
            if !history.contains(&tx_id) {
                CACHED_INSTRUCTIONS.save(deps.storage, tx_id, &instr.commitment.clone())?;
                attrs.push(("cached_instruction".to_string(), format!("{}-{}", tx_id, instr.commitment.clone())));
            }
            attrs.push((format!("mismatch_happened_{}", tx_id), format!("current_tx_{}", pending.map(|i| i.0.tx_id.to_string()).unwrap_or("None".to_string()))));
            return Ok((attrs, msgs))
        }
        let new_value = pending.unwrap().1;

        // finalization
        // update current state
        if instr.commitment {
            CURRENT_STATE.save(deps.storage, &new_value)?;
        }
        // update the history
        let mut history = HISTORY_TXS_LIST.load(deps.storage)?;
        history.push(tx_id);
        HISTORY_TXS_LIST.save(deps.storage, &history)?;
        // clear pending tx
        PENDING_TX.save(deps.storage, &None)?;
        // pull next execution tx
        match pull_execution_request(deps, env)? {
            None => {},
            Some(new_execution_msg) => {
                msgs.push(new_execution_msg);
            }, 
        }
        // log the end time of tx
        let mut tl = TIME_LOGS.load(deps.storage, tx_id)?;
        tl.end_time = Some(env.block.time);
        TIME_LOGS.save(deps.storage, tx_id, &tl)?;

        // resp
        attrs.extend(vec![("finalized_tx".to_string(), instr.tx_id.to_string()), ("committed".to_string(), instr.commitment.to_string())]);
        Ok((attrs, msgs))
    }
}