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
        if let Some(dynamic_settings_path) = &self.dynamic_settings_path {
            let existing_patch = if dynamic_settings_path.exists() {
                let file = std::fs::File::open(dynamic_settings_path)
                    .context("Cannot open existing dynamic settings file")?;
                serde_json::from_reader(file)
                    .context("Cannot parse existing dynamic settings file")?
            } else {
                SettingsPatch::default()
            };
            let merged_patch = existing_patch + settings.clone();
            let writer = std::fs::File::create(dynamic_settings_path)
                .context("Cannot create dynamic settings file to save settings override")?;
            serde_json::to_writer(writer, &merged_patch)
                .context("Cannot serialize settings override to dynamic settings file")?;
        } else {
            warn!("Dynamic settings path is not set; cannot save settings override");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use googletest::{expect_that, gtest, prelude::eq};
    use temp_dir::TempDir;

    use crate::configuration::SettingsPatch;

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

    #[gtest]
    fn test_save_settings_overloaded() {
        let settings = r#"---
debug:
  show_fps: true
"#;
        let settings_dir = gen_settings_from_str(settings).unwrap();
        let settings = r#"{"display_duration":"51s"}"#;
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
        expect_that!(settings.debug.show_fps, eq(true));
        provider
            .save_settings_override(&SettingsPatch {
                debug: Some(crate::configuration::DebugSettingsPatch {
                    show_fps: Some(false),
                }),
                ..Default::default()
            })
            .unwrap();
        // print file contents for debugging
        let saved_contents =
            std::fs::read_to_string(overload_dir.path().join("settings.yaml")).unwrap();
        println!("Saved contents: {}", saved_contents);
        let settings = provider.load_settings().unwrap();
        expect_that!(
            settings.debug.show_fps,
            eq(false),
            "Should retain saved override"
        );
        assert_eq!(settings.display_duration, Duration::from_secs(51));
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
