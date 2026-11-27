use std::path::{Path, PathBuf};

use git2::Repository;

use crate::git_command;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorktreeInfo {
    pub name: String,
    pub path: PathBuf,
    pub branch: Option<String>,
    pub is_locked: bool,
    pub lock_reason: Option<String>,
}

pub fn load_tab_worktrees(repo_path: &Path) -> Result<Vec<WorktreeInfo>, String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let mut worktrees_list = Vec::new();
    if let Ok(worktree_names) = repo.worktrees() {
        for name in worktree_names.iter() {
            let Ok(Some(wt_name)) = name else { continue };
            if let Ok(wt) = repo.find_worktree(wt_name) {
                let path = wt.path().to_path_buf();
                let mut branch = None;
                if let Ok(wt_repo) = Repository::open(&path) {
                    if let Ok(head) = wt_repo.head() {
                        branch = head.shorthand().map(String::from).ok();
                    }
                }

                let mut is_locked = false;
                let mut lock_reason = None;
                if let Ok(git2::WorktreeLockStatus::Locked(reason_opt)) = wt.is_locked() {
                    is_locked = true;
                    if let Some(reason) = reason_opt {
                        if !reason.is_empty() {
                            lock_reason = Some(reason);
                        }
                    }
                }

                worktrees_list.push(WorktreeInfo {
                    name: wt_name.to_string(),
                    path,
                    branch,
                    is_locked,
                    lock_reason,
                });
            }
        }
    }
    Ok(worktrees_list)
}

/// Adds a worktree at `wt_path` for `branch`. An existing branch, tag or
/// commit is checked out there, and a branch that only exists on one remote
/// becomes a local branch tracking it (git's own guess). Any other name
/// creates that branch, at HEAD, for the new worktree. Returns whether a new
/// branch was created.
pub fn worktree_add(repo_path: &Path, branch: &str, wt_path: &Path) -> Result<bool, String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let exists = repo.revparse_single(&format!("{}^{{commit}}", branch)).is_ok();
    let on_one_remote = || {
        let Ok(remotes) = repo.remotes() else { return false };
        let on = remotes.iter().flatten().flatten().filter(|remote| {
            repo.find_branch(&format!("{}/{}", remote, branch), git2::BranchType::Remote).is_ok()
        });
        on.count() == 1
    };
    let create = !exists && !on_one_remote();

    let mut cmd = git_command();
    cmd.arg("worktree").arg("add");
    if create {
        cmd.arg("-b").arg(branch);
    }
    cmd.arg("--").arg(wt_path);
    if !create {
        cmd.arg(branch);
    }
    let output = cmd.current_dir(repo_path).output().map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(create)
}

// The `git worktree lock` / `unlock` / `remove` helpers below take the
// worktree's checkout path (`WorktreeInfo::path`), never its admin name
// (`WorktreeInfo::name`, the directory under `.git/worktrees`). Git resolves a
// bare name as the trailing component of a worktree *path*, so the admin name
// of a moved worktree, or of one whose folder name another worktree shares,
// matches a different worktree or none at all: `remove` once force-deleted the
// wrong checkout that way.

pub fn worktree_lock(repo_path: &Path, worktree: &Path, reason: &str) -> Result<(), String> {
    let output = git_command()
        .arg("worktree")
        .arg("lock")
        .arg("--reason")
        .arg(reason)
        .arg(worktree)
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

pub fn worktree_unlock(repo_path: &Path, worktree: &Path) -> Result<(), String> {
    let output = git_command()
        .arg("worktree")
        .arg("unlock")
        .arg(worktree)
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

/// Removes the worktree checked out at `worktree`, deleting its directory.
///
/// Without `force`, git refuses a worktree with modified or untracked files, or
/// a locked one, and nothing is touched. With `force`, `--force` is passed
/// twice, which discards those changes and overrides the lock.
pub fn worktree_remove(repo_path: &Path, worktree: &Path, force: bool) -> Result<(), String> {
    let mut cmd = git_command();
    cmd.arg("worktree").arg("remove");
    if force {
        cmd.arg("--force").arg("--force");
    }
    let output = cmd.arg(worktree).current_dir(repo_path).output().map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

pub fn worktree_prune(repo_path: &Path) -> Result<(), String> {
    let output = git_command()
        .arg("worktree")
        .arg("prune")
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}
