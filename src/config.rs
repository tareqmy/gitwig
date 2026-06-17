use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use toml;

/// Represents the structure of the configuration file.
/// Currently, it holds a list of strings to display in the UI.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub items: Vec<String>,
}

/// Attempts to load the configuration from a preferred order of locations.
///
/// Order of preference:
/// 1. CLI-provided path (if available and exists)
/// 2. Local config file at `config/config.toml`
/// 3. Global config file at `~/.config/twig/config.toml`
/// 4. If none found, fallback to a sarcastic default
///
/// # Arguments
/// * `cli_path` - An optional `PathBuf` passed as a command-line argument
///
/// # Returns
/// * `Ok(Config)` - Parsed configuration
/// * `Err` - If any reading or parsing fails during valid file paths
pub fn load_config(cli_path: Option<PathBuf>) -> Result<Config, Box<dyn Error>> {
    // 1. Try to load from CLI-provided path if available
    if let Some(path) = cli_path {
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&contents)?;
            return Ok(config);
        }
    }

    // 2. Try to load from local project directory: ./config/config.toml
    // First, check relative to current working directory
    let mut local_path = PathBuf::from("config/config.toml");
    if !local_path.exists() {
        // Fallback: check relative to the executable's directory
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                local_path = exe_dir.join("config/config.toml");
            }
        }
    }

    if local_path.exists() {
        let contents = fs::read_to_string(local_path)?;
        let config: Config = toml::from_str(&contents)?;
        return Ok(config);
    }

    // 3. Try to load from user config directory: e.g., ~/.config/twig/config.toml
    if let Some(global_path) = dirs::config_dir().map(|p| p.join("twig/config.toml")) {
        if global_path.exists() {
            let contents = fs::read_to_string(global_path)?;
            let config: Config = toml::from_str(&contents)?;
            return Ok(config);
        }
    }

    // 4. Config not found anywhere. Return sarcastic fallback.
    Ok(Config {
        items: vec![
            "Nice job. You forgot the config, genius.".to_string(),
            "Still looking... it's not here either.".to_string(),
            "Try harder next time.".to_string(),
        ],
    })
}
