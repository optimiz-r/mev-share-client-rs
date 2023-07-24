#![feature(lazy_cell)]
#![feature(let_chains)]
#![feature(once_cell_try)]
#![allow(dead_code)]

use mev_share_client::prelude::*;
use mev_share_client::Buildable;
mod utils;
use utils::{init_tracing, Config, Result};

#[tokio::main]
async fn main() {
    _main().await.unwrap() // has to panic or thiserror won't print backtracd
}

async fn _main() -> Result<()> {
    init_tracing();

    let config = Config::from_env().await?;
    let client = MevShareClient::new(config.auth_wallet.clone(), Network::Goerli);
    let event_history_info = client.get_event_history_info().await?;
    dbg!(&event_history_info);

    let mut page = 0;
    let mut done = false;

    while !done {
        let res_history = client
            .get_event_history(
                GetEventHistoryParams::builder()
                    .limit(event_history_info.max_limit)
                    .offset(page * event_history_info.max_limit)
                    .block_start(event_history_info.min_block)
                    .build()?,
            )
            .await?;

        for event in &res_history {
            if let Some(txs) = &event.hint.txs && !txs.is_empty() { // TODO: can these be both null and empty or be only empty? check what the API returns
                dbg!(event);
                dbg!(txs);
                break;
            }
        }

        for event in &res_history {
            if let Some(logs) = &event.hint.logs && !logs.is_empty() {
                dbg!(logs);
                done = true;
                break;
            }
        }

        page += 1;
    }

    Ok(())
}
