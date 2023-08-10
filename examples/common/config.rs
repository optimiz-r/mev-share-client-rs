use dotenv::dotenv;
use envconfig::Envconfig;
use ethers::prelude::*;
use eyre::Result;
use tokio::sync::OnceCell;
use tracing::*;

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

        let auth_wallet = LocalWallet::from_bytes(&config.auth_private_key)?;
        let sender_wallet = LocalWallet::from_bytes(&config.sender_private_key)?;

        info!(
            sender = ?sender_wallet.address(),
            auth = ?auth_wallet.address(),
            provider = config.provider_url,
            "config"
        );

        Ok(Config {
            auth_wallet,
            sender_wallet,
            provider: Provider::connect(&config.provider_url).await?,
        })
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
