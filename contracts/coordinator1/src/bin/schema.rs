use coordinator1::msg::*;
use cosmwasm_schema::write_api;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg
    }

    // generate a valid json
    // let msg = ExecuteMsg::AddVote { vote: Vote { tx_id: 1, chain_id: 2, success: true } };
    let msg = InstantiateMsg{
        chain_num: 3,
    };
    let json = serde_json::to_string(&msg).unwrap();
    println!("{}", json);
    let msg2 = ExecuteMsg::AddVote { vote: Vote { tx_id: 2, chain_id: 1, success: true } };
    let json2 = serde_json::to_string(&msg2).unwrap();
    println!("{}", json2);
}
