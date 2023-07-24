use derive_builder::{Builder, UninitializedFieldError};
use ethers::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// Technically part of `simulation` but deserves it's own file because of how many sub-structs it has.

/// Parameters sent to [`mev_sendBundle`].
#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SendBundleParams {
    /// Smart bundle spec version
    pub version: String,
    /// Conditions for the bundle to be considered for inclusion in a block, evaluated _before_ the bundle is placed in a block
    pub inclusion: Inclusion,
    /// Transactions that make up the bundle. `hash` refers to a transaction hash from the Matchmaker event stream
    pub body: Vec<Body>,
    /// Conditions for bundle to be considered for inclusion in a block, evaluated _after_ the bundle is placed in the block
    pub validity: Option<Validity>,
    pub privacy: Option<Privacy>,
    pub metadata: Option<Metadata>,
}

#[derive(Default, Debug, Clone)]
pub struct SendBundleParamsBuilder {
    /// Smart bundle spec version
    version: Option<String>,
    /// Conditions for the bundle to be considered for inclusion in a block, evaluated _before_ the bundle is placed in a block
    inclusion: Option<Inclusion>,
    /// Transactions that make up the bundle. `hash` refers to a transaction hash from the Matchmaker event stream
    body: Option<Vec<Body>>,
    /// Conditions for bundle to be considered for inclusion in a block, evaluated _after_ the bundle is placed in the block
    validity: Option<Option<Validity>>,
    privacy: Option<Option<Privacy>>,
    metadata: Option<Option<Metadata>>,
}

impl SendBundleParamsBuilder {
    pub fn version(mut self, version: String) -> Self {
        self.version = Some(version);
        self
    }

    pub fn inclusion_block(mut self, block: U64) -> Self {
        self.inclusion.get_or_insert_with(Default::default).block = block;
        self
    }

    pub fn inclusion_max_block(mut self, max_block: U64) -> Self {
        self.inclusion
            .get_or_insert_with(Default::default)
            .max_block = Some(max_block);
        self
    }

    #[must_use]
    pub fn body(mut self, body: Vec<Body>) -> Self {
        self.body = Some(body);
        self
    }

    pub fn validitiy_refund(mut self, refund: Vec<Refund>) -> Self {
        self.validity
            .get_or_insert_with(|| Some(Default::default()))
            .as_mut()
            .unwrap()
            .refund = refund;
        self
    }

    pub fn validitiy_refund_config(mut self, refund_config: Vec<RefundConfig>) -> Self {
        self.validity
            .get_or_insert_with(|| Some(Default::default()))
            .as_mut()
            .unwrap()
            .refund_config = refund_config;
        self
    }

    pub fn privacy_hints(mut self, hints: HashSet<HintPreference>) -> Self {
        self.privacy
            .get_or_insert_with(|| Some(Default::default()))
            .as_mut()
            .unwrap()
            .hints = Some(hints);
        self
    }

    pub fn privacy_builders(mut self, builders: Vec<String>) -> Self {
        self.privacy
            .get_or_insert_with(|| Some(Default::default()))
            .as_mut()
            .unwrap()
            .builders = Some(builders);
        self
    }

    pub fn metadata_origin_id(mut self, origin_id: String) -> Self {
        self.metadata
            .get_or_insert_with(|| Some(Default::default()))
            .as_mut()
            .unwrap()
            .origin_id = Some(origin_id);
        self
    }

    pub fn build(self) -> std::result::Result<SendBundleParams, UninitializedFieldError> {
        Ok(SendBundleParams {
            version: self.version.unwrap_or("v0.1".to_string()),
            inclusion: self
                .inclusion
                .ok_or_else(|| UninitializedFieldError::new("inclusion"))?,
            body: self
                .body
                .ok_or_else(|| UninitializedFieldError::new("body"))?,
            validity: self.validity.unwrap_or_default(),
            privacy: self.privacy.unwrap_or_default(),
            metadata: self.metadata.unwrap_or_default(),
        })
    }
}

/// Conditions for bundle to be considered for inclusion in a block, evaluated _after_ the bundle is placed in the block.
#[derive(Clone, Serialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Validity {
    /// Conditions for receiving refunds (MEV kickbacks)
    pub refund: Vec<Refund>,
    /// Specifies how refund should be paid if bundle is used by another searcher
    pub refund_config: Vec<RefundConfig>,
}

/// Conditions for receiving refunds (MEV kickbacks).
#[derive(Clone, Serialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Refund {
    /// Index of entry in `body` to which the refund percentage applies.
    pub body_idx: u32,
    /// Minimum refund percentage required for this bundle to be eligible for use by another searcher.
    pub percent: u32,
}

/// Privacy settings for the submitted bundle.
#[derive(Clone, Serialize, Default, Debug)]
pub struct Privacy {
    pub hints: Option<HashSet<HintPreference>>,
    pub builders: Option<Vec<String>>,
}

#[derive(Clone, Serialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub origin_id: Option<String>,
}

#[derive(Clone, Serialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Inclusion {
    /// Target block number in which to include the bundle.
    pub block: U64,
    /// Maximum block height in which the bundle can be included.
    pub max_block: Option<U64>,
}

/// Specifies how refund should be paid if bundle is used by another searcher.
#[derive(Clone, Serialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RefundConfig {
    /// The address that receives this portion of the refund.
    pub address: Address,
    /// Percentage of refund to be paid to `address`.
    /// Set this to `100` unless splitting refunds between multiple recipients.
    pub percent: u32,
}

/// Transactions that make up the bundle.
/// `hash` refers to a transaction hash from the Matchmaker event stream.
#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase", untagged)]
pub enum Body {
    Hash(TxHash),
    Signed { tx: Bytes, can_revert: bool },
    Bundle(Box<SendBundleParams>),
}

/// Privacy settings for the submitted bundle.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "snake_case")]
pub enum HintPreference {
    /// Share the calldata of the transaction.
    Calldata,
    /// Share the contract address of the transaction.
    ContractAddress,
    /// Share the 4byte function selector of the transaction.
    FunctionSelector,
    /// Share the logs emitted by the transaction.
    Logs,
    /// Share tx hashes of transactions in bundle.
    TxHash,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SendBundleResponse {
    pub bundle_hash: TxHash,
}

/// Parameters accepted by the [`send_transaction`] function.
#[derive(Clone, Builder, Default, Debug, Serialize)]
#[builder(
    default,
    setter(strip_option),
    build_fn(error = "UninitializedFieldError")
)]
#[serde(rename_all = "camelCase")]
pub struct TransactionParams {
    /// Maximum block number for the transaction to be included in.
    pub max_block_number: Option<U64>,
    /// Hints define what data about a transaction is shared with searchers.
    pub hints: Option<HashSet<HintPreference>>,
    pub builders: Option<Vec<Address>>,
}
