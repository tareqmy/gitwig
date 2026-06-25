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

fn default_max_commits() -> usize {
    0
}

fn default_page_size() -> usize {
    10
}

fn default_git_app() -> String {
    "gitui".to_string()
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct ThemeConfig {
    #[serde(default = "default_accent")]
    pub accent: String,
    #[serde(default = "default_warning")]
    pub warning: String,
    #[serde(default = "default_danger")]
    pub danger: String,
    #[serde(default = "default_success")]
    pub success: String,
    #[serde(default = "default_border_type")]
    pub border_type: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct FzfConfig {
    #[serde(default = "default_fzf_max_depth")]
    pub max_depth: usize,
    #[serde(default = "default_fzf_excludes")]
    pub excludes: Vec<String>,
    #[serde(default = "default_fzf_start_dir")]
    pub start_dir: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        default_theme()
    }
}

impl Default for FzfConfig {
    fn default() -> Self {
        default_fzf()
    }
}

fn default_accent() -> String {
    "cyan".to_string()
}
fn default_warning() -> String {
    "yellow".to_string()
}
fn default_danger() -> String {
    "red".to_string()
}
fn default_success() -> String {
    "green".to_string()
}
fn default_border_type() -> String {
    "rounded".to_string()
}

fn default_theme() -> ThemeConfig {
    ThemeConfig {
        accent: default_accent(),
        warning: default_warning(),
        danger: default_danger(),
        success: default_success(),
        border_type: default_border_type(),
    }
}

fn default_theme_name() -> String {
    "default".to_string()
}

fn default_fzf_max_depth() -> usize {
    6
}
fn default_fzf_excludes() -> Vec<String> {
    vec![]
}

fn default_fzf_start_dir() -> String {
    dirs::home_dir()
        .map(|p| {
            let mut s = p.to_string_lossy().into_owned();
            if !s.ends_with(std::path::MAIN_SEPARATOR) {
                s.push(std::path::MAIN_SEPARATOR);
            }
            s
        })
        .unwrap_or_else(|| "/".to_string())
}

fn default_fzf() -> FzfConfig {
    FzfConfig {
        max_depth: default_fzf_max_depth(),
        excludes: default_fzf_excludes(),
        start_dir: default_fzf_start_dir(),
    }
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
    /// Maximum commits to load in workspace view. Default is 0 (unlimited).
    #[serde(default = "default_max_commits")]
    pub max_commits: usize,
    /// Number of lines/items to scroll when PageUp or PageDown is pressed. Default is 10.
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    /// Sort mode for the main page.
    #[serde(default = "default_sort_by")]
    pub sort_by: SortOrder,
    /// Map of repository items to their last visit time.
    #[serde(default = "default_visits")]
    pub visits: std::collections::HashMap<String, u64>,
    /// Whether sorting should be reversed.
    #[serde(default)]
    pub sort_reverse: bool,
    /// List of pinned repository paths.
    #[serde(default)]
    pub pinned: std::collections::HashSet<String>,
    /// Theme configurations for styling the terminal TUI.
    #[serde(skip)]
    pub theme: ThemeConfig,
    /// Active theme name selection.
    #[serde(rename = "theme", default = "default_theme_name")]
    pub theme_name: String,
    /// Configuration for interactive repository discovery via fzf.
    #[serde(default = "default_fzf")]
    pub fzf: FzfConfig,
    /// Preferred Git application (e.g. gitui or lazygit).
    #[serde(default = "default_git_app")]
    pub git_app: String,
}

/// Returns `~/.gitwig/`, the canonical Gitwig data directory.
/// Falls back to `./.gitwig/` in the unlikely event that the home directory
/// cannot be resolved (e.g. inside a stripped-down container).
fn home_gitwig_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".gitwig")
}

/// Loads the configuration, ensuring `~/.gitwig/` always exists.
///
/// Resolution order:
/// 1. CLI-provided path (if given, skip all migration logic).
/// 2. `~/.gitwig/config.toml` — the canonical location. If it already
///    exists it is loaded directly.
/// 3. First-run migration: copy the first config found among
///    `./config/config.toml` (CWD or exe-dir), `~/.twig/config.toml`,
///    `~/.config/gitwig/config.toml`, or `~/.config/twig/config.toml`
///    into `~/.gitwig/config.toml`, then load from there.
/// 4. No prior config anywhere: write a default config to
///    `~/.gitwig/config.toml` so the next run is an ordinary case 2.
///
/// # Returns
/// `Ok((Config, PathBuf))` — the parsed config plus its write-back path.
pub fn load_config(cli_path: Option<PathBuf>) -> Result<(Config, PathBuf), Box<dyn Error>> {
    // ── 1. CLI override ───────────────────────────────────────────────────
    if let Some(path) = cli_path {
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let mut config: Config = toml::from_str(&contents)?;

            let themes_dir = path.parent().unwrap_or(&path).join("themes");
            fs::create_dir_all(&themes_dir)?;
            write_popular_themes(&themes_dir)?;

            let theme_path = themes_dir.join(format!("{}.theme", config.theme_name));
            if theme_path.exists() {
                let theme_contents = fs::read_to_string(&theme_path)?;
                if let Ok(theme) = toml::from_str::<ThemeConfig>(&theme_contents) {
                    config.theme = theme;
                }
            } else {
                let legacy_theme_path = path.with_file_name("theme.toml");
                if legacy_theme_path.exists() {
                    let _ = fs::copy(&legacy_theme_path, themes_dir.join("default.theme"));
                    let _ = fs::remove_file(&legacy_theme_path);
                }

                let theme_serialized = toml::to_string_pretty(&config.theme)?;
                fs::write(&theme_path, theme_serialized)?;
            }

            return Ok((config, path));
        }
        let fallback_theme = default_theme();
        let fallback_theme_name = default_theme_name();

        let themes_dir = path.parent().unwrap_or(&path).join("themes");
        fs::create_dir_all(&themes_dir)?;
        write_popular_themes(&themes_dir)?;
        let theme_path = themes_dir.join(format!("{}.theme", fallback_theme_name));
        let theme_serialized = toml::to_string_pretty(&fallback_theme)?;
        fs::write(&theme_path, theme_serialized)?;

        return Ok((
            Config {
                items: vec![],
                poll_interval_ms: default_poll_interval_ms(),
                max_commits: default_max_commits(),
                page_size: default_page_size(),
                sort_by: default_sort_by(),
                visits: default_visits(),
                sort_reverse: false,
                pinned: std::collections::HashSet::new(),
                theme_name: fallback_theme_name,
                theme: fallback_theme,
                fzf: default_fzf(),
                git_app: default_git_app(),
            },
            path,
        ));
    }

    // ── Always ensure ~/.gitwig/ exists ───────────────────────────────────
    let gitwig_dir = home_gitwig_dir();
    fs::create_dir_all(&gitwig_dir)?;
    let canonical = gitwig_dir.join("config.toml");
    let themes_dir = gitwig_dir.join("themes");
    fs::create_dir_all(&themes_dir)?;
    write_popular_themes(&themes_dir)?;

    // ── 2. Canonical file already present ─────────────────────────────────
    if canonical.exists() {
        let contents = fs::read_to_string(&canonical)?;
        let mut config: Config = toml::from_str(&contents)?;

        let theme_path = themes_dir.join(format!("{}.theme", config.theme_name));
        if theme_path.exists() {
            let theme_contents = fs::read_to_string(&theme_path)?;
            let theme: ThemeConfig = toml::from_str(&theme_contents)?;
            config.theme = theme;
        } else {
            let legacy_theme_path = gitwig_dir.join("theme.toml");
            if legacy_theme_path.exists() {
                let _ = fs::copy(&legacy_theme_path, themes_dir.join("default.theme"));
                let _ = fs::remove_file(&legacy_theme_path);
            }

            let theme_serialized = toml::to_string_pretty(&config.theme)?;
            fs::write(&theme_path, theme_serialized)?;
        }

        return Ok((config, canonical));
    }

    // ── 3. First run: migrate an existing config into ~/.gitwig/ ──────────
    if let Some(source) = find_legacy_config() {
        fs::copy(&source, &canonical)?;
        let contents = fs::read_to_string(&canonical)?;
        let mut config: Config = toml::from_str(&contents)?;

        let theme_path = themes_dir.join(format!("{}.theme", config.theme_name));
        if theme_path.exists() {
            let theme_contents = fs::read_to_string(&theme_path)?;
            let theme: ThemeConfig = toml::from_str(&theme_contents)?;
            config.theme = theme;
        } else {
            let legacy_theme_path = gitwig_dir.join("theme.toml");
            if legacy_theme_path.exists() {
                let _ = fs::copy(&legacy_theme_path, themes_dir.join("default.theme"));
                let _ = fs::remove_file(&legacy_theme_path);
            }

            let theme_serialized = toml::to_string_pretty(&config.theme)?;
            fs::write(&theme_path, theme_serialized)?;
        }

        return Ok((config, canonical));
    }

    // ── 4. No config anywhere: write a default and use it ─────────────────
    let fallback = Config {
        items: vec![
            "Nice job. You forgot the config, genius.".to_string(),
            "Still looking... it's not here either.".to_string(),
        ],
        poll_interval_ms: default_poll_interval_ms(),
        max_commits: default_max_commits(),
        page_size: default_page_size(),
        sort_by: default_sort_by(),
        visits: default_visits(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme_name: default_theme_name(),
        theme: default_theme(),
        fzf: default_fzf(),
        git_app: default_git_app(),
    };
    save_config(&fallback, &canonical)?;

    let theme_path = themes_dir.join(format!("{}.theme", fallback.theme_name));
    let theme_serialized = toml::to_string_pretty(&fallback.theme)?;
    fs::write(&theme_path, theme_serialized)?;

    Ok((fallback, canonical))
}

/// Writes the popular themes to the themes directory if they don't already exist.
fn write_popular_themes(themes_dir: &Path) -> Result<(), Box<dyn Error>> {
    let popular_themes = [
        (
            "dracula",
            r#"accent = "lightmagenta"
warning = "lightyellow"
danger = "lightred"
success = "lightgreen"
border_type = "rounded"
"#,
        ),
        (
            "forest",
            r#"accent = "lightgreen"
warning = "yellow"
danger = "lightred"
success = "green"
border_type = "rounded"
"#,
        ),
        (
            "gruvbox",
            r#"accent = "yellow"
warning = "lightyellow"
danger = "red"
success = "green"
border_type = "plain"
"#,
        ),
        (
            "monokai",
            r#"accent = "lightyellow"
warning = "yellow"
danger = "red"
success = "green"
border_type = "rounded"
"#,
        ),
        (
            "nord",
            r#"accent = "lightblue"
warning = "yellow"
danger = "red"
success = "green"
border_type = "rounded"
"#,
        ),
        (
            "oceanic",
            r#"accent = "lightcyan"
warning = "yellow"
danger = "red"
success = "lightgreen"
border_type = "rounded"
"#,
        ),
    ];

    for (name, content) in popular_themes {
        let theme_path = themes_dir.join(format!("{}.theme", name));
        if !theme_path.exists() {
            fs::write(theme_path, content)?;
        }
    }
    Ok(())
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
    // Legacy Twig home location: ~/.twig/config.toml.
    if let Some(home) = dirs::home_dir() {
        let p = home.join(".twig/config.toml");
        if p.exists() {
            return Some(p);
        }
    }
    // New Gitwig XDG config location: ~/.config/gitwig/config.toml.
    if let Some(p) = dirs::config_dir().map(|d| d.join("gitwig/config.toml")) {
        if p.exists() {
            return Some(p);
        }
    }
    // Legacy Twig XDG config location: ~/.config/twig/config.toml.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_separation_load_and_save() {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!("gitwig_test_theme_{}", unique_id));
        fs::create_dir_all(&test_dir).unwrap();
        let config_path = test_dir.join("config.toml");
        let themes_dir = test_dir.join("themes");
        let theme_path = themes_dir.join("default.theme");

        // 1. Initial load when files do not exist (should write themes/default.theme but not config.toml in CLI mode)
        let (config, path) = load_config(Some(config_path.clone())).unwrap();
        assert_eq!(path, config_path);
        assert!(!config_path.exists());
        assert!(theme_path.exists());

        // Verify config.theme is default
        assert_eq!(config.theme.accent, "cyan");
        assert_eq!(config.theme_name, "default");

        // Verify default.theme contains "accent = "cyan""
        let theme_content = fs::read_to_string(&theme_path).unwrap();
        assert!(theme_content.contains("accent = \"cyan\""));

        // Save config
        save_config(&config, &config_path).unwrap();
        assert!(config_path.exists());

        // Verify config.toml does NOT contain the [theme] section but has theme name string
        let config_content = fs::read_to_string(&config_path).unwrap();
        assert!(!config_content.contains("[theme]"));
        assert!(config_content.contains("theme = \"default\""));

        // 2. Modify default.theme and reload
        let custom_theme = r#"accent = "magenta"
warning = "yellow"
danger = "red"
success = "green"
border_type = "double"
"#;
        fs::write(&theme_path, custom_theme).unwrap();
        let (loaded_config, _) = load_config(Some(config_path.clone())).unwrap();
        assert_eq!(loaded_config.theme.accent, "magenta");
        assert_eq!(loaded_config.theme.border_type, "double");

        // 3. Save config and verify config.toml still has no [theme] section but has theme name string
        save_config(&loaded_config, &config_path).unwrap();
        let config_content_after_save = fs::read_to_string(&config_path).unwrap();
        assert!(!config_content_after_save.contains("[theme]"));
        assert!(config_content_after_save.contains("theme = \"default\""));

        // Clean up
        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_write_popular_themes_creates_files() {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let test_dir =
            std::env::temp_dir().join(format!("gitwig_test_popular_themes_{}", unique_id));
        fs::create_dir_all(&test_dir).unwrap();

        write_popular_themes(&test_dir).unwrap();

        // Verify that Dracula, Forest, Gruvbox, Monokai, Nord, and Oceanic files are written
        assert!(test_dir.join("dracula.theme").exists());
        assert!(test_dir.join("forest.theme").exists());
        assert!(test_dir.join("gruvbox.theme").exists());
        assert!(test_dir.join("monokai.theme").exists());
        assert!(test_dir.join("nord.theme").exists());
        assert!(test_dir.join("oceanic.theme").exists());

        // Read one theme to verify content
        let contents = fs::read_to_string(test_dir.join("oceanic.theme")).unwrap();
        assert!(contents.contains("accent = \"lightcyan\""));

        let _ = fs::remove_dir_all(&test_dir);
    }
}
