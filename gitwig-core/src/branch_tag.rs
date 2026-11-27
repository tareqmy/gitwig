use std::path::Path;
use std::time::Duration;

use git2::Repository;

use crate::{
    BranchInfo, git_command, run_git_with_timeout, safe_ref, safe_sha_slice, sanitize_text,
};

pub fn checkout_local_branch(repo_path: &Path, branch_name: &str) -> Result<(), git2::Error> {
    let output = git_command()
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

    let output = git_command()
        .arg("checkout")
        .arg(local_name)
        .current_dir(repo_path)
        .output()
        .map_err(|e| git2::Error::from_str(&e.to_string()))?;

    if output.status.success() {
        return Ok(format!("Switched to existing branch '{}'", local_name));
    }

    let output = git_command()
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

/// Creates or updates a tag pointing at the specified commit OID with optional message and force flag.
pub fn create_tag(
    repo_path: &Path,
    tag_name: &str,
    commit_oid_str: &str,
    message: Option<&str>,
    force: bool,
) -> Result<(), git2::Error> {
    let repo = Repository::open(repo_path)?;
    let oid = git2::Oid::from_str(commit_oid_str)?;
    let target_object = repo.find_object(oid, Some(git2::ObjectType::Commit))?;
    if let Some(msg) = message.filter(|m| !m.trim().is_empty()) {
        let tagger_sig = repo.signature();
        let default_sig;
        let tagger = match &tagger_sig {
            Ok(s) => s,
            Err(_) => {
                if let Ok(sig) = git2::Signature::now("Gitwig", "gitwig@local") {
                    default_sig = sig;
                    &default_sig
                } else {
                    return Err(git2::Error::from_str("Failed to create signature"));
                }
            }
        };
        repo.tag(tag_name, &target_object, tagger, msg, force)?;
    } else {
        repo.tag_lightweight(tag_name, &target_object, force)?;
    }
    Ok(())
}

/// Checks if a local tag exists in the repository.
pub fn tag_exists(repo_path: &Path, tag_name: &str) -> bool {
    if let Ok(repo) = Repository::open(repo_path) {
        let ref_name = format!("refs/tags/{}", tag_name);
        repo.find_reference(&ref_name).is_ok()
    } else {
        false
    }
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
    timeout: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let safe_remote = safe_ref(remote_name)?;
    let safe_tag = safe_ref(tag_name)?;
    let mut cmd = git_command();
    cmd.arg("push").arg(safe_remote).arg("--delete").arg(safe_tag).current_dir(repo_path);
    let output = run_git_with_timeout(cmd, timeout)?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(err.into());
    }
    Ok(())
}

pub fn checkout_tag(repo_path: &Path, tag_name: &str) -> Result<(), git2::Error> {
    let safe_tag = safe_ref(tag_name).map_err(|e| git2::Error::from_str(&e))?;
    let output = git_command()
        .arg("checkout")
        .arg(safe_tag)
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
    timeout: Duration,
) -> Result<Vec<BranchInfo>, Box<dyn std::error::Error>> {
    let safe_remote = safe_ref(remote_name)?;
    let mut cmd = git_command();
    cmd.arg("ls-remote").arg("--tags").arg(safe_remote).current_dir(repo_path);
    let output = run_git_with_timeout(cmd, timeout)?;

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
                let clean_ref = if is_peeled {
                    safe_sha_slice(ref_name, ref_name.len().saturating_sub(3))
                } else {
                    ref_name
                };
                let tag_name = clean_ref.strip_prefix("refs/tags/").unwrap_or(clean_ref);
                let short_sha = safe_sha_slice(sha, 7);

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

                let sanitized_tag = sanitize_text(tag_name);
                let sanitized_msg = sanitize_text(&short_message);

                if is_peeled {
                    tags_map.insert(sanitized_tag, (short_sha.to_string(), sanitized_msg));
                } else {
                    tags_map
                        .entry(sanitized_tag)
                        .or_insert_with(|| (short_sha.to_string(), sanitized_msg));
                }
            }
        }
    }

    let mut tags = Vec::new();
    for (name, (short_sha, short_message)) in tags_map {
        tags.push(BranchInfo { name, is_head: false, short_sha, short_message });
    }
    tags.sort_by(|a, b| b.name.cmp(&a.name));
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

/// Returns the upstream tracking remote name for a branch, if configured.
pub fn get_branch_upstream_remote(repo_path: &Path, branch_name: &str) -> Option<String> {
    let repo = Repository::open(repo_path).ok()?;
    let branch = repo.find_branch(branch_name, git2::BranchType::Local).ok()?;
    let _upstream = branch.upstream().ok()?;
    let local_ref = branch.get().name().ok()?;
    let remote_buf = repo.branch_upstream_remote(local_ref).ok()?;
    remote_buf.as_str().ok().map(|s| s.to_string())
}

/// Returns whether the specified branch has a configured upstream tracking branch.
pub fn has_upstream_remote(repo_path: &Path, branch_name: &str) -> bool {
    get_branch_upstream_remote(repo_path, branch_name).is_some()
}

/// Finds the remote target for pushing a branch. Returns `(remote_name, set_upstream)`.
pub fn get_branch_push_target(repo_path: &Path, branch_name: &str) -> Option<(String, bool)> {
    let repo = Repository::open(repo_path).ok()?;
    let branch = repo.find_branch(branch_name, git2::BranchType::Local).ok()?;
    if branch.upstream().is_ok() {
        if let Ok(local_ref) = branch.get().name() {
            if let Ok(remote_buf) = repo.branch_upstream_remote(local_ref) {
                if let Ok(name) = remote_buf.as_str() {
                    return Some((name.to_string(), false));
                }
            }
        }
    }
    let remotes = repo.remotes().ok()?;
    let first_remote = remotes.iter().next()?.ok()??.to_string();
    Some((first_remote, true))
}

/// Returns whether the specified commit OID has no parents (i.e. it is the root commit).
pub fn is_root_commit(repo_path: &Path, commit_oid: &str) -> bool {
    if let Ok(repo) = Repository::open(repo_path) {
        if let Ok(oid) = git2::Oid::from_str(commit_oid) {
            if let Ok(commit) = repo.find_commit(oid) {
                return commit.parent_count() == 0;
            }
        }
    }
    false
}
