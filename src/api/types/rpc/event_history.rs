use super::super::Transaction;
use derive_builder::{Builder, UninitializedFieldError};
use ethers::prelude::*;
use serde::{Deserialize, Serialize};

/// Data about the event history endpoint.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventHistoryInfo {
    pub min_block: U64,
    pub max_block: U64,
    pub min_timestamp: U64,
    pub max_timestamp: U64,
    pub count: u32,
    pub max_limit: u32,
}

/// Arguments for the [`get_event_history`] function.
#[derive(Clone, Serialize, Default, Builder, Debug)]
#[builder(
    default,
    setter(strip_option),
    build_fn(error = "UninitializedFieldError")
)]
#[serde(rename_all = "camelCase")]
pub struct GetEventHistoryParams {
    pub block_start: Option<U64>,
    pub block_end: Option<U64>,
    pub timestamp_start: Option<U64>,
    pub timestamp_end: Option<U64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Data about an event from the [`get_event_history`] function.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventHistory {
    pub block: U64,
    pub timestamp: U64,
    pub hint: Hint,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Hint {
    pub txs: Option<Vec<Transaction>>,
    pub hash: H256,
    pub logs: Option<Vec<Log>>,
    pub gas_used: U256,
    pub mev_gas_price: U256,
}
