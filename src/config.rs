use crate::common::Chain;
use crate::errors::SwrError;
use figment::providers::Env;
use figment::providers::Format;
use figment::providers::Toml;
use figment::Figment;
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum SourceTypes {
    ReadLine,
    ReadDir {
        source_dir: String,
    },
    Nats {
        uri: String,
        subject: String,
        content_type: ContentType,
    },
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Json,
    Protobuf,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseConfig {
    Scylla { uri: String, keyspace: String },
}

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub chain: Chain,
    pub source: SourceTypes,
    pub subgraph_name: String,
    pub subgraph_id: Option<String>,
    pub subgraph_dir: String,
    pub transform: Option<TransformConfig>,
    pub transform_wasm: Option<String>,
    pub database: Option<DatabaseConfig>,
    pub reorg_threshold: u16,
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
        let config_file_path = std::env::var("CONFIG").unwrap_or("config.toml".to_string());
        Figment::new()
            .merge(Toml::file(config_file_path))
            .merge(Env::prefixed("SWR_"))
            .extract()
            .map_err(|e| SwrError::ConfigLoadFail(e.to_string()))
    }
}

#[cfg(test)]
mod test {
    use super::Config;

    #[test]
    fn test_config() {
        env_logger::try_init().unwrap_or_default();

        let config = Config::load().unwrap();
        log::info!("Config = {:?}", config);
    }
}
