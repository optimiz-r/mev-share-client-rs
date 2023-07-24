use crate::api::networks::{MevShareNetwork, Network};
use crate::api::types::*;
use crate::error::{Error, Result};
use crate::provider::FromEnv;
use crate::{SendBundleParams, TransactionParams};
use ethers::prelude::*;
use ethers::utils::keccak256;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest_eventsource::{Event, EventSource};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::json;
use std::time::{Duration, Instant};
use tokio_stream::Stream;

/// How long to wait for target to appear onchain, when calling mev_simBundle on a tx specified bundle.
const TIMEOUT_QUERY_TX_MS: Duration = Duration::from_secs(5 * 60);

pub enum MevShareRequest {
    SendPrivateTransaction,
    SendBundle,
    SimBundle,
}

impl MevShareRequest {
    pub fn as_method_name(&self) -> &str {
        match &self {
            Self::SendPrivateTransaction => "eth_sendPrivateTransaction",
            Self::SendBundle => "mev_sendBundle",
            Self::SimBundle => "mev_simBundle",
        }
    }
}

pub struct MevShareClient {
    auth_wallet: LocalWallet,
    network: MevShareNetwork,
    http: reqwest::Client,
}

impl MevShareClient {
    pub fn new(auth_wallet: LocalWallet, network: Network) -> Self {
        Self {
            auth_wallet,
            network: network.into(),
            http: reqwest::Client::new(),
        }
    }

    /// Sends a POST request to the Matchmaker API and returns the data.
    ///
    /// # Arguments
    ///
    /// * `method` - JSON-RPC method
    /// * `params` - JSON-RPC params
    ///
    /// # Returns
    ///
    /// Response data from the API request.
    pub async fn post_rpc<T, P>(&self, method: MevShareRequest, params: P) -> Result<T>
    where
        P: Serialize,
        T: DeserializeOwned,
    {
        let body = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 69,
            method: method.as_method_name().to_string(),
            params: serde_json::to_value(params)?,
        };

        tracing::debug!(request = ?body);

        let signature = format!(
            "{:?}:0x{}",
            self.auth_wallet.address(),
            self.auth_wallet
                .sign_message(format!(
                    "0x{}",
                    ethers::utils::hex::encode(keccak256(serde_json::to_string(&body)?.as_bytes()))
                ))
                .await?
        );

        let headers = {
            let mut headers = HeaderMap::new();
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            headers.insert(
                "X-Flashbots-Signature",
                HeaderValue::from_str(&signature).unwrap(),
            );
            headers
        };

        let response: String = self
            .http
            .post(self.network.api_url)
            .headers(headers)
            .json(&body)
            .send()
            .await?
            .text()
            .await?;

        tracing::debug!(%response);

        let response = serde_json::from_str::<JsonRpcResponse<T>>(&response).map_err(|error| {
            crate::Error::Deserialization {
                error,
                text: response,
            }
        })?;

        match response {
            JsonRpcResponse::Error(err) => Err(Error::JsonRpc(err)),
            JsonRpcResponse::Success(data) => Ok(data.result),
        }
    }

    /// Make an HTTP GET request.
    ///
    /// # Arguments
    ///
    /// * `url_suffix` - URL to send the request to.
    async fn stream_get<T>(&self, url_suffix: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = format!(
            "{}/api/v1/{url_suffix}",
            self.network.stream_url.trim_end_matches('/')
        );

        let response: String = reqwest::get(url).await?.text().await?;
        let response: StreamResponse<T> =
            serde_json::from_str(&response).map_err(|error| crate::Error::Deserialization {
                error,
                text: response,
            })?;

        Ok(response.data)
    }

    /// Starts listening to the Matchmaker event stream.
    ///
    /// # Arguments
    ///
    /// * `event_type` - The [`StreamEvent`] listen for. Options specified by [`StreamEvent`] enum.
    ///
    /// # Returns
    ///
    ///  Stream with the [`EventPayload`].
    pub fn subscribe(
        &self,
        event_type: EventType,
    ) -> impl Stream<Item = Result<EventPayload>> + '_ {
        let event_source = EventSource::get(self.network.stream_url);
        let event_type = serde_json::to_string(&event_type).unwrap();

        Box::pin(event_source.filter_map(move |event| {
            let event_type = event_type.clone();

            async move {
                match event {
                    Ok(Event::Open) => None,
                    Ok(Event::Message(msg)) => {
                        if msg.event != event_type {
                            return None;
                        }

                        let payload_received: MevShareEvent =
                            serde_json::from_str(&msg.data).unwrap();

                        match msg.event.as_str() {
                            "bundle" => Some(Ok(EventPayload::Bundle(payload_received))),
                            "transaction" => {
                                Some(Ok(EventPayload::Transaction(payload_received.into())))
                            }
                            other_event => {
                                tracing::warn!("Unhandled event: {other_event}");
                                None
                            }
                        }
                    }
                    Err(err) => Some(Err(err.into())),
                }
            }
        }))
    }

    /// Sends a private transaction with MEV hints to the Flashbots Matchmaker.
    ///
    /// # Arguments
    ///
    /// * `signedTx` - Signed transaction to send
    /// * `options` - Tx preferences; hints & block range
    ///
    /// # Returns
    ///
    /// Transaction hash.
    pub async fn send_transaction(
        &self,
        signed_tx: Bytes,
        options: TransactionParams,
    ) -> Result<TxHash> {
        let tx_params = json!({
            "tx": signed_tx,
            "maxBlockNumber": options.max_block_number,
            "preferences": {
                "fast": true, // deprecated but required; setting has no effect
                // privacy uses default (Stable) config if no hints specified
                "privacy": {
                    "hints": options.hints,
                    "builders": options.builders, // WARN: It's not where it's in the TS library, but seems to be good like this according to the docs https://docs.flashbots.net/flashbots-auction/searchers/advanced/rpc-endpoint#eth_sendprivatetransaction
                },
            }
        });

        self.post_rpc(MevShareRequest::SendPrivateTransaction, [tx_params])
            .await
    }

    /// Sends a bundle to mev-share.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for the bundle
    ///
    /// # Returns
    ///
    /// Array of bundle hashes.
    pub async fn send_bundle(&self, params: SendBundleParams) -> Result<SendBundleResponse> {
        self.post_rpc(MevShareRequest::SendBundle, [params]).await
    }

    /// Simulates a bundle specified by `params`.
    ///
    /// Bundles containing pending transactions (specified by `{hash}` instead of `{tx}` in `params.body`) may
    /// only be simulated after those transactions have landed on chain. If the bundle contains
    /// pending transactions, this method will wait for the transactions to land before simulating.
    ///
    /// # Arguments
    ///
    /// * `params` - JSON data params
    /// * `sim_options` - Simulation options; override block header data for simulation.
    ///
    /// # Returns
    ///
    /// Simulation result.
    pub async fn simulate_bundle(
        &self,
        bundle_params: SendBundleParams,
        sim_options: SimulateBundleParams,
    ) -> Result<SimulateBundleResponse> {
        match bundle_params.body.first() {
            // hash must appear onchain before simulation is possible
            Some(Body::Hash(hash)) => self.sim_tx(*hash, bundle_params, sim_options).await,
            _ => self.sim_bundle(bundle_params, sim_options).await,
        }
    }

    /// Internal mev_simBundle call.
    ///
    /// Note: This may only be used on matched bundles.
    /// Simulating unmatched bundles (i.e. bundles with a hash present) will throw an error.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for the bundle
    /// * `simOptions` - Simulation options; override block header data for simulation
    ///
    /// # Returns
    ///
    /// Simulation result.
    async fn sim_bundle(
        &self,
        bundle_params: SendBundleParams,
        sim_options: SimulateBundleParams,
    ) -> Result<SimulateBundleResponse> {
        self.post_rpc(
            MevShareRequest::SimBundle,
            json!([bundle_params, sim_options]),
        )
        .await
    }

    async fn sim_tx(
        &self,
        hash: TxHash,
        bundle_params: SendBundleParams,
        sim_options: SimulateBundleParams,
    ) -> Result<SimulateBundleResponse> {
        tracing::info!(
            "Transaction hash: {hash} must appear onchain before simulation is possible, waiting"
        );

        let provider = Provider::from_env().await?;

        let check_then_sim_tx = || async {
            let tx = match provider.get_transaction(hash).await {
                Ok(Some(tx)) => tx,
                _ => return None, // not yet landed
            };

            let block_number = match tx.block_number {
                Some(block_number) => block_number,
                None => return None, // not yet landed
            };

            tracing::info!(
                "Found transaction hash: {hash:?} onchain at block number: {block_number}, simulating",
            );

            let simulation_result = self
                .sim_bundle(
                    SendBundleParams {
                        body: {
                            let mut body = bundle_params.body.clone();
                            body[0] = Body::Signed {
                                tx: tx.rlp(),
                                can_revert: false,
                            };
                            body
                        },
                        ..bundle_params.clone()
                    },
                    SimulateBundleParams {
                        parent_block: sim_options.parent_block.or(Some(block_number - 1)),
                        ..sim_options.clone()
                    },
                )
                .await;

            Some(simulation_result)
        };

        // in case tx is already landed
        if let Some(result) = check_then_sim_tx().await {
            return result;
        }

        // check for `TIMEOUT_QUERY_TX_MS` millis
        let start_time = Instant::now();

        let mut block_subscription = provider.subscribe_blocks().await?;
        while let Some(_block) = block_subscription.next().await {
            if let Some(result) = check_then_sim_tx().await {
                return result;
            } else if Instant::now() - start_time > TIMEOUT_QUERY_TX_MS {
                break;
            }
        }

        Err(Error::TxTimeout(TIMEOUT_QUERY_TX_MS))
    }

    /// Gets information about the event history endpoint.
    pub async fn get_event_history_info(&self) -> Result<EventHistoryInfo> {
        self.stream_get("history/info").await
    }

    /// Gets past events that were broadcast via the SSE event stream.
    pub async fn get_event_history(
        &self,
        params: GetEventHistoryParams,
    ) -> Result<Vec<EventHistory>> {
        let params = serde_qs::to_string(&params).unwrap();
        self.stream_get(&format!("history?{}", params)).await
    }
}
