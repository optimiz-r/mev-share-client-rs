#![feature(lazy_cell)]
#![feature(let_chains)]
#![feature(array_try_map)]
#![allow(dead_code)]

/// Sends a bundle that shares as much data as possible by setting the `privacy` param.
use ethers::prelude::*;
use ethers::utils::parse_ether;
use eyre::{eyre, Result};
use mev_share_rs::prelude::*;
use mev_share_rs::MevShareClient;
use tracing::*;

mod common;
use common::{init_tracing, Config, MockTx, BUILDERS};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    // load config
    let c = Config::from_env().await?;
    let client = MevShareClient::new(c.auth_wallet.clone(), c.provider.clone()).await?;

    let current_block = c.provider.get_block_number().await?;

    /*
        NOTE: only bundles comprised solely of signed transactions are supported at the moment.
        Bundles containing `hash` cannot set `privacy` settings.
    */

    let tip = (parse_ether("0.0002")?, parse_ether("0.00002")?);

    let bundle_request = SendBundleParams::builder()
        .body(vec![
            Body::Signed {
                tx: MockTx::default().tip(tip).build().await?,
                can_revert: false,
            },
            Body::Signed {
                tx: MockTx::default().tip(tip).nonce_add(1).build().await?,
                can_revert: false,
            },
        ])
        .inclusion(current_block + 1, Some(current_block + 1 + 20))
        .privacy(
            // what to disclose
            Some(set![
                Hint::TxHash,
                Calldata,
                Logs,
                FunctionSelector,
                ContractAddress
            ]),
            // to whom
            Some(BUILDERS.clone()),
        )
        .build();

    // before submission, make sure simulation is ok
    // sending several reverting bundles may lead to our `auth_signer` losing high priority status
    // see: https://docs.flashbots.net/flashbots-auction/searchers/advanced/reputation
    let simulation = client
        .simulate_bundle(
            bundle_request.clone(),
            SimulateBundleParams::builder()
                .parent_block(current_block)
                .build(),
        )
        .await?;

    if !simulation.success {
        // abort
        return Err(eyre!("simulation failure: {simulation:?}"));
    }

    debug!(?simulation, "successful simulation");

    // send the bundle to the Flashbots MEV-Share relayer
    let pending_bundle = client.send_bundle(bundle_request).await?;
    let bundle_hash = pending_bundle.hash;
    info!(?bundle_hash, "bundle accepted by the relayer");

    // wait for the bundle to land
    pending_bundle.inclusion().await?;
    info!(?bundle_hash, "bundle landed on-chain");

    Ok(())
}
