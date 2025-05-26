use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use toml;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub items: Vec<String>,
}

pub fn load_config(cli_path: Option<PathBuf>) -> Result<Config, Box<dyn Error>> {
    if let Some(path) = cli_path {
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&contents)?;
            return Ok(config);
        }
    }

    // 1. Try local config/config.toml
    let local_path = Path::new("config/config.toml");
    if local_path.exists() {
        let contents = fs::read_to_string(local_path)?;
        let config: Config = toml::from_str(&contents)?;
        return Ok(config);
    }

    // 2. Try ~/.config/twig/config.toml
    if let Some(global_path) = dirs::home_dir().map(|p| p.join(".config/twig/config.toml")) {
        if global_path.exists() {
            let contents = fs::read_to_string(global_path)?;
            let config: Config = toml::from_str(&contents)?;
            return Ok(config);
        }
    }

    // 3. No config found. Return sarcastic default
    Ok(Config {
        items: vec![
            "Nice job. You forgot the config, genius.".to_string(),
            "Still looking... it's not here either.".to_string(),
            "Try harder next time.".to_string(),
        ],
    })
}
