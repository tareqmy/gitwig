//! Complete integration and unit test suite for the App state management and workflows.

#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;

use crate::config::{RepoConfig, ScanConfig, SortOrder, ThemeConfig};
use crate::test_support::{TestDirGuard, TestFileGuard, temp_config_path};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;

mod detail_tabs;
mod git_and_fetch;
mod home;
mod mouse_and_coverage;
mod settings_and_input;
mod workspace_and_stash;

/// A repository path that does not exist, for a detail view whose actions
/// must not reach git. Never use `PathBuf::from(".")`: that is the process
/// cwd — the developer's own checkout — so a stage all, discard, checkout,
/// fetch or push the test triggers would run there.
fn missing_repo_path(tag: &str) -> PathBuf {
    PathBuf::from(format!("/nonexistent/gitwig_test_{}", tag))
}

/// A fresh repository under the temp dir with `file` committed, removed when
/// the guard drops.
fn temp_repo_with_file(tag: &str, file: &str) -> (PathBuf, TestDirGuard) {
    let dir = std::env::temp_dir().join(format!("gitwig_test_{}", tag));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let guard = TestDirGuard { path: dir.clone() };
    run_git_in(&dir, &["init", "-q"]);
    std::fs::write(dir.join(file), "content\n").unwrap();
    run_git_in(&dir, &["add", file]);
    run_git_in(&dir, &["commit", "-q", "-m", "init"]);
    (dir, guard)
}

/// Tracked repositories `alpha`, `beta` and `gamma`, each with a `.git`
/// directory, in a scratch directory that also holds the app's config and
/// state files (removed when the guard drops).
fn app_with_three_repos(tag: &str) -> (App, Vec<String>, TestDirGuard) {
    let temp_dir = std::env::temp_dir().join(format!("gitwig_test_{}", tag));
    let _ = std::fs::remove_dir_all(&temp_dir);
    let items: Vec<String> = ["alpha", "beta", "gamma"]
        .iter()
        .map(|name| {
            let path = temp_dir.join(name);
            std::fs::create_dir_all(path.join(".git")).unwrap();
            path.to_string_lossy().to_string()
        })
        .collect();
    let config = Config { items: items.clone(), ..Default::default() };
    let app = App::new(config, temp_dir.join("config.toml"));
    (app, items, TestDirGuard { path: temp_dir })
}

/// Labels every repository `work` and records a visit to `gamma`, so the
/// home rows are `[Recent, gamma, work, alpha, beta, gamma]`: a repository's
/// row index differs from its `config.items` position, and `gamma` is listed
/// twice.
fn group_three_repos_under_work(app: &mut App, items: &[String]) {
    for item in items {
        app.config.labels.insert(item.clone(), vec!["work".to_string()]);
    }
    app.state.visits.insert(items[2].clone(), 1);
}

/// Runs `git` in `dir` with a throwaway identity, failing the test on error.
fn run_git_in(dir: &std::path::Path, args: &[&str]) {
    let out = crate::git_cmd::git_command()
        .args(["-c", "user.name=Gitwig Test", "-c", "user.email=test@gitwig.invalid"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {:?}: {}", args, String::from_utf8_lossy(&out.stderr));
}

fn advanced_tab_app(tag: &str, info: crate::repo::RepoInfo, tab: usize) -> (App, TestFileGuard) {
    let config_path = std::env::temp_dir().join(format!("gitwig_test_{}.toml", tag));
    let guard = TestFileGuard { path: config_path.clone() };
    let mut app = App::new(Config::default(), config_path);
    // A path that does not exist, so a confirmed checkout cannot reach git or gh.
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: missing_repo_path(tag),
        info: Box::new(info),
    });
    app.mode = Mode::Detail;
    app.advanced_tabs = true;
    app.detail_tab = tab;
    (app, guard)
}

fn test_worktree(name: &str) -> crate::repo::WorktreeInfo {
    crate::repo::WorktreeInfo {
        name: name.to_string(),
        path: PathBuf::from(format!("/nonexistent/{}", name)),
        branch: Some(name.to_string()),
        is_locked: false,
        lock_reason: None,
    }
}

fn test_reflog_entry(index: usize) -> crate::repo::ReflogEntry {
    crate::repo::ReflogEntry {
        index,
        target_oid: format!("{:040x}", index + 1),
        selector: format!("HEAD@{{{}}}", index),
        command: "commit".to_string(),
        message: format!("entry {}", index),
        when: "now".to_string(),
        date: "2026-01-01".to_string(),
    }
}

/// Builds an app whose single repo `/repo` carries `labels`, with the given
/// per-repo and per-label config entries wired in. `temp` disambiguates the
/// scratch config path so persist-based tests don't collide.
fn label_test_app(
    temp: &str,
    labels: Vec<&str>,
    repo_cfg: Option<RepoConfig>,
    label_cfgs: Vec<(&str, crate::config::LabelConfig)>,
) -> (App, PathBuf) {
    let mut config =
        Config { items: vec!["/repo".to_string()], show_grouping: false, ..Default::default() };
    if !labels.is_empty() {
        config.labels.insert("/repo".to_string(), labels.into_iter().map(String::from).collect());
    }
    if let Some(rc) = repo_cfg {
        config.repo_configs.insert("/repo".to_string(), rc);
    }
    for (label, lc) in label_cfgs {
        config.label_configs.insert(label.to_string(), lc);
    }
    let temp_path = std::env::temp_dir().join(format!("gitwig_test_{}.toml", temp));
    let app = App::new(config, temp_path.clone());
    (app, temp_path)
}

fn palette_test_app(tag: &str) -> (App, TestDirGuard) {
    let temp_dir = std::env::temp_dir().join(format!(
        "gitwig_test_palette_{}_{}_{}",
        tag,
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let guard = TestDirGuard { path: temp_dir.clone() };
    let config = Config {
        items: vec!["/path/to/frontend".to_string(), "/path/to/backend".to_string()],
        ..Default::default()
    };
    (App::new(config, temp_dir.join("config.toml")), guard)
}

fn palette_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)
}

fn type_into_palette(app: &mut App, text: &str) {
    for c in text.chars() {
        assert!(crate::input::handle_key(
            app,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()),
            5
        ));
    }
}

/// Three repos in non-alphabetical custom order, all carrying `work`, plus one
/// unlabelled repo, with the global sort left at Custom.
fn label_sort_app(temp: &str, work: crate::config::LabelConfig) -> (App, PathBuf) {
    let mut config = Config {
        items: vec!["/c".to_string(), "/a".to_string(), "/b".to_string(), "/z".to_string()],
        show_grouping: false,
        ..Default::default()
    };
    for path in ["/c", "/a", "/b"] {
        config.labels.insert(path.to_string(), vec!["work".to_string()]);
    }
    config.label_configs.insert("work".to_string(), work);
    let temp_path = std::env::temp_dir().join(format!("gitwig_test_{}.toml", temp));
    let app = App::new(config, temp_path.clone());
    (app, temp_path)
}

/// Waits for a background thread's report on `app.rx`, skipping messages from
/// other subsystems (the fs watcher shares the channel).
fn wait_for_repo_message(app: &App, expected: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut seen = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        let msg = app.rx.recv_timeout(remaining).unwrap_or_else(|_| {
            panic!("no message containing {:?}; messages seen: {:?}", expected, seen)
        });
        let text = match &msg {
            BgEvent::RepoOpResult { result: Ok(s) | Err(s), .. } => Some(s.as_str()),
            BgEvent::ForgeCheckoutResult { result: Ok(s) | Err(s), .. } => Some(s.as_str()),
            _ => None,
        };
        if text.is_some_and(|s| s.contains(expected)) {
            return;
        }
        seen.push(msg);
    }
}

/// A fresh repository under the temp dir with `files` committed, removed when
/// the guard drops.
fn temp_repo_with_files(tag: &str, files: &[(&str, &str)]) -> (PathBuf, TestDirGuard) {
    let dir = std::env::temp_dir().join(format!("gitwig_test_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let guard = TestDirGuard { path: dir.clone() };
    run_git_in(&dir, &["init", "-q"]);
    for (name, content) in files {
        std::fs::write(dir.join(name), content).unwrap();
        run_git_in(&dir, &["add", name]);
    }
    run_git_in(&dir, &["commit", "-q", "-m", "init"]);
    (dir, guard)
}

/// `git <args>`'s stdout in `dir`.
fn git_output_in(dir: &std::path::Path, args: &[&str]) -> String {
    let out = crate::git_cmd::git_command().args(args).current_dir(dir).output().unwrap();
    assert!(out.status.success(), "git {:?}: {}", args, String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Loads `dir` into the Workspace tab synchronously, as the background loader
/// would, so keys are not held back while a resync is pending.
fn load_workspace(app: &mut App, dir: &std::path::Path) {
    app.current_detail = Some(repo::inspect_detail(&dir.to_string_lossy(), 50, 100, false));
    app.loading_repo_path = None;
    app.mode = Mode::Detail;
    app.detail_tab = 0;
}
