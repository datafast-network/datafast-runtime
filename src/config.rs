use crate::common::Chain;
use figment::providers::Env;
use figment::providers::Format;
use figment::providers::Toml;
use figment::Figment;
use serde::Deserialize;

#[cfg(feature = "deltalake")]
#[derive(Clone, Debug, Deserialize)]
pub struct DeltaConfig {
    pub table_path: String,
    pub query_step: u64,
    pub version: Option<u64>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum SourceTypes {
    #[cfg(feature = "deltalake")]
    Delta(DeltaConfig),
    #[cfg(feature = "pubsub")]
    PubSub { sub_id: String, compression: bool },
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseConfig {
    #[cfg(feature = "scylla")]
    Scylla { uri: String, keyspace: String },
    #[cfg(feature = "mongo")]
    Mongo { uri: String, database: String },
}

#[derive(Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub struct ValveConfig {
    pub allowed_lag: u64,
    pub wait_time: u64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub chain: Chain,
    pub source: SourceTypes,
    pub subgraph_name: String,
    pub subgraph_dir: String,
    pub database: DatabaseConfig,
    pub reorg_threshold: u16,
    pub metric_port: Option<u16>,
    pub rpc_endpoint: String,
    pub valve: ValveConfig,
    pub block_data_retention: Option<u64>,
}

impl Config {
    pub fn load() -> Self {
        let config_file_path = std::env::var("CONFIG").unwrap_or("config.toml".to_string());
        let cfg: Config = Figment::new()
            .merge(Toml::file(config_file_path))
            .merge(Env::prefixed("DFR_"))
            .extract()
            .expect("Load config failed");

        if let Some(size) = cfg.block_data_retention {
            assert!(
                size > 20000,
                "per-block data should be stored for at least 20_000 blocks"
            );
        }

        cfg
    }
}

#[cfg(test)]
mod test {
    use super::Config;
    use df_logger::loggers::init_logger;
    #[test]
    fn test_config() {
        init_logger();

        let config = Config::load();
        df_logger::log::info!("Config = {:?}", config);
    }
}
