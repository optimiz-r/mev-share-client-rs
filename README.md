# [WIP] Flashbots MEV-Share Client

Client library for MEV-Share written in Rust.

Based on [MEV-Share Spec](https://github.com/flashbots/mev-share) and the [TypeScript reference implementation](https://github.com/flashbots/mev-share-client-ts).

## Progress

Mostly done. ETA for getting to feature parity with TS reference implementation is July, 26th.

Here's the current state:

- [x] api types + de/serialization
- [x] mev share client
- [x] [examples/send_private_tx.rs](examples/send_private_tx.rs)
- [x] [examples/send_bundle.rs](examples/send_bundle.rs)
- [ ] [examples/send_backrun_bundle.rs](examples/send_backrun_bundle.rs)
- [ ] [examples/historical_stream_data.rs](examples/historical_stream_data.rs)
- [ ] improve logs

## Quickstart

> [WIP]

### TypeScript to Rust

In Rust, though callbacks are feasible, streams are preferred, primarily due to their compatibility with the language's strong ownership and lifetime rules, as codified by the borrow checker, which allows efficient memory management without the need for a garbage collector.

Here's some common patterns and how they translate:

In TS:

```ts
const block_subscription = provider.on('block', (block) => {
  console.log(block.number);
});
```

In Rust:

```rust
let mut block_subscription = provider.subscribe_blocks().await?;
while let Some(block) = block_subscription.next().await {
    println!("block number: {}", block.number);
}
```

Relevant parts:

1. the first line initializes a mutable block subscription from the provider.
   - the `mut` keywork signifies that `block_subscription` is mutable, which is necessary here since the `next()` method changes the state of the subscription, i.e., moving to the next block
   - the `?` operator is used for error handling, it will return the error upstream if one occurs during the await operation
2. the second line loops as long as there's a new block, retrieving them asynchronously
3. the third line prints the block number to the console
   - the `!` near `println` makes it a macro, not a function. Macros in Rust are expanded at compile time into code, providing more flexibility than functions.

Similarly, in `mev_share_client`:

```ts
const handler = mevshare.on('transaction', (tx: IPendingTransaction) => {
  console.log(tx);
});
```

```rust
let mut tx_stream = client.subscribe(EventType::Transaction);
while let Some(pending_tx) = tx_stream.next().await {
    dbg!(pending_tx);
}
```

### Examples

Check out the [examples](examples/) directory for practical demonstrations of this library's capabilities.

```sh
cargo run --example send_private_tx
cargo run --example send_bundle
```

## Usage

> [WIP]

## Futher work

A few improvements idea, ETA ~1 week:

- [ ] Move `examples/` from Goerli to Sepolia as Goerli is being deprecated
- [ ] Move from `ethers-rs` to the newer `alloy`
- [ ] Add unit tests and/or more examples

### Additionally

- [ ] Refactor builders into a `#[derive(Builder)]` macro that also generates setters for fields of inner structs. PS. Do we want to maintain that?

## Security

The tool requires a private key for signing transactions. Make sure you don't share your private key or .env file with anyone or commit it to a public repository.

## License

> [WIP]
