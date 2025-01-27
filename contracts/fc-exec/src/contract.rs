use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, IbcMsg, MessageInfo, Response, StdResult
};
use crate::error::ContractError;
use crate::msg::{InstantiateMsg, QueryMsg, ExecuteMsg};
use crate::state::{CHAIN_ID, ERR_LOGS, HISTORY_TXS_LIST, MF_MAP, PENDING_TX_LIST, SECONDARY_PENDING_TX_LIST, WAITING_TX_LIST};

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    CHAIN_ID.save(deps.storage, &msg.chain_id)?;

    MF_MAP.save(deps.storage, &vec![Some(msg.original_value)])?;

    HISTORY_TXS_LIST.save(deps.storage, &Vec::new())?;
    PENDING_TX_LIST.save(deps.storage, &Vec::new())?;
    SECONDARY_PENDING_TX_LIST.save(deps.storage, &Vec::new())?;
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
        Multifuture{} => to_json_binary(&query::multifuture(deps)?),
        WaitingList {} => to_json_binary(&query::waiting_list(deps)?),
        AllLists {} => to_json_binary(&query::all_lists(deps)?),
        MyErrLogs{} => to_json_binary(&query::my_err_logs(deps)?),
        MyTimeLogs{} => to_json_binary(&query::my_time_logs(deps)?),
    }
}

mod query {

    use crate::{msg::{AllListsResp, MultifutureResp, MyErrLogsResp, MyTimeLogsResp, WaitingListResp}, state::{TIME_LOGS, WAITING_TX_LIST}};

    use super::*;

    pub fn multifuture(deps: Deps) -> StdResult<MultifutureResp> {
        let mf = MF_MAP.load(deps.storage)?
        .into_iter()
        .map(|i| {
            match i {
                None => "None".to_string(),
                Some(v) => v.to_string(),
            }
        })
        .collect::<Vec<String>>();
        Ok(MultifutureResp{futures: mf})
    }

    pub fn waiting_list(deps: Deps) -> StdResult<WaitingListResp> {
        let waiting_tx_list = WAITING_TX_LIST.load(deps.storage)?;
        Ok(WaitingListResp{ waiting_tx_list })
    }

    pub fn all_lists(deps: Deps) -> StdResult<AllListsResp> {
        let history_list = HISTORY_TXS_LIST.load(deps.storage)?;
        let pending_list = PENDING_TX_LIST.load(deps.storage)?;
        let secondary_pending_list = SECONDARY_PENDING_TX_LIST.load(deps.storage)?;
        let waiting_list = WAITING_TX_LIST.load(deps.storage)?;
        return Ok(AllListsResp{history_list, pending_list, secondary_pending_list, waiting_list});
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
    use cosmwasm_std::CosmosMsg;

    use crate::{error::ContractError, msg::{FcrossTx, Instruction}, state::{ScopeExecutedTx, TimeInfo, MAX_PENDING_LEN, MAX_SECONDARY_PENDING_LEN, MY_CHANNEL, TIME_LOGS, WAITING_TX_LIST}, utils};

    use super::*;
    use crate::msg::{Operation, Vote};

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum ExecutionStatus {
        Success,
        Failure,
        Uncertainty,
    }

    // default to trim the inherited, pos [0, len)
    pub fn check_execution_stautus(mfs: &Vec<Option<i64>>, pos: usize) -> ExecutionStatus{
        let values = &mfs[(1<<pos)..(1<<(pos+1))];
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

    pub fn generate_vote(tx_id: u32, chain_id: u16, status: ExecutionStatus, channel_id: String, env: &Env) -> StdResult<IbcMsg>{
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
        let tx_id = tx.tx_id;

        // we give no check to potentially same tx_id
        
        // check which list to insert the tx
        let mut pending = PENDING_TX_LIST.load(deps.storage)?;
        let pending_len = pending.len();
        if pending_len>=MAX_PENDING_LEN{
            // if pending_list is full
            let mut secondary = SECONDARY_PENDING_TX_LIST.load(deps.storage)?;
            if secondary.len()>=MAX_SECONDARY_PENDING_LEN {
                // if secondary_pending_list is also full
                let mut waiting = WAITING_TX_LIST.load(deps.storage)?;
                waiting.push(tx);
                WAITING_TX_LIST.save(deps.storage, &waiting)?;
                return Ok((attrs, msgs))
            }
            // else, scope execution
            
            let old_scope = match secondary.len()>0{
                false => {
                    let mfs = MF_MAP.load(deps.storage)?;
                    get_mfs_scope(&mfs)
                },
                true => {
                    secondary[secondary.len()-1].updated_scope
                }
            };
            let (new_scope, status) = scope_execute_app_logic(old_scope, &tx.operation);

            // log the executed time of tx
            TIME_LOGS.save(deps.storage, tx_id, &TimeInfo{
                start_time: env.block.time,
                end_time: None,
            })?;

            // update secondary_pending_list and history_list
            secondary.push(ScopeExecutedTx {
                tx,
                voted: match status {
                    ExecutionStatus::Success | ExecutionStatus::Failure => true,
                    ExecutionStatus::Uncertainty => false,
                },
                received_instruction: None,
                updated_scope: new_scope,
            });
            SECONDARY_PENDING_TX_LIST.save(deps.storage, &secondary)?;
            let mut history = HISTORY_TXS_LIST.load(deps.storage)?;
            history.push(tx_id);
            HISTORY_TXS_LIST.save(deps.storage, &history)?;

            // vote
            match status {
                ExecutionStatus::Success | ExecutionStatus::Failure => {
                    let msg = generate_vote(tx_id, CHAIN_ID.load(deps.storage)?, status, MY_CHANNEL.load(deps.storage)?.channel_id, env)?;
                    msgs.push(CosmosMsg::Ibc(msg));
                    attrs.push((format!("vote_{}", tx_id), (status==ExecutionStatus::Success).to_string()));
                },
                ExecutionStatus::Uncertainty => {},
            };

            attrs.extend(vec![("scope_executed_tx".to_string(), tx_id.to_string()), ("scope".to_string(), format!("{}-{}", new_scope.0, new_scope.1))]);
            return Ok((attrs, msgs))
        }

        // execution
        let mut mfs = MF_MAP.load(deps.storage)?;
        let new_values = execute_app_logic(&mfs, &tx.operation);
        // update mfs
        mfs.extend(new_values);
        MF_MAP.save(deps.storage, &mfs)?;

        // log the executed time of tx
        TIME_LOGS.save(deps.storage, tx_id, &TimeInfo{
            start_time: env.block.time,
            end_time: None,
        })?;

        // check if we can give instant voting
        let status = check_execution_stautus(&mfs, pending_len);

        // update pending_list and history_list
        pending.push((tx_id, match status {
            ExecutionStatus::Success | ExecutionStatus::Failure => true,
            ExecutionStatus::Uncertainty => false,
        }));
        PENDING_TX_LIST.save(deps.storage, &pending)?;
        let mut history = HISTORY_TXS_LIST.load(deps.storage)?;
        history.push(tx_id);
        HISTORY_TXS_LIST.save(deps.storage, &history)?;

        // vote
        match status {
            ExecutionStatus::Success | ExecutionStatus::Failure => {
                let msg = generate_vote(tx.tx_id, CHAIN_ID.load(deps.storage)?, status, MY_CHANNEL.load(deps.storage)?.channel_id, env)?;
                attrs.push((format!("vote_{}", tx_id), (status==ExecutionStatus::Success).to_string()));
                msgs.push(CosmosMsg::Ibc(msg));
            },
            ExecutionStatus::Uncertainty => {},
        };
        // attrs.extend(vec![("executed_tx".to_string(), tx.tx_id.to_string()), (format!("new_mf_{}", tx.tx_id), format_mf(&old_values))]);
        attrs.extend(vec![("executed_tx".to_string(), tx_id.to_string()), (format!("new_mf_{}_len", tx_id), mfs.len().to_string())]);
        Ok((attrs, msgs))
    }

    pub fn execute_app_logic(
        original_values: &Vec<Option<i64>>,
        operation: &Operation,
    ) ->  Vec<Option<i64>> {
        original_values.iter().map(|original_value| {
            match original_value {
                None => None,
                Some(v) => {
                    match operation {
                        Operation::DebitBalance { amount } => {
                            if v>=amount{
                                Some(v-amount)
                            } else {
                                None
                            }
                        },
                        Operation::CreditBalance { amount } => {
                            Some(v+amount)
                        },
                    }
                },
            }
        })
        .collect::<Vec<Option<i64>>>()
    }

    // return (new_merged_scope, ExecutionStatus)
    pub fn scope_execute_app_logic(
        old_scope: (i64, i64),
        operation: &Operation,
    ) -> ((i64, i64), ExecutionStatus) {
        // scope execution
        let new_scope = execute_app_logic(&vec![Some(old_scope.0), Some(old_scope.1)], operation);
        let (lower, upper) = (new_scope[0], new_scope[1]);

        // analyse new_scope
        match (lower, upper) {
            // certain cases, add to secondary_pending_list & give vote
            (Some(l), Some(u)) => { ((l.min(old_scope.0), u.max(old_scope.1)), ExecutionStatus::Success) },
            (None, Some(u)) => { ((0, u.max(old_scope.1)), ExecutionStatus::Uncertainty) },
            (None, None) => { (old_scope, ExecutionStatus::Failure) },
            (Some(_), None) => unreachable!(),
        }
    }

    pub fn get_mfs_scope(
        mfs: &Vec<Option<i64>>
    ) -> (i64, i64) {
        // derive scope from mfs
        // we expect there are at least one Some() in mfs
        let (mut min_value, mut max_value) = (100000, -100000);
        mfs.into_iter().filter(|i| i.is_some())
        .for_each(|i| {
            let i = i.unwrap();
            if i<min_value {
                min_value = i
            }
            if i>max_value {
                max_value = i
            }
        });
        (min_value, max_value)
    }

    pub fn flush_secondary_list(
        deps: &mut DepsMut, // actually no mut needed
        env: &Env,
        secondary: &mut Vec<ScopeExecutedTx>,
        start_pos: usize,
        start_scope: (i64, i64),
    ) -> Result<(Vec<(String, String)>, Vec<CosmosMsg>), ContractError> {
        let mut attrs: Vec<(String, String)> = Vec::new();
        let mut msgs: Vec<CosmosMsg> = Vec::new();

        let mut last_scope = start_scope;
        for i in start_pos..secondary.len(){
            // 1) derive updated_scope 2) give possible vote 3) set last_scope
            match secondary[i].received_instruction {
                Some(true) => {
                    let (updated_scope, _) = scope_execute_app_logic(last_scope, &secondary[i].tx.operation);
                    secondary[i].updated_scope = updated_scope;
                    // already committed, no vote necessary
                    last_scope = updated_scope;
                },
                Some(false) => {
                    secondary[i].updated_scope = last_scope;
                    // already aborted, no vote necessary
                    // last_scope no modification
                },
                None => {
                    let (updated_scope, status) = scope_execute_app_logic(last_scope, &secondary[i].tx.operation);
                    secondary[i].updated_scope = updated_scope;
                    if !secondary[i].voted {
                        match status {
                            ExecutionStatus::Success | ExecutionStatus::Failure => {
                                msgs.push(CosmosMsg::Ibc(generate_vote(secondary[i].tx.tx_id, CHAIN_ID.load(deps.storage)?, status, MY_CHANNEL.load(deps.storage)?.channel_id, env)?));
                                attrs.push((format!("vote_{}", secondary[i].tx.tx_id), (status==ExecutionStatus::Success).to_string()));
                                secondary[i].voted = true;
                            },
                            ExecutionStatus::Uncertainty => {},
                        }
                    }
                    last_scope = updated_scope;
                },
            }
        }

        Ok((attrs, msgs))
    }

    pub fn finalize_tx(
        deps: &mut DepsMut,
        env: &Env,
        instr: &Instruction,
    ) -> Result<(Vec<(String, String)>, Vec<CosmosMsg>), ContractError> {
        let mut msgs: Vec<CosmosMsg> = Vec::new();
        let mut attrs: Vec<(String, String)> = Vec::new();
        let tx_id = instr.tx_id;
        attrs.extend(vec![("finalized_tx".to_string(), instr.tx_id.to_string()), ("committed".to_string(), instr.commitment.to_string())]);

        // check if the tx is in pending_list/secondary_pending_list
        let mut pending = PENDING_TX_LIST.load(deps.storage)?;
        let pos = pending.iter().position(|&x| {x.0==tx_id});
        if pos.is_none() {
            // find in the secondary_pending_list
            let mut secondary = SECONDARY_PENDING_TX_LIST.load(deps.storage)?;
            let pos = secondary.iter().position(|x| {x.tx.tx_id==tx_id});
            if pos.is_none() {
                return Err(ContractError::TxNotFound(tx_id))
            }
            // finalization happened in the secondary_pending_list
            let pos = pos.unwrap();

            // log the end time of tx
            let mut tl = TIME_LOGS.load(deps.storage, instr.tx_id)?;
            tl.end_time = Some(env.block.time);
            TIME_LOGS.save(deps.storage, instr.tx_id, &tl)?;

            secondary[pos].received_instruction = Some(instr.commitment);

            // flush secondary_list from index pos
            let old_scope = match pos==0{
                false => {
                    secondary[pos-1].updated_scope
                },
                true => {
                    let mfs = MF_MAP.load(deps.storage)?;
                    get_mfs_scope(&mfs)
                }
            };
            let (attr, msg) = flush_secondary_list(deps, env, &mut secondary, pos, old_scope)?;
            attrs.extend(attr);
            msgs.extend(msg);

            // save secondary_list
            SECONDARY_PENDING_TX_LIST.save(deps.storage, &secondary)?;

            // resp
            attrs.extend(vec![("finalized_scope_tx".to_string(), instr.tx_id.to_string()), ("committed".to_string(), instr.commitment.to_string())]);
            return Ok((attrs, msgs))
        }
        // finalization happened in the pending_list
        let pos = pos.unwrap();

        // log the end time of tx
        let mut tl = TIME_LOGS.load(deps.storage, instr.tx_id)?;
        tl.end_time = Some(env.block.time);
        TIME_LOGS.save(deps.storage, instr.tx_id, &tl)?;

        // finalization
        // update mf_map
        let n = match instr.commitment {
            true => 1,
            false => 0,
        };
        let mut mfs = MF_MAP.load(deps.storage)?;
        mfs = mfs
        .into_iter()
        .enumerate()
        .filter(|(j,_)|{(j >> pos) & 1 == n})
        .map(|(_, v)| v)
        .collect::<Vec<Option<i64>>>();
        // update pending_list
        pending.remove(pos);

        // give possible vote
        for i in pos..pending.len() {
            // ignore the voted
            if pending[i].1 {
                continue;
            }
            let status = check_execution_stautus(&mfs, i);
            match status {
                ExecutionStatus::Success | ExecutionStatus::Failure => {
                    let msg = generate_vote(pending[i].0, CHAIN_ID.load(deps.storage)?, status, MY_CHANNEL.load(deps.storage)?.channel_id, env)?;
                    msgs.push(CosmosMsg::Ibc(msg));
                    attrs.push((format!("vote_{}", tx_id), (status==ExecutionStatus::Success).to_string()));
                    pending[i] = (pending[i].0, true);
                },
                ExecutionStatus::Uncertainty => {},
            }
        }

        // apply the leading txs that has been finalized in secondary_pending_list
        let mut secondary = SECONDARY_PENDING_TX_LIST.load(deps.storage)?;
        let p = secondary.iter().position(|i| i.received_instruction.is_none());
        if p.is_some() {
            for _ in 0..p.unwrap() {
                let stx = secondary.remove(0);
                let commit = stx.received_instruction.unwrap();
                if commit {
                    mfs = execute_app_logic(&mfs, &stx.tx.operation);
                }
            }
        }

        let mut waiting = WAITING_TX_LIST.load(deps.storage)?;
        if secondary.len()!=0 || waiting.len()!=0 {
            // pull new tx in secondary_pending_list/waiting_list to fill the vacancy of pending_list
            let (new_tx, voted) = match secondary.len()>0 {
                true => {
                    let stx = secondary.remove(0);
                    (stx.tx, stx.voted)
                },
                false => {
                    let tx = waiting.remove(0);
                    // note any tx from the waiting_list have to add to time_log first
                    TIME_LOGS.save(deps.storage, tx.tx_id, &TimeInfo{
                        start_time: env.block.time,
                        end_time: None,
                    })?;
                    (tx, false)
                },
            };
            // execute
            mfs.extend(execute_app_logic(&mfs, &new_tx.operation));
            // vote & update pending_list
            let status = check_execution_stautus(&mfs, pending.len());
            match status {
                ExecutionStatus::Success | ExecutionStatus::Failure => {
                    if !voted {
                        let msg = generate_vote(new_tx.tx_id, CHAIN_ID.load(deps.storage)?, status, MY_CHANNEL.load(deps.storage)?.channel_id, env)?;
                        msgs.push(CosmosMsg::Ibc(msg));
                        attrs.push((format!("vote_{}", tx_id), (status==ExecutionStatus::Success).to_string()));
                    }
                    pending.push((new_tx.tx_id, true));
                },
                ExecutionStatus::Uncertainty => {
                    pending.push((new_tx.tx_id, false));
                },
            };

            // pull new txs in waiting_list to fill the vacancy of secondary_pending_list
            let num_transferred = (MAX_SECONDARY_PENDING_LEN-secondary.len()).min(waiting.len());
            for _ in 0..num_transferred {
                let tx = waiting.remove(0);
                TIME_LOGS.save(deps.storage, tx.tx_id, &TimeInfo{
                    start_time: env.block.time,
                    end_time: None,
                })?;
                secondary.push(ScopeExecutedTx{
                    tx,
                    updated_scope: (0, 0),
                    voted: false,
                    received_instruction: None,
                });
            }
            let (attr, msg) = flush_secondary_list(deps, env, &mut secondary, 0, get_mfs_scope(&mfs))?;
            attrs.extend(attr);
            msgs.extend(msg);

            // update waiting_tx_list
            WAITING_TX_LIST.save(deps.storage, &waiting)?;
        }

        MF_MAP.save(deps.storage, &mfs)?;
        PENDING_TX_LIST.save(deps.storage, &pending)?;
        SECONDARY_PENDING_TX_LIST.save(deps.storage, &secondary)?;

        Ok((attrs, msgs))
    }
}