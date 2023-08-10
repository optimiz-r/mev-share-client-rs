mod event_history;
mod helpers;
mod send_bundle;
mod send_transaction;
mod simulate_bundle;
mod stats;

pub use event_history::*;
pub use helpers::PendingTransaction;
pub use helpers::*;
pub use send_bundle::*;
pub use send_transaction::*;
pub use simulate_bundle::*;
pub use stats::*;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcRequest<'a> {
    pub jsonrpc: &'a str,
    pub id: i32,
    pub method: &'a str,
    pub params: Value,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum JsonRpcResponse<T> {
    Success(JsonRpcResponseSuccess<T>),
    Error(JsonRpcResponseError),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcResponseSuccess<T> {
    pub jsonrpc: String,
    pub id: i32,
    pub result: T,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcResponseError {
    pub jsonrpc: Option<String>,
    pub id: Option<i32>,
    pub error: Error,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Error {
    Simple(String),
    Detailed(JsonRpcResponseDetailedError),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcResponseDetailedError {
    code: i32,
    message: String,
}
