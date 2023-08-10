use crate::api::types::{JsonRpcRequest, JsonRpcResponse};
use crate::client::MevShareRequest;
use crate::error::{JsonError, RpcError};
use ethers::signers::{LocalWallet, Signer};
use ethers::utils::keccak256;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::*;

type Result<T> = std::result::Result<T, RpcError>;

pub struct MevShareRpcClient<'a> {
    base_url: &'a str,
    request_id: AtomicI32,
    http: reqwest::Client,
    auth_wallet: LocalWallet,
}

impl<'a> MevShareRpcClient<'a> {
    pub fn new(base_url: &'a str, auth_wallet: LocalWallet) -> Self {
        Self {
            base_url,
            request_id: Self::new_request_id(),
            http: reqwest::Client::new(),
            auth_wallet,
        }
    }

    /// Sends a POST request to the MEV-Share API and returns the data.
    ///
    /// # Arguments
    ///
    /// * `method` - JSON-RPC method
    /// * `params` - JSON-RPC params
    ///
    /// # Returns
    ///
    /// Response data from the API request.
    ///
    /// # Errors
    ///
    /// * [`RpcError`] if the request fails.
    pub async fn post<T, P>(&self, method: MevShareRequest, params: P) -> Result<T>
    where
        P: Serialize,
        T: DeserializeOwned,
    {
        let body = JsonRpcRequest {
            jsonrpc: "2.0",
            id: self.request_id.fetch_add(1, Ordering::Relaxed),
            method: method.as_method_name(),
            params: serde_json::to_value(params)?,
        };

        trace!(request = %serde_json::to_string(&body).unwrap());

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

        trace!(?signature);

        let headers = {
            let mut headers = HeaderMap::new();
            headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
            headers.insert("X-Flashbots-Signature", HeaderValue::from_str(&signature)?);
            headers
        };

        let response: String = self
            .http
            .post(self.base_url)
            .headers(headers)
            .json(&body)
            .send()
            .await?
            .text()
            .await?;

        trace!(%response);

        let response = serde_json::from_str::<JsonRpcResponse<T>>(&response).map_err(|source| {
            JsonError::Deserialization {
                source,
                text: response,
            }
        })?;

        match response {
            JsonRpcResponse::Error(err) => Err(RpcError::Response(err)),
            JsonRpcResponse::Success(data) => Ok(data.result),
        }
    }

    // Pseudo-random number to avoid collisions between requests coming from different instances of this client.
    // It doesn't need to be cryptographically secure, so it's not worth adding a dependency for it.
    fn new_request_id() -> AtomicI32 {
        AtomicI32::new(
            (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time went backwards")
                .as_nanos()
                % 1_000_000) as i32,
        )
    }
}
