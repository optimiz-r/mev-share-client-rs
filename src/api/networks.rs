use ethers::types::{Chain, U256};

#[derive(Debug, Clone)]
pub struct MevShareNetwork {
    pub chain: Chain,
    pub stream_url: &'static str,
    pub api_url: &'static str,
}

const MAINNET: MevShareNetwork = MevShareNetwork {
    chain: Chain::Mainnet,
    stream_url: "https://mev-share.flashbots.net",
    api_url: "https://relay.flashbots.net",
};

const GOERLI: MevShareNetwork = MevShareNetwork {
    chain: Chain::Goerli,
    stream_url: "https://mev-share-goerli.flashbots.net",
    api_url: "https://relay-goerli.flashbots.net",
};

// const SEPOLIA: MevShareNetwork = MevShareNetwork {
//     chain: Chain::Sepolia,
//     stream_url: "NOT AVAILABLE YET",
//     api_url: "https://relay-sepolia.flashbots.net",
// };

impl TryFrom<U256> for MevShareNetwork {
    type Error = crate::Error;

    fn try_from(chain: U256) -> Result<Self, Self::Error> {
        match chain.as_u64() {
            1 => Ok(MAINNET),
            5 => Ok(GOERLI),
            // 11_155_111 => Ok(SEPOLIA),
            _ => Err(crate::Error::UnsupportedNetwork(chain)),
        }
    }
}
