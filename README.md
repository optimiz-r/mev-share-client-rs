# Flashbots MEV-Share Client

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?labelColor=555555&)](LICENSE-MIT)&ensp;

Rust client library for Flashbots MEV-Share.
Based on the MEV-Share [specs] and the [TypeScript reference implementation].


## Usage

In `Cargo.toml`:

```toml
[dependencies]
mev_share_rs = "0.1.0"
```

### Client initialization

First, client initialization may fail for various reasons and therefore returns a [`Result`].
The quickest way to initialize a client, is to pass it a [`Signer`] to use for [Flashbots authentication] and a [`Provider`].

```rust
use ethers::prelude::*;
use mev_share_rs::prelude::*;

let auth_wallet = LocalWallet::random();
let provider = Provider::connect("https://rpc.example/api_key");
```

In order to know which MEV-Share endpoint to query, the client needs to know which chain id you're on. You can either `.await`
the client to fetch it from the `provider`:

```rust
let client = MevShareClient::new(auth_wallet, provider).await?;
```

or you can supply it yourself:

```rust
let client = MevShareClient::new_with_chain_id(auth_wallet, provider, 1)?;
```

### Subscribing to MEV-Share events

Once you have a client, you can listen to the bundles submitted to the MEV-Share Flashbots relayer:

```rust
use mev_share_rs::prelude::*;

let mut stream = client.subscribe_bundles().await?;
while let Some(event) = stream.next().await {
    let bundle = event?; // upstream any error
    println!("Bundle received: {:?}", bundle);
}
```

### Sending private transactions
You can also send private transactions and bundles with the MEV-Share API:
[`mev_sendBundle`]/[`mev_simBundle`] and the latest version of [`eth_sendPrivateTransaction`] use an upgraded bundle format
than the order  [`eth_sendBundle`]/[`eth_callBundle`], in order to allow users to specify privacy and other guarantees.
You can send a private transaction:

```rust
use mev_share_rs::prelude::*;

let pending_tx = client.send_private_transaction(
    SendTransactionParams::builder()
        // a signed `TypedTransaction`
        .tx(tx)
        // drop after 20 blocks
        .max_block_number(current_block + 20)
        .preferences(
            // what to disclose 
            Some(set![
                Hint::Hash, 
                Hint::Calldata, 
                Hint::Logs, 
                Hint::ContractAddress, 
                Hint::FunctionSelector
            ]),
            // to whom
            Some(set![
                Builder::Flashbots, 
            ]),
        )
        .build()
).await?;
```

wait for its inclusion in a block:

```rust
let (receipt, block) = pending_tx.inclusion().await?;
println!("Transaction included in block {}", block);
```

and/or handle errors:

```rust
match pending_tx.inclusion().await {
    Err(Error::TransactionTimeout) =>
        println!("the transaction was not included after 25 blocks or params.max_block_number"),
    Err(Error::TransactionReverted) =>
        println!("the transaction was reverted"),
    Ok((receipt, block)) =>
        println!("Transaction included in block {}", block),
}
```

If you're upstreaming the errors:

```rust
client
    .send_private_transaction(tx_request)
    .await?
    .inclusion()
    .await?;
```

### Sending bundles
Similarly, you can send private bundles to the MEV-Share Flashbots relayer:

```rust
let pending_bundle = client.send_private_bundle(
    SendBundleParams::builder()
        .body(vec![
            // a signed `TypedTransaction`
            Signed { tx: tx1, can_revert: false },
            // another signed `TypedTransaction`
            Signed { tx: tx2, can_revert: false },
            // a transaction we found in the mempool
            Tx { hash: tx3 }
        ])
        // drop after 3 blocks
        .inclusion(current_block + 1, Some(current_block + 1 + 3)) 
        // what to disclose to whom
        .privacy(
            // do not share any hints
            None,
            // send only to Flashbots
            Some(set![Builder::Flashbots])
        ),
        )
        .build()
).await?;
```

await its inclusion in a block:

```rust
let (receipt, block) = pending_bundle.inclusion().await?;
println!("Bundle {:?} included in block {}", pending_bundle.hash, block);
```

and/or handle any eventual error:

```rust
match pending_bundle.inclusion().await {
    Err(Error::BundleTimeout(_hashes , max_block)) =>
        println!("bundle {:?} not included after max block {}", pending_bundle.hash, max_block),
    Err(Error::BundleRevert(receipts)) =>
        println!("bundle {:?} reverted: {:?}", pending_bundle.hash, receipts),
    Err(Error::BundleDiscard(landed_receipts)) =>
        println!("bundle has not been included, but some of the transactions landed: {:?}", landed_receipts),
    Ok((receipts, block)) =>
        println!("Bundle {:?} included in block {}", pending_bundle.hash, block),
}
```

Finally, if you upstream the errors, you can just:

```rust
let (bundle_receipts, included_block) = client
    .send_budnle(bundle_request)
    .await?
    .inclusion()
    .await?;
```

### Simulating bundles
If you send too many bad bundles to the Flashbots API, you risk losing your [searcher reputation].
To avoid that, you can simulate bundles before sending them to Flashbots.

```rust
let simulation = client.simulate_bundle(
    bundle_request.clone(),
    SimulateBundleParams::builder()
        .block(current_block + 1)
        .build()
    ).await?;

// avoid sending a reverting bundle
if !simulation.success { return Err(Error::SimulationFailed(simulation)) }

// simulation success! send the bundle to Flashbots
client.send_bundle(bundle_request).await?.inclusion().await?;
```

### Others

[`get_event_history`] and [`get_event_history_info`] allow you to query bundle submission history: check
[`examples/historycal_stream_data`] for an example.
Finally, [`examples/send_backrun_bundle`] gives you an idea on how you can put all of the above to use to listen to transactions
hints from the relayer and backrun those you're interested in.

## API reference

See [`MevShareClient`].

## Examples

> ℹ️ Examples require a .env file (or that you populate your environment directly with the appropriate variables).

```sh
cd examples
cp .env.example .env
vim .env
```

You can run any example using `cargo`, e.g.:

```sh
cargo run --example send_private_tx
```

Here's the current examples:

1. [send_private_tx.rs]: sends a private transaction to the Flashbots MEV-Share relayer
1. [send_bundle.rs]: simulates and sends a bundle with hints
1. [historical_stream_data.rs]: query bundles history
1. [send_backrun_bundle.rs]: subscribe to the Flashbots MEV-Share events stream in order to listen for submitted bundles and transactions and backrun them

## Contributing, improvements, and further work

Contributions are welcome! If you'd like to contribute to this project, feel free to open a pull request. Here are a few improvements that are currently being worked on:

- [ ] move `examples/` from Goerli to Sepolia as Goerli is being deprecated
- [ ] `PendingBundle::inclusion().await` could include the check for simulation errors via the flashbots APIs and return (an error) before we have on-chain proof the bundle is not landed (max_block reached or partial on-chain inclusion observed)
- [ ] move from `ethers-rs` to the newer `alloy`
- [ ] add unit tests
- [ ] use a `JsonRpcClient` trait instead of forcing `Ws`

If you'd like to see more, go ahead and [open an issue](https://github.com/optimiz-r/mev-share-client-rs/issues/new).

## Security

The tool requires a private key for signing transactions. Make sure you don't share your private key or .env file with anyone or commit it to a public repository.

## License

This project is licensed under the [MIT License]

<!-- hrefs -->
[Documentation]: https://docs.rs/mev-share-rs
[specs]: https://github.com/flashbots/mev-share
[TypeScript reference implementation]: https://github.com/flashbots/mev-share-client-ts
[`mev-share-client-ts`]: https://github.com/flashbots/mev-share-client-ts
[searcher reputation]: https://docs.flashbots.net/flashbots-auction/searchers/advanced/reputation
[MEV-Share]: https://docs.flashbots.net/flashbots-mev-share/overview
[Flashbots authentication]: https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#authentication
[Flashbots specs]: https://github.com/flashbots/mev-share
[Rust]: http://rust-lang.org
[`paradigm/mev-share-rs`]: https://github.com/paradigmxyz/mev-share-rs

<!-- docs -->
[`Signer`]: https://github.com/gakonst/ethers-rs/blob/5145992e4b03fdaebcc4d35aa7ee44504ca82b5a/ethers-signers/src/lib.rs#L59
[`LocalWallet`]: https://github.com/gakonst/ethers-rs/blob/5145992e4b03fdaebcc4d35aa7ee44504ca82b5a/ethers-signers/src/lib.rs#L12
[`Provider`]: https://github.com/gakonst/ethers-rs/blob/5145992e4b03fdaebcc4d35aa7ee44504ca82b5a/ethers-providers/src/rpc/provider.rs
[`Result`]: https://github.com/optimiz-r/mev-share-client-rs/blob/bf5e6783de4c2659c17b9e547cc6fae52dbb3822/src/error.rs#L109
[`get_event_history`]: https://github.com/optimiz-r/mev-share-client-rs/blob/bf5e6783de4c2659c17b9e547cc6fae52dbb3822/src/client.rs#L389
[`get_event_history_info`]: https://github.com/optimiz-r/mev-share-client-rs/blob/bf5e6783de4c2659c17b9e547cc6fae52dbb3822/src/client.rs#L371
[`mev_sendBundle`]: https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#mev_sendbundle
[`mev_simBundle`]: https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#mev_simbundle
[`eth_sendBundle`]: https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#eth_sendbundle
[`eth_callBundle`]: https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#eth_callbundle
[`eth_sendPrivateTransaction`]: https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#eth_sendprivatetransaction

<!-- files -->
[send_private_tx.rs]: /examples/send_private_tx.rs
[send_bundle.rs]: /examples/send_bundle.rs
[historical_stream_data.rs]: /examples/historical_stream_data.rs
[send_backrun_bundle.rs]: /examples/send_backrun_bundle.rs
[MIT License]: /LICENSE
[`examples/historycal_stream_data`]: https://github.com/optimiz-r/mev-share-client-rs/blob/main/examples/historical_stream_data.rs
[`examples/send_backrun_bundle`]: https://github.com/optimiz-r/mev-share-client-rs/blob/main/examples/send_backrun_bundle.rs

<!-- badges -->
[github]: https://img.shields.io/badge/github-8da0cb?labelColor=555555&logo=github
[crates-io]: https://img.shields.io/badge/crates.io-fc8d62?labelColor=555555&logo=rust
[docs-rs]: https://img.shields.io/badge/docs.rs-66c2a5?labelColor=555555&logo=docs.rs
