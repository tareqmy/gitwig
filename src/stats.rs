use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct AppStats {
    #[serde(default)]
    pub total_duration_secs: u64,
    #[serde(default)]
    pub commits_made: u64,
    #[serde(default)]
    pub files_modified: u64,
    #[serde(default)]
    pub branches_created: u64,
    #[serde(default)]
    pub branches_deleted: u64,
    #[serde(default)]
    pub merges: u64,
    #[serde(default)]
    pub rebases: u64,
    #[serde(default)]
    pub stashes: u64,
    #[serde(default)]
    pub fetches: u64,
    #[serde(default)]
    pub pushes: u64,
    #[serde(default)]
    pub pulls: u64,
    #[serde(default)]
    pub active_repositories: HashMap<String, u64>,
    #[serde(default)]
    pub daily_activity: HashMap<String, u64>,
    #[serde(default)]
    pub forge_prs_reviewed: u64,
    #[serde(default)]
    pub forge_comments_made: u64,
}

impl AppStats {
    pub fn track_daily_activity(&mut self) {
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        *self.daily_activity.entry(date).or_insert(0) += 1;
    }

    pub fn track_active_repo(&mut self, repo_path: &str) {
        *self.active_repositories.entry(repo_path.to_string()).or_insert(0) += 1;
    }
}

pub fn stats_path() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".gitwig").join("stats.toml")
}

pub fn load_stats() -> AppStats {
    let path = stats_path();
    if path.exists() {
        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(stats) = toml::from_str::<AppStats>(&contents) {
                return stats;
            }
        }
    }
    AppStats::default()
}

pub fn save_stats(stats: &AppStats) {
    let path = stats_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(serialized) = toml::to_string_pretty(stats) {
        let _ = fs::write(&path, serialized);
    }
}
