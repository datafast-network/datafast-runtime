use crate::common::Chain;
use figment::providers::Env;
use figment::providers::Format;
use figment::providers::Toml;
use figment::Figment;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct DeltaConfig {
    pub table_path: String,
    pub query_step: u64,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum SourceTypes {
    Delta(DeltaConfig),
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
    pub database: Option<DatabaseConfig>,
    pub reorg_threshold: u16,
    pub metric_port: Option<u16>,
    pub rpc_endpoint: String,
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
    pub fn load() -> Self {
        let config_file_path = std::env::var("CONFIG").unwrap_or("config.toml".to_string());
        Figment::new()
            .merge(Toml::file(config_file_path))
            .merge(Env::prefixed("SWR_"))
            .extract()
            .expect("Load config failed")
    }

    pub fn get_subgraph_id(&self) -> String {
        self.subgraph_id
            .clone()
            .unwrap_or(self.subgraph_name.to_owned())
            .to_owned()
    }
}

#[cfg(test)]
mod test {
    use super::Config;

    #[test]
    fn test_config() {
        env_logger::try_init().unwrap_or_default();

        let config = Config::load();
        log::info!("Config = {:?}", config);
    }
}
