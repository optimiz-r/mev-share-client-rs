#![feature(lazy_cell)]
#![feature(once_cell_try)]
#![feature(future_join)]
#![allow(dead_code)]

use ethers::{prelude::*, types::transaction::eip2718::TypedTransaction};
use mev_share_client::prelude::*;
use sugars::hset;
use tokio::try_join;

mod utils;
use utils::{init_tracing, Config, Result};

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    tracing::info!("sending tx to Flashbots Bundle API...");

    let (provider, sender_wallet, auth_wallet) = Config::from_env().await?.as_tuple();
    dbg!(&sender_wallet.address());

    let (chain_id, fee_data, current_block) = try_join!(
        provider.get_chainid(),
        provider.estimate_eip1559_fees(None),
        provider.get_block_number()
    )?;

    let tx: TypedTransaction = Eip1559TransactionRequest::default()
        .chain_id(chain_id.as_u64())
        .from(sender_wallet.address())
        .to(sender_wallet.address())
        .gas(22_000)
        .data(b"i'm shariiiing")
        .max_fee_per_gas(fee_data.0)
        .max_priority_fee_per_gas(fee_data.1)
        .into();

    let client = MevShareClient::new(auth_wallet.clone(), Network::Goerli);

    client
        .send_transaction(
            tx.rlp_signed(&sender_wallet.sign_transaction_sync(&tx)?),
            TransactionParams::builder()
                .max_block_number(current_block + U64::one())
                .hints(hset![Calldata, Logs, ContractAddress, FunctionSelector])
                .build()?,
        )
        .await?;

    Ok(())
}
