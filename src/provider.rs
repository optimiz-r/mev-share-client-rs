use crate::Result;
use ethers::providers::{Provider, Ws};
use tokio::sync::OnceCell;

/// A [`Provider<Ws>`] pointing to the `PROVIDER_URL` defined in the environment variables.
///
/// Get this via `Provider::from_env()`, as in:
///
///     let provider = Provider::from_env().await?;
///
///
/// TODO: Note: We could avoid having this statically here and have it as a field of [`MevShareClient`] instead but then the
/// client initialization will go from `let client = MevShareClient::new();` to `let client = MevShareClient::new().await?;`
/// i.e. from returning `Self` to returning  `Result<Future<Output = Self>>`, not sure if we're ok will all that complexity
/// in an initializer?
/// Usually, I'd just ask for the provider from above (i.e. `MevShareClient::new(provider, ...)`), just trying to adhere to the
/// TypeScript interface.
static PROVIDER: OnceCell<Provider<Ws>> = OnceCell::const_new();

async fn init_from_env() -> Result<Provider<Ws>> {
    let url = std::env::var("PROVIDER_URL")?;
    Provider::connect(&url).await.map_err(Into::into)
}

pub trait FromEnv {
    async fn from_env() -> Result<&'static Self>
    where
        Self: std::marker::Sized + 'static;
}

impl FromEnv for Provider<Ws> {
    /// Returns a lazily-initiailized `Provider<Ws>` pointing to the `PROVIDER_URL` defined in the envirinment variables.
    async fn from_env() -> Result<&'static Self> {
        PROVIDER.get_or_try_init(init_from_env).await
    }
}
