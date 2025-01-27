use mf2::msg::*;
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
    let msg2 = QueryMsg::MyErrLogs {  };
    let json2 = serde_json::to_string(&msg2).unwrap();
    println!("{}", json2);
    let msg3 = QueryMsg::Multifuture { tx_id: 1 };
    let json3 = serde_json::to_string(&msg3).unwrap();
    println!("{}", json3);
}
