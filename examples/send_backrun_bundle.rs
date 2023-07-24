#![feature(lazy_cell)]
#![feature(once_cell_try)]
#![allow(dead_code)]

use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::utils::format_ether;
use ethers::utils::keccak256;
use mev_share_client::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;
use sugars::hset;
use tokio::try_join;
use utils::init_tracing;
mod utils;
use utils::{Config, Error, Result};

const NUM_TARGET_BLOCKS: u64 = 3;

/// Sends a tx on every block and backruns it with a simple example tx.
///
/// Continues until we land a backrun, then exits.
#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let (provider, sender_wallet, auth_wallet) = Config::from_env().await?.as_tuple();

    let client = MevShareClient::new(auth_wallet.clone(), Network::Goerli);

    // used for tracking txs we sent. we only want to backrun txs we sent.
    let target_txs = HashSet::<ethers::types::TxHash>::new();

    // prepare tasks dependencies
    let client = Arc::new(client);
    let target_txs = Arc::new(Mutex::new(target_txs));

    // this task makes sure that there's at least a transaction that we can backrun on every block
    let _send_tx_task = tokio::spawn({
        let client = Arc::clone(&client);
        let target_txs = Arc::clone(&target_txs);

        async move {
            let mut block_stream = provider.subscribe_blocks().await?;
            while let Some(block) = block_stream.next().await {
                if target_txs.lock()?.len() != 0 {
                    continue;
                }

                let tx: TypedTransaction = Eip1559TransactionRequest::new().into(); // TODO: fill tx
                let signed_tx = tx.rlp_signed(&sender_wallet.sign_transaction_sync(&tx)?);
                let txhash = client
                    .send_transaction(
                        signed_tx,
                        TransactionParams::builder()
                            .max_block_number(block.number.unwrap() + NUM_TARGET_BLOCKS)
                            .hints(hset![mev_share_client::TxHash, Calldata, Logs])
                            .build()?,
                    )
                    .await?;
                target_txs.lock()?.insert(txhash);
                tracing::info!("sent tx: {txhash:?}");
            }

            Result::Ok(())
        }
    });

    // this task listens the mev share server events for txs and backruns them
    let _backrun_task = tokio::spawn({
        let client = Arc::clone(&client);
        let target_txs = Arc::clone(&target_txs);

        async move {
            tracing::info!("listening for transactions...");

            let mut event_stream = client.subscribe(EventType::Transaction);
            while let Some(event) = event_stream.next().await {
                let pending_tx = event?.try_into()?;
                let backrun_result = backrun(pending_tx, target_txs.clone()).await;

                match backrun_result {
                    Err(err) => tracing::error!("{err}"),
                    Ok(_) => tracing::info!("backrun successful!"),
                }
            }

            Result::Ok(())
        }
    });

    // both tasks will run in parallel until user ctrl-c
    tokio::signal::ctrl_c().await.unwrap();

    Ok(())
}

/// Async handler which backruns an mev-share tx with another basic example tx.
pub async fn backrun(
    pending_tx: mev_share_client::PendingTransaction,
    target_txs: Arc<Mutex<HashSet<ethers::types::TxHash>>>,
) -> Result<()> {
    // ignore txs we didn't send.
    if !target_txs.lock()?.contains(&pending_tx.hash) {
        return Ok(());
    }

    // for testing, this is fine. in prod, you'll want an abstraction that manages these
    let (provider, sender_wallet, auth_wallet) = Config::from_env().await?.as_tuple();

    let client = MevShareClient::new(auth_wallet.clone(), Network::Goerli);

    let (start_block, nonce, fees) = try_join!(
        provider.get_block_number(),
        provider.get_transaction_count(sender_wallet.address(), None),
        provider.estimate_eip1559_fees(None),
    )?;

    // the transaction that will land immediately after the target, capturing the value that is left behind
    let backrun_tx = {
        let mut tx: TypedTransaction = Eip1559TransactionRequest::new()
            .from(sender_wallet.address())
            .to(sender_wallet.address())
            .value(U256::zero())
            .nonce(nonce)
            .data(b"im backrunniiiiing") // send bundle w/ (basefee + 100)gwei gas fee
            .max_fee_per_gas(fees.0)
            .max_priority_fee_per_gas(fees.1)
            .into();
        tx.set_nonce(tx.nonce().unwrap() + U256::one()); // in this example, we're sending the target from the same wallet
        tx.rlp_signed(&sender_wallet.sign_transaction_sync(&tx)?)
    };
    let backrun_txhash = keccak256(&backrun_tx);

    // compose the bundle
    let backrun_bundle = vec![
        // the tx to backrun
        Hash(pending_tx.hash),
        // the backrun tx
        Signed {
            tx: backrun_tx,
            can_revert: false,
        },
    ];

    let backrun_request = SendBundleParams::builder()
        .inclusion_block(start_block + 1)
        .inclusion_max_block(start_block + 1 + NUM_TARGET_BLOCKS)
        .body(backrun_bundle)
        .build()?;

    tracing::info!("sending backrun bundles targeting next {NUM_TARGET_BLOCKS} blocks...");
    dbg!(&backrun_request);

    let backrun_response = client.send_bundle(backrun_request.clone()).await;
    dbg!(&backrun_response);

    // checks for inclusion of `backrun_txhash` in the next `NUM_TARGET_BLOCKS`
    let mut blocks_subscription = provider.subscribe_blocks().await?;
    while let Some(block) = blocks_subscription.next().await {
        let block_number = block.number.unwrap();

        if block_number > start_block + NUM_TARGET_BLOCKS {
            target_txs.lock()?.remove(&pending_tx.hash);
            return Err(Error::BackrunTxDropped(NUM_TARGET_BLOCKS));
        }

        tracing::info!(
            "tx {} waiting for block {}",
            pending_tx.hash,
            block.number.unwrap()
        );

        // check for inclusion of backrun tx in target block
        let receipt = match provider.get_transaction_receipt(backrun_txhash).await? {
            Some(receipt) => receipt,
            None => continue, // not included yet
        };

        let status = match receipt.status {
            Some(status) => status,
            None => continue, // not included yet
        };

        if status != U64::one() {
            return Err(Error::BackrunTxReverted(receipt));
        }

        tracing::info!("bundle included! (found tx {})", receipt.transaction_hash);

        // simulate for funzies
        let sim_result = client
            .simulate_bundle(
                backrun_request,
                SimulateBundleParams::builder()
                    .parent_block(receipt.block_number.unwrap() - 1)
                    .build()?,
            )
            .await?;

        tracing::info!("profit: {} ETH", format_ether(sim_result.profit));

        return Ok(());
    }

    Ok(())
}
