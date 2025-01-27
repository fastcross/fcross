use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),

    #[error("tx {0} to be finalized not found")]
    TxNotFound(u32),
    #[error("reach maximum pending transaction length {max_length}")]
    UpperBound{
        max_length: u32,
    },
}