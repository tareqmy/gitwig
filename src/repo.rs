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
    /// Short 7-char display ID.
    pub id: String,
    /// Full 40-char hex OID — used for diff lookup.
    pub oid: String,
    pub author: String,
    pub when: String,
    pub summary: String,
    /// Local branch names and tags pointing at this commit.
    /// Tags are prefixed with `"tag:"`, remote branches with `"remote:"`.
    pub refs: Vec<String>,
    /// Files changed in this commit (diff against first parent, or empty tree).
    pub files: Vec<FileEntry>,
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

// ── Per-file diff ──────────────────────────────────────────────────────────

/// The type of a single line in a unified diff.
#[derive(Debug, Clone, PartialEq)]
pub enum DiffLineKind {
    /// `@@ ... @@` hunk header.
    Header,
    /// `+` added line.
    Added,
    /// `-` removed line.
    Removed,
    /// Unchanged context line.
    Context,
}

/// One line of a unified diff, as rendered in the Diff panel.
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    /// Raw content (already includes the leading +/−/space prefix character).
    pub content: String,
}

/// Return the unified diff of `file_path` as it changed in `commit_oid`
/// (hex string) inside the repository at `repo_path`.
/// Returns an empty Vec on any error.
pub fn get_commit_file_diff(repo_path: &Path, commit_oid: &str, file_path: &str) -> Vec<DiffLine> {
    get_file_diff_inner(repo_path, commit_oid, file_path).unwrap_or_default()
}

/// Return the diff for `file_path` in the working tree.
///
/// - `staged = true`:  diff between HEAD and the index (what would be committed).
/// - `staged = false`: diff between the index and the working directory (unstaged changes).
///
/// Returns an empty Vec on any error.
pub fn get_worktree_file_diff(repo_path: &Path, file_path: &str, staged: bool) -> Vec<DiffLine> {
    get_worktree_diff_inner(repo_path, file_path, staged).unwrap_or_default()
}

/// Add `file_path` to the index (equivalent to `git add <file>`).
/// Returns a human-readable error string on failure.
pub fn stage_file(repo_path: &Path, file_path: &str) -> Result<(), String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let mut index = repo.index().map_err(|e| e.to_string())?;
    index
        .add_path(Path::new(file_path))
        .map_err(|e| e.to_string())?;
    index.write().map_err(|e| e.to_string())?;
    Ok(())
}

/// Remove `file_path` from the index (equivalent to `git restore --staged <file>`).
/// When HEAD exists the index entry is reset to the HEAD tree value; for a brand-new
/// repo with no commits the entry is simply removed from the index.
/// Returns a human-readable error string on failure.
pub fn unstage_file(repo_path: &Path, file_path: &str) -> Result<(), String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    // Prefer reset_default (git reset HEAD -- <file>) when a HEAD commit exists.
    if let Some(commit) = repo.head().ok().and_then(|h| h.peel_to_commit().ok()) {
        repo.reset_default(Some(commit.as_object()), std::iter::once(file_path))
            .map_err(|e| e.to_string())?;
    } else {
        // New repo with no commits: just remove the entry from the index.
        let mut index = repo.index().map_err(|e| e.to_string())?;
        index
            .remove_path(Path::new(file_path))
            .map_err(|e| e.to_string())?;
        index.write().map_err(|e| e.to_string())?;
    }
    Ok(())
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

fn collect_commits(
    repo: &Repository,
    limit: usize,
    ref_map: &std::collections::HashMap<git2::Oid, Vec<String>>,
) -> Result<Vec<CommitEntry>, git2::Error> {
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
            let oid_str = commit.id().to_string();
            let summary = commit
                .summary()
                .ok()
                .flatten()
                .unwrap_or("(no commit message)")
                .to_string();
            let author = commit.author();
            let author_str = author.name().unwrap_or("?").to_string();
            let when = format_relative_time(commit.time().seconds());
            let refs = ref_map.get(&oid).cloned().unwrap_or_default();
            let files = commit_changed_files(repo, &commit);
            commits.push(CommitEntry {
                id: short_id,
                oid: oid_str,
                author: author_str,
                when,
                summary,
                refs,
                files,
            });
        }
    }
    Ok(commits)
}

/// Diff `commit` against its first parent (or against an empty tree for the
/// initial commit) and return the list of changed files. Capped at
/// `MAX_FILES_PER_SECTION` entries.
fn commit_changed_files(repo: &Repository, commit: &git2::Commit) -> Vec<FileEntry> {
    let commit_tree = match commit.tree() {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    // For the initial commit parent_tree is None — libgit2 treats that as an
    // empty tree, so all files appear as "added".
    let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

    let diff = match repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), None) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let mut files = Vec::new();
    for delta in diff.deltas() {
        if files.len() >= MAX_FILES_PER_SECTION {
            break;
        }
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "(unknown)".to_string());

        let label: &'static str = match delta.status() {
            git2::Delta::Added => "new",
            git2::Delta::Deleted => "deleted",
            git2::Delta::Modified => "modified",
            git2::Delta::Renamed => "renamed",
            git2::Delta::Typechange => "typechange",
            _ => "modified",
        };
        files.push(FileEntry { path, label });
    }
    files
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

    if let Ok(commits) = collect_commits(&repo, 50, &build_ref_map(&repo)) {
        info.commits = commits;
    }

    populate_file_changes(&repo, &mut info);
    Ok(info)
}

/// Build a map from commit `Oid` → list of ref names that point to it.
/// Local branches are stored as plain names (e.g. `"main"`).
/// Lightweight and annotated tags are stored with a `"tag:"` prefix
/// (e.g. `"tag:v1.0"`) so the UI can colour them differently.
fn build_ref_map(repo: &Repository) -> std::collections::HashMap<git2::Oid, Vec<String>> {
    let mut map: std::collections::HashMap<git2::Oid, Vec<String>> =
        std::collections::HashMap::new();

    if let Ok(refs) = repo.references() {
        for reference in refs.flatten() {
            // Resolve to the underlying commit Oid (peeling through tags).
            let Ok(target) = reference.peel_to_commit() else {
                continue;
            };
            let oid = target.id();

            let Ok(full_name) = reference.name() else {
                continue;
            };

            let label = if let Some(branch) = full_name.strip_prefix("refs/heads/") {
                branch.to_string()
            } else if let Some(tag) = full_name.strip_prefix("refs/tags/") {
                format!("tag:{}", tag)
            } else if let Some(remote) = full_name.strip_prefix("refs/remotes/") {
                // Skip the symbolic HEAD pointer each remote keeps (e.g. origin/HEAD).
                if remote.ends_with("/HEAD") {
                    continue;
                }
                format!("remote:{}", remote)
            } else {
                continue;
            };

            map.entry(oid).or_default().push(label);
        }
    }
    map
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

// ── Per-file diff (private) ────────────────────────────────────────────────

fn get_file_diff_inner(
    repo_path: &Path,
    commit_oid: &str,
    file_path: &str,
) -> Option<Vec<DiffLine>> {
    let repo = Repository::open(repo_path).ok()?;
    let oid = git2::Oid::from_str(commit_oid).ok()?;
    let commit = repo.find_commit(oid).ok()?;

    let commit_tree = commit.tree().ok()?;
    // For the initial commit, parent_tree is None; libgit2 treats it as empty.
    let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());

    let mut opts = git2::DiffOptions::new();
    opts.pathspec(file_path);

    let diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), Some(&mut opts))
        .ok()?;

    collect_diff_lines(&diff)
}

/// Diff a single file in the working tree.
///
/// `staged = true`:  HEAD-tree → index (what `git diff --cached` shows).
/// `staged = false`: index → working directory (what `git diff` shows).
fn get_worktree_diff_inner(
    repo_path: &Path,
    file_path: &str,
    staged: bool,
) -> Option<Vec<DiffLine>> {
    let repo = Repository::open(repo_path).ok()?;
    let mut opts = git2::DiffOptions::new();
    opts.pathspec(file_path);

    let diff = if staged {
        // Staged: diff HEAD tree (or empty tree for new repos) → index.
        let head_tree = repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_tree().ok());
        repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))
            .ok()?
    } else {
        // Unstaged: diff index → working directory.
        repo.diff_index_to_workdir(None, Some(&mut opts)).ok()?
    };

    collect_diff_lines(&diff)
}

/// Walk a libgit2 `Diff` and collect coloured `DiffLine` values.
fn collect_diff_lines(diff: &git2::Diff<'_>) -> Option<Vec<DiffLine>> {
    let mut lines: Vec<DiffLine> = Vec::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let kind = match line.origin() {
            '+' => DiffLineKind::Added,
            '-' => DiffLineKind::Removed,
            'H' => DiffLineKind::Header,
            ' ' => DiffLineKind::Context,
            _ => return true, // skip file-header meta lines
        };
        let content = String::from_utf8_lossy(line.content())
            .trim_end_matches('\n')
            .trim_end_matches('\r')
            .to_string();
        lines.push(DiffLine { kind, content });
        true
    })
    .ok()?;
    Some(lines)
}

