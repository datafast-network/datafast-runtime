use figment::providers::Env;
use figment::providers::Format;
use figment::providers::Toml;
use figment::Figment;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub manifest: String,
}

impl Config {
    pub fn load() -> Self {
        let config: Config = Figment::new()
            .merge(Toml::file("config.toml"))
            .merge(Env::prefixed("SWR_"))
            .extract()
            .expect("Failed to init config");
        return config;
    }
}
