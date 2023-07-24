use ethers::types::{Address, Bytes, Log, Selector, TxHash, U256};
use serde::{Deserialize, Serialize};

/// General API wrapper for events received by the SSE stream (via `client.on(...)`)
///
/// Represents a pending bundle.
#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MevShareEvent {
    /// Transaction or Bundle hash
    pub hash: TxHash,
    /// Logs emitted by the transaction or bundle
    pub logs: Option<Vec<Log>>, // TODO: sure it can be None or just an empty Vec?
    /// Transactions included in the bundle.
    pub txs: Option<Vec<Transaction>>, // TODO: sure it can be None or just an empty Vec?
    /// Change in coinbase value after inserting tx/bundle, divided by gas used.
    ///
    /// Can be used to determine the minimum payment to the builder to make your backrun look more profitable to builders.
    /// _Note: this only applies to builders like Flashbots who order bundles by MEV gas price._
    pub mev_gas_price: Option<U256>, // hex string TODO: Use Bytes?
    /// Gas used by the tx/bundle, rounded up to 2 most significant digi
    /// _Note: EXPERIMENTAL; only implemented on Goerli_
    pub gas_used: Option<U256>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    /// Transaction recipient address
    pub to: Option<Address>,
    /// 4byte function selector
    pub function_selector: Option<Selector>,
    /// Calldata of the tx
    pub call_data: Option<Bytes>,
}

/// The different types of events that can be requested to the streaming server.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EventType {
    Bundle,
    Transaction,
}

/// The different types of events payload that can be received by the streaming server.
pub enum EventPayload {
    Bundle(MevShareEvent),
    Transaction(PendingTransaction),
}

impl TryInto<PendingTransaction> for EventPayload {
    type Error = crate::Error;

    fn try_into(self) -> Result<PendingTransaction, Self::Error> {
        match self {
            Self::Bundle(_) => Err(crate::Error::Custom(
                "Expected PendingTransaction, received a PendingBundle instead".to_string(),
            )),
            Self::Transaction(t) => Ok(t),
        }
    }
}

impl TryInto<MevShareEvent> for EventPayload {
    type Error = crate::Error;

    fn try_into(self) -> Result<MevShareEvent, Self::Error> {
        match self {
            Self::Bundle(b) => Ok(b),
            Self::Transaction(_) => Err(crate::Error::Custom(
                "Expected a PendingBundle received a PendingTransaction instead".to_string(),
            )),
        }
    }
}

/// Pending transaction from the MEV-Share event stream.
#[derive(Clone, Debug)]
pub struct PendingTransaction {
    pub hash: TxHash,
    pub logs: Option<Vec<Log>>,
    pub to: Option<Address>,
    pub function_selector: Option<Selector>,
    pub call_data: Option<Bytes>,
    pub mev_gas_price: Option<U256>,
    pub gas_used: Option<U256>,
}

impl From<MevShareEvent> for PendingTransaction {
    fn from(event: MevShareEvent) -> Self {
        let first_tx = event.txs.as_ref().and_then(|t| t.get(0));

        let to = first_tx.and_then(|tx| tx.to);
        let function_selector = first_tx.and_then(|tx| tx.function_selector);
        let call_data = first_tx.and_then(|tx| tx.call_data.as_ref().cloned());

        Self {
            to,
            function_selector,
            call_data,
            hash: event.hash,
            logs: event.logs,
            mev_gas_price: event.mev_gas_price,
            gas_used: event.gas_used,
        }
    }
}
