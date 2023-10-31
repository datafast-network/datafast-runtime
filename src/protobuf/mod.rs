use serde::Deserialize;
use serde::Serialize;
use std::fmt;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use strum::Display;
use strum::EnumString;

pub mod ethereum;

#[derive(Debug, Clone, Display, Serialize, Deserialize, EnumString)]
pub enum EvmChainName {
    #[strum(serialize = "ethereum")]
    Ethereum,
    #[strum(serialize = "bsc")]
    Bsc,
    #[strum(serialize = "polygon")]
    Polygon,
    #[strum(serialize = "optimism")]
    Optimism,
    #[strum(serialize = "fantom")]
    Fantom,
    #[strum(serialize = "avalanche")]
    Avalanche,
    #[strum(serialize = "arbitrum")]
    Arbitrum,
    #[strum(serialize = "evm-unknown")]
    Unknown,
}

impl Default for EvmChainName {
    fn default() -> Self {
        Self::Ethereum
    }
}

impl From<u64> for EvmChainName {
    fn from(chain_id: u64) -> Self {
        match chain_id {
            1 => EvmChainName::Ethereum,
            56 => EvmChainName::Bsc,
            137 => EvmChainName::Polygon,
            10 => EvmChainName::Optimism,
            250 => EvmChainName::Fantom,
            43114 => EvmChainName::Avalanche,
            42161 => EvmChainName::Arbitrum,
            _ => EvmChainName::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, EnumString)]
pub enum BlockChain {
    #[strum(serialize = "ethereum")]
    Ethereum,
    #[strum(serialize = "mock")]
    MockChain,
}

impl fmt::Display for BlockChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlockChain::Ethereum => write!(f, "ethereum"),
            BlockChain::MockChain => write!(f, "vutr@MockChain"),
        }
    }
}

pub trait BlockTrait:
    prost::Message + Serialize + Default + From<(u64, String, String)> + Clone + Send + Sized + 'static
{
    fn get_blockchain(&self) -> BlockChain;
    fn get_number(&self) -> u64;
    fn get_hash(&self) -> String;
    fn get_parent_hash(&self) -> String;
    fn get_writer_timestamp(&self) -> u64 {
        let now = SystemTime::now();
        now.duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
    }
}
