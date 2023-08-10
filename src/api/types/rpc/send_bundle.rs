use ethers::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use typed_builder::TypedBuilder;

/// Parameters for RPC `mev_sendBundle` requests. See [`crate::MevShareClient::send_bundle`].
#[derive(Clone, Serialize, Deserialize, Debug, Default, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct SendBundleParams<'lt> {
    /// Smart bundle spec version
    #[builder(default = "v0.1")]
    pub version: &'lt str,

    /// Conditions for the bundle to be considered for inclusion in a block, evaluated _before_ the bundle is placed in a block
    #[builder(setter(transform = |block: U64, max_block: Option<U64>|  Inclusion { block, max_block }))]
    pub inclusion: Inclusion,

    /// Transactions that make up the bundle. `hash` refers to a transaction hash from the MevShare event stream
    pub body: Vec<Body<'lt>>,

    /// Conditions for bundle to be considered for inclusion in a block, evaluated _after_ the bundle is placed in the block
    #[builder(default, setter(transform = |refund: Vec<Refund>, refund_config: Vec<RefundConfig>| Some(Validity { refund, refund_config })))]
    pub validity: Option<Validity>,

    /// Privacy settings. See [`Hint`] and [`Builder`] for more info.
    #[builder(default, setter(transform = |hints: Option<HashSet<Hint>>, builders: Option<HashSet<Builder<'lt>>>| Some(Privacy { hints, builders })))]
    pub privacy: Option<Privacy<'lt>>,

    #[builder(default, setter(transform = |origin_id: &'lt str| Some(Metadata { origin_id: Some(origin_id) })))]
    pub metadata: Option<Metadata<'lt>>,
}

/// Response for RPC `mev_sendBundle` requests. See [`crate::MevShareClient::send_bundle`].
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SendBundleResponse {
    pub bundle_hash: TxHash,
}

/// See [`SendBundleParams::validity`].
#[derive(Clone, Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Validity {
    /// Conditions for receiving refunds (MEV kickbacks)
    pub refund: Vec<Refund>,
    /// Specifies how refund should be paid if bundle is used by another searcher
    pub refund_config: Vec<RefundConfig>,
}

/// See [`Validity::refund`].
#[derive(Clone, Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Refund {
    /// Index of entry in `body` to which the refund percentage applies.
    pub body_idx: u32,
    /// Minimum refund percentage required for this bundle to be eligible for use by another searcher.
    pub percent: u32,
}

/// See [`SendBundleParams::privacy`].
#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct Privacy<'lt> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<HashSet<Hint>>,
    #[serde(skip_serializing_if = "Option::is_none", borrow)]
    pub builders: Option<HashSet<Builder<'lt>>>,
}

/// List of builders to share transactions/bundles with that are currently [supported by Flashbots].
///
/// ## Usage:
///
/// ```
/// let bundle = BundleParams::builder()
///     ...
///     .privacy(/* hints */, Some(set![
///         Builder::Flashbots,
///         Builder::Rsync,
///         Builder::Other("a non-flashbots builder")
///     ])
///     .build();
/// ```
///
/// [supported by Flashbots]: https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#mev_sendbundle
#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Builder<'lt> {
    Default,
    Flashbots,
    Rsync,
    #[serde(rename = "beaverbuild.org")]
    BeaverBuild,
    Builder0x69,
    #[serde(rename = "Titan")]
    Titan,
    #[serde(rename = "EigenPhi")]
    EigenPhi,
    #[serde(rename = "boba-builder")]
    BobaBuilder,
    Other(&'lt str),
}

/// See [`SendBundleParams::metadata`].
#[derive(Clone, Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Metadata<'lt> {
    pub origin_id: Option<&'lt str>,
}

/// See [`SendBundleParams::inclusion`].
#[derive(Clone, Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Inclusion {
    /// Target block number in which to include the bundle.
    pub block: U64,
    /// Maximum block height in which the bundle can be included.
    pub max_block: Option<U64>,
}

/// See [`Validity::refund_config`].
#[derive(Clone, Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RefundConfig {
    /// The address that receives this portion of the refund.
    pub address: Address,
    /// Percentage of refund to be paid to `address`.
    /// Set this to `100` unless splitting refunds between multiple recipients.
    pub percent: u32,
}

/// Transactions that make up the bundle.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", untagged)]
pub enum Body<'lt> {
    // A transaction hash from the MEV-Share event stream.
    Tx {
        hash: TxHash,
    },
    // A signed transaction.
    Signed {
        tx: Bytes,
        can_revert: bool,
    },
    // A nested bundle
    #[serde(borrow)]
    Bundle(Box<SendBundleParams<'lt>>),
}

/// Privacy settings for the submitted bundle.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Hint {
    /// Share the calldata of the transaction.
    Calldata,
    /// Share the contract address of the transaction.
    ContractAddress,
    /// Share the 4byte function selector of the transaction.
    FunctionSelector,
    /// Share the logs emitted by the transaction.
    Logs,
    /// Share tx hashes of the transactions in the bundle.
    TxHash,
    TransactionHash,
    /// Share the hash of the bundle/transaction being sent.
    Hash,
}
