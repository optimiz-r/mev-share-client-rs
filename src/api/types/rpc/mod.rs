use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: i32,
    pub method: String,
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

/// Implementor will provider a builder (of a `#[derive(Builder])` struct) via the static `::builder()` method,
/// rather than forcing the user to import the builder struct itself.
///
/// Usage:
///
/// ```
/// impl_buildable!(Foo, FooBuilder);
/// ```
pub trait Buildable {
    type Builder;

    fn builder() -> Self::Builder;
}

macro_rules! impl_buildable {
    ($type:ty, $builder:ty) => {
        impl Buildable for $type {
            type Builder = $builder;

            fn builder() -> Self::Builder {
                <$builder>::default()
            }
        }
    };
}

impl_buildable!(GetEventHistoryParams, GetEventHistoryParamsBuilder);
impl_buildable!(TransactionParams, TransactionParamsBuilder);
impl_buildable!(SimulateBundleParams, SimulateBundleParamsBuilder);
impl_buildable!(SendBundleParams, SendBundleParamsBuilder);

mod event_history;
mod send_bundle;
mod simulate_bundle;

pub use event_history::*;
pub use send_bundle::*;
pub use simulate_bundle::*;
