use cosmwasm_std::StdError;
use thiserror::Error;

use crate::state::BATCH_NUM;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),

    #[error("unexpected validity instruction {received_commitment} of tx {received_tx_id} {:?}", expected_tx_ids)]
    UnexpectedValidityInstruction{
        received_tx_id: u32,
        received_commitment: bool,
        expected_tx_ids: Vec<u32>,
    },

    #[error("pending txs number should either be 0 or {}, got {0}", BATCH_NUM)]
    UnexpectedPendingTxsNumber(usize),

    #[error("the tx {0} has already prepared")]
    TxAlreadyPrepared(u32),

    #[error("the tx {0} has not prepared yet")]
    TxUnprepared(u32),

    #[error("the tx {0} has already finalized")]
    TxAlreadyFinalized(u32),
}