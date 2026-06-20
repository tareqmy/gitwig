use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Alphabetical,
    RecentVisit,
    LatestChanges,
    Custom,
}

fn default_sort_by() -> SortOrder {
    SortOrder::Custom
}

fn default_visits() -> std::collections::HashMap<String, u64> {
    std::collections::HashMap::new()
}

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
    /// Sort mode for the main page.
    #[serde(default = "default_sort_by")]
    pub sort_by: SortOrder,
    /// Map of repository items to their last visit time.
    #[serde(default = "default_visits")]
    pub visits: std::collections::HashMap<String, u64>,
    /// Whether sorting should be reversed.
    #[serde(default)]
    pub sort_reverse: bool,
}

/// Returns `~/.twig/`, the canonical Twig data directory.
/// Falls back to `./.twig/` in the unlikely event that the home directory
/// cannot be resolved (e.g. inside a stripped-down container).
fn home_twig_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".twig")
}

/// Loads the configuration, ensuring `~/.twig/` always exists.
///
/// Resolution order:
/// 1. CLI-provided path (if given, skip all migration logic).
/// 2. `~/.twig/config.toml` — the canonical location. If it already
///    exists it is loaded directly.
/// 3. First-run migration: copy the first config found among
///    `./config/config.toml` (CWD or exe-dir) or `~/.config/twig/config.toml`
///    into `~/.twig/config.toml`, then load from there.
/// 4. No prior config anywhere: write a default config to
///    `~/.twig/config.toml` so the next run is an ordinary case 2.
///
/// # Returns
/// `Ok((Config, PathBuf))` — the parsed config plus its write-back path.
pub fn load_config(cli_path: Option<PathBuf>) -> Result<(Config, PathBuf), Box<dyn Error>> {
    // ── 1. CLI override ───────────────────────────────────────────────────
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
                sort_by: default_sort_by(),
                visits: default_visits(),
                sort_reverse: false,
            },
            path,
        ));
    }

    // ── Always ensure ~/.twig/ exists ─────────────────────────────────────
    let twig_dir = home_twig_dir();
    fs::create_dir_all(&twig_dir)?;
    let canonical = twig_dir.join("config.toml");

    // ── 2. Canonical file already present ─────────────────────────────────
    if canonical.exists() {
        let contents = fs::read_to_string(&canonical)?;
        let config: Config = toml::from_str(&contents)?;
        return Ok((config, canonical));
    }

    // ── 3. First run: migrate an existing config into ~/.twig/ ────────────
    if let Some(source) = find_legacy_config() {
        fs::copy(&source, &canonical)?;
        let contents = fs::read_to_string(&canonical)?;
        let config: Config = toml::from_str(&contents)?;
        return Ok((config, canonical));
    }

    // ── 4. No config anywhere: write a default and use it ─────────────────
    let fallback = Config {
        items: vec![
            "Nice job. You forgot the config, genius.".to_string(),
            "Still looking... it's not here either.".to_string(),
            "Try harder next time.".to_string(),
        ],
        poll_interval_ms: default_poll_interval_ms(),
        sort_by: default_sort_by(),
        visits: default_visits(),
        sort_reverse: false,
    };
    save_config(&fallback, &canonical)?;
    Ok((fallback, canonical))
}

/// Searches for a pre-existing config at legacy / local locations.
/// Returns the first path that exists, or `None`.
fn find_legacy_config() -> Option<PathBuf> {
    // Local project config relative to CWD.
    let local = PathBuf::from("config/config.toml");
    if local.exists() {
        return Some(local);
    }
    // Local project config relative to the executable directory.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("config/config.toml");
            if p.exists() {
                return Some(p);
            }
        }
    }
    // Old XDG location: ~/.config/twig/config.toml.
    if let Some(p) = dirs::config_dir().map(|d| d.join("twig/config.toml")) {
        if p.exists() {
            return Some(p);
        }
    }
    None
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
