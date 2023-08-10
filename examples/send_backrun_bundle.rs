#![feature(lazy_cell)]
#![feature(let_chains)]
#![feature(impl_trait_projections)]
#![allow(dead_code)]

use ethers::prelude::*;
use ethers::utils::keccak256;
use ethers::utils::parse_ether;
use eyre::Result;
use mev_share_rs::prelude::*;
use mev_share_rs::SendTransactionParams;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::try_join;
use tracing::*;

mod common;
use common::{init_tracing, Config, MockTx, BUILDERS};

const INCLUSION_BLOCKS: u64 = 10;

/// Sends a tx on every block and backruns it with a simple example tx.
#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let c = Config::from_env().await?;
    Executor::new(c.provider.clone(), c.auth_wallet.clone())
        .await?
        .run()
        .await?;

    Ok(())
}

struct Executor<'a> {
    provider: Provider<Ws>,
    client: MevShareClient<'a>,

    // used for tracking txs we sent. we only want to backrun txs we sent.
    target_txs: Arc<Mutex<HashSet<TxHash>>>,
}

impl<'a> Executor<'a> {
    pub async fn new(provider: Provider<Ws>, auth_wallet: LocalWallet) -> Result<Self> {
        Ok(Self {
            client: MevShareClient::<'a>::new(auth_wallet, provider.clone()).await?,
            provider,
            target_txs: Default::default(),
        })
    }

    pub async fn run(&self) -> Result<()> {
        // wait for both tasks
        try_join!(
            // this task makes sure that there's at least a transaction that we can backrun on every block
            self.send_tx_task(),
            // this task listens MEV-Share events for txs and backruns them
            self.backrun_task()
        )?;

        Ok(())
    }

    #[instrument(skip_all)]
    async fn send_tx_task(&self) -> Result<()> {
        info!("listening for blocks");

        let mut block_stream = self.provider.subscribe_blocks().await?;

        while let Some(block) = block_stream.next().await {
            let block_number = block.number.unwrap();
            debug!(number = ?block_number, "received block");

            // hold the lock until the we can read the tx hash from the response:
            // if the backrun_task receives it before we have time to check it here,
            // it will think it's not a target and discard it
            let mut target_txs = self.target_txs.lock().await;

            // if we have a tx we can backrun, don't bother creating more
            if target_txs.len() != 0 {
                continue;
            }

            info!(?block_number, "sending tx");

            let sent_tx = self
                .client
                .send_private_transaction(
                    SendTransactionParams::builder()
                        .tx(MockTx::default().data(b"plz backrun me").build().await?)
                        .max_block_number(block_number + 1 + INCLUSION_BLOCKS)
                        .preferences(None, Some(BUILDERS.clone()))
                        .build(),
                )
                .await?;

            info!(hash = ?sent_tx.hash, "sent tx");

            target_txs.insert(sent_tx.hash);
        }

        Ok(())
    }

    #[instrument(skip_all)]
    async fn backrun_task(&self) -> Result<()> {
        info!("listening for transactions");

        let mut mev_share_stream = self.client.subscribe_bundles();

        while let Some(event) = mev_share_stream.next().await {
            let bundle = event?; // upstream errors in event

            if let Some((hash, hints, logs)) = bundle.as_transaction() {
                debug!(?hash, ?hints, ?logs, "received transaction");

                info_span!("tx", ?hash);

                if self.target_txs.lock().await.contains(&hash) {
                    info!("tx is backrunnable");

                    self.backrun(hash).await?;
                    info!("backrunned tx");

                    self.target_txs.lock().await.remove(&hash);
                }
            } else {
                debug!(?bundle, "received");
            }
        }

        Ok(())
    }

    /// Async handler which backruns an mev-share tx with another basic example tx.
    async fn backrun(&self, target_tx: TxHash) -> Result<()> {
        // for testing, this is fine. in prod, you'll want an abstraction that manages these
        let current_block = self.provider.get_block_number().await?;

        // the transaction that will land immediately after the target, capturing the value that is left behind
        let backrun_tx = MockTx::default()
            .data(b"im backrunniiiiiiing")
            .tip((parse_ether("0.0002")?, parse_ether("0.00002")?))
            // tx has yet to land and only private relay knows about it:
            // provider's nonce will have to be incremented by 1
            .nonce_add(1)
            .build()
            .await?;

        debug!(
            hash = ?TxHash::from(keccak256(&backrun_tx)),
            "made backrun tx"
        );

        let backrun_bundle_request = SendBundleParams::builder()
            .inclusion(
                current_block + 1,
                Some(current_block + 1 + INCLUSION_BLOCKS),
            )
            .body(vec![
                // the tx to backrun, from the MEV-Share SSE
                Body::Tx { hash: target_tx },
                // our backrun tx
                Body::Signed {
                    tx: backrun_tx,
                    can_revert: false,
                },
            ])
            // .validity(vec![], vec![])
            .privacy(None, Some(BUILDERS.clone()))
            .build();

        info!("simulating backrun bundle");

        info!(
            params = %serde_json::to_string(&backrun_bundle_request).unwrap(),
            "sending backrun bundle",
        );

        let pending_bundle = self
            .client
            .send_bundle(backrun_bundle_request.clone())
            .await?;

        info!(hash = ?pending_bundle.hash, "bundle accepted by the relayer, waiting for landing");

        pending_bundle.inclusion().await?;

        Ok(())
    }
}
