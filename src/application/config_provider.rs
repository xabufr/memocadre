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

    /// Saves the settings patch to the dynamic settings file.
    ///
    /// This function serializes the provided `SettingsPatch` to YAML format and writes it to
    /// the settings file. The file location is determined by:
    /// 1. The `DYNAMIC_SETTINGS_PATH` environment variable, if set
    /// 2. The platform-specific user config directory (e.g., `~/.config/photokiosk/settings.yaml` on Linux)
    /// 3. Falls back to an error if no path can be determined
    ///
    /// The function will create parent directories as needed if they don't exist.
    ///
    /// # Usage with MQTT
    /// Settings can be changed via MQTT by publishing to the command topic:
    /// ```json
    /// { "type": "display_duration", "value": 45 }
    /// ```
    ///
    /// # Usage with HTTP
    /// Settings can be changed via HTTP PATCH to `/settings`:
    /// ```bash
    /// curl -X PATCH http://localhost:3000/settings \
    ///   -H "Content-Type: application/json" \
    ///   -d '{"display_duration": "45s"}'
    /// ```
    ///
    /// # Returns
    /// - `Ok(())` if the settings were successfully saved
    /// - `Err` if there was an error saving the settings (e.g., no settings path, I/O error)
    pub fn save_settings_override(&self, settings: &SettingsPatch) -> Result<()> {
        let settings_path = self
            .settings_path
            .as_ref()
            .context("No settings path available")?;

        // Create parent directories if they don't exist
        if let Some(parent) = settings_path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create parent directories for settings file")?;
        }

        // Serialize the settings patch to YAML
        let yaml = serde_yaml::to_string(settings).context("Failed to serialize settings")?;

        // Write to file
        std::fs::write(settings_path, yaml).context("Failed to write settings file")?;

        debug!("Settings saved to {:?}", settings_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_save_and_load_settings_override() {
        // Create a temporary directory for testing
        let temp_dir = std::env::temp_dir().join("photokiosk_test");
        std::fs::create_dir_all(&temp_dir).unwrap();

        let settings_path = temp_dir.join("test_settings.yaml");

        // Set the environment variable to use our test path
        std::env::set_var("DYNAMIC_SETTINGS_PATH", settings_path.to_str().unwrap());

        let provider = ConfigProvider::new();

        // Create a settings patch
        let patch = SettingsPatch {
            display_duration: Some(Duration::from_secs(60)),
            transition_duration: Some(Duration::from_millis(1000)),
            ..Default::default()
        };

        // Save the settings
        let result = provider.save_settings_override(&patch);
        assert!(result.is_ok(), "Failed to save settings: {:?}", result.err());

        // Verify the file was created
        assert!(settings_path.exists(), "Settings file was not created");

        // Read the file and verify its contents
        let content = std::fs::read_to_string(&settings_path).unwrap();
        // Check that the file contains duration fields (format can vary)
        assert!(
            content.contains("display_duration") || content.contains("displayDuration"),
            "File should contain display_duration field. Content: {}",
            content
        );

        // Clean up
        std::fs::remove_file(&settings_path).ok();
        std::fs::remove_dir(&temp_dir).ok();
        std::env::remove_var("DYNAMIC_SETTINGS_PATH");
    }

    #[test]
    fn test_save_settings_creates_parent_dirs() {
        // Create a temporary directory for testing
        let temp_dir = std::env::temp_dir().join("photokiosk_test_nested");
        let settings_path = temp_dir.join("nested").join("path").join("settings.yaml");

        // Make sure the directories don't exist
        std::fs::remove_dir_all(&temp_dir).ok();

        // Set the environment variable to use our test path
        std::env::set_var("DYNAMIC_SETTINGS_PATH", settings_path.to_str().unwrap());

        let provider = ConfigProvider::new();

        // Create a settings patch
        let patch = SettingsPatch {
            display_duration: Some(Duration::from_secs(45)),
            ..Default::default()
        };

        // Save the settings
        let result = provider.save_settings_override(&patch);
        assert!(result.is_ok(), "Failed to save settings: {:?}", result.err());

        // Verify the file and parent directories were created
        assert!(settings_path.exists(), "Settings file was not created");
        assert!(
            settings_path.parent().unwrap().exists(),
            "Parent directories were not created"
        );

        // Clean up
        std::fs::remove_dir_all(&temp_dir).ok();
        std::env::remove_var("DYNAMIC_SETTINGS_PATH");
    }
}
