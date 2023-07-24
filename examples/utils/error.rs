use derive_builder::UninitializedFieldError;
use ethers::{providers::ProviderError, types::TransactionReceipt, utils::ConversionError};
use std::sync::PoisonError;

/// The crate's [`Error`] type.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Backrun transaction assumed dropped because it has not been included for {0} blocks")]
    BackrunTxDropped(u64),

    #[error("Backrun transaction reverted with receipt {0:?}")]
    BackrunTxReverted(TransactionReceipt),

    #[error("Failed to parse ether {0}")]
    ParseEther(#[from] ConversionError),

    #[error("Mutex poisoned")]
    MutexPoisoned,

    #[error(transparent)]
    MevShareClient(#[from] mev_share_client::Error),

    #[error(transparent)]
    UninitializedField(#[from] UninitializedFieldError),

    #[error(transparent)]
    Config(#[from] envconfig::Error),

    #[error(transparent)]
    WalletDeserialization(#[from] ethers::signers::WalletError),

    #[error(transparent)]
    Provider(#[from] ProviderError),

    #[error(transparent)]
    Dotenv(#[from] dotenv::Error),
}

impl<T> From<PoisonError<T>> for Error {
    fn from(_: PoisonError<T>) -> Self {
        Self::MutexPoisoned
    }
}

/// The crate's [`Result`] type.
pub type Result<T> = core::result::Result<T, Error>;
