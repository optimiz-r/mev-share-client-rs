use crate::error::{JsonError, RestError};
use serde::{de::DeserializeOwned, Serialize};
use tracing::*;

type Result<T> = std::result::Result<T, RestError>;

pub struct RestClient {
    base_url: String,
    http: reqwest::Client,
}

impl RestClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            http: reqwest::Client::new(),
        }
    }

    pub async fn get<T>(&self, path: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        get(&self.http, &self.base_url, path, Option::<String>::None).await
    }

    pub async fn get_with_params<T, P>(&self, path: &str, params: P) -> Result<T>
    where
        P: Serialize + std::fmt::Debug,
        T: DeserializeOwned,
    {
        get(&self.http, &self.base_url, path, Some(params)).await
    }
}

/// Performs an HTTP GET request.
///
/// # Arguments
///
/// * `path` - Resources to GET.
/// * `params` - Query parameters.
///
/// # Errors
///
/// * [`RestError`] if the request fails.
#[instrument]
async fn get<T, P>(
    client: &reqwest::Client,
    base_url: &str,
    path: &str,
    params: Option<P>,
) -> Result<T>
where
    P: Serialize + std::fmt::Debug,
    T: DeserializeOwned,
{
    let params = match params {
        None => String::new(),
        Some(params) => serde_qs::to_string(&params)?,
    };

    let url = format!("{base_url}/{path}?{params}");
    trace!(?url);

    let response: String = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    trace!(response);

    let response: T =
        serde_json::from_str(&response).map_err(|source| JsonError::Deserialization {
            source,
            text: response,
        })?;

    Ok(response)
}
