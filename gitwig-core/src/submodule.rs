use std::path::{Path, PathBuf};
use std::time::Duration;

use git2::Repository;

use crate::{git_command, run_git_with_timeout};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SubmoduleInfo {
    pub name: String,
    pub path: PathBuf,
    pub url: String,
    pub commit_id: Option<String>,
    pub head_id: Option<String>,
    pub is_initialized: bool,
    pub is_dirty: bool,
}

impl SubmoduleInfo {
    /// Deleted from the index but still in HEAD, until the removal is committed.
    pub fn is_removal_staged(&self) -> bool {
        self.commit_id.is_none() && self.head_id.is_some()
    }

    /// Checked out, with a commit or files that differ from what the
    /// superproject records: what the Submodules tab labels `Modified`.
    pub fn is_modified(&self) -> bool {
        !self.is_removal_staged() && self.is_initialized && self.is_dirty
    }
}

pub fn load_tab_submodules(repo_path: &Path) -> Result<Vec<SubmoduleInfo>, String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let submodules = repo.submodules().map_err(|e| e.to_string())?;
    let mut list = Vec::new();
    for sub in submodules {
        let name = sub.name().unwrap_or("").to_string();
        let path = sub.path().to_path_buf();
        let url = sub.url().unwrap_or(None).unwrap_or("").to_string();
        let commit_id = sub.index_id().map(|id| id.to_string());
        let head_id = sub.head_id().map(|id| id.to_string());

        let mut is_initialized = true;
        let mut is_dirty = false;

        if let Ok(status) = repo.submodule_status(&name, git2::SubmoduleIgnore::None) {
            is_initialized = !status.contains(git2::SubmoduleStatus::WD_UNINITIALIZED);
            is_dirty = status.contains(git2::SubmoduleStatus::WD_MODIFIED)
                || status.contains(git2::SubmoduleStatus::WD_WD_MODIFIED)
                || status.contains(git2::SubmoduleStatus::WD_UNTRACKED)
                || status.contains(git2::SubmoduleStatus::WD_INDEX_MODIFIED)
                || status.contains(git2::SubmoduleStatus::INDEX_MODIFIED);
        }

        list.push(SubmoduleInfo { name, path, url, commit_id, head_id, is_initialized, is_dirty });
    }
    Ok(list)
}

/// Deletes the submodule registered as `name` and checked out at `path`
/// (relative to the repository root): deinitialises it, removes its gitlink,
/// `.gitmodules` entry and working directory with `git rm`, then deletes its
/// module directory under the git dir so re-adding it later starts clean.
/// The result is staged, not committed.
///
/// `git submodule deinit` and `git rm` take a *path*. The *name* differs from
/// it after `git submodule add --name` or a `git mv`, and only locates the
/// module directory: passing the name to `git rm` once deleted an unrelated
/// tracked file that happened to share it. Nothing is changed unless `path` is
/// a submodule (gitlink) entry in the index.
pub fn submodule_remove(repo_path: &Path, name: &str, path: &Path) -> Result<(), String> {
    let plain_relative = |p: &Path| {
        !p.as_os_str().is_empty()
            && p.components().all(|c| matches!(c, std::path::Component::Normal(_)))
    };
    if !plain_relative(path) {
        return Err(format!("Refusing to remove submodule at unsafe path '{}'", path.display()));
    }
    if !plain_relative(Path::new(name)) {
        return Err(format!("Refusing to remove submodule with unsafe name '{}'", name));
    }
    let path_str = path.to_str().ok_or("Submodule path is not valid UTF-8")?;

    let run = |args: &[&str]| -> Result<Vec<u8>, String> {
        let output = git_command()
            .arg("--literal-pathspecs")
            .args(args)
            .current_dir(repo_path)
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
        }
        Ok(output.stdout)
    };

    // `ls-files --stage -z` prints `<mode> <oid> <stage>\t<path>\0` per entry;
    // a submodule is exactly one entry, of mode 160000, at exactly `path`.
    let staged = run(&["ls-files", "--stage", "-z", "--", path_str])?;
    let entries: Vec<&[u8]> = staged.split(|b| *b == 0).filter(|e| !e.is_empty()).collect();
    let is_gitlink = match entries.as_slice() {
        [entry] => {
            let entry = String::from_utf8_lossy(entry);
            entry.starts_with("160000 ")
                && entry.split_once('\t').is_some_and(|(_, p)| p == path_str)
        }
        _ => false,
    };
    if !is_gitlink {
        return Err(format!("'{}' is not a submodule in the index; nothing was removed", path_str));
    }

    run(&["submodule", "deinit", "-f", "--", path_str])?;
    run(&["rm", "-f", "--", path_str])?;

    // Module directories are per worktree, so ask git where this one's lives
    // (relative to `repo_path` in the main worktree, absolute in a linked one).
    let module_path = run(&["rev-parse", "--git-path", &format!("modules/{}", name)])?;
    let module_dir = repo_path.join(String::from_utf8_lossy(&module_path).trim());
    if module_dir.is_dir() {
        std::fs::remove_dir_all(&module_dir).map_err(|e| {
            format!(
                "Submodule removed, but its module directory {} could not be deleted: {}",
                module_dir.display(),
                e
            )
        })?;
    }
    Ok(())
}

/// Initialises and updates the submodule checked out at `path` (relative to
/// the repository root), or every submodule when `path` is `None`, together
/// with any submodules nested inside it: `git submodule update --init
/// --recursive`. Each submodule's configured update mode applies and nothing
/// is forced, so local changes a checkout would overwrite fail the update
/// instead of being lost. It may clone or fetch, so it is bounded by `timeout`
/// (zero means no limit).
pub fn submodule_update(
    repo_path: &Path,
    path: Option<&Path>,
    timeout: Duration,
) -> Result<(), String> {
    let mut cmd = git_command();
    cmd.args(["--literal-pathspecs", "submodule", "update", "--init", "--recursive"])
        .current_dir(repo_path);
    if let Some(path) = path {
        cmd.arg("--").arg(path);
    }
    let output = run_git_with_timeout(cmd, timeout).map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().replace('\n', " "));
    }
    Ok(())
}
