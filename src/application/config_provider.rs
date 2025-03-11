use crate::configuration::AppConfiguration;
use anyhow::{Context, Result};
use config::Config;

struct ConfigProvider {}

impl ConfigProvider {
    pub fn new() -> ConfigProvider {
        ConfigProvider {}
    }

    pub fn load_config(&self) -> Result<AppConfiguration> {
        let config_path = std::env::var("CONFIG_PATH").unwrap_or("config.yaml".to_string());
        let settings = Config::builder()
            .add_source(::config::File::with_name(&config_path))
            .build()
            .context("Cannot parse configuration")?;
        let config: AppConfiguration = settings
            .try_deserialize()
            .context("Cannot deserialize configuration")?;
        Ok(config)
    }
}
