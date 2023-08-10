use std::backtrace::Backtrace;

use crate::api::types::JsonRpcResponseError;
use ethers::{
    providers::ProviderError,
    types::{TransactionReceipt, TxHash, U256, U64},
};
use reqwest::header::InvalidHeaderValue;
use thiserror::Error;

/// The crate `Error` type.
#[derive(Error, Debug)]
pub enum Error {
    #[error("Bundle {0:?} did not appaear on-chain before maxBlock: {1}")]
    BundleTimeout(Vec<TxHash>, U64),

    #[error("Bundle {0:?} reverterd because of the following transactions")]
    BundleRevert(Vec<TransactionReceipt>),

    #[error("Bundle dropped: only partially appeared onchain. Receipts: {0:?}")]
    BundleDiscard(Vec<TransactionReceipt>),

    #[error("Transaction {0:?} did not appaear on-chain before maxBlock: {1}")]
    TransactionTimeout(TxHash, U64),

    #[error("Transaction {0:?} reverterd")]
    TransactionRevert(TransactionReceipt),

    #[error("UnsupportedNetwork: {0}")]
    UnsupportedNetwork(U256),

    #[error(transparent)]
    Json(#[from] JsonError),

    #[error(transparent)]
    Provider(#[from] ProviderError),

    #[error(transparent)]
    EventSource(#[from] reqwest_eventsource::Error),

    #[error(transparent)]
    Rpc(#[from] RpcError),

    #[error(transparent)]
    Rest(#[from] RestError),
}

#[derive(thiserror::Error, Debug)]
pub enum RpcError {
    #[error(transparent)]
    Json(#[from] JsonError),

    #[error("Error: {0:?}")]
    Response(JsonRpcResponseError),

    #[error(transparent)]
    Signing(#[from] ethers::signers::WalletError),

    #[error(transparent)]
    InvalidHeader(#[from] InvalidHeaderValue),

    #[error(transparent)]
    Network(#[from] reqwest::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum RestError {
    #[error(transparent)]
    Json(#[from] JsonError),

    #[error(transparent)]
    QueryDeserialization(#[from] serde_qs::Error),

    #[error(transparent)]
    Network(#[from] reqwest::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum JsonError {
    #[error("Failed to deserialize into JSON: {text}")]
    Deserialization {
        source: serde_json::Error,
        text: String,
    },

    #[error("{source} at {backtrace:#?}")]
    Serde {
        #[from]
        source: serde_json::Error,
        backtrace: Backtrace,
    },
}

macro_rules! impl_from_serde_json_error {
    ($error: ident) => {
        impl From<serde_json::Error> for $error {
            fn from(err: serde_json::Error) -> Self {
                Self::Json(err.into())
            }
        }
    };
}

impl_from_serde_json_error!(Error);
impl_from_serde_json_error!(RpcError);
impl_from_serde_json_error!(RestError);

/// The crate `Result` type.
pub type Result<T> = core::result::Result<T, Error>;
