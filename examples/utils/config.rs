use super::Result;
use dotenv::dotenv;
use envconfig::Envconfig;
use ethers::prelude::*;
use tokio::sync::OnceCell;

pub struct Config {
    pub auth_wallet: LocalWallet,
    pub sender_wallet: LocalWallet,
    pub provider: Provider<Ws>,
}

impl Config {
    pub async fn from_env() -> Result<&'static Config> {
        CONFIG.get_or_try_init(Config::init_from_env).await
    }

    async fn init_from_env() -> Result<Config> {
        dotenv()?;
        let config = ConfigRaw::init_from_env()?;

        Ok(Config {
            auth_wallet: LocalWallet::from_bytes(&config.auth_private_key)?,
            sender_wallet: LocalWallet::from_bytes(&config.sender_private_key)?,
            provider: Provider::connect(&config.provider_url).await?,
        })
    }

    /// Syntactic sugar for loading the config in a single line.
    ///
    /// Meant to avoid boilerplate in test/example functions.
    /// In prod, after initial load, the components should be moved into the struct responsible for managing them.
    ///
    /// Usage:
    ///
    /// ```rust
    /// let (provider, sender_wallet, auth_wallet) = Config::from_env().await?.as_tuple();
    /// ```
    ///
    /// It's the same as:
    ///
    /// ```rust
    /// let config = Config::from_env().await?;
    /// let provider = &config.provider;
    /// let sender_wallet = &config.sender_wallet;
    /// let auth_wallet = &config.auth_wallet;
    /// ```
    pub fn as_tuple(&self) -> (&Provider<Ws>, &LocalWallet, &LocalWallet) {
        (&self.provider, &self.sender_wallet, &self.auth_wallet)
    }
}

static CONFIG: OnceCell<Config> = OnceCell::const_new();

#[derive(Envconfig)]
pub struct ConfigRaw {
    #[envconfig(from = "AUTH_PRIVATE_KEY")]
    pub auth_private_key: Bytes,
    #[envconfig(from = "SENDER_PRIVATE_KEY")]
    pub sender_private_key: Bytes,
    #[envconfig(from = "PROVIDER_URL")]
    pub provider_url: String,
}
