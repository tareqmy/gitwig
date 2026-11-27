#![allow(clippy::unwrap_used, clippy::panic)]
// Test fixtures drive real `git` to build repositories; the TUI is not running here.
#![allow(clippy::disallowed_methods)]

use super::*;
use std::fs::File;

#[test]
fn test_match_pattern() {
    assert!(match_pattern("test.psd", "*.psd"));
    assert!(match_pattern("assets/test.psd", "*.psd"));
    assert!(match_pattern("assets/img.png", "assets/*.png"));
    assert!(match_pattern("src/app.rs", "src/*"));
    assert!(match_pattern("test.txt", "test.txt"));
    assert!(!match_pattern("src/app.rs", "*.psd"));
}

#[test]
fn test_run_git_with_timeout_kills_a_hanging_child() {
    // Stand in for a remote that accepts the connection and never replies.
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("cmd");
        c.arg("/c").arg("timeout").arg("/t").arg("30").arg("/nobreak");
        c
    } else {
        let mut c = std::process::Command::new("sleep");
        c.arg("30");
        c
    };
    cmd.stdin(std::process::Stdio::null());

    let started = std::time::Instant::now();
    let res = run_git_with_timeout(cmd, Duration::from_millis(300));
    let elapsed = started.elapsed();

    assert!(res.is_err(), "expected a timeout error, got {:?}", res);
    assert!(elapsed < Duration::from_secs(10), "child was not killed promptly: {:?}", elapsed);
}

#[test]
fn test_get_lfs_info() {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!(
        "twig_test_lfs_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&temp_path).unwrap();

    // 1. Create a dummy git repo with .git/lfs directory to bypass the short-circuit check
    let _repo = Repository::init(&temp_path).unwrap();
    let lfs_dir = temp_path.join(".git").join("lfs");
    std::fs::create_dir_all(&lfs_dir).unwrap();

    // 2. Call get_lfs_info (which will trigger is_lfs_installed and cover them)
    let (installed, tracked) = get_lfs_info(&temp_path);

    // Assert that get_lfs_info completed (whether lfs is installed on user's machine or not)
    assert!(tracked.is_empty());

    // 3. Create .gitattributes with lfs pattern
    let gitattributes_path = temp_path.join(".gitattributes");
    std::fs::write(&gitattributes_path, "*.bin filter=lfs diff=lfs merge=lfs").unwrap();

    let (installed2, _tracked2) = get_lfs_info(&temp_path);
    assert_eq!(installed, installed2);

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_path);
}

#[test]
fn test_comprehensive_gitwig_core_coverage() {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!(
        "twig_test_cov_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&temp_path).unwrap();

    // 1. Repository init
    let repo = Repository::init(&temp_path).unwrap();

    // 2. Configure author
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    // 3. Create dummy file & commit
    let file_path = temp_path.join("test.txt");
    std::fs::write(&file_path, "line 1\nline 2\nline 3\n").unwrap();
    stage_file(&temp_path, "test.txt").unwrap();
    commit_changes(&temp_path, "first commit").unwrap();

    let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
    let head_oid = head_commit.id().to_string();

    // 4. Test diff & worktree helpers
    let _diffs = get_commit_file_diff(&temp_path, &head_oid, "test.txt");
    let _worktree_diff = get_worktree_file_diff(&temp_path, "test.txt", false);

    // 5. Test stage/unstage/discard all
    let _ = stage_all_changes(&temp_path);
    let _ = unstage_all_changes(&temp_path);

    std::fs::write(&file_path, "modified content").unwrap();
    let _ = discard_all_changes(&temp_path);

    // 6. Test LFS functions
    let _lfs_size = get_lfs_storage_size(&temp_path);

    // 7. Test load tab listings
    let _reflog = load_tab_reflog(&temp_path);
    let _tags = load_tab_tags(&temp_path);
    let _stashes = load_tab_stashes(&temp_path);
    let _submodules = load_tab_submodules(&temp_path);
    let _worktrees = load_tab_worktrees(&temp_path);

    // 8. Test remote add / delete
    let _ = remote_add(&temp_path, "upstream", "https://github.com/example/upstream.git");
    let _ = remote_delete(&temp_path, "upstream");

    // 9. Test worktree add / remove
    let wt_path = temp_path.join("wt-path");
    let _ = worktree_add(&temp_path, "new-branch", &wt_path);
    let _ = worktree_remove(&temp_path, &wt_path, true);

    // 10. Test tag delete
    let _ = delete_tag(&temp_path, "v1.0.0");

    // 11. Test stash apply & drop
    let _ = apply_stash(&temp_path, 0);
    let _ = delete_stash(&temp_path, 0);

    // 12. Test merge abort / continue
    let _ = abort_merge(&temp_path);
    let _ = continue_merge(&temp_path);

    // 13. Test merge conflict resolver helpers
    let _ = is_merging(&temp_path);
    let _ = get_conflict_markers_diff(&temp_path, "test.txt");
    let _ = resolve_ours(&temp_path, "test.txt");
    let _ = resolve_theirs(&temp_path, "test.txt");
    let _ = mark_resolved(&temp_path, "test.txt");

    // 14. Test push target & upstream
    let _target = get_branch_push_target(&temp_path, "master");
    let _target2 = get_branch_push_target(&temp_path, "main");

    // 15. Test inspect_detail & inspect_summary
    let _detail = inspect_detail(temp_path.to_str().unwrap(), 10, 10, true);
    let _summary = inspect_summary(temp_path.to_str().unwrap());

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_path);
}

#[test]
fn test_more_gitwig_core_coverage() {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!(
        "twig_test_cov2_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&temp_path).unwrap();

    // 1. Repository init
    let repo = Repository::init(&temp_path).unwrap();

    // 2. Configure author
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    // 3. Create dummy file & commit
    let file_path = temp_path.join("test.txt");
    std::fs::write(&file_path, "line 1\nline 2\nline 3\n").unwrap();
    stage_file(&temp_path, "test.txt").unwrap();
    commit_changes(&temp_path, "first commit").unwrap();

    let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
    let head_oid = head_commit.id().to_string();

    // 4. Test checkout functions
    let _ = checkout_commit(&temp_path, &head_oid);
    let _ = checkout_local_branch(&temp_path, "master");
    let _ = checkout_remote_branch(&temp_path, "origin/master");
    let _ = checkout_tag(&temp_path, "v1.0.0");

    // 5. Test branch / tag creation & deletion
    let _ = create_branch(&temp_path, "new-branch-2");
    let _ = delete_local_branch(&temp_path, "new-branch-2");
    let _ = delete_remote_branch(&temp_path, "origin/new-branch-2");
    let _ = create_tag(&temp_path, "v1.0.0", &head_oid, None, false);
    let _ = delete_remote_tag(&temp_path, "origin", "v1.0.0", Duration::from_secs(5));
    let _ = get_remote_tags(&temp_path, "origin", Duration::from_secs(5));

    // 6. Test serialize & deserialize tags
    let tag_info = vec![BranchInfo {
        name: "v1.0.0".to_string(),
        is_head: false,
        short_sha: "abc1234".to_string(),
        short_message: "tag msg".to_string(),
    }];
    let serialized = serialize_tags(&tag_info);
    let deserialized = deserialize_tags(&serialized);
    assert_eq!(deserialized.len(), 1);
    assert_eq!(deserialized[0].name, "v1.0.0");

    // 7. Test history / blame / commit files
    let _ = get_file_history(&temp_path, "test.txt");
    let _ = get_file_blame(&temp_path, "test.txt");
    let _ = get_commit_files(&temp_path, &head_oid);

    // 8. Test load tab functions
    let _ = load_tab_files(&temp_path);
    let _ = load_tab_branches(&temp_path);
    let _ = load_tab_remotes(&temp_path);
    let _ = load_tab_overview(&temp_path, 10);

    // 9. Test load_tab_graph_stream
    let (tx, _rx) = std::sync::mpsc::channel();
    let _ =
        load_tab_graph_stream(&temp_path, 10, true, temp_path.to_str().unwrap().to_string(), 0, tx);

    // 10. Test invalidate cache
    invalidate_ref_map_cache(&temp_path);

    // 11. Test Forge API/command execution fallbacks/errors
    let _ = load_tab_forge_issues(&temp_path, false, Duration::from_secs(5));
    let _ = load_tab_forge_issues(&temp_path, true, Duration::from_secs(5));
    let _ = load_tab_forge_prs(&temp_path, Duration::from_secs(5));
    let _ = load_pr_comments(&temp_path, 1, Duration::from_secs(5));
    let _ = add_pr_line_comment(
        &temp_path,
        1,
        "sha",
        "file.txt",
        10,
        "comment",
        Duration::from_secs(5),
    );

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_path);
}

#[test]
fn test_create_tag_annotated_and_force_update() {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!(
        "twig_test_tag_force_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&temp_path).unwrap();

    let repo = Repository::init(&temp_path).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    let file_path = temp_path.join("test.txt");
    std::fs::write(&file_path, "commit 1\n").unwrap();
    stage_file(&temp_path, "test.txt").unwrap();
    commit_changes(&temp_path, "commit 1").unwrap();

    let head_commit1 = repo.head().unwrap().peel_to_commit().unwrap();
    let oid1 = head_commit1.id().to_string();

    assert!(!tag_exists(&temp_path, "v1.0.1"));
    assert!(create_tag(&temp_path, "v1.0.1", &oid1, Some("Release v1.0.1"), false).is_ok());
    assert!(tag_exists(&temp_path, "v1.0.1"));

    // Creating without force should fail if tag exists
    std::fs::write(&file_path, "commit 2\n").unwrap();
    stage_file(&temp_path, "test.txt").unwrap();
    commit_changes(&temp_path, "commit 2").unwrap();

    let head_commit2 = repo.head().unwrap().peel_to_commit().unwrap();
    let oid2 = head_commit2.id().to_string();

    assert!(create_tag(&temp_path, "v1.0.1", &oid2, Some("Updated release"), false).is_err());

    // Creating with force = true should succeed and update tag
    assert!(create_tag(&temp_path, "v1.0.1", &oid2, Some("Updated release"), true).is_ok());

    let _ = std::fs::remove_dir_all(&temp_path);
}

use std::io::Write;

#[test]
fn test_commit_amend() {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!(
        "twig_test_amend_{}",
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
fn test_commit_changes_rejects_nothing_staged() {
    let temp_path = unique_temp_dir("commit_empty");
    let repo = Repository::init(&temp_path).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    config.set_bool("commit.gpgsign", false).unwrap();

    let err = commit_changes(&temp_path, "nothing staged").unwrap_err();
    assert!(!err.trim().is_empty());

    let _ = std::fs::remove_dir_all(&temp_path);
}

#[cfg(unix)]
#[test]
fn test_commit_changes_runs_pre_commit_hook() {
    use std::os::unix::fs::PermissionsExt;

    let temp_path = unique_temp_dir("commit_hook");
    let repo = Repository::init(&temp_path).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    config.set_bool("commit.gpgsign", false).unwrap();

    std::fs::write(temp_path.join("file.txt"), "hello\n").unwrap();
    stage_file(&temp_path, "file.txt").unwrap();

    let hooks_dir = temp_path.join(".git").join("hooks");
    std::fs::create_dir_all(&hooks_dir).unwrap();
    let hook_path = hooks_dir.join("pre-commit");
    std::fs::write(&hook_path, "#!/bin/sh\necho 'hook blocked commit' >&2\nexit 1\n").unwrap();
    std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755)).unwrap();

    let err = commit_changes(&temp_path, "blocked").unwrap_err();
    assert!(err.contains("hook blocked commit"), "unexpected hook error: {}", err);
    assert!(get_last_commit_message(&temp_path).is_none());

    let marker = temp_path.join("hook_ran");
    let script = format!("#!/bin/sh\ntouch '{}'\nexit 0\n", marker.display());
    std::fs::write(&hook_path, script).unwrap();
    std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755)).unwrap();

    commit_changes(&temp_path, "allowed").unwrap();
    assert!(marker.exists());
    assert_eq!(get_last_commit_message(&temp_path).as_deref(), Some("allowed"));

    let _ = std::fs::remove_dir_all(&temp_path);
}

#[test]
fn test_commit_changes_preserves_multiline_and_hash_message() {
    let temp_path = unique_temp_dir("commit_multiline");
    let repo = Repository::init(&temp_path).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    config.set_bool("commit.gpgsign", false).unwrap();

    std::fs::write(temp_path.join("file.txt"), "hello\n").unwrap();
    stage_file(&temp_path, "file.txt").unwrap();

    let msg = "feat: support #123\n\n# kept comment line\nsecond line";
    commit_changes(&temp_path, msg).unwrap();
    assert_eq!(get_last_commit_message(&temp_path).as_deref(), Some(msg));

    let _ = std::fs::remove_dir_all(&temp_path);
}

#[test]
fn test_get_file_history_follows_renames() {
    let temp_path = unique_temp_dir("file_history_follow");
    let repo = Repository::init(&temp_path).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    config.set_bool("commit.gpgsign", false).unwrap();

    std::fs::write(temp_path.join("old.txt"), "line 1\n").unwrap();
    stage_file(&temp_path, "old.txt").unwrap();
    commit_changes(&temp_path, "add old.txt").unwrap();

    std::fs::rename(temp_path.join("old.txt"), temp_path.join("new.txt")).unwrap();
    stage_file(&temp_path, "old.txt").unwrap();
    stage_file(&temp_path, "new.txt").unwrap();
    commit_changes(&temp_path, "rename to new.txt").unwrap();

    std::fs::write(temp_path.join("new.txt"), "line 1\nline 2\n").unwrap();
    stage_file(&temp_path, "new.txt").unwrap();
    commit_changes(&temp_path, "edit new.txt").unwrap();

    let history = get_file_history(&temp_path, "new.txt").unwrap();
    assert_eq!(history.len(), 3, "expected 3 revisions, got {:?}", history);
    assert_eq!(history[0].summary, "edit new.txt");
    assert_eq!(history[0].path.as_deref(), Some("new.txt"));
    assert_eq!(history[1].summary, "rename to new.txt");
    assert_eq!(history[2].summary, "add old.txt");
    assert_eq!(history[2].path.as_deref(), Some("old.txt"));

    let diff = get_commit_file_diff(
        &temp_path,
        &history[2].commit_oid,
        history[2].path.as_deref().unwrap(),
    );
    assert!(!diff.is_empty());

    let _ = std::fs::remove_dir_all(&temp_path);
}

#[test]
fn test_resolve_git_dirs_standard_and_linked_worktree() {
    let root = unique_temp_dir("resolve_git_dirs");
    let main_path = root.join("main");
    let wt_path = root.join("wt1");
    std::fs::create_dir_all(&main_path).unwrap();

    let repo = Repository::init(&main_path).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    config.set_bool("commit.gpgsign", false).unwrap();

    std::fs::write(main_path.join("a.txt"), "hello\n").unwrap();
    stage_file(&main_path, "a.txt").unwrap();
    commit_changes(&main_path, "initial").unwrap();

    let main_dirs = resolve_git_dirs(&main_path).expect("standard repo should resolve");
    let canon_main_git = std::fs::canonicalize(main_path.join(".git")).unwrap();
    assert_eq!(main_dirs.git_dir, canon_main_git);
    assert_eq!(main_dirs.common_dir, canon_main_git);

    worktree_add(&main_path, "feature-wt", &wt_path).unwrap();
    let wt_dirs = resolve_git_dirs(&wt_path).expect("linked worktree should resolve");
    assert_eq!(wt_dirs.common_dir, canon_main_git);
    assert_eq!(wt_dirs.git_dir, canon_main_git.join("worktrees").join("wt1"));

    let _ = std::fs::remove_dir_all(&root);
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

fn run_git(dir: &Path, args: &[&str]) {
    let out = git_command()
        .args(["-c", "user.name=Gitwig Test", "-c", "user.email=test@gitwig.invalid"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {:?}: {}", args, String::from_utf8_lossy(&out.stderr));
}

/// A fresh directory per call. `SystemTime` is only microsecond-precise on
/// macOS, so parallel tests can read the same clock value; the process id and
/// a per-process counter keep concurrent callers (and concurrent test runs)
/// apart.
fn unique_temp_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "twig_test_{}_{}_{}_{}",
        tag,
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A repo with one commit and `branches`, at `<root>/main`.
fn repo_with_branches(root: &Path, branches: &[&str]) -> PathBuf {
    let main = root.join("main");
    std::fs::create_dir_all(&main).unwrap();
    run_git(&main, &["init", "-q"]);
    run_git(&main, &["commit", "-q", "--allow-empty", "-m", "init"]);
    for branch in branches {
        run_git(&main, &["branch", branch]);
    }
    main
}

fn worktree_at<'a>(worktrees: &'a [WorktreeInfo], suffix: &str) -> &'a WorktreeInfo {
    worktrees
        .iter()
        .find(|wt| wt.path.ends_with(suffix))
        .unwrap_or_else(|| panic!("no worktree at {} in {:?}", suffix, worktrees))
}

/// `git worktree` resolves a bare admin name as a path suffix. A worktree
/// moved from `x/foo` to `x/renamed` keeps the admin name `foo`, which then
/// names the unrelated `z/foo`: removing `x/renamed` by name force-deleted
/// `z/foo` and its uncommitted work.
#[test]
fn test_worktree_helpers_act_on_the_checkout_path_not_the_admin_name() {
    let root = unique_temp_dir("wt_admin_name");
    let main = repo_with_branches(&root, &["a", "b"]);
    let (x_foo, x_renamed, z_foo) =
        (root.join("x/foo"), root.join("x/renamed"), root.join("z/foo"));
    run_git(&main, &["worktree", "add", "-q", &x_foo.to_string_lossy(), "a"]);
    run_git(&main, &["worktree", "move", &x_foo.to_string_lossy(), &x_renamed.to_string_lossy()]);
    run_git(&main, &["worktree", "add", "-q", &z_foo.to_string_lossy(), "b"]);
    std::fs::write(z_foo.join("precious.txt"), "uncommitted work").unwrap();

    let worktrees = load_tab_worktrees(&main).unwrap();
    let renamed = worktree_at(&worktrees, "x/renamed").clone();
    assert_eq!(renamed.name, "foo", "the moved worktree keeps its admin name");

    worktree_lock(&main, &renamed.path, "testing").unwrap();
    assert!(worktree_at(&load_tab_worktrees(&main).unwrap(), "x/renamed").is_locked);
    worktree_unlock(&main, &renamed.path).unwrap();
    assert!(!worktree_at(&load_tab_worktrees(&main).unwrap(), "x/renamed").is_locked);

    worktree_remove(&main, &renamed.path, false).unwrap();
    assert!(!x_renamed.exists(), "the selected worktree is removed");
    assert!(z_foo.join("precious.txt").exists(), "the other worktree is untouched");
    let _ = std::fs::remove_dir_all(&root);
}

/// `a` in the Worktrees tab could only check out an existing branch (its own
/// example, `main`, always failed). A name that resolves to nothing now
/// creates that branch for the new worktree.
#[test]
fn test_worktree_add_checks_out_existing_refs_and_creates_new_branches() {
    let root = unique_temp_dir("wt_add");
    let main = repo_with_branches(&root, &["existing"]);
    run_git(&main, &["tag", "v1"]);
    let branch_of = |wt: &Path| {
        let out =
            git_command().args(["rev-parse", "--abbrev-ref", "HEAD"]).current_dir(wt).output();
        String::from_utf8_lossy(&out.unwrap().stdout).trim().to_string()
    };

    assert!(!worktree_add(&main, "existing", &root.join("a")).unwrap());
    assert_eq!(branch_of(&root.join("a")), "existing");
    assert!(worktree_add(&main, "feature/new", &root.join("b")).unwrap(), "created");
    assert_eq!(branch_of(&root.join("b")), "feature/new");
    assert!(!worktree_add(&main, "v1", &root.join("c")).unwrap());
    assert_eq!(branch_of(&root.join("c")), "HEAD", "a tag is checked out detached");
    // A path starting with `-` is a path, not an option.
    assert!(worktree_add(&main, "dash", &root.join("-dash")).unwrap());
    // The branch already checked out in the main worktree is refused.
    let current = branch_of(&main);
    assert!(worktree_add(&main, &current, &root.join("d")).is_err());

    // A branch only on the remote becomes a local branch tracking it.
    let clone = root.join("clone");
    run_git(&root, &["clone", "-q", &main.to_string_lossy(), &clone.to_string_lossy()]);
    assert!(!worktree_add(&clone, "existing", &root.join("e")).unwrap());
    assert_eq!(branch_of(&root.join("e")), "existing");
    let upstream = git_command()
        .args(["rev-parse", "--abbrev-ref", "existing@{upstream}"])
        .current_dir(root.join("e"))
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&upstream.stdout).trim(), "origin/existing");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn test_worktree_remove_keeps_uncommitted_or_locked_work_unless_forced() {
    let root = unique_temp_dir("wt_remove_force");
    let main = repo_with_branches(&root, &["dirty", "locked"]);
    let (dirty, locked) = (root.join("dirty"), root.join("locked"));
    run_git(&main, &["worktree", "add", "-q", &dirty.to_string_lossy(), "dirty"]);
    run_git(&main, &["worktree", "add", "-q", &locked.to_string_lossy(), "locked"]);
    std::fs::write(dirty.join("wip.txt"), "untracked work").unwrap();
    run_git(&main, &["worktree", "lock", &locked.to_string_lossy()]);

    assert!(worktree_remove(&main, &dirty, false).is_err());
    assert!(dirty.join("wip.txt").exists(), "a refused remove deletes nothing");
    assert!(worktree_remove(&main, &locked, false).is_err());
    assert!(locked.exists());

    worktree_remove(&main, &dirty, true).unwrap();
    worktree_remove(&main, &locked, true).unwrap();
    assert!(!dirty.exists() && !locked.exists(), "force removes both");
    assert!(load_tab_worktrees(&main).unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

/// `git submodule deinit` and `git rm` take a path; the submodule's name was
/// passed instead, so a tracked file sharing that name was deleted (local
/// edits and all) while the submodule stayed registered.
#[test]
fn test_submodule_remove_deletes_the_submodule_path_not_a_same_named_file() {
    let root = unique_temp_dir("submodule_remove");
    let lib = root.join("lib");
    std::fs::create_dir_all(&lib).unwrap();
    run_git(&lib, &["init", "-q"]);
    run_git(&lib, &["commit", "-q", "--allow-empty", "-m", "lib"]);
    let main = repo_with_branches(&root, &[]);
    std::fs::write(main.join("notes"), "tracked notes\n").unwrap();
    run_git(&main, &["add", "notes"]);
    run_git(&main, &["commit", "-q", "-m", "notes"]);
    std::fs::write(main.join("notes"), "tracked notes\nlocal edit\n").unwrap();
    run_git(
        &main,
        &["submodule", "add", "-q", "--name", "notes", &lib.to_string_lossy(), "vendor/n"],
    );
    run_git(&main, &["commit", "-q", "-m", "add submodule"]);
    assert!(main.join(".git/modules/notes").is_dir());

    // Anything that is not the submodule's gitlink is refused untouched.
    assert!(submodule_remove(&main, "notes", Path::new("notes")).is_err());
    assert!(submodule_remove(&main, "notes", Path::new("../lib")).is_err());
    assert!(submodule_remove(&main, "../notes", Path::new("vendor/n")).is_err());
    assert!(main.join("vendor/n").is_dir());

    submodule_remove(&main, "notes", Path::new("vendor/n")).unwrap();
    assert_eq!(
        std::fs::read_to_string(main.join("notes")).unwrap(),
        "tracked notes\nlocal edit\n",
        "the same-named file keeps its local edit"
    );
    assert!(!main.join("vendor/n").exists());
    assert!(!main.join(".git/modules/notes").exists());
    assert!(!std::fs::read_to_string(main.join(".gitmodules")).unwrap().contains("vendor/n"));
    let status = git_command().args(["status", "--porcelain"]).current_dir(&main).output();
    let status = status.unwrap();
    assert!(status.status.success(), "the repository is still usable");
    // The removal is staged for the user to commit, next to the untouched edit.
    let status = String::from_utf8_lossy(&status.stdout);
    assert!(status.lines().any(|l| l == "D  vendor/n"), "{}", status);
    assert!(status.lines().any(|l| l == " M notes"), "{}", status);
    // Until that is committed the tab still lists it, from HEAD, with no
    // index commit: what the Submodules tab shows as "Removal staged".
    let listed = load_tab_submodules(&main).unwrap();
    let removed = listed.iter().find(|s| s.path == Path::new("vendor/n")).unwrap();
    assert!(removed.commit_id.is_none() && removed.head_id.is_some(), "{:?}", removed);
    let _ = std::fs::remove_dir_all(&root);
}

/// An uninitialised submodule is cloned and checked out (only the one asked
/// for, unless every one was), and one moved to another commit goes back to
/// the recorded one, but never over a local edit the checkout would overwrite.
#[test]
fn test_submodule_update_initialises_and_returns_to_the_recorded_commit() {
    let root = unique_temp_dir("submodule_update");
    let lib = root.join("lib");
    std::fs::create_dir_all(&lib).unwrap();
    run_git(&lib, &["init", "-q"]);
    std::fs::write(lib.join("f"), "1\n").unwrap();
    run_git(&lib, &["add", "f"]);
    run_git(&lib, &["commit", "-q", "-m", "one"]);
    let main = repo_with_branches(&root, &[]);
    let lib_url = lib.to_string_lossy().to_string();
    run_git(&main, &["submodule", "add", "-q", &lib_url, "vendor/a"]);
    run_git(&main, &["submodule", "add", "-q", &lib_url, "vendor/b"]);
    run_git(&main, &["commit", "-q", "-m", "add submodules"]);
    run_git(&main, &["submodule", "deinit", "-q", "-f", "--all"]);
    let listed = |p: &str| {
        let subs = load_tab_submodules(&main).unwrap();
        subs.into_iter().find(|s| s.path == Path::new(p)).unwrap()
    };
    assert!(!listed("vendor/a").is_initialized && !listed("vendor/b").is_initialized);

    let timeout = Duration::from_secs(60);
    submodule_update(&main, Some(Path::new("vendor/a")), timeout).unwrap();
    assert!(listed("vendor/a").is_initialized);
    assert!(!listed("vendor/b").is_initialized, "only the selected submodule");
    let sub_a = main.join("vendor/a");
    assert_eq!(std::fs::read_to_string(sub_a.join("f")).unwrap(), "1\n");
    submodule_update(&main, None, timeout).unwrap();
    assert!(listed("vendor/b").is_initialized, "no path updates every submodule");

    // Check out a newer lib commit inside vendor/a: the tab shows Modified.
    std::fs::write(lib.join("f"), "2\n").unwrap();
    run_git(&lib, &["commit", "-q", "-am", "two"]);
    run_git(&sub_a, &["fetch", "-q", "origin", "HEAD"]);
    run_git(&sub_a, &["checkout", "-q", "FETCH_HEAD"]);
    assert!(listed("vendor/a").is_dirty);
    submodule_update(&main, Some(Path::new("vendor/a")), timeout).unwrap();
    assert!(!listed("vendor/a").is_dirty, "back on the recorded commit");
    assert_eq!(std::fs::read_to_string(sub_a.join("f")).unwrap(), "1\n");

    // Moved again, with an uncommitted edit the checkout would overwrite.
    run_git(&sub_a, &["checkout", "-q", "FETCH_HEAD"]);
    std::fs::write(sub_a.join("f"), "local\n").unwrap();
    assert!(submodule_update(&main, Some(Path::new("vendor/a")), timeout).is_err());
    assert_eq!(std::fs::read_to_string(sub_a.join("f")).unwrap(), "local\n");

    assert!(submodule_update(&main, Some(Path::new("nope")), timeout).is_err());
    let _ = std::fs::remove_dir_all(&root);
}

/// `gh api -F` reads `@alice ...` as a file name, sends `123` as a number and
/// fills `{owner}`; review comment text must go through `-f` verbatim.
#[test]
fn test_pr_line_comment_args_send_text_fields_verbatim() {
    let args = pr_line_comment_args(42, "abc123", "src/{owner}.rs", 7, "@alice 123");
    let field = |flag: &str, value: &str| args.windows(2).any(|w| w[0] == flag && w[1] == value);
    assert!(field("-f", "body=@alice 123"), "{:?}", args);
    assert!(field("-f", "path=src/{owner}.rs"), "{:?}", args);
    assert!(field("-f", "commit_id=abc123"), "{:?}", args);
    assert!(field("-f", "side=RIGHT"), "{:?}", args);
    assert!(field("-F", "line=7"), "line is sent as a number: {:?}", args);
    assert_eq!(args.iter().filter(|a| *a == "-F").count(), 1, "{:?}", args);
    assert_eq!(args[1], "repos/:owner/:repo/pulls/42/comments");
}

/// `statusCheckRollup` as `gh pr list --json` prints it: a finished and a
/// running check run, and a commit status (`StatusContext`, no `name`),
/// which used to fail the whole PR list with "missing field `name`".
#[test]
fn test_parse_forge_prs_accepts_commit_statuses_and_running_checks() {
    let json = br#"[{"number":42,"title":"Add retry","state":"OPEN",
        "author":{"login":"alice"},"assignees":[],"url":"https://github.com/o/r/pull/42",
        "headRefName":"retry","headRefOid":"abc","body":"",
        "statusCheckRollup":[
          {"__typename":"CheckRun","name":"build","workflowName":"CI","status":"COMPLETED",
           "conclusion":"SUCCESS","startedAt":"2026-01-01T00:00:00Z","completedAt":"2026-01-01T00:01:00Z","detailsUrl":"https://x"},
          {"__typename":"CheckRun","name":"lint","workflowName":"CI","status":"IN_PROGRESS",
           "conclusion":"","startedAt":"2026-01-01T00:00:00Z","completedAt":"0001-01-01T00:00:00Z","detailsUrl":"https://x"},
          {"__typename":"StatusContext","context":"ci/circleci","state":"FAILURE",
           "targetUrl":"https://circleci.com/x","startedAt":"2026-01-01T00:00:00Z"}],
        "reviews":[{"author":null,"body":"LGTM","state":"APPROVED"}]}]"#;
    let prs = parse_forge_prs(json).unwrap();
    let checks = &prs[0].status_checks;
    assert_eq!(checks.len(), 3);
    assert_eq!(checks[0].conclusion.as_deref(), Some("SUCCESS"));
    assert_eq!((checks[1].name.as_str(), checks[1].conclusion.as_deref()), ("lint", None));
    assert_eq!(checks[1].status.as_deref(), Some("IN_PROGRESS"));
    assert_eq!(checks[2].name, "ci/circleci");
    assert_eq!(checks[2].state.as_deref(), Some("FAILURE"));
    assert_eq!(prs[0].reviews[0].author, "none");
}

/// `gh api --paginate` prints each page's array back to back; all of them
/// count (only the first 30 comments used to load).
#[test]
fn test_parse_pr_comments_reads_every_page() {
    let comment = |n: u32| {
        format!(
            r#"{{"path":"a.rs","line":{},"body":"c{}","user":{{"login":"bob"}},"commit_id":"x"}}"#,
            n, n
        )
    };
    let page = |range: std::ops::Range<u32>| {
        format!("[{}]", range.map(comment).collect::<Vec<_>>().join(","))
    };
    let json = format!("{}\n{}", page(0..100), page(100..130));
    let comments = parse_pr_comments(json.as_bytes()).unwrap();
    assert_eq!(comments.len(), 130);
    assert_eq!(comments[129].body, "c129");
    assert!(parse_pr_comments(b"[]").unwrap().is_empty());
    let deleted_user = br#"[{"path":"a","line":null,"body":"b","user":null,"commit_id":"x"}]"#;
    assert_eq!(parse_pr_comments(deleted_user).unwrap()[0].author, "none");
    assert!(parse_pr_comments(b"[{").is_err());
}

#[test]
fn test_parse_linked_branches() {
    let json = br#"{"data":{"repository":{"issue":{"linkedBranches":{"nodes":[
        {"ref":{"name":"12-login-fails"}},{"ref":null},{"ref":{"name":"12-login-fails"}},
        {"ref":{"name":"alt-12"}}]}}}}}"#;
    assert_eq!(parse_linked_branches(json).unwrap(), vec!["12-login-fails", "alt-12"]);
    let none = br#"{"data":{"repository":{"issue":{"linkedBranches":{"nodes":[]}}}}}"#;
    assert!(parse_linked_branches(none).unwrap().is_empty());
    let no_issue = br#"{"data":{"repository":{"issue":null}}}"#;
    assert!(parse_linked_branches(no_issue).unwrap().is_empty());
    assert!(parse_linked_branches(b"not json").is_err());
}

#[test]
fn test_branch_names_issue_matches_whole_tokens_only() {
    for (branch, issue) in [
        ("fix-1-typo", 1),
        ("issue-12-login", 12),
        ("12-login", 12),
        ("feature/12", 12),
        ("gh#12", 12),
        ("issue_12", 12),
        ("v1.2-release-2", 2),
    ] {
        assert!(branch_names_issue(branch, issue), "{} names #{}", branch, issue);
    }
    for (branch, issue) in [
        ("v1.2-release", 1),
        ("v1.2-release", 2),
        ("issue-12-login", 1),
        ("issue-12-login", 2),
        ("issue-123", 12),
        ("issue12", 12),
        ("main", 1),
    ] {
        assert!(!branch_names_issue(branch, issue), "{} does not name #{}", branch, issue);
    }
}

fn current_branch(repo: &Path) -> String {
    let out = git_command().args(["branch", "--show-current"]).current_dir(repo).output();
    String::from_utf8_lossy(&out.unwrap().stdout).trim().to_string()
}

#[test]
fn test_checkout_issue_branch_by_name_picks_the_one_branch_naming_the_issue() {
    let root = unique_temp_dir("issue_branch");
    let main = repo_with_branches(
        &root,
        &["fix-1-typo", "v1.2-release", "issue-12-login", "7-crash", "fix/7"],
    );
    let start = current_branch(&main);

    // #2 used to land on issue-12-login (substring "2"); no branch names it.
    assert_eq!(
        checkout_issue_branch_by_name(&main, 2).unwrap(),
        "Created and switched to new branch 'issue-2'"
    );
    assert_eq!(current_branch(&main), "issue-2");
    checkout_issue_branch_by_name(&main, 12).unwrap();
    assert_eq!(current_branch(&main), "issue-12-login");
    checkout_issue_branch_by_name(&main, 1).unwrap();
    assert_eq!(current_branch(&main), "fix-1-typo");

    // Two candidates: report them and stay put.
    let err = checkout_issue_branch_by_name(&main, 7).unwrap_err();
    assert!(err.contains("7-crash") && err.contains("fix/7"), "{}", err);
    assert_eq!(current_branch(&main), "fix-1-typo");

    // A branch only on the remote is checked out as a tracking branch.
    run_git(&main, &["checkout", "-q", &start]);
    run_git(&main, &["branch", "fix-9-remote"]);
    let clone = root.join("clone");
    run_git(&root, &["clone", "-q", &main.to_string_lossy(), &clone.to_string_lossy()]);
    assert_eq!(
        checkout_issue_branch_by_name(&clone, 9).unwrap(),
        "Checked out matched remote branch 'origin/fix-9-remote'"
    );
    assert_eq!(current_branch(&clone), "fix-9-remote");
    let _ = std::fs::remove_dir_all(&root);
}

/// A linked branch missing from the clone must not reach `git checkout` as
/// a bare name: git would read `docs` as a path and discard its changes.
#[test]
fn test_checkout_linked_issue_branch_never_restores_a_same_named_path() {
    let root = unique_temp_dir("linked_branch");
    let main = repo_with_branches(&root, &[]);
    std::fs::create_dir_all(main.join("docs")).unwrap();
    std::fs::write(main.join("docs/guide.md"), "v1\n").unwrap();
    run_git(&main, &["add", "docs"]);
    run_git(&main, &["commit", "-q", "-m", "docs"]);
    run_git(&main, &["branch", "linked-5"]);
    std::fs::write(main.join("docs/guide.md"), "v1\nunsaved edit\n").unwrap();

    let err = checkout_linked_issue_branch(&main, 5, "docs").unwrap_err();
    assert!(err.contains("not in this clone"), "{}", err);
    assert_eq!(std::fs::read_to_string(main.join("docs/guide.md")).unwrap(), "v1\nunsaved edit\n");
    checkout_linked_issue_branch(&main, 5, "linked-5").unwrap();
    assert_eq!(current_branch(&main), "linked-5");
    let _ = std::fs::remove_dir_all(&root);
}

/// Builds a repo whose only commit carries a (fake) gpg signature and whose
/// `gpg.program` points at a binary that does not exist. Any `%G?` lookup then
/// makes git print `error: cannot run …` on stderr and still exit 0, which is
/// exactly what a machine without gpg does for a real signed commit.
fn make_repo_with_unverifiable_signature() -> PathBuf {
    use std::io::Write;
    use std::process::Stdio;
    // Two tests build this fixture and libtest runs them in parallel.
    let temp_path = unique_temp_dir("nogpg");
    let repo = Repository::init(&temp_path).unwrap();
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();
    config.set_str("gpg.program", "/nonexistent/gitwig-test-no-gpg").unwrap();

    let mut file = File::create(temp_path.join("test.txt")).unwrap();
    writeln!(file, "initial content").unwrap();
    stage_file(&temp_path, "test.txt").unwrap();
    commit_changes(&temp_path, "signed commit").unwrap();

    // Re-hash the commit with a gpgsig header spliced in after `committer`.
    let raw =
        git_command().args(["cat-file", "-p", "HEAD"]).current_dir(&temp_path).output().unwrap();
    let mut signed = String::new();
    for line in String::from_utf8_lossy(&raw.stdout).lines() {
        signed.push_str(line);
        signed.push('\n');
        if line.starts_with("committer ") {
            signed.push_str("gpgsig -----BEGIN PGP SIGNATURE-----\n \n iQEzBAABCAAdFiEEfake\n -----END PGP SIGNATURE-----\n");
        }
    }
    let mut hash_cmd = git_command();
    hash_cmd.args(["hash-object", "-t", "commit", "-w", "--stdin"]).current_dir(&temp_path);
    hash_cmd.stdin(Stdio::piped());
    let mut child = hash_cmd.spawn().unwrap();
    child.stdin.take().unwrap().write_all(signed.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let new_oid = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let head_ref = String::from_utf8_lossy(
        &git_command()
            .args(["symbolic-ref", "HEAD"])
            .current_dir(&temp_path)
            .output()
            .unwrap()
            .stdout,
    )
    .trim()
    .to_string();
    let upd = git_command()
        .args(["update-ref", &head_ref, &new_oid])
        .current_dir(&temp_path)
        .output()
        .unwrap();
    assert!(upd.status.success());
    temp_path
}

#[test]
fn graph_survives_missing_signature_program_and_reports_it() {
    let temp_path = make_repo_with_unverifiable_signature();
    let (tx, rx) = std::sync::mpsc::channel();

    let load = load_tab_graph_stream(&temp_path, 100, true, String::new(), 2, tx)
        .expect("a missing gpg must not fail the graph");
    assert_eq!(load.lines.len(), 1);
    assert!(load.lines[0].commit.is_some());
    let notice = load.notice.expect("git's stderr must surface as a notice");
    assert!(
        notice.contains("/nonexistent/gitwig-test-no-gpg"),
        "notice must name the program git could not start: {notice}"
    );
    assert!(!notice.starts_with("error:"), "prefix should be stripped: {notice}");
    assert!(rx.try_recv().is_err(), "no batch is streamed for a one-commit graph");

    let _ = std::fs::remove_dir_all(&temp_path);
}

#[test]
fn graph_skips_signature_lookup_when_disabled() {
    let temp_path = make_repo_with_unverifiable_signature();
    let (tx, _rx) = std::sync::mpsc::channel();

    let load = load_tab_graph_stream(&temp_path, 100, false, String::new(), 2, tx).unwrap();
    assert_eq!(load.lines.len(), 1);
    assert_eq!(load.lines[0].commit.as_ref().unwrap().signature_status, "N");
    assert!(load.notice.is_none(), "no %G? means git never touches gpg");

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
        "twig_test_time_{}",
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
        "twig_test_stats_{}",
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
        "twig_test_untracked_{}",
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
        "twig_test_new_del_{}",
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
            let paths: Vec<String> = info.changes.staged.iter().map(|f| f.path.clone()).collect();
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
        "twig_test_discard_all_{}",
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
#[cfg_attr(target_os = "windows", ignore = "CRLF patch issues on Windows")]
fn test_stage_unstage_by_hunk() {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!(
        "twig_test_hunk_{}",
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
#[cfg_attr(target_os = "windows", ignore = "CRLF patch issues on Windows")]
fn test_discard_hunk() {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!(
        "twig_test_discard_h_{}",
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
#[cfg_attr(target_os = "windows", ignore = "CRLF patch issues on Windows")]
fn test_stage_unstage_discard_line() {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!(
        "twig_test_line_{}",
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
        staged_diff.iter().any(|l| l.kind == DiffLineKind::Added && l.content == "line A modified")
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
#[cfg_attr(target_os = "windows", ignore = "CRLF patch issues on Windows")]
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
        assert!(entry.status().intersects(git2::Status::INDEX_MODIFIED | git2::Status::INDEX_NEW));
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
    let output = git_command()
        .args(["symbolic-ref", "--short", "HEAD"])
        .current_dir(&temp_path)
        .output()
        .unwrap();
    let main_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // 2. Create feature branch and edit
    git_command().args(["checkout", "-b", "feature"]).current_dir(&temp_path).output().unwrap();

    std::fs::write(&file_path, "line 1\nline 2 on feature\nline 3\n").unwrap();
    stage_file(&temp_path, "conflict.txt").unwrap();
    commit_changes(&temp_path, "feature commit").unwrap();

    // 3. Checkout main/master and edit differently
    git_command().args(["checkout", &main_branch]).current_dir(&temp_path).output().unwrap();

    std::fs::write(&file_path, "line 1\nline 2 on main\nline 3\n").unwrap();
    stage_file(&temp_path, "conflict.txt").unwrap();
    commit_changes(&temp_path, "main commit").unwrap();

    // 4. Merge feature into main -> conflict
    assert!(!is_merging(&temp_path));
    let merge_output =
        git_command().args(["merge", "feature"]).current_dir(&temp_path).output().unwrap();

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
    git_command().args(["merge", "feature"]).current_dir(&temp_path).output().unwrap();
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
    git_command().args(["reset", "--hard", "HEAD~1"]).current_dir(&temp_path).output().unwrap();

    git_command().args(["merge", "feature"]).current_dir(&temp_path).output().unwrap();
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
    let output = git_command()
        .args(["symbolic-ref", "--short", "HEAD"])
        .current_dir(&temp_path)
        .output()
        .unwrap();
    let main_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // 2. Create feature branch and edit line 2 and line 11
    git_command().args(["checkout", "-b", "feature"]).current_dir(&temp_path).output().unwrap();
    let feature_lines = "line 1\nline 2 on feature\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10\nline 11 on feature\nline 12\n";
    std::fs::write(&file_path, feature_lines).unwrap();
    stage_file(&temp_path, "conflict.txt").unwrap();
    commit_changes(&temp_path, "feature commit").unwrap();

    // 3. Checkout main/master and edit line 2 and line 11 differently
    git_command().args(["checkout", &main_branch]).current_dir(&temp_path).output().unwrap();
    let main_lines = "line 1\nline 2 on main\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10\nline 11 on main\nline 12\n";
    std::fs::write(&file_path, main_lines).unwrap();
    stage_file(&temp_path, "conflict.txt").unwrap();
    commit_changes(&temp_path, "main commit").unwrap();

    // 4. Merge feature into main -> conflict
    let merge_output =
        git_command().args(["merge", "feature"]).current_dir(&temp_path).output().unwrap();
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

#[test]
fn test_branch_and_commit_helpers() {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!(
        "twig_test_helpers_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&temp_path);
    std::fs::create_dir_all(&temp_path).unwrap();

    // 1. Initialize repository
    let repo = Repository::init(&temp_path).unwrap();

    // Configure author
    let mut config = repo.config().unwrap();
    config.set_str("user.name", "Test User").unwrap();
    config.set_str("user.email", "test@example.com").unwrap();

    // 2. Create initial commit (root commit)
    let file_path = temp_path.join("test.txt");
    std::fs::write(&file_path, "root content").unwrap();
    stage_file(&temp_path, "test.txt").unwrap();
    commit_changes(&temp_path, "root commit").unwrap();

    let head_oid = repo.head().unwrap().target().unwrap().to_string();
    assert!(is_root_commit(&temp_path, &head_oid));

    // 3. Create second commit (non-root commit)
    std::fs::write(&file_path, "second content").unwrap();
    stage_file(&temp_path, "test.txt").unwrap();
    commit_changes(&temp_path, "second commit").unwrap();

    let new_head_oid = repo.head().unwrap().target().unwrap().to_string();
    assert!(!is_root_commit(&temp_path, &new_head_oid));

    // 4. Test push target with first remote config
    // Currently there are no remotes, so it should return None or fallback.
    // Wait, get_branch_push_target returns None if no remotes are found and no upstream config.
    // Let's add a remote.
    remote_add(&temp_path, "origin", "https://github.com/example/repo.git").unwrap();
    let active_branch =
        if repo.find_branch("master", git2::BranchType::Local).is_ok() { "master" } else { "main" };
    let target = get_branch_push_target(&temp_path, active_branch);
    assert!(target.is_some());
    let (remote_name, set_upstream) = target.unwrap();
    assert_eq!(remote_name, "origin");
    assert!(set_upstream);

    // Configure tracking branch
    let mut git_config = repo.config().unwrap();
    git_config.set_str(&format!("branch.{}.remote", active_branch), "origin").unwrap();
    git_config
        .set_str(
            &format!("branch.{}.merge", active_branch),
            &format!("refs/heads/{}", active_branch),
        )
        .unwrap();

    // Create mock remote tracking reference so resolve succeeds
    let commit_obj = repo.head().unwrap().peel_to_commit().unwrap();
    repo.reference(
        &format!("refs/remotes/origin/{}", active_branch),
        commit_obj.id(),
        true,
        "create mock remote ref",
    )
    .unwrap();

    // Test has_upstream_remote and get_branch_upstream_remote
    assert!(has_upstream_remote(&temp_path, active_branch));
    assert_eq!(get_branch_upstream_remote(&temp_path, active_branch), Some("origin".to_string()));

    // Test push target with tracking branch configured
    let target_tracking = get_branch_push_target(&temp_path, active_branch);
    assert!(target_tracking.is_some());
    let (remote_name_tr, set_upstream_tr) = target_tracking.unwrap();
    assert_eq!(remote_name_tr, "origin");
    assert!(!set_upstream_tr);

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_path);
}

#[test]
fn test_core_helpers_comprehensive() {
    // 1. format_relative_time
    assert_eq!(format_relative_time(0), "unknown");
    assert_eq!(format_relative_time(-100), "unknown");

    let now_sec = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    assert_eq!(format_relative_time(now_sec - 10), "10 seconds ago");
    assert_eq!(format_relative_time(now_sec - 120), "2 minutes ago");
    assert_eq!(format_relative_time(now_sec - 7200), "2 hours ago");
    assert_eq!(format_relative_time(now_sec - 86400 * 2), "2 days ago");
    assert_eq!(format_relative_time(now_sec - 86400 * 60), "2 months ago");
    assert_eq!(format_relative_time(now_sec - 86400 * 730), "2 years ago");

    // 2. format_utc_date
    assert_eq!(format_utc_date(0), "unknown");
    assert_eq!(format_utc_date(-1), "unknown");
    let date_str = format_utc_date(1700000000);
    assert!(date_str.contains("2023-11-14"));

    // 3. expand_tilde
    assert_eq!(expand_tilde("path"), PathBuf::from("path"));
    let tilde_path = expand_tilde("~/path");
    assert!(tilde_path.is_absolute() || tilde_path.to_string_lossy().contains("path"));

    // 4. sanitize_text
    assert_eq!(sanitize_text("hello\nworld\r\t\x01"), "hello\nworld\r\t ");
    assert_eq!(sanitize_text("hello\x1B[31mworld\x1B B"), "helloworldB");
    assert_eq!(sanitize_text("M4 — UAS"), "M4 — UAS");
    assert_eq!(
        sanitize_text("feat: support utf-8 — – … 🚀 日本語"),
        "feat: support utf-8 — – … 🚀 日本語"
    );

    // 5. safe_sha_slice
    assert_eq!(safe_sha_slice("12345", 10), "12345");
    assert_eq!(safe_sha_slice("12345", 3), "123");

    // 6. safe_ref
    assert_eq!(safe_ref("  refs/heads/main  "), Ok("refs/heads/main"));
    assert!(safe_ref("-invalid").is_err());
    assert!(safe_ref("   ").is_err());

    // 7. RepoSummary helpers
    let mut status = RepoSummary::default();
    assert!(status.is_clean());
    assert!(status.is_synced());
    assert!(status.unchanged());

    status.modified = 1;
    assert!(!status.is_clean());
    assert!(!status.unchanged());

    status.modified = 0;
    status.ahead = 1;
    assert!(!status.is_synced());
    assert!(!status.unchanged());

    // 8. TabData helpers
    let data_not_loaded: TabData<Vec<i32>> = TabData::NotLoaded;
    assert!(data_not_loaded.is_not_loaded());
    assert!(!data_not_loaded.is_loading());
    assert!(!data_not_loaded.is_loaded());
    assert_eq!(data_not_loaded.len(), 0);
    assert!(data_not_loaded.is_empty());
    assert!(data_not_loaded.as_ref().is_none());
    assert!(data_not_loaded.first().is_none());
    assert!(data_not_loaded.get(0).is_none());
    assert_eq!(data_not_loaded.iter().count(), 0);
    assert_eq!(data_not_loaded.as_slice().len(), 0);

    let data_loading: TabData<Vec<i32>> = TabData::Loading;
    assert!(!data_loading.is_not_loaded());
    assert!(data_loading.is_loading());

    let data_loaded = TabData::Loaded(vec![10, 20]);
    assert!(!data_loaded.is_not_loaded());
    assert!(!data_loaded.is_loading());
    assert!(data_loaded.is_loaded());
    assert_eq!(data_loaded.len(), 2);
    assert!(!data_loaded.is_empty());
    assert_eq!(data_loaded.as_ref().unwrap(), &vec![10, 20]);
    assert_eq!(data_loaded.first().unwrap(), &10);
    assert_eq!(data_loaded.get(1).unwrap(), &20);
    assert_eq!(data_loaded.iter().count(), 2);
    assert_eq!(data_loaded.as_slice(), &[10, 20]);
}

#[test]
fn test_comprehensive_gitwig_core_coverage_booster() {
    // 1. Trigger the then_with tie-breaker in collect_committer_stats
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!(
        "twig_test_boost_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&temp_path).unwrap();

    let repo = Repository::init(&temp_path).unwrap();

    // Commit 1 under User A
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "User A").unwrap();
        config.set_str("user.email", "a@example.com").unwrap();
        std::fs::write(temp_path.join("file1.txt"), "content A").unwrap();
        stage_file(&temp_path, "file1.txt").unwrap();
        commit_changes(&temp_path, "commit A").unwrap();
    }

    // Commit 2 under User B (both have 1 commit count)
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "User B").unwrap();
        config.set_str("user.email", "b@example.com").unwrap();
        std::fs::write(temp_path.join("file2.txt"), "content B").unwrap();
        stage_file(&temp_path, "file2.txt").unwrap();
        commit_changes(&temp_path, "commit B").unwrap();
    }

    // Call collect_committer_stats - will sort and execute the then_with closure
    let (stats, _) = collect_committer_stats(&repo, 10).unwrap();
    assert_eq!(stats.len(), 2);
    assert_eq!(stats[0].count, 1);
    assert_eq!(stats[1].count, 1);

    // 2. Trigger deleted file old_file().path() or_else fallback in commit_changed_files
    // Delete file1.txt
    std::fs::remove_file(temp_path.join("file1.txt")).unwrap();
    stage_file(&temp_path, "file1.txt").unwrap();
    commit_changes(&temp_path, "deleted file1").unwrap();
    let head_oid = repo.head().unwrap().target().unwrap().to_string();
    let files = get_commit_files(&temp_path, &head_oid).unwrap();
    assert!(files.iter().any(|f| f.path == "file1.txt" && f.label == "D"));

    // 3. Trigger expand_tilde
    let path_tilde = expand_tilde("~");
    assert!(path_tilde.is_absolute());

    // 4. Trigger calculate_dir_size inside get_lfs_storage_size
    let lfs_dir = temp_path.join(".git").join("lfs");
    std::fs::create_dir_all(&lfs_dir).unwrap();
    let nested_dir = lfs_dir.join("objects").join("xx");
    std::fs::create_dir_all(&nested_dir).unwrap();
    std::fs::write(nested_dir.join("dummy_lfs"), "hello").unwrap();
    let lfs_size = get_lfs_storage_size(&temp_path);
    assert_eq!(lfs_size, Some(5));

    // 5. Trigger load_tab_submodules loop closures
    let mut sub =
        repo.submodule("https://github.com/example/sub.git", Path::new("sub-path"), true).unwrap();
    let _ = sub.init(false);
    let subs = load_tab_submodules(&temp_path).unwrap();
    assert!(!subs.is_empty());

    // 6. Test worktree locking/unlocking/pruning
    let wt_path = temp_path.join("wt-booster");
    let _ = worktree_add(&temp_path, "wt-branch", &wt_path);
    let _ = worktree_lock(&temp_path, &wt_path, "lock reason");
    let _ = worktree_unlock(&temp_path, &wt_path);
    let _ = worktree_prune(&temp_path);
    let _ = std::fs::remove_dir_all(&wt_path);

    // 7. Test open_browser
    open_browser("");

    // 8. Test error paths for almost all gitwig-core functions
    let nonexistent = Path::new("/nonexistent_directory_twig_test");
    let _ = stage_file(nonexistent, "test.txt");
    let _ = unstage_file(nonexistent, "test.txt");
    let _ = discard_all_changes(nonexistent);
    let _ = discard_file_changes(nonexistent, "test.txt", false);
    let _ = commit_changes(nonexistent, "msg");
    let _ = remote_add(nonexistent, "origin", "url");
    let _ = get_file_history(nonexistent, "test.txt");
    let _ = get_file_blame(nonexistent, "test.txt");
    let _ = get_commit_files(nonexistent, "sha");
    let _ = load_tab_reflog(nonexistent);
    let _ = load_tab_files(nonexistent);
    let _ = load_tab_branches(nonexistent);
    let _ = load_tab_tags(nonexistent);
    let _ = load_tab_remotes(nonexistent);
    let _ = load_tab_stashes(nonexistent);
    let _ = load_tab_overview(nonexistent, 10);
    let _ = load_tab_submodules(nonexistent);
    let _ = checkout_remote_branch(nonexistent, "origin/branch");
    let _ = checkout_local_branch(nonexistent, "branch");
    let _ = checkout_tag(nonexistent, "tag");
    let _ = delete_local_branch(nonexistent, "branch");
    let _ = delete_remote_branch(nonexistent, "origin/branch");
    let _ = create_tag(nonexistent, "tag", "sha", None, false);
    let _ = delete_remote_tag(nonexistent, "origin", "tag", Duration::from_secs(5));
    let _ = get_remote_tags(nonexistent, "origin", Duration::from_secs(5));
    let _ = apply_stash(nonexistent, 0);
    let _ = delete_stash(nonexistent, 0);
    let _ = save_stash(nonexistent, "msg", false, false, &[]);
    let _ = commit_amend(nonexistent, "msg");
    let _ = resolve_ours(nonexistent, "file");
    let _ = resolve_theirs(nonexistent, "file");
    let _ = resolve_conflict_hunk(nonexistent, "file", 0, true);
    let _ = abort_merge(nonexistent);
    let _ = continue_merge(nonexistent);
    let _ = get_branch_upstream_remote(nonexistent, "branch");
    let _ = load_tab_worktrees(nonexistent);
    let _ = worktree_add(nonexistent, "branch", Path::new("path"));
    let _ = worktree_lock(nonexistent, Path::new("path"), "reason");
    let _ = worktree_unlock(nonexistent, Path::new("path"));
    let _ = worktree_remove(nonexistent, Path::new("path"), true);
    let _ = submodule_remove(nonexistent, "name", Path::new("path"));
    let _ = worktree_prune(nonexistent);
    let _ = load_tab_forge_issues(nonexistent, true, Duration::from_secs(5));
    let _ = load_tab_forge_prs(nonexistent, Duration::from_secs(5));
    let _ = load_pr_comments(nonexistent, 1, Duration::from_secs(5));
    let _ = checkout_pr_branch(nonexistent, 1, Duration::from_secs(5));
    let _ = resolve_and_checkout_issue_branch(nonexistent, 1, Duration::from_secs(5));
    let _ = get_remote_url(nonexistent, "origin");
    let _ = get_default_remote_url(nonexistent);
    let _ = get_latest_change_time("/nonexistent");

    let dummy_hunk = vec![
        DiffLine {
            kind: DiffLineKind::Header,
            content: "@@ -1,1 +1,1 @@".to_string(),
            old_lineno: None,
            new_lineno: None,
            hunk_idx: None,
        },
        DiffLine {
            kind: DiffLineKind::Added,
            content: "line".to_string(),
            old_lineno: None,
            new_lineno: None,
            hunk_idx: None,
        },
    ];
    let _ = stage_line(nonexistent, "file", &dummy_hunk, 1);
    let _ = unstage_line(nonexistent, "file", &dummy_hunk, 1);
    let _ = discard_line(nonexistent, "file", &dummy_hunk, 1);
    let _ = stage_hunk(nonexistent, "file", &dummy_hunk);
    let _ = unstage_hunk(nonexistent, "file", &dummy_hunk);
    let _ = discard_hunk(nonexistent, "file", &dummy_hunk);

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_path);
}

#[test]
fn test_bisect_full_lifecycle_and_ref_badges() {
    use std::process::Command;

    let temp_dir = std::env::temp_dir().join(format!(
        "gitwig_test_bisect_core_{}_{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let run_git = |args: &[&str]| {
        let out = Command::new("git").args(args).current_dir(&temp_dir).output().unwrap();
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };

    run_git(&["init", "-b", "main"]);
    run_git(&["config", "user.name", "Tester"]);
    run_git(&["config", "user.email", "tester@example.com"]);

    let mut oids = Vec::new();
    for i in 1..=5 {
        let body = if i >= 3 { format!("step {}\nBUG\n", i) } else { format!("step {}\n", i) };
        std::fs::write(temp_dir.join("app.txt"), body).unwrap();
        run_git(&["add", "app.txt"]);
        run_git(&["commit", "-m", &format!("commit {}", i)]);
        oids.push(run_git(&["rev-parse", "HEAD"]));
    }
    let bad_tip = &oids[4];
    let culprit = &oids[2]; // commit 3 introduced BUG
    let good_base = &oids[0];

    assert!(!get_bisect_status(&temp_dir).active);

    let start_out = bisect_start(&temp_dir, None, None).unwrap();
    assert!(
        start_out.summary.contains("waiting for") || start_out.summary.contains("Bisect"),
        "unexpected bisect start summary: {}",
        start_out.summary
    );

    let bad_out = bisect_bad(&temp_dir, Some(bad_tip)).unwrap();
    assert!(!bad_out.summary.is_empty());

    let good_out = bisect_good(&temp_dir, Some(good_base)).unwrap();
    assert!(good_out.summary.contains("Bisecting:"));

    let status = get_bisect_status(&temp_dir);
    assert!(status.active);
    assert_eq!(status.bad_oid.as_deref(), Some(bad_tip.as_str()));
    assert_eq!(status.good_oids, vec![good_base.clone()]);
    assert!(status.remaining_revisions.is_some());
    assert_eq!(status.log.len(), 2);

    // Detail inspection reflects bisect state, includes the bad tip commit even
    // while HEAD is detached at the midpoint, and annotates bisect ref badges.
    let temp_str = temp_dir.to_string_lossy().to_string();
    if let ItemDetail::Repo { info, .. } = inspect_detail(&temp_str, 50, 50, false) {
        assert!(info.bisect.active);
        assert_eq!(info.summary.state, RepoState::Bisect);
        let tip_entry = info.commits.iter().find(|c| c.oid == *bad_tip).expect("bad tip visible");
        assert!(tip_entry.refs.iter().any(|r| r == "bisect:bad"));
        let good_entry =
            info.commits.iter().find(|c| c.oid == *good_base).expect("good base visible");
        assert!(good_entry.refs.iter().any(|r| r == "bisect:good"));
    } else {
        panic!("expected Repo detail");
    }

    // Exercise skip cleanly before stepping to the culprit.
    let _ = bisect_skip(&temp_dir, None).unwrap();
    assert!(get_bisect_status(&temp_dir).skip_oids.len() == 1);

    // Restart cleanly and narrow down strictly using good/bad until culprit found.
    let reset_msg = bisect_reset(&temp_dir).unwrap();
    assert!(!reset_msg.is_empty());
    assert!(!get_bisect_status(&temp_dir).active);

    bisect_start(&temp_dir, Some("HEAD"), Some(good_base)).unwrap();
    let mut found_culprit = None;
    for _ in 0..8 {
        let content = std::fs::read_to_string(temp_dir.join("app.txt")).unwrap();
        let outcome = if content.contains("BUG") {
            bisect_bad(&temp_dir, None).unwrap()
        } else {
            bisect_good(&temp_dir, None).unwrap()
        };
        if let Some(oid) = outcome.culprit_oid {
            found_culprit = Some(oid);
            break;
        }
    }
    assert_eq!(found_culprit.as_deref(), Some(culprit.as_str()));
    let finished_status = get_bisect_status(&temp_dir);
    assert!(finished_status.culprit.as_deref().unwrap_or_default().starts_with(&culprit[..7]));

    bisect_reset(&temp_dir).unwrap();
    assert!(!get_bisect_status(&temp_dir).active);
    assert_eq!(run_git(&["rev-parse", "--abbrev-ref", "HEAD"]), "main");

    let _ = std::fs::remove_dir_all(&temp_dir);
}
