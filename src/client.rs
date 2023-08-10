use crate::api::networks::MevShareNetwork;
use crate::api::rest_client::RestClient;
use crate::api::rpc_client::MevShareRpcClient;
use crate::api::types::PendingTransaction;
use crate::api::types::*;
use crate::error::JsonError;
use crate::helpers::provider::Waiter;
use crate::{Result, SendBundleParams, SendTransactionParams};
use ethers::prelude::*;
use reqwest_eventsource::{Event, EventSource};
use serde_json::json;
use tokio_stream::{Stream, StreamExt};
use tracing::trace;

pub struct MevShareClient<'a> {
    provider: Provider<Ws>,
    network: MevShareNetwork,
    rpc: MevShareRpcClient<'a>,
    rest: RestClient,
}

impl MevShareClient<'_> {
    /// Initializes a [`MevShareClient`].
    ///
    /// If you already have a `chain_id`, you can use [`Self::new_with_chain_id`], which is not async because it avoids the network trip.
    /// `chain_id` is needed to infer which MEV-Share endpoint (e.g. mainnet or goerli) to query.
    ///
    /// # Example
    ///
    /// ```
    /// let client = MevShareClient::new(auth_wallet. provider).await?;
    /// ```
    ///
    /// # Errors
    ///
    /// * [`crate::Error::Provider`] if the `provider` fails to retrieve a `chain_id`.
    /// * [`crate::Error::UnsupportedNetwork`] if the `chain_id` is not supported by the MEV-Share client.
    pub async fn new(auth_wallet: LocalWallet, provider: Provider<Ws>) -> Result<Self> {
        let chain_id = provider.get_chainid().await?;
        Self::new_with_chain_id(auth_wallet, provider, chain_id)
    }

    /// Initializes a [`MevShareClient`].
    ///
    /// # Example
    ///
    /// ```
    /// // no need to await here
    /// let client = MevShareProvider::new_with_chain_id(auth_wallte, provider, chain_id)?;
    /// ```
    ///
    /// # Errors
    ///
    /// * [`crate::Error::UnsupportedNetwork`] if the `chain_id` is not supported by the MEV-Share client.
    pub fn new_with_chain_id(
        auth_wallet: LocalWallet,
        provider: Provider<Ws>,
        chain_id: U256,
    ) -> Result<Self> {
        let network = MevShareNetwork::try_from(chain_id)?;
        let rest_url = format!("{}/api/v1", network.stream_url.trim_end_matches('/'));

        Ok(Self {
            rpc: MevShareRpcClient::new(network.api_url, auth_wallet),
            rest: RestClient::new(rest_url),
            provider,
            network,
        })
    }

    /// Starts listening to the MEV-Share event stream.
    ///
    /// # Example
    ///
    /// ```
    /// let block_stream = client.subscribe_bundles();
    /// while let Some(block) = block_stream.next().await {
    ///     println!("received new block {:?}", block);
    /// }
    /// ```
    ///
    /// # Returns
    ///
    ///  A stream of [`MevShareEvent`]s.
    pub fn subscribe_bundles(&self) -> impl Stream<Item = Result<MevShareEvent>> + '_ {
        EventSource::get(self.network.stream_url).filter_map(move |event| match event {
                    Ok(Event::Open) => None,
                    Ok(Event::Message(msg)) => {
                trace!(%msg.data);

                Some(
                    serde_json::from_str(&msg.data)
                        .map_err(|source| JsonError::Deserialization {
                            text: msg.data,
                            source,
                        })
                        .map_err(Into::into),
                )
            }
            Err(err) => Some(Err(err.into())),
        })
    }

    /// Sends a private transaction with MEV hints to Flashbots MEV-Share.
    ///
    /// # Example
    ///
    /// ```
    /// let client = MevShareClient::new(auth_wallet, provider).await?;
    ///
    /// // compose a tx
    /// let tx: TypedTransaction = Eip1559TransactionRequest::default()
    ///     .chain_id(chain_id.as_u64())
    ///     .from(sender_wallet.address())
    ///     .to(sender_wallet.address())
    ///     .gas(22_000)
    ///     .data(b"i'm shariiiing")
    ///     .max_fee_per_gas(fee_data.0 * 110 / 100)
    ///     .max_priority_fee_per_gas(fee_data.1 * 110 / 100)
    ///     .into();
    ///
    /// // specify the `eth_sendPrivateTransaction` params
    /// let private_tx = SendTransactionParams::builder()
    ///     .tx(tx.rlp_signed(&sender_wallet.sign_transaction_sync(&tx)?))
    ///     .max_block_number(current_block + 20)
    ///     .preferences(
    ///         Some(set![Hint::Hash, Calldata, Logs, ContractAddress, FunctionSelector]),
    ///         Some(set![Builder::Flashbots]),
    ///     )
    ///     .build();
    ///
    /// debug!(?private_tx, "sending private tx to Flashbots MEV-Share API");
    ///
    /// let pending_tx = client.send_private_transaction(private_tx.clone()).await?;
    /// debug!(?pending_tx.hash, "relayer accepted the transaction");
    ///
    /// let (_receipt, block) = pending_tx.inclusion().await?;
    /// debug!(?pending_tx.hash, "the transaction has been included in block={block}");
    /// ```
    ///
    /// # Arguments
    ///
    /// * `signedTx` - Signed transaction to send
    /// * `options` - Tx preferences; hints & block range
    ///
    /// # Returns
    ///
    /// Transaction hash.
    ///
    /// # Errors
    ///
    /// * [`crate::Error::Rpc`] if the network request to the MEV-Share API fails.
    /// * [`crate::Error::Provider`] if `self.provider` fails to get the [`TransactionReceipt`] or subscribing to blocks to wait for it.
    /// * [`crate::Error::TransactionTimeout`] if the transaction is not included in a block before `params.max_block_number` or 25[^1] blocks.
    /// * [`crate::Error::TransactionRevert`] if the transaction reverts.
    ///
    /// [^1]: See [flashbots docs](https://docs.flashbots.net/flashbots-auction/searchers/advanced/private-transaction).
    pub async fn send_private_transaction(
        &self,
        params: SendTransactionParams<'_>,
    ) -> Result<PendingTransaction> {
        let max_block_number = params.max_block_number;

        let hash: TxHash = self
            .rpc
            .post(MevShareRequest::SendPrivateTransaction, [params])
            .await?;

        Ok(PendingTransaction::new(
            hash,
            max_block_number,
            &self.provider,
        ))
    }

    /// Sends a bundle to mev-share.
    ///
    /// # Example
    ///
    /// ```
    /// use crate::prelude::*;
    ///
    /// let bundle_request = SendBundleParams::builder()
    ///     .body(vec![
    ///         Body::Signed {
    ///             tx: MockTx::default().tip(tip).build().await?,
    ///             can_revert: false,
    ///         },
    ///         Body::Signed {
    ///             tx: MockTx::default().tip(tip).nonce_add(1).build().await?,
    ///             can_revert: false,
    ///         },
    ///     ])
    ///     .inclusion(current_block + 1, Some(current_block + 1 + 20))
    ///     .privacy(
    ///         Some(set![Hash, Calldata, Logs, FunctionSelector, ContractAddress]),
    ///         Some(set![Flashbots, Builder::Custom("my own builder")]),
    ///     )
    ///     .build();
    ///
    /// // send the bundle to the Flashbots MEV-Share relayer
    /// let pending_bundle = client.send_bundle(bundle_request).await?;
    /// let bundle_hash = pending_bundle.hash;
    /// info!(?bundle_hash, "bundle accepted by the relayer");
    ///
    /// // wait for the bundle to land
    /// pending_bundle.inclusion().await?;
    /// info!(?bundle_hash, "bundle landed on-chain");
    /// ```
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for the bundle
    ///
    /// # Returns
    ///
    /// Array of bundle hashes.
    ///
    /// # Errors
    ///
    /// * [`crate::Error::Rpc`] if the JSON-RPC request to the MEV-Share API fails.
    /// * [`crate::Error::Provider`] if `self.provider` fails to get the [`TransactionReceipt`] for the transactions that or subscribing to blocks to wait for it.
    /// * [`crate::Error::BundleTimeout`] if the bundle is not included in a block before `params.inclusion.max_block`.
    /// * [`crate::Error::BundleRevert`] if any transaction in the bundle reverts.
    /// * [`crate::Error::BundleDiscard`] if the bundle was not included as a whole but some of the transactions in its body were included
    /// (before `params.inclusion.max_block`, otherwise [`crate::Error::BundleTimeout`] will be returned instead).
    pub async fn send_bundle<'lt>(
        &'lt self,
        params: SendBundleParams<'lt>,
    ) -> Result<PendingBundle> {
        let send_bundle_response: SendBundleResponse = self
            .rpc
            .post(MevShareRequest::SendBundle, [params.clone()])
            .await?;

        Ok(PendingBundle::new(
            send_bundle_response.bundle_hash,
            params,
            &self.provider,
        ))
    }

    /// Simulates a bundle specified by `params`.
    ///
    /// Bundles containing pending transactions (specified by `{hash}` instead of `{tx}` in `params.body`) may
    /// only be simulated after those transactions have landed on chain. If the bundle contains
    /// pending transactions, this method will wait for the transactions to land before simulating.
    ///
    /// # Example
    ///
    /// ```
    ///  let simulation_result = client
    ///     .simulate_bundle(
    ///         bundle_request.clone(),
    ///         SimulateBundleParams::builder()
    ///             .parent_block(current_block)
    ///             .build(),
    ///     )
    ///     .await?;
    ///
    /// if !simulation_result.success {
    ///     // do not send to avoid losing priority
    ///     return Err(Error::SimulationFailure(simulation_result));
    /// }
    /// ```
    ///
    /// # Arguments
    ///
    /// * `params` - JSON data params
    /// * `sim_options` - Simulation options; override block header data for simulation.
    ///
    /// # Returns
    ///
    /// Simulation result.
    ///
    /// # Errors
    ///
    /// * [`crate::Error::Rpc`] if any JSON-RPC request to the MEV-Share API fails.
    /// * [`crate::Error::Provider`] if the provider can't subscribe to the blocks to wait for the unsigned
    /// transactions to land, or fetch the transactions.
    ///
    /// For a more comprehensive example, see [`crate::MevShareClient::send_bundle`].
    pub async fn simulate_bundle(
        &self,
        mut bundle_params: SendBundleParams<'_>,
        mut sim_options: SimulateBundleParams,
    ) -> Result<SimulateBundleResponse> {
        if let Some(Body::Tx { hash }) = bundle_params.body.first() {
            // hash must appear on-chain before simulation is possible
            let (tx, block_number) = self
                .provider
                .wait_for_tx(*hash, bundle_params.inclusion.block + TX_WAIT_MAX_BLOCKS)
                .await?;

            // replace hash with signed tx
                            let mut body = bundle_params.body.clone();
                            body[0] = Body::Signed {
                                tx: tx.rlp(),
                                can_revert: false,
                            };

            bundle_params = SendBundleParams {
                body,
                        ..bundle_params.clone()
            };

            sim_options = SimulateBundleParams {
                        parent_block: sim_options.parent_block.or(Some(block_number - 1)),
                        ..sim_options.clone()
        };
        }

        self.rpc
            .post(
                MevShareRequest::SimBundle,
                json!([bundle_params, sim_options]),
            )
            .await
            .map_err(Into::into)
    }

    /// Gets information about the event history endpoint.
    ///
    /// # Example
    ///
    /// ```
    /// let client = MevShareClient::new_with_chain_id(
    ///     config.auth_wallet.clone(),
    ///     config.provider.clone(),
    ///     U256::one(), // EventHistory seems to be only supported on mainnet
    /// )?;
    /// let event_history_info = client.get_event_history_info().await?;
    ///
    /// let mut page = 0;
    /// let mut done = false;
    ///
    /// while !done {
    ///     let events = client
    ///         .get_event_history(
    ///             GetEventHistoryParams::builder()
    ///                 .limit(event_history_info.max_limit)
    ///                 .offset(page * event_history_info.max_limit)
    ///                 .block_start(event_history_info.min_block)
    ///                 .build(),
    ///         )
    ///         .await?;
    ///
    ///     for event in &events {
    ///         if let Some(txs) = &event.hint.txs && !txs.is_empty() {
    ///             debug!(?event);
    ///             debug!(?txs);
    ///             break;
    ///         }
    ///     }
    ///
    ///     for event in &events {
    ///         if let Some(logs) = &event.hint.logs && !logs.is_empty() {
    ///             debug!(?logs);
    ///             done = true;
    ///             break;
    ///         }
    ///     }
    ///
    ///     page += 1;
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// * [`crate::Error::Rest`] if the network GET request to the MEV-Share API fails.
    pub async fn get_event_history_info(&self) -> Result<EventHistoryInfo> {
        self.rest.get("history/info").await.map_err(Into::into)
    }

    /// Gets past events that were broadcast via the SSE event stream.
    ///
    /// # Example
    ///
    /// ```
    /// let info = client.get_event_history_info().await?;
    /// pritnln!("min_block={}, max_limit={}", info.min_block, info.max_limit);
    /// ```
    ///
    /// # Errors
    ///
    /// * [`crate::Error::Rest`] if the network GET request to the MEV-Share API fails.
    ///
    /// For a more comprehensive example, see [`crate::MevShareClient::get_event_history_info`].
    pub async fn get_event_history(
        &self,
        params: GetEventHistoryParams,
    ) -> Result<Vec<EventHistory>> {
        self.rest
            .get_with_params("history", params)
            .await
            .map_err(Into::into)
    }

}

pub enum MevShareRequest {
    SendPrivateTransaction,
    SendBundle,
    SimBundle,
    GetUserStats,
    GetBundleStats,
}

impl MevShareRequest {
    pub fn as_method_name(&self) -> &str {
        match &self {
            Self::SendPrivateTransaction => "eth_sendPrivateTransaction",
            Self::SendBundle => "mev_sendBundle",
            Self::SimBundle => "mev_simBundle",
            Self::GetUserStats => "flashbots_getUserStatsV2",
            Self::GetBundleStats => "flashbots_getBundleStatsV2",
        }
    }
}
