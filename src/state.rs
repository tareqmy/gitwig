//! Usage state that changes as a side effect of using the app: last-visit
//! times, per-repository commit-message history, the quick-label slots and
//! the sticky label filter.
//!
//! It lives in `state.toml` beside `config.toml` so that merely opening a
//! repository or committing never rewrites the file that holds hand-edited
//! settings. Older versions kept these keys in `config.toml`; `take_legacy`
//! lifts them out on load and the next save writes a clean config.

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// File name of the state file, always a sibling of `config.toml`.
pub const STATE_FILE_NAME: &str = "state.toml";

/// How many recent commit messages are remembered per repository.
pub const COMMIT_HISTORY_LIMIT: usize = 10;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppState {
    /// Repository path → last time it was opened (seconds since the epoch).
    /// Feeds the `recent_visit` sort and the Recent group.
    #[serde(default)]
    pub visits: HashMap<String, u64>,

    /// Sticky home-list label filter ("project view"). Survives restarts
    /// until deselected or until the label disappears from every repository.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_label_filter: Option<String>,

    /// Quick-label slots behind the `1`-`9` keys, in slot order (FIFO on
    /// first view).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub label_slots: Vec<String>,

    /// Resolved repository path → most recent commit messages, newest first.
    #[serde(default)]
    pub commit_history: HashMap<String, Vec<String>>,
}

impl AppState {
    /// `state.toml` next to `config_path`.
    pub fn path_for(config_path: &Path) -> PathBuf {
        config_path.with_file_name(STATE_FILE_NAME)
    }

    /// Whether nothing at all is recorded.
    pub fn is_empty(&self) -> bool {
        self.visits.is_empty()
            && self.active_label_filter.is_none()
            && self.label_slots.is_empty()
            && self.commit_history.is_empty()
    }

    /// Lifts the state keys older versions stored in `config.toml` out of a
    /// parsed config table: top-level `visits`, `active_label_filter` and
    /// `label_slots`, plus `commit_history` under each `[repo_configs.*]`.
    /// The keys are removed from `table` so a later save writes a clean
    /// config. Returns `None` when none of them were present.
    pub fn take_legacy(table: &mut toml::Table) -> Option<AppState> {
        let mut found = false;
        let mut state = AppState::default();

        if let Some(value) = table.remove("visits") {
            found = true;
            state.visits = value.try_into().unwrap_or_default();
        }
        if let Some(value) = table.remove("active_label_filter") {
            found = true;
            state.active_label_filter = value.as_str().map(str::to_string);
        }
        if let Some(value) = table.remove("label_slots") {
            found = true;
            state.label_slots = value.try_into().unwrap_or_default();
        }
        if let Some(toml::Value::Table(repos)) = table.get_mut("repo_configs") {
            for (repo, entry) in repos.iter_mut() {
                let Some(entry) = entry.as_table_mut() else { continue };
                if let Some(value) = entry.remove("commit_history") {
                    found = true;
                    let history: Vec<String> = value.try_into().unwrap_or_default();
                    if !history.is_empty() {
                        state.commit_history.insert(repo.clone(), history);
                    }
                }
            }
        }

        found.then_some(state)
    }

    /// Merges values recovered from a legacy `config.toml` without overriding
    /// anything `state.toml` already records.
    pub fn absorb(&mut self, legacy: AppState) {
        for (repo, time) in legacy.visits {
            self.visits.entry(repo).or_insert(time);
        }
        if self.active_label_filter.is_none() {
            self.active_label_filter = legacy.active_label_filter;
        }
        if self.label_slots.is_empty() {
            self.label_slots = legacy.label_slots;
        }
        for (repo, history) in legacy.commit_history {
            self.commit_history.entry(repo).or_insert(history);
        }
    }

    /// Records `msg` as the most recent commit message for `repo`, de-duplicating
    /// an identical earlier entry and keeping at most `COMMIT_HISTORY_LIMIT`.
    pub fn record_commit_message(&mut self, repo: &str, msg: &str) {
        let history = self.commit_history.entry(repo.to_string()).or_default();
        history.retain(|x| x != msg);
        history.insert(0, msg.to_string());
        history.truncate(COMMIT_HISTORY_LIMIT);
    }

    /// Recent commit messages for `repo`, newest first.
    pub fn commit_history_for(&self, repo: &str) -> Vec<String> {
        self.commit_history.get(repo).cloned().unwrap_or_default()
    }
}

/// Loads `state.toml`. A missing file yields an empty state; an unreadable or
/// unparsable one is set aside as `state.toml.corrupt-<ts>` and reset, and the
/// warning says so. Nothing here should ever stop the app from starting.
pub fn load_state(path: &Path) -> (AppState, Option<String>) {
    if !path.exists() {
        return (AppState::default(), None);
    }
    let parsed = fs::read_to_string(path)
        .map_err(|e| e.to_string())
        .and_then(|contents| toml::from_str::<AppState>(&contents).map_err(|e| e.to_string()));
    match parsed {
        Ok(state) => (state, None),
        Err(_) => {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let corrupt_path = path.with_extension(format!("toml.corrupt-{}", ts));
            let msg = match fs::rename(path, &corrupt_path) {
                Ok(_) => format!(
                    "State file corrupt! Moved to {} and reset",
                    corrupt_path.file_name().unwrap_or_default().to_string_lossy()
                ),
                Err(e) => format!("State file corrupt! Reset (failed to rename: {})", e),
            };
            (AppState::default(), Some(msg))
        }
    }
}

/// Serializes the state to TOML and writes it atomically to `path`.
pub fn save_state(state: &AppState, path: &Path) -> Result<(), Box<dyn Error>> {
    let serialized = toml::to_string_pretty(state)?;
    crate::config::write_toml_atomic(path, &serialized)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn temp_state_path(tag: &str) -> PathBuf {
        let nanos =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "gitwig_state_{}_{}_{}",
            tag,
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir).unwrap();
        dir.join(STATE_FILE_NAME)
    }

    #[test]
    fn path_for_is_a_sibling_of_the_config() {
        let config = PathBuf::from("/home/me/.gitwig/config.toml");
        assert_eq!(AppState::path_for(&config), PathBuf::from("/home/me/.gitwig/state.toml"));
    }

    #[test]
    fn take_legacy_lifts_every_state_key_out_of_a_config_table() {
        let mut table: toml::Table = toml::from_str(
            r#"
items = ["/a", "/b"]
active_label_filter = "work"
label_slots = ["work", "oss"]

[visits]
"/a" = 10
"/b" = 20

[repo_configs."/a"]
page_size = 5
commit_history = ["second", "first"]

[repo_configs."/b"]
note = "keep me"
"#,
        )
        .unwrap();

        let legacy = AppState::take_legacy(&mut table).expect("legacy keys present");

        assert_eq!(legacy.visits.get("/a"), Some(&10));
        assert_eq!(legacy.visits.get("/b"), Some(&20));
        assert_eq!(legacy.active_label_filter.as_deref(), Some("work"));
        assert_eq!(legacy.label_slots, vec!["work".to_string(), "oss".to_string()]);
        assert_eq!(
            legacy.commit_history.get("/a"),
            Some(&vec!["second".to_string(), "first".to_string()])
        );
        assert!(!legacy.commit_history.contains_key("/b"));

        // The config table is left clean, with the real settings untouched.
        assert!(table.get("visits").is_none());
        assert!(table.get("active_label_filter").is_none());
        assert!(table.get("label_slots").is_none());
        let repos = table["repo_configs"].as_table().unwrap();
        assert!(repos["/a"].get("commit_history").is_none());
        assert_eq!(repos["/a"]["page_size"].as_integer(), Some(5));
        assert_eq!(repos["/b"]["note"].as_str(), Some("keep me"));
        assert_eq!(table["items"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn take_legacy_is_none_for_a_clean_config() {
        let mut table: toml::Table = toml::from_str(
            r#"
items = ["/a"]
[repo_configs."/a"]
page_size = 5
"#,
        )
        .unwrap();
        assert!(AppState::take_legacy(&mut table).is_none());
        assert!(table.get("repo_configs").is_some());
    }

    #[test]
    fn absorb_never_overrides_what_the_state_file_holds() {
        let mut state = AppState {
            visits: HashMap::from([("/a".to_string(), 99)]),
            active_label_filter: Some("oss".to_string()),
            label_slots: vec!["oss".to_string()],
            commit_history: HashMap::from([("/a".to_string(), vec!["new".to_string()])]),
        };
        let legacy = AppState {
            visits: HashMap::from([("/a".to_string(), 1), ("/b".to_string(), 2)]),
            active_label_filter: Some("work".to_string()),
            label_slots: vec!["work".to_string()],
            commit_history: HashMap::from([
                ("/a".to_string(), vec!["old".to_string()]),
                ("/b".to_string(), vec!["b".to_string()]),
            ]),
        };

        state.absorb(legacy);

        assert_eq!(state.visits.get("/a"), Some(&99));
        assert_eq!(state.visits.get("/b"), Some(&2));
        assert_eq!(state.active_label_filter.as_deref(), Some("oss"));
        assert_eq!(state.label_slots, vec!["oss".to_string()]);
        assert_eq!(state.commit_history["/a"], vec!["new".to_string()]);
        assert_eq!(state.commit_history["/b"], vec!["b".to_string()]);
    }

    #[test]
    fn absorb_fills_gaps_from_legacy() {
        let mut state = AppState::default();
        let legacy = AppState {
            visits: HashMap::from([("/a".to_string(), 1)]),
            active_label_filter: Some("work".to_string()),
            label_slots: vec!["work".to_string()],
            commit_history: HashMap::new(),
        };
        state.absorb(legacy.clone());
        assert_eq!(state, legacy);
    }

    #[test]
    fn record_commit_message_dedups_and_caps_the_history() {
        let mut state = AppState::default();
        for i in 0..(COMMIT_HISTORY_LIMIT + 3) {
            state.record_commit_message("/repo", &format!("msg {}", i));
        }
        let history = state.commit_history_for("/repo");
        assert_eq!(history.len(), COMMIT_HISTORY_LIMIT);
        assert_eq!(history[0], format!("msg {}", COMMIT_HISTORY_LIMIT + 2));

        // Re-committing an existing message moves it to the front without a duplicate.
        state.record_commit_message("/repo", "msg 5");
        let history = state.commit_history_for("/repo");
        assert_eq!(history[0], "msg 5");
        assert_eq!(history.iter().filter(|m| *m == "msg 5").count(), 1);
        assert_eq!(history.len(), COMMIT_HISTORY_LIMIT);

        assert!(state.commit_history_for("/other").is_empty());
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = temp_state_path("roundtrip");
        let mut state = AppState::default();
        state.visits.insert("/a".to_string(), 42);
        state.active_label_filter = Some("work".to_string());
        state.label_slots = vec!["work".to_string(), "oss".to_string()];
        state.record_commit_message("/a", "feat: thing");

        save_state(&state, &path).unwrap();
        let (loaded, warning) = load_state(&path);
        assert!(warning.is_none());
        assert_eq!(loaded, state);

        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn missing_state_file_is_empty_without_a_warning() {
        let path = temp_state_path("missing");
        let (loaded, warning) = load_state(&path);
        assert!(loaded.is_empty());
        assert!(warning.is_none());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn corrupt_state_file_is_set_aside_and_reset() {
        let path = temp_state_path("corrupt");
        fs::write(&path, "this is = not [ toml").unwrap();
        let (loaded, warning) = load_state(&path);
        assert!(loaded.is_empty());
        assert!(warning.unwrap().contains("corrupt"));
        assert!(!path.exists());
        let set_aside = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().starts_with("state.toml.corrupt-"));
        assert!(set_aside);
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn empty_optional_keys_are_not_written() {
        let path = temp_state_path("sparse");
        let mut state = AppState::default();
        state.visits.insert("/a".to_string(), 1);
        save_state(&state, &path).unwrap();
        let written = fs::read_to_string(&path).unwrap();
        assert!(!written.contains("active_label_filter"));
        assert!(!written.contains("label_slots"));
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }
}
