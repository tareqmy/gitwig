//! Filesystem + git repository inspection.
//!
//! This module owns both the "is this a git repo?" classification used by
//! the per-card indicator AND the richer detail collection used by the
//! Detail view. They share a single `collect_summary` helper so the same
//! libgit2 work doesn't run twice.
//!
//! The cheap `is_dir()` + `.git`-existence check still gates everything —
//! we only spin up libgit2 when both checks pass.

#![deny(unsafe_code)]
#![deny(unused_imports, unused_must_use, dead_code, unused_assignments)]
#![deny(clippy::all, clippy::perf)]
#![allow(clippy::collapsible_if, clippy::collapsible_else_if)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::panic))]

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use git2::{Repository, StatusOptions, StatusShow};

mod branch_tag;
mod conflicts;
mod forge;
mod history;
mod patch;
mod process;
mod stash;
mod submodule;
#[cfg(test)]
mod tests;
mod worktree;
pub use branch_tag::{
    checkout_local_branch, checkout_remote_branch, checkout_tag, create_branch, create_tag,
    delete_local_branch, delete_remote_branch, delete_remote_tag, delete_tag, deserialize_tags,
    get_branch_push_target, get_branch_upstream_remote, get_remote_tags, has_upstream_remote,
    is_root_commit, serialize_tags, tag_exists,
};
pub use conflicts::{
    abort_merge, continue_merge, get_conflict_markers_diff, is_merging, mark_resolved,
    resolve_conflict_hunk, resolve_ours, resolve_theirs,
};
pub use forge::{
    CIStatusCheck, ForgeIssue, ForgePR, ForgePRComment, PRReview, add_pr_line_comment,
    branch_names_issue, checkout_issue_branch_by_name, checkout_pr_branch, get_default_remote_url,
    get_remote_url, load_pr_comments, load_tab_forge_issues, load_tab_forge_prs, open_browser,
    resolve_and_checkout_issue_branch,
};
#[cfg(test)]
pub(crate) use forge::{
    checkout_linked_issue_branch, parse_forge_prs, parse_linked_branches, parse_pr_comments,
    pr_line_comment_args,
};
pub use history::{
    BlameEntry, FileRevision, checkout_commit, get_commit_files, get_file_blame, get_file_history,
    load_tab_reflog,
};
pub use process::{
    GitRunError, detached_command, git_command, run_git_with_timeout, set_ssh_strict_host_checking,
    spawn_pipe_reader, ssh_command_val, summarize_stderr, tool_command,
};
pub use stash::{save_stash, stash_hunk, stash_line};
pub use submodule::{SubmoduleInfo, load_tab_submodules, submodule_remove, submodule_update};
pub use worktree::{
    WorktreeInfo, load_tab_worktrees, worktree_add, worktree_lock, worktree_prune, worktree_remove,
    worktree_unlock,
};

// ── Card-level status ──────────────────────────────────────────────────────

/// Per-item filesystem classification carried alongside `config.items`.
/// `GitRepo`'s inner `Option` is `None` when `.git` exists but libgit2
/// couldn't open or read the repo — we know it's a repo, we just can't
/// summarize its state.
#[derive(Debug, Clone)]
pub enum ItemStatus {
    Loading,
    Missing,
    Directory,
    GitRepo(Option<RepoSummary>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepoState {
    #[default]
    Clean,
    Merge,
    Revert,
    CherryPick,
    Bisect,
    Rebase,
    ApplyMailbox,
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
    pub state: RepoState,
    pub last_commit_time: Option<i64>,
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

#[derive(Debug, Clone)]
pub enum ItemDetail {
    Missing { resolved: PathBuf },
    Directory { resolved: PathBuf },
    Repo { resolved: PathBuf, info: Box<RepoInfo> },
    Error { resolved: PathBuf, message: String },
}

#[derive(Debug, Clone, Default)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub short_sha: String,
    pub short_message: String,
}

#[derive(Debug, Clone, Default)]
pub struct StashInfo {
    pub index: usize,
    pub message: String,
    pub commit_id: String,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct CommitterStat {
    pub name: String,
    pub email: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TabData<T> {
    #[default]
    NotLoaded,
    Loading,
    Loaded(T),
    Error(String),
}

impl<T> TabData<T> {
    pub fn is_not_loaded(&self) -> bool {
        matches!(self, TabData::NotLoaded)
    }
    pub fn is_loading(&self) -> bool {
        matches!(self, TabData::Loading)
    }
    #[allow(dead_code)]
    pub fn is_loaded(&self) -> bool {
        matches!(self, TabData::Loaded(_))
    }
    pub fn as_ref(&self) -> Option<&T> {
        match self {
            TabData::Loaded(val) => Some(val),
            _ => None,
        }
    }
}

impl<T> TabData<Vec<T>> {
    pub fn len(&self) -> usize {
        self.as_ref().map(|v| v.len()).unwrap_or(0)
    }
    pub fn is_empty(&self) -> bool {
        self.as_ref().map(|v| v.is_empty()).unwrap_or(true)
    }
    pub fn first(&self) -> Option<&T> {
        self.as_ref().and_then(|v| v.first())
    }
    pub fn get(&self, index: usize) -> Option<&T> {
        self.as_ref().and_then(|v| v.get(index))
    }
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        match self {
            TabData::Loaded(v) => v.iter(),
            _ => [].iter(),
        }
    }
    pub fn as_slice(&self) -> &[T] {
        match self {
            TabData::Loaded(v) => v.as_slice(),
            _ => &[],
        }
    }
}

#[derive(Debug, Clone)]
pub enum TabPayload {
    Files(Result<Vec<String>, String>),
    Graph(Result<Vec<GraphLine>, String>),
    Branches {
        local: Result<Vec<BranchInfo>, String>,
        remote: Result<Vec<BranchInfo>, String>,
    },
    Tags {
        local: Result<Vec<BranchInfo>, String>,
        remote: Result<Vec<BranchInfo>, String>,
    },
    Remotes(Result<Vec<RemoteInfo>, String>),
    Stashes(Result<Vec<StashInfo>, String>),
    Overview(Result<(Vec<CommitterStat>, bool), String>),
    Worktrees(Result<Vec<WorktreeInfo>, String>),
    Submodules(Result<Vec<SubmoduleInfo>, String>),
    Reflog(Result<Vec<ReflogEntry>, String>),
    /// A non-fatal message for the status bar, sent alongside a tab payload
    /// (for example: git could not run the signature program).
    Notice(String),
    ForgeIssues(Result<Vec<ForgeIssue>, String>),
    ForgePRs(Result<Vec<ForgePR>, String>),
    /// Line comments of PR `pr_number`. Loads can finish out of order, so the
    /// number says which PR a result belongs to.
    PRComments {
        pr_number: u32,
        result: Result<Vec<ForgePRComment>, String>,
    },
}

#[derive(Debug, Default, Clone)]
pub struct RepoInfo {
    pub branch: Option<String>,
    pub head: Option<HeadInfo>,
    pub remotes: TabData<Vec<RemoteInfo>>,
    /// Configured upstream branch (e.g. "origin/main") if HEAD tracks one.
    pub upstream: Option<String>,
    pub summary: RepoSummary,
    /// File-level changes, populated by `collect_info` for the Detail view.
    pub changes: WorktreeChanges,
    /// Recent commits in this repository.
    pub commits: Vec<CommitEntry>,
    /// Graph view lines for the repository.
    pub graph_lines: TabData<Vec<GraphLine>>,
    /// Local branches in the repository.
    pub local_branches: TabData<Vec<BranchInfo>>,
    /// Remote branches in the repository.
    pub remote_branches: TabData<Vec<BranchInfo>>,
    /// Local tags in the repository.
    pub local_tags: TabData<Vec<BranchInfo>>,
    /// Remote tags in the repository.
    pub remote_tags: TabData<Vec<BranchInfo>>,
    /// Whether remote tags have been loaded from the remote repository.
    pub remote_tags_loaded: bool,
    /// Whether a remote tag fetch has been attempted in this session.
    pub remote_tags_attempted: bool,
    /// Tracked files in the repository.
    pub files: TabData<Vec<String>>,
    /// Available stashes in the repository.
    pub stashes: TabData<Vec<StashInfo>>,
    /// Available worktrees in the repository.
    pub worktrees: TabData<Vec<WorktreeInfo>>,
    /// Configured submodules in the repository.
    pub submodules: TabData<Vec<SubmoduleInfo>>,
    /// Available reflog entries.
    pub reflog: TabData<Vec<ReflogEntry>>,
    pub forge_issues: TabData<Vec<ForgeIssue>>,
    pub forge_prs: TabData<Vec<ForgePR>>,
    /// Committer statistics.
    pub committer_stats: TabData<Vec<CommitterStat>>,
    /// Whether the committer statistics walk was capped by the limit.
    pub committer_stats_limit_reached: bool,
    /// Timestamps when each tab was loaded (index matches tab_idx)
    pub tab_loaded_at: [Option<std::time::Instant>; 12],
    /// Whether each tab is currently loading in the background (index matches tab_idx)
    pub tab_loading: [bool; 12],
    /// Files tracked by Git LFS in the index/working copy.
    pub lfs_files: std::collections::HashSet<String>,
    /// Whether git-lfs is installed in the system PATH.
    pub lfs_installed: bool,
    /// Storage size of the local LFS directory if LFS is used.
    pub lfs_storage_size: Option<u64>,
}

impl RepoInfo {
    /// Forgets the background tab loads that were in flight. Their results were
    /// meant for a detail view that has since closed and are dropped on arrival,
    /// so a tab restored still flagged as loading (or showing `Loading`) would
    /// never load again.
    pub fn reset_in_flight_loads(&mut self) {
        fn unstick<T>(data: &mut TabData<T>) {
            if data.is_loading() {
                *data = TabData::NotLoaded;
            }
        }
        self.tab_loading = [false; 12];
        unstick(&mut self.remotes);
        unstick(&mut self.graph_lines);
        unstick(&mut self.local_branches);
        unstick(&mut self.remote_branches);
        unstick(&mut self.local_tags);
        unstick(&mut self.remote_tags);
        unstick(&mut self.files);
        unstick(&mut self.stashes);
        unstick(&mut self.worktrees);
        unstick(&mut self.submodules);
        unstick(&mut self.reflog);
        unstick(&mut self.forge_issues);
        unstick(&mut self.forge_prs);
        unstick(&mut self.committer_stats);
    }
}

#[derive(Debug, Clone)]
pub struct HeadInfo {
    pub short_id: String,
    pub summary: String,
    pub author: String,
    pub when: String,
}

#[derive(Debug, Clone)]
pub struct ReflogEntry {
    pub index: usize,
    pub target_oid: String,
    pub selector: String,
    pub command: String,
    pub message: String,
    pub when: String,
    pub date: String,
}

#[derive(Debug, Clone)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
    pub push_url: Option<String>,
    pub refspecs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CommitEntry {
    /// Short 7-char display ID.
    pub id: String,
    /// Full 40-char hex OID — used for diff lookup.
    pub oid: String,
    pub author: String,
    pub when: String,
    pub date: String,
    pub summary: String,
    pub message: String,
    /// Local branch names and tags pointing at this commit.
    /// Tags are prefixed with `"tag:"`, remote branches with `"remote:"`.
    pub refs: Vec<String>,
    /// Files changed in this commit (diff against first parent, or empty tree).
    pub files: Vec<FileEntry>,
    /// GPG/SSH signature status.
    pub signature_status: String,
}

#[derive(Debug, Clone)]
pub struct GraphLine {
    pub graph: String,
    pub commit: Option<GraphCommit>,
}

#[derive(Debug, Clone)]
pub struct GraphCommit {
    pub oid: String,
    pub decoration: String,
    pub summary: String,
    pub author: String,
    pub date: String,
    /// GPG/SSH signature status.
    pub signature_status: String,
}

/// One changed file in the working tree or index.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Path relative to the repository root.
    pub path: String,
    /// Short human-readable label: "N", "M", "D", "R", "T", "?", or "C".
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
    /// Line in OURS section of a conflict.
    ConflictOurs,
    /// Line in THEIRS section of a conflict.
    ConflictTheirs,
    /// Conflict marker line (<<<<<<<, =======, >>>>>>>).
    ConflictSeparator,
}

/// One line of a unified diff, as rendered in the Diff panel.
#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    /// Raw content (already includes the leading +/−/space prefix character).
    pub content: String,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub hunk_idx: Option<usize>,
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
    let full_path = repo_path.join(file_path);
    if full_path.exists() {
        index.add_path(Path::new(file_path)).map_err(|e| e.to_string())?;
    } else {
        index.remove_path(Path::new(file_path)).map_err(|e| e.to_string())?;
    }
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
        index.remove_path(Path::new(file_path)).map_err(|e| e.to_string())?;
        index.write().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn sanitize_text(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1B' {
            // Escape character: let's skip the sequence
            if let Some(&'[') = chars.peek() {
                chars.next();
                for seq_c in chars.by_ref() {
                    // ANSI escape sequences end with letters (A-Z, a-z)
                    if seq_c.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                // Single ESC character or other VT100: skip it
                for seq_c in chars.by_ref() {
                    if seq_c == ' ' || seq_c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else if (c < ' ' || c == '\x7F') && c != '\n' && c != '\r' && c != '\t' {
            // Replace non-printable control characters except \n, \r, \t
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

pub fn safe_sha_slice(sha: &str, len: usize) -> &str {
    let mut end = len.min(sha.len());
    while end > 0 && !sha.is_char_boundary(end) {
        end -= 1;
    }
    &sha[..end]
}

pub fn safe_ref(r: &str) -> Result<&str, String> {
    let trimmed = r.trim();
    if trimmed.starts_with('-') {
        return Err(format!("Invalid ref name: '{}' (ref names cannot start with '-')", r));
    }
    if trimmed.is_empty() {
        return Err("Ref name cannot be empty".to_string());
    }
    Ok(trimmed)
}

/// Stage all unstaged/untracked changes (equivalent to `git add -A`).
pub fn stage_all_changes(repo_path: &Path) -> Result<(), String> {
    let output = git_command()
        .arg("add")
        .arg("-A")
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Unstage all staged changes (equivalent to `git reset`).
pub fn unstage_all_changes(repo_path: &Path) -> Result<(), String> {
    let output =
        git_command().arg("reset").current_dir(repo_path).output().map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Discard all staged, unstaged, and untracked changes in the repository.
pub fn discard_all_changes(repo_path: &Path) -> Result<(), String> {
    // 1. Unstage all first so everything is in the working tree
    let _ = git_command().arg("reset").current_dir(repo_path).output();

    // 2. Discard all tracked modifications
    let checkout_out = git_command()
        .arg("checkout")
        .arg("--")
        .arg(".")
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;

    if !checkout_out.status.success() {
        return Err(String::from_utf8_lossy(&checkout_out.stderr).trim().to_string());
    }

    // 3. Clean all untracked files/folders
    let clean_out = git_command()
        .arg("clean")
        .arg("-fd")
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;

    if !clean_out.status.success() {
        return Err(String::from_utf8_lossy(&clean_out.stderr).trim().to_string());
    }

    Ok(())
}

/// Stage a single hunk of unstaged changes (equivalent to `git apply --cached -`).
pub fn stage_hunk(repo_path: &Path, file_path: &str, hunk: &[DiffLine]) -> Result<(), String> {
    patch::git_apply(repo_path, &patch::hunk_patch(file_path, hunk)?, &["--cached"], None)
}

/// Unstage a single hunk of staged changes (equivalent to `git apply --cached --reverse -`).
pub fn unstage_hunk(repo_path: &Path, file_path: &str, hunk: &[DiffLine]) -> Result<(), String> {
    let patch = patch::hunk_patch(file_path, hunk)?;
    patch::git_apply(repo_path, &patch, &["--cached", "--reverse"], None)
}

/// Discard a single hunk of unstaged changes in the working tree (equivalent to `git apply --reverse -`).
pub fn discard_hunk(repo_path: &Path, file_path: &str, hunk: &[DiffLine]) -> Result<(), String> {
    patch::git_apply(repo_path, &patch::hunk_patch(file_path, hunk)?, &["--reverse"], None)
}

/// Stage a single line from the Unstaged diff.
pub fn stage_line(
    repo_path: &Path,
    file_path: &str,
    hunk: &[DiffLine],
    selected_line_idx: usize,
) -> Result<(), String> {
    let patch = patch::line_patch(file_path, hunk, selected_line_idx, false, false)?;
    patch::git_apply(repo_path, &patch, &["--cached"], None)
}

/// Unstage a single line from the Staged diff.
pub fn unstage_line(
    repo_path: &Path,
    file_path: &str,
    hunk: &[DiffLine],
    selected_line_idx: usize,
) -> Result<(), String> {
    let patch = patch::line_patch(file_path, hunk, selected_line_idx, true, true)?;
    patch::git_apply(repo_path, &patch, &["--cached"], None)
}

/// Discard a single line from the Unstaged diff in the working tree.
pub fn discard_line(
    repo_path: &Path,
    file_path: &str,
    hunk: &[DiffLine],
    selected_line_idx: usize,
) -> Result<(), String> {
    let patch = patch::line_patch(file_path, hunk, selected_line_idx, true, true)?;
    patch::git_apply(repo_path, &patch, &[], None)
}

/// Discards uncommitted changes in `file_path`.
/// - If the file is untracked, it is deleted from the filesystem.
/// - If the file is tracked and modified/deleted, it is restored from the index.
/// - If the file is staged, it is first unstaged (reset to HEAD) and then restored from index.
pub fn discard_file_changes(repo_path: &Path, file_path: &str, staged: bool) -> Result<(), String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;

    if staged {
        // First unstage it (reset to HEAD)
        unstage_file(repo_path, file_path)?;
    }

    // Now check if the file is untracked
    let is_untracked = if let Ok(status) = repo.status_file(Path::new(file_path)) {
        status.contains(git2::Status::WT_NEW)
    } else {
        false
    };

    if is_untracked {
        let full_path = repo_path.join(file_path);
        if full_path.exists() {
            if full_path.is_file() {
                std::fs::remove_file(&full_path).map_err(|e| e.to_string())?;
            } else if full_path.is_dir() {
                std::fs::remove_dir_all(&full_path).map_err(|e| e.to_string())?;
            }
        }
    } else {
        // Tracked file: checkout from index to working tree
        let mut checkout_opts = git2::build::CheckoutBuilder::new();
        checkout_opts.path(Path::new(file_path));
        checkout_opts.force();
        repo.checkout_index(None, Some(&mut checkout_opts)).map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn run_git_commit(repo_path: &Path, message: &str, amend: bool) -> Result<(), String> {
    use std::io::Write;
    use std::process::Stdio;

    let mut cmd = git_command();
    cmd.arg("commit");
    if amend {
        cmd.arg("--amend");
    }
    cmd.args(["--cleanup=verbatim", "-F", "-"]).current_dir(repo_path).stdin(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("Failed to run git commit: {}", e))?;
    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(message.as_bytes()) {
            // A pre-commit hook can fail before `git commit` reads `-F -`,
            // closing the pipe; wait_with_output still captures the hook error.
            if e.kind() != std::io::ErrorKind::BrokenPipe {
                return Err(format!("Failed to write commit message: {}", e));
            }
        }
    }

    let output =
        child.wait_with_output().map_err(|e| format!("Failed to wait for git commit: {}", e))?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let msg = match (stderr.is_empty(), stdout.is_empty()) {
        (false, false) => format!("{}\n{}", stderr, stdout),
        (false, true) => stderr,
        (true, false) => stdout,
        (true, true) => "git commit failed".to_string(),
    };
    Err(msg)
}

/// Create a commit in the repository with the given message.
/// Shells out to `git commit` so repository hooks and signing configs run.
/// Returns a human-readable error string on failure.
pub fn commit_changes(repo_path: &Path, message: &str) -> Result<(), String> {
    run_git_commit(repo_path, message, false)
}

// ── Public entry points ────────────────────────────────────────────────────

/// Resolved `.git` admin directory and shared common directory for a checkout.
/// For a standard repository both point to `<repo>/.git`; for a linked worktree
/// `git_dir` is `<main>/.git/worktrees/<name>` and `common_dir` is `<main>/.git`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitDirPaths {
    pub git_dir: PathBuf,
    pub common_dir: PathBuf,
}

/// Resolves the `git_dir` and `common_dir` of `repo_path` without opening
/// libgit2, handling both `.git/` directories and linked-worktree `.git` files.
pub fn resolve_git_dirs(repo_path: &Path) -> Option<GitDirPaths> {
    let dot_git = repo_path.join(".git");
    if dot_git.is_dir() {
        let git_dir = std::fs::canonicalize(&dot_git).unwrap_or(dot_git);
        return Some(GitDirPaths { common_dir: git_dir.clone(), git_dir });
    }
    if !dot_git.is_file() {
        return None;
    }

    let content = std::fs::read_to_string(&dot_git).ok()?;
    let spec = content.lines().find_map(|line| line.trim().strip_prefix("gitdir:"))?.trim();
    if spec.is_empty() {
        return None;
    }

    let raw_git_dir = PathBuf::from(spec);
    let joined_git_dir =
        if raw_git_dir.is_absolute() { raw_git_dir } else { repo_path.join(raw_git_dir) };
    let git_dir = std::fs::canonicalize(&joined_git_dir).unwrap_or(joined_git_dir);
    if !git_dir.is_dir() {
        return None;
    }

    let common_file = git_dir.join("commondir");
    let common_dir = if common_file.is_file() {
        std::fs::read_to_string(&common_file)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(|rel| {
                let p = PathBuf::from(rel);
                let joined = if p.is_absolute() { p } else { git_dir.join(p) };
                std::fs::canonicalize(&joined).unwrap_or(joined)
            })
            .unwrap_or_else(|| git_dir.clone())
    } else {
        git_dir.clone()
    };

    Some(GitDirPaths { git_dir, common_dir })
}

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

/// Walk up from `start` and return the first directory containing a `.git`
/// entry, or `None` if there is no repository above it.
///
/// `.git` is matched as either a directory (an ordinary clone) or a file (a
/// linked worktree or a submodule, where it holds a `gitdir:` pointer), so a
/// worktree checkout reports its own root rather than the main one.
pub fn discover_repo_root(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_dir() { Some(start) } else { start.parent() };
    while let Some(current) = dir {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }
        dir = current.parent();
    }
    None
}

/// Add a new git remote.
pub fn remote_add(repo_path: &std::path::Path, name: &str, url: &str) -> Result<(), git2::Error> {
    let repo = Repository::open(repo_path)?;
    repo.remote(name, url)?;
    Ok(())
}

/// Delete an existing git remote.
pub fn remote_delete(repo_path: &std::path::Path, name: &str) -> Result<(), git2::Error> {
    let repo = Repository::open(repo_path)?;
    repo.remote_delete(name)?;
    Ok(())
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
pub fn inspect_detail(
    item: &str,
    commit_limit: usize,
    graph_max_commits: usize,
    enable_commit_signatures: bool,
) -> ItemDetail {
    let resolved = expand_tilde(item);
    if !resolved.is_dir() {
        return ItemDetail::Missing { resolved };
    }
    if !resolved.join(".git").exists() {
        return ItemDetail::Directory { resolved };
    }
    match collect_info(&resolved, commit_limit, graph_max_commits, enable_commit_signatures) {
        Ok(info) => ItemDetail::Repo { resolved, info: Box::new(info) },
        Err(e) => ItemDetail::Error { resolved, message: e.to_string() },
    }
}

fn collect_signatures(repo_path: &Path, limit: usize) -> std::collections::HashMap<String, String> {
    let mut sigs = std::collections::HashMap::new();
    let mut cmd = git_command();
    cmd.arg("log").arg("--all");

    if limit > 0 {
        cmd.arg(format!("-n{}", limit));
    }

    cmd.arg("--pretty=format:%H %G?").current_dir(repo_path);

    if let Ok(out) = cmd.output() {
        if out.status.success() {
            let stdout_str = String::from_utf8_lossy(&out.stdout);
            for line in stdout_str.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() == 2 {
                    sigs.insert(parts[0].to_string(), parts[1].to_string());
                } else if parts.len() == 1 {
                    sigs.insert(parts[0].to_string(), "N".to_string());
                }
            }
        }
    }
    sigs
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct CachedCommit {
    id: String,
    author: String,
    date: String,
    summary: String,
    message: String,
    time: i64,
}

fn hash_path(path: &Path) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn collect_commits(
    repo: &Repository,
    limit: usize,
    repo_path: &Path,
    enable_commit_signatures: bool,
) -> Result<Vec<CommitEntry>, git2::Error> {
    let mut walk = repo.revwalk()?;
    if walk.push_head().is_err() {
        return Ok(Vec::new());
    }
    walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;

    let mut commits = Vec::new();
    let oids: Vec<Result<git2::Oid, git2::Error>> =
        if limit > 0 { walk.take(limit).collect() } else { walk.collect() };

    let sig_map = if enable_commit_signatures {
        collect_signatures(repo_path, limit)
    } else {
        std::collections::HashMap::new()
    };
    let ref_map = get_cached_ref_map(repo, repo_path);

    // Load commit cache
    let cache_dir = dirs::home_dir().map(|h| h.join(".gitwig/commit_cache"));
    if let Some(ref dir) = cache_dir {
        let _ = std::fs::create_dir_all(dir);
    }
    let hash = hash_path(repo_path);
    let cache_file = cache_dir.as_ref().map(|d| d.join(format!("v2_{}.json", hash)));
    let mut cache: std::collections::HashMap<String, CachedCommit> = cache_file
        .as_ref()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    let mut cache_updated = false;

    for id in oids {
        let oid = id?;
        let oid_str = oid.to_string();
        let sig_status = sig_map.get(&oid_str).cloned().unwrap_or_else(|| "N".to_string());
        let refs = ref_map.get(&oid).cloned().unwrap_or_default();
        let files = Vec::new();

        if let Some(cached) = cache.get(&oid_str) {
            let when = format_relative_time(cached.time);
            commits.push(CommitEntry {
                id: cached.id.clone(),
                oid: oid_str,
                author: sanitize_text(&cached.author),
                when,
                date: cached.date.clone(),
                summary: sanitize_text(&cached.summary),
                message: sanitize_text(&cached.message),
                refs,
                files,
                signature_status: sig_status,
            });
        } else if let Ok(commit) = repo.find_commit(oid) {
            let short_id = format!("{:.7}", commit.id());
            let summary =
                sanitize_text(commit.summary().ok().flatten().unwrap_or("(no commit message)"));
            let author = commit.author();
            let author_name = author.name().unwrap_or("?");
            let author_email = author.email().unwrap_or("?");
            let author_str = sanitize_text(&format!("{} <{}>", author_name, author_email));
            let time_secs = commit.time().seconds();
            let when = format_relative_time(time_secs);
            let date = format_utc_date(time_secs);
            let message = sanitize_text(commit.message().unwrap_or("(no commit message)"));

            let cached = CachedCommit {
                id: short_id.clone(),
                author: author_str.clone(),
                date: date.clone(),
                summary: summary.clone(),
                message: message.clone(),
                time: time_secs,
            };
            cache.insert(oid_str.clone(), cached);
            cache_updated = true;

            commits.push(CommitEntry {
                id: short_id,
                oid: oid_str,
                author: author_str,
                when,
                date,
                summary,
                message,
                refs,
                files,
                signature_status: sig_status,
            });
        }
    }

    if cache_updated {
        if let Some(ref f) = cache_file {
            if let Ok(json) = serde_json::to_string(&cache) {
                let _ = std::fs::write(f, json);
            }
        }
    }

    Ok(commits)
}

fn collect_committer_stats(
    repo: &Repository,
    limit: usize,
) -> Result<(Vec<CommitterStat>, bool), git2::Error> {
    let mut walk = repo.revwalk()?;
    if walk.push_head().is_err() {
        return Ok((Vec::new(), false));
    }
    let mut counts = std::collections::HashMap::new();
    let mut count = 0;
    let mut limit_reached = false;
    for id in walk {
        let oid = id?;
        if let Ok(commit) = repo.find_commit(oid) {
            let author = commit.author();
            let name = author.name().unwrap_or("?").to_string();
            let email = author.email().unwrap_or("?").to_string();
            let key = (name, email);
            *counts.entry(key).or_insert(0) += 1;
            count += 1;
            if count >= limit {
                limit_reached = true;
                break;
            }
        }
    }

    let mut stats: Vec<CommitterStat> = counts
        .into_iter()
        .map(|((name, email), count)| CommitterStat { name, email, count })
        .collect();

    stats.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));

    Ok((stats, limit_reached))
}

/// Diff `commit` against its first parent (or against an empty tree for the
/// initial commit) and return the list of changed files. Capped at
/// `MAX_FILES_PER_SECTION` entries.
pub(crate) fn commit_changed_files(repo: &Repository, commit: &git2::Commit) -> Vec<FileEntry> {
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
            git2::Delta::Added => "N",
            git2::Delta::Deleted => "D",
            git2::Delta::Modified => "M",
            git2::Delta::Renamed => "R",
            git2::Delta::Typechange => "T",
            _ => "M",
        };
        files.push(FileEntry { path, label });
    }
    files
}

fn match_pattern(path: &str, pattern: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix('*') {
        return path.ends_with(suffix);
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return path.starts_with(prefix);
    }
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            return path.starts_with(parts[0]) && path.ends_with(parts[1]);
        }
    }
    path == pattern || path.ends_with(&format!("/{}", pattern))
}

static LFS_INSTALLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

fn is_lfs_installed() -> bool {
    *LFS_INSTALLED.get_or_init(|| {
        if let Ok(output) = git_command().arg("lfs").arg("--version").output() {
            output.status.success()
        } else {
            false
        }
    })
}

fn get_lfs_info(repo_path: &Path) -> (bool, std::collections::HashSet<String>) {
    let mut tracked = std::collections::HashSet::new();

    // Skip LFS subprocesses entirely if the repo is not using LFS
    let lfs_dir = repo_path.join(".git").join("lfs");
    let gitattributes_path = repo_path.join(".gitattributes");
    let has_lfs_attributes = if gitattributes_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&gitattributes_path) {
            content.contains("filter=lfs")
                || content.contains("diff=lfs")
                || content.contains("merge=lfs")
        } else {
            false
        }
    } else {
        false
    };

    if !lfs_dir.exists() && !has_lfs_attributes {
        return (false, tracked);
    }

    let installed = is_lfs_installed();
    if installed {
        if let Ok(output) =
            git_command().arg("lfs").arg("ls-files").arg("-n").current_dir(repo_path).output()
        {
            if output.status.success() {
                if let Ok(stdout) = String::from_utf8(output.stdout) {
                    for line in stdout.lines() {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            tracked.insert(trimmed.to_string());
                        }
                    }
                }
            }
        }
    }

    let gitattributes_path = repo_path.join(".gitattributes");
    if gitattributes_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&gitattributes_path) {
            let mut patterns = Vec::new();
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if line.contains("filter=lfs")
                    || line.contains("diff=lfs")
                    || line.contains("merge=lfs")
                {
                    if let Some(pattern) = line.split_whitespace().next() {
                        patterns.push(pattern.to_string());
                    }
                }
            }

            if !patterns.is_empty() {
                if let Ok(repo) = Repository::open(repo_path) {
                    if let Ok(index) = repo.index() {
                        for entry in index.iter() {
                            if let Ok(path_str) = std::str::from_utf8(&entry.path) {
                                for pat in &patterns {
                                    if match_pattern(path_str, pat) {
                                        tracked.insert(path_str.to_string());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    (installed, tracked)
}

fn get_lfs_storage_size(repo_path: &Path) -> Option<u64> {
    let lfs_path = repo_path.join(".git").join("lfs");
    if !lfs_path.exists() {
        return None;
    }

    fn calculate_dir_size(dir: &Path) -> u64 {
        let mut size = 0;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        size += calculate_dir_size(&entry.path());
                    } else {
                        size += metadata.len();
                    }
                }
            }
        }
        size
    }

    Some(calculate_dir_size(&lfs_path))
}

// ── Internal collection ────────────────────────────────────────────────────

fn collect_info(
    path: &Path,
    commit_limit: usize,
    _graph_max_commits: usize,
    enable_commit_signatures: bool,
) -> Result<RepoInfo, git2::Error> {
    let repo = Repository::open(path)?;
    let mut summary = RepoSummary::default();
    if let Ok(head) = repo.head() {
        summary.branch = head.shorthand().ok().map(String::from);
    }
    populate_ahead_behind(&repo, &mut summary);

    let mut info = RepoInfo { summary, ..RepoInfo::default() };
    let (lfs_inst, lfs_f) = get_lfs_info(path);
    info.lfs_installed = lfs_inst;
    info.lfs_files = lfs_f;
    info.lfs_storage_size = get_lfs_storage_size(path);

    let is_detached = repo.head_detached().unwrap_or(false);
    if is_detached {
        info.branch = Some("HEAD".to_string());
        if let Ok(head) = repo.head() {
            if let Ok(commit) = head.peel_to_commit() {
                let short_id = format!("{:.7}", commit.id());
                let summary_text =
                    sanitize_text(commit.summary().ok().flatten().unwrap_or("(no commit message)"));
                let author = commit.author();
                let author_str = sanitize_text(&format!(
                    "{} <{}>",
                    author.name().unwrap_or("?"),
                    author.email().unwrap_or("?")
                ));
                let when = format_relative_time(commit.time().seconds());
                info.head =
                    Some(HeadInfo { short_id, summary: summary_text, author: author_str, when });
            }
        }
    } else if let Ok(head) = repo.head() {
        info.branch = head.shorthand().ok().map(String::from);

        if let Ok(commit) = head.peel_to_commit() {
            let short_id = format!("{:.7}", commit.id());
            let summary_text =
                sanitize_text(commit.summary().ok().flatten().unwrap_or("(no commit message)"));
            let author = commit.author();
            let author_str = sanitize_text(&format!(
                "{} <{}>",
                author.name().unwrap_or("?"),
                author.email().unwrap_or("?")
            ));
            let when = format_relative_time(commit.time().seconds());
            info.head =
                Some(HeadInfo { short_id, summary: summary_text, author: author_str, when });
        }

        if let Ok(head_name) = head.name() {
            info.upstream = upstream_short_name(&repo, head_name);
        }
    }

    if let Ok(commits) = collect_commits(&repo, commit_limit, path, enable_commit_signatures) {
        info.commits = commits;
    }

    populate_summary_and_file_changes(&repo, &mut info);

    if let Ok(remotes) = load_tab_remotes(path) {
        info.remotes = TabData::Loaded(remotes);
        info.tab_loaded_at[5] = Some(std::time::Instant::now());
    }

    Ok(info)
}

pub fn load_tab_files(repo_path: &Path) -> Result<Vec<String>, String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    if let Ok(index) = repo.index() {
        for entry in index.iter() {
            if let Ok(path_str) = std::str::from_utf8(&entry.path) {
                files.push(path_str.to_string());
            }
        }
    }
    Ok(files)
}

/// Everything the Graph tab needs from one `git log --graph` run.
#[derive(Debug, Clone, Default)]
pub struct GraphLoad {
    pub lines: Vec<GraphLine>,
    /// One-line, deduplicated summary of anything git wrote to stderr while
    /// still producing a graph (typically a missing signature program). The
    /// UI shows it in the status bar; it never replaces the graph itself.
    pub notice: Option<String>,
}

pub fn load_tab_graph_stream(
    repo_path: &Path,
    graph_max_commits: usize,
    enable_commit_signatures: bool,
    repo_resolved_path: String,
    tab_idx: usize,
    tx: std::sync::mpsc::Sender<(String, usize, TabPayload)>,
) -> Result<GraphLoad, String> {
    let mut graph_lines = Vec::new();
    // `%G?` makes git exec gpg/ssh-keygen for every signed commit, so it is only
    // requested when the user opted into signature badges.
    let format_str = if enable_commit_signatures {
        "%H__TWIG_SEP__%d__TWIG_SEP__%s__TWIG_SEP__%an__TWIG_SEP__%ad__TWIG_SEP__%G?"
    } else {
        "%H__TWIG_SEP__%d__TWIG_SEP__%s__TWIG_SEP__%an__TWIG_SEP__%ad"
    };

    let mut args = vec![
        "log".to_string(),
        "--graph".to_string(),
        "--all".to_string(),
        "--date=relative".to_string(),
    ];
    if graph_max_commits > 0 {
        args.push(format!("--max-count={}", graph_max_commits));
    }
    args.push(format!("--pretty=format:{}", format_str));
    args.push("--color=never".to_string());

    let mut child =
        git_command().args(&args).current_dir(repo_path).spawn().map_err(|e| e.to_string())?;

    // stderr must be drained alongside stdout: left unread it would fill and
    // stall git; left inherited (the old bug) it would print over the TUI.
    let stderr_reader = spawn_pipe_reader(&mut child);
    let stdout = child.stdout.take().ok_or_else(|| "Failed to open stdout".to_string())?;
    let reader = std::io::BufReader::new(stdout);
    use std::io::BufRead;

    for (idx, line_res) in reader.lines().enumerate() {
        let line = line_res.map_err(|e| e.to_string())?;
        let parsed = parse_graph_line(&line);
        graph_lines.push(parsed);

        // Every 200 lines, send a cloned batch to UI
        if (idx + 1) % 200 == 0 {
            let _ = tx.send((
                repo_resolved_path.clone(),
                tab_idx,
                TabPayload::Graph(Ok(graph_lines.clone())),
            ));
        }
    }

    // Wait for the child process to exit
    let status = child.wait().map_err(|e| e.to_string())?;
    let stderr = stderr_reader.join().unwrap_or_default();
    let notice = summarize_stderr(&stderr);
    if !status.success() && graph_lines.is_empty() {
        return Err(notice.unwrap_or_else(|| "git log failed".to_string()));
    }

    Ok(GraphLoad { lines: graph_lines, notice })
}

pub fn load_tab_branches(
    repo_path: &Path,
) -> (Result<Vec<BranchInfo>, String>, Result<Vec<BranchInfo>, String>) {
    let repo = match Repository::open(repo_path) {
        Ok(r) => r,
        Err(e) => return (Err(e.to_string()), Err(e.to_string())),
    };

    let mut local_branches = Vec::new();
    let is_detached = repo.head_detached().unwrap_or(false);
    if is_detached {
        let mut short_sha = String::new();
        let mut short_message = String::new();
        if let Ok(head) = repo.head() {
            if let Ok(target) = head.peel_to_commit() {
                let id = target.id();
                let id_str = id.to_string();
                short_sha = safe_sha_slice(&id_str, 7).to_string();
                if let Ok(Some(summary)) = target.summary() {
                    short_message = summary.to_string();
                }
            }
        }
        local_branches.push(BranchInfo {
            name: "HEAD".to_string(),
            is_head: true,
            short_sha,
            short_message: sanitize_text(&short_message),
        });
    }
    if let Ok(branches) = repo.branches(Some(git2::BranchType::Local)) {
        for (branch, _) in branches.flatten() {
            if let Ok(Some(name)) = branch.name() {
                let is_head = branch.is_head();
                let mut short_sha = String::new();
                let mut short_message = String::new();
                if let Ok(target) = branch.get().peel_to_commit() {
                    let id = target.id();
                    let id_str = id.to_string();
                    short_sha = safe_sha_slice(&id_str, 7).to_string();
                    if let Ok(Some(summary)) = target.summary() {
                        short_message = summary.to_string();
                    }
                }
                local_branches.push(BranchInfo {
                    name: sanitize_text(name),
                    is_head,
                    short_sha,
                    short_message: sanitize_text(&short_message),
                });
            }
        }
    }
    local_branches.sort_by(|a, b| b.is_head.cmp(&a.is_head).then_with(|| a.name.cmp(&b.name)));

    let mut remote_branches = Vec::new();
    if let Ok(branches) = repo.branches(Some(git2::BranchType::Remote)) {
        for (branch, _) in branches.flatten() {
            if let Ok(Some(name)) = branch.name() {
                if !name.ends_with("/HEAD") {
                    let is_head = branch.is_head();
                    let mut short_sha = String::new();
                    let mut short_message = String::new();
                    if let Ok(target) = branch.get().peel_to_commit() {
                        let id = target.id();
                        let id_str = id.to_string();
                        short_sha = safe_sha_slice(&id_str, 7).to_string();
                        if let Ok(Some(summary)) = target.summary() {
                            short_message = summary.to_string();
                        }
                    }
                    remote_branches.push(BranchInfo {
                        name: sanitize_text(name),
                        is_head,
                        short_sha,
                        short_message: sanitize_text(&short_message),
                    });
                }
            }
        }
    }
    remote_branches.sort_by(|a, b| a.name.cmp(&b.name));

    (Ok(local_branches), Ok(remote_branches))
}

pub fn load_tab_tags(
    repo_path: &Path,
) -> (Result<Vec<BranchInfo>, String>, Result<Vec<BranchInfo>, String>) {
    let repo = match Repository::open(repo_path) {
        Ok(r) => r,
        Err(e) => return (Err(e.to_string()), Err(e.to_string())),
    };

    let mut local_tags = Vec::new();
    if let Ok(tags) = repo.tag_names(None) {
        for tag_opt in tags.iter() {
            if let Ok(Some(tag)) = tag_opt {
                let mut short_sha = String::new();
                let mut short_message = String::new();
                if let Ok(reference) = repo.find_reference(&format!("refs/tags/{}", tag)) {
                    if let Ok(target) = reference.peel_to_commit() {
                        let id = target.id();
                        let id_str = id.to_string();
                        short_sha = safe_sha_slice(&id_str, 7).to_string();
                        if let Ok(Some(summary)) = target.summary() {
                            short_message = summary.to_string();
                        }
                    }
                }
                local_tags.push(BranchInfo {
                    name: sanitize_text(tag),
                    is_head: false,
                    short_sha,
                    short_message: sanitize_text(&short_message),
                });
            }
        }
    }
    local_tags.sort_by(|a, b| b.name.cmp(&a.name));

    (Ok(local_tags), Ok(Vec::new()))
}

pub fn load_tab_remotes(repo_path: &Path) -> Result<Vec<RemoteInfo>, String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let mut remotes_list = Vec::new();
    if let Ok(remotes) = repo.remotes() {
        for name in remotes.iter() {
            let Ok(Some(name)) = name else { continue };
            if let Ok(remote) = repo.find_remote(name) {
                let push_url = remote.pushurl().ok().flatten().map(String::from);
                let mut refspecs = Vec::new();
                for r in remote.refspecs() {
                    if let Ok(s) = r.str() {
                        refspecs.push(s.to_string());
                    }
                }
                remotes_list.push(RemoteInfo {
                    name: name.to_string(),
                    url: remote.url().unwrap_or("(no url)").to_string(),
                    push_url,
                    refspecs,
                });
            }
        }
    }
    Ok(remotes_list)
}

pub fn load_tab_stashes(repo_path: &Path) -> Result<Vec<StashInfo>, String> {
    let mut repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let mut temp_stashes = Vec::new();
    let _ = repo.stash_foreach(|index, message, oid| {
        temp_stashes.push((index, message.to_string(), *oid));
        true
    });

    let mut stashes = Vec::new();
    for (index, message, oid) in temp_stashes {
        let mut files = Vec::new();
        if let Ok(commit) = repo.find_commit(oid) {
            files = commit_changed_files(&repo, &commit);
        }
        stashes.push(StashInfo {
            index,
            message: sanitize_text(&message),
            commit_id: oid.to_string(),
            files,
        });
    }
    Ok(stashes)
}

pub fn load_tab_overview(
    repo_path: &Path,
    _commit_limit: usize,
) -> Result<(Vec<CommitterStat>, bool), String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let (stats, limit_reached) =
        collect_committer_stats(&repo, usize::MAX).map_err(|e| e.to_string())?;
    Ok((stats, limit_reached))
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

    if repo.head_detached().unwrap_or(false) {
        if let Ok(head) = repo.head() {
            if let Ok(target) = head.peel_to_commit() {
                let oid = target.id();
                let label = "head:HEAD".to_string();
                let mut entries = map.remove(&oid).unwrap_or_default();
                entries.insert(0, label);
                map.insert(oid, entries);
            }
        }
    }

    map
}

#[allow(clippy::type_complexity)]
static REF_MAP_CACHE: std::sync::OnceLock<
    std::sync::Mutex<
        std::collections::HashMap<
            String,
            (std::collections::HashMap<git2::Oid, Vec<String>>, std::time::Instant),
        >,
    >,
> = std::sync::OnceLock::new();

fn get_cached_ref_map(
    repo: &Repository,
    repo_path: &Path,
) -> std::collections::HashMap<git2::Oid, Vec<String>> {
    let cache_lock =
        REF_MAP_CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    let mut cache = cache_lock.lock().unwrap_or_else(|e| e.into_inner());
    let path_key = repo_path.to_string_lossy().to_string();

    if let Some((map, loaded_at)) = cache.get(&path_key) {
        if loaded_at.elapsed() < std::time::Duration::from_secs(10) {
            return map.clone();
        }
    }

    let map = build_ref_map(repo);
    cache.insert(path_key, (map.clone(), std::time::Instant::now()));
    map
}

pub fn invalidate_ref_map_cache(repo_path: &Path) {
    if let Some(cache_lock) = REF_MAP_CACHE.get() {
        if let Ok(mut cache) = cache_lock.lock() {
            cache.remove(&repo_path.to_string_lossy().to_string());
        }
    }
}

/// Maximum file entries collected per bucket. Prevents pathologically large
/// working trees from overwhelming the detail view.
const MAX_FILES_PER_SECTION: usize = 100;

/// Walk the working-tree status once and collect both summary counts and per-file info.
fn populate_summary_and_file_changes(repo: &Repository, info: &mut RepoInfo) {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .renames_head_to_index(true)
        .recurse_untracked_dirs(true)
        .show(StatusShow::IndexAndWorkdir);
    let Ok(statuses) = repo.statuses(Some(&mut opts)) else {
        return;
    };
    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("(unknown)").to_string();
        let flags = entry.status();

        // 1. Populate summary counters
        if flags.is_conflicted() {
            info.summary.conflicted += 1;
        } else {
            if flags.is_wt_new() {
                info.summary.untracked += 1;
            }
            if flags.is_wt_modified()
                || flags.is_wt_deleted()
                || flags.is_wt_renamed()
                || flags.is_wt_typechange()
            {
                info.summary.modified += 1;
            }
            if flags.is_index_new()
                || flags.is_index_modified()
                || flags.is_index_deleted()
                || flags.is_index_renamed()
                || flags.is_index_typechange()
            {
                info.summary.staged += 1;
            }
        }

        // 2. Populate file entries
        // Skip directories to avoid showing folders in staging panels.
        // We only call is_dir() for untracked files (since tracked items in git status are always files).
        let is_dir = if flags.is_wt_new() && !flags.is_index_new() {
            let path_buf = repo.workdir().unwrap_or(Path::new("")).join(&path);
            path_buf.is_dir()
        } else {
            false
        };
        if is_dir {
            continue;
        }

        if flags.is_conflicted() {
            if info.changes.conflicted.len() < MAX_FILES_PER_SECTION {
                info.changes.conflicted.push(FileEntry { path: path.clone(), label: "C" });
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
                "N"
            } else if flags.is_index_deleted() {
                "D"
            } else if flags.is_index_renamed() {
                "R"
            } else if flags.is_index_typechange() {
                "T"
            } else {
                "M"
            };
            info.changes.staged.push(FileEntry { path: path.clone(), label });
        }

        // Working-tree changes
        if flags.is_wt_new() {
            if info.changes.untracked.len() < MAX_FILES_PER_SECTION {
                info.changes.untracked.push(FileEntry { path: path.clone(), label: "?" });
            }
            if info.changes.unstaged.len() < MAX_FILES_PER_SECTION {
                info.changes.unstaged.push(FileEntry { path: path.clone(), label: "N" });
            }
        } else if (flags.is_wt_modified()
            || flags.is_wt_deleted()
            || flags.is_wt_renamed()
            || flags.is_wt_typechange())
            && info.changes.unstaged.len() < MAX_FILES_PER_SECTION
        {
            let label = if flags.is_wt_deleted() {
                "D"
            } else if flags.is_wt_renamed() {
                "R"
            } else if flags.is_wt_typechange() {
                "T"
            } else {
                "M"
            };
            info.changes.unstaged.push(FileEntry { path: path.clone(), label });
        }
    }
}

/// Collect the branch name, worktree counts, and ahead/behind for an opened
/// repo. Used by both `inspect_summary` (card) and `collect_info` (detail)
/// so the values shown in both places always agree.
fn collect_summary(repo: &Repository) -> RepoSummary {
    let mut s = RepoSummary::default();
    let is_detached = repo.head_detached().unwrap_or(false);
    if is_detached {
        s.branch = Some("HEAD".to_string());
        if let Ok(head) = repo.head() {
            if let Ok(commit) = head.peel_to_commit() {
                s.last_commit_time = Some(commit.time().seconds());
            }
        }
    } else if let Ok(head) = repo.head() {
        s.branch = head.shorthand().ok().map(String::from);
        if let Ok(commit) = head.peel_to_commit() {
            s.last_commit_time = Some(commit.time().seconds());
        }
    }
    populate_worktree(repo, &mut s);
    populate_ahead_behind(repo, &mut s);
    s.state = match repo.state() {
        git2::RepositoryState::Clean => RepoState::Clean,
        git2::RepositoryState::Merge => RepoState::Merge,
        git2::RepositoryState::Revert | git2::RepositoryState::RevertSequence => RepoState::Revert,
        git2::RepositoryState::CherryPick | git2::RepositoryState::CherryPickSequence => {
            RepoState::CherryPick
        }
        git2::RepositoryState::Bisect => RepoState::Bisect,
        git2::RepositoryState::Rebase
        | git2::RepositoryState::RebaseInteractive
        | git2::RepositoryState::RebaseMerge => RepoState::Rebase,
        git2::RepositoryState::ApplyMailbox | git2::RepositoryState::ApplyMailboxOrRebase => {
            RepoState::ApplyMailbox
        }
    };
    s
}

fn populate_worktree(repo: &Repository, s: &mut RepoSummary) {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).renames_head_to_index(true).show(StatusShow::IndexAndWorkdir);
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
pub fn format_relative_time(secs: i64) -> String {
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

/// Format a unix-epoch timestamp as a UTC date string ("YYYY-MM-DD HH:MM:SS UTC").
pub(crate) fn format_utc_date(secs: i64) -> String {
    if secs <= 0 {
        return "unknown".to_string();
    }
    let seconds_in_day = 86400;
    let day_number = secs / seconds_in_day;
    let time_of_day = secs % seconds_in_day;

    let mut hour = time_of_day / 3600;
    let mut minute = (time_of_day % 3600) / 60;
    let mut second = time_of_day % 60;
    if hour < 0 {
        hour += 24;
    }
    if minute < 0 {
        minute += 60;
    }
    if second < 0 {
        second += 60;
    }

    // Howard Hinnant's civil date from epoch days algorithm
    let z = day_number + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i32) + (era as i32) * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = y + if m <= 2 { 1 } else { 0 };

    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC", y, m, d, hour, minute, second)
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

    let diff =
        repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), Some(&mut opts)).ok()?;

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
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);
    opts.show_untracked_content(true);

    let diff = if staged {
        // Staged: diff HEAD tree (or empty tree for new repos) → index.
        let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
        repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts)).ok()?
    } else {
        // Unstaged: diff index → working directory.
        repo.diff_index_to_workdir(None, Some(&mut opts)).ok()?
    };

    collect_diff_lines(&diff)
}

/// Walk a libgit2 `Diff` and collect coloured `DiffLine` values.
fn collect_diff_lines(diff: &git2::Diff<'_>) -> Option<Vec<DiffLine>> {
    let mut lines: Vec<DiffLine> = Vec::new();
    let mut current_hunk_idx = None;
    let mut hunk_count = 0;
    diff.print(git2::DiffFormat::Patch, |_, _, line| {
        let kind = match line.origin() {
            '+' => DiffLineKind::Added,
            '-' => DiffLineKind::Removed,
            'H' => {
                current_hunk_idx = Some(hunk_count);
                hunk_count += 1;
                DiffLineKind::Header
            }
            ' ' => DiffLineKind::Context,
            _ => return true, // skip file-header meta lines
        };
        let content = String::from_utf8_lossy(line.content())
            .trim_end_matches('\n')
            .trim_end_matches('\r')
            .to_string();
        lines.push(DiffLine {
            kind,
            content,
            old_lineno: line.old_lineno(),
            new_lineno: line.new_lineno(),
            hunk_idx: current_hunk_idx,
        });
        true
    })
    .ok()?;
    Some(lines)
}

fn parse_graph_line(line: &str) -> GraphLine {
    if line.contains("__TWIG_SEP__") {
        let parts: Vec<&str> = line.split("__TWIG_SEP__").collect();
        if parts.len() >= 5 {
            let graph_and_hash = parts[0];
            let decoration = parts[1].trim().to_string();
            let summary = parts[2].trim().to_string();
            let author = parts[3].trim().to_string();
            let date = parts[4].trim().to_string();
            let signature_status =
                if parts.len() >= 6 { parts[5].trim().to_string() } else { "N".to_string() };

            let char_count = graph_and_hash.chars().count();
            if char_count >= 40 {
                let graph: String = graph_and_hash.chars().take(char_count - 40).collect();
                let oid: String = graph_and_hash.chars().skip(char_count - 40).collect();
                GraphLine {
                    graph,
                    commit: Some(GraphCommit {
                        oid,
                        decoration,
                        summary,
                        author,
                        date,
                        signature_status,
                    }),
                }
            } else {
                GraphLine { graph: graph_and_hash.to_string(), commit: None }
            }
        } else {
            GraphLine { graph: line.to_string(), commit: None }
        }
    } else {
        GraphLine { graph: line.to_string(), commit: None }
    }
}

#[allow(dead_code)]
fn collect_graph_lines(repo_path: &Path, graph_max_commits: usize) -> Vec<GraphLine> {
    let mut graph_lines = Vec::new();
    let format_str = "%H__TWIG_SEP__%d__TWIG_SEP__%s__TWIG_SEP__%an__TWIG_SEP__%ad__TWIG_SEP__%G?";

    let mut args = vec![
        "log".to_string(),
        "--graph".to_string(),
        "--all".to_string(),
        "--date=relative".to_string(),
    ];
    if graph_max_commits > 0 {
        args.push(format!("--max-count={}", graph_max_commits));
    }
    args.push(format!("--pretty=format:{}", format_str));
    args.push("--color=never".to_string());

    let output = git_command().args(&args).current_dir(repo_path).output();

    if let Ok(out) = output {
        if out.status.success() {
            let stdout_str = String::from_utf8_lossy(&out.stdout);
            for line in stdout_str.lines() {
                graph_lines.push(parse_graph_line(line));
            }
        }
    }
    graph_lines
}

pub fn delete_stash(repo_path: &Path, index: usize) -> Result<(), git2::Error> {
    let mut repo = Repository::open(repo_path)?;
    repo.stash_drop(index)?;
    Ok(())
}

pub fn apply_stash(repo_path: &Path, index: usize) -> Result<(), String> {
    let stash_ref = format!("stash@{{{}}}", index);
    let output = git_command()
        .arg("stash")
        .arg("apply")
        .arg(&stash_ref)
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(err_msg);
    }
    Ok(())
}

pub fn get_latest_change_time(item: &str) -> u64 {
    let path = expand_tilde(item);
    if !path.exists() {
        return 0;
    }

    if path.join(".git").exists() {
        if let Ok(repo) = Repository::open(&path) {
            if let Ok(head) = repo.head() {
                if let Ok(commit) = head.peel_to_commit() {
                    return commit.time().seconds() as u64;
                }
            }
        }
    }

    if let Ok(meta) = std::fs::metadata(&path) {
        if let Ok(modified) = meta.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                return duration.as_secs();
            }
        }
    }
    0
}

pub fn get_last_commit_message(repo_path: &Path) -> Option<String> {
    if let Ok(repo) = Repository::open(repo_path) {
        if let Ok(head) = repo.head() {
            if let Ok(commit) = head.peel_to_commit() {
                if let Ok(msg) = commit.message() {
                    return Some(msg.to_string());
                }
            }
        }
    }
    None
}

pub fn commit_amend(repo_path: &Path, message: &str) -> Result<(), String> {
    run_git_commit(repo_path, message, true)
}
