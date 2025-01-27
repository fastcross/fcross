use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),

    #[error("chain {chain_id} have already voted for tx {tx_id}")]
    AlreadyVoted {
        tx_id: u32,
        chain_id: u16,
    },

    #[error("vote for have already closed for tx {tx_id}")]
    AlreadyClosed {
        tx_id: u32,
    },
}