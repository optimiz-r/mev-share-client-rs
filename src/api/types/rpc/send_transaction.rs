use super::Privacy;
use crate::{Builder, Hint};
use ethers::prelude::*;
use serde::Serialize;
use std::collections::HashSet;
use typed_builder::TypedBuilder;

/// Parameters for RPC `eth_sendPrivateTransaction` requests. See [`crate::MevShareClient::send_private_transaction`].
#[derive(Clone, Default, Debug, Serialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct SendTransactionParams<'lt> {
    /// The signed transaction bytes.
    pub tx: Bytes,

    /// Maximum block number for the transaction to be included in.
    #[builder(default, setter(strip_option))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_block_number: Option<U64>,

    #[builder(default, setter(transform = |hints: Option<HashSet<Hint>>, builders: Option<HashSet<Builder<'lt>>>| Some(Preferences { fast: true, privacy: Privacy { hints, builders } })))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferences: Option<Preferences<'lt>>,
}

/// See [`SendTransactionParams`].
#[derive(Clone, Default, Debug, Serialize)]
pub struct Preferences<'lt> {
    pub fast: bool,
    pub privacy: Privacy<'lt>,
}
