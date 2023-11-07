use crate::common::Chain;
use crate::errors::SwrError;
use figment::providers::Env;
use figment::providers::Format;
use figment::providers::Toml;
use figment::Figment;
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub chain: Chain,
    pub subgraph_name: String,
    pub subgraph_id: Option<String>,
    pub manifest: String,
    pub transform: Option<TransformConfig>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TransformConfig {
    Ethereum {
        block: String,
        transactions: String,
        logs: String,
    },
    Mock,
}

impl Config {
    pub fn load() -> Result<Self, SwrError> {
        Figment::new()
            .merge(Toml::file("config.toml"))
            .merge(Env::prefixed("SWR_"))
            .extract()
            .map_err(|e| SwrError::ConfigLoadFail(e.to_string()))
    }
}

#[cfg(test)]
mod test {
    use super::Config;
    use env_logger;

    #[test]
    fn test_config() {
        env_logger::try_init().unwrap_or_default();

        let config = Config::load().unwrap();
        ::log::info!("Config = {:?}", config);
    }
}
