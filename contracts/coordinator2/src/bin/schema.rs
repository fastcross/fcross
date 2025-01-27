use coordinator2::msg::*;
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
}
