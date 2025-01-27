use cosmwasm_std::{Env, IbcTimeout};

pub fn future_index_to_string(index: u16) -> String {
    format!("{:b}", index)
    .chars()
    .rev()
    .collect()
}

pub fn keys_format(ks: &Vec<u32>) -> String {
    ks
    .iter()
    .map(|&i| format!("{:b}", i).chars().rev().collect())
    .collect::<Vec<String>>()
    .join(",")
}

pub fn get_timeout(env: &Env) -> IbcTimeout {
    let timeout = env.block.time.plus_seconds(3600000);
    IbcTimeout::with_timestamp(timeout)
}