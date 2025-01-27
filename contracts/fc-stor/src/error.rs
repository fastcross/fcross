use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),

    #[error("execution tx id {sent_id} already exist")]
    ExecutionTxIdAlreadyExist {
        sent_id: u32,
    },
    #[error("expect finalization tx ids {:?} but got {sent_id}", expected_id)]
    FinalizationTxNotFound{
        sent_id: u32,
        expected_id: Vec<u32>,
    },
    #[error("reach maximum pending transaction length {max_length}")]
    UpperBound{
        max_length: u32,
    },
    
    // #[error("{sender} is not contract admin")]
    // Unauthorized { sender: Addr },
    // #[error("Payment error: {0}")]
    // PaymentError(#[from] PaymentError),
}