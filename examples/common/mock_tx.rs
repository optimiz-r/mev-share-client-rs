use crate::common::Config;
use ethers::{prelude::*, types::transaction::eip2718::TypedTransaction};
use eyre::Result;
use tokio::try_join;

#[derive(Default)]
pub struct MockTx {
    data: Option<Bytes>,
    tip: Option<(U256, U256)>,
    nonce_add: Option<U256>,
    to: Option<Address>,
}

impl MockTx {
    pub fn data<T: Into<Bytes>>(mut self, data: T) -> Self {
        self.data = Some(data.into());
        self
    }

    pub fn tip(mut self, tip: (U256, U256)) -> Self {
        self.tip = Some(tip);
        self
    }

    pub fn to<T: Into<Address>>(mut self, to: T) -> Self {
        self.to = Some(to.into());
        self
    }

    // TODO: I think there's an ethers SignerMiddleware in ethers-rs that does this
    pub fn nonce_add<T: Into<U256>>(mut self, nonce_add: T) -> Self {
        self.nonce_add = Some(nonce_add.into());
        self
    }

    pub async fn build(self) -> Result<Bytes> {
        let c = Config::from_env().await?;

        let (chain_id, fees, transaction_count) = try_join!(
            c.provider.get_chainid(),
            c.provider.estimate_eip1559_fees(None),
            c.provider
                .get_transaction_count(c.sender_wallet.address(), None),
        )?;

        let tip = self.tip.unwrap_or_default();
        let gas = 500_000;
        let nonce = transaction_count + self.nonce_add.unwrap_or(U256::zero());

        let tx: TypedTransaction = Eip1559TransactionRequest::new()
            .chain_id(chain_id.as_u64())
            .from(c.sender_wallet.address())
            .to(self.to.unwrap_or(c.sender_wallet.address()))
            .data(self.data.unwrap_or_default())
            .nonce(nonce)
            .gas(gas)
            .max_fee_per_gas(fees.0 + tip.0 / gas)
            .max_priority_fee_per_gas(fees.1 + tip.1 / gas)
            .into();

        Ok(tx.rlp_signed(&c.sender_wallet.sign_transaction_sync(&tx)?))
    }
}
