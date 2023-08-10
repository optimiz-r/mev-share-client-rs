#![feature(lazy_cell)]
#![feature(let_chains)]
#![allow(dead_code)]

use ethers::{prelude::*, types::transaction::eip2718::TypedTransaction};
use mev_share_rs::prelude::*;
use mev_share_rs::MevShareClient;
use mev_share_rs::SendTransactionParams;
use tokio::try_join;
use tracing::*;

mod common;
use common::{init_tracing, Config};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    init_tracing();

    let c = Config::from_env().await?;

    let provider = &c.provider;

    let (chain_id, fees, nonce, block_number) = try_join!(
        provider.get_chainid(),
        provider.estimate_eip1559_fees(None),
        provider.get_transaction_count(c.sender_wallet.address(), None),
        provider.get_block_number()
    )?;

    // if you init the client with a `chain_id`, the initialization is not `async`
    // no additional network requests performed
    let client =
        MevShareClient::new_with_chain_id(c.auth_wallet.clone(), provider.clone(), chain_id)?;

    debug!(?block_number);
    debug!(?fees);

    let tx: TypedTransaction = Eip1559TransactionRequest::default()
        .chain_id(chain_id.as_u64())
        .from(c.sender_wallet.address())
        .to(c.sender_wallet.address())
        .nonce(nonce)
        .gas(22_000)
        .data(b"i'm shariiiiiing")
        .max_fee_per_gas(fees.0)
        .max_priority_fee_per_gas(fees.1)
        .into();

    // check that simulation doesn't fail
    provider.call_raw(&tx).await?;

    let tx_request = SendTransactionParams::builder()
        .tx(tx.rlp_signed(&c.sender_wallet.sign_transaction_sync(&tx)?))
        .preferences(Some(set![Hint::Calldata, Hint::TransactionHash]), None)
        .build();

    loop {
        debug!("sending private tx to Flashbots MEV-Share API");

        let pending_tx = client.send_private_transaction(tx_request.clone()).await?;

        debug_span!("tx", hash = ?pending_tx.hash);

        match pending_tx.inclusion().await {
            Ok((_, block)) => {
                debug!("the transaction has been included in block={block}");
                return Ok(());
            }
            Err(mev_share_rs::Error::TransactionTimeout(_, block)) => {
                warn!(%block, "the transaction has not been included in time (no blocks built by your builders?), retrying");
            }
            Err(e) => Err(e)?,
        }
    }
}
