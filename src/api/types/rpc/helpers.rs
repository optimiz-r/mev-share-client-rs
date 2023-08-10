use super::*;
use crate::helpers::provider::Waiter;
use crate::{Error, Result};
use derive_new::new;
use ethers::prelude::*;
use ethers::utils::keccak256;
use std::fmt::Display;
use std::slice::Iter;

/// A bundle that is pending inclusion.
///
/// See [`PendingBundle::inclusion`] for usage.
#[derive(new)]
pub struct PendingBundle<'lt> {
    /// Bundle hash.
    pub hash: TxHash,

    /// Bundle info.
    pub request: SendBundleParams<'lt>,

    /// Client to simulate the bundle with, in case it's necessary.
    pub provider: &'lt Provider<Ws>,
}

impl Display for PendingBundle<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.hash)
    }
}

impl PendingBundle<'_> {
    /// Returns a [`futures::Future`] that becomes [`std::task::Poll::Ready`] when the bundle lands on-chain.
    ///
    /// # Errors
    ///
    /// * [`Error::BundleTimeout`] if the bundle is not included in a block before `max_block`.
    /// * [`Error::BundleRevert`] if the bundle reverted.
    /// * [`Error::Provider`] if the provider fails to subscribe to fetch the [`TransactionReceipt`]s
    /// or to `subscribe_blocks` in order to to wait for them.
    pub async fn inclusion(self) -> Result<(Vec<TransactionReceipt>, U64)> {
        let txs = self.request.body.hashes().collect();
        let max_block = self
            .request
            .inclusion
            .max_block
            .unwrap_or(self.request.inclusion.block);

        self.provider
            .wait_for_bundle(self.hash, txs, max_block)
            .await
    }
}

/// Number of blocks to wait before the transaction is considered dropped.
///
/// By default, Flashbots will try to submit the transaction for 25 blocks.
/// See <https://docs.flashbots.net/flashbots-auction/searchers/advanced/private-transaction> for more info.
pub const TX_WAIT_MAX_BLOCKS: u64 = 25;

/// A private transaction that is pending inclusion.
///
/// See [`PendingTransaction::inclusion`] for usage.
#[derive(new)]
pub struct PendingTransaction<'lt> {
    /// Transaction hash.
    pub hash: TxHash,

    /// Maximum block number to wait for.
    pub max_block: Option<U64>,

    /// Client to simulate the bundle with, in case it's necessary.
    pub provider: &'lt Provider<Ws>,
}

impl Display for PendingTransaction<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.hash)
    }
}

impl PendingTransaction<'_> {
    /// Waits for transaction inclusion.
    ///
    /// # Returns
    ///
    /// A tuple containing the [`TransactionReceipt`] and the block number it was included in.
    ///
    /// # Errors
    ///
    /// * [`Error::TransactionTimeout`] if the transaction is not included in a block before `max_block`.
    /// * [`Error::TransactionRevert`] if the transaction reverted.
    /// * [`Error::Provider`] if the provider fails to subscribe to fetch the [`TransactionReceipt`]
    /// or to `subscribe_blocks` in order to to wait for them.
    pub async fn inclusion(&self) -> Result<(TransactionReceipt, U64)> {
        let max_block = match self.max_block {
            Some(block) => block,
            None => self.provider.get_block_number().await? + TX_WAIT_MAX_BLOCKS,
        };

        let (receipt, block) = self
            .provider
            .wait_for_tx_receipt(self.hash, max_block)
            .await?;

        if receipt.status.unwrap() != U64::one() {
            return Err(Error::TransactionRevert(receipt));
        }

        Ok((receipt, block))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashes() {
        let mut txs = [[0_u8; 64]; 7];
        let mut hashes = [TxHash::default(); 7];

        for i in 0..8 {
            txs[i][63] += u8::try_from(i).unwrap();
            hashes[i] = TxHash(keccak256(txs[i]));
        }

        let tx = |i: usize| Body::Signed {
            tx: txs[i].into(),
            can_revert: false,
        };

        let unsigned_tx = |i: usize| Body::Tx { hash: hashes[i] };

        let bundle = SendBundleParams {
            body: vec![
                tx(0),
                tx(1),
                unsigned_tx(2),
                Body::Bundle(Box::new(SendBundleParams {
                    body: vec![tx(3), tx(4), unsigned_tx(5)],
                    ..Default::default()
                })),
                tx(6),
            ],
            ..Default::default()
        };

        assert!(bundle.body.hashes().eq(hashes));
    }
}

/// Iterator over the hashes of a bundle body.
/// See [`HahsIter`] for usage.
pub struct BodyHashIterator<'lt> {
    stack: Vec<Iter<'lt, Body<'lt>>>,
}

impl<'lt> BodyHashIterator<'lt> {
    pub fn new(bodies: &'lt [Body]) -> Self {
        BodyHashIterator {
            stack: vec![bodies.iter()],
        }
    }
}

impl<'a> Iterator for BodyHashIterator<'a> {
    type Item = TxHash;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(bodies) = self.stack.last_mut() {
            match bodies.next() {
                Some(body) => match body {
                    Body::Tx { hash } => return Some(*hash),
                    Body::Signed { tx, .. } => return Some(keccak256(tx).into()),
                    Body::Bundle(bundle) => {
                        self.stack.push(bundle.body.iter());
                        continue;
                    }
                },
                None => {
                    self.stack.pop();
                    continue;
                }
            }
        }
        None
    }
}

/// Trait for iterating over the hashes of a bundle.
pub trait HashesIter {
    fn hashes(&self) -> BodyHashIterator;
}

impl HashesIter for Vec<Body<'_>> {
    fn hashes(&self) -> BodyHashIterator {
        BodyHashIterator::new(self.as_slice())
    }
}

impl HashesIter for &[Body<'_>] {
    fn hashes(&self) -> BodyHashIterator {
        BodyHashIterator::new(self)
    }
}
