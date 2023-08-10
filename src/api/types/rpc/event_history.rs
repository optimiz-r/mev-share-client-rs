use super::super::Transaction;
use ethers::prelude::*;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

/// MEV-Share API response from '/history/info'. See [`crate::MevShareClient::get_event_history_info`].
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::module_name_repetitions)]
pub struct EventHistoryInfo {
    pub min_block: u64,
    pub max_block: u64,
    pub min_timestamp: u64,
    pub max_timestamp: u64,
    pub count: u32,
    pub max_limit: u32,
}

/// MEV-Share API parameteres for requests to '/history'. See [`crate::MevShareClient::get_event_history`].
#[derive(Clone, Serialize, Default, TypedBuilder, Debug)]
#[builder(field_defaults(default, setter(strip_option),))]
#[serde(rename_all = "camelCase")]
pub struct GetEventHistoryParams {
    pub block_start: Option<u64>,
    pub block_end: Option<u64>,
    pub timestamp_start: Option<u64>,
    pub timestamp_end: Option<u64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// MEV-Share API return from '/history'. See [`crate::MevShareClient::get_event_history`].
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventHistory {
    pub block: u64,
    pub timestamp: u64,
    pub hint: EventHint,
}

/// See [`EventHistory::hint`].
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventHint {
    pub txs: Option<Vec<Transaction>>,
    pub hash: H256,
    pub logs: Option<Vec<Log>>,
    pub gas_used: Option<U256>,
    pub mev_gas_price: Option<U256>,
}
