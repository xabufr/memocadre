use crate::configuration::{AppSources, Settings};
use anyhow::{Context, Result};
use config::Config;

pub struct ConfigProvider {}

impl ConfigProvider {
    pub fn new() -> ConfigProvider {
        ConfigProvider {}
    }

    pub fn load_settings(&self) -> Result<Settings> {
        let config_path = std::env::var("SETTINGS_PATH").unwrap_or("settings".to_string());
        let settings = Config::builder()
            .add_source(::config::File::with_name(&config_path))
            .build()
            .context("Cannot parse configuration")?;
        let config: Settings = settings
            .try_deserialize()
            .context("Cannot deserialize settings")?;
        Ok(config)
    }

    pub fn load_sources(&self) -> Result<AppSources> {
        let config_path = std::env::var("SOURCES_PATH").unwrap_or("sources".to_string());
        let settings = Config::builder()
            .add_source(::config::File::with_name(&config_path))
            .build()
            .context("Cannot parse configuration")?;
        let config: AppSources = settings
            .try_deserialize()
            .context("Cannot deserialize sources")?;
        Ok(config)
    }
}
