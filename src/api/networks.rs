use ethers::types::Chain;

#[derive(Debug, Clone)]
pub struct MevShareNetwork {
    // pub name: &'static str,
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

pub enum Network {
    Mainnet,
    Goerli,
}

impl From<Network> for MevShareNetwork {
    fn from(value: Network) -> Self {
        match value {
            Network::Mainnet => MAINNET,
            Network::Goerli => GOERLI,
        }
    }
}
