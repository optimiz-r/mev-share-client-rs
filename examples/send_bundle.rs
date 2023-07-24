#![feature(lazy_cell)]
#![feature(once_cell_try)]
#![feature(const_trait_impl)]
#![allow(dead_code)]

/// Sends a bundle that shares as much data as possible by setting the `privacy` param.
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::utils::parse_ether;
use mev_share_client::prelude::*;
use mev_share_client::MevShareClient;
use sugars::hset;
use tokio::try_join;

mod utils;
use utils::{init_tracing, Config, Error, Result};

const NUM_TARGET_BLOCKS: u64 = 3;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    // load config
    let (provider, sender_wallet, auth_wallet) = Config::from_env().await?.as_tuple();
    let client = MevShareClient::new(auth_wallet.clone(), Network::Goerli);

    let (chain_id, fees, transaction_count, current_block) = try_join!(
        provider.get_chainid(),
        provider.estimate_eip1559_fees(None),
        provider.get_transaction_count(sender_wallet.address(), None),
        provider.get_block_number()
    )?;
    dbg!(&fees);

    /*
        NOTE: only bundles comprised solely of signed transactions are supported at the moment.
        Bundles containing `hash` cannot set `privacy` settings.
    */

    let bundle = {
        let tx = {
            let tip = parse_ether("0.0002").map_err(Error::ParseEther)?;
            let tx: TypedTransaction = Eip1559TransactionRequest::new()
                .chain_id(chain_id.as_u64())
                .from(sender_wallet.address())
                .to(sender_wallet.address())
                .nonce(transaction_count)
                .max_fee_per_gas(fees.0 + tip)
                .max_priority_fee_per_gas(fees.1 + tip)
                .into();
            tx.rlp_signed(&sender_wallet.sign_transaction_sync(&tx)?)
        };

        vec![Body::Signed {
            tx,
            can_revert: false,
        }]
    };

    tracing::info!("Sending bundle targeting next {NUM_TARGET_BLOCKS} blocks...");

    let request_params = SendBundleParams::builder()
        .inclusion_block(current_block + 1)
        .inclusion_max_block(current_block + 1 + NUM_TARGET_BLOCKS)
        .body(bundle)
        .privacy_hints(hset![
            mev_share_client::TxHash,
            Calldata,
            Logs,
            FunctionSelector,
            ContractAddress
        ])
        .privacy_builders(vec!["flashbots".to_string()])
        .build()?;
    dbg!(&request_params);

    let backrun_result = client.send_bundle(request_params).await;
    dbg!(&backrun_result);

    Ok(())
}
