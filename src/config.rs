use figment::providers::Env;
use figment::providers::Format;
use figment::providers::Toml;
use figment::Figment;
use serde::Deserialize;
use std::collections::HashMap;

use crate::errors::SwrError;

#[derive(Deserialize)]
pub struct Config {
    pub subgraph_name: String,
    pub subgraph_id: Option<String>,
    pub manifest: String,
    pub transforms: Option<HashMap<String, TransformConfig>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TransformConfig {
    pub datasource: String,
    pub func_name: String,
}

impl Config {
    pub fn load() -> Result<Self, SwrError> {
        Figment::new()
            .merge(Toml::file("config.toml"))
            .merge(Env::prefixed("SWR_"))
            .extract()
            .map_err(|_| SwrError::ConfigLoadFail)
    }
}
