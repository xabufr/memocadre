use std::path::PathBuf;

use anyhow::{Context, Result};
use config::Config;
use directories::ProjectDirs;
use log::{debug, warn};

use crate::configuration::{AppConfig, Settings, SettingsPatch};

pub struct ConfigProvider {
    settings_path: Option<PathBuf>,
}

impl ConfigProvider {
    pub fn new() -> ConfigProvider {
        let settings_path = std::env::var("DYNAMIC_SETTINGS_PATH");
        let settings_path = if let Ok(path) = settings_path {
            Some(PathBuf::from(path))
        } else if let Some(proj_dirs) = ProjectDirs::from("com", "xabufr", "photokiosk") {
            Some(proj_dirs.config_dir().join("settings.yaml"))
        } else {
            warn!("Cannot find settings path");
            None
        };
        ConfigProvider { settings_path }
    }

    pub fn load_settings(&self) -> Result<Settings> {
        let config_path = std::env::var("SETTINGS_PATH").unwrap_or("settings".to_string());
        let mut builder = Config::builder().add_source(::config::File::with_name(&config_path));

        if let Some(settings_path) = &self.settings_path {
            debug!("Loading settings from {:?}", settings_path);
            let source = ::config::File::from(settings_path.as_path()).required(false);
            builder = builder.add_source(source);
        }

        let settings = builder.build().context("Cannot parse configuration")?;
        let config: Settings = settings
            .try_deserialize()
            .context("Cannot deserialize settings")?;
        Ok(config)
    }

    pub fn load_config(&self) -> Result<AppConfig> {
        let config_path = std::env::var("CONFIG_PATH").unwrap_or("config".to_string());
        let settings = Config::builder()
            .add_source(::config::File::with_name(&config_path))
            .build()
            .context("Cannot parse configuration")?;
        let config: AppConfig = settings
            .try_deserialize()
            .context("Cannot deserialize sources")?;
        Ok(config)
    }

    pub fn save_settings_override(&self, settings: &SettingsPatch) -> Result<()> {
        // TODO implement me!
        anyhow::bail!("Not implemented");
    }
}
