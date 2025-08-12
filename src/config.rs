use crate::settings;
use color_eyre::Result; // Add this import
use dirs::config_dir;
use std::fs;
use std::path::PathBuf;

pub struct Config;

impl Config {
    fn get_config_path() -> Result<PathBuf> {
        // Use Result instead of Result<PathBuf, _>
        let config_dir = config_dir()
            .ok_or_else(|| color_eyre::eyre::eyre!("Could not find config directory"))?;

        let app_config_dir = config_dir.join("fokus");

        // eprintln!("Debug - config_dir: {:?}", config_dir); // Goes to stderr
        // eprintln!("Debug - app_config_dir: {:?}", app_config_dir);

        // Create the directory if it doesn't exist
        fs::create_dir_all(&app_config_dir)?;
        Ok(app_config_dir.join("settings.toml"))
    }

    pub fn load_settings() -> Result<settings::Settings> {
        // Use Result instead of Result<settings::Settings, _>
        let config_path = Self::get_config_path()?;
        println!("{:?}", config_path);

        if config_path.exists() {
            let config_str = fs::read_to_string(config_path)?;
            let settings: settings::Settings = toml::from_str(&config_str)?;
            Ok(settings)
        } else {
            // Return default settings if file doesn't exist
            Ok(settings::Settings::default())
        }
    }

    pub fn save_settings(settings: &settings::Settings) -> Result<()> {
        // Use Result<()> instead of Result<(), _>
        let config_path = Self::get_config_path()?;
        let config_str = toml::to_string_pretty(settings)?;
        fs::write(config_path, config_str)?;
        Ok(())
    }
}
