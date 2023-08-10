use crate::helpers::SelectorDeserializer;
use ethers::types::{Address, Bytes, Log, Selector, TxHash, U256};
use serde::Deserialize;
use serde_with::serde_as;

/// MEV-Share API response for subscription to the SSE bundles stream (via [`crate::MevShareClient::subscribe_bundles`])
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MevShareEvent {
    /// Transaction or Bundle hash
    pub hash: TxHash,
    /// Logs emitted by the transaction or bundle
    pub logs: Option<Vec<Log>>,
    /// Transactions included in the bundle.
    pub txs: Option<Vec<Transaction>>,
    /// Change in coinbase value after inserting tx/bundle, divided by gas used.
    ///
    /// Can be used to determine the minimum payment to the builder to make your backrun look more profitable to builders.
    /// _Note: this only applies to builders like Flashbots who order bundles by MEV gas price._
    pub mev_gas_price: Option<U256>,
    /// Gas used by the tx/bundle, rounded up to 2 most significant digi
    /// _Note: EXPERIMENTAL; only implemented on Goerli_
    pub gas_used: Option<U256>,
}

/// See [`MevShareEvent::txs`].
#[serde_as]
#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    /// Transaction recipient address
    pub to: Option<Address>,
    /// 4byte function selector
    #[serde_as(as = "Option<SelectorDeserializer>")]
    pub function_selector: Option<Selector>,
    /// Calldata of the tx
    pub call_data: Option<Bytes>,
}
