pub use crate::api::types::JsonRpcResponseError;
use derive_builder::UninitializedFieldError;
use ethers::providers::ProviderError;
use std::{env::VarError, time::Duration};
use thiserror::Error;

/// The crate `Error` type.
#[derive(Error, Debug)]
pub enum Error {
    #[error("JsonRpcError: {0:?}")]
    JsonRpc(JsonRpcResponseError),

    #[error("Target transaction did not appear onchain before {0:?}")]
    TxTimeout(Duration),

    #[error("Error: {0}")]
    Custom(String),

    #[error(transparent)]
    Network(#[from] reqwest::Error),

    #[error(transparent)]
    EnvVar(#[from] VarError),

    #[error("Failed to deserialize into JSON: {text}")]
    Deserialization {
        error: serde_json::Error,
        text: String,
    },

    #[error(transparent)]
    Serialization(#[from] serde_json::Error),

    #[error(transparent)]
    Provider(#[from] ProviderError),

    #[error(transparent)]
    UnitializedField(#[from] UninitializedFieldError),

    #[error(transparent)]
    EventSource(#[from] reqwest_eventsource::Error),

    #[error(transparent)]
    Wallet(#[from] ethers::signers::WalletError),
}

/// The crate `Result` type.
pub type Result<T> = core::result::Result<T, Error>;
