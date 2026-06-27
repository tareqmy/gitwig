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
#![warn(clippy::all, clippy::perf, clippy::nursery)]
#![warn(clippy::unwrap_used, clippy::panic)]

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
    Branches { local: Result<Vec<BranchInfo>, String>, remote: Result<Vec<BranchInfo>, String> },
    Tags { local: Result<Vec<BranchInfo>, String>, remote: Result<Vec<BranchInfo>, String> },
    Remotes(Result<Vec<RemoteInfo>, String>),
    Stashes(Result<Vec<StashInfo>, String>),
    Overview(Result<(Vec<CommitterStat>, bool), String>),
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
    /// Committer statistics.
    pub committer_stats: TabData<Vec<CommitterStat>>,
    /// Whether the committer statistics walk was capped by the limit.
    pub committer_stats_limit_reached: bool,
    /// Timestamps when each tab was loaded (index matches tab_idx)
    pub tab_loaded_at: [Option<std::time::Instant>; 8],
    /// Whether each tab is currently loading in the background (index matches tab_idx)
    pub tab_loading: [bool; 8],
}

#[derive(Debug, Clone)]
pub struct HeadInfo {
    pub short_id: String,
    pub summary: String,
    pub author: String,
    pub when: String,
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

/// Stage all unstaged/untracked changes (equivalent to `git add -A`).
pub fn stage_all_changes(repo_path: &Path) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
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
    let output = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .arg("reset")
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Discard all staged, unstaged, and untracked changes in the repository.
pub fn discard_all_changes(repo_path: &Path) -> Result<(), String> {
    // 1. Unstage all first so everything is in the working tree
    let _ = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .arg("reset")
        .current_dir(repo_path)
        .output();

    // 2. Discard all tracked modifications
    let checkout_out = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
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
    let clean_out = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
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
    apply_hunk_patch(repo_path, file_path, hunk, false, true)
}

/// Unstage a single hunk of staged changes (equivalent to `git apply --cached --reverse -`).
pub fn unstage_hunk(repo_path: &Path, file_path: &str, hunk: &[DiffLine]) -> Result<(), String> {
    apply_hunk_patch(repo_path, file_path, hunk, true, true)
}

/// Discard a single hunk of unstaged changes in the working tree (equivalent to `git apply --reverse -`).
pub fn discard_hunk(repo_path: &Path, file_path: &str, hunk: &[DiffLine]) -> Result<(), String> {
    apply_hunk_patch(repo_path, file_path, hunk, true, false)
}

/// Stage a single line from the Unstaged diff.
pub fn stage_line(
    repo_path: &Path,
    file_path: &str,
    hunk: &[DiffLine],
    selected_line_idx: usize,
) -> Result<(), String> {
    apply_line_patch_inner(repo_path, file_path, hunk, selected_line_idx, false, false, true)
}

/// Unstage a single line from the Staged diff.
pub fn unstage_line(
    repo_path: &Path,
    file_path: &str,
    hunk: &[DiffLine],
    selected_line_idx: usize,
) -> Result<(), String> {
    apply_line_patch_inner(repo_path, file_path, hunk, selected_line_idx, true, true, true)
}

/// Discard a single line from the Unstaged diff in the working tree.
pub fn discard_line(
    repo_path: &Path,
    file_path: &str,
    hunk: &[DiffLine],
    selected_line_idx: usize,
) -> Result<(), String> {
    apply_line_patch_inner(repo_path, file_path, hunk, selected_line_idx, true, true, false)
}

fn parse_hunk_header(header: &str) -> Option<(usize, usize, usize, usize)> {
    if !header.starts_with("@@") {
        return None;
    }
    let parts: Vec<&str> = header.split("@@").collect();
    if parts.len() < 3 {
        return None;
    }
    let meta = parts[1].trim();
    let subparts: Vec<&str> = meta.split_whitespace().collect();
    if subparts.len() < 2 {
        return None;
    }

    let parse_part = |p: &str| -> (usize, usize) {
        let s = p.trim_start_matches(['-', '+']);
        let comps: Vec<&str> = s.split(',').collect();
        let start = comps[0].parse::<usize>().unwrap_or(0);
        let count = if comps.len() > 1 { comps[1].parse::<usize>().unwrap_or(1) } else { 1 };
        (start, count)
    };

    let (old_start, old_count) = parse_part(subparts[0]);
    let (new_start, new_count) = parse_part(subparts[1]);
    Some((old_start, old_count, new_start, new_count))
}

fn apply_line_patch_inner(
    repo_path: &Path,
    file_path: &str,
    hunk: &[DiffLine],
    selected_line_idx_in_hunk: usize,
    revert: bool,
    target_has_modification: bool,
    cached: bool,
) -> Result<(), String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    if hunk.is_empty() {
        return Err("Empty hunk".to_string());
    }

    let selected_line = match hunk.get(selected_line_idx_in_hunk) {
        Some(line) => line,
        None => return Err("Invalid line index".to_string()),
    };

    if selected_line.kind != DiffLineKind::Added && selected_line.kind != DiffLineKind::Removed {
        return Err("Selected line is not a modification (must be + or -)".to_string());
    }

    let header_line = &hunk[0];
    let (old_start, _old_count, new_start, _new_count) =
        match parse_hunk_header(&header_line.content) {
            Some(coords) => coords,
            None => return Err(format!("Invalid hunk header: {}", header_line.content)),
        };

    let mut patch_lines = Vec::new();
    let mut new_old_count = 0;
    let mut new_new_count = 0;

    for (i, line) in hunk.iter().enumerate() {
        if i == 0 {
            continue;
        }

        if i == selected_line_idx_in_hunk {
            if revert {
                match line.kind {
                    DiffLineKind::Added => {
                        patch_lines.push(DiffLine {
                            kind: DiffLineKind::Removed,
                            content: line.content.clone(),
                        });
                        new_old_count += 1;
                    }
                    DiffLineKind::Removed => {
                        patch_lines.push(DiffLine {
                            kind: DiffLineKind::Added,
                            content: line.content.clone(),
                        });
                        new_new_count += 1;
                    }
                    _ => {}
                }
            } else {
                match line.kind {
                    DiffLineKind::Added => {
                        patch_lines.push(DiffLine {
                            kind: DiffLineKind::Added,
                            content: line.content.clone(),
                        });
                        new_new_count += 1;
                    }
                    DiffLineKind::Removed => {
                        patch_lines.push(DiffLine {
                            kind: DiffLineKind::Removed,
                            content: line.content.clone(),
                        });
                        new_old_count += 1;
                    }
                    _ => {}
                }
            }
        } else {
            match line.kind {
                DiffLineKind::Context => {
                    patch_lines.push(line.clone());
                    new_old_count += 1;
                    new_new_count += 1;
                }
                DiffLineKind::Added => {
                    if target_has_modification {
                        patch_lines.push(DiffLine {
                            kind: DiffLineKind::Context,
                            content: line.content.clone(),
                        });
                        new_old_count += 1;
                        new_new_count += 1;
                    } else {
                        // Omit
                    }
                }
                DiffLineKind::Removed => {
                    if target_has_modification {
                        // Omit
                    } else {
                        patch_lines.push(DiffLine {
                            kind: DiffLineKind::Context,
                            content: line.content.clone(),
                        });
                        new_old_count += 1;
                        new_new_count += 1;
                    }
                }
                _ => {}
            }
        }
    }

    let mut patch = String::new();
    patch.push_str(&format!("diff --git a/{} b/{}\n", file_path, file_path));
    patch.push_str(&format!("--- a/{}\n", file_path));
    patch.push_str(&format!("+++ b/{}\n", file_path));
    patch.push_str(&format!(
        "@@ -{},{} +{},{} @@\n",
        old_start, new_old_count, new_start, new_new_count
    ));

    for line in patch_lines {
        let prefix = match line.kind {
            DiffLineKind::Added => "+",
            DiffLineKind::Removed => "-",
            DiffLineKind::Context => " ",
            DiffLineKind::Header => "",
            _ => "",
        };
        patch.push_str(prefix);
        patch.push_str(&line.content);
        patch.push('\n');
    }

    let mut args = vec!["apply"];
    if cached {
        args.push("--cached");
    }
    args.push("-");

    let mut cmd = Command::new("git");
    let mut child = cmd
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .args(&args)
        .current_dir(repo_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn git apply: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(patch.as_bytes())
            .map_err(|e| format!("Failed to write patch to stdin: {}", e))?;
    }

    let output =
        child.wait_with_output().map_err(|e| format!("Failed to wait for git apply: {}", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("git apply failed: {}", err_msg.trim()));
    }

    Ok(())
}

fn apply_hunk_patch(
    repo_path: &Path,
    file_path: &str,
    hunk: &[DiffLine],
    reverse: bool,
    cached: bool,
) -> Result<(), String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut patch = String::new();
    patch.push_str(&format!("diff --git a/{} b/{}\n", file_path, file_path));
    patch.push_str(&format!("--- a/{}\n", file_path));
    patch.push_str(&format!("+++ b/{}\n", file_path));
    for line in hunk {
        let prefix = match line.kind {
            DiffLineKind::Added => "+",
            DiffLineKind::Removed => "-",
            DiffLineKind::Context => " ",
            DiffLineKind::Header => "",
            _ => "",
        };
        patch.push_str(prefix);
        patch.push_str(&line.content);
        patch.push('\n');
    }

    let mut args = vec!["apply"];
    if cached {
        args.push("--cached");
    }
    if reverse {
        args.push("--reverse");
    }
    args.push("-");

    let mut cmd = Command::new("git");
    let mut child = cmd
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .args(&args)
        .current_dir(repo_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn git apply: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(patch.as_bytes())
            .map_err(|e| format!("Failed to write patch to stdin: {}", e))?;
    }

    let output =
        child.wait_with_output().map_err(|e| format!("Failed to wait for git apply: {}", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("git apply failed: {}", err_msg.trim()));
    }

    Ok(())
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

/// Create a commit in the repository with the given message.
/// Returns a human-readable error string on failure.
pub fn commit_changes(repo_path: &Path, message: &str) -> Result<(), String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let mut index = repo.index().map_err(|e| e.to_string())?;
    let tree_id = index.write_tree().map_err(|e| e.to_string())?;
    let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;

    let signature = repo
        .signature()
        .map_err(|e| format!("Failed to get signature. Check user.name/email config: {}", e))?;

    // Find parent commits
    let mut parents = Vec::new();
    let mut has_head = false;
    if let Ok(head) = repo.head() {
        if let Ok(parent_commit) = head.peel_to_commit() {
            has_head = true;
            // Check if there are changes staged compared to HEAD
            let parent_tree = parent_commit.tree().map_err(|e| e.to_string())?;
            if parent_tree.id() == tree_id {
                return Err("No staged changes to commit".to_string());
            }
            parents.push(parent_commit);
        }
    }

    if !has_head && index.is_empty() {
        return Err("No staged changes to commit (index is empty)".to_string());
    }

    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

    repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &parent_refs)
        .map_err(|e| e.to_string())?;

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
    let mut cmd = std::process::Command::new("git");
    cmd.env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .arg("log")
        .arg("--all");

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

    for id in oids {
        let oid = id?;
        if let Ok(commit) = repo.find_commit(oid) {
            let short_id = format!("{:.7}", commit.id());
            let oid_str = commit.id().to_string();
            let summary =
                commit.summary().ok().flatten().unwrap_or("(no commit message)").to_string();
            let author = commit.author();
            let author_name = author.name().unwrap_or("?");
            let author_email = author.email().unwrap_or("?");
            let author_str = format!("{} <{}>", author_name, author_email);
            let when = format_relative_time(commit.time().seconds());
            let date = format_utc_date(commit.time().seconds());
            let refs = ref_map.get(&oid).cloned().unwrap_or_default();
            let files = Vec::new();
            let message = commit.message().unwrap_or("(no commit message)").to_string();
            let sig_status = sig_map.get(&oid_str).cloned().unwrap_or_else(|| "N".to_string());
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

pub fn get_commit_files(repo_path: &Path, oid: &str) -> Result<Vec<FileEntry>, String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let oid = git2::Oid::from_str(oid).map_err(|e| e.to_string())?;
    let commit = repo.find_commit(oid).map_err(|e| e.to_string())?;
    Ok(commit_changed_files(&repo, &commit))
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

    if let Ok(head) = repo.head() {
        info.branch = head.shorthand().ok().map(String::from);

        if let Ok(commit) = head.peel_to_commit() {
            let short_id = format!("{:.7}", commit.id());
            let summary_text =
                commit.summary().ok().flatten().unwrap_or("(no commit message)").to_string();
            let author = commit.author();
            let author_str =
                format!("{} <{}>", author.name().unwrap_or("?"), author.email().unwrap_or("?"));
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

pub fn load_tab_graph_stream(
    repo_path: &Path,
    graph_max_commits: usize,
    repo_resolved_path: String,
    tab_idx: usize,
    tx: std::sync::mpsc::Sender<(String, usize, TabPayload)>,
) -> Result<Vec<GraphLine>, String> {
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

    let mut child = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .args(&args)
        .current_dir(repo_path)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

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
    if !status.success() && graph_lines.is_empty() {
        return Err("git log failed".to_string());
    }

    Ok(graph_lines)
}

pub fn load_tab_branches(
    repo_path: &Path,
) -> (Result<Vec<BranchInfo>, String>, Result<Vec<BranchInfo>, String>) {
    let repo = match Repository::open(repo_path) {
        Ok(r) => r,
        Err(e) => return (Err(e.to_string()), Err(e.to_string())),
    };

    let mut local_branches = Vec::new();
    if let Ok(branches) = repo.branches(Some(git2::BranchType::Local)) {
        for (branch, _) in branches.flatten() {
            if let Ok(Some(name)) = branch.name() {
                let is_head = branch.is_head();
                let mut short_sha = String::new();
                let mut short_message = String::new();
                if let Ok(target) = branch.get().peel_to_commit() {
                    let id = target.id();
                    short_sha = id.to_string()[..7.min(id.to_string().len())].to_string();
                    if let Ok(Some(summary)) = target.summary() {
                        short_message = summary.to_string();
                    }
                }
                local_branches.push(BranchInfo {
                    name: name.to_string(),
                    is_head,
                    short_sha,
                    short_message,
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
                        short_sha = id.to_string()[..7.min(id.to_string().len())].to_string();
                        if let Ok(Some(summary)) = target.summary() {
                            short_message = summary.to_string();
                        }
                    }
                    remote_branches.push(BranchInfo {
                        name: name.to_string(),
                        is_head,
                        short_sha,
                        short_message,
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
                        short_sha = id.to_string()[..7.min(id.to_string().len())].to_string();
                        if let Ok(Some(summary)) = target.summary() {
                            short_message = summary.to_string();
                        }
                    }
                }
                local_tags.push(BranchInfo {
                    name: tag.to_string(),
                    is_head: false,
                    short_sha,
                    short_message,
                });
            }
        }
    }
    local_tags.sort_by(|a, b| a.name.cmp(&b.name));

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
        stashes.push(StashInfo { index, message, commit_id: oid.to_string(), files });
    }
    Ok(stashes)
}

pub fn load_tab_overview(
    repo_path: &Path,
    commit_limit: usize,
) -> Result<(Vec<CommitterStat>, bool), String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let stats_limit = if commit_limit > 0 { commit_limit.min(10000) } else { 10000 };
    let (stats, limit_reached) =
        collect_committer_stats(&repo, stats_limit).map_err(|e| e.to_string())?;
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
    let mut cache = cache_lock.lock().unwrap();
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
        // Skip directories to avoid showing folders in staging panels
        let path_buf = repo.workdir().unwrap_or(Path::new("")).join(&path);
        if path_buf.is_dir() {
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

/// Format a unix-epoch timestamp as a UTC date string ("YYYY-MM-DD HH:MM:SS UTC").
fn format_utc_date(secs: i64) -> String {
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

    let output = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .args(&args)
        .current_dir(repo_path)
        .output();

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

pub fn checkout_local_branch(repo_path: &Path, branch_name: &str) -> Result<(), git2::Error> {
    let output = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .arg("checkout")
        .arg(branch_name)
        .current_dir(repo_path)
        .output()
        .map_err(|e| git2::Error::from_str(&e.to_string()))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(git2::Error::from_str(&err));
    }
    Ok(())
}

pub fn checkout_remote_branch(
    repo_path: &Path,
    remote_branch_name: &str,
) -> Result<String, git2::Error> {
    let parts: Vec<&str> = remote_branch_name.splitn(2, '/').collect();
    if parts.len() < 2 {
        return Err(git2::Error::from_str("Invalid remote branch name"));
    }
    let local_name = parts[1];

    let output = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .arg("checkout")
        .arg(local_name)
        .current_dir(repo_path)
        .output()
        .map_err(|e| git2::Error::from_str(&e.to_string()))?;

    if output.status.success() {
        return Ok(format!("Switched to existing branch '{}'", local_name));
    }

    let output = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .arg("checkout")
        .arg("--track")
        .arg(remote_branch_name)
        .current_dir(repo_path)
        .output()
        .map_err(|e| git2::Error::from_str(&e.to_string()))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(git2::Error::from_str(&err));
    }

    Ok(format!("Created and switched to branch '{}' tracking '{}'", local_name, remote_branch_name))
}

/// Creates a new local branch pointing at HEAD.
pub fn create_branch(repo_path: &Path, branch_name: &str) -> Result<(), git2::Error> {
    let repo = Repository::open(repo_path)?;
    let head = repo.head()?;
    let target_commit = head.peel_to_commit()?;
    repo.branch(branch_name, &target_commit, false)?;
    Ok(())
}

/// Deletes a local branch.
pub fn delete_local_branch(repo_path: &Path, branch_name: &str) -> Result<(), git2::Error> {
    let repo = Repository::open(repo_path)?;
    let mut branch = repo.find_branch(branch_name, git2::BranchType::Local)?;
    branch.delete()?;
    Ok(())
}

/// Deletes a remote-tracking branch locally.
pub fn delete_remote_branch(repo_path: &Path, branch_name: &str) -> Result<(), git2::Error> {
    let repo = Repository::open(repo_path)?;
    let mut branch = repo.find_branch(branch_name, git2::BranchType::Remote)?;
    branch.delete()?;
    Ok(())
}

/// Creates a new lightweight tag pointing at the specified commit OID.
pub fn create_tag(
    repo_path: &Path,
    tag_name: &str,
    commit_oid_str: &str,
) -> Result<(), git2::Error> {
    let repo = Repository::open(repo_path)?;
    let oid = git2::Oid::from_str(commit_oid_str)?;
    let target_object = repo.find_object(oid, Some(git2::ObjectType::Commit))?;
    repo.tag_lightweight(tag_name, &target_object, false)?;
    Ok(())
}

/// Deletes a local tag.
pub fn delete_tag(repo_path: &Path, tag_name: &str) -> Result<(), git2::Error> {
    let repo = Repository::open(repo_path)?;
    repo.tag_delete(tag_name)?;
    Ok(())
}

/// Deletes a tag on the remote.
pub fn delete_remote_tag(
    repo_path: &Path,
    remote_name: &str,
    tag_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .arg("push")
        .arg(remote_name)
        .arg("--delete")
        .arg(tag_name)
        .current_dir(repo_path)
        .output()?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(err.into());
    }
    Ok(())
}

pub fn checkout_tag(repo_path: &Path, tag_name: &str) -> Result<(), git2::Error> {
    let output = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .arg("checkout")
        .arg(tag_name)
        .current_dir(repo_path)
        .output()
        .map_err(|e| git2::Error::from_str(&e.to_string()))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(git2::Error::from_str(&err));
    }
    Ok(())
}

/// Helper to run `git ls-remote --tags` and return parsed tag information.
pub fn get_remote_tags(
    repo_path: &Path,
    remote_name: &str,
) -> Result<Vec<BranchInfo>, Box<dyn std::error::Error>> {
    let output = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .arg("ls-remote")
        .arg("--tags")
        .arg(remote_name)
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(err.into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let repo = git2::Repository::open(repo_path)?;
    let mut tags_map = std::collections::HashMap::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let sha = parts[0];
            let ref_name = parts[1];
            if ref_name.starts_with("refs/tags/") {
                let is_peeled = ref_name.ends_with("^{}");
                let clean_ref = if is_peeled { &ref_name[..ref_name.len() - 3] } else { ref_name };
                let tag_name = clean_ref.strip_prefix("refs/tags/").unwrap_or(clean_ref);
                let short_sha = if sha.len() >= 7 { &sha[..7] } else { sha };

                // Try to resolve the summary locally
                let mut short_message = String::new();
                if let Ok(oid) = git2::Oid::from_str(sha) {
                    if let Ok(commit) = repo.find_commit(oid) {
                        if let Ok(Some(summary)) = commit.summary() {
                            short_message = summary.to_string();
                        }
                    }
                }
                if short_message.is_empty() {
                    short_message = "(not fetched)".to_string();
                }

                if is_peeled {
                    tags_map.insert(tag_name.to_string(), (short_sha.to_string(), short_message));
                } else {
                    tags_map
                        .entry(tag_name.to_string())
                        .or_insert_with(|| (short_sha.to_string(), short_message));
                }
            }
        }
    }

    let mut tags = Vec::new();
    for (name, (short_sha, short_message)) in tags_map {
        tags.push(BranchInfo { name, is_head: false, short_sha, short_message });
    }
    tags.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(tags)
}

pub fn serialize_tags(tags: &[BranchInfo]) -> String {
    let mut s = String::new();
    for tag in tags {
        s.push_str(&format!("{}|{}|{}\n", tag.name, tag.short_sha, tag.short_message));
    }
    s
}

pub fn deserialize_tags(s: &str) -> Vec<BranchInfo> {
    let mut tags = Vec::new();
    for line in s.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 3 {
            tags.push(BranchInfo {
                name: parts[0].to_string(),
                is_head: false,
                short_sha: parts[1].to_string(),
                short_message: parts[2].to_string(),
            });
        }
    }
    tags
}

pub fn delete_stash(repo_path: &Path, index: usize) -> Result<(), git2::Error> {
    let mut repo = Repository::open(repo_path)?;
    repo.stash_drop(index)?;
    Ok(())
}

pub fn apply_stash(repo_path: &Path, index: usize) -> Result<(), String> {
    let stash_ref = format!("stash@{{{}}}", index);
    let output = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
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

pub fn save_stash(repo_path: &Path, message: &str) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .arg("stash")
        .arg("push")
        .arg("-m")
        .arg(message)
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
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let head = repo.head().map_err(|e| format!("No HEAD commit to amend: {}", e))?;
    let head_commit = head.peel_to_commit().map_err(|e| e.to_string())?;

    let mut index = repo.index().map_err(|e| e.to_string())?;
    let tree_id = index.write_tree().map_err(|e| e.to_string())?;
    let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;

    let signature = repo
        .signature()
        .map_err(|e| format!("Failed to get signature. Check user.name/email config: {}", e))?;

    head_commit
        .amend(Some("HEAD"), None, Some(&signature), None, Some(message), Some(&tree))
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ── Merge Conflict Helpers ──────────────────────────────────────────────────

/// Returns `true` when `.git/MERGE_HEAD` exists — i.e. a merge is in progress.
/// Cheap file-existence check, no libgit2 required.
pub fn is_merging(repo_path: &Path) -> bool {
    repo_path.join(".git/MERGE_HEAD").exists()
}

/// Returns the conflict-marker diff for a conflicted file by parsing the file on disk.
/// Colorizes conflict blocks using DiffLineKind variants.
pub fn get_conflict_markers_diff(repo_path: &Path, file_path: &str) -> Vec<DiffLine> {
    let full_path = repo_path.join(file_path);
    let content = match std::fs::read_to_string(&full_path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut lines = Vec::new();
    let mut in_ours = false;
    let mut in_theirs = false;

    for line in content.lines() {
        if line.starts_with("<<<<<<<") {
            in_ours = true;
            in_theirs = false;
            lines.push(DiffLine {
                kind: DiffLineKind::ConflictSeparator,
                content: line.to_string(),
            });
        } else if line.starts_with("=======") {
            in_ours = false;
            in_theirs = true;
            lines.push(DiffLine {
                kind: DiffLineKind::ConflictSeparator,
                content: line.to_string(),
            });
        } else if line.starts_with(">>>>>>>") {
            in_ours = false;
            in_theirs = false;
            lines.push(DiffLine {
                kind: DiffLineKind::ConflictSeparator,
                content: line.to_string(),
            });
        } else if in_ours {
            lines.push(DiffLine { kind: DiffLineKind::ConflictOurs, content: line.to_string() });
        } else if in_theirs {
            lines.push(DiffLine { kind: DiffLineKind::ConflictTheirs, content: line.to_string() });
        } else {
            lines.push(DiffLine { kind: DiffLineKind::Context, content: line.to_string() });
        }
    }
    lines
}

/// Accept the OURS (HEAD) version of a conflicted file.
/// Equivalent to: git checkout --ours <file> && git add <file>
pub fn resolve_ours(repo_path: &Path, file_path: &str) -> Result<(), String> {
    let output1 = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .args(["checkout", "--ours", file_path])
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;
    if !output1.status.success() {
        return Err(String::from_utf8_lossy(&output1.stderr).to_string());
    }
    stage_file(repo_path, file_path)?;
    Ok(())
}

/// Accept the THEIRS (incoming) version of a conflicted file.
/// Equivalent to: git checkout --theirs <file> && git add <file>
pub fn resolve_theirs(repo_path: &Path, file_path: &str) -> Result<(), String> {
    let output1 = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .args(["checkout", "--theirs", file_path])
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;
    if !output1.status.success() {
        return Err(String::from_utf8_lossy(&output1.stderr).to_string());
    }
    stage_file(repo_path, file_path)?;
    Ok(())
}

/// Mark the file as resolved (stage it) after manual edits.
pub fn mark_resolved(repo_path: &Path, file_path: &str) -> Result<(), String> {
    stage_file(repo_path, file_path)
}

/// Resolve a specific conflict hunk inside a file (Ours vs Theirs).
/// Replaces the hunk at index `hunk_idx` in the file on disk.
/// If no more conflicts remain in the file, it automatically stages the file.
pub fn resolve_conflict_hunk(
    repo_path: &Path,
    file_path: &str,
    hunk_idx: usize,
    accept_ours: bool,
) -> Result<(), String> {
    let full_path = repo_path.join(file_path);
    let content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;

    let mut new_lines = Vec::new();
    let mut lines_iter = content.lines().peekable();
    let mut current_hunk_idx = 0;

    while let Some(line) = lines_iter.next() {
        if line.starts_with("<<<<<<<") {
            let mut ours_block = Vec::new();
            let mut theirs_block = Vec::new();

            // Read ours block (until =======)
            let mut found_separator = false;
            while let Some(next_line) = lines_iter.peek() {
                if next_line.starts_with("=======") {
                    lines_iter.next(); // consume =======
                    found_separator = true;
                    break;
                }
                ours_block.push(lines_iter.next().unwrap().to_string());
            }

            // Read theirs block (until >>>>>>>)
            let mut found_end = false;
            let mut end_line_marker = ">>>>>>>".to_string();
            while let Some(next_line) = lines_iter.peek() {
                if next_line.starts_with(">>>>>>>") {
                    end_line_marker = lines_iter.next().unwrap().to_string(); // consume >>>>>>>
                    found_end = true;
                    break;
                }
                theirs_block.push(lines_iter.next().unwrap().to_string());
            }

            if current_hunk_idx == hunk_idx {
                if accept_ours {
                    new_lines.extend(ours_block);
                } else {
                    new_lines.extend(theirs_block);
                }
            } else {
                new_lines.push(line.to_string());
                new_lines.extend(ours_block);
                if found_separator {
                    new_lines.push("=======".to_string());
                }
                new_lines.extend(theirs_block);
                if found_end {
                    new_lines.push(end_line_marker);
                }
            }

            current_hunk_idx += 1;
        } else {
            new_lines.push(line.to_string());
        }
    }

    let mut new_content = new_lines.join("\n");
    if content.ends_with('\n') && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    std::fs::write(&full_path, new_content).map_err(|e| e.to_string())?;

    // Check if any conflict markers remain in the file
    let updated_content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;
    let has_conflict_markers = updated_content
        .lines()
        .any(|l| l.starts_with("<<<<<<<") || l.starts_with("=======") || l.starts_with(">>>>>>>"));

    if !has_conflict_markers {
        stage_file(repo_path, file_path)?;
    }

    Ok(())
}

/// Abort the in-progress merge.
pub fn abort_merge(repo_path: &Path) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .args(["merge", "--abort"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

/// Continue the merge after conflicts are resolved.
pub fn continue_merge(repo_path: &Path) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
        .args(["merge", "--continue"])
        .env("GIT_EDITOR", "true")
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_commit_amend() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push(format!(
            "twig_test_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&temp_path).unwrap();

        // Init repo
        let repo = Repository::init(&temp_path).unwrap();

        // Configure author
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Create initial file
        let file_path = temp_path.join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "initial content").unwrap();

        // Stage and commit initial
        stage_file(&temp_path, "test.txt").unwrap();
        commit_changes(&temp_path, "initial commit").unwrap();

        // Verify message
        let msg = get_last_commit_message(&temp_path).unwrap();
        assert_eq!(msg, "initial commit");

        // Amend the commit message
        commit_amend(&temp_path, "amended commit").unwrap();

        // Verify amended message
        let amended_msg = get_last_commit_message(&temp_path).unwrap();
        assert_eq!(amended_msg, "amended commit");

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_path);
    }

    #[test]
    fn test_commit_signatures_collection() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push(format!(
            "twig_test_sig_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&temp_path).unwrap();

        // Init repo
        let repo = Repository::init(&temp_path).unwrap();

        // Configure author
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Create initial file
        let file_path = temp_path.join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "initial content").unwrap();

        // Stage and commit initial
        stage_file(&temp_path, "test.txt").unwrap();
        commit_changes(&temp_path, "initial commit").unwrap();

        // 1. Test collect_signatures
        let sigs = collect_signatures(&temp_path, 0);
        assert_eq!(sigs.len(), 1);
        let head_oid = repo.head().unwrap().target().unwrap().to_string();
        let sig_status = sigs.get(&head_oid).unwrap();
        assert_eq!(sig_status, "N");

        // 2. Test collect_commits (files should be empty by default)
        let commits = collect_commits(&repo, 0, &temp_path, true).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].signature_status, "N");
        assert!(commits[0].files.is_empty());

        // Test get_commit_files (lazy loading)
        let files = get_commit_files(&temp_path, &commits[0].oid).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "test.txt");
        assert_eq!(files[0].label, "N");

        // 3. Test collect_graph_lines
        let graph = collect_graph_lines(&temp_path, 1000);
        assert_eq!(graph.len(), 1);
        assert!(graph[0].commit.is_some());
        assert_eq!(graph[0].commit.as_ref().unwrap().signature_status, "N");

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_path);
    }

    #[test]
    fn test_ref_map_cache_behavior() {
        let temp_dir = std::env::temp_dir();
        let repo_path = temp_dir.join("test_ref_map_repo");
        let _ = std::fs::remove_dir_all(&repo_path);
        std::fs::create_dir_all(&repo_path).unwrap();

        let repo = Repository::init(&repo_path).unwrap();

        // 1. First fetch (rebuilds and caches)
        let map1 = get_cached_ref_map(&repo, &repo_path);

        // 2. Second fetch (returns cached map)
        let map2 = get_cached_ref_map(&repo, &repo_path);
        assert_eq!(map1.len(), map2.len());

        // 3. Invalidate cache
        invalidate_ref_map_cache(&repo_path);

        // Clean up
        let _ = std::fs::remove_dir_all(&repo_path);
    }

    #[test]
    fn test_get_latest_change_time() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push(format!(
            "twig_test_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&temp_path).unwrap();

        let change_time = get_latest_change_time(temp_path.to_str().unwrap());
        assert!(change_time > 0);

        let _ = std::fs::remove_dir_all(&temp_path);
    }

    #[test]
    fn test_committer_stats() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push(format!(
            "twig_test_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&temp_path).unwrap();

        // Init repo
        let repo = Repository::init(&temp_path).unwrap();

        // Configure author
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Create initial file
        let file_path = temp_path.join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "initial content").unwrap();

        // Stage and commit initial
        stage_file(&temp_path, "test.txt").unwrap();
        commit_changes(&temp_path, "initial commit").unwrap();

        // Collect stats
        let (stats, limit_reached) = collect_committer_stats(&repo, 10).unwrap();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].name, "Test User");
        assert_eq!(stats[0].email, "test@example.com");
        assert_eq!(stats[0].count, 1);
        assert!(!limit_reached);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_path);
    }

    #[test]
    fn test_untracked_files_in_unstaged() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push(format!(
            "twig_test_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&temp_path).unwrap();

        // Init repo
        let _repo = Repository::init(&temp_path).unwrap();

        // Create an untracked file
        let file_path = temp_path.join("untracked.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "hello untracked").unwrap();

        // Create an untracked directory and a file inside it
        let untracked_dir = temp_path.join("untracked_dir");
        std::fs::create_dir_all(&untracked_dir).unwrap();
        let nested_file_path = untracked_dir.join("nested.txt");
        std::fs::write(&nested_file_path, "nested untracked file").unwrap();

        // Inspect detail
        let detail = inspect_detail(temp_path.to_str().unwrap(), 0, 1000, false);
        match detail {
            ItemDetail::Repo { info, .. } => {
                // Verify no folders are in the unstaged/untracked list
                let unstaged_paths: Vec<String> =
                    info.changes.unstaged.iter().map(|f| f.path.clone()).collect();
                let untracked_paths: Vec<String> =
                    info.changes.untracked.iter().map(|f| f.path.clone()).collect();

                // Folder itself should NOT be listed
                assert!(!unstaged_paths.contains(&"untracked_dir".to_string()));
                assert!(!unstaged_paths.contains(&"untracked_dir/".to_string()));
                assert!(!untracked_paths.contains(&"untracked_dir".to_string()));
                assert!(!untracked_paths.contains(&"untracked_dir/".to_string()));

                // Untracked files (both root and nested) should be listed
                assert!(unstaged_paths.contains(&"untracked.txt".to_string()));
                assert!(unstaged_paths.contains(&"untracked_dir/nested.txt".to_string()));
                assert!(untracked_paths.contains(&"untracked.txt".to_string()));
                assert!(untracked_paths.contains(&"untracked_dir/nested.txt".to_string()));
            }
            _ => panic!("Expected ItemDetail::Repo"),
        }

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_path);
    }

    #[test]
    fn test_stage_new_and_deleted_files() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push(format!(
            "twig_test_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&temp_path).unwrap();

        // Init repo
        let repo = Repository::init(&temp_path).unwrap();

        // Configure author
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Create initial file & commit
        let init_file = temp_path.join("init.txt");
        std::fs::write(&init_file, "initial").unwrap();
        stage_file(&temp_path, "init.txt").unwrap();
        commit_changes(&temp_path, "initial commit").unwrap();

        // 1. Create a new file (untracked)
        let untracked_file = temp_path.join("untracked.txt");
        std::fs::write(&untracked_file, "new file content").unwrap();

        // Try staging untracked file
        stage_file(&temp_path, "untracked.txt").unwrap();

        // 2. Delete the initial file
        std::fs::remove_file(&init_file).unwrap();

        // Try staging deleted file
        stage_file(&temp_path, "init.txt").unwrap();

        // Check status of repo
        let detail = inspect_detail(temp_path.to_str().unwrap(), 0, 1000, false);
        match detail {
            ItemDetail::Repo { info, .. } => {
                // Both should be in staged changes
                assert_eq!(info.changes.staged.len(), 2);
                let paths: Vec<String> =
                    info.changes.staged.iter().map(|f| f.path.clone()).collect();
                assert!(paths.contains(&"untracked.txt".to_string()));
                assert!(paths.contains(&"init.txt".to_string()));
            }
            _ => panic!("Expected ItemDetail::Repo"),
        }

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_path);
    }

    #[test]
    fn test_discard_file_changes_all_cases() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push(format!(
            "twig_test_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&temp_path).unwrap();

        // Init repo
        let repo = Repository::init(&temp_path).unwrap();

        // Configure author
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Create and commit initial files
        let file_tracked = temp_path.join("tracked.txt");
        std::fs::write(&file_tracked, "original content\n").unwrap();
        stage_file(&temp_path, "tracked.txt").unwrap();
        commit_changes(&temp_path, "initial commit").unwrap();

        // Case 1: Untracked file
        let file_untracked = temp_path.join("untracked.txt");
        std::fs::write(&file_untracked, "new untracked file\n").unwrap();
        assert!(file_untracked.exists());
        discard_file_changes(&temp_path, "untracked.txt", false).unwrap();
        assert!(!file_untracked.exists());

        // Case 2: Tracked file with unstaged modification
        std::fs::write(&file_tracked, "unstaged modifications\n").unwrap();
        discard_file_changes(&temp_path, "tracked.txt", false).unwrap();
        assert_eq!(std::fs::read_to_string(&file_tracked).unwrap(), "original content\n");

        // Case 3: Tracked file with staged modification
        std::fs::write(&file_tracked, "staged modifications\n").unwrap();
        stage_file(&temp_path, "tracked.txt").unwrap();
        // verify it's staged
        let detail = inspect_detail(temp_path.to_str().unwrap(), 0, 1000, false);
        match detail {
            ItemDetail::Repo { info, .. } => {
                assert!(!info.changes.staged.is_empty());
            }
            _ => panic!("Expected ItemDetail::Repo"),
        }
        discard_file_changes(&temp_path, "tracked.txt", true).unwrap();
        assert_eq!(std::fs::read_to_string(&file_tracked).unwrap(), "original content\n");
        // verify it's no longer staged/unstaged (it's clean)
        let detail = inspect_detail(temp_path.to_str().unwrap(), 0, 1000, false);
        match detail {
            ItemDetail::Repo { info, .. } => {
                assert!(info.changes.staged.is_empty());
                assert!(info.changes.unstaged.is_empty());
            }
            _ => panic!("Expected ItemDetail::Repo"),
        }

        // Case 4: Tracked deleted file
        std::fs::remove_file(&file_tracked).unwrap();
        assert!(!file_tracked.exists());
        discard_file_changes(&temp_path, "tracked.txt", false).unwrap();
        assert!(file_tracked.exists());
        assert_eq!(std::fs::read_to_string(&file_tracked).unwrap(), "original content\n");

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_path);
    }

    #[test]
    fn test_stage_unstage_by_hunk() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push(format!(
            "twig_test_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&temp_path).unwrap();

        // Init repo
        let repo = Repository::init(&temp_path).unwrap();

        // Configure author
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Create initial file with multiple lines
        let file_path = temp_path.join("multihunk.txt");
        let mut file = File::create(&file_path).unwrap();
        for i in 1..=20 {
            writeln!(file, "Line {}", i).unwrap();
        }
        drop(file);

        // Stage and commit initial
        stage_file(&temp_path, "multihunk.txt").unwrap();
        commit_changes(&temp_path, "initial commit").unwrap();

        // Now modify lines 2 and 18 to create two distinct hunks
        let mut file = File::create(&file_path).unwrap();
        for i in 1..=20 {
            if i == 2 || i == 18 {
                writeln!(file, "Line {} modified", i).unwrap();
            } else {
                writeln!(file, "Line {}", i).unwrap();
            }
        }
        drop(file);

        // Get the unstaged diff lines
        let diff_lines = get_worktree_file_diff(&temp_path, "multihunk.txt", false);
        // Identify hunk ranges. A hunk header starts with "@@"
        let mut hunk_ranges = Vec::new();
        let mut current_start = None;
        for (i, line) in diff_lines.iter().enumerate() {
            if line.kind == DiffLineKind::Header {
                if let Some(start) = current_start {
                    hunk_ranges.push(start..i);
                }
                current_start = Some(i);
            }
        }
        if let Some(start) = current_start {
            hunk_ranges.push(start..diff_lines.len());
        }

        // We expect exactly 2 hunks
        assert_eq!(hunk_ranges.len(), 2);

        // Stage the second hunk
        let hunk2 = &diff_lines[hunk_ranges[1].clone()];
        stage_hunk(&temp_path, "multihunk.txt", hunk2).unwrap();

        // Now check staged diff for the file: it should contain the second modification
        let staged_diff = get_worktree_file_diff(&temp_path, "multihunk.txt", true);
        let staged_content: String =
            staged_diff.iter().map(|l| l.content.as_str()).collect::<Vec<_>>().join("\n");
        assert!(staged_content.contains("Line 18 modified"));
        assert!(!staged_content.contains("Line 2 modified"));

        // Check unstaged diff for the file: it should contain the first modification
        let unstaged_diff = get_worktree_file_diff(&temp_path, "multihunk.txt", false);
        let unstaged_content: String =
            unstaged_diff.iter().map(|l| l.content.as_str()).collect::<Vec<_>>().join("\n");
        assert!(unstaged_content.contains("Line 2 modified"));
        assert!(!unstaged_content.contains("Line 18 modified"));

        // Unstage the staged hunk
        let staged_hunk_ranges = {
            let mut ranges = Vec::new();
            let mut current_start = None;
            for (i, line) in staged_diff.iter().enumerate() {
                if line.kind == DiffLineKind::Header {
                    if let Some(start) = current_start {
                        ranges.push(start..i);
                    }
                    current_start = Some(i);
                }
            }
            if let Some(start) = current_start {
                ranges.push(start..staged_diff.len());
            }
            ranges
        };
        assert_eq!(staged_hunk_ranges.len(), 1);
        let staged_hunk = &staged_diff[staged_hunk_ranges[0].clone()];
        unstage_hunk(&temp_path, "multihunk.txt", staged_hunk).unwrap();

        // Staged diff should now be empty
        let staged_diff_after = get_worktree_file_diff(&temp_path, "multihunk.txt", true);
        assert!(staged_diff_after.is_empty());

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_path);
    }

    #[test]
    fn test_discard_hunk() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push(format!(
            "twig_test_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&temp_path).unwrap();

        // Init repo
        let repo = Repository::init(&temp_path).unwrap();

        // Configure author
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Create initial file with multiple lines
        let file_path = temp_path.join("discardhunk.txt");
        let mut file = File::create(&file_path).unwrap();
        for i in 1..=20 {
            writeln!(file, "Line {}", i).unwrap();
        }
        drop(file);

        // Stage and commit initial
        stage_file(&temp_path, "discardhunk.txt").unwrap();
        commit_changes(&temp_path, "initial commit").unwrap();

        // Now modify lines 2 and 18 to create two distinct hunks
        let mut file = File::create(&file_path).unwrap();
        for i in 1..=20 {
            if i == 2 || i == 18 {
                writeln!(file, "Line {} modified", i).unwrap();
            } else {
                writeln!(file, "Line {}", i).unwrap();
            }
        }
        drop(file);

        // Get the unstaged diff lines
        let diff_lines = get_worktree_file_diff(&temp_path, "discardhunk.txt", false);
        // Identify hunk ranges
        let mut hunk_ranges = Vec::new();
        let mut current_start = None;
        for (i, line) in diff_lines.iter().enumerate() {
            if line.kind == DiffLineKind::Header {
                if let Some(start) = current_start {
                    hunk_ranges.push(start..i);
                }
                current_start = Some(i);
            }
        }
        if let Some(start) = current_start {
            hunk_ranges.push(start..diff_lines.len());
        }

        // We expect exactly 2 hunks
        assert_eq!(hunk_ranges.len(), 2);

        // Discard the second hunk (Line 18 modified)
        let hunk2 = &diff_lines[hunk_ranges[1].clone()];
        discard_hunk(&temp_path, "discardhunk.txt", hunk2).unwrap();

        // Now check file contents: line 18 should be reverted to "Line 18", while line 2 should remain "Line 2 modified"
        let contents = std::fs::read_to_string(&file_path).unwrap();
        assert!(contents.contains("Line 2 modified"));
        assert!(contents.contains("Line 18\n"));
        assert!(!contents.contains("Line 18 modified"));

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_path);
    }

    #[test]
    fn test_stage_unstage_discard_line() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push(format!(
            "twig_test_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&temp_path).unwrap();

        let repo = Repository::init(&temp_path).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // 1. Create initial file
        let file_path = temp_path.join("line_test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "line A").unwrap();
        writeln!(file, "line B").unwrap();
        writeln!(file, "line C").unwrap();
        drop(file);

        stage_file(&temp_path, "line_test.txt").unwrap();
        commit_changes(&temp_path, "initial").unwrap();

        // 2. Modify to introduce two distinct changes in one hunk
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "line A modified").unwrap();
        writeln!(file, "line B").unwrap();
        writeln!(file, "line C modified").unwrap();
        drop(file);

        let diff_lines = get_worktree_file_diff(&temp_path, "line_test.txt", false);
        let mut hunk_ranges = Vec::new();
        let mut current_start = None;
        for (i, line) in diff_lines.iter().enumerate() {
            if line.kind == DiffLineKind::Header {
                if let Some(start) = current_start {
                    hunk_ranges.push(start..i);
                }
                current_start = Some(i);
            }
        }
        if let Some(start) = current_start {
            hunk_ranges.push(start..diff_lines.len());
        }

        assert_eq!(hunk_ranges.len(), 1);
        let hunk0 = &diff_lines[hunk_ranges[0].clone()];

        assert_eq!(hunk0[2].content, "line A modified");
        assert_eq!(hunk0[5].content, "line C modified");

        // A) Stage line A modified (relative index 2)
        stage_line(&temp_path, "line_test.txt", hunk0, 2).unwrap();

        // Check staged diff
        let staged_diff = get_worktree_file_diff(&temp_path, "line_test.txt", true);
        assert!(
            staged_diff
                .iter()
                .any(|l| l.kind == DiffLineKind::Added && l.content == "line A modified")
        );
        assert!(
            !staged_diff
                .iter()
                .any(|l| l.kind == DiffLineKind::Added && l.content == "line C modified")
        );

        // Check unstaged diff
        let unstaged_diff = get_worktree_file_diff(&temp_path, "line_test.txt", false);
        assert!(
            !unstaged_diff
                .iter()
                .any(|l| l.kind == DiffLineKind::Added && l.content == "line A modified")
        );
        assert!(
            unstaged_diff
                .iter()
                .any(|l| l.kind == DiffLineKind::Added && l.content == "line C modified")
        );

        // B) Unstage line A modified
        assert_eq!(staged_diff[2].content, "line A modified");
        unstage_line(&temp_path, "line_test.txt", &staged_diff, 2).unwrap();

        // Staged diff should now be empty
        assert!(get_worktree_file_diff(&temp_path, "line_test.txt", true).is_empty());

        // C) Discard line C modified (index 5) in unstaged diff
        let unstaged_diff2 = get_worktree_file_diff(&temp_path, "line_test.txt", false);
        assert_eq!(unstaged_diff2[5].content, "line C modified");
        discard_line(&temp_path, "line_test.txt", &unstaged_diff2, 5).unwrap();

        let unstaged_diff3 = get_worktree_file_diff(&temp_path, "line_test.txt", false);
        let remove_idx = unstaged_diff3
            .iter()
            .position(|l| l.kind == DiffLineKind::Removed && l.content == "line C")
            .unwrap();
        discard_line(&temp_path, "line_test.txt", &unstaged_diff3, remove_idx).unwrap();

        // File contents check
        let contents = std::fs::read_to_string(&file_path).unwrap();
        assert!(contents.contains("line A modified"));
        assert!(contents.contains("line B"));
        assert!(contents.contains("line C\n"));
        assert!(!contents.contains("line C modified"));

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_path);
    }

    #[test]
    fn test_stage_unstage_discard_all_changes() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push(format!(
            "twig_test_all_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&temp_path).unwrap();

        // Init repo
        let repo = Repository::init(&temp_path).unwrap();

        // Configure author
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Create initial file & commit it
        let file_path = temp_path.join("tracked.txt");
        std::fs::write(&file_path, "original content\n").unwrap();
        stage_file(&temp_path, "tracked.txt").unwrap();
        commit_changes(&temp_path, "initial").unwrap();

        // 1. Make a modification and create a new untracked file
        std::fs::write(&file_path, "modified content\n").unwrap();
        let untracked_path = temp_path.join("untracked.txt");
        std::fs::write(&untracked_path, "untracked content\n").unwrap();

        // Verify status has unstaged changes
        let status = repo.statuses(None).unwrap();
        assert_eq!(status.len(), 2);

        // Stage all changes
        stage_all_changes(&temp_path).unwrap();

        // Verify all changes are staged
        let status = repo.statuses(None).unwrap();
        for entry in status.iter() {
            assert!(
                entry.status().intersects(git2::Status::INDEX_MODIFIED | git2::Status::INDEX_NEW)
            );
        }

        // Unstage all changes
        unstage_all_changes(&temp_path).unwrap();

        // Verify all changes are unstaged again
        let status = repo.statuses(None).unwrap();
        for entry in status.iter() {
            assert!(entry.status().intersects(git2::Status::WT_MODIFIED | git2::Status::WT_NEW));
        }

        // Discard all changes
        discard_all_changes(&temp_path).unwrap();

        // Verify repo is completely clean
        let status = repo.statuses(None).unwrap();
        assert_eq!(status.len(), 0);

        // Verify tracked file is reset and untracked file is removed
        let contents = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(contents, "original content\n");
        assert!(!untracked_path.exists());

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_path);
    }

    #[test]
    fn test_merge_conflicts_flow() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push(format!(
            "twig_test_conflict_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&temp_path).unwrap();

        // Init repo
        let repo = Repository::init(&temp_path).unwrap();

        // Configure author
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // 1. Initial commit on main
        let file_path = temp_path.join("conflict.txt");
        std::fs::write(&file_path, "line 1\nline 2\nline 3\n").unwrap();
        stage_file(&temp_path, "conflict.txt").unwrap();
        commit_changes(&temp_path, "initial commit").unwrap();

        // Get the main branch name first
        let output = std::process::Command::new("git")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
            .args(["symbolic-ref", "--short", "HEAD"])
            .current_dir(&temp_path)
            .output()
            .unwrap();
        let main_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // 2. Create feature branch and edit
        std::process::Command::new("git")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
            .args(["checkout", "-b", "feature"])
            .current_dir(&temp_path)
            .output()
            .unwrap();

        std::fs::write(&file_path, "line 1\nline 2 on feature\nline 3\n").unwrap();
        stage_file(&temp_path, "conflict.txt").unwrap();
        commit_changes(&temp_path, "feature commit").unwrap();

        // 3. Checkout main/master and edit differently
        std::process::Command::new("git")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
            .args(["checkout", &main_branch])
            .current_dir(&temp_path)
            .output()
            .unwrap();

        std::fs::write(&file_path, "line 1\nline 2 on main\nline 3\n").unwrap();
        stage_file(&temp_path, "conflict.txt").unwrap();
        commit_changes(&temp_path, "main commit").unwrap();

        // 4. Merge feature into main -> conflict
        assert!(!is_merging(&temp_path));
        let merge_output = std::process::Command::new("git")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
            .args(["merge", "feature"])
            .current_dir(&temp_path)
            .output()
            .unwrap();

        assert!(!merge_output.status.success());
        assert!(is_merging(&temp_path));

        // 5. Check conflict markers diff
        let diff = get_conflict_markers_diff(&temp_path, "conflict.txt");
        assert!(!diff.is_empty());
        let has_separator = diff.iter().any(|l| matches!(l.kind, DiffLineKind::ConflictSeparator));
        let has_ours = diff.iter().any(|l| matches!(l.kind, DiffLineKind::ConflictOurs));
        let has_theirs = diff.iter().any(|l| matches!(l.kind, DiffLineKind::ConflictTheirs));
        assert!(has_separator);
        assert!(has_ours);
        assert!(has_theirs);

        // 6. Abort merge and verify
        abort_merge(&temp_path).unwrap();
        assert!(!is_merging(&temp_path));

        // 7. Conflict again to test resolve_ours/resolve_theirs
        std::process::Command::new("git")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
            .args(["merge", "feature"])
            .current_dir(&temp_path)
            .output()
            .unwrap();
        assert!(is_merging(&temp_path));

        // Test resolve_ours
        resolve_ours(&temp_path, "conflict.txt").unwrap();
        let contents = std::fs::read_to_string(&file_path).unwrap();
        assert!(contents.contains("line 2 on main"));
        assert!(!contents.contains("<<<<<<<"));

        // Since it's resolved, we can continue merge
        continue_merge(&temp_path).unwrap();
        assert!(!is_merging(&temp_path));

        // 8. Test resolve_theirs by resetting main to before the merge
        std::process::Command::new("git")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
            .args(["reset", "--hard", "HEAD~1"])
            .current_dir(&temp_path)
            .output()
            .unwrap();

        std::process::Command::new("git")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
            .args(["merge", "feature"])
            .current_dir(&temp_path)
            .output()
            .unwrap();
        assert!(is_merging(&temp_path));

        resolve_theirs(&temp_path, "conflict.txt").unwrap();
        let contents_theirs = std::fs::read_to_string(&file_path).unwrap();
        assert!(contents_theirs.contains("line 2 on feature"));
        assert!(!contents_theirs.contains("<<<<<<<"));

        continue_merge(&temp_path).unwrap();
        assert!(!is_merging(&temp_path));

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_path);
    }

    #[test]
    fn test_resolve_conflict_hunk() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push(format!(
            "twig_test_hunk_conflict_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&temp_path).unwrap();

        // Init repo
        let repo = Repository::init(&temp_path).unwrap();

        // Configure author
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // 1. Initial commit on main
        let file_path = temp_path.join("conflict.txt");
        let initial_lines = "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10\nline 11\nline 12\n";
        std::fs::write(&file_path, initial_lines).unwrap();
        stage_file(&temp_path, "conflict.txt").unwrap();
        commit_changes(&temp_path, "initial commit").unwrap();

        // Get the main branch name first
        let output = std::process::Command::new("git")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
            .args(["symbolic-ref", "--short", "HEAD"])
            .current_dir(&temp_path)
            .output()
            .unwrap();
        let main_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // 2. Create feature branch and edit line 2 and line 11
        std::process::Command::new("git")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
            .args(["checkout", "-b", "feature"])
            .current_dir(&temp_path)
            .output()
            .unwrap();
        let feature_lines = "line 1\nline 2 on feature\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10\nline 11 on feature\nline 12\n";
        std::fs::write(&file_path, feature_lines).unwrap();
        stage_file(&temp_path, "conflict.txt").unwrap();
        commit_changes(&temp_path, "feature commit").unwrap();

        // 3. Checkout main/master and edit line 2 and line 11 differently
        std::process::Command::new("git")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
            .args(["checkout", &main_branch])
            .current_dir(&temp_path)
            .output()
            .unwrap();
        let main_lines = "line 1\nline 2 on main\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10\nline 11 on main\nline 12\n";
        std::fs::write(&file_path, main_lines).unwrap();
        stage_file(&temp_path, "conflict.txt").unwrap();
        commit_changes(&temp_path, "main commit").unwrap();

        // 4. Merge feature into main -> conflict
        let merge_output = std::process::Command::new("git")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
            .args(["merge", "feature"])
            .current_dir(&temp_path)
            .output()
            .unwrap();
        assert!(!merge_output.status.success());
        assert!(is_merging(&temp_path));

        // 5. Resolve first hunk as Ours
        resolve_conflict_hunk(&temp_path, "conflict.txt", 0, true).unwrap();
        let contents_after_first = std::fs::read_to_string(&file_path).unwrap();
        // Line 2 should be resolved to main
        assert!(contents_after_first.contains("line 2 on main"));
        assert!(!contents_after_first.contains("line 2 on feature"));
        // Line 11 should still have conflict markers
        assert!(contents_after_first.contains("<<<<<<<"));
        assert!(contents_after_first.contains("line 11 on main"));
        assert!(contents_after_first.contains("line 11 on feature"));

        // Repo should still be in a merging state because 1 conflict hunk remains
        assert!(is_merging(&temp_path));

        // 6. Resolve second hunk (which is now hunk 0, since hunk 0 was resolved and removed)
        // Wait, did the hunk count change? Yes, the first conflict block was removed,
        // so the remaining conflict block at line 11 becomes the 0th hunk in the file!
        // Let's call resolve_conflict_hunk with hunk_idx 0!
        resolve_conflict_hunk(&temp_path, "conflict.txt", 0, false).unwrap();
        let contents_after_second = std::fs::read_to_string(&file_path).unwrap();
        // Both lines should be resolved, no conflict markers left
        assert!(contents_after_second.contains("line 2 on main"));
        assert!(contents_after_second.contains("line 11 on feature"));
        assert!(!contents_after_second.contains("<<<<<<<"));

        // File is fully resolved so it should have been automatically staged
        let status = repo.statuses(None).unwrap();
        assert_eq!(status.len(), 1);
        assert!(status.get(0).unwrap().status().contains(git2::Status::INDEX_MODIFIED));

        // Continue and finalize the merge
        continue_merge(&temp_path).unwrap();
        assert!(!is_merging(&temp_path));

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_path);
    }
}
