use avalon::msg::*;
use cosmwasm_schema::write_api;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg
    }

    // generate a valid json
    let msg = ExecuteMsg::ExecuteTxs {
        fcross_txs: vec![FcrossTx{
            tx_id: 5,
            operation: Operation::CreditBalance { amount: 10 }
        },
        FcrossTx{
            tx_id: 7,
            operation: Operation::CreditBalance { amount: 13 }
        }],
    };
    let json = serde_json::to_string(&msg).unwrap();
    println!("{}", json);
}
