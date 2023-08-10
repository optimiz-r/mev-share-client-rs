use super::*;
use ethers::types::{TxHash, Log};

impl MevShareEvent {
    /// Helps check whether this [`MevShareEvent`] instance is a transaction or not:
    /// a transaction appears as a bundle composed of a single transaction.
    #[must_use]
    pub fn as_transaction(&self) -> Option<(TxHash, Option<&Transaction>, Option<&Log>)> {
        let logs = self.logs.as_ref().and_then(|logs| logs.first());

        match &self.txs {
            None => Some((self.hash, None, logs)),
            Some(txs) if txs.len() == 1 => Some((self.hash, txs.first(), logs)),
            _ => None,
        }
    }
}
