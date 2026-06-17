//! Filesystem inspection for config items.
//!
//! Each item is interpreted as a path. We classify it as:
//! - `GitRepo` — a directory containing a `.git` entry (dir, file, or symlink).
//! - `Directory` — a directory that isn't a git repo.
//! - `Missing` — anything else (path doesn't exist, isn't a directory, or
//!   isn't accessible).
//!
//! The classifier is intentionally cheap (no `git2`/`gix` dependency) — it
//! just stats the path and looks for `.git`. That covers regular repos,
//! worktrees, and submodules (where `.git` is a file pointing to the real
//! gitdir) without parsing anything.

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemStatus {
    Missing,
    Directory,
    GitRepo,
}

/// Classify `item` after `~` expansion. Treats `~` or `~/...` as relative
/// to the user's home directory if it can be resolved; otherwise the path
/// is used as-is.
pub fn inspect(item: &str) -> ItemStatus {
    let path = expand_tilde(item);
    if !path.is_dir() {
        return ItemStatus::Missing;
    }
    if path.join(".git").exists() {
        ItemStatus::GitRepo
    } else {
        ItemStatus::Directory
    }
}

fn expand_tilde(s: &str) -> PathBuf {
    if s == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from(s));
    }
    if let Some(stripped) = s.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(stripped);
    }
    PathBuf::from(s)
}
