//! Filesystem + git repository inspection.
//!
//! This module owns both the "is this a git repo?" classification used by
//! the per-card indicator AND the richer detail collection used by the
//! Detail view. They share a single `collect_summary` helper so the same
//! libgit2 work doesn't run twice.
//!
//! The cheap `is_dir()` + `.git`-existence check still gates everything —
//! we only spin up libgit2 when both checks pass.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use git2::{Repository, StatusOptions, StatusShow};

// ── Card-level status ──────────────────────────────────────────────────────

/// Per-item filesystem classification carried alongside `config.items`.
/// `GitRepo`'s inner `Option` is `None` when `.git` exists but libgit2
/// couldn't open or read the repo — we know it's a repo, we just can't
/// summarize its state.
#[derive(Debug, Clone)]
pub enum ItemStatus {
    Missing,
    Directory,
    GitRepo(Option<RepoSummary>),
}

/// Compact summary used to draw the per-card indicator. Also embedded in
/// `RepoInfo` so the Detail view doesn't re-collect the same data.
#[derive(Debug, Default, Clone)]
pub struct RepoSummary {
    /// Current branch shorthand (e.g. `"main"`). `None` for detached HEAD
    /// or when the ref cannot be read.
    pub branch: Option<String>,
    pub staged: usize,
    pub modified: usize,
    pub untracked: usize,
    pub conflicted: usize,
    pub ahead: usize,
    pub behind: usize,
}

impl RepoSummary {
    pub fn is_clean(&self) -> bool {
        self.staged + self.modified + self.untracked + self.conflicted == 0
    }
    pub fn is_synced(&self) -> bool {
        self.ahead + self.behind == 0
    }
    pub fn unchanged(&self) -> bool {
        self.is_clean() && self.is_synced()
    }
}

// ── Detail view ────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum ItemDetail {
    Missing {
        resolved: PathBuf,
    },
    Directory {
        resolved: PathBuf,
    },
    Repo {
        resolved: PathBuf,
        info: Box<RepoInfo>,
    },
    Error {
        resolved: PathBuf,
        message: String,
    },
}

#[derive(Debug, Default)]
pub struct RepoInfo {
    pub branch: Option<String>,
    pub head: Option<HeadInfo>,
    pub remotes: Vec<RemoteInfo>,
    /// Configured upstream branch (e.g. "origin/main") if HEAD tracks one.
    pub upstream: Option<String>,
    pub summary: RepoSummary,
    /// File-level changes, populated by `collect_info` for the Detail view.
    pub changes: WorktreeChanges,
    /// Recent commits in this repository.
    pub commits: Vec<CommitEntry>,
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

#[derive(Debug, Clone)]
pub struct CommitEntry {
    pub id: String,
    pub author: String,
    pub when: String,
    pub summary: String,
}

/// One changed file in the working tree or index.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Path relative to the repository root.
    pub path: String,
    /// Short human-readable label: "new", "modified", "deleted",
    /// "renamed", "typechange", "??", or "conflict".
    pub label: &'static str,
}

/// File-level working-tree state collected for the Detail view.
/// Split into four buckets so the UI can render them as separate sections.
#[derive(Debug, Default, Clone)]
pub struct WorktreeChanges {
    pub staged: Vec<FileEntry>,
    pub unstaged: Vec<FileEntry>,
    pub untracked: Vec<FileEntry>,
    pub conflicted: Vec<FileEntry>,
}

// ── Public entry points ────────────────────────────────────────────────────

/// Expand a leading `~` or `~/` in a user-supplied path to the user's home
/// directory. Returns the input unchanged if there is no home dir or no
/// tilde to expand.
pub fn expand_tilde(s: &str) -> PathBuf {
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

/// Classify `item` and produce a card-level summary. Used by the list view.
pub fn inspect_summary(item: &str) -> ItemStatus {
    let path = expand_tilde(item);
    if !path.is_dir() {
        return ItemStatus::Missing;
    }
    if !path.join(".git").exists() {
        return ItemStatus::Directory;
    }
    match Repository::open(&path) {
        Ok(repo) => ItemStatus::GitRepo(Some(collect_summary(&repo))),
        Err(_) => ItemStatus::GitRepo(None),
    }
}

/// Inspect `item` and produce the rich detail report shown on Enter.
pub fn inspect_detail(item: &str) -> ItemDetail {
    let resolved = expand_tilde(item);
    if !resolved.is_dir() {
        return ItemDetail::Missing { resolved };
    }
    if !resolved.join(".git").exists() {
        return ItemDetail::Directory { resolved };
    }
    match collect_info(&resolved) {
        Ok(info) => ItemDetail::Repo {
            resolved,
            info: Box::new(info),
        },
        Err(e) => ItemDetail::Error {
            resolved,
            message: e.to_string(),
        },
    }
}

fn collect_commits(repo: &Repository, limit: usize) -> Result<Vec<CommitEntry>, git2::Error> {
    let mut walk = repo.revwalk()?;
    if walk.push_head().is_err() {
        return Ok(Vec::new());
    }
    walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;

    let mut commits = Vec::new();
    for id in walk.take(limit) {
        let oid = id?;
        if let Ok(commit) = repo.find_commit(oid) {
            let short_id = format!("{:.7}", commit.id());
            let summary = commit
                .summary()
                .ok()
                .flatten()
                .unwrap_or("(no commit message)")
                .to_string();
            let author = commit.author();
            let author_str = author.name().unwrap_or("?").to_string();
            let when = format_relative_time(commit.time().seconds());
            commits.push(CommitEntry {
                id: short_id,
                author: author_str,
                when,
                summary,
            });
        }
    }
    Ok(commits)
}

// ── Internal collection ────────────────────────────────────────────────────

fn collect_info(path: &Path) -> Result<RepoInfo, git2::Error> {
    let repo = Repository::open(path)?;
    let summary = collect_summary(&repo);
    let mut info = RepoInfo {
        summary,
        ..RepoInfo::default()
    };

    if let Ok(head) = repo.head() {
        // git2 0.21: shorthand() returns Result<&str, Error>.
        info.branch = head.shorthand().ok().map(String::from);

        if let Ok(commit) = head.peel_to_commit() {
            let short_id = format!("{:.7}", commit.id());
            // summary() returns Result<Option<&str>, Error>.
            let summary_text = commit
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
                summary: summary_text,
                author: author_str,
                when,
            });
        }

        // Upstream branch (short form, "origin/main"). git2 0.21:
        // Reference::name returns Result<&str, Error>.
        if let Ok(head_name) = head.name() {
            info.upstream = upstream_short_name(&repo, head_name);
        }
    }

    if let Ok(remotes) = repo.remotes() {
        for name in remotes.iter() {
            let Ok(Some(name)) = name else { continue };
            if let Ok(remote) = repo.find_remote(name) {
                info.remotes.push(RemoteInfo {
                    name: name.to_string(),
                    url: remote.url().unwrap_or("(no url)").to_string(),
                });
            }
        }
    }

    if let Ok(commits) = collect_commits(&repo, 50) {
        info.commits = commits;
    }

    populate_file_changes(&repo, &mut info);
    Ok(info)
}

/// Maximum file entries collected per bucket. Prevents pathologically large
/// working trees from overwhelming the detail view.
const MAX_FILES_PER_SECTION: usize = 100;

/// Walk the working-tree status once more and collect per-file info for
/// the Detail view. Called only from `collect_info` (i.e. once per Enter
/// press), never per frame.
fn populate_file_changes(repo: &Repository, info: &mut RepoInfo) {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .renames_head_to_index(true)
        .show(StatusShow::IndexAndWorkdir);
    let Ok(statuses) = repo.statuses(Some(&mut opts)) else {
        return;
    };
    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("(unknown)").to_string();
        let flags = entry.status();

        if flags.is_conflicted() {
            if info.changes.conflicted.len() < MAX_FILES_PER_SECTION {
                info.changes.conflicted.push(FileEntry {
                    path: path.clone(),
                    label: "conflict",
                });
            }
            continue;
        }

        // Index (staged) changes
        if (flags.is_index_new()
            || flags.is_index_modified()
            || flags.is_index_deleted()
            || flags.is_index_renamed()
            || flags.is_index_typechange())
            && info.changes.staged.len() < MAX_FILES_PER_SECTION
        {
            let label = if flags.is_index_new() {
                "new"
            } else if flags.is_index_deleted() {
                "deleted"
            } else if flags.is_index_renamed() {
                "renamed"
            } else if flags.is_index_typechange() {
                "typechange"
            } else {
                "modified"
            };
            info.changes.staged.push(FileEntry {
                path: path.clone(),
                label,
            });
        }

        // Working-tree changes
        if flags.is_wt_new() {
            if info.changes.untracked.len() < MAX_FILES_PER_SECTION {
                info.changes.untracked.push(FileEntry {
                    path: path.clone(),
                    label: "??",
                });
            }
        } else if (flags.is_wt_modified()
            || flags.is_wt_deleted()
            || flags.is_wt_renamed()
            || flags.is_wt_typechange())
            && info.changes.unstaged.len() < MAX_FILES_PER_SECTION
        {
            let label = if flags.is_wt_deleted() {
                "deleted"
            } else if flags.is_wt_renamed() {
                "renamed"
            } else if flags.is_wt_typechange() {
                "typechange"
            } else {
                "modified"
            };
            info.changes.unstaged.push(FileEntry {
                path: path.clone(),
                label,
            });
        }
    }
}

/// Collect the branch name, worktree counts, and ahead/behind for an opened
/// repo. Used by both `inspect_summary` (card) and `collect_info` (detail)
/// so the values shown in both places always agree.
fn collect_summary(repo: &Repository) -> RepoSummary {
    let mut s = RepoSummary::default();
    // git2 0.21: head() + shorthand() = Result<&str, Error>.
    if let Ok(head) = repo.head() {
        s.branch = head.shorthand().ok().map(String::from);
    }
    populate_worktree(repo, &mut s);
    populate_ahead_behind(repo, &mut s);
    s
}

fn populate_worktree(repo: &Repository, s: &mut RepoSummary) {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .renames_head_to_index(true)
        .show(StatusShow::IndexAndWorkdir);
    let Ok(statuses) = repo.statuses(Some(&mut opts)) else {
        return;
    };
    for entry in statuses.iter() {
        let flags = entry.status();
        if flags.is_conflicted() {
            s.conflicted += 1;
            continue;
        }
        if flags.is_wt_new() {
            s.untracked += 1;
        }
        if flags.is_wt_modified()
            || flags.is_wt_deleted()
            || flags.is_wt_renamed()
            || flags.is_wt_typechange()
        {
            s.modified += 1;
        }
        if flags.is_index_new()
            || flags.is_index_modified()
            || flags.is_index_deleted()
            || flags.is_index_renamed()
            || flags.is_index_typechange()
        {
            s.staged += 1;
        }
    }
}

/// Compute commits ahead/behind the upstream branch. Silently leaves
/// both at 0 if HEAD is detached, the branch has no upstream configured,
/// or any libgit2 lookup fails — the card simply shows no ↑/↓ then.
fn populate_ahead_behind(repo: &Repository, s: &mut RepoSummary) {
    let Ok(head) = repo.head() else { return };
    let Some(local_oid) = head.target() else {
        return;
    };
    let Ok(head_name) = head.name() else { return };
    let Ok(upstream_buf) = repo.branch_upstream_name(head_name) else {
        return;
    };
    let Ok(upstream_name) = std::str::from_utf8(&upstream_buf) else {
        return;
    };
    let Ok(upstream_ref) = repo.find_reference(upstream_name) else {
        return;
    };
    let Some(upstream_oid) = upstream_ref.target() else {
        return;
    };
    if let Ok((ahead, behind)) = repo.graph_ahead_behind(local_oid, upstream_oid) {
        s.ahead = ahead;
        s.behind = behind;
    }
}

/// `"origin/main"`-style short name for HEAD's upstream, or `None`.
fn upstream_short_name(repo: &Repository, head_name: &str) -> Option<String> {
    let buf = repo.branch_upstream_name(head_name).ok()?;
    let raw = std::str::from_utf8(&buf).ok()?;
    Some(raw.strip_prefix("refs/remotes/").unwrap_or(raw).to_string())
}

/// Format a unix-epoch timestamp as a relative time string ("3 days ago").
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
