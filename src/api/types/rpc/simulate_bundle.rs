use derive_builder::{Builder, UninitializedFieldError};
use ethers::prelude::*;
use serde::{Deserialize, Serialize};

/// Optional fields to override simulation state.
#[derive(Serialize, Clone, Default, Builder, Debug)]
#[builder(
    default,
    setter(strip_option),
    build_fn(error = "UninitializedFieldError")
)]
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

/// Simulation details.
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
    pub logs: Option<Vec<BundleLogs>>,
}

/// Logs returned by `mev_simBundle`.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BundleLogs {
    pub tx_logs: Option<Vec<Log>>, // TODO: Is this actually ethers::Log?
    pub bundle_logs: Option<Vec<BundleLogs>>,
}
