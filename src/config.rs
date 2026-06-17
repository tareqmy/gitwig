use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// How long the event loop waits for input before re-drawing (milliseconds).
/// Lower values feel more responsive; higher values use less CPU.
fn default_poll_interval_ms() -> u64 {
    100
}

/// Represents the structure of the configuration file.
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// Repository/directory paths shown in the main list.
    pub items: Vec<String>,
    /// Event-loop poll interval in milliseconds (default: 100).
    /// Lower → more responsive, higher → less CPU. Sane range: 16–500.
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,
}

/// Resolves the path that should be used for persisting config edits when
/// no config file currently exists. Prefers the user's global config dir
/// (`~/.config/twig/config.toml`); falls back to `./config/config.toml`.
fn default_write_path() -> PathBuf {
    if let Some(global) = dirs::config_dir().map(|p| p.join("twig/config.toml")) {
        return global;
    }
    PathBuf::from("config/config.toml")
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
/// * `Ok((Config, PathBuf))` - Parsed configuration plus the path it should
///   be written back to when the user edits items.
/// * `Err` - If any reading or parsing fails during valid file paths
pub fn load_config(cli_path: Option<PathBuf>) -> Result<(Config, PathBuf), Box<dyn Error>> {
    // 1. Try to load from CLI-provided path if available. If the user passed
    // a path that doesn't exist yet, we still honor it as the write target.
    if let Some(path) = cli_path {
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&contents)?;
            return Ok((config, path));
        }
        return Ok((
            Config {
                items: vec![],
                poll_interval_ms: default_poll_interval_ms(),
            },
            path,
        ));
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
        let contents = fs::read_to_string(&local_path)?;
        let config: Config = toml::from_str(&contents)?;
        return Ok((config, local_path));
    }

    // 3. Try to load from user config directory: e.g., ~/.config/twig/config.toml
    if let Some(global_path) = dirs::config_dir().map(|p| p.join("twig/config.toml")) {
        if global_path.exists() {
            let contents = fs::read_to_string(&global_path)?;
            let config: Config = toml::from_str(&contents)?;
            return Ok((config, global_path));
        }
    }

    // 4. Config not found anywhere. Return sarcastic fallback. Subsequent
    // edits will be persisted to the default write path so the user gets
    // a real file they can keep.
    let fallback = Config {
        items: vec![
            "Nice job. You forgot the config, genius.".to_string(),
            "Still looking... it's not here either.".to_string(),
            "Try harder next time.".to_string(),
        ],
        poll_interval_ms: default_poll_interval_ms(),
    };
    Ok((fallback, default_write_path()))
}

/// Serializes the config back to TOML and writes it to `path`, creating any
/// missing parent directories first.
pub fn save_config(config: &Config, path: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let serialized = toml::to_string_pretty(config)?;
    fs::write(path, serialized)?;
    Ok(())
}
