//! Richer per-item inspection used by the Detail view.
//!
//! `status.rs` answers the cheap "is this a git repo" question for the
//! list-row indicator. This module is invoked on-demand (when the user
//! presses Enter on an item) and uses `git2` to read branch / HEAD /
//! remotes / working-tree status. The two split so the cheap path
//! doesn't pay for libgit2 just to render a row.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use git2::{Repository, StatusOptions, StatusShow};

use crate::status::expand_tilde;

#[derive(Debug)]
pub enum ItemDetail {
    /// Path does not exist or isn't a directory.
    Missing { resolved: PathBuf },
    /// Directory exists but isn't a git repository.
    Directory { resolved: PathBuf },
    /// A real git repository, with details collected.
    Repo { resolved: PathBuf, info: RepoInfo },
    /// The directory looked like a repo but `git2` couldn't read it.
    /// Surfaces the error to the user instead of pretending.
    Error { resolved: PathBuf, message: String },
}

#[derive(Debug, Default)]
pub struct RepoInfo {
    /// Branch shorthand (e.g. "main"). `None` for detached HEAD or empty repos.
    pub branch: Option<String>,
    /// HEAD commit summary. `None` for empty repos or read failures.
    pub head: Option<HeadInfo>,
    pub remotes: Vec<RemoteInfo>,
    pub worktree: WorktreeStatus,
}

#[derive(Debug)]
pub struct HeadInfo {
    pub short_id: String,
    pub summary: String,
    pub author: String,
    pub when: String,
}

#[derive(Debug)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Default)]
pub struct WorktreeStatus {
    pub staged: usize,
    pub modified: usize,
    pub untracked: usize,
    pub conflicted: usize,
}

impl WorktreeStatus {
    pub fn is_clean(&self) -> bool {
        self.staged + self.modified + self.untracked + self.conflicted == 0
    }
}

/// Inspect `item` and produce a rich detail report. Uses the same tilde
/// expansion as the cheap classifier so the resolved path matches what
/// the user sees in the list.
pub fn inspect_detail(item: &str) -> ItemDetail {
    let resolved = expand_tilde(item);
    if !resolved.is_dir() {
        return ItemDetail::Missing { resolved };
    }
    if !resolved.join(".git").exists() {
        return ItemDetail::Directory { resolved };
    }
    match collect_repo_info(&resolved) {
        Ok(info) => ItemDetail::Repo { resolved, info },
        Err(e) => ItemDetail::Error {
            resolved,
            message: e.to_string(),
        },
    }
}

fn collect_repo_info(path: &Path) -> Result<RepoInfo, git2::Error> {
    let repo = Repository::open(path)?;
    let mut info = RepoInfo::default();

    if let Ok(head) = repo.head() {
        // git2 0.21: `shorthand()` returns `Result<&str, Error>` and
        // `summary()` returns `Result<Option<&str>, Error>` — outer = read
        // success, inner = UTF-8 validity. Collapse both to plain Option.
        info.branch = head.shorthand().ok().map(String::from);
        if let Ok(commit) = head.peel_to_commit() {
            let short_id = format!("{:.7}", commit.id());
            let summary = commit
                .summary()
                .ok()
                .flatten()
                .unwrap_or("(no commit message)")
                .to_string();
            let author = commit.author();
            let author_str = format!(
                "{} <{}>",
                author.name().unwrap_or("?"),
                author.email().unwrap_or("?")
            );
            let when = format_relative_time(commit.time().seconds());
            info.head = Some(HeadInfo {
                short_id,
                summary,
                author: author_str,
                when,
            });
        }
    }

    if let Ok(remotes) = repo.remotes() {
        for name in remotes.iter() {
            // `name` is Option<&str> — None means non-UTF-8 remote name,
            // which we can't address by name through libgit2's safe API.
            // Iter yields Result<Option<&str>, git2::Error>: skip both
            // libgit2 errors and non-UTF-8 remote names.
            let Ok(Some(name)) = name else { continue };
            if let Ok(remote) = repo.find_remote(name) {
                info.remotes.push(RemoteInfo {
                    name: name.to_string(),
                    url: remote.url().unwrap_or("(no url)").to_string(),
                });
            }
        }
    }

    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .renames_head_to_index(true)
        .show(StatusShow::IndexAndWorkdir);
    if let Ok(statuses) = repo.statuses(Some(&mut opts)) {
        for entry in statuses.iter() {
            let flags = entry.status();
            if flags.is_conflicted() {
                info.worktree.conflicted += 1;
                continue;
            }
            if flags.is_wt_new() {
                info.worktree.untracked += 1;
            }
            if flags.is_wt_modified()
                || flags.is_wt_deleted()
                || flags.is_wt_renamed()
                || flags.is_wt_typechange()
            {
                info.worktree.modified += 1;
            }
            if flags.is_index_new()
                || flags.is_index_modified()
                || flags.is_index_deleted()
                || flags.is_index_renamed()
                || flags.is_index_typechange()
            {
                info.worktree.staged += 1;
            }
        }
    }

    Ok(info)
}

/// Format a unix-epoch timestamp as a relative time string ("3 days ago").
/// Avoids the `chrono` dependency — exact dates aren't needed for the
/// detail view, just a rough recency indicator.
fn format_relative_time(secs: i64) -> String {
    if secs <= 0 {
        return "unknown".to_string();
    }
    let then = UNIX_EPOCH + Duration::from_secs(secs as u64);
    let now = SystemTime::now();
    let Ok(elapsed) = now.duration_since(then) else {
        return "in the future".to_string();
    };
    let secs = elapsed.as_secs();
    let (n, unit) = if secs < 60 {
        (secs, "second")
    } else if secs < 3600 {
        (secs / 60, "minute")
    } else if secs < 86_400 {
        (secs / 3600, "hour")
    } else if secs < 86_400 * 30 {
        (secs / 86_400, "day")
    } else if secs < 86_400 * 365 {
        (secs / (86_400 * 30), "month")
    } else {
        (secs / (86_400 * 365), "year")
    };
    let plural = if n == 1 { "" } else { "s" };
    format!("{} {}{} ago", n, unit, plural)
}
