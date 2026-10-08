//! Creating stashes: the whole workspace or chosen paths (`git stash push`),
//! or a single hunk or line of a file's unstaged changes.
//!
//! git has no non-interactive "stash this hunk", so a partial stash is built
//! the way `git stash push --patch` builds one: a stash commit based on HEAD
//! whose tree is HEAD plus just that change, recorded with `git stash store`
//! and then taken out of the working tree. It lists, shows, applies and pops
//! like any other stash.

use std::path::{Path, PathBuf};
use std::process::Command;

use git2::build::TreeUpdateBuilder;
use git2::{Commit, FileMode, Oid, Repository, Signature, Tree};

use crate::{DiffLine, DiffLineKind, git_command, patch};

/// Stashes the working tree (`git stash push`). A non-empty `paths` limits the
/// stash to those files; everything else is left as it is.
///
/// Fails with git's "No local changes to save" when there was nothing to
/// stash: `git stash push` reports that as success without creating a stash.
pub fn save_stash(
    repo_path: &Path,
    message: &str,
    include_untracked: bool,
    keep_index: bool,
    paths: &[String],
) -> Result<(), String> {
    let mut cmd = git_command();
    cmd.arg("stash").arg("push");

    if include_untracked {
        cmd.arg("--include-untracked");
    }
    if keep_index {
        cmd.arg("--keep-index");
    }
    if !message.is_empty() {
        cmd.arg("-m").arg(message);
    }
    if !paths.is_empty() {
        // Paths are pathspecs to git: a `*` or `[` in a file name must not glob.
        cmd.env("GIT_LITERAL_PATHSPECS", "1");
        cmd.arg("--").args(paths);
    }

    let before = stash_tip(repo_path);
    let output = run(cmd.current_dir(repo_path))?;
    if stash_tip(repo_path) == before {
        let said = output.trim();
        return Err(if said.is_empty() { "No local changes to save" } else { said }.to_string());
    }
    if paths.is_empty() {
        return Ok(());
    }
    narrow_stash(repo_path, paths)
        .map_err(|e| format!("Stashed, but limiting the stash to the picked files failed: {}", e))
}

/// The commit `refs/stash` points at, if any.
fn stash_tip(repo_path: &Path) -> Option<Oid> {
    Repository::open(repo_path).ok()?.refname_to_id("refs/stash").ok()
}

/// Rewrites the newest stash so its index and working-tree commits differ
/// from their base only in `paths`.
///
/// `git stash push -- <paths>` resets just those paths, but records the
/// *whole* index in the stash, so another file's staged changes would show in
/// the stash and come back when it is applied. The untracked-files commit is
/// already limited to `paths` and is kept as it is.
fn narrow_stash(repo_path: &Path, paths: &[String]) -> Result<(), String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let stash = repo
        .refname_to_id("refs/stash")
        .and_then(|oid| repo.find_commit(oid))
        .map_err(|e| e.to_string())?;
    let base = stash.parent(0).map_err(|e| e.to_string())?;
    let index = stash.parent(1).map_err(|e| e.to_string())?;
    let base_tree = base.tree().map_err(|e| e.to_string())?;
    let index_tree = index.tree().map_err(|e| e.to_string())?;
    let stash_tree = stash.tree().map_err(|e| e.to_string())?;

    let narrow_index = narrow_tree(&repo, &base_tree, &index_tree, paths)?;
    let narrow_work = narrow_tree(&repo, &base_tree, &stash_tree, paths)?;
    if narrow_index == index_tree.id() && narrow_work == stash_tree.id() {
        return Ok(());
    }

    let recommit = |like: &Commit<'_>, tree: Oid, parents: &[&Commit<'_>]| {
        let tree = repo.find_tree(tree)?;
        let message = String::from_utf8_lossy(like.message_bytes()).into_owned();
        repo.commit(None, &like.author(), &like.committer(), &message, &tree, parents)
    };
    let new_index = recommit(&index, narrow_index, &[&base])
        .and_then(|oid| repo.find_commit(oid))
        .map_err(|e| e.to_string())?;
    let untracked = stash.parent(2).ok();
    let mut parents = vec![&base, &new_index];
    parents.extend(untracked.as_ref());
    let new_stash = recommit(&stash, narrow_work, &parents).map_err(|e| e.to_string())?;

    // Store the replacement before dropping the original, so a failure can
    // leave a duplicate but never lose the stash.
    let reflog = repo.reflog("refs/stash").map_err(|e| e.to_string())?;
    let message = reflog
        .get(0)
        .and_then(|entry| entry.message_bytes().map(|m| String::from_utf8_lossy(m).into_owned()))
        .unwrap_or_else(|| String::from_utf8_lossy(stash.message_bytes()).trim().to_string());
    run(git_command()
        .args(["stash", "store", "-m", &message, &new_stash.to_string()])
        .current_dir(repo_path))?;
    run(git_command().args(["stash", "drop", "-q", "stash@{1}"]).current_dir(repo_path)).map(|_| ())
}

/// `base` with each of `paths` set to its entry in `source`, or removed where
/// `source` has none.
fn narrow_tree(
    repo: &Repository,
    base: &Tree<'_>,
    source: &Tree<'_>,
    paths: &[String],
) -> Result<Oid, String> {
    let mut update = TreeUpdateBuilder::new();
    for path in paths {
        match source.get_path(Path::new(path)) {
            Ok(entry) => {
                update.upsert(path.as_str(), entry.id(), file_mode(entry.filemode()));
            }
            Err(_) if base.get_path(Path::new(path)).is_ok() => {
                update.remove(path.as_str());
            }
            Err(_) => {}
        }
    }
    update.create_updated(repo, base).map_err(|e| e.to_string())
}

fn file_mode(raw: i32) -> FileMode {
    match raw {
        0o040000 => FileMode::Tree,
        0o100755 => FileMode::BlobExecutable,
        0o120000 => FileMode::Link,
        0o160000 => FileMode::Commit,
        _ => FileMode::Blob,
    }
}

/// Stashes one hunk of `file_path`'s unstaged changes (`hunk[0]` is its `@@`
/// header, as in the Unstaged diff) and removes it from the working tree.
/// Returns what was stashed, e.g. `hunk at line 12 of src/app.rs`.
pub fn stash_hunk(repo_path: &Path, file_path: &str, hunk: &[DiffLine]) -> Result<String, String> {
    let forward = patch::hunk_patch(file_path, hunk)?;
    let line = hunk
        .first()
        .and_then(|header| patch::parse_hunk_header(&header.content))
        .map_or(0, |(_, _, new_start, _)| new_start);
    let what = format!("hunk at line {} of {}", line, file_path);
    stash_change(repo_path, &forward, &forward, true, &what)?;
    Ok(what)
}

/// Stashes the single `+` / `-` line at `selected_line_idx` of an Unstaged
/// diff hunk and removes it from the working tree. Returns what was stashed,
/// e.g. `line 40 of src/app.rs`.
pub fn stash_line(
    repo_path: &Path,
    file_path: &str,
    hunk: &[DiffLine],
    selected_line_idx: usize,
) -> Result<String, String> {
    // The staging patch carries the line alone onto the index's version of the
    // file; the discard patch takes it back out of the working tree.
    let forward = patch::line_patch(file_path, hunk, selected_line_idx, false, false)?;
    let removal = patch::line_patch(file_path, hunk, selected_line_idx, true, true)?;
    let line = hunk
        .get(selected_line_idx)
        .and_then(|l| if l.kind == DiffLineKind::Removed { l.old_lineno } else { l.new_lineno })
        .unwrap_or(0);
    let what = format!("line {} of {}", line, file_path);
    stash_change(repo_path, &forward, &removal, false, &what)?;
    Ok(what)
}

/// Records `forward` (a one-file patch against the index's version of the
/// file) as a stash on top of HEAD, then applies `removal` to the working
/// tree, reversed when `reverse_removal`, to take the change out of it.
///
/// The stash's index commit has HEAD's tree: nothing staged goes into it, so
/// `git stash apply --index` has no index changes to restore. The
/// repository's own index is never touched.
fn stash_change(
    repo_path: &Path,
    forward: &str,
    removal: &str,
    reverse_removal: bool,
    what: &str,
) -> Result<(), String> {
    let removal_args: &[&str] = if reverse_removal { &["--reverse"] } else { &[] };
    // Nothing is written until the change is known to come out cleanly.
    let mut check_args = vec!["--check"];
    check_args.extend_from_slice(removal_args);
    patch::git_apply(repo_path, removal, &check_args, None)?;

    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let head = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .ok_or_else(|| "Cannot stash: the repository has no commits yet".to_string())?;
    let stash_tree = tree_with_patch(&repo, repo_path, &head, forward)?;

    let branch = if repo.head_detached().unwrap_or(false) {
        None
    } else {
        repo.head().ok().and_then(|h| h.shorthand().ok().map(str::to_string))
    };
    let branch = branch.unwrap_or_else(|| "(no branch)".to_string());
    let short_id = head
        .as_object()
        .short_id()
        .ok()
        .and_then(|id| id.as_str().ok().map(str::to_string))
        .unwrap_or_default();
    let message = format!("On {}: {}", branch, what);

    // Like `git stash`, fall back to a placeholder identity rather than fail
    // when user.name / user.email are not configured.
    let sig = repo
        .signature()
        .or_else(|_| Signature::now("git stash", "git@stash"))
        .map_err(|e| e.to_string())?;
    let head_tree = head.tree().map_err(|e| e.to_string())?;
    let index_message = format!(
        "index on {}: {} {}",
        branch,
        short_id,
        head.summary().ok().flatten().unwrap_or("")
    );
    let index_commit = repo
        .commit(None, &sig, &sig, &index_message, &head_tree, &[&head])
        .and_then(|oid| repo.find_commit(oid))
        .map_err(|e| e.to_string())?;
    let stash_tree = repo.find_tree(stash_tree).map_err(|e| e.to_string())?;
    let stash_commit = repo
        .commit(None, &sig, &sig, &message, &stash_tree, &[&head, &index_commit])
        .map_err(|e| e.to_string())?;

    run(git_command()
        .args(["stash", "store", "-m", &message, &stash_commit.to_string()])
        .current_dir(repo_path))?;

    patch::git_apply(repo_path, removal, removal_args, None).map_err(|e| {
        format!("Stashed {}, but taking it out of the working tree failed: {}", what, e)
    })
}

/// HEAD's tree with `forward` applied, built in a scratch index so the
/// repository's own index is left alone.
fn tree_with_patch(
    repo: &Repository,
    repo_path: &Path,
    head: &Commit<'_>,
    forward: &str,
) -> Result<Oid, String> {
    let scratch = ScratchIndex::new(repo.path())?;
    run(git_command()
        .env("GIT_INDEX_FILE", &scratch.0)
        .args(["read-tree", &head.id().to_string()])
        .current_dir(repo_path))?;
    patch::git_apply(repo_path, forward, &["--cached"], Some(&scratch.0)).map_err(|e| {
        format!(
            "The change does not apply to the committed version of the file; \
             if the file has staged changes, unstage them first ({})",
            e
        )
    })?;
    let tree = run(git_command()
        .env("GIT_INDEX_FILE", &scratch.0)
        .arg("write-tree")
        .current_dir(repo_path))?;
    Oid::from_str(tree.trim()).map_err(|e| e.to_string())
}

/// A throwaway index file in the git directory, deleted when dropped.
struct ScratchIndex(PathBuf);

impl ScratchIndex {
    fn new(git_dir: &Path) -> Result<Self, String> {
        // GIT_INDEX_FILE is resolved against git's working directory, so the
        // path must not depend on where gitwig itself was started.
        let git_dir = std::path::absolute(git_dir).map_err(|e| e.to_string())?;
        Ok(Self(git_dir.join(format!("gitwig-stash-{}.index", std::process::id()))))
    }
}

impl Drop for ScratchIndex {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Runs a local git command and returns its stdout, or its stderr as the error.
fn run(cmd: &mut Command) -> Result<String, String> {
    let output = cmd.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
// Fixtures drive real `git` to inspect the stash; the TUI is not running here.
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::{commit_changes, get_worktree_file_diff, stage_file};

    /// A repository with one commit holding `files`, in its own scratch directory.
    struct Fixture(PathBuf);

    impl Fixture {
        fn new(tag: &str, files: &[(&str, String)]) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir().join(format!("gitwig_stash_{}_{}", tag, nanos));
            std::fs::create_dir_all(&dir).unwrap();
            let repo = Repository::init(&dir).unwrap();
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
            let fixture = Self(dir);
            for (name, content) in files {
                fixture.write(name, content);
                stage_file(&fixture.0, name).unwrap();
            }
            commit_changes(&fixture.0, "initial").unwrap();
            fixture
        }

        fn write(&self, name: &str, content: &str) {
            std::fs::write(self.0.join(name), content).unwrap();
        }

        fn read(&self, name: &str) -> String {
            std::fs::read_to_string(self.0.join(name)).unwrap()
        }

        fn git(&self, args: &[&str]) -> String {
            let out = Command::new("git").args(args).current_dir(&self.0).output().unwrap();
            assert!(
                out.status.success(),
                "git {:?}: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
            String::from_utf8_lossy(&out.stdout).into_owned()
        }

        /// The Unstaged diff of `name`, split into hunks.
        fn hunks(&self, name: &str) -> Vec<Vec<DiffLine>> {
            let mut hunks: Vec<Vec<DiffLine>> = Vec::new();
            for line in get_worktree_file_diff(&self.0, name, false) {
                if line.kind == DiffLineKind::Header {
                    hunks.push(Vec::new());
                }
                if let Some(hunk) = hunks.last_mut() {
                    hunk.push(line);
                }
            }
            hunks
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn lines(n: usize, edit: impl Fn(usize) -> Option<String>) -> String {
        (1..=n).map(|i| edit(i).unwrap_or_else(|| format!("Line {}", i)) + "\n").collect()
    }

    #[test]
    fn stash_hunk_stashes_only_that_hunk() {
        let fx = Fixture::new("hunk", &[("f.txt", lines(20, |_| None))]);
        fx.write(
            "f.txt",
            &lines(20, |i| (i == 2 || i == 18).then(|| format!("Line {} modified", i))),
        );
        let hunks = fx.hunks("f.txt");
        assert_eq!(hunks.len(), 2);

        let what = stash_hunk(&fx.0, "f.txt", &hunks[1]).unwrap();
        assert_eq!(what, "hunk at line 15 of f.txt");

        // The other hunk stays; the stashed one is gone from the working tree.
        let content = fx.read("f.txt");
        assert!(content.contains("Line 2 modified"));
        assert!(!content.contains("Line 18 modified"));
        // The index is untouched.
        assert!(get_worktree_file_diff(&fx.0, "f.txt", true).is_empty());

        let list = fx.git(&["stash", "list", "--format=%gs"]);
        assert!(list.contains(": hunk at line 15 of f.txt"), "{}", list);
        let shown = fx.git(&["stash", "show", "-p", "stash@{0}"]);
        assert!(shown.contains("+Line 18 modified"));
        assert!(!shown.contains("Line 2 modified"));

        // Popped onto a clean tree, it brings back exactly that hunk.
        fx.git(&["checkout", "--", "f.txt"]);
        fx.git(&["stash", "pop"]);
        let popped = fx.read("f.txt");
        assert!(popped.contains("Line 18 modified"));
        assert!(!popped.contains("Line 2 modified"));
    }

    #[test]
    fn stash_line_stashes_only_that_line() {
        let fx = Fixture::new("line", &[("f.txt", lines(20, |_| None))]);
        fx.write("f.txt", &lines(20, |i| (i == 10).then(|| "Line 10\nnew A\nnew B".to_string())));
        let hunks = fx.hunks("f.txt");
        assert_eq!(hunks.len(), 1);
        let hunk = &hunks[0];
        let idx = hunk.iter().position(|l| l.content == "new B").unwrap();

        let what = stash_line(&fx.0, "f.txt", hunk, idx).unwrap();
        assert_eq!(what, "line 12 of f.txt");

        let content = fx.read("f.txt");
        assert!(content.contains("new A"));
        assert!(!content.contains("new B"));

        fx.git(&["checkout", "--", "f.txt"]);
        fx.git(&["stash", "pop"]);
        let popped = fx.read("f.txt");
        assert!(popped.contains("Line 10\nnew B\nLine 11"), "{}", popped);
        assert!(!popped.contains("new A"));
    }

    #[test]
    fn stash_hunk_that_no_longer_matches_leaves_everything_alone() {
        let fx = Fixture::new("stale", &[("f.txt", lines(5, |_| None))]);
        fx.write("f.txt", &lines(5, |i| (i == 3).then(|| "Line 3 modified".to_string())));
        let hunk = fx.hunks("f.txt").remove(0);
        let changed_again = lines(5, |i| (i == 3).then(|| "Line 3 rewritten".to_string()));
        fx.write("f.txt", &changed_again);

        assert!(stash_hunk(&fx.0, "f.txt", &hunk).is_err());
        assert_eq!(fx.read("f.txt"), changed_again);
        assert!(fx.git(&["stash", "list"]).is_empty());
    }

    #[test]
    fn stash_line_rejects_a_context_line() {
        let fx = Fixture::new("context", &[("f.txt", lines(5, |_| None))]);
        fx.write("f.txt", &lines(5, |i| (i == 3).then(|| "Line 3 modified".to_string())));
        let hunk = fx.hunks("f.txt").remove(0);
        let ctx = hunk.iter().position(|l| l.kind == DiffLineKind::Context).unwrap();

        assert!(stash_line(&fx.0, "f.txt", &hunk, ctx).is_err());
        assert!(fx.git(&["stash", "list"]).is_empty());
    }

    #[test]
    fn save_stash_with_paths_stashes_only_those_files() {
        let fx = Fixture::new(
            "paths",
            &[
                ("a.txt", "a\n".to_string()),
                ("b.txt", "b\n".to_string()),
                ("fa.txt", "fa\n".to_string()),
                ("f[ab].txt", "glob\n".to_string()),
            ],
        );
        for name in ["a.txt", "b.txt", "fa.txt", "f[ab].txt"] {
            fx.write(name, "changed\n");
        }
        // A staged change in a file that is not picked stays out of the stash.
        fx.git(&["add", "b.txt"]);
        fx.write("new.txt", "untracked\n");

        let picked = ["a.txt".to_string(), "f[ab].txt".to_string(), "new.txt".to_string()];
        save_stash(&fx.0, "picked", true, false, &picked).unwrap();

        assert_eq!(fx.read("a.txt"), "a\n");
        assert_eq!(fx.read("f[ab].txt"), "glob\n");
        assert!(!fx.0.join("new.txt").exists());
        // `f[ab].txt` is taken literally, so it did not match `fa.txt`.
        assert_eq!(fx.read("fa.txt"), "changed\n");
        assert_eq!(fx.read("b.txt"), "changed\n");
        assert_eq!(fx.git(&["diff", "--cached", "--name-only"]), "b.txt\n");

        let files = fx.git(&["stash", "show", "--name-only", "--include-untracked", "stash@{0}"]);
        assert_eq!(files.lines().collect::<Vec<_>>(), ["a.txt", "f[ab].txt", "new.txt"]);
        assert_eq!(fx.git(&["stash", "list", "--format=%gs"]).lines().count(), 1);
        assert!(fx.git(&["stash", "list", "--format=%gs"]).contains(": picked"));

        // Applied after b.txt's staged change is dropped, it does not bring it back.
        fx.git(&["reset", "-q", "--hard"]);
        fx.git(&["stash", "pop"]);
        assert_eq!(fx.read("a.txt"), "changed\n");
        assert_eq!(fx.read("new.txt"), "untracked\n");
        assert_eq!(fx.read("b.txt"), "b\n");
    }

    #[test]
    fn save_stash_with_nothing_to_save_fails_and_keeps_older_stashes() {
        let fx =
            Fixture::new("nothing", &[("a.txt", "a\n".to_string()), ("b.txt", "b\n".to_string())]);
        fx.write("a.txt", "changed\n");
        save_stash(&fx.0, "first", false, false, &[]).unwrap();

        let err = save_stash(&fx.0, "", false, false, &[]).unwrap_err();
        assert!(err.contains("No local changes"), "{}", err);
        fx.write("a.txt", "changed again\n");
        assert!(save_stash(&fx.0, "", false, false, &["b.txt".to_string()]).is_err());

        let list = fx.git(&["stash", "list", "--format=%gs"]);
        assert_eq!(list.lines().count(), 1);
        assert!(list.contains(": first"), "{}", list);
    }
}
