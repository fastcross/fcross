use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),

    #[error("chain {chain_id} has already voted for tx {tx_id}")]
    AlreadyVoted {
        tx_id: u32,
        chain_id: u16,
    },
    
    #[error("r1 vote unfinished for tx {0}")]
    StatusVoteUnfinished(u32),
    
    #[error("tx {0} are not allowed to submit r2 vote since its abort in r1 vote")]
    ValidityVoteNotAllowed(u32),

    #[error("r2 vote of tx {0} has already been closed")]
    TxClosed(u32),

    // #[error("vote for have already closed for tx {tx_id}")]
    // AlreadyClosed {
    //     tx_id: u32,
    // },
}