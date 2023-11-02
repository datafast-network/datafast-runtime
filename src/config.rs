use std::collections::HashMap;

use figment::providers::Env;
use figment::providers::Format;
use figment::providers::Toml;
use figment::Figment;
use serde::Deserialize;

use crate::errors::SwrError;

#[derive(Deserialize, Debug)]
pub struct Transform {
    pub datasource: String,
    pub func_name: String,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub subgraph_name: String,
    pub subgraph_id: Option<String>,
    pub manifest: String,
    pub transform: Option<HashMap<String, Transform>>,
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

    #[test]
    fn test_config_load() {
        ::env_logger::try_init().unwrap_or_default();
        let config = Config::load().unwrap();
        log::info!("Config: {:?}", config);
    }
}
