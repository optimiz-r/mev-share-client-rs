use crate::{Error, Result};
use ethers::prelude::*;
use futures::future::try_join_all;
use tracing::*;

/// A helper module for waiting on transactions and bundles inclusion.
/// 
/// Internally used to implement [`crate::PendingBundle::inclusion`] and [`crate::PendingTransaction::inclusion`].
/// 
// TODO: Looks like some of this is already in `ethers_provider::FilterWatcher`, so maybe we can use that internally?
pub trait Waiter {
    /// Waits for a transaction to be included in a block.
    ///
    /// # Arguments
    ///
    /// * `hash` - Transaction hash.
    /// * `max_block` - Maximum block number to wait for.
    ///
    /// # Returns
    ///
    /// A tuple containing the [`Transaction`] and the block number it was included in.
    async fn wait_for_tx(&self, hash: TxHash, max_block: U64) -> Result<(Transaction, U64)>;

    /// Waits for a transaction to be included in a block.
    ///
    /// # Arguments
    ///
    /// * `hash` - Transaction hash.
    /// * `max_block` - Maximum block number to wait for.
    ///
    /// # Returns
    ///
    /// A tuple containing the [`TransactionReceipt`] and the block number it was included in.
    async fn wait_for_tx_receipt(
        &self,
        hash: TxHash,
        max_block: U64,
    ) -> Result<(TransactionReceipt, U64)>;

    /// Waits for a bundle to be included in a block.
    ///
    /// # Arguments
    ///
    /// * `hash` - Bundle hash.
    /// * `txs` - Transactions in the bundle..
    /// * `max_block` - Maximum block number to wait for.
    ///
    /// # Returns
    ///
    /// A tuple containing the [`TransactionReceipt`]s and the block number it was included in.
    async fn wait_for_bundle(
        &self,
        hash: TxHash,
        txs: Vec<TxHash>,
        max_block: U64,
    ) -> Result<(Vec<TransactionReceipt>, U64)>;
}

macro_rules! wait_for_tx {
    ($hash: ident, $max_block: ident, $provider: ident, $get_tx: ident) => {
        if let Some(tx) = $provider.$get_tx($hash).await? {
            let block = tx.block_number.unwrap();
            return Ok((tx, block));
        }

        let mut block_subscription = $provider.subscribe_blocks().await?;
        while let Some(block) = block_subscription.next().await {
            if let Some(tx) = $provider.$get_tx($hash).await? {
                return Ok((tx, block.number.unwrap()));
            }

            let block_number = block.number.unwrap();

            if block_number >= $max_block {
                return Err(Error::TransactionTimeout($hash, block_number));
            }
        }

        unreachable!("at each iteration, block number increases")
    };
}

impl Waiter for Provider<Ws> {
    /// See [`Waiter::wait_for_tx`]
    #[instrument(skip(self))]
    async fn wait_for_tx(&self, hash: TxHash, max_block: U64) -> Result<(Transaction, U64)> {
        wait_for_tx!(hash, max_block, self, get_transaction);
    }

    /// See [`Waiter::wait_for_tx_receipt`]
    #[instrument(skip(self))]
    async fn wait_for_tx_receipt(
        &self,
        hash: TxHash,
        max_block: U64,
    ) -> Result<(TransactionReceipt, U64)> {
        wait_for_tx!(hash, max_block, self, get_transaction_receipt);
    }

    /// See [`Waiter::wait_for_bundle`]
    #[instrument(skip(self, txs))]
    async fn wait_for_bundle(
        &self,
        hash: TxHash,
        txs: Vec<TxHash>,
        max_block: U64,
    ) -> Result<(Vec<TransactionReceipt>, U64)> {
        // checks whether the bundle has landed
        macro_rules! check_inclusion {
            () => {
                let receipts = fetch_receipts(self, &txs).await?;
                if receipts.len() > 0 {
                    let block = receipts.first().expect("len() > 0").block_number.unwrap();
                    return Ok((receipts, block));
                }
            };
        }

        // in case it's already landed
        check_inclusion!();

        // subscribe to blocks up to max_block and check for bundle to land
        let mut block_subscription = self.subscribe_blocks().await?;
        while let Some(block) = block_subscription.next().await {
            check_inclusion!();

            if let Some(block) = block.number && block > max_block {
                return Err(Error::BundleTimeout(txs, block));
            }
        }

        unreachable!("at each iteration, block number increases")
    }
}

async fn fetch_receipts(
    provider: &Provider<Ws>,
    hashes: &[TxHash],
) -> Result<Vec<TransactionReceipt>> {
    let receipts = try_join_all(
        hashes
            .iter()
            .map(|tx| provider.get_transaction_receipt(*tx)),
    )
    .await?
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if receipts.is_empty() {
        Ok(receipts)
    } else if receipts.len() < hashes.len() {
        // some tx landed but some didn't
        Err(Error::BundleDiscard(receipts))
    } else if receipts
        .iter()
        .filter(|r| r.status.unwrap() != U64::one())
        .count()
        > 0
    {
        // every tx landed, but there are reverts
        Err(Error::BundleRevert(receipts))
    } else {
        // every tx landed, no reverts
        Ok(receipts)
    }
}
