use ethers::prelude::*;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

/// MEV-Share API parameters for RPC `mev_simBundle` requests:
/// optional fields to override simulation state
///
/// See [`crate::MevShareClient::simulate_bundle`].
#[derive(Serialize, Clone, Default, TypedBuilder, Debug)]
#[builder(field_defaults(default, setter(strip_option)))]
#[serde(rename_all = "camelCase")]
pub struct SimulateBundleParams {
    /// Block used for simulation state. Defaults to latest block.
    /// Block header data will be derived from parent block by default.
    /// Specify other params in this interface to override the default values.
    pub parent_block: Option<U64>,
    // override the default values for the parentBlock header
    /// default = parentBlock.number + 1.
    pub block_number: Option<U64>,
    /// default = parentBlock.coinbase.
    pub coinbase: Option<Address>,
    /// default = parentBlock.timestamp + 12.
    pub timestamp: Option<U64>,
    /// default = parentBlock.gasLimit.
    pub gas_limit: Option<U64>,
    /// default = parentBlock.baseFeePerGas.
    pub base_fee: Option<U256>,
    /// default = 5 (defined in seconds).
    pub timeout: Option<u64>,
}

/// MEV-Share API response for RPC `mev_simBundle` requests:
/// simulation details.
/// .
/// See [`crate::MevShareClient::simulate_bundle`].
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SimulateBundleResponse {
    pub success: bool,
    pub error: Option<String>,
    pub state_block: U64,
    pub mev_gas_price: U256,
    pub profit: U256,
    pub refundable_value: U256,
    pub gas_used: U256,
    pub logs: Vec<BundleLogs>,
}

/// See [`SimulateBundleResponse::logs`].
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BundleLogs {
    pub tx_logs: Option<Vec<Log>>,
    pub bundle_logs: Option<Vec<BundleLogs>>,
}
