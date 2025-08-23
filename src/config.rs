use crate::{settings};
use color_eyre::Result;
use dirs::config_dir;
use std::{fs, path::PathBuf};

pub struct Config;

impl Config {
    pub fn get_config_dir() -> Result<PathBuf> {
        let config_dir = config_dir()
            .ok_or_else(|| color_eyre::eyre::eyre!("Could not find config directory"))?;
        let app_config_dir = config_dir.join("fokus");
        // Create the directory if it doesn't exist
        fs::create_dir_all(&app_config_dir)?;
        Ok(app_config_dir)
    }

    fn get_settings_path() -> Result<PathBuf> {
        Ok(Self::get_config_dir()?.join("settings.toml"))
    }

    pub fn load_settings() -> Result<settings::Settings> {
        let config_path = Self::get_settings_path()?;
        if config_path.exists() {
            let config_str = fs::read_to_string(config_path)?;
            let settings: settings::Settings = toml::from_str(&config_str)?;
            Ok(settings)
        } else {
            Ok(settings::Settings::default())
        }
    }

    pub fn save_settings(settings: &settings::Settings) -> Result<()> {
        let config_path = Self::get_settings_path()?;
        let config_str = toml::to_string_pretty(settings)?;
        fs::write(config_path, config_str)?;
        Ok(())
    }
}
