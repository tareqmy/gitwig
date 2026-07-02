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
    500
}

fn default_graph_max_commits() -> usize {
    1000
}

fn default_detail_cache_ttl_secs() -> u64 {
    30
}

fn default_tab_ttl_secs() -> u64 {
    60
}

fn default_page_size() -> usize {
    10
}

fn default_git_app() -> String {
    "gitui".to_string()
}

pub fn ssh_command_val() -> &'static str {
    if std::env::var("GITWIG_SSH_STRICT").map(|v| v == "1").unwrap_or(false) {
        "ssh -o StrictHostKeyChecking=yes"
    } else {
        "ssh -o StrictHostKeyChecking=accept-new"
    }
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct RepoConfig {
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub page_size: Option<usize>,
    #[serde(default)]
    pub max_commits: Option<usize>,
    #[serde(default)]
    pub resync_on_tab_change: Option<bool>,
    #[serde(default)]
    pub editor: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct FzfConfig {
    #[serde(default = "default_fzf_max_depth")]
    pub max_depth: usize,
    #[serde(default = "default_fzf_excludes")]
    pub excludes: Vec<String>,
    #[serde(default = "default_fzf_start_dir")]
    pub start_dir: String,
    #[serde(default = "default_fzf_git_only")]
    pub git_only: bool,
    #[serde(default = "default_fzf_enabled")]
    pub enabled: bool,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            items: vec![],
            poll_interval_ms: default_poll_interval_ms(),
            max_commits: default_max_commits(),
            graph_max_commits: default_graph_max_commits(),
            detail_cache_ttl_secs: default_detail_cache_ttl_secs(),
            tab_ttl_secs: default_tab_ttl_secs(),
            page_size: default_page_size(),
            sort_by: default_sort_by(),
            visits: default_visits(),
            labels: std::collections::HashMap::new(),
            repo_configs: std::collections::HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            starred: std::collections::HashSet::new(),
            theme_name: default_theme_name(),
            theme: default_theme(),
            fzf: default_fzf(),
            git_app: default_git_app(),
            compatibility_mode: true,
            resync_on_tab_change: false,
            enable_commit_signatures: false,
            ssh_strict_host_checking: false,
            editor: default_editor(),
            compact_view: false,
            show_grouping: true,
        }
    }
}

fn default_ssh_strict_host_checking() -> bool {
    false
}

fn default_editor() -> String {
    std::env::var("EDITOR").or_else(|_| std::env::var("VISUAL")).unwrap_or_else(|_| {
        if cfg!(target_os = "windows") { "notepad".to_string() } else { "vim".to_string() }
    })
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
fn default_fzf_git_only() -> bool {
    true
}
fn default_fzf_enabled() -> bool {
    !cfg!(target_os = "windows")
}
fn default_compatibility_mode() -> bool {
    true
}
fn default_show_grouping() -> bool {
    true
}
fn default_resync_on_tab_change() -> bool {
    false
}
fn default_enable_commit_signatures() -> bool {
    false
}

fn default_fzf() -> FzfConfig {
    FzfConfig {
        max_depth: default_fzf_max_depth(),
        excludes: default_fzf_excludes(),
        start_dir: default_fzf_start_dir(),
        git_only: default_fzf_git_only(),
        enabled: default_fzf_enabled(),
    }
}

/// Represents the structure of the configuration file.
#[derive(Debug, Deserialize, Serialize, Clone)]
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
    /// Maximum commits visualised in the Graph tab (0 = unlimited; default 1000)
    #[serde(default = "default_graph_max_commits")]
    pub graph_max_commits: usize,
    /// TTL in seconds for the detail view cache (default: 30)
    #[serde(default = "default_detail_cache_ttl_secs")]
    pub detail_cache_ttl_secs: u64,
    /// TTL in seconds for the lazy-loaded tabs (default: 60)
    #[serde(default = "default_tab_ttl_secs")]
    pub tab_ttl_secs: u64,
    /// Number of lines/items to scroll when PageUp or PageDown is pressed. Default is 10.
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    /// Sort mode for the main page.
    #[serde(default = "default_sort_by")]
    pub sort_by: SortOrder,
    /// Map of repository items to their last visit time.
    #[serde(default = "default_visits")]
    pub visits: std::collections::HashMap<String, u64>,
    /// Map of repository paths to their labels.
    #[serde(default)]
    pub labels: std::collections::HashMap<String, Vec<String>>,
    /// Repository specific configurations.
    #[serde(default)]
    pub repo_configs: std::collections::HashMap<String, RepoConfig>,
    /// Whether sorting should be reversed.
    #[serde(default)]
    pub sort_reverse: bool,
    /// List of pinned repository paths.
    #[serde(default)]
    pub pinned: std::collections::HashSet<String>,
    /// List of starred repository paths.
    #[serde(default)]
    pub starred: std::collections::HashSet<String>,
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
    /// Enable compatibility mode to use ASCII/simple symbols instead of complex Unicode.
    #[serde(default = "default_compatibility_mode")]
    pub compatibility_mode: bool,
    /// Whether to resync the repository details from disk on tab change.
    #[serde(default = "default_resync_on_tab_change")]
    pub resync_on_tab_change: bool,
    /// Whether to enable commit GPG/SSH signatures collection (spawns a git shell process).
    #[serde(default = "default_enable_commit_signatures")]
    pub enable_commit_signatures: bool,
    /// Whether to enforce strict SSH host key checking (StrictHostKeyChecking=yes)
    #[serde(default = "default_ssh_strict_host_checking")]
    pub ssh_strict_host_checking: bool,
    /// Custom terminal editor to open files with.
    #[serde(default = "default_editor")]
    pub editor: String,
    /// Whether to show the compact (1-row) list on the home page.
    #[serde(default)]
    pub compact_view: bool,
    /// Whether to enable repository grouping on the home page.
    #[serde(default = "default_show_grouping")]
    pub show_grouping: bool,
}

impl Config {
    pub fn sym(&self, key: &str) -> &'static str {
        if self.compatibility_mode {
            match key {
                "branch" => "* ",
                "git_repo" => "G  ",
                "arrow_down" => "v",
                "arrow_right" => ">",
                "folder_tree_expanded" => "v ",
                "folder_tree_collapsed" => "> ",
                "file_tree" => "  -  ",
                "folder" => "[D]",
                "file" => "[F]",
                "pinned" => "[P]",
                "action" => "[!]",
                "warning" => "! ",
                "close" => "x",
                "bullet_empty" => "o",
                "bullet_filled" => "*",
                "star" => "*",
                "block" => "#",
                "bar" => "|",
                "esc" => "ESC",
                "backspace" => "Backspace",
                "tab" => "Tab",
                "shift" => "Shift",
                "enter" => "Enter",
                "up" => "^",
                "down" => "v",
                "page_up" => "PgUp",
                "page_down" => "PgDn",
                "transfer" => "<->",
                "up_down" => "^/v",
                "selection_mark" => "> ",
                _ => "",
            }
        } else {
            match key {
                "branch" => " ",
                "git_repo" => "⎇  ",
                "arrow_down" => "▼",
                "arrow_right" => "▶",
                "folder_tree_expanded" => "▼ ",
                "folder_tree_collapsed" => "> ",
                "file_tree" => "  📄 ",
                "folder" => "📁 ",
                "file" => "📄 ",
                "pinned" => "📌 ",
                "action" => "⚡ ",
                "warning" => "⚠ ",
                "close" => "✕",
                "bullet_empty" => "○",
                "bullet_filled" => "●",
                "star" => "★",
                "block" => "█",
                "bar" => "▍",
                "esc" => "⎋",
                "backspace" => "⌫",
                "tab" => "⇥",
                "shift" => "⇧",
                "enter" => "↵",
                "up" => "↑",
                "down" => "↓",
                "page_up" => "⇞",
                "page_down" => "⇟",
                "transfer" => "⇆",
                "up_down" => "↑↓",
                "selection_mark" => "▌ ",
                _ => "",
            }
        }
    }
}

/// Returns `~/.gitwig/`, the canonical Gitwig data directory.
/// Falls back to `./.gitwig/` in the unlikely event that the home directory
/// cannot be resolved (e.g. inside a stripped-down container).
fn home_gitwig_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".gitwig")
}

fn handle_parse_error(path: &Path, _error: Box<dyn Error>) -> (Config, Option<String>) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let corrupt_path = path.with_extension(format!("toml.corrupt-{}", ts));
    let rename_result = fs::rename(path, &corrupt_path);

    let fallback = Config {
        items: vec![],
        poll_interval_ms: default_poll_interval_ms(),
        max_commits: default_max_commits(),
        graph_max_commits: default_graph_max_commits(),
        detail_cache_ttl_secs: default_detail_cache_ttl_secs(),
        tab_ttl_secs: default_tab_ttl_secs(),
        page_size: default_page_size(),
        sort_by: default_sort_by(),
        visits: default_visits(),
        labels: std::collections::HashMap::new(),
        repo_configs: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        starred: std::collections::HashSet::new(),
        theme_name: default_theme_name(),
        theme: default_theme(),
        fzf: default_fzf(),
        git_app: default_git_app(),
        compatibility_mode: true,
        resync_on_tab_change: false,
        enable_commit_signatures: false,
        ssh_strict_host_checking: false,
        editor: default_editor(),
        compact_view: false,
        show_grouping: true,
    };

    // Attempt to save the fallback back to the original path.
    let _ = save_config(&fallback, path);

    let msg = match rename_result {
        Ok(_) => format!(
            "Config corrupt! Moved to {} and reset to defaults",
            corrupt_path.file_name().unwrap_or_default().to_string_lossy()
        ),
        Err(e) => format!("Config corrupt! Reset to defaults (failed to rename: {})", e),
    };
    (fallback, Some(msg))
}

fn load_and_parse_config(path: &Path) -> (Config, Option<String>) {
    match fs::read_to_string(path) {
        Ok(contents) => match toml::from_str::<Config>(&contents) {
            Ok(mut config) => {
                let allowed = ["git", "gitui", "lazygit"];
                if !allowed.contains(&config.git_app.as_str()) {
                    let old_val = config.git_app.clone();
                    config.git_app = "gitui".to_string();
                    (
                        config,
                        Some(format!(
                            "Invalid preferred Git client '{}' reset to 'gitui'",
                            old_val
                        )),
                    )
                } else {
                    (config, None)
                }
            }
            Err(err) => handle_parse_error(path, err.into()),
        },
        Err(err) => handle_parse_error(path, err.into()),
    }
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
/// `Ok((Config, PathBuf, Option<String>))` — the parsed config, its write-back path, and an optional recovery warning.
pub fn load_config(
    cli_path: Option<PathBuf>,
) -> Result<(Config, PathBuf, Option<String>), Box<dyn Error>> {
    // ── 1. CLI override ───────────────────────────────────────────────────
    if let Some(path) = cli_path {
        if path.exists() {
            let (mut config, warning) = load_and_parse_config(&path);

            let themes_dir = path.parent().unwrap_or(&path).join("themes");
            let _ = fs::create_dir_all(&themes_dir);
            let _ = write_popular_themes(&themes_dir);

            let theme_path = themes_dir.join(format!("{}.theme", config.theme_name));
            if theme_path.exists() {
                if let Ok(theme_contents) = fs::read_to_string(&theme_path) {
                    if let Ok(theme) = toml::from_str::<ThemeConfig>(&theme_contents) {
                        config.theme = theme;
                    } else {
                        config.theme = default_theme();
                    }
                } else {
                    config.theme = default_theme();
                }
            } else {
                let legacy_theme_path = path.with_file_name("theme.toml");
                if legacy_theme_path.exists() {
                    let _ = fs::copy(&legacy_theme_path, themes_dir.join("default.theme"));
                    let _ = fs::remove_file(&legacy_theme_path);
                }

                if let Ok(theme_serialized) = toml::to_string_pretty(&config.theme) {
                    let _ = fs::write(&theme_path, theme_serialized);
                }
            }

            return Ok((config, path, warning));
        }
        let fallback_theme = default_theme();
        let fallback_theme_name = default_theme_name();

        let themes_dir = path.parent().unwrap_or(&path).join("themes");
        let _ = fs::create_dir_all(&themes_dir);
        let _ = write_popular_themes(&themes_dir);
        let theme_path = themes_dir.join(format!("{}.theme", fallback_theme_name));
        if let Ok(theme_serialized) = toml::to_string_pretty(&fallback_theme) {
            let _ = fs::write(&theme_path, theme_serialized);
        }

        return Ok((
            Config {
                items: vec![],
                poll_interval_ms: default_poll_interval_ms(),
                max_commits: default_max_commits(),
                graph_max_commits: default_graph_max_commits(),
                detail_cache_ttl_secs: default_detail_cache_ttl_secs(),
                tab_ttl_secs: default_tab_ttl_secs(),
                page_size: default_page_size(),
                sort_by: default_sort_by(),
                visits: default_visits(),
                labels: std::collections::HashMap::new(),
                repo_configs: std::collections::HashMap::new(),
                sort_reverse: false,
                pinned: std::collections::HashSet::new(),
                starred: std::collections::HashSet::new(),
                theme_name: fallback_theme_name,
                theme: fallback_theme,
                fzf: default_fzf(),
                git_app: default_git_app(),
                compatibility_mode: true,
                resync_on_tab_change: false,
                enable_commit_signatures: false,
                ssh_strict_host_checking: false,
                editor: default_editor(),
                compact_view: false,
                show_grouping: true,
            },
            path,
            None,
        ));
    }

    // ── Always ensure ~/.gitwig/ exists ───────────────────────────────────
    let gitwig_dir = home_gitwig_dir();
    let _ = fs::create_dir_all(&gitwig_dir);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&gitwig_dir) {
            let mut perms = meta.permissions();
            perms.set_mode(0o700);
            let _ = fs::set_permissions(&gitwig_dir, perms);
        }
    }
    let canonical = gitwig_dir.join("config.toml");
    let themes_dir = gitwig_dir.join("themes");
    let _ = fs::create_dir_all(&themes_dir);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&themes_dir) {
            let mut perms = meta.permissions();
            perms.set_mode(0o700);
            let _ = fs::set_permissions(&themes_dir, perms);
        }
    }
    let _ = write_popular_themes(&themes_dir);

    // ── 2. Canonical file already present ─────────────────────────────────
    if canonical.exists() {
        let (mut config, warning) = load_and_parse_config(&canonical);

        let theme_path = themes_dir.join(format!("{}.theme", config.theme_name));
        if theme_path.exists() {
            if let Ok(theme_contents) = fs::read_to_string(&theme_path) {
                if let Ok(theme) = toml::from_str::<ThemeConfig>(&theme_contents) {
                    config.theme = theme;
                } else {
                    config.theme = default_theme();
                }
            } else {
                config.theme = default_theme();
            }
        } else {
            let legacy_theme_path = gitwig_dir.join("theme.toml");
            if legacy_theme_path.exists() {
                let _ = fs::copy(&legacy_theme_path, themes_dir.join("default.theme"));
                let _ = fs::remove_file(&legacy_theme_path);
            }

            if let Ok(theme_serialized) = toml::to_string_pretty(&config.theme) {
                let _ = fs::write(&theme_path, theme_serialized);
            }
        }

        return Ok((config, canonical, warning));
    }

    // ── 3. First run: migrate an existing config into ~/.gitwig/ ──────────
    if let Some(source) = find_legacy_config() {
        fs::copy(&source, &canonical)?;
        let (mut config, warning) = load_and_parse_config(&canonical);

        let theme_path = themes_dir.join(format!("{}.theme", config.theme_name));
        if theme_path.exists() {
            if let Ok(theme_contents) = fs::read_to_string(&theme_path) {
                if let Ok(theme) = toml::from_str::<ThemeConfig>(&theme_contents) {
                    config.theme = theme;
                } else {
                    config.theme = default_theme();
                }
            } else {
                config.theme = default_theme();
            }
        } else {
            let legacy_theme_path = gitwig_dir.join("theme.toml");
            if legacy_theme_path.exists() {
                let _ = fs::copy(&legacy_theme_path, themes_dir.join("default.theme"));
                let _ = fs::remove_file(&legacy_theme_path);
            }

            if let Ok(theme_serialized) = toml::to_string_pretty(&config.theme) {
                let _ = fs::write(&theme_path, theme_serialized);
            }
        }

        return Ok((config, canonical, warning));
    }

    // ── 4. No config anywhere: write a default and use it ─────────────────
    let fallback = Config {
        items: vec![
            "Nice job. You forgot the config, genius.".to_string(),
            "Still looking... it's not here either.".to_string(),
        ],
        poll_interval_ms: default_poll_interval_ms(),
        max_commits: default_max_commits(),
        graph_max_commits: default_graph_max_commits(),
        detail_cache_ttl_secs: default_detail_cache_ttl_secs(),
        tab_ttl_secs: default_tab_ttl_secs(),
        page_size: default_page_size(),
        sort_by: default_sort_by(),
        visits: default_visits(),
        labels: std::collections::HashMap::new(),
        repo_configs: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        starred: std::collections::HashSet::new(),
        theme_name: default_theme_name(),
        theme: default_theme(),
        fzf: default_fzf(),
        git_app: default_git_app(),
        compatibility_mode: true,
        resync_on_tab_change: false,
        enable_commit_signatures: false,
        ssh_strict_host_checking: false,
        editor: default_editor(),
        compact_view: false,
        show_grouping: true,
    };
    save_config(&fallback, &canonical)?;

    let theme_path = themes_dir.join(format!("{}.theme", fallback.theme_name));
    if let Ok(theme_serialized) = toml::to_string_pretty(&fallback.theme) {
        let _ = fs::write(&theme_path, theme_serialized);
    }

    Ok((fallback, canonical, None))
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
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = fs::metadata(parent) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o700);
                    let _ = fs::set_permissions(parent, perms);
                }
            }
        }
    }
    let serialized = toml::to_string_pretty(config)?;

    // Write atomically: write to a .tmp file first, then rename.
    let tmp_path = path.with_extension("toml.tmp");
    if let Err(e) = fs::write(&tmp_path, serialized) {
        let _ = fs::remove_file(&tmp_path);
        return Err(e.into());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&tmp_path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o600);
            let _ = fs::set_permissions(&tmp_path, perms);
        }
    }

    if let Err(e) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(e.into());
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn get_unique_id() -> String {
        let count = TEST_DIR_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let nanos =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        format!("{}_{}_{}", pid, nanos, count)
    }

    #[test]
    fn test_theme_separation_load_and_save() {
        let unique_id = get_unique_id();
        let test_dir = std::env::temp_dir().join(format!("gitwig_test_theme_{}", unique_id));
        fs::create_dir_all(&test_dir).unwrap();
        let config_path = test_dir.join("config.toml");
        let themes_dir = test_dir.join("themes");
        let theme_path = themes_dir.join("default.theme");

        // 1. Initial load when files do not exist (should write themes/default.theme but not config.toml in CLI mode)
        let (config, path, _) = load_config(Some(config_path.clone())).unwrap();
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
        let (loaded_config, _, _) = load_config(Some(config_path.clone())).unwrap();
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
        let unique_id = get_unique_id();
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

    #[test]
    fn test_corrupt_config_recovery() {
        let unique_id = get_unique_id();
        let test_dir = std::env::temp_dir().join(format!("gitwig_test_corrupt_{}", unique_id));
        fs::create_dir_all(&test_dir).unwrap();
        let config_path = test_dir.join("config.toml");

        // Write a corrupt/invalid TOML file
        fs::write(&config_path, "items = [").unwrap();

        // Load config (should move to .corrupt-<ts>, create default config and return warning)
        let (config, path, warning) = load_config(Some(config_path.clone())).unwrap();
        assert_eq!(path, config_path);
        assert!(config_path.exists());
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("Config corrupt!"));

        // Verify loaded config is default
        assert!(config.items.is_empty());

        // Verify that a corrupt backup file was created in the same directory
        let files = fs::read_dir(&test_dir).unwrap();
        let corrupt_exists = files
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().contains("config.toml.corrupt-"));
        assert!(corrupt_exists);

        let _ = fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_git_app_validation_recovery() {
        let unique_id = get_unique_id();
        let test_dir = std::env::temp_dir().join(format!("gitwig_test_git_app_{}", unique_id));
        fs::create_dir_all(&test_dir).unwrap();
        let config_path = test_dir.join("config.toml");

        let config_toml = r#"
items = []
poll_interval_ms = 1000
max_commits = 100
graph_max_commits = 100
detail_cache_ttl_secs = 60
tab_ttl_secs = 60
page_size = 50
sort_by = "alphabetical"
visits = {}
labels = {}
repo_configs = {}
sort_reverse = false
pinned = []
theme_name = "default"
fzf = { excludes = [], max_depth = 5, start_dir = "~" }
git_app = "malicious_binary"
compatibility_mode = true
resync_on_tab_change = false
enable_commit_signatures = false
"#;
        fs::write(&config_path, config_toml).unwrap();

        let (config, path, warning) = load_config(Some(config_path.clone())).unwrap();
        assert_eq!(path, config_path);
        assert_eq!(config.git_app, "gitui");
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("Invalid preferred Git client"));

        let _ = fs::remove_dir_all(&test_dir);
    }
}
