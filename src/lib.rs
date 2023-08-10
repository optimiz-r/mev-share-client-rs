//! [![github]](https://github.com/optmiz-r/mev-share-client-rs)&ensp;[![crates-io]](https://crates.io/crates/mev-share-rs)&ensp;[![docs-rs]](https://docs.rs/mev-share-rs)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [docs-rs]: https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs
//!
//! An implementation of a [MEV-Share] client, based on the [Flashbots specs], for the [Rust] programming language.
//!
//! ## Quick start
//!
//! First, client initialization may fail for various reasons and therefore returns a [`Result`].
//!
//! The quickest way to initialize a client, is to pass it a [`Signer`] to use for [Flashbots authentication] and a [`Provider`].
//!
//! ```
//! use ethers::prelude::*;
//! use mev_share_rs::prelude::*;
//!
//! let auth_wallet = LocalWallet::random();
//! let provider = Provider::connect("https://rpc.example/api_key");
//! ```
//!
//! In order to know which MEV-Share endpoint to query, the client needs to know which chain id you're on. You can either `.await`
//! the client to fetch it from the `provider`:
//!
//! ```
//! let client = MevShareClient::new(auth_wallet, provider).await?;
//! ```
//!
//! or you can provide it yourself:
//!
//! ```
//! let client = MevShareClient::new_with_chain_id(auth_wallet, provider, 1)?;
//! ```
//!
//! ### Subscribing to MEV-Share events
//!
//! Once you have a client, you can listen to the bundles submitted to the MEV-Share Flashbots relayer:
//!
//! ```
//! use mev_share_rs::prelude::*;
//! # use ethers::prelude::*;
//! # use futures::stream::StreamExt;
//! # use std::convert::TryFrom;
//! #
//! # #[tokio::main]
//! # async fn main() -> Result<()> {
//! #     let auth_wallet = LocalWallet::random();
//! #     let provider = Provider::connect("https://rpc.example/api_key");
//! #     let client = MevShareClient::new(auth_wallet, provider).await?;
//!
//! let mut stream = client.subscribe_bundles().await?;
//! while let Some(event) = stream.next().await {
//!     let bundle = event?; // upstream any error
//!     println!("Bundle received: {:?}", bundle);
//! }
//! #     Ok(())
//! # }
//! ```
//!
//! ### Sending private transactions
//!
//! You can also send private transactions and bundles with the MEV-Share API:
//!
//! [`mev_sendBundle`]/[`mev_simBundle`] and the latest version of [`eth_sendPrivateTransaction`] use an upgraded bundle format
//! than the order  [`eth_sendBundle`]/[`eth_callBundle`], in order to allow users to specify privacy and other guarantees.
//!
//! You can send a private transaction:
//!
//! ```
//! use mev_share_rs::prelude::*;
//!
//! let pending_tx = client.send_private_transaction(
//!     SendTransactionParams::builder()
//!          // `tx` is a `TypedTransaction` (https://docs.rs/ethers/latest/ethers/types/transaction/eip2718/enum.TypedTransaction.html)
//!         .tx(tx.rlp_signed(&sender_wallet.sign_transaction_sync(&tx)?))
//!         .max_block_number(current_block + 20)
//!         .preferences(
//!             Some(set![Hint::Hash, Calldata, Logs, ContractAddress, FunctionSelector]),
//!             Some(BUILDERS.clone()),
//!         )
//!         .build()
//! ).await?;
//! ```
//!
//! wait for its inclusion in a block:
//!
//! ```
//! let (receipt, block) = pending_tx.inclusion().await?;
//! println!("Transaction included in block {}", block);
//! ```
//!
//! and/or handle errors:
//!
//! ```
//! match pending_tx.inclusion().await {
//!     Err(Error::TransactionTimeout) =>
//!         println!("the transaction was not included after 25 blocks or params.max_block_number"),
//!     Err(Error::TransactionReverted) =>
//!         println!("the transaction was reverted"),
//!     Ok((receipt, block)) =>
//!         println!("Transaction included in block {}", block),
//! }
//! ```
//!
//! If you're upstreaming the errors:
//!
//! ```
//! client
//!     .send_private_transaction(tx_request)
//!     .await?
//!     .inclusion()
//!     .await?;
//! ```
//!
//! ### Sending bundles
//!
//! Similarly, you can send private bundles to the MEV-Share Flashbots relayer:
//!
//! ```
//! let pending_bundle = client.send_private_bundle(
//!     SendBundleParams::builder()
//!         .body(vec![
//!             // a signed `TypedTransaction`
//!             Signed { tx: tx1, can_revert: false },
//!             Signed { tx: tx2, can_revert: false },
//!             // a transaction we found in the mempool
//!             Tx { hash: tx3 }
//!         ])
//!         .inclusion(current_block + 1, Some(current_block + 1 + 3)) // drop after 3 blocks
//!         .privacy(
//!             Some(set![Hint::Hash, Calldata, Logs, FunctionSelector, ContractAddress]),
//!             Some(set![
//!                 Builder::Flashbots,
//!                 Builder::Rsync,
//!                 Builder::Other("a non-flashbots builder")
//!             ]),
//!         )
//!         .build()
//! ).await?;
//! ```
//!
//! await its inclusion in a block:
//!
//! ```
//! let (receipt, block) = pending_bundle.inclusion().await?;
//! println!("Bundle {:?} included in block {}", pending_bundle.hash, block);
//! ```
//!
//! and/or handle any eventual error:
//!
//! ```
//! match pending_bundle.inclusion().await {
//!     Err(Error::BundleTimeout(_hashes , max_block)) =>
//!         println!("bundle {:?} not included after max block {}", pending_bundle.hash, max_block),
//!     Err(Error::BundleRevert(receipts)) =>
//!         println!("bundle {:?} reverted: {:?}", pending_bundle.hash, receipts),
//!     Err(Error::BundleDiscard(landed_receipts)) =>
//!         println!("bundle has not been included, but some of the transactions landed: {:?}", landed_receipts),
//!     Ok((receipts, block)) =>
//!         println!("Bundle {:?} included in block {}", pending_bundle.hash, block),
//! }
//! ```
//!
//! Finally, if you upstream the errors, you can just:
//!
//! ```
//! let (bundle_receipts, included_block) = client
//!     .send_budnle(bundle_request)
//!     .await?
//!     .inclusion()
//!     .await?;
//! ```
//!
//! ### Simulating bundles
//!
//! If you send too many bad bundles to the Flashbots API, you risk losing your [searcher reputation].
//! To avoid that, you can simulate bundles before sending them to Flashbots.
//!
//! ```
//! let simulation = client.simulate_bundle(
//!     bundle_request.clone(),
//!     SimulateBundleParams::builder()
//!         .block(current_block + 1)
//!         .build()
//!     ).await?;
//!
//! // avoid sending a reverting bundle
//! if !simulation.success { return Err(Error::SimulationFailed(simulation)) }
//!
//! // simulation success! send the bundle to Flashbots
//! client.send_bundle(bundle_request).await?.inclusion().await?;
//! ```
//!
//! ### Others
//!
//! [`get_event_history`] and [`get_event_history_info`] allow you to query bundle submission history: check
//! [`examples/historycal_stream_data`] for an example.
//!
//! Finally, [`examples/send_backrun_bundle`] gives you an idea on how you can put all of the above to use to listen to transactions
//! hints from the relayer and backrun those you're interested in.
//!
//! ## API reference
//!
//! See [`MevShareClient`].
//!
//! <!-- Links -->
//!
//! [`Signer`]: ethers::signers::Signer
//! [`LocalWallet`]: ethers::signers::LocalWallet
//! [`Provider`]: ethers::providers::Provider
//! [`get_event_history`]: MevShareClient::get_event_history
//! [`get_event_history_info`]: MevShareClient::get_event_history_info
//! [`mev_sendBundle`]: https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#mev_sendbundle
//! [`mev_simBundle`]: https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#mev_simbundle
//! [`eth_sendBundle`]: https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#eth_sendbundle
//! [`eth_callBundle`]: https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#eth_callbundle
//! [`eth_sendPrivateTransaction`]: https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#eth_sendprivatetransaction
//! [searcher reputation]: https://docs.flashbots.net/flashbots-auction/searchers/advanced/reputation
//! [MEV-Share]: https://docs.flashbots.net/flashbots-mev-share/overview
//! [Flashbots authentication]: https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#authentication
//! [Flashbots specs]: https://github.com/flashbots/mev-share
//! [Rust]: http://rust-lang.org
//! [`examples/historycal_stream_data`]: https://github.com/optimiz-r/mev-share-client-rs/blob/main/examples/historical_stream_data.rs
//! [`examples/send_backrun_bundle`]: https://github.com/optimiz-r/mev-share-client-rs/blob/main/examples/send_backrun_bundle.rs

#![warn(clippy::pedantic)]
#![allow(
    clippy::wildcard_imports,
    clippy::module_name_repetitions,
    clippy::single_match_else
)]
#![feature(
    let_chains,
    async_closure,
    async_fn_in_trait,
    lazy_cell,
    concat_idents,
    impl_trait_projections,
    return_position_impl_trait_in_trait,
    provide_any,
    error_generic_member_access,
    if_let_guard
)]

mod api;
mod client;
mod error;
mod helpers;
pub mod prelude;

pub use error::{Error, Result};
pub use prelude::*;
