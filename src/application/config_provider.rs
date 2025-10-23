use std::path::PathBuf;

use anyhow::{Context, Result};
use config::Config;
use directories::ProjectDirs;
use log::{debug, warn};

use crate::configuration::{AppConfig, Settings, SettingsPatch};

pub struct ConfigProvider {
    dynamic_settings_path: Option<PathBuf>,
    settings_path: String,
}

impl ConfigProvider {
    pub fn new() -> ConfigProvider {
        let settings_path = std::env::var("DYNAMIC_SETTINGS_PATH");
        let dynamic_settings_path = if let Ok(path) = settings_path {
            Some(PathBuf::from(path))
        } else if let Some(proj_dirs) = ProjectDirs::from("com", "xabufr", "photokiosk") {
            Some(proj_dirs.config_dir().join("settings.yaml"))
        } else {
            warn!("Cannot find settings path");
            None
        };

        let settings_path = std::env::var("SETTINGS_PATH").unwrap_or("settings".to_string());
        ConfigProvider {
            dynamic_settings_path,
            settings_path: settings_path,
        }
    }

    pub fn load_settings(&self) -> Result<Settings> {
        let mut builder =
            Config::builder().add_source(::config::File::with_name(&self.settings_path));

        if let Some(settings_path) = &self.dynamic_settings_path {
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use googletest::{
        expect_that, gtest,
        matchers::matches_pattern,
        prelude::{approx_eq, eq},
    };
    use temp_dir::TempDir;

    use super::ConfigProvider;

    #[gtest]
    fn test_load_default_settings() {
        let settings = "";
        let settings_dir = gen_settings_from_str(settings).unwrap();

        let provider = ConfigProvider {
            dynamic_settings_path: None,
            settings_path: settings_dir
                .path()
                .join("settings.yaml")
                .to_str()
                .unwrap()
                .to_string(),
        };
        let settings = provider.load_settings().unwrap();
        expect_that!(settings.debug.show_fps, eq(false));
    }

    #[gtest]
    fn test_load_missing_dynamic_file() {
        let settings = "";
        let settings_dir = gen_settings_from_str(settings).unwrap();
        let empty_dir = empty_dir().unwrap();

        let provider = ConfigProvider {
            dynamic_settings_path: Some(empty_dir.path().join("missing.yaml")),
            settings_path: settings_dir
                .path()
                .join("settings.yaml")
                .to_str()
                .unwrap()
                .to_string(),
        };
        let settings = provider.load_settings().unwrap();
        expect_that!(settings.transition_duration, eq(Duration::from_millis(500)));
    }

    #[gtest]
    fn test_load_existing_settings() {
        let settings = r#"---
debug:
  show_fps: true
"#;
        let settings_dir = gen_settings_from_str(settings).unwrap();

        let provider = ConfigProvider {
            dynamic_settings_path: None,
            settings_path: settings_dir
                .path()
                .join("settings.yaml")
                .to_str()
                .unwrap()
                .to_string(),
        };
        let settings = provider.load_settings().unwrap();
        expect_that!(settings.debug.show_fps, eq(true));
    }

    #[gtest]
    fn test_load_existing_settings_overloaded() {
        let settings = r#"---
debug:
  show_fps: true
"#;
        let settings_dir = gen_settings_from_str(settings).unwrap();
        let settings = r#"---
debug:
    show_fps: false
"#;
        let overload_dir = gen_settings_from_str(settings).unwrap();

        let provider = ConfigProvider {
            dynamic_settings_path: Some(overload_dir.path().join("settings.yaml")),
            settings_path: settings_dir
                .path()
                .join("settings.yaml")
                .to_str()
                .unwrap()
                .to_string(),
        };
        let settings = provider.load_settings().unwrap();
        expect_that!(settings.debug.show_fps, eq(false));
    }

    fn gen_settings_from_str(s: &str) -> Result<TempDir, anyhow::Error> {
        let temp_dir = TempDir::new().unwrap();
        let settings_path = temp_dir.path().join("settings.yaml");
        std::fs::write(&settings_path, s).unwrap();
        Ok(temp_dir)
    }

    fn empty_dir() -> Result<TempDir, anyhow::Error> {
        let temp_dir = TempDir::new().unwrap();
        Ok(temp_dir)
    }
}
