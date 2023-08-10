#![feature(lazy_cell)]
#![feature(let_chains)]
#![allow(dead_code)]

use ethers::types::U256;
use mev_share_rs::prelude::*;
use mev_share_rs::GetEventHistoryParams;
use tracing::*;

mod common;
use common::{init_tracing, Config};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    init_tracing();

    let config = Config::from_env().await?;
    let client = MevShareClient::new_with_chain_id(
        config.auth_wallet.clone(),
        config.provider.clone(),
        U256::one(), // EventHistory seems to be only supported on mainnet
    )?;
    let event_history_info = client.get_event_history_info().await?;
    debug!("{event_history_info:#?}");

    let mut page = 0;
    let mut done = false;

    while !done {
        let events = client
            .get_event_history(
                GetEventHistoryParams::builder()
                    .limit(event_history_info.max_limit)
                    .offset(page * event_history_info.max_limit)
                    .block_start(event_history_info.min_block)
                    .build(),
            )
            .await?;

        for event in &events {
            if let Some(txs) = &event.hint.txs && !txs.is_empty() {
                debug!("event: {event:#?}");
                debug!("txs: {txs:#?}");
                break;
            }
        }

        for event in &events {
            if let Some(logs) = &event.hint.logs && !logs.is_empty() {
                debug!("logs: {logs:#?}");
                done = true;
                break;
            }
        }

        page += 1;
    }

    Ok(())
}
