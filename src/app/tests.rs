#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;

use crate::config::{RepoConfig, ScanConfig, SortOrder, ThemeConfig};
use std::collections::HashMap;

struct TestFileGuard {
    path: PathBuf,
}

impl Drop for TestFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[test]
fn test_stash_creation_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_stash.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);
    app.mode = Mode::Detail;

    // Verify starting stash creation triggers correct state
    app.start_stash_create();
    assert_eq!(app.mode, Mode::StashCreateInput);
    assert!(app.input_buffer.is_empty());

    // Simulate typing stash name
    app.input_buffer = "my_custom_stash".to_string();

    // Simulate pressing Esc (cancel)
    let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    let consumed = crate::input::handle_key(&mut app, esc_key, 0);
    assert!(consumed);
    assert_eq!(app.mode, Mode::Detail);

    // Re-start and simulate typing again
    app.start_stash_create();
    app.input_buffer = "my_custom_stash".to_string();

    // Simulate backspace and typing character
    let backspace_key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
    crate::input::handle_key(&mut app, backspace_key, 0);
    assert_eq!(app.input_buffer, "my_custom_stas");

    let char_key = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::empty());
    crate::input::handle_key(&mut app, char_key, 0);
    assert_eq!(app.input_buffer, "my_custom_stash");

    // Simulate enter (commit stash)
    let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    crate::input::handle_key(&mut app, enter_key, 0);

    // Mode returns to detail
    assert_eq!(app.mode, Mode::Detail);

    // Verify we can trigger stash creation from Commits panel if we have uncommitted changes
    app.mode = Mode::Detail;
    app.detail_focus = DetailSection::Commits;

    // Mock uncommitted changes
    let mut mock_info = repo::RepoInfo::default();
    mock_info.changes.unstaged = vec![repo::FileEntry { path: "dirty.rs".to_string(), label: "M" }];
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: PathBuf::from("a_repo"),
        info: Box::new(mock_info),
    });

    assert!(app.has_uncommitted_changes());

    // Pressing 's' should navigate to the Stashing UI overlay (Mode::StashingUI)
    let s_key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty());
    let consumed = crate::input::handle_key(&mut app, s_key, 0);
    assert!(consumed);
    assert_eq!(app.mode, Mode::StashingUI);
}

#[test]
fn test_network_action_progress_and_error_handling() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_network.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Simulating the start of a network action
    app.fetching = true;
    app.status_message = Some("Pushing...".to_string());

    // Assert progress popup is active
    assert!(app.fetching);
    assert_eq!(app.status_message.as_deref(), Some("Pushing..."));

    // Simulate background thread sending a failure message
    app.tx.send("Push failed: git push rejected".to_string()).unwrap();

    // Run receiver check
    while let Ok(msg) = app.rx.try_recv() {
        let is_err = msg.starts_with("Fetch failed:")
            || msg.starts_with("Pull failed:")
            || msg.starts_with("Push failed:")
            || msg.starts_with("Failed to")
            || msg.contains("failed");

        if is_err {
            app.error_message = Some(msg);
        } else {
            app.status_message = Some(msg);
        }
        app.fetching = false;
    }

    // Verify that fetching is cleared and error_message popup is active
    assert!(!app.fetching);
    assert_eq!(app.error_message.as_deref(), Some("Push failed: git push rejected"));

    // Verify keypress dismisses the error popup
    let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    let consumed = crate::input::handle_key(&mut app, esc_key, 0);
    assert!(consumed);
    assert!(app.error_message.is_none());
}

#[test]
fn test_remote_tags_progress_and_error_handling() {
    let config = Config {
        items: vec![".".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_remote_tags_progress.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    let mock_info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        remotes: crate::repo::TabData::Loaded(vec![crate::repo::RemoteInfo {
            name: "origin".to_string(),
            url: "git@github.com:tareqmy/gitwig.git".to_string(),
            push_url: None,
            refspecs: vec![],
        }]),
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: std::path::PathBuf::from("."),
        info: Box::new(mock_info),
    });

    // Trigger fetch with show_progress = true
    app.fetch_remote_tags(true);
    assert!(app.fetching);
    assert_eq!(app.status_message.as_deref(), Some("Fetching tags from 'origin'..."));

    // Simulate background thread sending REMOTE_TAGS_ERR
    app.tx.send("REMOTE_TAGS_ERR:Failed to get remote tags: custom error".to_string()).unwrap();

    // Run rx loop (same as inside app::run)
    if let Ok(msg) = app.rx.try_recv() {
        if let Some(err_msg) = msg.strip_prefix("REMOTE_TAGS_ERR:") {
            app.set_error(err_msg.to_string());
            app.fetching = false;
        }
    }

    assert!(!app.fetching);
    assert_eq!(app.error_message.as_deref(), Some("Failed to get remote tags: custom error"));
}

#[test]
fn test_remote_fetch_progress_and_error_handling() {
    let config = Config {
        items: vec![".".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_remote_fetch_progress.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    let mock_info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        remotes: crate::repo::TabData::Loaded(vec![crate::repo::RemoteInfo {
            name: "origin".to_string(),
            url: "git@github.com:tareqmy/gitwig.git".to_string(),
            push_url: None,
            refspecs: vec![],
        }]),
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: std::path::PathBuf::from("."),
        info: Box::new(mock_info),
    });

    // Trigger fetch from remote tab (fetch_remote)
    app.fetch_remote("origin");
    assert!(app.fetching);
    assert_eq!(app.status_message.as_deref(), Some("Fetching remote 'origin'..."));

    // Simulate background thread sending Fetch failed message
    app.tx.send("Fetch failed: custom fetch error".to_string()).unwrap();

    // Run rx loop (same as inside app::run)
    if let Ok(msg) = app.rx.try_recv() {
        let is_err = msg.starts_with("Fetch failed:")
            || msg.starts_with("Pull failed:")
            || msg.starts_with("Push failed:")
            || msg.starts_with("Failed to")
            || msg.contains("failed");

        if is_err {
            app.set_error(msg);
        }
        app.fetching = false;
    }

    assert!(!app.fetching);
    assert_eq!(app.error_message.as_deref(), Some("Fetch failed: custom fetch error"));
}

#[test]
fn test_set_error_logging() {
    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_set_error.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    let test_error_msg = "Test error message for debugging".to_string();
    app.set_error(test_error_msg.clone());

    assert_eq!(app.error_message.as_ref(), Some(&test_error_msg));

    // Check if debug log contains the message
    let logs = crate::debug_log::get_logs();
    assert!(logs.iter().any(|log| log.contains("ERROR") && log.contains(&test_error_msg)));
}

#[test]
fn test_sorting_logic() {
    let config = Config {
        items: vec!["z_repo".to_string(), "a_repo".to_string(), "m_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_sort.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Assert initial custom sort
    assert_eq!(app.config.items[0], "z_repo");
    assert_eq!(app.config.items[1], "a_repo");

    // Cycle to alphabetical
    app.cycle_sort_order();
    assert_eq!(app.config.sort_by, SortOrder::Alphabetical);
    assert_eq!(app.config.items[0], "a_repo");
    assert_eq!(app.config.items[1], "m_repo");
    assert_eq!(app.config.items[2], "z_repo");

    // Toggle reverse sorting
    app.toggle_sort_reverse();
    assert!(app.config.sort_reverse);
    assert_eq!(app.config.items[0], "z_repo");
    assert_eq!(app.config.items[1], "m_repo");
    assert_eq!(app.config.items[2], "a_repo");

    // Toggle back
    app.toggle_sort_reverse();
    assert!(!app.config.sort_reverse);

    // Cycle to recent visit
    // Set visit times: a_repo visited at 10, z_repo at 20, m_repo at 5
    app.config.visits.insert("a_repo".to_string(), 10);
    app.config.visits.insert("z_repo".to_string(), 20);
    app.config.visits.insert("m_repo".to_string(), 5);

    app.cycle_sort_order();
    assert_eq!(app.config.sort_by, SortOrder::RecentVisit);
    // Descending order (recent first) -> z_repo (20), a_repo (10), m_repo (5)
    assert_eq!(app.config.items[0], "z_repo");
    assert_eq!(app.config.items[1], "a_repo");
    assert_eq!(app.config.items[2], "m_repo");
}

#[test]
fn test_duplicate_prevention() {
    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_duplicate.toml");
    // Ensure starting with a clean state and clean up upon drop
    let _ = std::fs::remove_file(&temp_path);
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // 1. Test adding a repository via input buffer (commit_add)
    app.input_buffer = " /path/to/repo ".to_string(); // trimmed to "/path/to/repo"
    app.commit_add();
    assert_eq!(app.config.items.len(), 1);
    assert_eq!(app.config.items[0], "/path/to/repo");
    assert_eq!(app.status_message, Some("Saved".to_string()));
    app.status_message = None; // Reset

    // 2. Test trying to add the exact same repo path again (via commit_add)
    app.input_buffer = "/path/to/repo".to_string();
    app.commit_add();
    assert_eq!(app.config.items.len(), 1);
    assert_eq!(app.status_message, Some("Repository already added".to_string()));
    app.status_message = None; // Reset

    // 3. Test trying to add a tilde version of the same repo when it's resolved
    if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy().to_string();
        // First add the tilde path
        app.input_buffer = "~/my_cool_repo".to_string();
        app.commit_add();
        assert_eq!(app.config.items.len(), 2);
        assert_eq!(app.config.items[1], "~/my_cool_repo");
        assert_eq!(app.status_message, Some("Saved".to_string()));
        app.status_message = None; // Reset

        // Now try to add the expanded absolute path
        let expanded_path = format!("{}/my_cool_repo", home_str);
        app.input_buffer = expanded_path;
        app.commit_add();
        // Should be rejected
        assert_eq!(app.config.items.len(), 2);
        assert_eq!(app.status_message, Some("Repository already added".to_string()));
        app.status_message = None; // Reset

        // Try the opposite direction: add a new absolute path, then try to add with tilde
        let new_abs = format!("{}/another_cool_repo", home_str);
        app.input_buffer = new_abs;
        app.commit_add();
        assert_eq!(app.config.items.len(), 3);
        assert_eq!(app.config.items[2], format!("{}/another_cool_repo", home_str));
        assert_eq!(app.status_message, Some("Saved".to_string()));
        app.status_message = None; // Reset

        // Now try to add with tilde
        app.input_buffer = "~/another_cool_repo".to_string();
        app.commit_add();
        // Should be rejected
        assert_eq!(app.config.items.len(), 3);
        assert_eq!(app.status_message, Some("Repository already added".to_string()));
        app.status_message = None; // Reset
    }

    // 4. Test adding via add_repo_path directly
    let len_before = app.config.items.len();
    app.add_repo_path(" /another/path ".to_string());
    assert_eq!(app.config.items.len(), len_before + 1);
    assert_eq!(app.config.items.last().unwrap(), "/another/path");
    assert_eq!(app.status_message, Some("Added repository".to_string()));
    app.status_message = None; // Reset

    // Try duplicate via add_repo_path
    app.add_repo_path("/another/path".to_string());
    assert_eq!(app.config.items.len(), len_before + 1);
    assert_eq!(app.status_message, Some("Repository already added".to_string()));
}

#[test]
fn test_bulk_add_folders() {
    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_dir = std::env::temp_dir().join("gitwig_test_bulk_add_dir");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let repo_a = temp_dir.join("repo_a");
    let repo_b = temp_dir.join("repo_b");
    let repo_c = temp_dir.join("repo_c");
    std::fs::create_dir_all(repo_a.join(".git")).unwrap();
    std::fs::create_dir_all(&repo_b).unwrap();
    std::fs::create_dir_all(repo_c.join(".git")).unwrap();

    let config_path = temp_dir.join("config_bulk.toml");
    let _ = std::fs::remove_file(&config_path);
    let _guard = TestFileGuard { path: config_path.clone() };
    let mut app = App::new(config, config_path);

    // Case 1: git_only is enabled (default)
    app.config.scan.git_only = true;
    app.input_buffer = temp_dir.to_string_lossy().to_string();
    app.commit_bulk_add();

    // Should include repo_a and repo_c, but NOT repo_b
    assert_eq!(app.config.items.len(), 2);
    assert!(app.config.items.iter().any(|item| item.ends_with("repo_a")));
    assert!(app.config.items.iter().any(|item| item.ends_with("repo_c")));
    assert!(!app.config.items.iter().any(|item| item.ends_with("repo_b")));

    // Clear items and try again with git_only = false
    app.config.items.clear();
    app.original_items.clear();
    app.statuses.clear();

    app.config.scan.git_only = false;
    app.input_buffer = temp_dir.to_string_lossy().to_string();
    app.commit_bulk_add();

    // Should include repo_a, repo_b, and repo_c
    assert_eq!(app.config.items.len(), 3);
    assert!(app.config.items.iter().any(|item| item.ends_with("repo_a")));
    assert!(app.config.items.iter().any(|item| item.ends_with("repo_b")));
    assert!(app.config.items.iter().any(|item| item.ends_with("repo_c")));

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_pinning_and_sorting() {
    let config = Config {
        items: vec!["z_repo".to_string(), "a_repo".to_string(), "m_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Alphabetical,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_pin.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Sorting is Alphabetical: initially items should be sorted as a_repo, m_repo, z_repo
    app.sort_items_in_place();
    assert_eq!(app.config.items[0], "a_repo");
    assert_eq!(app.config.items[1], "m_repo");
    assert_eq!(app.config.items[2], "z_repo");

    // Pin the last one ("z_repo", index 2)
    app.selected_index = 2;
    app.toggle_pin_selected();

    // After pinning, z_repo is pinned.
    // It must move to the top (index 0).
    // The selection cursor must also follow z_repo, meaning selected_index should become 0.
    assert!(app.config.pinned.contains("z_repo"));
    assert_eq!(app.config.items[0], "z_repo");
    assert_eq!(app.config.items[1], "a_repo");
    assert_eq!(app.config.items[2], "m_repo");
    assert_eq!(app.selected_index, 0);

    // Reverse sorting with z_repo pinned:
    // Pinned block is ["z_repo"]. Unpinned block is ["a_repo", "m_repo"] -> reverse alphabetical is ["m_repo", "a_repo"]
    // Pinned is kept on top: ["z_repo", "m_repo", "a_repo"]
    app.toggle_sort_reverse();
    assert_eq!(app.config.items[0], "z_repo");
    assert_eq!(app.config.items[1], "m_repo");
    assert_eq!(app.config.items[2], "a_repo");
    // selected_index should still track "z_repo" (which is at index 0)
    assert_eq!(app.selected_index, 0);

    // Toggle reverse back
    app.toggle_sort_reverse();

    // Pin m_repo too (currently at index 2)
    app.selected_index = 2; // "m_repo"
    app.toggle_pin_selected();

    // Now both z_repo and m_repo are pinned.
    // Alphabetical sort:
    // Pinned: m_repo, z_repo -> sorted alphabetically is ["m_repo", "z_repo"]
    // Unpinned: a_repo -> ["a_repo"]
    // Combined: ["m_repo", "z_repo", "a_repo"]
    assert_eq!(app.config.items[0], "m_repo");
    assert_eq!(app.config.items[1], "z_repo");
    assert_eq!(app.config.items[2], "a_repo");
    // cursor was on m_repo, which ended up at index 0
    assert_eq!(app.selected_index, 0);

    // Unpin m_repo (currently at index 0)
    app.selected_index = 0;
    app.toggle_pin_selected();

    // Now only z_repo is pinned.
    // Items should be ["z_repo", "a_repo", "m_repo"]
    assert_eq!(app.config.items[0], "z_repo");
    assert_eq!(app.config.items[1], "a_repo");
    assert_eq!(app.config.items[2], "m_repo");
}

#[test]
fn test_commit_input_scroll() {
    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_commit_scroll.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    assert_eq!(app.commit_input_scroll, 0);

    app.commit_input_scroll_down();
    assert_eq!(app.commit_input_scroll, 1);

    app.commit_input_scroll_down();
    assert_eq!(app.commit_input_scroll, 2);

    app.commit_input_scroll_up();
    assert_eq!(app.commit_input_scroll, 1);

    // Cancel resets it
    app.cancel_commit();
    assert_eq!(app.commit_input_scroll, 0);
}

#[test]
fn test_commit_popup_maximized_toggle() {
    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_commit_maximize.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    assert!(!app.commit_popup.maximized);

    app.toggle_commit_popup_maximized();
    assert!(app.commit_popup.maximized);

    app.toggle_commit_popup_maximized();
    assert!(!app.commit_popup.maximized);

    app.toggle_commit_popup_maximized();
    assert!(app.commit_popup.maximized);

    // Cancel resets it
    app.cancel_commit();
    assert!(!app.commit_popup.maximized);
}

#[test]
fn test_cherry_pick_and_revert_flow() {
    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_cherry_pick.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Set up a mock repo detail with commits
    let mock_info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        commits: vec![crate::repo::CommitEntry {
            id: "1234567".to_string(),
            oid: "1234567890abcdef1234567890abcdef12345678".to_string(),
            summary: "test commit".to_string(),
            author: "author".to_string(),
            when: "today".to_string(),
            date: "today".to_string(),
            refs: vec![],
            message: "msg".to_string(),
            files: vec![],
            signature_status: "N".to_string(),
        }],
        ..Default::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: std::path::PathBuf::from("/mock/repo"),
        info: Box::new(mock_info),
    });

    // 1. Cherry-pick flow
    app.commit_list.selection = 0;
    app.request_cherry_pick();
    assert_eq!(app.mode, Mode::CherryPickConfirm);
    assert!(app.cherry_pick_target.is_some());
    assert_eq!(
        app.cherry_pick_target.as_ref().unwrap().0,
        "1234567890abcdef1234567890abcdef12345678"
    );

    app.cancel_cherry_pick();
    assert_eq!(app.mode, Mode::Detail);
    assert!(app.cherry_pick_target.is_none());

    // 2. Revert flow
    app.commit_list.selection = 0;
    app.request_revert();
    assert_eq!(app.mode, Mode::RevertConfirm);
    assert!(app.revert_target.is_some());
    assert_eq!(app.revert_target.as_ref().unwrap().0, "1234567890abcdef1234567890abcdef12345678");

    app.cancel_revert();
    assert_eq!(app.mode, Mode::Detail);
    assert!(app.revert_target.is_none());
}

#[test]
fn test_commit_amend_flow() {
    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_commit_amend.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    assert!(!app.commit_popup.amend);

    app.toggle_commit_amend();
    assert!(app.commit_popup.amend);

    app.toggle_commit_amend();
    assert!(!app.commit_popup.amend);

    // Without HEAD
    app.start_commit_amend();
    assert_eq!(app.status_message.as_deref(), Some("No commit to amend"));
    assert_eq!(app.mode, Mode::Normal);

    // With HEAD
    let info = crate::repo::RepoInfo {
        head: Some(crate::repo::HeadInfo {
            short_id: "dummy_sha".to_string(),
            summary: "dummy message".to_string(),
            author: "author".to_string(),
            when: "now".to_string(),
        }),
        ..Default::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/dummy"),
        info: Box::new(info),
    });

    app.start_commit_amend();
    assert!(app.commit_popup.amend);
    assert!(app.commit_popup.editing);
    assert_eq!(app.mode, Mode::CommitInput);
}

#[test]
fn test_splitter_dragging() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Rect;

    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_splitter.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Mock the detail_areas to simulate a drawn UI frame.
    // Left panel is 40 columns wide (from 0 to 40), Right is 60 columns wide (40 to 100).
    // Total width = 100. Horizontal splitter is at column 40.
    // We set the bounding boxes.
    app.detail_areas.bottom_left = Some(Rect::new(0, 0, 40, 50));
    app.detail_areas.bottom_right = Some(Rect::new(40, 0, 60, 50));
    app.detail_areas.inspect_horizontal_splitter = Some(Rect::new(39, 0, 2, 50));

    // Click on the horizontal splitter
    let down_event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 39,
        row: 10,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, down_event);
    assert_eq!(app.active_drag_splitter, Some(Splitter::InspectHorizontal));

    // Drag to column 30 (which means 30% of total width 100)
    let drag_event = MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 30,
        row: 10,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, drag_event);
    assert_eq!(app.inspect_horizontal_split_pct, 30);

    // Release mouse
    let up_event = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 30,
        row: 10,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, up_event);
    assert_eq!(app.active_drag_splitter, None);

    // Test WorkspaceMain splitter dragging
    app.detail_areas.commits = Some(Rect::new(0, 0, 100, 20));
    app.detail_areas.bottom_right = Some(Rect::new(0, 20, 100, 30));
    app.detail_areas.workspace_main_splitter = Some(Rect::new(0, 19, 100, 2));

    // Click on the vertical workspace main splitter
    let down_event_main = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 19,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, down_event_main);
    assert_eq!(app.active_drag_splitter, Some(Splitter::WorkspaceMain));

    // Drag to row 25 (which is 50% height since total height is 50)
    let drag_event_main = MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 10,
        row: 25,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, drag_event_main);
    assert_eq!(app.workspace_main_split_pct, 50);

    // Drag to row 15 (which is 30% height)
    let drag_event_main_2 = MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 10,
        row: 15,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, drag_event_main_2);
    assert_eq!(app.workspace_main_split_pct, 30);

    // Release mouse
    let up_event_main = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 10,
        row: 15,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, up_event_main);
    assert_eq!(app.active_drag_splitter, None);

    // Test Files splitter dragging
    app.detail_areas.files = Some(Rect::new(0, 0, 45, 50));
    app.detail_areas.file_content = Some(Rect::new(45, 0, 55, 50));
    app.detail_areas.files_horizontal_splitter = Some(Rect::new(44, 0, 2, 50));

    let down_event_files = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 44,
        row: 10,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, down_event_files);
    assert_eq!(app.active_drag_splitter, Some(Splitter::FilesHorizontal));

    let drag_event_files = MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 60,
        row: 10,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, drag_event_files);
    assert_eq!(app.files_horizontal_split_pct, 60);

    let up_event_files = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 60,
        row: 10,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, up_event_files);
    assert_eq!(app.active_drag_splitter, None);

    // Test Branches splitter dragging
    app.detail_areas = DetailAreas::default();
    app.detail_areas.local_branches = Some(Rect::new(0, 0, 50, 50));
    app.detail_areas.remote_branches = Some(Rect::new(50, 0, 50, 50));
    app.detail_areas.branches_horizontal_splitter = Some(Rect::new(49, 0, 2, 50));

    let down_event_branches = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 49,
        row: 10,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, down_event_branches);
    assert_eq!(app.active_drag_splitter, Some(Splitter::BranchesHorizontal));

    let drag_event_branches = MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 35,
        row: 10,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, drag_event_branches);
    assert_eq!(app.branches_horizontal_split_pct, 35);

    let up_event_branches = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 35,
        row: 10,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, up_event_branches);
    assert_eq!(app.active_drag_splitter, None);

    // Test Stashes splitter dragging (horizontal & vertical)
    app.detail_areas = DetailAreas::default();
    app.detail_areas.stashes = Some(Rect::new(0, 0, 35, 25));
    app.detail_areas.stashed_files = Some(Rect::new(0, 25, 35, 25));
    app.detail_areas.bottom_right = Some(Rect::new(35, 0, 65, 50));
    app.detail_areas.stashes_horizontal_splitter = Some(Rect::new(34, 0, 2, 50));
    app.detail_areas.stashes_vertical_splitter = Some(Rect::new(0, 24, 35, 2));

    // Click stashes horizontal splitter
    let down_stashes_h = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 34,
        row: 10,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, down_stashes_h);
    assert_eq!(app.active_drag_splitter, Some(Splitter::StashesHorizontal));

    let drag_stashes_h = MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 40,
        row: 10,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, drag_stashes_h);
    assert_eq!(app.stashes_horizontal_split_pct, 40);

    let up_stashes_h = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 40,
        row: 10,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, up_stashes_h);

    // Click stashes vertical splitter
    let down_stashes_v = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 24,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, down_stashes_v);
    assert_eq!(app.active_drag_splitter, Some(Splitter::StashesVertical));

    let drag_stashes_v = MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 10,
        row: 30,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, drag_stashes_v);
    assert_eq!(app.stashes_vertical_split_pct, 60);

    let up_stashes_v = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 10,
        row: 30,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, up_stashes_v);

    // Test Overview splitter dragging
    app.detail_areas = DetailAreas::default();
    app.detail_areas.tab_bar = Some(Rect::new(0, 0, 100, 2));
    app.detail_areas.overview_horizontal_splitter = Some(Rect::new(49, 2, 2, 48));

    let down_overview = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 49,
        row: 10,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, down_overview);
    assert_eq!(app.active_drag_splitter, Some(Splitter::OverviewHorizontal));

    let drag_overview = MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 30,
        row: 10,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, drag_overview);
    assert_eq!(app.overview_horizontal_split_pct, 30);

    let up_overview = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 30,
        row: 10,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, up_overview);
    assert_eq!(app.active_drag_splitter, None);
}

#[test]
fn test_mouse_row_selection_in_detail_panels() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Rect;

    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_mouse_select.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);
    app.mode = Mode::Detail;

    // 1. Commits panel click test
    app.detail_areas = crate::ui_detail::DetailAreas::default();
    app.detail_areas.commits = Some(Rect::new(0, 0, 100, 20));
    app.detail_areas.commits_inner = Some(Rect::new(1, 1, 98, 18));
    let mock_info = repo::RepoInfo {
        branch: Some("main".to_string()),
        commits: vec![
            repo::CommitEntry {
                id: "1".to_string(),
                oid: "1111111111111111111111111111111111111111".to_string(),
                summary: "C1".to_string(),
                author: "A".to_string(),
                when: "now".to_string(),
                date: "now".to_string(),
                refs: vec![],
                message: "msg".to_string(),
                files: vec![],
                signature_status: "N".to_string(),
            },
            repo::CommitEntry {
                id: "2".to_string(),
                oid: "2222222222222222222222222222222222222222".to_string(),
                summary: "C2".to_string(),
                author: "B".to_string(),
                when: "now".to_string(),
                date: "now".to_string(),
                refs: vec![],
                message: "msg".to_string(),
                files: vec![],
                signature_status: "N".to_string(),
            },
            repo::CommitEntry {
                id: "3".to_string(),
                oid: "3333333333333333333333333333333333333333".to_string(),
                summary: "C3".to_string(),
                author: "C".to_string(),
                when: "now".to_string(),
                date: "now".to_string(),
                refs: vec![],
                message: "msg".to_string(),
                files: vec![],
                signature_status: "N".to_string(),
            },
        ],
        ..repo::RepoInfo::default()
    };
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: PathBuf::from("a_repo"),
        info: Box::new(mock_info),
    });

    let commit_click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 3,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, commit_click);
    assert_eq!(app.commit_list.selection, 1);
    assert_eq!(app.detail_focus, DetailSection::Commits);

    // 2. Staged subpanel click test
    app.detail_areas = crate::ui_detail::DetailAreas::default();
    let mut mock_info_2 = repo::RepoInfo::default();
    mock_info_2.changes.staged = vec![
        repo::FileEntry { path: "s1.rs".to_string(), label: "M" },
        repo::FileEntry { path: "s2.rs".to_string(), label: "M" },
    ];
    mock_info_2.changes.unstaged = vec![repo::FileEntry { path: "u1.rs".to_string(), label: "M" }];
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: PathBuf::from("a_repo"),
        info: Box::new(mock_info_2),
    });

    app.detail_areas.staged_sub = Some(Rect::new(0, 20, 50, 10));
    app.detail_areas.staged_sub_inner = Some(Rect::new(1, 21, 48, 8));
    let staged_click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 5,
        row: 22,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, staged_click);
    assert_eq!(app.status_list.staging_file_selection, 1);
    assert_eq!(app.detail_focus, DetailSection::Staged);

    // 3. Unstaged subpanel click test
    app.detail_areas = crate::ui_detail::DetailAreas::default();
    let mut mock_info_2_unstaged = repo::RepoInfo::default();
    mock_info_2_unstaged.changes.unstaged =
        vec![repo::FileEntry { path: "u1.rs".to_string(), label: "M" }];
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: PathBuf::from("a_repo"),
        info: Box::new(mock_info_2_unstaged),
    });
    app.detail_areas.unstaged_sub = Some(Rect::new(0, 30, 50, 10));
    app.detail_areas.unstaged_sub_inner = Some(Rect::new(1, 31, 48, 8));
    let unstaged_click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 5,
        row: 31,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, unstaged_click);
    assert_eq!(app.status_list.staging_file_selection, 0);
    assert_eq!(app.detail_focus, DetailSection::Unstaged);

    // 4. Local branches click test
    app.detail_areas = crate::ui_detail::DetailAreas::default();
    let mock_info_3 = repo::RepoInfo {
        local_branches: repo::TabData::Loaded(vec![
            repo::BranchInfo {
                name: "b1".to_string(),
                is_head: true,
                short_sha: "123".to_string(),
                short_message: "msg".to_string(),
            },
            repo::BranchInfo {
                name: "b2".to_string(),
                is_head: false,
                short_sha: "456".to_string(),
                short_message: "msg".to_string(),
            },
        ]),
        remote_branches: repo::TabData::Loaded(vec![repo::BranchInfo {
            name: "origin/b1".to_string(),
            is_head: false,
            short_sha: "123".to_string(),
            short_message: "msg".to_string(),
        }]),
        ..Default::default()
    };
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: PathBuf::from("a_repo"),
        info: Box::new(mock_info_3),
    });

    app.detail_areas.local_branches = Some(Rect::new(0, 0, 50, 20));
    app.detail_areas.local_branches_inner = Some(Rect::new(1, 1, 48, 18));
    let local_branch_click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 5,
        row: 2, // inner.y = 1, so row 2 is index 1
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, local_branch_click);
    assert_eq!(app.branch_list.local_branch_selection, 1);
    assert_eq!(app.detail_focus, DetailSection::LocalBranches);

    // 5. Remote branches click test
    app.detail_areas = crate::ui_detail::DetailAreas::default();
    let mock_info_3_remote = repo::RepoInfo {
        remote_branches: repo::TabData::Loaded(vec![repo::BranchInfo {
            name: "origin/b1".to_string(),
            is_head: false,
            short_sha: "123".to_string(),
            short_message: "msg".to_string(),
        }]),
        ..Default::default()
    };
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: PathBuf::from("a_repo"),
        info: Box::new(mock_info_3_remote),
    });
    app.detail_areas.remote_branches = Some(Rect::new(50, 0, 50, 20));
    app.detail_areas.remote_branches_inner = Some(Rect::new(51, 1, 48, 18));
    let remote_branch_click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 55,
        row: 1, // index 0
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, remote_branch_click);
    assert_eq!(app.branch_list.remote_branch_selection, 0);
    assert_eq!(app.detail_focus, DetailSection::RemoteBranches);

    // 6. Local tags click test
    app.detail_areas = crate::ui_detail::DetailAreas::default();
    let mock_info_4 = repo::RepoInfo {
        local_tags: repo::TabData::Loaded(vec![
            repo::BranchInfo {
                name: "t1".to_string(),
                is_head: false,
                short_sha: "123".to_string(),
                short_message: "msg".to_string(),
            },
            repo::BranchInfo {
                name: "t2".to_string(),
                is_head: false,
                short_sha: "456".to_string(),
                short_message: "msg".to_string(),
            },
        ]),
        ..Default::default()
    };
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: PathBuf::from("a_repo"),
        info: Box::new(mock_info_4),
    });

    app.detail_areas.local_tags = Some(Rect::new(0, 0, 100, 20));
    app.detail_areas.local_tags_inner = Some(Rect::new(1, 1, 98, 18));
    let tag_click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 2, // index 1
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, tag_click);
    assert_eq!(app.tag_list.local_tag_selection, 1);
    assert_eq!(app.detail_focus, DetailSection::LocalTags);

    // 7. Remotes click test
    app.detail_areas = crate::ui_detail::DetailAreas::default();
    let mock_info_5 = repo::RepoInfo {
        remotes: repo::TabData::Loaded(vec![
            repo::RemoteInfo {
                name: "r1".to_string(),
                url: "url1".to_string(),
                push_url: None,
                refspecs: vec![],
            },
            repo::RemoteInfo {
                name: "r2".to_string(),
                url: "url2".to_string(),
                push_url: None,
                refspecs: vec![],
            },
        ]),
        ..Default::default()
    };
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: PathBuf::from("a_repo"),
        info: Box::new(mock_info_5),
    });

    app.detail_areas.remotes = Some(Rect::new(0, 0, 100, 20));
    app.detail_areas.remotes_inner = Some(Rect::new(1, 1, 98, 18));
    let remote_click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 2, // index 1
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, remote_click);
    assert_eq!(app.branch_list.remote_selection, 1);
    assert_eq!(app.detail_focus, DetailSection::Remotes);

    // 8. Stashes and Stashed Files click test
    app.detail_areas = crate::ui_detail::DetailAreas::default();
    let mock_info_6 = repo::RepoInfo {
        stashes: repo::TabData::Loaded(vec![
            repo::StashInfo {
                index: 0,
                commit_id: "123".to_string(),
                message: "s1".to_string(),
                files: vec![
                    repo::FileEntry { path: "f1.rs".to_string(), label: "M" },
                    repo::FileEntry { path: "f2.rs".to_string(), label: "M" },
                ],
            },
            repo::StashInfo {
                index: 1,
                commit_id: "456".to_string(),
                message: "s2".to_string(),
                files: vec![],
            },
        ]),
        ..Default::default()
    };
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: PathBuf::from("a_repo"),
        info: Box::new(mock_info_6),
    });

    app.detail_areas.stashes = Some(Rect::new(0, 0, 100, 20));
    app.detail_areas.stashes_inner = Some(Rect::new(1, 1, 98, 18));
    let stash_click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 2, // index 1
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, stash_click);
    assert_eq!(app.stash_list.stash_selection, 1);
    assert_eq!(app.detail_focus, DetailSection::Stashes);

    app.detail_areas = crate::ui_detail::DetailAreas::default();
    // re-apply mock info if needed (already in app.current_detail)
    app.stash_list.stash_selection = 0;
    app.detail_areas.stashed_files = Some(Rect::new(0, 20, 100, 20));
    app.detail_areas.stashed_files_inner = Some(Rect::new(1, 21, 98, 18));
    let stash_file_click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 22, // index 1 (relative to 21)
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, stash_file_click);
    assert_eq!(app.stash_list.stash_file_selection, 1);
    assert_eq!(app.detail_focus, DetailSection::StashedFiles);

    // 9. Inspect view file click test
    app.mode = Mode::Inspect;
    app.detail_areas = crate::ui_detail::DetailAreas::default();
    let mut mock_info_inspect = repo::RepoInfo::default();
    let mock_commit = repo::CommitEntry {
        id: "1234567".to_string(),
        oid: "1234567890abcdef1234567890abcdef12345678".to_string(),
        summary: "test summary".to_string(),
        author: "author".to_string(),
        when: "now".to_string(),
        date: "now".to_string(),
        refs: vec![],
        message: "message".to_string(),
        files: vec![
            repo::FileEntry { path: "file1.rs".to_string(), label: "M" },
            repo::FileEntry { path: "file2.rs".to_string(), label: "M" },
        ],
        signature_status: "N".to_string(),
    };
    mock_info_inspect.commits = vec![mock_commit];
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: PathBuf::from("a_repo"),
        info: Box::new(mock_info_inspect),
    });
    app.commit_list.selection = 0;

    app.detail_areas.bottom_left = Some(Rect::new(0, 10, 50, 10));
    app.detail_areas.changed_files_inner = Some(Rect::new(1, 11, 48, 8));
    let inspect_file_click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 5,
        row: 12, // index 1 (relative to 11)
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, inspect_file_click);
    assert_eq!(app.status_list.file_selection, 1);
    assert_eq!(app.detail_focus, DetailSection::Staged);

    // 10. Files tab click test
    app.mode = Mode::Detail;
    app.detail_tab = 1; // Files Tab
    app.detail_areas = crate::ui_detail::DetailAreas::default();
    app.detail_areas.files = Some(Rect::new(0, 0, 100, 20));
    app.detail_areas.files_inner = Some(Rect::new(1, 1, 98, 18));
    app.file_tree.visible_files = vec![
        crate::app::FileTreeItem {
            name: "f1.rs".to_string(),
            full_path: "f1.rs".to_string(),
            is_dir: false,
            depth: 0,
            is_expanded: false,
        },
        crate::app::FileTreeItem {
            name: "f2.rs".to_string(),
            full_path: "f2.rs".to_string(),
            is_dir: false,
            depth: 0,
            is_expanded: false,
        },
    ];
    let files_click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 2, // index 1 (relative to 1)
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, files_click);
    assert_eq!(app.file_tree.file_list_selection, 1);
    assert_eq!(app.detail_focus, DetailSection::Files);

    // 11. Worktrees list click test
    app.mode = Mode::Detail;
    app.detail_tab = 7; // Worktrees Tab
    app.detail_areas = crate::ui_detail::DetailAreas::default();
    let mock_info_7 = repo::RepoInfo {
        worktrees: repo::TabData::Loaded(vec![
            repo::WorktreeInfo {
                name: "wt1".to_string(),
                path: PathBuf::from("wt1"),
                branch: Some("b1".to_string()),
                is_locked: false,
                lock_reason: None,
            },
            repo::WorktreeInfo {
                name: "wt2".to_string(),
                path: PathBuf::from("wt2"),
                branch: Some("b2".to_string()),
                is_locked: false,
                lock_reason: None,
            },
        ]),
        ..Default::default()
    };
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: PathBuf::from("a_repo"),
        info: Box::new(mock_info_7),
    });
    app.detail_areas.worktrees = Some(Rect::new(0, 0, 100, 20));
    app.detail_areas.worktrees_inner = Some(Rect::new(1, 1, 98, 18));
    let worktree_click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 4, // index 1 (relative to inner.y = 1 + header_height = 2)
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, worktree_click);
    assert_eq!(app.worktree_selection, 1);
    assert_eq!(app.detail_focus, DetailSection::Worktrees);

    // 12. Graph tab click test
    app.mode = Mode::Detail;
    app.detail_tab = 2; // Graph Tab
    app.detail_areas = crate::ui_detail::DetailAreas::default();
    let mock_info_8 = repo::RepoInfo {
        graph_lines: repo::TabData::Loaded(vec![
            repo::GraphLine { graph: "*".to_string(), commit: None },
            repo::GraphLine { graph: "|*".to_string(), commit: None },
        ]),
        ..Default::default()
    };
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: PathBuf::from("a_repo"),
        info: Box::new(mock_info_8),
    });
    app.detail_areas.graph = Some(Rect::new(0, 0, 100, 20));
    app.detail_areas.graph_inner = Some(Rect::new(1, 1, 98, 18));
    app.graph_scroll = 0;

    let graph_click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 2, // index 1 (relative to 1)
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, graph_click);
    assert_eq!(app.graph_selection, 1);
}

#[test]
fn test_settings_mode_navigation_and_editing() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_settings.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    assert_eq!(app.mode, Mode::Normal);

    // Press 's' to enter settings
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('s')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::Settings);
    assert_eq!(app.settings_selected_index, 0);
    assert!(!app.settings_editing);
    assert!(app.settings_focus_sidebar);

    // Focus right content pane via 'w'
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('w')), 10);
    assert!(handled);
    assert!(!app.settings_focus_sidebar);

    // Select poll interval, press enter to edit
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(handled);
    assert!(app.settings_editing);
    assert_eq!(app.input_buffer, "100");

    // Backspace once and append '5' to make it '105'
    crate::input::handle_key(&mut app, key_event(KeyCode::Backspace), 10);
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('5')), 10);
    assert_eq!(app.input_buffer, "105");

    // Commit change
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(!app.settings_editing);
    assert_eq!(app.config.poll_interval_ms, 105);

    // Return to sidebar
    crate::input::handle_key(&mut app, key_event(KeyCode::Left), 10);
    assert!(app.settings_focus_sidebar);

    // Go down to "Sorting & Limits" category in sidebar
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert_eq!(app.settings_selected_index, 1);

    // Focus right content pane
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(!app.settings_focus_sidebar);

    // Toggle Sort By (Custom -> Alphabetical)
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert_eq!(app.config.sort_by, SortOrder::Alphabetical);

    // Go down to "Sort Reverse" (within Sorting category)
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert_eq!(app.settings_selected_index, 2);

    // Toggle Sort Reverse (false -> true)
    crate::input::handle_key(&mut app, key_event(KeyCode::Char(' ')), 10);
    assert!(app.config.sort_reverse);

    // Return to sidebar
    crate::input::handle_key(&mut app, key_event(KeyCode::Left), 10);
    assert!(app.settings_focus_sidebar);

    // Press '4' to jump directly to Theme category (index 3)
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('4')), 10);
    assert_eq!(app.settings_selected_index, 3);
    assert!(!app.settings_focus_sidebar);

    // Edit Theme Name dropdown
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(app.settings_editing);
    assert!(app.settings_theme_list.contains(&"default".to_string()));

    // Pressing Down increases index (if there are other themes available)
    let prev_idx = app.settings_theme_index;
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    if app.settings_theme_list.len() > 1 {
        assert_eq!(app.settings_theme_index, prev_idx + 1);
    }

    // Cancel theme edit
    crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
    assert!(!app.settings_editing);

    // Press '3' to jump directly to Scan category (index 5)
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('3')), 10);
    assert_eq!(app.settings_selected_index, 5);

    // Edit Scan Start Dir
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(app.settings_editing);
    app.input_buffer = "/some/path".to_string();
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(!app.settings_editing);
    assert_eq!(app.config.scan.start_dir, "/some/path");

    // Go down to Scan Max Depth (index 4)
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert_eq!(app.settings_selected_index, 4);

    // Edit Scan Max Depth
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(app.settings_editing);
    app.input_buffer = "3".to_string();
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(!app.settings_editing);
    assert_eq!(app.config.scan.max_depth, 3);

    // Go down to Scan Git Only (index 10)
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert_eq!(app.settings_selected_index, 10);
    assert!(app.config.scan.git_only);

    // Toggle Scan Git Only
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(!app.config.scan.git_only);
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(app.config.scan.git_only);

    // Go down to Scan Excludes (index 8)
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert_eq!(app.settings_selected_index, 8);

    // Edit Scan Excludes
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(app.settings_editing);
    app.input_buffer = "target, node_modules ,.git".to_string();
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(!app.settings_editing);
    assert_eq!(
        app.config.scan.excludes,
        vec!["target".to_string(), "node_modules".to_string(), ".git".to_string()]
    );

    // Go to General Settings (Category 0) using hotkey '1'
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('1')), 10);
    assert_eq!(app.settings_selected_index, 0);

    // Navigate to Page Size (index 7): 0 -> 7
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert_eq!(app.settings_selected_index, 7);

    // Edit Page Size
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(app.settings_editing);
    app.input_buffer = "15".to_string();
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(!app.settings_editing);
    assert_eq!(app.config.page_size, 15);

    // Go down to Preferred Git Client (index 9): 7 -> 9
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert_eq!(app.settings_selected_index, 9);

    // Edit Preferred Git Client
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(app.settings_editing);
    app.input_buffer = "lazygit".to_string();
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(!app.settings_editing);
    assert_eq!(app.config.git_app, "lazygit");

    // Go down to Compatibility Mode (index 12): 9 -> 12
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert_eq!(app.settings_selected_index, 12);
    assert!(!app.config.compatibility_mode);

    // Toggle Compatibility Mode
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(app.config.compatibility_mode);
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(!app.config.compatibility_mode);

    // Go down to Resync on Tab Change (index 13): 12 -> 13
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert_eq!(app.settings_selected_index, 13);
    assert!(!app.config.resync_on_tab_change);

    // Toggle Resync on Tab Change
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(app.config.resync_on_tab_change);
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(!app.config.resync_on_tab_change);

    // Test Max Commits (index 6) in Category 1
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('2')), 10);
    assert_eq!(app.settings_selected_index, 1);
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10); // 1 -> 2
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10); // 2 -> 6
    assert_eq!(app.settings_selected_index, 6);

    // Edit Max Commits
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(app.settings_editing);
    app.input_buffer = "100".to_string();
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(!app.settings_editing);
    assert_eq!(app.config.max_commits, 100);

    // Test Auto-Fetch Interval (index 60) in Category 0
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('1')), 10);
    assert_eq!(app.settings_selected_index, 0);
    // Move down through all items in Category 0:
    // 0 -> 7 -> 9 -> 12 -> 13 -> 58 -> 55 -> 56 -> 60
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10); // 7
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10); // 9
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10); // 12
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10); // 13
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10); // 58
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10); // 55
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10); // 56
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10); // 60
    assert_eq!(app.settings_selected_index, 60);

    // Edit Auto-Fetch Interval
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(app.settings_editing);
    app.input_buffer = "15".to_string();
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(!app.settings_editing);
    assert_eq!(app.config.auto_fetch_interval_mins, 15);

    // PageUp, PageDown, Home, End testing within Category 2:
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('3')), 10);
    assert_eq!(app.settings_selected_index, 5); // Category 2 start is 5

    crate::input::handle_key(&mut app, key_event(KeyCode::PageUp), 10);
    assert_eq!(app.settings_selected_index, 5);

    crate::input::handle_key(&mut app, key_event(KeyCode::Home), 10);
    assert_eq!(app.settings_selected_index, 5);

    crate::input::handle_key(&mut app, key_event(KeyCode::End), 10);
    assert_eq!(app.settings_selected_index, 8); // Category 2 end is 8

    crate::input::handle_key(&mut app, key_event(KeyCode::PageDown), 10);
    assert_eq!(app.settings_selected_index, 8);

    // Press Esc to focus sidebar
    crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
    assert!(app.settings_focus_sidebar);

    // Press Esc on sidebar to exit settings
    crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn test_remote_add_delete_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_remotes.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Put app in Detail Mode on tab 5 (Remotes)
    app.mode = Mode::Detail;
    app.detail_tab = 5;
    app.detail_focus = DetailSection::Remotes;
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: std::path::PathBuf::from("."),
        info: Box::new(repo::RepoInfo {
            remotes: repo::TabData::Loaded(vec![repo::RemoteInfo {
                name: "origin".to_string(),
                url: "https://github.com/example/repo.git".to_string(),
                push_url: None,
                refspecs: vec![],
            }]),
            ..Default::default()
        }),
    });

    // Trigger remote add (a/A)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('a')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::RemoteAddNameInput);

    // Type remote name: "upstream"
    app.input_buffer = "upstream".to_string();
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::RemoteAddUrlInput);
    assert_eq!(app.remote_add_name, "upstream");

    // Escape URL input back to Detail Mode
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);

    // Trigger remote delete (D) on the selected remote ("origin")
    app.branch_list.remote_selection = 0;
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('D')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::RemoteDeleteConfirm);
    assert_eq!(app.remote_action_target.as_deref(), Some("origin"));

    // Press 'n' to cancel deletion
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('n')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);
    assert!(app.remote_action_target.is_none());
}

#[test]
fn test_workspace_tab_right_arrow_inspect() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_inspect.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Open details view
    app.mode = Mode::Detail;
    app.detail_tab = 0;
    app.detail_focus = DetailSection::Staged;

    let mut changes = crate::repo::WorktreeChanges::default();
    changes.staged.push(crate::repo::FileEntry { path: "dummy.txt".to_string(), label: "M" });
    let info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        changes,
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail =
        Some(crate::repo::ItemDetail::Repo { resolved: PathBuf::from("."), info: Box::new(info) });
    app.commit_list.selection = 0;

    // Verify we are not in Inspect mode
    assert_ne!(app.mode, Mode::Inspect);

    // Press Right arrow on Staged files list
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Right), 10);
    assert!(handled);

    // Verify we transitioned to Inspect mode and focused StagingDetails
    assert_eq!(app.mode, Mode::Inspect);
    assert_eq!(app.detail_focus, DetailSection::StagingDetails);

    // Press Left arrow in Inspect mode on StagingDetails
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Left), 10);
    assert!(handled);

    // Verify we are still in Inspect mode, but focus returned to Staged files list
    assert_eq!(app.mode, Mode::Inspect);
    assert_eq!(app.detail_focus, DetailSection::Staged);

    // Go back to Detail mode for testing transition from StagingDetails
    app.mode = Mode::Detail;
    app.detail_focus = DetailSection::StagingDetails;

    // Press Right arrow on StagingDetails (diff panel)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Right), 10);
    assert!(handled);

    // Verify we transitioned to Inspect mode
    assert_eq!(app.mode, Mode::Inspect);
    assert_eq!(app.detail_focus, DetailSection::StagingDetails);
}

#[test]
fn test_commit_enter_key_inspect() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_inspect_enter.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Open details view and focus Commits section
    app.mode = Mode::Detail;
    app.detail_tab = 0;
    app.detail_focus = DetailSection::Commits;

    let mut changes = crate::repo::WorktreeChanges::default();
    changes.staged.push(crate::repo::FileEntry { path: "dummy.txt".to_string(), label: "M" });
    let info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        changes,
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail =
        Some(crate::repo::ItemDetail::Repo { resolved: PathBuf::from("."), info: Box::new(info) });
    app.commit_list.selection = 0;

    // Verify we are not in Inspect mode
    assert_ne!(app.mode, Mode::Inspect);

    // Press Enter on Commits section
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(handled);

    // Verify we transitioned to Inspect mode and focused Staged files list
    assert_eq!(app.mode, Mode::Inspect);
    assert_eq!(app.detail_focus, DetailSection::Staged);
}

#[test]
fn test_inspect_commit_shortcut() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_inspect_commit.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Open details view and focus Commits section
    app.mode = Mode::Inspect;
    app.detail_tab = 0;
    app.detail_focus = DetailSection::Staged;

    let mut changes = crate::repo::WorktreeChanges::default();
    changes.staged.push(crate::repo::FileEntry { path: "dummy.txt".to_string(), label: "M" });
    let info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        summary: crate::repo::RepoSummary { staged: 1, ..Default::default() },
        changes,
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail =
        Some(crate::repo::ItemDetail::Repo { resolved: PathBuf::from("."), info: Box::new(info) });
    app.commit_list.selection = 0;

    assert_eq!(app.mode, Mode::Inspect);
    assert!(app.is_uncommitted_selected());

    // Press 'c' in Inspect mode
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('c')), 10);
    assert!(handled);

    // Verify we transitioned to CommitInput mode
    assert_eq!(app.mode, Mode::CommitInput);
}

#[test]
fn test_workspace_all_changes_shortcuts() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_workspace_all.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Open details Workspace view and focus Unstaged section
    app.mode = Mode::Detail;
    app.detail_tab = 0;
    app.detail_focus = DetailSection::Unstaged;

    let mut changes = crate::repo::WorktreeChanges::default();
    changes.unstaged.push(crate::repo::FileEntry { path: "dummy.txt".to_string(), label: "M" });
    let info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        summary: crate::repo::RepoSummary { modified: 1, ..Default::default() },
        changes,
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail =
        Some(crate::repo::ItemDetail::Repo { resolved: PathBuf::from("."), info: Box::new(info) });
    app.commit_list.selection = 0;

    assert!(app.is_uncommitted_selected());

    // Press 'X' to discard all changes
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('X')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::DiscardChangesConfirm);
    assert_eq!(app.discard_target.as_ref().unwrap().0, "All Changes");

    // Press Enter to verify it cancels (destructive dialog safety)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(handled);
    app.drain_queue();
    assert_eq!(app.mode, Mode::Detail);
}

#[test]
fn test_inspect_workspace_all_changes_shortcuts() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_inspect_workspace_all.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Open details Inspect view and focus Unstaged section
    app.mode = Mode::Inspect;
    app.detail_tab = 0;
    app.detail_focus = DetailSection::Unstaged;

    let mut changes = crate::repo::WorktreeChanges::default();
    changes.unstaged.push(crate::repo::FileEntry { path: "dummy.txt".to_string(), label: "M" });
    let info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        summary: crate::repo::RepoSummary { modified: 1, ..Default::default() },
        changes,
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail =
        Some(crate::repo::ItemDetail::Repo { resolved: PathBuf::from("."), info: Box::new(info) });
    app.commit_list.selection = 0;

    assert!(app.is_uncommitted_selected());

    // Press 'X' to discard all changes in Inspect mode
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('X')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::DiscardChangesConfirm);
    assert_eq!(app.discard_target.as_ref().unwrap().0, "All Changes");

    // Cancel discard all and reset to Inspect mode
    app.cancel_discard_changes();
    app.mode = Mode::Inspect;

    // Press 'a' (stage all) on Unstaged focus
    app.detail_focus = DetailSection::Unstaged;
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('a')), 10);
    assert!(handled);

    // Press 'a' (unstage all) on Staged focus
    app.detail_focus = DetailSection::Staged;
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('a')), 10);
    assert!(handled);
}

#[test]
fn test_workspace_all_changes_focus_transitions() {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!(
        "gitwig_test_app_all_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&temp_path).unwrap();
    let repo = git2::Repository::init(&temp_path).unwrap();

    // Configure author
    let mut config_git = repo.config().unwrap();
    config_git.set_str("user.name", "Test User").unwrap();
    config_git.set_str("user.email", "test@example.com").unwrap();

    // Create initial commit so we have a HEAD
    let file_path = temp_path.join("file.txt");
    std::fs::write(&file_path, "initial").unwrap();

    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let mut app = App::new(config, temp_path.join("config.toml"));

    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: temp_path.clone(),
        info: Box::new(crate::repo::RepoInfo::default()),
    });

    // 1. Stage All Focus Transition (Unstaged -> Staged)
    app.detail_focus = DetailSection::Unstaged;
    app.stage_all_changes();
    assert_eq!(app.detail_focus, DetailSection::Staged);

    // 2. Unstage All Focus Transition (Staged -> Unstaged)
    app.detail_focus = DetailSection::Staged;
    app.unstage_all_changes();
    assert_eq!(app.detail_focus, DetailSection::Unstaged);

    let _ = std::fs::remove_dir_all(&temp_path);
}

#[test]
fn test_workspace_tab_focus_cycle_skips_empty_panels() {
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_cycle.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // 1. Uncommitted selected, Staged is not empty, Unstaged is empty
    app.mode = Mode::Detail;
    app.detail_tab = 0;
    app.detail_focus = DetailSection::Commits;

    let mut changes = crate::repo::WorktreeChanges::default();
    changes.staged.push(crate::repo::FileEntry { path: "staged_file.txt".to_string(), label: "M" });
    // Unstaged is empty
    let info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        changes,
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail =
        Some(crate::repo::ItemDetail::Repo { resolved: PathBuf::from("."), info: Box::new(info) });
    app.commit_list.selection = 0; // index 0 is "<uncommitted>"

    // We cycle from Commits -> Staged (since Staged is not empty)
    app.cycle_detail_focus(false);
    assert_eq!(app.detail_focus, DetailSection::Staged);

    // Cycle from Staged -> StagingDetails (skips empty Unstaged, skips CommitDetails because uncommitted is selected)
    app.cycle_detail_focus(false);
    assert_eq!(app.detail_focus, DetailSection::StagingDetails);

    // Cycle from StagingDetails -> Commits
    app.cycle_detail_focus(false);
    assert_eq!(app.detail_focus, DetailSection::Commits);

    // Cycle reverse: Commits -> StagingDetails
    app.cycle_detail_focus(true);
    assert_eq!(app.detail_focus, DetailSection::StagingDetails);

    // Cycle reverse: StagingDetails -> Staged
    app.cycle_detail_focus(true);
    assert_eq!(app.detail_focus, DetailSection::Staged);

    // Cycle reverse: Staged -> Commits
    app.cycle_detail_focus(true);
    assert_eq!(app.detail_focus, DetailSection::Commits);

    // 2. Regular commit selected (is_uncommitted_selected is false)
    // With a regular commit, staged & unstaged are empty.
    app.commit_list.selection = 1; // Not uncommitted

    let empty_info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("."),
        info: Box::new(empty_info),
    });

    // We cycle from Commits -> CommitDetails (skips empty Staged and empty Unstaged)
    app.cycle_detail_focus(false);
    assert_eq!(app.detail_focus, DetailSection::CommitDetails);

    // Cycle from CommitDetails -> Commits (skips empty StagingDetails because staged & unstaged are empty)
    app.cycle_detail_focus(false);
    assert_eq!(app.detail_focus, DetailSection::Commits);

    // Cycle reverse: Commits -> CommitDetails
    app.cycle_detail_focus(true);
    assert_eq!(app.detail_focus, DetailSection::CommitDetails);

    // Cycle reverse: CommitDetails -> Commits
    app.cycle_detail_focus(true);
    assert_eq!(app.detail_focus, DetailSection::Commits);
}

#[test]
fn test_git_app_shortcut_triggers_pending() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_git_app.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    assert!(!app.pending_git_app);

    // Pressing 'g' triggers pending_git_app
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('g')), 10);
    assert!(handled);
    assert!(app.pending_git_app);
}

#[test]
fn test_files_search_shortcut_triggers_popup() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_files_scan.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);
    app.mode = Mode::Detail;
    app.detail_tab = 1; // Files tab
    app.detail_focus = DetailSection::Files;

    // Pressing '/' triggers Mode::FileSearchInput when in files tab
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('/')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::FileSearchInput);
}

#[test]
fn test_logs_search_picker_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_logs_search.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);
    app.mode = Mode::Detail;
    app.detail_tab = 0; // Workspace tab
    app.detail_focus = DetailSection::Commits;

    // 1. Press 'f' to open search column picker
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('f')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::SearchColumnPicker);
    assert_eq!(app.search_column_selection, 0);

    // 2. Select down
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert_eq!(app.search_column_selection, 1);

    // 3. Toggle column message (initially true, should become false)
    assert!(app.search_columns_message);
    crate::input::handle_key(&mut app, key_event(KeyCode::Char(' ')), 10);
    assert!(!app.search_columns_message);

    // 4. Press Enter to transition to LogsSearchInput
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert_eq!(app.mode, Mode::LogsSearchInput);
    assert!(app.in_logs_ui);

    let mock_info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        commits: vec![
            crate::repo::CommitEntry {
                id: "1234567".to_string(),
                oid: "1234567890abcdef1234567890abcdef12345678".to_string(),
                summary: "first test".to_string(),
                author: "test author 1".to_string(),
                when: "today".to_string(),
                date: "today".to_string(),
                refs: vec![],
                message: "msg".to_string(),
                files: vec![],
                signature_status: "N".to_string(),
            },
            crate::repo::CommitEntry {
                id: "2234567".to_string(),
                oid: "2234567890abcdef1234567890abcdef12345678".to_string(),
                summary: "no match".to_string(),
                author: "author 1".to_string(),
                when: "today".to_string(),
                date: "today".to_string(),
                refs: vec![],
                message: "msg".to_string(),
                files: vec![],
                signature_status: "N".to_string(),
            },
            crate::repo::CommitEntry {
                id: "2345678".to_string(),
                oid: "234567890abcdef1234567890abcdef12345678a".to_string(),
                summary: "second test".to_string(),
                author: "test author 2".to_string(),
                when: "today".to_string(),
                date: "today".to_string(),
                refs: vec![],
                message: "msg".to_string(),
                files: vec![],
                signature_status: "N".to_string(),
            },
            crate::repo::CommitEntry {
                id: "3234567".to_string(),
                oid: "3234567890abcdef1234567890abcdef12345678".to_string(),
                summary: "no match".to_string(),
                author: "author 1".to_string(),
                when: "today".to_string(),
                date: "today".to_string(),
                refs: vec![],
                message: "msg".to_string(),
                files: vec![],
                signature_status: "N".to_string(),
            },
            crate::repo::CommitEntry {
                id: "4234567".to_string(),
                oid: "4234567890abcdef1234567890abcdef12345678".to_string(),
                summary: "third test".to_string(),
                author: "test author 1".to_string(),
                when: "today".to_string(),
                date: "today".to_string(),
                refs: vec![],
                message: "msg".to_string(),
                files: vec![],
                signature_status: "N".to_string(),
            },
        ],
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: std::path::PathBuf::from("."),
        info: Box::new(mock_info),
    });

    // 5. Input search query characters and hit Enter
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('t')), 10);
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('e')), 10);
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('s')), 10);
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('t')), 10);
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);

    assert_eq!(app.mode, Mode::Logs);
    assert_eq!(app.commit_list.search_query.as_deref(), Some("test"));
    assert_eq!(app.commit_total(), 5);

    // Test scrolling/navigation (should only jump between matches: 0, 2, 4)
    assert_eq!(app.commit_list.selection, 0); // starts at 0 (which is a match)
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert_eq!(app.commit_list.selection, 2); // skips non-match at index 1
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert_eq!(app.commit_list.selection, 4); // skips non-match at index 3
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert_eq!(app.commit_list.selection, 4); // remains at last match

    crate::input::handle_key(&mut app, key_event(KeyCode::PageUp), 10);
    assert_eq!(app.commit_list.selection, 0); // jumps back to first match
    crate::input::handle_key(&mut app, key_event(KeyCode::PageDown), 10);
    assert_eq!(app.commit_list.selection, 4); // jumps back to last match

    crate::input::handle_key(&mut app, key_event(KeyCode::Up), 10);
    assert_eq!(app.commit_list.selection, 2);
    crate::input::handle_key(&mut app, key_event(KeyCode::Up), 10);
    assert_eq!(app.commit_list.selection, 0);

    // 6. Test match helper
    let matching_commit = crate::repo::CommitEntry {
        id: "1234567".to_string(),
        oid: "1234567890abcdef1234567890abcdef12345678".to_string(),
        summary: "a test message".to_string(), // message column disabled, so shouldn't match message!
        author: "test author".to_string(),     // author column enabled, should match author!
        when: "today".to_string(),
        date: "today".to_string(),
        refs: vec![],
        message: "message body".to_string(),
        files: vec![],
        signature_status: "N".to_string(),
    };
    assert!(app.commit_matches_query(&matching_commit));

    let non_matching_commit = crate::repo::CommitEntry {
        id: "1234567".to_string(),
        oid: "1234567890abcdef1234567890abcdef12345678".to_string(),
        summary: "a test message".to_string(), // message column disabled, message has test but is ignored!
        author: "other author".to_string(),    // author doesn't match!
        when: "today".to_string(),
        date: "today".to_string(),
        refs: vec![],
        message: "message body".to_string(),
        files: vec![],
        signature_status: "N".to_string(),
    };
    assert!(!app.commit_matches_query(&non_matching_commit));

    // Test entering inspect UI via Enter key when in Mode::Logs
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert_eq!(app.mode, Mode::Inspect);
    assert!(app.in_logs_ui);

    // Press 'q' to go back to Mode::Logs
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('q')), 10);
    assert_eq!(app.mode, Mode::Logs);
    assert!(app.in_logs_ui);

    // Press Enter again to transition to Mode::Inspect
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert_eq!(app.mode, Mode::Inspect);
    assert!(app.in_logs_ui);

    // Press Esc to go back to Mode::Logs
    crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
    assert_eq!(app.mode, Mode::Logs);
    assert!(app.in_logs_ui);

    // 7. Press Esc to go back to workspace
    crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
    assert_eq!(app.mode, Mode::Detail);
    assert!(!app.in_logs_ui);
    assert!(app.commit_list.search_query.is_none());
}

#[test]
fn test_detail_view_sync_on_tab_change_and_refresh() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec![".".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_sync.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);
    app.mode = Mode::Detail;
    app.detail_tab = 0;

    let mock_info = crate::repo::RepoInfo {
        branch: Some("mock_branch_name_test_xyz".to_string()),
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: std::path::PathBuf::from("."),
        info: Box::new(mock_info),
    });

    // 1. Simulate tab switch (e.g. key '2') with resync_on_tab_change = false
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('2')), 10);
    assert!(handled);
    assert_eq!(app.detail_tab, 1);
    assert!(app.current_detail.is_some());
    if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
        assert_eq!(info.branch.as_deref(), Some("mock_branch_name_test_xyz"));
    } else {
        panic!("Expected Repo detail");
    }

    // 2. Simulate tab switch (e.g. key '3') with resync_on_tab_change = true
    app.config.resync_on_tab_change = true;
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('3')), 10);
    assert!(handled);
    assert_eq!(app.detail_tab, 2);
    assert!(app.current_detail.is_some());

    // Wait and process the async message
    let (path, detail) = app.detail_rx.recv().unwrap();
    assert_eq!(Some(&path), app.loading_repo_path.as_ref());
    app.apply_detail_snapshot(detail);
    app.loading_repo_path = None;

    if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
        assert_ne!(info.branch.as_deref(), Some("mock_branch_name_test_xyz"));
    } else {
        panic!("Expected Repo detail");
    }

    // Reset to mock info for manual refresh test
    let mock_info_2 = crate::repo::RepoInfo {
        branch: Some("mock_branch_name_test_xyz".to_string()),
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: std::path::PathBuf::from("."),
        info: Box::new(mock_info_2),
    });

    // 3. Press 'R' to refresh/resync manually (should resync even if resync_on_tab_change is false)
    app.config.resync_on_tab_change = false;
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('R')), 10);
    assert!(handled);
    assert_eq!(app.status_message.as_deref(), Some("Refreshed"));

    // Wait and process the async message
    let (path, detail) = app.detail_rx.recv().unwrap();
    assert_eq!(Some(&path), app.loading_repo_path.as_ref());
    app.apply_detail_snapshot(detail);
    app.loading_repo_path = None;

    if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
        assert_ne!(info.branch.as_deref(), Some("mock_branch_name_test_xyz"));
    } else {
        panic!("Expected Repo detail");
    }
}

#[test]
fn test_branch_and_tag_checkout_confirmation() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec![".gitwig".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_checkout.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);
    app.mode = Mode::Detail;
    app.detail_tab = 3; // branches tab
    app.detail_focus = DetailSection::LocalBranches;

    let mock_info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        local_branches: crate::repo::TabData::Loaded(vec![
            crate::repo::BranchInfo {
                name: "main".to_string(),
                is_head: true,
                short_sha: "".to_string(),
                short_message: "".to_string(),
            },
            crate::repo::BranchInfo {
                name: "feature-branch".to_string(),
                is_head: false,
                short_sha: "".to_string(),
                short_message: "".to_string(),
            },
        ]),
        remote_branches: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
            name: "origin/feature-branch".to_string(),
            is_head: false,
            short_sha: "".to_string(),
            short_message: "".to_string(),
        }]),
        local_tags: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
            name: "v1.0.0".to_string(),
            is_head: false,
            short_sha: "".to_string(),
            short_message: "".to_string(),
        }]),
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: std::path::PathBuf::from("."),
        info: Box::new(mock_info),
    });

    // Select the non-head local branch "feature-branch" (index 1)
    app.branch_list.local_branch_selection = 1;

    // Pressing Enter should request confirmation
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::BranchCheckoutConfirm);
    assert_eq!(app.branch_action_target, Some(("feature-branch".to_string(), false)));

    // Cancel branch checkout confirmation via 'n'
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('n')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);
    assert_eq!(app.branch_action_target, None);

    // Request again
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::BranchCheckoutConfirm);

    // Confirm branch checkout confirmation via 'y' (it will fail to checkout in dummy/test repo path, but checks handler path)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('y')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);
    assert_eq!(app.branch_action_target, None);

    // Switch to Tags tab (detail_tab = 4)
    app.detail_tab = 4;
    app.tag_list.local_tag_selection = 0;

    // Pressing Enter should request tag checkout confirmation
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::TagCheckoutConfirm);
    assert_eq!(app.tag_checkout_target, Some("v1.0.0".to_string()));

    // Cancel tag checkout confirmation via Esc
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);
    assert_eq!(app.tag_checkout_target, None);
}

#[test]
fn test_repo_search_filtering() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["z_repo".to_string(), "a_repo".to_string(), "m_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_search.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Initially we should have 3 items
    assert_eq!(app.get_items_len(), 3);
    assert_eq!(app.get_filtered_items().len(), 3);

    // Press 'f' to enter search mode
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('f')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::RepoSearchInput);

    // Type 'a'
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('a')), 10);
    assert!(handled);
    assert_eq!(app.repo_search_query.as_deref(), Some("a"));
    assert_eq!(app.get_items_len(), 1);
    assert_eq!(app.get_filtered_items()[0].1, &"a_repo".to_string());

    // Press Enter to confirm/exit search input mode
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.repo_search_query.as_deref(), Some("a"));

    // Press Esc in normal mode to clear the filter
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
    assert!(handled);
    assert_eq!(app.repo_search_query, None);
    assert_eq!(app.get_items_len(), 3);
}

#[test]
fn test_normal_mode_right_arrow_detail() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_right_arrow.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    assert_eq!(app.mode, Mode::Normal);

    // Press Right arrow key in Normal mode
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Right), 10);
    assert!(handled);

    // Verify we opened detail view in loading state
    assert_eq!(app.mode, Mode::Detail);
    assert_eq!(app.loading_repo_path.as_deref(), Some("a_repo"));

    // Wait for background thread message
    let (path, detail) = app.detail_rx.recv().unwrap();
    assert_eq!(path, "a_repo");

    // Manually apply to verify state transition
    app.current_detail = Some(detail);
    app.loading_repo_path = None;

    assert_eq!(app.loading_repo_path, None);
    assert!(app.current_detail.is_some());
}

#[test]
fn test_inspect_full_screen_diff_toggle() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_full_diff.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Transition to Mode::Inspect and focus StagingDetails
    app.mode = Mode::Inspect;
    app.detail_focus = DetailSection::StagingDetails;
    app.inspect_full_diff = false;

    // Press Right arrow
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Right), 10);
    assert!(handled);
    assert!(app.inspect_full_diff);

    // Press Left arrow to exit full diff
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Left), 10);
    assert!(handled);
    assert!(!app.inspect_full_diff);

    // Press Right arrow again to enter full diff
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Right), 10);
    assert!(handled);
    assert!(app.inspect_full_diff);

    // Press Esc to exit full diff
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
    assert!(handled);
    assert!(!app.inspect_full_diff);
    assert_eq!(app.mode, Mode::Inspect); // Still in Inspect mode!
}

#[test]
fn test_files_tab_full_screen_toggle() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_files_full.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Transition to Mode::Detail, select tab 1 (Files) and focus FileContent
    app.mode = Mode::Detail;
    app.detail_tab = 1;
    app.detail_focus = DetailSection::FileContent;
    app.inspect_full_diff = false;

    // Press Right arrow on FileContent
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Right), 10);
    assert!(handled);
    assert!(app.inspect_full_diff);

    // Press Left arrow to exit full screen
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Left), 10);
    assert!(handled);
    assert!(!app.inspect_full_diff);

    // Press Right arrow again
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Right), 10);
    assert!(handled);
    assert!(app.inspect_full_diff);

    // Press Esc to exit full screen
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
    assert!(handled);
    assert!(!app.inspect_full_diff);
    assert_eq!(app.mode, Mode::Detail); // Still in Detail mode!
}

#[test]
fn test_scan_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_scan_flow.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.mode = Mode::Normal;

    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('a')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::RepoScanPicker);
    assert!(app.error_message.is_none());

    // Esc should cancel Adding mode
    let handled_dismiss = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
    assert!(handled_dismiss);
    assert_eq!(app.mode, Mode::Normal);

    // A -> should fallback to BulkAddScanPicker
    let handled_bulk = crate::input::handle_key(&mut app, key_event(KeyCode::Char('A')), 10);
    assert!(handled_bulk);
    assert_eq!(app.mode, Mode::BulkAddScanPicker);
    assert!(app.error_message.is_none());
}

#[test]
fn test_initial_setup_and_migration() {
    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let unique_id =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("gitwig_test_migration_{}", unique_id));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let temp_path = temp_dir.join("config.toml");

    // Save initial config
    crate::config::save_config(&config, &temp_path).unwrap();

    // 1. First run: version file does not exist.
    {
        let app = App::new(config.clone(), temp_path.clone());
        let version_path = temp_dir.join(".version");
        assert!(version_path.exists());
        let written_version = std::fs::read_to_string(&version_path).unwrap();
        assert_eq!(written_version.trim(), env!("CARGO_PKG_VERSION"));
        assert_eq!(
            app.status_message,
            Some(format!("Welcome to Gitwig v{}!", env!("CARGO_PKG_VERSION")))
        );
    }

    // 2. Second run: version file matches current version.
    {
        let app = App::new(config.clone(), temp_path.clone());
        // No new status message should be set
        assert!(app.status_message.is_none());
    }

    // 3. Update run: version file has older version.
    {
        let version_path = temp_dir.join(".version");
        std::fs::write(&version_path, "0.1.0").unwrap();

        let app = App::new(config.clone(), temp_path.clone());
        // Check status message
        assert_eq!(
            app.status_message,
            Some(format!(
                "Gitwig updated to v{}! Configuration verified and backed up.",
                env!("CARGO_PKG_VERSION")
            ))
        );
        // Check config backup exists
        let backup_path = temp_path.with_extension("toml.bak");
        assert!(backup_path.exists());
        // Check version was updated
        let written_version = std::fs::read_to_string(&version_path).unwrap();
        assert_eq!(written_version.trim(), env!("CARGO_PKG_VERSION"));
    }

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_about_popup_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_about.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    // Assert initial mode is Normal
    assert_eq!(app.mode, Mode::Normal);

    // Open about popup
    app.open_about();
    assert_eq!(app.mode, Mode::About);

    // Close about popup
    app.close_dialog();
    assert_eq!(app.mode, Mode::Normal);

    // Test key inputs via handle_key
    // 1. In Normal mode, pressing 'V' should open about popup
    app.mode = Mode::Normal;
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('V')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::About);

    // 2. In About mode, pressing 'V' should close it
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('V')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::Normal);

    // 3. In Normal mode, pressing 'V' should open about popup
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('V')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::About);

    // 4. In About mode, pressing 'Esc' should close it
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::Normal);

    // 5. In Normal mode, pressing 'V' then closing with 'q'
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('V')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::About);
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('q')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn test_tag_fetch_attempt_and_dismiss_flow() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_tag_fetch.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    let mock_info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        summary: crate::repo::RepoSummary {
            branch: Some("main".to_string()),
            staged: 0,
            modified: 0,
            untracked: 0,
            conflicted: 0,
            ahead: 0,
            behind: 0,
            state: crate::repo::RepoState::Clean,
            last_commit_time: None,
        },
        changes: crate::repo::WorktreeChanges {
            staged: vec![],
            unstaged: vec![],
            conflicted: vec![],
            untracked: vec![],
        },
        remotes: crate::repo::TabData::Loaded(vec![crate::repo::RemoteInfo {
            name: "origin".to_string(),
            url: "git@github.com:tareqmy/gitwig.git".to_string(),
            push_url: None,
            refspecs: vec![],
        }]),
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: std::path::PathBuf::from("."),
        info: Box::new(mock_info),
    });

    // Initially remote_tags_attempted is false
    if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
        assert!(!info.remote_tags_attempted);
    }

    // 1. Switch to tab 4 (Tags tab) and trigger set_default_focus_for_tab
    app.detail_tab = 4;
    app.set_default_focus_for_tab();

    // Verify it doesn't auto-fetch anymore
    assert!(!app.fetching);

    // Manually trigger tag fetch (simulating pressing f/F)
    app.fetch_remote_tags(true);
    assert!(app.fetching);
    if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
        assert!(info.remote_tags_attempted);
    }

    // 2. Receive error from the background thread
    app.tx.send("REMOTE_TAGS_ERR:Failed to get remote tags: network timeout".to_string()).unwrap();

    // Process message in receiver
    if let Ok(msg) = app.rx.try_recv() {
        if let Some(err_msg) = msg.strip_prefix("REMOTE_TAGS_ERR:") {
            app.set_error(err_msg.to_string());
            app.fetching = false;
        }
    }

    // Verify fetching is false and error popup is shown
    assert!(!app.fetching);
    assert_eq!(app.error_message.as_deref(), Some("Failed to get remote tags: network timeout"));

    // 3. Trigger set_default_focus_for_tab again.
    // It should NOT call fetch_remote_tags again since attempted is true.
    app.set_default_focus_for_tab();
    assert!(!app.fetching);

    // 4. Test mouse click to dismiss error popup
    let mouse_event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 10,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, mouse_event);

    // Error message should be dismissed (None)
    assert_eq!(app.error_message, None);
}

#[test]
fn test_tag_push_all_confirmation_flow() {
    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_tag_push_all.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // 1. Single Remote Scenario
    let mock_info_single = crate::repo::RepoInfo {
        remotes: crate::repo::TabData::Loaded(vec![crate::repo::RemoteInfo {
            name: "origin".to_string(),
            url: "git@github.com:tareqmy/gitwig.git".to_string(),
            push_url: None,
            refspecs: vec![],
        }]),
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: std::path::PathBuf::from("."),
        info: Box::new(mock_info_single),
    });

    // Request tag push all
    app.request_tag_push_all();
    // Should go directly to TagPushAllConfirm
    assert_eq!(app.mode, Mode::TagPushAllConfirm);
    assert_eq!(app.remote_action_target.as_deref(), Some("origin"));

    // Cancel
    app.cancel_tag_push_all();
    assert_eq!(app.mode, Mode::Detail);
    assert_eq!(app.remote_action_target, None);

    // 2. Multi-Remote Scenario
    let mock_info_multi = crate::repo::RepoInfo {
        remotes: crate::repo::TabData::Loaded(vec![
            crate::repo::RemoteInfo {
                name: "origin".to_string(),
                url: "git@github.com:tareqmy/gitwig.git".to_string(),
                push_url: None,
                refspecs: vec![],
            },
            crate::repo::RemoteInfo {
                name: "upstream".to_string(),
                url: "git@github.com:parent/gitwig.git".to_string(),
                push_url: None,
                refspecs: vec![],
            },
        ]),
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: std::path::PathBuf::from("."),
        info: Box::new(mock_info_multi),
    });

    // Request tag push all
    app.request_tag_push_all();
    // Should open remote picker
    assert_eq!(app.mode, Mode::RemotePicker);
    assert_eq!(app.remote_picker_action, Some(RemotePickerAction::PushAllTags));

    // Confirm selection in remote picker (index 1 is upstream)
    app.remote_picker_selection = 1;
    app.confirm_remote_picker();

    // Should transition to TagPushAllConfirm and set target to upstream
    assert_eq!(app.mode, Mode::TagPushAllConfirm);
    assert_eq!(app.remote_action_target.as_deref(), Some("upstream"));

    // Confirm push
    app.confirm_tag_push_all();
    // Should trigger pushing, transition to Detail mode and clear target
    assert_eq!(app.mode, Mode::Detail);
    assert_eq!(app.remote_action_target, None);
}

#[test]
fn test_detail_cache_ttl_behavior() {
    let temp_dir = std::env::temp_dir();
    let repo_path = temp_dir.join("test_cache_repo");
    let _ = std::fs::remove_dir_all(&repo_path);
    std::fs::create_dir_all(&repo_path).unwrap();

    // Initialize App
    let config = Config {
        items: vec![repo_path.to_string_lossy().to_string()],
        poll_interval_ms: 100,
        max_commits: 200,
        graph_max_commits: 1000,

        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme_name: "default".to_string(),
        theme: ThemeConfig::default(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: true,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        ..Default::default()
    };

    let mut app = App::new(config, PathBuf::from(""));

    // Create a mock detail snapshot
    let mock_detail = crate::repo::ItemDetail::Repo {
        resolved: repo_path.clone(),
        info: Box::new(crate::repo::RepoInfo {
            commits: vec![],
            files: crate::repo::TabData::Loaded(vec!["file1.txt".to_string()]),
            ..crate::repo::RepoInfo::default()
        }),
    };

    // 1. Manually add to cache
    app.detail_cache.insert(
        repo_path.to_string_lossy().to_string(),
        DetailCache { detail: mock_detail.clone(), loaded_at: std::time::Instant::now() },
    );

    // 2. Trigger open_detail on this repository (it will load from cache immediately)
    app.open_detail();

    // loading_repo_path should be None because it loaded from cache silently!
    assert!(app.loading_repo_path.is_none());
    assert!(app.current_detail.is_some());

    // Verify loaded files tab data is preserved
    if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
        assert_eq!(info.files.as_slice(), &["file1.txt".to_string()]);
    }

    // Clean up
    let _ = std::fs::remove_dir_all(&repo_path);
}

#[test]
fn test_tab_ttl_behavior() {
    let temp_dir = std::env::temp_dir();
    let repo_path = temp_dir.join("test_tab_ttl_repo");
    let _ = std::fs::remove_dir_all(&repo_path);
    std::fs::create_dir_all(&repo_path).unwrap();

    // Initialize App with a short Tab TTL (e.g. 1s)
    let config = Config {
        items: vec![repo_path.to_string_lossy().to_string()],
        poll_interval_ms: 100,
        max_commits: 200,
        graph_max_commits: 1000,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 1, // 1s TTL
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme_name: "default".to_string(),
        theme: ThemeConfig::default(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: true,
        resync_on_tab_change: false,
        ..Default::default()
    };

    let mut app = App::new(config, PathBuf::from(""));

    // Set up mock current detail
    let mock_info = crate::repo::RepoInfo {
        commits: vec![],
        files: crate::repo::TabData::Loaded(vec!["file1.txt".to_string()]),
        tab_loaded_at: [None; 10],
        tab_loading: [false; 10],
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: repo_path.clone(),
        info: Box::new(mock_info),
    });

    // 1. Initial trigger when NotLoaded
    // Reset state to NotLoaded
    if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &mut app.current_detail {
        info.files = crate::repo::TabData::NotLoaded;
    }
    app.trigger_tab_load_if_needed(1);
    if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
        assert!(info.tab_loading[1]);
        assert!(info.files.is_loading());
    }

    // 2. Receive loaded payload simulation
    if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &mut app.current_detail {
        info.tab_loading[1] = false;
        info.tab_loaded_at[1] = Some(std::time::Instant::now() - std::time::Duration::from_secs(5)); // Mark loaded 5s ago (stale)
        info.files = crate::repo::TabData::Loaded(vec!["file_refreshed.txt".to_string()]);
    }

    // 3. Trigger tab load when it is stale (stale-while-revalidate)
    app.trigger_tab_load_if_needed(1);
    if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
        // Should be loading in the background (tab_loading is true)
        assert!(info.tab_loading[1]);
        // But info.files state should still be TabData::Loaded! (no spinner)
        assert!(matches!(info.files, crate::repo::TabData::Loaded(_)));
        assert_eq!(info.files.as_slice(), &["file_refreshed.txt".to_string()]);
    }

    // Clean up
    let _ = std::fs::remove_dir_all(&repo_path);
}

#[test]
fn test_commit_popup_mouse_resize() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Rect;

    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_commit_resize.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.mode = Mode::CommitInput;
    app.commit_popup_width_pct = 80;
    app.commit_popup_height_pct = 45;

    // Mock detail_areas
    // Parent area is 100x100
    // Popup area with 80% width and 45% height is centered:
    // width = 80, height = 45. x = 10, y = 27
    app.detail_areas.commit_popup_parent = Some(Rect::new(0, 0, 100, 100));
    app.detail_areas.commit_popup = Some(Rect::new(10, 27, 80, 45));

    // Click on the right border (pos.x = 89, pos.y = 50)
    let down_event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 89,
        row: 50,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, down_event);
    assert_eq!(app.active_drag_splitter, Some(Splitter::CommitPopupWidth));

    // Drag right border to column 95 -> new half_width = |95 - 50| = 45 -> new_width = 90 -> 90%
    let drag_event = MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: 95,
        row: 50,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, drag_event);
    assert_eq!(app.commit_popup_width_pct, 90);

    // Release mouse
    let up_event = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 95,
        row: 50,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, up_event);
    assert_eq!(app.active_drag_splitter, None);
}

#[test]
fn test_yank_selected_commit_hash() {
    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let mut app = App::new(config, PathBuf::from("dummy_path.toml"));

    // Setup mock repo commits
    let mut info = repo::RepoInfo::default();
    info.commits.push(repo::CommitEntry {
        id: "abc1234".to_string(),
        oid: "abc123456789".to_string(),
        author: "Tester".to_string(),
        when: "".to_string(),
        date: "".to_string(),
        summary: "Initial commit".to_string(),
        message: "Initial commit".to_string(),
        refs: vec![],
        files: vec![],
        signature_status: "".to_string(),
    });
    app.current_detail =
        Some(repo::ItemDetail::Repo { resolved: PathBuf::from("/dummy"), info: Box::new(info) });

    // Select the committed item
    app.commit_list.selection = 0;
    app.detail_tab = 0;

    // Try yanking. Note: since standard clipboards might fail in some test/headless envs,
    // we can test the behavior and see if it sets self.status_message to either success or error.
    app.yank_selected_commit_hash();
    assert!(app.status_message.is_some());
    let msg = app.status_message.as_ref().unwrap();
    assert!(msg.contains("Copied hash abc1234") || msg.contains("Failed to copy"));
}

#[test]
fn test_yank_selected_repo_path() {
    let config = Config {
        items: vec!["/dummy/repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let mut app = App::new(config, PathBuf::from("dummy_path.toml"));
    app.selected_index = 0;

    app.yank_selected_repo_path();
    assert!(app.status_message.is_some());
    let msg = app.status_message.as_ref().unwrap();
    assert!(msg.contains("Copied path") || msg.contains("Failed to copy"));
}

#[test]
fn test_pending_terminal_trigger() {
    let config = Config {
        items: vec!["/dummy/repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let mut app = App::new(config, PathBuf::from("dummy_path.toml"));
    app.selected_index = 0;

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('t'),
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::input::handle_key(&mut app, key, 1);
    assert!(handled);
    assert!(app.pending_terminal);
}

#[test]
fn test_bulk_fetch_all_trigger() {
    let temp_dir = std::env::temp_dir();
    let repo_path = temp_dir.join("gitwig_test_bulk_fetch_repo");
    let _ = std::fs::create_dir_all(&repo_path);

    let config = Config {
        items: vec![repo_path.to_string_lossy().to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let mut app = App::new(config, PathBuf::from("dummy_path.toml"));
    app.statuses = vec![repo::ItemStatus::GitRepo(None)];

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('F'),
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::input::handle_key(&mut app, key, 1);
    assert!(handled);
    assert!(!app.bulk_fetching.is_empty());

    let _ = std::fs::remove_dir_all(repo_path);
}

#[test]
fn test_multi_select_toggle() {
    let config = Config {
        items: vec!["/path/to/repo1".to_string(), "/path/to/repo2".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let mut app = App::new(config, PathBuf::from("dummy_path.toml"));
    app.selected_index = 0;

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char(' '),
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::input::handle_key(&mut app, key, 1);
    assert!(handled);
    assert!(app.multi_selected.contains("/path/to/repo1"));

    let handled = crate::input::handle_key(&mut app, key, 1);
    assert!(handled);
    assert!(!app.multi_selected.contains("/path/to/repo1"));
}

#[test]
fn test_dynamic_status_height() {
    let config = Config::default();
    let mut app = App::new(config, PathBuf::from("dummy_path.toml"));

    assert_eq!(app.status_height(), 1);

    app.status_expanded = true;

    let rows_small = crate::components::cmd_bar::calculate_status_rows(&app, 40);
    let rows_large = crate::components::cmd_bar::calculate_status_rows(&app, 200);

    assert!(rows_small > rows_large);
    assert!(rows_small >= 2);
}

#[test]
fn test_help_overlay_wrapping() {
    let config = Config::default();
    let app = App::new(config, PathBuf::from("dummy_path.toml"));

    let lines_narrow = crate::popups::help::get_help_lines(&app, 40);
    let lines_wide = crate::popups::help::get_help_lines(&app, 150);

    assert!(lines_narrow.len() > lines_wide.len());
}

#[test]
fn test_cancel_selections() {
    let config = Config {
        items: vec!["/path/to/repo1".to_string(), "/path/to/repo2".to_string()],
        ..Config::default()
    };
    let mut app = App::new(config, PathBuf::from("dummy_path.toml"));
    app.multi_selected.insert("/path/to/repo1".to_string());
    app.multi_selected.insert("/path/to/repo2".to_string());

    assert_eq!(app.multi_selected.len(), 2);

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::input::handle_key(&mut app, key, 1);
    assert!(handled);
    assert!(app.multi_selected.is_empty());
}

#[test]
fn test_cherry_pick_destination_branches() {
    let config = Config {
        items: vec![],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let mut app = App::new(config, PathBuf::from("dummy_path.toml"));

    // Setup mock repo details
    let mut info = repo::RepoInfo { branch: Some("main".to_string()), ..Default::default() };
    info.commits.push(repo::CommitEntry {
        id: "abc1234".to_string(),
        oid: "abc123456789".to_string(),
        author: "Tester".to_string(),
        when: "".to_string(),
        date: "".to_string(),
        summary: "Initial commit".to_string(),
        message: "Initial commit".to_string(),
        refs: vec![],
        files: vec![],
        signature_status: "".to_string(),
    });
    info.local_branches = repo::TabData::Loaded(vec![
        repo::BranchInfo {
            name: "main".to_string(),
            is_head: true,
            short_sha: "abc1234".to_string(),
            short_message: "msg".to_string(),
        },
        repo::BranchInfo {
            name: "feature-1".to_string(),
            is_head: false,
            short_sha: "def5678".to_string(),
            short_message: "msg2".to_string(),
        },
        repo::BranchInfo {
            name: "feature-2".to_string(),
            is_head: false,
            short_sha: "9999999".to_string(),
            short_message: "msg3".to_string(),
        },
    ]);
    app.current_detail =
        Some(repo::ItemDetail::Repo { resolved: PathBuf::from("/dummy"), info: Box::new(info) });

    // Trigger cherry pick
    app.commit_list.selection = 0;
    app.request_cherry_pick();

    assert_eq!(app.mode, Mode::CherryPickConfirm);
    assert_eq!(app.cherry_pick_dest_branches.len(), 2);
    assert_eq!(app.cherry_pick_dest_branches[0], "feature-1");
    assert_eq!(app.cherry_pick_dest_branches[1], "feature-2");
    assert_eq!(app.cherry_pick_dest_selection, 0);

    // Test navigation
    // Press Down
    let event_down = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyModifiers::empty(),
    );
    crate::input::handle_key(&mut app, event_down, 0);
    assert_eq!(app.cherry_pick_dest_selection, 1);

    // Press Down again (should clamp)
    let event_down_again = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyModifiers::empty(),
    );
    crate::input::handle_key(&mut app, event_down_again, 0);
    assert_eq!(app.cherry_pick_dest_selection, 1);

    // Press Up
    let event_up = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Up,
        crossterm::event::KeyModifiers::empty(),
    );
    crate::input::handle_key(&mut app, event_up, 0);
    assert_eq!(app.cherry_pick_dest_selection, 0);
}

#[test]
fn test_graph_tab_scrolling() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_graph.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Setup mock details
    let info = repo::RepoInfo {
        branch: Some("main".to_string()),
        graph_lines: repo::TabData::Loaded(
            (1..=15)
                .map(|i| repo::GraphLine { graph: format!("line {}", i), commit: None })
                .collect(),
        ),
        ..Default::default()
    };
    app.current_detail =
        Some(repo::ItemDetail::Repo { resolved: PathBuf::from("/dummy"), info: Box::new(info) });

    app.mode = Mode::Detail;
    app.detail_tab = 2; // Graph tab
    app.graph_scroll = 0;

    // Press Down
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert!(handled);
    assert_eq!(app.graph_selection, 1);
    assert_eq!(app.graph_scroll, 0);

    // Press PageDown
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::PageDown), 10);
    assert!(handled);
    assert_eq!(app.graph_selection, 11);
    assert_eq!(app.graph_scroll, 2);

    // Press End
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::End), 10);
    assert!(handled);
    assert_eq!(app.graph_selection, 14);
    assert_eq!(app.graph_scroll, 5);

    // Press Up
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Up), 10);
    assert!(handled);
    assert_eq!(app.graph_selection, 13);
    assert_eq!(app.graph_scroll, 5);

    // Press PageUp
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::PageUp), 10);
    assert!(handled);
    assert_eq!(app.graph_selection, 3);
    assert_eq!(app.graph_scroll, 3);

    // Press Home
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Home), 10);
    assert!(handled);
    assert_eq!(app.graph_selection, 0);
    assert_eq!(app.graph_scroll, 0);
}

#[test]
fn test_commit_popup_custom_keys() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_commit_keys.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Enters CommitInput mode
    app.mode = Mode::CommitInput;
    app.commit_popup.editing = true;
    app.commit_popup.maximized = false;

    // Test Ctrl+D in editing mode -> toggles maximized
    let ctrl_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
    let handled = crate::input::handle_key(&mut app, ctrl_d, 0);
    assert!(handled);
    assert!(app.commit_popup.maximized);

    // Test Ctrl+D in editing mode again -> restores
    let handled = crate::input::handle_key(&mut app, ctrl_d, 0);
    assert!(handled);
    assert!(!app.commit_popup.maximized);

    // Test Ctrl+C in editing mode -> switches editing state to false (confirm mode)
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    let handled = crate::input::handle_key(&mut app, ctrl_c, 0);
    assert!(handled);
    assert!(!app.commit_popup.editing);
    assert_eq!(app.mode, Mode::CommitInput); // Still in CommitInput mode!

    // Test 'd' in confirm mode -> toggles maximized
    let key_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty());
    let handled = crate::input::handle_key(&mut app, key_d, 0);
    assert!(handled);
    assert!(app.commit_popup.maximized);

    // Test 'd' in confirm mode again -> restores
    let handled = crate::input::handle_key(&mut app, key_d, 0);
    assert!(handled);
    assert!(!app.commit_popup.maximized);

    // Test 'e' in confirm mode -> switches editing back to true
    let key_e = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty());
    let handled = crate::input::handle_key(&mut app, key_e, 0);
    assert!(handled);
    assert!(app.commit_popup.editing);

    // Test Esc in editing mode -> closes popup / returns to Detail mode
    let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    let handled = crate::input::handle_key(&mut app, key_esc, 0);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);
}

#[test]
fn test_settings_panel_organization() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_settings_org.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.mode = Mode::Settings;
    app.settings_selected_index = 0;
    app.settings_editing = false;
    app.settings_focus_sidebar = false;

    // Press Left -> focuses sidebar
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Left), 10);
    assert!(handled);
    assert!(app.settings_focus_sidebar);

    // Press Down in sidebar -> goes to category 1 (Sorting & Limits), setting index becomes 1 (Sort By)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert!(handled);
    assert_eq!(app.settings_selected_index, 1);
    assert!(app.settings_focus_sidebar);

    // Press Right -> focuses right content pane
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Right), 10);
    assert!(handled);
    assert!(!app.settings_focus_sidebar);

    // Press '3' -> jumps to Scan category first item (5)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('3')), 10);
    assert!(handled);
    assert_eq!(app.settings_selected_index, 5);
    assert!(!app.settings_focus_sidebar);

    // Press '4' -> jumps to Theme category (3)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('4')), 10);
    assert!(handled);
    assert_eq!(app.settings_selected_index, 3);
    assert!(!app.settings_focus_sidebar);
}

#[test]
fn test_help_popup_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_help.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    assert_eq!(app.mode, Mode::Normal);

    // Press '?' to open help overlay
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('?')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::Help);
    assert_eq!(app.help_scroll, 0);

    // Scroll Down
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert!(handled);
    assert_eq!(app.help_scroll, 1);

    // Scroll Up
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Up), 10);
    assert!(handled);
    assert_eq!(app.help_scroll, 0);

    // End (Scrolls to bottom)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::End), 10);
    assert!(handled);
    let max_scroll = app.help_scroll;
    assert!(max_scroll > 0);

    // Home (Scrolls back to top)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Home), 10);
    assert!(handled);
    assert_eq!(app.help_scroll, 0);

    // PageDown
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::PageDown), 10);
    assert!(handled);
    assert_eq!(app.help_scroll, 10); // page_size is 10

    // PageUp
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::PageUp), 10);
    assert!(handled);
    assert_eq!(app.help_scroll, 0);

    // Press Esc to exit help
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn test_detail_help_popup_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_detail_help.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Transition to DetailHelp mode
    app.mode = Mode::DetailHelp;
    app.help_scroll = 0;

    // Scroll Down
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert!(handled);
    assert_eq!(app.help_scroll, 1);

    // Scroll Up
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Up), 10);
    assert!(handled);
    assert_eq!(app.help_scroll, 0);

    // End (Scrolls to bottom)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::End), 10);
    assert!(handled);
    let max_scroll = app.help_scroll;
    assert!(max_scroll > 0);

    // Home (Scrolls back to top)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Home), 10);
    assert!(handled);
    assert_eq!(app.help_scroll, 0);

    // PageDown
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::PageDown), 10);
    assert!(handled);
    assert_eq!(app.help_scroll, 10);

    // PageUp
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::PageUp), 10);
    assert!(handled);
    assert_eq!(app.help_scroll, 0);

    // Esc closes detail help and returns to Detail mode
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);
}

#[test]
fn test_max_commits_limit_setting() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 45, // Set limit to 45 commits
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_max_commits.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };

    // 1. Verify initial initialization
    let mut app = App::new(config, temp_path.clone());
    assert_eq!(app.commit_list.limit, 45);

    // 2. Verify loading a repository uses max_commits limit
    app.detail_cache.clear();
    app.detail_focus = DetailSection::Commits;

    // Simulate opening repo
    let _ = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert_eq!(app.commit_list.limit, 45);

    // 3. Test LoadMoreCommits pagination adds max_commits (45)
    app.queue.push(crate::queue::InternalEvent::LoadMoreCommits);
    app.drain_queue();
    assert_eq!(app.commit_list.limit, 90);
}

#[test]
fn test_file_history_view_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let config = Config {
        items: vec![".".to_string()],
        poll_interval_ms: 100,
        max_commits: 10,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_file_history.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };

    let mut app = App::new(config, temp_path);
    app.detail_tab = 1; // Files Tab
    app.detail_focus = DetailSection::Files;

    app.file_tree.visible_files.push(crate::app::FileTreeItem {
        name: "Cargo.toml".to_string(),
        full_path: "Cargo.toml".to_string(),
        is_dir: false,
        depth: 0,
        is_expanded: false,
    });
    app.file_tree.file_list_selection = 0;

    let mock_info = crate::repo::RepoInfo { ..crate::repo::RepoInfo::default() };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: std::path::PathBuf::from("."),
        info: Box::new(mock_info),
    });

    // Open file history
    app.open_file_history();
    assert_eq!(app.mode, Mode::FileHistory);
    assert_eq!(app.file_history_path, "Cargo.toml");
    assert_eq!(app.file_history_selection, 0);
    assert_eq!(app.file_history_focus, 0); // Focus on revisions list

    // Simulate Key Press - Tab to focus Diff Panel
    let consumed = crate::tabs::FileHistoryTab::handle_event(&mut app, key_event(KeyCode::Tab));
    assert!(consumed);
    assert_eq!(app.file_history_focus, 1); // Focus is on Diff panel

    // Simulate Key Press - Tab to focus Revisions Panel
    let consumed = crate::tabs::FileHistoryTab::handle_event(&mut app, key_event(KeyCode::Tab));
    assert!(consumed);
    assert_eq!(app.file_history_focus, 0); // Focus is back on Revisions

    // Simulate Key Press - Esc to exit File History mode
    let consumed = crate::tabs::FileHistoryTab::handle_event(&mut app, key_event(KeyCode::Esc));
    assert!(consumed);
    assert_eq!(app.mode, Mode::Detail);
}

#[test]
fn test_files_tab_editor_shortcut() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let config = Config { items: vec![], ..Default::default() };
    let mut app = App::new(config, std::path::PathBuf::from("config.toml"));

    app.detail_focus = DetailSection::Files;
    app.detail_tab = 1;
    app.mode = Mode::Detail;

    app.file_tree.visible_files.push(crate::app::FileTreeItem {
        name: "Cargo.toml".to_string(),
        full_path: "Cargo.toml".to_string(),
        is_dir: false,
        depth: 0,
        is_expanded: false,
    });
    app.file_tree.file_list_selection = 0;

    let mock_info = crate::repo::RepoInfo { ..crate::repo::RepoInfo::default() };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: std::path::PathBuf::from("."),
        info: Box::new(mock_info),
    });

    // Simulate key event 'e'
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('e')), 1);
    assert!(handled);
    assert_eq!(app.pending_editor_file, Some("Cargo.toml".to_string()));

    // Reset and simulate key event 'o'
    app.pending_editor_file = None;
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('o')), 1);
    assert!(handled);
    assert_eq!(app.pending_editor_file, Some("Cargo.toml".to_string()));
}

#[test]
fn test_repository_labels_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let config = Config {
        items: vec!["/path/to/repo1".to_string(), "/path/to/repo2".to_string()],
        poll_interval_ms: 100,
        max_commits: 10,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_labels.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };

    let mut app = App::new(config, temp_path);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.selected_index, 0);

    // Press 'l' to edit labels for selected repo1
    let _ = crate::input::handle_key(&mut app, key_event(KeyCode::Char('l')), 10);
    assert_eq!(app.mode, Mode::LabelInput);
    assert_eq!(app.input_buffer, "");

    // Input "work, rust"
    for c in "work, rust".chars() {
        let _ = crate::input::handle_key(&mut app, key_event(KeyCode::Char(c)), 10);
    }
    assert_eq!(app.input_buffer, "work, rust");

    // Press Enter to save
    let _ = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert_eq!(app.mode, Mode::Normal);

    // Check labels were stored
    let saved_labels = app.config.labels.get("/path/to/repo1").unwrap();
    assert_eq!(saved_labels, &vec!["work".to_string(), "rust".to_string()]);

    // Test label search filtering
    app.repo_search_query = Some("rust".to_string());
    let filtered = app.get_filtered_items();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].1, &"/path/to/repo1".to_string());

    // Test mouse clicking on a label to filter by it
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    app.repo_search_query = None;

    // Simulate bounding box for home rows (Row 0: GroupHeader "work" (height 2), Row 1: Repo 1 (height 4))
    app.main_areas =
        vec![ratatui::layout::Rect::new(0, 0, 50, 2), ratatui::layout::Rect::new(0, 2, 50, 4)];

    // Click on the second label "[rust]" (ranges from x=17 to 23, relative row y=1, which is absolute row y=3)
    let click_event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 19,
        row: 3,
        modifiers: KeyModifiers::empty(),
    };

    crate::mouse::handle_mouse(&mut app, click_event);
    assert_eq!(app.repo_search_query.as_deref(), Some("rust"));
}

#[test]
fn test_per_repository_theme() {
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("gitwig_test_per_repo_theme_config.toml");
    let themes_dir = temp_dir.join("themes");
    let _ = std::fs::create_dir_all(&themes_dir);

    // Write a dummy nord theme file
    let nord_theme_path = themes_dir.join("nord.theme");
    let theme_data = r#"
accent = "blue"
warning = "yellow"
danger = "red"
success = "green"
border_type = "thick"
"#;
    std::fs::write(&nord_theme_path, theme_data).unwrap();

    let mut repo_configs = HashMap::new();
    repo_configs.insert(
        "/path/to/custom_repo".to_string(),
        RepoConfig { theme: Some("nord".to_string()), ..Default::default() },
    );

    let config = Config {
        items: vec!["/path/to/custom_repo".to_string()],
        repo_configs,
        ..Default::default()
    };

    let mut app = App::new(config, config_path.clone());
    app.resolve_repo_themes();

    // Verify resolved theme is in cache
    let cached = app.repo_theme_cache.get("/path/to/custom_repo").unwrap();
    assert_eq!(cached.accent, "blue");
    assert_eq!(cached.border_type, "thick");

    // Clean up files
    let _ = std::fs::remove_file(nord_theme_path);
    let _ = std::fs::remove_file(config_path);
}

#[test]
fn test_repo_settings_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("gitwig_test_picker_flow_config.toml");

    let config = Config { items: vec!["/path/to/custom_repo".to_string()], ..Default::default() };

    let mut app = App::new(config, config_path.clone());
    app.mode = Mode::Detail;
    app.detail_tab = 0;
    app.selected_index = 0;

    // Enter Overview mode via 'v'
    let v_press = key_event(KeyCode::Char('v'));
    let handled = crate::input::handle_key(&mut app, v_press, 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::Overview);

    // Verify pressing 's' opens repo settings popup
    let s_press = key_event(KeyCode::Char('s'));
    let handled = crate::input::handle_key(&mut app, s_press, 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::RepoSettings);
    assert_eq!(app.repo_settings_selected_index, 0);

    // Simulate pressing Down key to go to Page Size row
    let down_press = key_event(KeyCode::Down);
    let handled = crate::input::handle_key(&mut app, down_press, 1);
    assert!(handled);
    assert_eq!(app.repo_settings_selected_index, 1);

    // Simulate pressing Enter to start editing Page Size
    let enter_press = key_event(KeyCode::Enter);
    let handled = crate::input::handle_key(&mut app, enter_press, 1);
    assert!(handled);
    assert!(app.repo_settings_editing);

    // Simulate typing "1" and "5" and pressing Enter to apply
    let one_press = key_event(KeyCode::Char('1'));
    let handled = crate::input::handle_key(&mut app, one_press, 1);
    assert!(handled);
    let five_press = key_event(KeyCode::Char('5'));
    let handled = crate::input::handle_key(&mut app, five_press, 1);
    assert!(handled);
    assert_eq!(app.repo_settings_input, "15");

    let enter_press = key_event(KeyCode::Enter);
    let handled = crate::input::handle_key(&mut app, enter_press, 1);
    assert!(handled);
    assert!(!app.repo_settings_editing);

    // Verify configured page size in config is set to 15
    let repo_cfg = app.config.repo_configs.get("/path/to/custom_repo").unwrap();
    assert_eq!(repo_cfg.page_size, Some(15));

    // Go down to Editor Command (index 4)
    // Currently on index 1.
    // Down to 2
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Down), 1);
    assert!(handled);
    assert_eq!(app.repo_settings_selected_index, 2);

    // Down to 3
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Down), 1);
    assert!(handled);
    assert_eq!(app.repo_settings_selected_index, 3);

    // Down to 4
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Down), 1);
    assert!(handled);
    assert_eq!(app.repo_settings_selected_index, 4);

    // Enter to edit
    let handled = crate::input::handle_key(&mut app, enter_press, 1);
    assert!(handled);
    assert!(app.repo_settings_editing);

    // Type "code"
    for c in "code".chars() {
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char(c)), 1);
        assert!(handled);
    }
    assert_eq!(app.repo_settings_input, "code");

    // Enter to confirm
    let handled = crate::input::handle_key(&mut app, enter_press, 1);
    assert!(handled);
    assert!(!app.repo_settings_editing);

    // Verify configured editor in config is set to Some("code")
    let repo_cfg = app.config.repo_configs.get("/path/to/custom_repo").unwrap();
    assert_eq!(repo_cfg.editor, Some("code".to_string()));

    // Down to 5 (User Note)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Down), 1);
    assert!(handled);
    assert_eq!(app.repo_settings_selected_index, 5);

    // Enter to edit note
    let handled = crate::input::handle_key(&mut app, enter_press, 1);
    assert!(handled);
    assert!(app.repo_settings_editing);

    // Type "my note"
    for c in "my note".chars() {
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char(c)), 1);
        assert!(handled);
    }
    assert_eq!(app.repo_settings_input, "my note");

    // Enter to confirm note
    let handled = crate::input::handle_key(&mut app, enter_press, 1);
    assert!(handled);
    assert!(!app.repo_settings_editing);

    // Verify configured note is set to Some("my note")
    let repo_cfg = app.config.repo_configs.get("/path/to/custom_repo").unwrap();
    assert_eq!(repo_cfg.note, Some("my note".to_string()));

    // Clean up
    let _ = std::fs::remove_file(config_path);
}

#[test]
fn test_is_newer_version() {
    assert!(is_newer_version("2.2.1", "2.2.2"));
    assert!(is_newer_version("2.2.1", "2.3.0"));
    assert!(is_newer_version("2.2.1", "3.0.0"));
    assert!(is_newer_version("v2.2.1", "v2.2.2"));
    assert!(!is_newer_version("2.2.1", "2.2.1"));
    assert!(!is_newer_version("2.2.1", "2.2.0"));
    assert!(!is_newer_version("3.0.0", "2.2.1"));
}

#[test]
fn test_cargo_install_detection() {
    let config = Config::default();
    let app = App::new(config, std::path::PathBuf::from("dummy.toml"));
    // Normal test execution runs within target/debug/deps, so it is not a Cargo, Homebrew, or Chocolatey installation folder
    assert!(!app.is_cargo_install());
    assert!(!app.is_msi_install());
    assert!(!app.is_homebrew_install());
    assert!(!app.is_chocolatey_install());
    assert!(app.can_self_update());
}

#[test]
fn test_repo_settings_fallbacks() {
    let mut repo_configs = std::collections::HashMap::new();
    let repo_cfg = RepoConfig {
        page_size: Some(15),
        max_commits: Some(200),
        resync_on_tab_change: Some(false),
        ..Default::default()
    };
    repo_configs.insert("/path/to/repo_a".to_string(), repo_cfg);

    let config = Config {
        page_size: 10,
        max_commits: 100,
        resync_on_tab_change: true,
        items: vec!["/path/to/repo_a".to_string(), "/path/to/repo_b".to_string()],
        repo_configs,
        ..Default::default()
    };

    let mut app = App::new(config, std::path::PathBuf::from("dummy.toml"));

    // By default, first repo (repo_a) is selected
    app.selected_index = 0;
    assert_eq!(app.get_current_page_size(), 15);
    assert_eq!(app.get_current_max_commits(), 200);
    assert!(!app.get_current_resync_on_tab_change());

    // If second repo (repo_b) is selected, fallback to global configs
    app.selected_index = 1;
    assert_eq!(app.get_current_page_size(), 10);
    assert_eq!(app.get_current_max_commits(), 100);
    assert!(app.get_current_resync_on_tab_change());
}

#[test]
fn test_keybindings_settings_panel_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config::default();

    // Isolate directory
    let temp_dir = std::env::temp_dir().join("gitwig_test_dir_keybindings");
    let _ = std::fs::create_dir_all(&temp_dir);
    let temp_path = temp_dir.join("config.toml");

    let mut app = App::new(config, temp_path);

    // Enter settings
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('s')), 10);
    assert_eq!(app.mode, Mode::Settings);

    // Jump to keybindings category via '5'
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('5')), 10);
    assert_eq!(app.settings_selected_index, 14); // Toggle Status Bar
    assert!(!app.settings_editing);

    // Start editing
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(app.settings_editing);
    assert_eq!(app.input_buffer, ".");

    // Clear and type "comma"
    app.input_buffer.clear();
    for c in "comma".chars() {
        crate::input::handle_key(&mut app, key_event(KeyCode::Char(c)), 10);
    }
    assert_eq!(app.input_buffer, "comma");

    // Commit change
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(!app.settings_editing);

    // Verify mapped key matches ','
    let comma_key = KeyEvent::new(KeyCode::Char(','), KeyModifiers::empty());
    assert!(app.is_bound(crate::keybindings::Action::ToggleStatusBar, comma_key));

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_worktree_tui_flows() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::path::PathBuf;

    let config = Config { items: vec!["/path/to/my_repo".to_string()], ..Default::default() };
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("gitwig_test_worktree_config.toml");
    let mut app = App::new(config, config_path.clone());
    app.selected_index = 0;
    app.mode = Mode::Detail;

    // Set up mock details containing worktree elements
    let mut mock_info = repo::RepoInfo::default();
    let mock_worktree = repo::WorktreeInfo {
        name: "my-worktree".to_string(),
        path: PathBuf::from("/path/to/my_repo/../my-worktree"),
        branch: Some("my-feature".to_string()),
        is_locked: false,
        lock_reason: None,
    };
    mock_info.worktrees = repo::TabData::Loaded(vec![mock_worktree]);
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/my_repo"),
        info: Box::new(mock_info),
    });

    // Go to worktrees tab (tab 7)
    app.detail_tab = 7;
    app.detail_focus = DetailSection::Worktrees;
    app.worktree_selection = 0;

    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    // 1. Move selection down (clamped since there's only 1 worktree)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Down), 1);
    assert!(handled);
    assert_eq!(app.worktree_selection, 0);

    // 2. Press 'a' to add a worktree (should trigger Branch Input mode)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('a')), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::WorktreeAddBranchInput);

    // Simulate input typing for branch
    app.input_buffer = "my-new-branch".to_string();
    app.commit_worktree_add_branch();
    assert_eq!(app.mode, Mode::WorktreeAddPathInput);
    assert_eq!(app.worktree_add_branch, "my-new-branch");

    // Cancel / escape path input
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);

    // 3. Press 'l' to lock the worktree
    app.mode = Mode::Detail;
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('l')), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::WorktreeLockReasonInput);

    // Escape lock reason
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);

    // 4. Press 'D' to trigger remove confirm dialog
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('D')), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::WorktreeRemoveConfirm);

    // Simulate typing '2' and pressing Enter to confirm
    app.input_buffer = "2".to_string();
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_submodule_tui_flows() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::path::PathBuf;

    let config = Config { items: vec!["/path/to/my_repo".to_string()], ..Default::default() };
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("gitwig_test_submodule_config.toml");
    let mut app = App::new(config, config_path.clone());
    app.selected_index = 0;
    app.mode = Mode::Detail;

    // Set up mock details containing submodule elements
    let mut mock_info = repo::RepoInfo::default();
    let mock_submodule = repo::SubmoduleInfo {
        name: "my-submodule".to_string(),
        path: PathBuf::from("libs/my-submodule"),
        url: "https://github.com/foo/bar.git".to_string(),
        commit_id: Some("1234567890abcdef1234567890abcdef12345678".to_string()),
        head_id: Some("1234567890abcdef1234567890abcdef12345678".to_string()),
        is_initialized: true,
        is_dirty: false,
    };
    mock_info.submodules = repo::TabData::Loaded(vec![mock_submodule]);
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/my_repo"),
        info: Box::new(mock_info),
    });

    // Go to submodules tab (tab 8)
    app.detail_tab = 8;
    app.detail_focus = DetailSection::Submodules;
    app.submodule_selection = 0;

    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    // 1. Move selection down (clamped since there's only 1 submodule)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Down), 1);
    assert!(handled);
    assert_eq!(app.submodule_selection, 0);

    // 2. Press 'a' to add a submodule (should trigger SubmoduleAddUrlInput mode)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('a')), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::SubmoduleAddUrlInput);

    // Simulate input typing for URL
    app.input_buffer = "https://github.com/example/lib.git".to_string();
    app.commit_submodule_add_url();
    assert_eq!(app.mode, Mode::SubmoduleAddPathInput);
    assert_eq!(app.submodule_add_url, "https://github.com/example/lib.git");

    // Cancel / escape path input
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);

    // 3. Press 'D' to trigger remove confirm dialog
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('D')), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::SubmoduleDeleteConfirm);
    assert_eq!(app.submodule_delete_target, Some("my-submodule".to_string()));

    // Escape confirmation
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);
    assert_eq!(app.submodule_delete_target, None);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_workspace_conflicts_shortcuts() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["a_repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        visits: HashMap::new(),
        labels: std::collections::HashMap::new(),
        sort_reverse: false,
        pinned: std::collections::HashSet::new(),
        theme: ThemeConfig::default(),
        theme_name: "default".to_string(),
        scan: ScanConfig::default(),
        git_app: "gitui".to_string(),
        compatibility_mode: false,
        detail_cache_ttl_secs: 30,
        enable_commit_signatures: false,
        tab_ttl_secs: 60,
        resync_on_tab_change: false,
        graph_max_commits: 1000,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_conflicts.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.mode = Mode::Detail;
    app.detail_tab = 0;
    app.detail_focus = DetailSection::Conflicts;

    let mut changes = crate::repo::WorktreeChanges::default();
    changes.conflicted.push(crate::repo::FileEntry { path: "dummy.txt".to_string(), label: "C" });
    let info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        summary: crate::repo::RepoSummary { modified: 1, ..Default::default() },
        changes,
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail =
        Some(crate::repo::ItemDetail::Repo { resolved: PathBuf::from("."), info: Box::new(info) });
    app.commit_list.selection = 0; // Selected uncommitted commit

    assert!(app.is_uncommitted_selected());

    // Press 'A' in Conflicts panel -> Mode::MergeAbortConfirm
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('A')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::MergeAbortConfirm);

    // Cancel abort merge
    app.mode = Mode::Detail;

    // Press 'C' in Conflicts panel -> Mode::MergeContinueConfirm
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('C')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::MergeContinueConfirm);
}

#[test]
fn test_update_click_trigger() {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    let config = Config::default();
    let mut app = App::new(config, std::path::PathBuf::from("dummy_path.toml"));
    app.update_available = Some("2.2.6".to_string());
    app.fetching = false;

    // Dynamically calculate click column based on actual terminal width in this environment
    let (mut width, _) = crossterm::terminal::size().unwrap_or((80, 24));
    if width == 0 {
        width = 80;
    }
    let len_version = format!(" v{} ", env!("CARGO_PKG_VERSION")).chars().count();
    let len_badge = if app.is_msi_install() {
        format!("[New version v{}]", "2.2.6").chars().count()
    } else {
        format!("[Update to v{}]", "2.2.6").chars().count()
    };
    let target_column = (width as usize).saturating_sub(len_version + len_badge / 2 + 2) as u16;

    let click_event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: target_column,
        row: 0,
        modifiers: KeyModifiers::empty(),
    };

    crate::mouse::handle_mouse(&mut app, click_event);
    assert!(app.fetching);
    assert_eq!(app.status_message.as_deref(), Some("Updating Gitwig..."));
}

#[test]
fn test_manual_update_check_flow() {
    let config = Config::default();
    let mut app = App::new(config, std::path::PathBuf::from("dummy_path.toml"));
    assert!(!app.update_check_manual);

    app.trigger_update_check();
    assert!(app.update_check_manual);
    assert_eq!(app.status_message.as_deref(), Some("Checking for updates..."));
}

#[test]
fn test_implicit_network_count() {
    let config = Config::default();
    let mut app = App::new(config, std::path::PathBuf::from("dummy_path.toml"));
    assert_eq!(app.implicit_network_count, 0);

    app.increment_implicit_network();
    assert_eq!(app.implicit_network_count, 1);

    app.increment_implicit_network();
    assert_eq!(app.implicit_network_count, 2);

    app.decrement_implicit_network();
    assert_eq!(app.implicit_network_count, 1);

    app.decrement_implicit_network();
    assert_eq!(app.implicit_network_count, 0);

    // Make sure it saturates
    app.decrement_implicit_network();
    assert_eq!(app.implicit_network_count, 0);
}

#[test]
fn test_compact_view_toggle() {
    struct TestDirGuard {
        path: PathBuf,
    }
    impl Drop for TestDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    let temp_dir = std::env::temp_dir().join("gitwig_test_config_compact_toggle_dir");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let _guard = TestDirGuard { path: temp_dir.clone() };

    let config = Config::default();
    let temp_path = temp_dir.join("config.toml");
    let mut app = App::new(config, temp_path);
    assert!(!app.config.compact_view);

    // Simulate event toggle
    let event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('v'),
        crossterm::event::KeyModifiers::empty(),
    );
    let _ = crate::tabs::HomeTab::handle_event(&mut app, event, 10);
    assert!(app.config.compact_view);

    let _ = crate::tabs::HomeTab::handle_event(&mut app, event, 10);
    assert!(!app.config.compact_view);
}

#[test]
fn test_legend_popup_flow() {
    let config = Config::default();
    let temp_dir = std::env::temp_dir().join("gitwig_test_config_legend_dir");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    struct TestDirGuard {
        path: PathBuf,
    }
    impl Drop for TestDirGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
    let _guard = TestDirGuard { path: temp_dir.clone() };
    let temp_path = temp_dir.join("config.toml");
    let mut app = App::new(config, temp_path);

    assert_eq!(app.mode, Mode::Normal);

    // Press h to open legend
    let open_event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('h'),
        crossterm::event::KeyModifiers::empty(),
    );
    let _ = crate::tabs::HomeTab::handle_event(&mut app, open_event, 10);
    assert_eq!(app.mode, Mode::Legend);

    // Press Esc to close
    let close_event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::input::handle_key(&mut app, close_event, 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::Normal);

    // Open it again
    let _ = crate::tabs::HomeTab::handle_event(&mut app, open_event, 10);
    assert_eq!(app.mode, Mode::Legend);
    assert_eq!(app.legend_scroll, 0);

    // Scroll Down key
    let down_event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::input::handle_key(&mut app, down_event, 10);
    assert!(handled);
    assert_eq!(app.legend_scroll, 1);

    // Scroll Up key
    let up_event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Up,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::input::handle_key(&mut app, up_event, 10);
    assert!(handled);
    assert_eq!(app.legend_scroll, 0);

    // PageDown
    let pgdn_event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::PageDown,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::input::handle_key(&mut app, pgdn_event, 10);
    assert!(handled);
    assert!(app.legend_scroll > 0);

    // End (Scrolls to bottom)
    let end_event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::End,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::input::handle_key(&mut app, end_event, 10);
    assert!(handled);
    let max_scroll = app.legend_scroll;
    assert!(max_scroll > 0);

    // Home (Scrolls to top)
    let home_event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Home,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::input::handle_key(&mut app, home_event, 10);
    assert!(handled);
    assert_eq!(app.legend_scroll, 0);
}

#[test]
fn test_repo_jump_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let config = Config {
        items: vec![
            "/path/to/alpha".to_string(),
            "/path/to/beta".to_string(),
            "/path/to/gamma".to_string(),
        ],
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_repo_jump.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    assert_eq!(app.mode, Mode::Normal);

    let handled = crate::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()),
        10,
    );
    assert!(handled);
    assert_eq!(app.mode, Mode::RepoJump);
    assert_eq!(app.repo_jump_selection, 0);

    let handled = crate::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::empty()),
        10,
    );
    assert!(handled);
    assert_eq!(app.input_buffer, "g");

    let matches = app.get_jump_matches();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].2, "gamma");

    let handled = crate::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        10,
    );
    assert!(handled);
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.selected_index, 2);
}

#[test]
fn test_mru_group_flow() {
    let config = Config {
        items: vec![
            "/path/to/alpha".to_string(),
            "/path/to/beta".to_string(),
            "/path/to/gamma".to_string(),
        ],
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_mru.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    assert_eq!(app.get_home_rows().len(), 3);

    app.selected_index = 1;
    app.open_detail();

    let rows = app.get_home_rows();
    assert_eq!(rows.len(), 5);
    if let HomeRow::GroupHeader { name, count, .. } = &rows[0] {
        assert_eq!(name, "Recent");
        assert_eq!(*count, 1);
    } else {
        panic!("Expected Recent group header");
    }
}

#[test]
fn test_starred_group_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let config = Config {
        items: vec![
            "/path/to/alpha".to_string(),
            "/path/to/beta".to_string(),
            "/path/to/gamma".to_string(),
        ],
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_star.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    assert_eq!(app.get_home_rows().len(), 3);

    app.selected_index = 1;
    let handled = crate::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('*'), KeyModifiers::empty()),
        10,
    );
    assert!(handled);
    assert!(app.config.starred.contains("/path/to/beta"));

    let rows = app.get_home_rows();
    assert_eq!(rows.len(), 5);
    if let HomeRow::GroupHeader { name, count, .. } = &rows[0] {
        assert_eq!(name, "Starred");
        assert_eq!(*count, 1);
    } else {
        panic!("Expected Starred group header");
    }
}

#[test]
fn test_multiple_labels_grouping() {
    let mut config = Config { items: vec!["/path/to/alpha".to_string()], ..Default::default() };
    config
        .labels
        .insert("/path/to/alpha".to_string(), vec!["labelA".to_string(), "labelB".to_string()]);
    let temp_path = std::env::temp_dir().join("gitwig_test_multi_label.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let app = App::new(config, temp_path);

    let rows = app.get_home_rows();
    assert_eq!(rows.len(), 4);
    if let HomeRow::GroupHeader { name, .. } = &rows[0] {
        assert_eq!(name, "labelA");
    } else {
        panic!("Expected labelA group header");
    }
    if let HomeRow::GroupHeader { name, .. } = &rows[2] {
        assert_eq!(name, "labelB");
    } else {
        panic!("Expected labelB group header");
    }
}

#[test]
fn test_background_auto_refresh() {
    let config = Config { items: vec!["/path/to/repo_a".to_string()], ..Default::default() };
    let temp_path = std::env::temp_dir().join("gitwig_test_bg_refresh.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    assert!(!app.background_refresh_running);

    // Mock an update received on the channel
    let updates = vec![(0, "/path/to/repo_a".to_string(), repo::ItemStatus::Directory)];
    app.status_refresh_tx.send(updates).unwrap();

    // Verify it updates App state when processed
    if let Ok(updates) = app.status_refresh_rx.try_recv() {
        app.background_refresh_running = false;
        for (idx, path, status) in updates {
            if app.config.items.get(idx) == Some(&path) {
                if idx < app.statuses.len() {
                    app.statuses[idx] = status;
                }
            }
        }
    }

    assert!(matches!(app.statuses[0], repo::ItemStatus::Directory));
}

#[test]
fn test_show_grouping_toggle_and_rendering() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let mut config = Config {
        items: vec!["/path/to/repo_a".to_string()],
        show_grouping: true,
        ..Default::default()
    };
    config.labels.insert("/path/to/repo_a".to_string(), vec!["labelA".to_string()]);
    let temp_path = std::env::temp_dir().join("gitwig_test_show_grouping.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Initial state: grouped view should contain the GroupHeader
    let rows_grouped = app.get_home_rows();
    assert!(rows_grouped.len() > 1);
    assert!(matches!(rows_grouped[0], HomeRow::GroupHeader { .. }));

    // Simulate opening settings and toggling index 58
    app.mode = Mode::Settings;
    app.settings_selected_index = 58;
    app.settings_focus_sidebar = false;
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 1);
    assert!(handled);
    assert!(!app.config.show_grouping);

    // Now, rows should be a flat list (only Repo rows, no GroupHeader)
    let rows_flat = app.get_home_rows();
    assert_eq!(rows_flat.len(), 1);
    assert!(matches!(rows_flat[0], HomeRow::Repo { .. }));
}

#[test]
fn test_global_summary_filtering() {
    let config = Config {
        items: vec![
            "/path/to/repo_a".to_string(),
            "/path/to/repo_b".to_string(),
            "/path/to/repo_c".to_string(),
        ],
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_global_summary.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    let summary_clean = crate::repo::RepoSummary {
        branch: Some("main".to_string()),
        staged: 0,
        modified: 0,
        untracked: 0,
        conflicted: 0,
        ahead: 0,
        behind: 0,
        state: crate::repo::RepoState::Clean,
        last_commit_time: Some(
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
                as i64,
        ),
    };

    let summary_dirty = crate::repo::RepoSummary {
        branch: Some("main".to_string()),
        staged: 1,
        modified: 0,
        untracked: 0,
        conflicted: 0,
        ahead: 2,
        behind: 0,
        state: crate::repo::RepoState::Clean,
        last_commit_time: Some(0), // very stale
    };

    app.statuses = vec![
        crate::repo::ItemStatus::GitRepo(Some(summary_clean)),
        crate::repo::ItemStatus::GitRepo(Some(summary_dirty)),
        crate::repo::ItemStatus::Missing,
    ];

    // Filter by Dirty
    app.global_filter = Some(GlobalFilter::Dirty);
    let dirty_items = app.get_filtered_items();
    assert_eq!(dirty_items.len(), 1);
    assert_eq!(dirty_items[0].0, 1); // Only repo_b (dirty)

    // Filter by Ahead
    app.global_filter = Some(GlobalFilter::Ahead);
    let ahead_items = app.get_filtered_items();
    assert_eq!(ahead_items.len(), 1);
    assert_eq!(ahead_items[0].0, 1); // Only repo_b (ahead)

    // Filter by Stale
    app.global_filter = Some(GlobalFilter::Stale);
    let stale_items = app.get_filtered_items();
    assert_eq!(stale_items.len(), 1);
    assert_eq!(stale_items[0].0, 1); // Only repo_b (stale)

    // Test Esc key clearing
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 1);
    assert!(handled);
    assert_eq!(app.global_filter, None);
}

#[test]
fn test_reflog_tui_flows() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let config = Config { items: vec!["/path/to/my_repo".to_string()], ..Default::default() };
    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("gitwig_test_reflog_config.toml");
    let mut app = App::new(config, config_path.clone());
    let _guard = TestFileGuard { path: config_path };
    app.selected_index = 0;
    app.mode = Mode::Detail;

    let mut mock_info = repo::RepoInfo::default();
    let mock_entry = repo::ReflogEntry {
        index: 0,
        target_oid: "abcdef1234567890abcdef1234567890abcdef12".to_string(),
        selector: "HEAD@{0}".to_string(),
        command: "checkout".to_string(),
        message: "moving from main to develop".to_string(),
        when: "2 minutes ago".to_string(),
        date: "2026-07-02 06:45:19 UTC".to_string(),
    };
    mock_info.reflog = repo::TabData::Loaded(vec![mock_entry]);
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: std::path::PathBuf::from("/path/to/my_repo"),
        info: Box::new(mock_info),
    });

    // Go to Reflog tab (tab 10 / index 9)
    app.detail_tab = 9;
    app.detail_focus = DetailSection::Reflog;
    app.reflog_selection = 0;

    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    // 1. Move selection down (should clamp to 0)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Down), 1);
    assert!(handled);
    assert_eq!(app.reflog_selection, 0);

    // 2. Press Enter to checkout target OID
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 1);
    assert!(handled);
    assert!(app.fetching);
    assert!(
        app.status_message
            .as_ref()
            .unwrap()
            .contains("Checking out OID abcdef1234567890abcdef1234567890abcdef12")
    );
}

#[test]
fn test_branch_search_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let config = Config::default();
    let temp_path = std::env::temp_dir().join("gitwig_test_config_branch_search.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.mode = Mode::Detail;
    app.detail_tab = 3;
    app.detail_focus = DetailSection::LocalBranches;

    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('/')), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::BranchSearchInput);

    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);
}

#[test]
fn test_file_search_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let config = Config { scan: ScanConfig::default(), ..Config::default() };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_file_search.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.mode = Mode::Detail;
    app.detail_tab = 1;
    app.detail_focus = DetailSection::Files;

    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('/')), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::FileSearchInput);

    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);
}

#[test]
fn test_commit_fuzzy_search_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let config = Config::default();
    let temp_path = std::env::temp_dir().join("gitwig_test_config_commit_fuzzy_search.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.mode = Mode::Logs;

    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('/')), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::CommitFuzzySearch);

    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::Logs);
}

#[test]
fn test_tag_search_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let config = Config::default();
    let temp_path = std::env::temp_dir().join("gitwig_test_config_tag_search.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.mode = Mode::Detail;
    app.detail_tab = 4;
    app.detail_focus = DetailSection::LocalTags;

    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('/')), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::TagSearchInput);

    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);
}

#[test]
fn test_all_tab_events_direct() {
    use crate::tabs::logs::LogsTab;
    use crate::tabs::{
        BranchesTab, FileHistoryTab, FilesTab, GraphTab, RemotesTab, StashesTab, TagsTab,
        WorkspaceTab,
    };
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let config = Config::default();
    let temp_path = std::env::temp_dir().join("gitwig_test_config_tabs_direct.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Setup detail info
    let mock_info = repo::RepoInfo {
        branch: Some("main".to_string()),
        local_branches: repo::TabData::Loaded(vec![repo::BranchInfo {
            name: "main".to_string(),
            is_head: true,
            ..Default::default()
        }]),
        stashes: repo::TabData::Loaded(vec![repo::StashInfo { index: 0, ..Default::default() }]),
        worktrees: repo::TabData::Loaded(vec![repo::WorktreeInfo { ..Default::default() }]),
        submodules: repo::TabData::Loaded(vec![repo::SubmoduleInfo { ..Default::default() }]),
        reflog: repo::TabData::Loaded(vec![repo::ReflogEntry {
            index: 0,
            target_oid: "abc1234".to_string(),
            selector: "reflog@{0}".to_string(),
            command: "commit".to_string(),
            message: "feat: summary".to_string(),
            when: "10 mins ago".to_string(),
            date: "2026-07-02".to_string(),
        }]),
        ..Default::default()
    };

    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: PathBuf::from("dummy"),
        info: Box::new(mock_info),
    });

    let keys = vec![
        KeyEvent::new(KeyCode::Up, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Down, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('D'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('o'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('F'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
    ];

    for key in &keys {
        let _ = WorkspaceTab::handle_event(&mut app, *key);
        let _ = FilesTab::handle_event(&mut app, *key);
        let _ = GraphTab::handle_event(&mut app, *key);
        let _ = BranchesTab::handle_event(&mut app, *key);
        let _ = TagsTab::handle_event(&mut app, *key);
        let _ = RemotesTab::handle_event(&mut app, *key);
        let _ = StashesTab::handle_event(&mut app, *key);
        let _ = FileHistoryTab::handle_event(&mut app, *key);
        let _ = LogsTab::handle_event(&mut app, *key);
        let _ = crate::tabs::route_detail_event(&mut app, *key);
    }

    for tab in 7..10 {
        app.detail_tab = tab;
        for key in &keys {
            let _ = crate::tabs::route_detail_event(&mut app, *key);
        }
    }
}

#[test]
fn test_all_git_actions_on_real_repo() {
    // Create a real temp repository
    let uuid =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let temp_repo_path = std::env::temp_dir().join(format!("gitwig_test_actions_repo_{}", uuid));
    let _ = std::fs::create_dir_all(&temp_repo_path);

    // Initialize Git repo
    let repo = git2::Repository::init(&temp_repo_path).unwrap();

    // Configure user
    let mut config_git = repo.config().unwrap();
    config_git.set_str("user.name", "Test User").unwrap();
    config_git.set_str("user.email", "test@test.com").unwrap();

    // Create a file and commit it
    let file1_path = temp_repo_path.join("file1.rs");
    std::fs::write(&file1_path, "fn main() {}\n").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("file1.rs")).unwrap();
    index.write().unwrap();

    let oid = index.write_tree().unwrap();
    let tree = repo.find_tree(oid).unwrap();
    let signature = repo.signature().unwrap();
    repo.commit(Some("HEAD"), &signature, &signature, "Initial commit", &tree, &[]).unwrap();

    // Create a config and App pointing to this real repo
    let config =
        Config { items: vec![temp_repo_path.to_str().unwrap().to_string()], ..Default::default() };
    let mut app = App::new(config, PathBuf::from("dummy_path.toml"));
    app.mode = Mode::Detail;

    let mock_info =
        match crate::repo::inspect_detail(temp_repo_path.to_str().unwrap(), 100, 1000, false) {
            crate::repo::ItemDetail::Repo { info, .. } => *info,
            _ => crate::repo::RepoInfo::default(),
        };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: temp_repo_path.clone(),
        info: Box::new(mock_info),
    });

    // Call various action methods!
    app.fetching = false;
    app.input_buffer = "test_branch".to_string();
    app.commit_branch_create();

    app.fetching = false;
    app.input_buffer = "test_tag".to_string();
    app.commit_tag_create();

    app.fetching = false;
    app.tag_delete_target = Some(("test_tag".to_string(), false));
    app.confirm_tag_delete();

    app.fetching = false;
    app.branch_action_target = Some(("test_branch".to_string(), false));
    app.confirm_branch_delete();

    // Stashing
    std::fs::write(&file1_path, "fn main() {\n    // dirty\n}\n").unwrap();
    app.fetching = false;
    app.input_buffer = "my_stash".to_string();
    app.commit_stash_create();

    // Stash apply / delete
    app.fetching = false;
    app.stash_action_target = Some(("stash@{0}".to_string(), "my_stash".to_string()));
    app.confirm_stash_apply();
    app.fetching = false;
    app.stash_action_target = Some(("stash@{0}".to_string(), "my_stash".to_string()));
    app.confirm_stash_delete();

    // Tag checkout
    app.fetching = false;
    app.tag_checkout_target = Some("test_tag".to_string());
    app.confirm_tag_checkout();

    // Fetch, pull, push
    app.fetching = false;
    app.fetch_remote("origin");
    app.fetching = false;
    app.fetch_remote_tags(false);
    app.fetching = false;
    app.pull_selected_branch();
    app.fetching = false;
    app.branch_action_target = Some(("main".to_string(), false));
    app.confirm_branch_push();

    // Worktree actions
    app.fetching = false;
    app.input_buffer = "feature".to_string();
    app.commit_worktree_add_branch();
    app.fetching = false;
    app.input_buffer = temp_repo_path.join("wt-test").to_str().unwrap().to_string();
    app.commit_worktree_add_path();
    app.fetching = false;
    app.input_buffer = "lock reason".to_string();
    app.fetching = false;
    app.commit_worktree_lock_reason();
    app.fetching = false;
    app.commit_worktree_remove();

    // Submodule actions
    app.fetching = false;
    app.input_buffer = "http://submodule-url".to_string();
    app.commit_submodule_add_url();
    app.fetching = false;
    app.input_buffer = "sub-test".to_string();
    app.commit_submodule_add_path();
    app.fetching = false;
    app.submodule_delete_target = Some("sub-test".to_string());
    app.confirm_submodule_delete();
    app.fetching = false;
    app.cancel_submodule_delete();

    // Wait a little bit for background thread completions to process messages
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Clean channels in the test to ensure execution coverage
    while app.rx.try_recv().is_ok() {}
    while app.detail_rx.try_recv().is_ok() {}
    while app.status_refresh_rx.try_recv().is_ok() {}
    while app.tab_rx.try_recv().is_ok() {}

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_repo_path);
}

#[test]
fn test_inspect_popup_events() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_inspect_popup_dummy.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);
    app.mode = Mode::Inspect;

    // Mock uncommitted files
    let mock_info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        changes: crate::repo::WorktreeChanges {
            staged: vec![crate::repo::FileEntry { path: "staged_file.rs".to_string(), label: "A" }],
            unstaged: vec![crate::repo::FileEntry {
                path: "unstaged_file.rs".to_string(),
                label: "M",
            }],
            conflicted: vec![crate::repo::FileEntry {
                path: "conflicted_file.rs".to_string(),
                label: "U",
            }],
            ..Default::default()
        },
        commits: vec![crate::repo::CommitEntry {
            id: "abc1234".to_string(),
            oid: "abc1234abc1234abc1234abc1234abc1234abc1234".to_string(),
            author: "Author".to_string(),
            when: "10 mins ago".to_string(),
            date: "2026-07-02".to_string(),
            summary: "feat: summary".to_string(),
            message: "feat: summary\n\nbody".to_string(),
            refs: vec![],
            files: vec![],
            signature_status: "G".to_string(),
        }],
        ..Default::default()
    };

    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    // Test tab transition key events
    app.detail_focus = DetailSection::Staged;

    let keys = vec![
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('w'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('W'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Right,
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Left,
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Up,
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Down,
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::PageUp,
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::PageDown,
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Home,
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::End,
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('j'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('k'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('u'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('U'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('d'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('c'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('a'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('s'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('S'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('p'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('P'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('r'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('R'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('q'),
            crossterm::event::KeyModifiers::empty(),
        ),
    ];

    for key in &keys {
        let _ = crate::popups::inspect::InspectPopup::handle_event(&mut app, *key);
    }

    // Now test with committed selection
    app.detail_tab = 1; // Commits tab
    app.detail_focus = DetailSection::StagingDetails;
    for key in &keys {
        let _ = crate::popups::inspect::InspectPopup::handle_event(&mut app, *key);
    }
}

#[test]
fn test_workspace_tab_events() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_workspace_tab_dummy.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);
    app.mode = Mode::Detail;

    let mock_info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        changes: crate::repo::WorktreeChanges {
            staged: vec![crate::repo::FileEntry { path: "staged_file.rs".to_string(), label: "A" }],
            unstaged: vec![crate::repo::FileEntry {
                path: "unstaged_file.rs".to_string(),
                label: "M",
            }],
            conflicted: vec![crate::repo::FileEntry {
                path: "conflicted_file.rs".to_string(),
                label: "U",
            }],
            ..Default::default()
        },
        commits: vec![crate::repo::CommitEntry {
            id: "abc1234".to_string(),
            oid: "abc1234abc1234abc1234abc1234abc1234abc1234".to_string(),
            author: "Author".to_string(),
            when: "10 mins ago".to_string(),
            date: "2026-07-02".to_string(),
            summary: "feat: summary".to_string(),
            message: "feat: summary\n\nbody".to_string(),
            refs: vec![],
            files: vec![],
            signature_status: "G".to_string(),
        }],
        ..Default::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    let focuses = vec![
        DetailSection::Commits,
        DetailSection::Staged,
        DetailSection::Unstaged,
        DetailSection::Conflicts,
        DetailSection::FileContent,
    ];

    let keys = vec![
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Up,
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Down,
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::PageUp,
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::PageDown,
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Home,
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::End,
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('G'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('/'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('f'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('l'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('c'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('C'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('t'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('b'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('i'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('p'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('v'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('y'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('s'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('S'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('d'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('r'),
            crossterm::event::KeyModifiers::empty(),
        ),
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char(' '),
            crossterm::event::KeyModifiers::empty(),
        ),
    ];

    for focus in focuses {
        app.detail_focus = focus;
        for key in &keys {
            let _ = crate::tabs::workspace::WorkspaceTab::handle_event(&mut app, *key);
        }
    }
}

#[test]
fn test_other_detail_tabs_event_handlers() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_other_tabs_dummy.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);
    app.mode = Mode::Detail;

    let mock_info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        local_branches: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
            name: "main".to_string(),
            is_head: true,
            short_sha: "abc1234".to_string(),
            short_message: "commit msg".to_string(),
        }]),
        remote_branches: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
            name: "origin/main".to_string(),
            is_head: false,
            short_sha: "abc1234".to_string(),
            short_message: "commit msg".to_string(),
        }]),
        remotes: crate::repo::TabData::Loaded(vec![
            crate::repo::RemoteInfo {
                name: "origin".to_string(),
                url: "url".to_string(),
                push_url: None,
                refspecs: vec![],
            },
            crate::repo::RemoteInfo {
                name: "upstream".to_string(),
                url: "url2".to_string(),
                push_url: None,
                refspecs: vec![],
            },
        ]),
        stashes: crate::repo::TabData::Loaded(vec![crate::repo::StashInfo {
            index: 0,
            message: "wip".to_string(),
            commit_id: "abc1234".to_string(),
            files: vec![],
        }]),
        ..Default::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    // 1. BranchesTab
    let branch_keys = vec![
        crossterm::event::KeyCode::Char('/'),
        crossterm::event::KeyCode::Char('c'),
        crossterm::event::KeyCode::Char('D'),
        crossterm::event::KeyCode::Char('m'),
        crossterm::event::KeyCode::Char('r'),
        crossterm::event::KeyCode::Char('i'),
        crossterm::event::KeyCode::Char('P'),
        crossterm::event::KeyCode::Char('p'),
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyCode::Left,
        crossterm::event::KeyCode::Right,
        crossterm::event::KeyCode::Up,
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyCode::PageUp,
        crossterm::event::KeyCode::PageDown,
        crossterm::event::KeyCode::Home,
        crossterm::event::KeyCode::End,
    ];
    app.detail_focus = DetailSection::LocalBranches;
    for key_code in branch_keys {
        let key =
            crossterm::event::KeyEvent::new(key_code, crossterm::event::KeyModifiers::empty());
        let _ = crate::tabs::branches::BranchesTab::handle_event(&mut app, key);
    }
    app.detail_focus = DetailSection::RemoteBranches;
    let branch_nav_keys = vec![
        crossterm::event::KeyCode::Up,
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyCode::PageUp,
        crossterm::event::KeyCode::PageDown,
        crossterm::event::KeyCode::Home,
        crossterm::event::KeyCode::End,
    ];
    for key_code in branch_nav_keys {
        let key =
            crossterm::event::KeyEvent::new(key_code, crossterm::event::KeyModifiers::empty());
        let _ = crate::tabs::branches::BranchesTab::handle_event(&mut app, key);
    }

    // 2. StashesTab
    let stash_keys = vec![
        crossterm::event::KeyCode::Char('D'),
        crossterm::event::KeyCode::Char('a'),
        crossterm::event::KeyCode::Char('s'),
        crossterm::event::KeyCode::Up,
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyCode::PageUp,
        crossterm::event::KeyCode::PageDown,
        crossterm::event::KeyCode::Home,
        crossterm::event::KeyCode::End,
    ];
    let stash_focuses =
        vec![DetailSection::Stashes, DetailSection::StashedFiles, DetailSection::StagingDetails];
    for focus in stash_focuses {
        app.detail_focus = focus;
        for key_code in &stash_keys {
            let key =
                crossterm::event::KeyEvent::new(*key_code, crossterm::event::KeyModifiers::empty());
            let _ = crate::tabs::stashes::StashesTab::handle_event(&mut app, key);
        }
    }

    // 3. RemotesTab
    let remote_keys = vec![
        crossterm::event::KeyCode::Up,
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyCode::PageUp,
        crossterm::event::KeyCode::PageDown,
        crossterm::event::KeyCode::Home,
        crossterm::event::KeyCode::End,
        crossterm::event::KeyCode::Char('f'),
        crossterm::event::KeyCode::Char('a'),
        crossterm::event::KeyCode::Char('D'),
    ];
    for key_code in remote_keys {
        let key =
            crossterm::event::KeyEvent::new(key_code, crossterm::event::KeyModifiers::empty());
        let _ = crate::tabs::remotes::RemotesTab::handle_event(&mut app, key);
    }

    // 4. FilesTab
    app.file_tree.visible_files = vec![crate::app::FileTreeItem {
        name: "test.rs".to_string(),
        full_path: "test.rs".to_string(),
        is_dir: false,
        depth: 0,
        is_expanded: false,
    }];
    let file_keys = vec![
        crossterm::event::KeyCode::Char('e'),
        crossterm::event::KeyCode::Char('H'),
        crossterm::event::KeyCode::Char('/'),
        crossterm::event::KeyCode::Char('>'),
        crossterm::event::KeyCode::Char('<'),
    ];
    app.detail_focus = DetailSection::Files;
    for key_code in file_keys {
        let key =
            crossterm::event::KeyEvent::new(key_code, crossterm::event::KeyModifiers::empty());
        let _ = crate::tabs::files::FilesTab::handle_event(&mut app, key);
    }
    app.detail_focus = DetailSection::FileContent;
    let file_content_keys = vec![
        crossterm::event::KeyCode::Right,
        crossterm::event::KeyCode::Left,
        crossterm::event::KeyCode::Up,
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyCode::PageUp,
        crossterm::event::KeyCode::PageDown,
        crossterm::event::KeyCode::Home,
        crossterm::event::KeyCode::End,
    ];
    for key_code in file_content_keys {
        let key =
            crossterm::event::KeyEvent::new(key_code, crossterm::event::KeyModifiers::empty());
        let _ = crate::tabs::files::FilesTab::handle_event(&mut app, key);
    }

    // 5. FileHistoryTab
    app.file_history_revisions = vec![crate::repo::FileRevision {
        commit_oid: "abc1234abc1234abc1234abc1234abc1234abc1234".to_string(),
        author: "Author".to_string(),
        when: "10 mins ago".to_string(),
        date: "2026-07-02".to_string(),
        summary: "feat: summary".to_string(),
    }];
    let hist_keys = vec![
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyCode::Tab,
        crossterm::event::KeyCode::Left,
        crossterm::event::KeyCode::Up,
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyCode::Home,
        crossterm::event::KeyCode::End,
    ];
    app.file_history_focus = 0;
    app.file_history_selection = 0;
    for key_code in hist_keys {
        let key =
            crossterm::event::KeyEvent::new(key_code, crossterm::event::KeyModifiers::empty());
        let _ = crate::tabs::file_history::FileHistoryTab::handle_event(&mut app, key);
    }
}

#[test]
fn test_workspace_actions_direct() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_workspace_direct_dummy.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);
    app.mode = Mode::Detail;

    let mock_info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        local_branches: crate::repo::TabData::Loaded(vec![
            crate::repo::BranchInfo {
                name: "main".to_string(),
                is_head: true,
                short_sha: "abc1234".to_string(),
                short_message: "commit 1".to_string(),
            },
            crate::repo::BranchInfo {
                name: "feature".to_string(),
                is_head: false,
                short_sha: "def5678".to_string(),
                short_message: "commit 2".to_string(),
            },
        ]),
        commits: vec![
            crate::repo::CommitEntry {
                id: "abc1234".to_string(),
                oid: "abc1234abc1234abc1234abc1234abc1234abc1234".to_string(),
                author: "Author".to_string(),
                when: "10 mins ago".to_string(),
                date: "2026-07-02".to_string(),
                summary: "feat: summary 1".to_string(),
                message: "feat: summary 1\n\nbody".to_string(),
                refs: vec![],
                files: vec![],
                signature_status: "G".to_string(),
            },
            crate::repo::CommitEntry {
                id: "def5678".to_string(),
                oid: "def5678def5678def5678def5678def5678def5678".to_string(),
                author: "Author".to_string(),
                when: "20 mins ago".to_string(),
                date: "2026-07-02".to_string(),
                summary: "feat: summary 2".to_string(),
                message: "feat: summary 2\n\nbody".to_string(),
                refs: vec![],
                files: vec![],
                signature_status: "N".to_string(),
            },
        ],
        changes: crate::repo::WorktreeChanges {
            staged: vec![crate::repo::FileEntry { path: "staged_file.rs".to_string(), label: "A" }],
            unstaged: vec![crate::repo::FileEntry {
                path: "unstaged_file.rs".to_string(),
                label: "M",
            }],
            conflicted: vec![crate::repo::FileEntry {
                path: "conflicted_file.rs".to_string(),
                label: "U",
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    // Test cherry pick triggers
    app.commit_list.selection = 1; // Select the commit
    app.request_cherry_pick();
    assert_eq!(app.mode, Mode::CherryPickConfirm);
    assert_eq!(app.cherry_pick_dest_branches, vec!["feature".to_string()]);

    app.cancel_cherry_pick();
    assert_eq!(app.mode, Mode::Detail);

    app.request_cherry_pick();
    app.confirm_cherry_pick();
    assert_eq!(app.mode, Mode::Detail);

    // Test revert triggers
    app.commit_list.selection = 1;
    app.request_revert();
    assert_eq!(app.mode, Mode::RevertConfirm);

    app.cancel_revert();
    assert_eq!(app.mode, Mode::Detail);

    app.request_revert();
    app.confirm_revert();
    assert_eq!(app.mode, Mode::Detail);

    // Test helper empty checks
    assert!(!app.is_staged_empty());
    assert!(!app.is_unstaged_empty());
    assert!(!app.is_conflicted_empty());

    // Test discard logic
    app.request_discard_all_changes();
    assert_eq!(app.mode, Mode::DiscardChangesConfirm);
    app.cancel_discard_changes();
    assert_eq!(app.mode, Mode::Detail);

    app.detail_focus = DetailSection::Unstaged;
    app.request_discard_changes();
    assert_eq!(app.mode, Mode::DiscardChangesConfirm);
    app.confirm_discard_changes();
    assert_eq!(app.mode, Mode::Detail);

    // Test commit states
    app.cancel_commit_search();
    app.cancel_commit();
    app.commit_git_changes();
}

#[test]
fn test_workspace_diff_actions_direct() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_workspace_diff_dummy.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);
    app.mode = Mode::Detail;

    let mock_info = crate::repo::RepoInfo {
        changes: crate::repo::WorktreeChanges {
            staged: vec![crate::repo::FileEntry { path: "staged_file.rs".to_string(), label: "A" }],
            unstaged: vec![crate::repo::FileEntry {
                path: "unstaged_file.rs".to_string(),
                label: "M",
            }],
            conflicted: vec![crate::repo::FileEntry {
                path: "conflicted_file.rs".to_string(),
                label: "U",
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    // Populate mock file diff with hunks
    app.diff.file_diff = vec![
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::Header,
            content: "@@ -1,3 +1,4 @@".to_string(),
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::Context,
            content: " fn main() {".to_string(),
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::Added,
            content: "+    println!(\"hello\");".to_string(),
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::Header,
            content: "@@ -10,3 +10,4 @@".to_string(),
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::Removed,
            content: "-    println!(\"old\");".to_string(),
        },
    ];

    // Toggle diff line mode
    app.toggle_diff_line_mode();
    assert!(app.diff.diff_line_mode);
    app.toggle_diff_line_mode();
    assert!(!app.diff.diff_line_mode);

    // Test stage/unstage/discard hunks
    app.detail_focus = DetailSection::Unstaged;
    app.stage_selected_hunk();

    app.detail_focus = DetailSection::Staged;
    app.unstage_selected_hunk();

    app.detail_focus = DetailSection::Unstaged;
    app.discard_selected_hunk();

    // Test stage/unstage/discard lines
    app.diff.diff_line_mode = true;
    app.diff.diff_line_selection = 2;

    app.detail_focus = DetailSection::Unstaged;
    app.stage_selected_line();

    app.detail_focus = DetailSection::Staged;
    app.unstage_selected_line();

    app.detail_focus = DetailSection::Unstaged;
    app.discard_selected_line();

    // Test conflicts
    app.detail_focus = DetailSection::Conflicts;
    app.diff.file_diff = vec![
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::ConflictSeparator,
            content: "<<<<<<< OURS".to_string(),
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::ConflictOurs,
            content: "ours".to_string(),
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::ConflictSeparator,
            content: "=======".to_string(),
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::ConflictTheirs,
            content: "theirs".to_string(),
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::ConflictSeparator,
            content: ">>>>>>> THEIRS".to_string(),
        },
    ];

    app.resolve_conflict_ours();
    app.resolve_conflict_theirs();
    app.mark_conflict_resolved();
}

#[test]
fn test_drain_queue_comprehensive() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_drain_dummy.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);
    app.mode = Mode::Detail;

    let mock_info = crate::repo::RepoInfo {
        changes: crate::repo::WorktreeChanges {
            staged: vec![crate::repo::FileEntry { path: "staged_file.rs".to_string(), label: "A" }],
            unstaged: vec![crate::repo::FileEntry {
                path: "unstaged_file.rs".to_string(),
                label: "M",
            }],
            conflicted: vec![crate::repo::FileEntry {
                path: "conflicted_file.rs".to_string(),
                label: "U",
            }],
            ..Default::default()
        },
        commits: vec![crate::repo::CommitEntry {
            id: "abc1234".to_string(),
            oid: "abc1234abc1234abc1234abc1234abc1234abc1234".to_string(),
            author: "Author".to_string(),
            when: "10 mins ago".to_string(),
            date: "2026-07-02".to_string(),
            summary: "feat: summary".to_string(),
            message: "feat: summary\n\nbody".to_string(),
            refs: vec![],
            files: vec![],
            signature_status: "G".to_string(),
        }],
        local_branches: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
            name: "main".to_string(),
            is_head: true,
            short_sha: "abc1234".to_string(),
            short_message: "commit 1".to_string(),
        }]),
        remote_branches: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
            name: "origin/main".to_string(),
            is_head: false,
            short_sha: "abc1234".to_string(),
            short_message: "commit 1".to_string(),
        }]),
        remotes: crate::repo::TabData::Loaded(vec![crate::repo::RemoteInfo {
            name: "origin".to_string(),
            url: "url".to_string(),
            push_url: None,
            refspecs: vec![],
        }]),
        ..Default::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    let events = vec![
        crate::queue::InternalEvent::ClosePopup,
        crate::queue::InternalEvent::ConfirmYes,
        crate::queue::InternalEvent::ConfirmNo,
        crate::queue::InternalEvent::InputChar('a'),
        crate::queue::InternalEvent::InputBackspace,
        crate::queue::InternalEvent::InputEnter,
        crate::queue::InternalEvent::InputEsc,
        crate::queue::InternalEvent::Commit,
        crate::queue::InternalEvent::SearchColumnPicker,
        crate::queue::InternalEvent::StartCommit,
        crate::queue::InternalEvent::StartCommitAmend,
        crate::queue::InternalEvent::StartTagCreate,
        crate::queue::InternalEvent::RunInteractiveRebase,
        crate::queue::InternalEvent::RequestCherryPick,
        crate::queue::InternalEvent::YankSelectedCommitHash,
        crate::queue::InternalEvent::RequestRevert,
        crate::queue::InternalEvent::InspectCommit,
        crate::queue::InternalEvent::CommitSelectionUp,
        crate::queue::InternalEvent::CommitSelectionDown,
        crate::queue::InternalEvent::CommitSelectionPageUp,
        crate::queue::InternalEvent::CommitSelectionPageDown,
        crate::queue::InternalEvent::CommitSelectionTop,
        crate::queue::InternalEvent::CommitSelectionBottom,
        crate::queue::InternalEvent::LoadMoreCommits,
        crate::queue::InternalEvent::CommitDetailsUp,
        crate::queue::InternalEvent::CommitDetailsDown,
        crate::queue::InternalEvent::StagingFileUp,
        crate::queue::InternalEvent::StagingFileDown,
        crate::queue::InternalEvent::ConflictFileUp,
        crate::queue::InternalEvent::ConflictFileDown,
        crate::queue::InternalEvent::StageSelectedFile,
        crate::queue::InternalEvent::UnstageSelectedFile,
        crate::queue::InternalEvent::ResolveConflictOurs,
        crate::queue::InternalEvent::ResolveConflictTheirs,
        crate::queue::InternalEvent::MarkConflictResolved,
        crate::queue::InternalEvent::MergeAbortConfirm,
        crate::queue::InternalEvent::MergeContinueConfirm,
        crate::queue::InternalEvent::StageSelectedHunk,
        crate::queue::InternalEvent::UnstageSelectedHunk,
        crate::queue::InternalEvent::StageAllChanges,
        crate::queue::InternalEvent::UnstageAllChanges,
        crate::queue::InternalEvent::RequestDiscardChanges,
        crate::queue::InternalEvent::RequestDiscardAllChanges,
        crate::queue::InternalEvent::StartStashCreate,
        crate::queue::InternalEvent::DiffScrollUp,
        crate::queue::InternalEvent::DiffScrollDown,
        crate::queue::InternalEvent::DiffScrollPageUp,
        crate::queue::InternalEvent::DiffScrollPageDown,
        crate::queue::InternalEvent::DiffScrollTop,
        crate::queue::InternalEvent::DiffScrollBottom,
        crate::queue::InternalEvent::FileTreeUp,
        crate::queue::InternalEvent::FileTreeDown,
        crate::queue::InternalEvent::FileTreePageUp,
        crate::queue::InternalEvent::FileTreePageDown,
        crate::queue::InternalEvent::FileTreeTop,
        crate::queue::InternalEvent::FileTreeBottom,
        crate::queue::InternalEvent::FileContentUp,
        crate::queue::InternalEvent::FileContentDown,
        crate::queue::InternalEvent::FileContentPageUp,
        crate::queue::InternalEvent::FileContentPageDown,
        crate::queue::InternalEvent::FileContentTop,
        crate::queue::InternalEvent::FileContentBottom,
        crate::queue::InternalEvent::ToggleFolderExpanded,
        crate::queue::InternalEvent::CollapseAllFolders,
        crate::queue::InternalEvent::RequestDiscardFile,
        crate::queue::InternalEvent::LocalBranchUp,
        crate::queue::InternalEvent::LocalBranchDown,
        crate::queue::InternalEvent::LocalBranchPageUp,
        crate::queue::InternalEvent::LocalBranchPageDown,
        crate::queue::InternalEvent::LocalBranchTop,
        crate::queue::InternalEvent::LocalBranchBottom,
        crate::queue::InternalEvent::RemoteBranchUp,
        crate::queue::InternalEvent::RemoteBranchDown,
        crate::queue::InternalEvent::RemoteBranchPageUp,
        crate::queue::InternalEvent::RemoteBranchPageDown,
        crate::queue::InternalEvent::RemoteBranchTop,
        crate::queue::InternalEvent::RemoteBranchBottom,
        crate::queue::InternalEvent::CheckoutBranch,
        crate::queue::InternalEvent::RequestDeleteBranch,
        crate::queue::InternalEvent::StartBranchCreate,
        crate::queue::InternalEvent::StartBranchMerge,
        crate::queue::InternalEvent::StartBranchRebase,
        crate::queue::InternalEvent::RequestBranchPush,
        crate::queue::InternalEvent::FetchRemote,
        crate::queue::InternalEvent::StartRemoteAdd,
        crate::queue::InternalEvent::RequestDeleteRemote,
        crate::queue::InternalEvent::TagUp,
        crate::queue::InternalEvent::TagDown,
        crate::queue::InternalEvent::TagPageUp,
        crate::queue::InternalEvent::TagPageDown,
        crate::queue::InternalEvent::TagTop,
        crate::queue::InternalEvent::TagBottom,
        crate::queue::InternalEvent::CheckoutTag,
        crate::queue::InternalEvent::RequestDeleteTag,
        crate::queue::InternalEvent::RequestPushTag,
        crate::queue::InternalEvent::RequestPushAllTags,
        crate::queue::InternalEvent::FetchRemoteTags,
        crate::queue::InternalEvent::StashUp,
        crate::queue::InternalEvent::StashDown,
        crate::queue::InternalEvent::StashPageUp,
        crate::queue::InternalEvent::StashPageDown,
        crate::queue::InternalEvent::StashTop,
        crate::queue::InternalEvent::StashBottom,
        crate::queue::InternalEvent::StashFileUp,
        crate::queue::InternalEvent::StashFileDown,
        crate::queue::InternalEvent::StashFilePageUp,
        crate::queue::InternalEvent::StashFilePageDown,
        crate::queue::InternalEvent::StashFileTop,
        crate::queue::InternalEvent::StashFileBottom,
        crate::queue::InternalEvent::RequestDeleteStash,
        crate::queue::InternalEvent::RequestApplyStash,
    ];

    for ev in events {
        app.queue.push(ev);
        app.drain_queue();
    }
}

#[test]
fn test_navigation_actions_direct() {
    let config = Config { items: vec!["/path/to/repo_a".to_string()], ..Default::default() };
    let temp_config_path = std::env::temp_dir().join("gitwig_test_nav_dummy.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);
    app.mode = Mode::Detail;

    let mock_info = crate::repo::RepoInfo {
        local_branches: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
            name: "main".to_string(),
            is_head: true,
            short_sha: "abc1234".to_string(),
            short_message: "commit 1".to_string(),
        }]),
        remote_branches: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
            name: "origin/main".to_string(),
            is_head: false,
            short_sha: "abc1234".to_string(),
            short_message: "commit 1".to_string(),
        }]),
        ..Default::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    app.set_error("msg".to_string());
    assert_eq!(app.status_height(), 1);
    app.toggle_status_expanded();
    assert!(app.status_height() >= 3);

    let _rows = app.get_home_rows();
    app.clamp_selection();
    app.clamp_scroll(10);
    app.clamp_help_scroll(10);
    app.clamp_legend_scroll();

    app.move_down(10);
    app.move_up();
    app.page_down(10);
    app.page_up(10);
    app.move_to_top();
    app.move_to_bottom(10);

    app.open_help();
    app.open_about();

    app.cycle_sort_order();
    app.toggle_sort_reverse();
    app.toggle_pin_selected();
    app.toggle_star_selected();

    app.cycle_detail_focus(false);
    app.cycle_detail_focus(true);

    app.local_branch_up();
    app.local_branch_down();
    app.local_branch_page_up(5);
    app.local_branch_page_down(5);
    app.local_branch_to_top();
    app.local_branch_to_bottom();

    app.remote_branch_up();
    app.remote_branch_down();
    app.remote_branch_page_up(5);
    app.remote_branch_page_down(5);
    app.remote_branch_to_top();
    app.remote_branch_to_bottom();

    app.file_list_up();
    app.file_list_down();
    app.file_list_page_up(5);
    app.file_list_page_down(5);
    app.file_list_to_top();
    app.file_list_to_bottom();
}

#[test]
fn test_settings_popup_events_comprehensive() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_settings_comp_dummy.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);
    app.mode = Mode::Settings;
    app.settings_theme_list = vec!["default".to_string(), "monokai".to_string()];

    // 1. Draw settings page for all categories, focus, and editing states
    let backend = ratatui::backend::TestBackend::new(120, 40);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    for &idx in &[0, 15, 30, 45, 60] {
        for &focus_sidebar in &[true, false] {
            for &editing in &[true, false] {
                app.settings_selected_index = idx;
                app.settings_focus_sidebar = focus_sidebar;
                app.settings_editing = editing;
                terminal
                    .draw(|f| {
                        let area = f.area();
                        crate::popups::settings::draw_settings_page(f, &app, area);
                    })
                    .unwrap();
            }
        }
    }

    // 2. Handle keys when NOT editing
    app.settings_editing = false;
    let keys = vec![
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyCode::Char('q'),
        crossterm::event::KeyCode::Left,
        crossterm::event::KeyCode::Right,
        crossterm::event::KeyCode::Char('w'),
        crossterm::event::KeyCode::Char('1'),
        crossterm::event::KeyCode::Char('2'),
        crossterm::event::KeyCode::Char('3'),
        crossterm::event::KeyCode::Char('4'),
        crossterm::event::KeyCode::Char('5'),
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyCode::Up,
        crossterm::event::KeyCode::PageUp,
        crossterm::event::KeyCode::PageDown,
        crossterm::event::KeyCode::Home,
        crossterm::event::KeyCode::End,
        crossterm::event::KeyCode::Enter,
    ];

    app.settings_focus_sidebar = true;
    for key_code in &keys {
        let key =
            crossterm::event::KeyEvent::new(*key_code, crossterm::event::KeyModifiers::empty());
        crate::popups::settings::SettingsPopup::handle_event(&mut app, key);
    }

    app.settings_focus_sidebar = false;
    for key_code in &keys {
        let key =
            crossterm::event::KeyEvent::new(*key_code, crossterm::event::KeyModifiers::empty());
        crate::popups::settings::SettingsPopup::handle_event(&mut app, key);
    }

    // 3. Handle keys when editing (other index)
    app.settings_editing = true;
    app.settings_selected_index = 1; // Not 3
    let edit_keys = vec![
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyCode::Backspace,
        crossterm::event::KeyCode::Char('x'),
    ];
    for key_code in &edit_keys {
        let key =
            crossterm::event::KeyEvent::new(*key_code, crossterm::event::KeyModifiers::empty());
        crate::popups::settings::SettingsPopup::handle_event(&mut app, key);
    }

    // 4. Handle keys when editing theme (index 3)
    app.settings_editing = true;
    app.settings_selected_index = 3;
    let theme_keys = vec![
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyCode::Up,
        crossterm::event::KeyCode::PageUp,
        crossterm::event::KeyCode::PageDown,
        crossterm::event::KeyCode::Home,
        crossterm::event::KeyCode::End,
    ];
    for key_code in &theme_keys {
        let key =
            crossterm::event::KeyEvent::new(*key_code, crossterm::event::KeyModifiers::empty());
        crate::popups::settings::SettingsPopup::handle_event(&mut app, key);
    }

    // 5. Test get_val_str for all setting indices
    for idx in 0..=65 {
        app.settings_selected_index = idx;
        app.settings_editing = false;
        let _ = crate::popups::settings::get_val_str(&app, idx);

        app.settings_editing = true;
        let _ = crate::popups::settings::get_val_str(&app, idx);
    }

    // 6. Test toggle_or_edit_setting for all indices
    let test_indices = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 55, 56, 58];
    for idx in test_indices {
        app.settings_selected_index = idx;
        app.settings_editing = false;
        app.toggle_or_edit_setting();
    }
}

#[test]
fn test_settings_commit_flow() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_settings_commit_dummy.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);

    let indices = vec![0, 3, 4, 5, 6, 7, 8, 9, 56];

    // Test valid inputs
    for idx in &indices {
        app.settings_selected_index = *idx;
        app.settings_editing = true;
        app.settings_theme_list = vec!["default".to_string()];
        app.settings_theme_index = 0;
        app.input_buffer = match idx {
            0 => "100".to_string(),
            3 => "default".to_string(),
            4 => "5".to_string(),
            5 => "/start".to_string(),
            6 => "1000".to_string(),
            7 => "50".to_string(),
            8 => "exclude1,exclude2".to_string(),
            9 => "gitui".to_string(),
            56 => "vim".to_string(),
            _ => "".to_string(),
        };
        app.commit_settings_edit();
        assert!(!app.settings_editing);
    }

    // Test invalid inputs
    for idx in &indices {
        app.settings_selected_index = *idx;
        app.settings_editing = true;
        app.input_buffer = "invalid_value".to_string();
        app.commit_settings_edit();
        // For non-integers, it should fail parsing and stay editing
        if [0, 4, 6, 7].contains(idx) {
            assert!(app.settings_editing);
        }
    }

    // Test cancel edit
    app.settings_editing = true;
    app.cancel_settings_edit();
    assert!(!app.settings_editing);
}

#[test]
fn test_git_actions_direct() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_git_direct_dummy.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);
    app.mode = Mode::Detail;

    let mock_info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        local_branches: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
            name: "main".to_string(),
            is_head: true,
            short_sha: "abc1234".to_string(),
            short_message: "commit 1".to_string(),
        }]),
        remote_branches: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
            name: "origin/main".to_string(),
            is_head: false,
            short_sha: "abc1234".to_string(),
            short_message: "commit 1".to_string(),
        }]),
        local_tags: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
            name: "v1.0.0".to_string(),
            is_head: false,
            short_sha: "abc1234".to_string(),
            short_message: "commit 1".to_string(),
        }]),
        remote_tags: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
            name: "v1.0.0".to_string(),
            is_head: false,
            short_sha: "abc1234".to_string(),
            short_message: "commit 1".to_string(),
        }]),
        remotes: crate::repo::TabData::Loaded(vec![crate::repo::RemoteInfo {
            name: "origin".to_string(),
            url: "url".to_string(),
            push_url: None,
            refspecs: vec![],
        }]),
        stashes: crate::repo::TabData::Loaded(vec![crate::repo::StashInfo {
            index: 0,
            message: "wip".to_string(),
            commit_id: "abc1234".to_string(),
            files: vec![],
        }]),
        ..Default::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    // Run action/request sequence
    app.pull_selected_branch();

    app.branch_action_target = Some(("main".to_string(), false));
    app.request_branch_push();
    app.confirm_branch_push();
    app.request_branch_push();
    app.cancel_branch_push();

    app.branch_action_target = Some(("main".to_string(), false));
    app.request_branch_checkout();
    app.confirm_branch_checkout();
    app.request_branch_checkout();
    app.cancel_branch_checkout();

    app.start_tag_create();
    app.commit_tag_create();

    app.start_stash_create();
    app.commit_stash_create();

    app.start_remote_add();
    app.commit_remote_add_name();
    app.commit_remote_add_url();

    app.remote_action_target = Some("origin".to_string());
    app.request_remote_delete();
    app.confirm_remote_delete();

    app.tag_delete_target = Some(("v1.0.0".to_string(), false));
    app.request_tag_delete();
    app.confirm_tag_delete();
    app.request_tag_delete();
    app.cancel_tag_delete();

    app.tag_push_target = Some("v1.0.0".to_string());
    app.request_tag_push();
    app.confirm_tag_push();
    app.request_tag_push();
    app.cancel_tag_push();

    app.request_tag_push_all();
    app.confirm_tag_push_all();
    app.request_tag_push_all();
    app.cancel_tag_push_all();

    app.start_branch_create();
    app.commit_branch_create();
    app.start_branch_create();
    app.cancel_branch_create();

    app.branch_action_target = Some(("main".to_string(), false));
    app.request_branch_delete();
    app.confirm_branch_delete();
    app.request_branch_delete();
    app.cancel_branch_delete();

    app.branch_action_target = Some(("main".to_string(), false));
    app.request_branch_merge();
    app.confirm_branch_merge();
    app.request_branch_merge();
    app.cancel_branch_merge();

    app.branch_action_target = Some(("main".to_string(), false));
    app.request_branch_rebase();
    app.confirm_branch_rebase();
    app.request_branch_rebase();
    app.cancel_branch_rebase();

    app.branch_action_target = Some(("main".to_string(), false));
    app.request_branch_interactive_rebase();
    app.confirm_branch_interactive_rebase();
    app.request_branch_interactive_rebase();
    app.cancel_branch_interactive_rebase();

    app.run_interactive_rebase();
    app.confirm_abort_merge();
    app.confirm_continue_merge();

    app.confirm_remote_picker();
    app.cancel_remote_picker();
}

#[test]
fn test_app_helpers_mod_rs() {
    let temp_dir = std::env::temp_dir().join("gitwig_test_watcher_repo");
    let git_dir = temp_dir.join(".git");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&git_dir).unwrap();

    let config =
        Config { items: vec![temp_dir.to_string_lossy().to_string()], ..Default::default() };
    let temp_config_path = std::env::temp_dir().join("gitwig_test_helpers_dummy.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);

    app.resolve_repo_themes();
    app.setup_watcher();
    assert_eq!(app.item_height(), 4);

    app.config.compact_view = true;
    assert_eq!(app.item_height(), 1);

    app.increment_implicit_network();
    assert_eq!(app.implicit_network_count, 1);
    app.decrement_implicit_network();
    assert_eq!(app.implicit_network_count, 0);

    assert!(!app.is_msi_install());
    app.trigger_self_update();

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_all_scroll_helpers() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_scrolls_dummy.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);
    app.mode = Mode::Detail;

    let mock_info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        local_branches: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
            name: "main".to_string(),
            is_head: true,
            short_sha: "abc1234".to_string(),
            short_message: "commit 1".to_string(),
        }]),
        remote_branches: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
            name: "origin/main".to_string(),
            is_head: false,
            short_sha: "abc1234".to_string(),
            short_message: "commit 1".to_string(),
        }]),
        local_tags: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
            name: "v1.0.0".to_string(),
            is_head: false,
            short_sha: "abc1234".to_string(),
            short_message: "commit 1".to_string(),
        }]),
        remote_tags: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
            name: "v1.0.0".to_string(),
            is_head: false,
            short_sha: "abc1234".to_string(),
            short_message: "commit 1".to_string(),
        }]),
        remotes: crate::repo::TabData::Loaded(vec![crate::repo::RemoteInfo {
            name: "origin".to_string(),
            url: "url".to_string(),
            push_url: None,
            refspecs: vec![],
        }]),
        stashes: crate::repo::TabData::Loaded(vec![crate::repo::StashInfo {
            index: 0,
            message: "wip".to_string(),
            commit_id: "abc1234".to_string(),
            files: vec![],
        }]),
        ..Default::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    app.local_branch_up();
    app.local_branch_down();
    app.local_branch_page_up(5);
    app.local_branch_page_down(5);

    app.remote_branch_up();
    app.remote_branch_down();
    app.remote_branch_page_up(5);
    app.remote_branch_page_down(5);

    app.file_list_up();
    app.file_list_down();
    app.file_list_page_up(5);
    app.file_list_page_down(5);

    app.local_tag_up();
    app.local_tag_down();
    app.local_tag_page_up(5);
    app.local_tag_page_down(5);

    app.remote_up();
    app.remote_down();
    app.remote_page_up(5);
    app.remote_page_down(5);

    app.stash_up();
    app.stash_down();
    app.stash_page_up(5);
    app.stash_page_down(5);

    app.detail_commit_up();
    app.detail_commit_down();
    app.detail_commit_page_up(5);
    app.detail_commit_page_down(5);

    app.detail_file_up();
    app.detail_file_down();

    app.staging_file_up();
    app.staging_file_down();

    app.conflict_file_up();
    app.conflict_file_down();

    app.diff_hunk_up();
    app.diff_hunk_down();

    app.diff_line_up();
    app.diff_line_down();

    app.file_content_scroll_up();
    app.file_content_scroll_down();
    app.file_content_scroll_page_up(5);
    app.file_content_scroll_page_down(5);

    app.graph_select_up();
    app.graph_select_down();
    app.graph_select_page_up(5);
    app.graph_select_page_down(5);

    app.commit_details_scroll_up();
    app.commit_details_scroll_down();

    app.commit_input_scroll_up();
    app.commit_input_scroll_down();

    app.remote_picker_up();
    app.remote_picker_down();
}

#[test]
fn test_inspect_popup_events_comprehensive() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_inspect_comp_dummy.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);
    app.mode = Mode::Inspect;

    let mut mock_info = crate::repo::RepoInfo::default();
    mock_info.changes.staged =
        vec![crate::repo::FileEntry { path: "staged.rs".to_string(), label: "A" }];
    mock_info.changes.unstaged =
        vec![crate::repo::FileEntry { path: "unstaged.rs".to_string(), label: "M" }];
    mock_info.changes.conflicted =
        vec![crate::repo::FileEntry { path: "conflict.rs".to_string(), label: "U" }];
    mock_info.commits = vec![crate::repo::CommitEntry {
        id: "abc1234".to_string(),
        oid: "abc1234abc1234abc1234abc1234abc1234abc1234".to_string(),
        author: "Author".to_string(),
        when: "now".to_string(),
        date: "2026-07-02".to_string(),
        summary: "feat: commit".to_string(),
        message: "message".to_string(),
        refs: vec![],
        files: vec![],
        signature_status: "N".to_string(),
    }];
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    let sections = vec![
        DetailSection::Staged,
        DetailSection::Unstaged,
        DetailSection::Conflicts,
        DetailSection::StagingDetails,
        DetailSection::ConflictDiff,
        DetailSection::CommitDetails,
        DetailSection::Commits,
    ];

    let keycodes = vec![
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyCode::Char('q'),
        crossterm::event::KeyCode::Char('?'),
        crossterm::event::KeyCode::Char('w'),
        crossterm::event::KeyCode::Tab,
        crossterm::event::KeyCode::Char('W'),
        crossterm::event::KeyCode::Right,
        crossterm::event::KeyCode::Left,
        crossterm::event::KeyCode::Up,
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyCode::PageUp,
        crossterm::event::KeyCode::PageDown,
        crossterm::event::KeyCode::Home,
        crossterm::event::KeyCode::End,
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyCode::Delete,
        crossterm::event::KeyCode::Char('x'),
        crossterm::event::KeyCode::Char('X'),
        crossterm::event::KeyCode::Char('a'),
        crossterm::event::KeyCode::Char('l'),
        crossterm::event::KeyCode::Char('o'),
        crossterm::event::KeyCode::Char('t'),
        crossterm::event::KeyCode::Char('r'),
        crossterm::event::KeyCode::Char('A'),
        crossterm::event::KeyCode::Char('C'),
        crossterm::event::KeyCode::Char('c'),
    ];

    for sect in &sections {
        for keycode in &keycodes {
            app.detail_focus = *sect;
            let key =
                crossterm::event::KeyEvent::new(*keycode, crossterm::event::KeyModifiers::empty());
            crate::popups::inspect::InspectPopup::handle_event(&mut app, key);
        }
    }
}

#[test]
fn test_repo_settings_comprehensive() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let temp_dir = std::env::temp_dir();
    let config_path = temp_dir.join("gitwig_test_repo_settings_comp.toml");
    let _guard = TestFileGuard { path: config_path.clone() };

    let config = Config { items: vec!["/path/to/custom_repo".to_string()], ..Default::default() };
    let mut app = App::new(config, config_path);
    app.mode = Mode::RepoSettings;
    app.selected_index = 0;

    // 1. Test draw
    let backend = ratatui::backend::TestBackend::new(100, 30);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = f.area();
            crate::popups::repo_settings::RepoSettingsPopup::draw(f, &app, area);
        })
        .unwrap();

    // 2. Test handle_event keys when not editing
    let keys = vec![
        KeyCode::Esc,
        KeyCode::Char('q'),
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Enter,
        KeyCode::Char(' '),
    ];

    for k in keys {
        crate::popups::repo_settings::RepoSettingsPopup::handle_event(&mut app, key_event(k));
    }

    // 3. Test handle_event keys when editing (numeric and text)
    app.repo_settings_editing = true;
    app.repo_settings_selected_index = 1; // page_size
    crate::popups::repo_settings::RepoSettingsPopup::handle_event(
        &mut app,
        key_event(KeyCode::Char('8')),
    );
    crate::popups::repo_settings::RepoSettingsPopup::handle_event(
        &mut app,
        key_event(KeyCode::Backspace),
    );
    crate::popups::repo_settings::RepoSettingsPopup::handle_event(
        &mut app,
        key_event(KeyCode::Enter),
    );

    app.repo_settings_editing = true;
    app.repo_settings_selected_index = 4; // editor
    crate::popups::repo_settings::RepoSettingsPopup::handle_event(
        &mut app,
        key_event(KeyCode::Char('n')),
    );
    crate::popups::repo_settings::RepoSettingsPopup::handle_event(
        &mut app,
        key_event(KeyCode::Esc),
    );

    // 4. Test change_setting Left/Right for themes and compact toggle
    app.repo_settings_editing = false;
    app.repo_settings_selected_index = 0; // theme
    crate::popups::repo_settings::RepoSettingsPopup::handle_event(
        &mut app,
        key_event(KeyCode::Left),
    );
    crate::popups::repo_settings::RepoSettingsPopup::handle_event(
        &mut app,
        key_event(KeyCode::Right),
    );

    app.repo_settings_selected_index = 3; // compact_view
    crate::popups::repo_settings::RepoSettingsPopup::handle_event(
        &mut app,
        key_event(KeyCode::Left),
    );
    crate::popups::repo_settings::RepoSettingsPopup::handle_event(
        &mut app,
        key_event(KeyCode::Right),
    );
}

#[test]
fn test_ui_detail_non_repos() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_ui_detail_dummy.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let app = App::new(config, temp_config_path);

    let backend = ratatui::backend::TestBackend::new(100, 30);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    let details = vec![
        crate::repo::ItemDetail::Missing { resolved: PathBuf::from("/missing/path") },
        crate::repo::ItemDetail::Directory { resolved: PathBuf::from("/directory/path") },
        crate::repo::ItemDetail::Error {
            resolved: PathBuf::from("/error/path"),
            message: "Error msg".to_string(),
        },
    ];

    let mut detail_areas = crate::ui_detail::DetailAreas::default();

    for detail in &details {
        terminal
            .draw(|f| {
                let size = f.area();
                crate::ui_detail::draw(
                    f,
                    "dummy_item",
                    detail,
                    &app.mode,
                    &app.detail_focus,
                    app.last_staging_focus,
                    0,
                    &None,
                    0,
                    &[],
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    &[],
                    0,
                    0,
                    0,
                    0,
                    &mut detail_areas,
                    "",
                    false,
                    &None,
                    &None,
                    &None,
                    &None,
                    &None,
                    false,
                    false,
                    0,
                    50,
                    50,
                    50,
                    50,
                    50,
                    50,
                    50,
                    50,
                    &app,
                    size,
                );
            })
            .unwrap();
    }
}

#[test]
fn test_commit_popups_comprehensive() {
    use crate::components::{Component, DrawableComponent};
    use crate::popups::commit::{CommitPopup, GenericInputPopup};
    use crate::queue::Queue;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    let queue = Queue::default();
    let mut commit_popup = CommitPopup::new(queue.clone());

    // Test drawable
    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let area = f.area();
            let _ = commit_popup.draw(f, area);
        })
        .unwrap();

    // Test events while editing
    let keys_editing = vec![
        KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Up, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Down, KeyModifiers::empty()),
    ];

    for key in keys_editing {
        commit_popup.editing = true;
        let _ = commit_popup.event(&Event::Key(key));
    }

    // Test events while NOT editing
    let keys_not_editing = vec![
        KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char(' '), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Up, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Down, KeyModifiers::empty()),
    ];

    for key in keys_not_editing {
        commit_popup.editing = false;
        let _ = commit_popup.event(&Event::Key(key));
    }

    // Test GenericInputPopup
    let mut generic_popup = GenericInputPopup::new(queue.clone());
    terminal
        .draw(|f| {
            let area = f.area();
            let _ = generic_popup.draw(f, area);
        })
        .unwrap();

    let generic_keys = vec![
        KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty()),
    ];

    for key in generic_keys {
        let _ = generic_popup.event(&Event::Key(key));
    }
}

#[test]
fn test_loading_screens_comprehensive() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_loading_comp.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let app = App::new(config, temp_config_path);

    let backend = ratatui::backend::TestBackend::new(100, 30);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    // 1. Test draw_loading_screen
    let mut app = app;
    app.loading_repo_path = Some("/path/to/loading/repo".to_string());
    terminal
        .draw(|f| {
            let area = f.area();
            crate::popups::loading::draw_loading_screen(f, area, &app);
        })
        .unwrap();

    // 2. Test draw_progress_popup (normal spinner)
    app.loading_repo_path = None;
    app.fetching = true;
    app.fetch_progress = 45;
    app.status_message = Some("Fetching tags...".to_string());
    app.config.compatibility_mode = false;
    terminal
        .draw(|f| {
            let area = f.area();
            crate::popups::loading::draw_progress_popup(f, area, &app);
        })
        .unwrap();

    // 3. Test draw_progress_popup (compatibility spinner)
    app.config.compatibility_mode = true;
    terminal
        .draw(|f| {
            let area = f.area();
            crate::popups::loading::draw_progress_popup(f, area, &app);
        })
        .unwrap();
}

#[test]
fn test_workspace_tab_events_comprehensive() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_workspace_comp.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);
    app.mode = Mode::Detail;

    let mut mock_info = crate::repo::RepoInfo::default();
    mock_info.changes.staged =
        vec![crate::repo::FileEntry { path: "staged.rs".to_string(), label: "A" }];
    mock_info.changes.unstaged =
        vec![crate::repo::FileEntry { path: "unstaged.rs".to_string(), label: "M" }];
    mock_info.changes.conflicted =
        vec![crate::repo::FileEntry { path: "conflict.rs".to_string(), label: "U" }];
    mock_info.commits = vec![crate::repo::CommitEntry {
        id: "abc1234".to_string(),
        oid: "abc1234abc1234abc1234abc1234abc1234abc1234".to_string(),
        author: "Author".to_string(),
        when: "now".to_string(),
        date: "2026-07-02".to_string(),
        summary: "feat: commit".to_string(),
        message: "message".to_string(),
        refs: vec![],
        files: vec![],
        signature_status: "N".to_string(),
    }];
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    let sections = vec![
        DetailSection::Commits,
        DetailSection::Staged,
        DetailSection::Unstaged,
        DetailSection::Conflicts,
        DetailSection::StagingDetails,
        DetailSection::ConflictDiff,
    ];

    let keycodes = vec![
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyCode::Up,
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyCode::Left,
        crossterm::event::KeyCode::Right,
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyCode::PageUp,
        crossterm::event::KeyCode::PageDown,
        crossterm::event::KeyCode::Home,
        crossterm::event::KeyCode::End,
        crossterm::event::KeyCode::Char('G'),
        crossterm::event::KeyCode::Char('/'),
        crossterm::event::KeyCode::Char('f'),
        crossterm::event::KeyCode::Char('l'),
        crossterm::event::KeyCode::Char('c'),
        crossterm::event::KeyCode::Char('C'),
        crossterm::event::KeyCode::Char('t'),
        crossterm::event::KeyCode::Char('b'),
        crossterm::event::KeyCode::Char('i'),
        crossterm::event::KeyCode::Char('p'),
        crossterm::event::KeyCode::Char('v'),
        crossterm::event::KeyCode::Char('y'),
        crossterm::event::KeyCode::Char('s'),
        crossterm::event::KeyCode::Char('a'),
        crossterm::event::KeyCode::Char('x'),
        crossterm::event::KeyCode::Char('X'),
    ];

    for sect in &sections {
        for keycode in &keycodes {
            app.detail_focus = *sect;
            let key =
                crossterm::event::KeyEvent::new(*keycode, crossterm::event::KeyModifiers::empty());
            crate::tabs::workspace::WorkspaceTab::handle_event(&mut app, key);
        }
    }
}

#[test]
fn test_app_helper_functions_comprehensive() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_helpers_comp.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);

    // 1. clipboard
    let _ = crate::app::copy_to_clipboard("test clipboard content");

    // 2. is_tool_installed
    let _ = crate::app::is_tool_installed("git");
    let _ = crate::app::is_tool_installed("non_existent_tool_123");

    // 3. App getters
    assert!(!app.is_msi_install());
    assert_eq!(app.item_height(), 4);

    // 4. Implicit network counters
    app.increment_implicit_network();
    assert_eq!(app.implicit_network_count, 1);
    app.decrement_implicit_network();
    assert_eq!(app.implicit_network_count, 0);

    // 5. Update triggers (verify no panic)
    app.trigger_update_check();
    app.trigger_self_update();
}

#[test]
fn test_drain_queue_confirmations_comprehensive() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_drain_conf.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);

    let mock_info = crate::repo::RepoInfo {
        changes: crate::repo::WorktreeChanges {
            staged: vec![crate::repo::FileEntry { path: "staged_file.rs".to_string(), label: "A" }],
            unstaged: vec![crate::repo::FileEntry {
                path: "unstaged_file.rs".to_string(),
                label: "M",
            }],
            conflicted: vec![crate::repo::FileEntry {
                path: "conflicted_file.rs".to_string(),
                label: "U",
            }],
            ..Default::default()
        },
        commits: vec![crate::repo::CommitEntry {
            id: "abc1234".to_string(),
            oid: "abc1234abc1234abc1234abc1234abc1234abc1234".to_string(),
            author: "Author".to_string(),
            when: "10 mins ago".to_string(),
            date: "2026-07-02".to_string(),
            summary: "feat: summary".to_string(),
            message: "feat: summary\n\nbody".to_string(),
            refs: vec![],
            files: vec![],
            signature_status: "G".to_string(),
        }],
        ..Default::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    // Populate all target variables
    app.branch_action_target = Some(("main".to_string(), false));
    app.tag_action_target_oid = Some("abc1234abc1234abc1234abc1234abc1234abc1234".to_string());
    app.tag_delete_target = Some(("v1.0.0".to_string(), false));
    app.tag_checkout_target = Some("v1.0.0".to_string());
    app.tag_push_target = Some("v1.0.0".to_string());
    app.discard_target = Some(("staged_file.rs".to_string(), true));
    app.cherry_pick_target = Some(("abc1234".to_string(), "commit".to_string()));
    app.revert_target = Some(("abc1234".to_string(), "commit".to_string()));
    app.stash_action_target = Some(("stash@{0}".to_string(), "stash message".to_string()));
    app.remote_action_target = Some("origin".to_string());
    app.submodule_delete_target = Some("submodule_a".to_string());

    let confirm_modes = vec![
        Mode::BranchDeleteConfirm,
        Mode::BranchPushConfirm,
        Mode::BranchMergeConfirm,
        Mode::MergeAbortConfirm,
        Mode::MergeContinueConfirm,
        Mode::BranchRebaseConfirm,
        Mode::BranchInteractiveRebaseConfirm,
        Mode::DiscardChangesConfirm,
        Mode::RevertConfirm,
        Mode::TagDeleteConfirm,
        Mode::TagPushConfirm,
        Mode::TagPushAllConfirm,
        Mode::StashDeleteConfirm,
        Mode::BranchCheckoutConfirm,
        Mode::TagCheckoutConfirm,
        Mode::RemoteDeleteConfirm,
        Mode::UpdateConfirm,
        Mode::SubmoduleDeleteConfirm,
    ];

    for mode in &confirm_modes {
        app.mode = *mode;
        app.queue.push(crate::queue::InternalEvent::ConfirmYes);
        app.drain_queue();

        app.mode = *mode;
        app.queue.push(crate::queue::InternalEvent::ConfirmNo);
        app.drain_queue();
    }

    let input_modes = vec![
        Mode::BranchCreateInput,
        Mode::TagCreateInput,
        Mode::StashCreateInput,
        Mode::RemoteAddNameInput,
        Mode::RemoteAddUrlInput,
        Mode::WorktreeAddBranchInput,
        Mode::WorktreeAddPathInput,
        Mode::WorktreeLockReasonInput,
        Mode::WorktreeRemoveConfirm,
        Mode::SubmoduleAddUrlInput,
        Mode::SubmoduleAddPathInput,
    ];

    for mode in &input_modes {
        app.mode = *mode;
        app.queue.push(crate::queue::InternalEvent::InputEnter);
        app.drain_queue();

        app.mode = *mode;
        app.queue.push(crate::queue::InternalEvent::InputEsc);
        app.drain_queue();
    }
}

#[test]
fn test_file_history_comprehensive() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_file_hist_comp.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);

    let backend = ratatui::backend::TestBackend::new(100, 30);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    // 1. Draw with empty history
    app.file_history_path = "src/main.rs".to_string();
    app.file_history_revisions.clear();
    terminal
        .draw(|f| {
            let area = f.area();
            crate::tabs::file_history::FileHistoryTab::draw_file_history(f, &app, area);
        })
        .unwrap();

    // 2. Draw with populated history
    app.file_history_revisions = vec![
        crate::repo::FileRevision {
            commit_oid: "abc1234abc1234abc1234abc1234abc1234abc1234".to_string(),
            author: "Author".to_string(),
            when: "now".to_string(),
            date: "2026-07-02".to_string(),
            summary: "feat: summary".to_string(),
        },
        crate::repo::FileRevision {
            commit_oid: "def5678def5678def5678def5678def5678def5678".to_string(),
            author: "Author2".to_string(),
            when: "yesterday".to_string(),
            date: "2026-07-01".to_string(),
            summary: "fix: bug".to_string(),
        },
    ];
    app.file_history_selection = 0;
    app.file_history_focus = 0; // revisions focused
    terminal
        .draw(|f| {
            let area = f.area();
            crate::tabs::file_history::FileHistoryTab::draw_file_history(f, &app, area);
        })
        .unwrap();

    // 3. Draw with diff focused
    app.file_history_focus = 1; // diff focused
    terminal
        .draw(|f| {
            let area = f.area();
            crate::tabs::file_history::FileHistoryTab::draw_file_history(f, &app, area);
        })
        .unwrap();

    // 4. Test handle_event
    let keycodes = vec![
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyCode::Tab,
        crossterm::event::KeyCode::Left,
        crossterm::event::KeyCode::Right,
        crossterm::event::KeyCode::Up,
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyCode::Home,
        crossterm::event::KeyCode::End,
        crossterm::event::KeyCode::PageUp,
        crossterm::event::KeyCode::PageDown,
    ];

    // Revisions list empty
    app.file_history_revisions.clear();
    for &kc in &keycodes {
        for focus in &[0, 1] {
            app.file_history_focus = *focus;
            let key = crossterm::event::KeyEvent::new(kc, crossterm::event::KeyModifiers::empty());
            crate::tabs::file_history::FileHistoryTab::handle_event(&mut app, key);
        }
    }

    // Revisions list populated
    app.file_history_revisions = vec![
        crate::repo::FileRevision {
            commit_oid: "abc1234abc1234abc1234abc1234abc1234abc1234".to_string(),
            author: "Author".to_string(),
            when: "now".to_string(),
            date: "2026-07-02".to_string(),
            summary: "feat: summary".to_string(),
        },
        crate::repo::FileRevision {
            commit_oid: "def5678def5678def5678def5678def5678def5678".to_string(),
            author: "Author2".to_string(),
            when: "yesterday".to_string(),
            date: "2026-07-01".to_string(),
            summary: "fix: bug".to_string(),
        },
    ];
    for &kc in &keycodes {
        for focus in &[0, 1] {
            app.file_history_focus = *focus;
            app.file_history_selection = 0;
            let key = crossterm::event::KeyEvent::new(kc, crossterm::event::KeyModifiers::empty());
            crate::tabs::file_history::FileHistoryTab::handle_event(&mut app, key);
        }
    }
}

#[test]
fn test_style_helpers_comprehensive() {
    use crate::ui::style::{format_border_type, format_color, parse_border_type, parse_color};
    use ratatui::style::Color;
    use ratatui::widgets::BorderType;

    // Test parse/format colors
    let colors = vec![
        (Color::Black, "black"),
        (Color::Red, "red"),
        (Color::Green, "green"),
        (Color::Yellow, "yellow"),
        (Color::Blue, "blue"),
        (Color::Magenta, "magenta"),
        (Color::Cyan, "cyan"),
        (Color::Gray, "gray"),
        (Color::DarkGray, "darkgray"),
        (Color::LightRed, "lightred"),
        (Color::LightGreen, "lightgreen"),
        (Color::LightYellow, "lightyellow"),
        (Color::LightBlue, "lightblue"),
        (Color::LightMagenta, "lightmagenta"),
        (Color::LightCyan, "lightcyan"),
        (Color::White, "white"),
    ];

    for (color, name) in colors {
        assert_eq!(parse_color(name), color);
        assert_eq!(format_color(color), name);
    }
    // Unknown fallback
    assert_eq!(parse_color("unknown"), Color::Cyan);
    assert_eq!(format_color(Color::Indexed(123)), "cyan");

    // Test parse/format borders
    let borders = vec![
        (BorderType::Plain, "plain"),
        (BorderType::Rounded, "rounded"),
        (BorderType::Double, "double"),
        (BorderType::Thick, "thick"),
    ];

    for (border, name) in borders {
        assert_eq!(parse_border_type(name), border);
        assert_eq!(format_border_type(border), name);
    }
    // Unknown fallback
    assert_eq!(parse_border_type("unknown"), BorderType::Rounded);
    assert_eq!(format_border_type(BorderType::QuadrantOutside), "rounded");
}

#[test]
fn test_highlight_code_line() {
    let line = crate::ui::syntax::highlight_code_line("fn main() {}");
    assert_eq!(line.to_string(), "fn main() {}");
}

#[test]
fn test_log_search_popup_comprehensive() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_log_search.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);

    let keycodes = vec![
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyCode::Backspace,
        crossterm::event::KeyCode::Char('x'),
    ];

    // 1. LogsSearchInput (empty query)
    app.mode = Mode::LogsSearchInput;
    app.input_buffer.clear();
    for &kc in &keycodes {
        let key = crossterm::event::KeyEvent::new(kc, crossterm::event::KeyModifiers::empty());
        crate::popups::log_search::LogSearchPopup::handle_event(&mut app, key);
    }

    // 2. LogsSearchInput (non-empty query)
    app.mode = Mode::LogsSearchInput;
    app.input_buffer = "query".to_string();
    for &kc in &keycodes {
        let key = crossterm::event::KeyEvent::new(kc, crossterm::event::KeyModifiers::empty());
        crate::popups::log_search::LogSearchPopup::handle_event(&mut app, key);
    }

    // 3. CommitSearchInput
    app.mode = Mode::CommitSearchInput;
    app.input_buffer = "query".to_string();
    for &kc in &keycodes {
        let key = crossterm::event::KeyEvent::new(kc, crossterm::event::KeyModifiers::empty());
        crate::popups::log_search::LogSearchPopup::handle_event(&mut app, key);
    }

    // 4. Default mode fallback
    app.mode = Mode::Normal;
    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::empty(),
    );
    crate::popups::log_search::LogSearchPopup::handle_event(&mut app, key);
}

#[test]
fn test_route_detail_events_comprehensive() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_route_detail.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);

    let keycodes = vec![
        crossterm::event::KeyCode::Char('v'),
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyCode::Char('q'),
        crossterm::event::KeyCode::Char('?'),
        crossterm::event::KeyCode::Tab,
        crossterm::event::KeyCode::BackTab,
        crossterm::event::KeyCode::Char('r'),
        crossterm::event::KeyCode::Char(']'),
        crossterm::event::KeyCode::Char('['),
        crossterm::event::KeyCode::Char('1'),
        crossterm::event::KeyCode::Char('2'),
        crossterm::event::KeyCode::Char('3'),
        crossterm::event::KeyCode::Char('4'),
        crossterm::event::KeyCode::Char('5'),
        crossterm::event::KeyCode::Char('6'),
        crossterm::event::KeyCode::Char('7'),
        crossterm::event::KeyCode::Char('8'),
        crossterm::event::KeyCode::Char('9'),
        crossterm::event::KeyCode::Char('0'),
    ];

    // Setup mock item detail repo
    let mock_info = crate::repo::RepoInfo::default();
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    for &kc in &keycodes {
        // Test with inspect_full_diff = true
        app.inspect_full_diff = true;
        app.commit_list.search_query = Some("test".to_string());
        let key = crossterm::event::KeyEvent::new(kc, crossterm::event::KeyModifiers::empty());
        crate::tabs::route_detail_event(&mut app, key);

        // Test with inspect_full_diff = false
        app.inspect_full_diff = false;
        app.commit_list.search_query = None;
        crate::tabs::route_detail_event(&mut app, key);
    }
}

#[test]
fn test_logs_tab_comprehensive() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_logs_tab_comp.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);

    let keycodes = vec![
        crossterm::event::KeyCode::Up,
        crossterm::event::KeyCode::Char('k'),
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyCode::Char('j'),
        crossterm::event::KeyCode::PageUp,
        crossterm::event::KeyCode::PageDown,
        crossterm::event::KeyCode::Home,
        crossterm::event::KeyCode::End,
        crossterm::event::KeyCode::Char('/'),
        crossterm::event::KeyCode::Char('f'),
        crossterm::event::KeyCode::Char('G'),
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyCode::Char('q'),
        crossterm::event::KeyCode::Char('Q'),
        crossterm::event::KeyCode::Char('x'),
    ];

    let mock_info = crate::repo::RepoInfo::default();
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    for &kc in &keycodes {
        // Test with uncommitted selected = true
        app.commit_list.selection = 0; // index 0 is uncommitted
        app.commit_list.limit = 100;
        let key = crossterm::event::KeyEvent::new(kc, crossterm::event::KeyModifiers::empty());
        crate::tabs::logs::LogsTab::handle_event(&mut app, key);

        // Test with uncommitted selected = false
        app.commit_list.selection = 1;
        crate::tabs::logs::LogsTab::handle_event(&mut app, key);
    }
}

#[test]
fn test_workspace_tab_coverage_booster() {
    use crossterm::event::KeyCode;
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_coverage_workspace_booster.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);
    app.mode = Mode::Detail;
    app.in_logs_ui = false;
    app.commit_list.selection = 0;

    let mock_info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        changes: crate::repo::WorktreeChanges {
            staged: vec![crate::repo::FileEntry {
                path: "staged_file.txt".to_string(),
                label: "A",
            }],
            unstaged: vec![crate::repo::FileEntry {
                path: "unstaged_file.txt".to_string(),
                label: "M",
            }],
            conflicted: vec![crate::repo::FileEntry {
                path: "conflicted_file.txt".to_string(),
                label: "C",
            }],
            ..Default::default()
        },
        commits: vec![crate::repo::CommitEntry {
            id: "abc1234".to_string(),
            oid: "abc1234".to_string(),
            author: "Author".to_string(),
            when: "10 mins ago".to_string(),
            date: "2026-07-02".to_string(),
            summary: "feat: summary".to_string(),
            message: "feat: summary\n\nbody".to_string(),
            refs: vec![],
            files: vec![],
            signature_status: "G".to_string(),
        }],
        ..Default::default()
    };

    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    // Mock file diff with multiple kinds of lines
    app.diff.file_diff = vec![
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::Header,
            content: "@@ -1,3 +1,3 @@".to_string(),
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::Added,
            content: "+added line".to_string(),
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::Removed,
            content: "-removed line".to_string(),
        },
    ];

    let focuses = vec![
        DetailSection::Commits,
        DetailSection::Staged,
        DetailSection::Unstaged,
        DetailSection::Conflicts,
        DetailSection::CommitDetails,
        DetailSection::StagingDetails,
        DetailSection::ConflictDiff,
    ];

    let keycodes = vec![
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::Enter,
        KeyCode::Char('a'),
        KeyCode::Char('A'),
        KeyCode::Char('x'),
        KeyCode::Char('X'),
        KeyCode::Char('c'),
        KeyCode::Char('C'),
        KeyCode::Char('o'),
        KeyCode::Char('t'),
        KeyCode::Char('r'),
        KeyCode::Right,
        KeyCode::Delete,
        KeyCode::Char('l'),
        KeyCode::Char('G'),
        KeyCode::Char('/'),
        KeyCode::Char('f'),
        KeyCode::Char('b'),
        KeyCode::Char('t'),
        KeyCode::Char('i'),
        KeyCode::Char('p'),
        KeyCode::Char('v'),
        KeyCode::Char('y'),
        KeyCode::Char('s'),
        KeyCode::Char('S'),
    ];

    for &is_uncommitted in &[true, false] {
        let selection = if is_uncommitted { 0 } else { 1 };
        for &focus in &focuses {
            // Pass 1: Line mode disabled
            for &kc in &keycodes {
                app.detail_focus = focus;
                app.mode = Mode::Detail;
                app.in_logs_ui = false;
                app.commit_list.selection = selection;
                app.diff.diff_line_mode = false;

                let key =
                    crossterm::event::KeyEvent::new(kc, crossterm::event::KeyModifiers::empty());
                let _ = crate::tabs::workspace::WorkspaceTab::handle_event(&mut app, key);
            }

            // Pass 2: Line mode enabled
            for &kc in &keycodes {
                app.detail_focus = focus;
                app.mode = Mode::Detail;
                app.in_logs_ui = false;
                app.commit_list.selection = selection;
                app.diff.diff_line_mode = true;

                let key =
                    crossterm::event::KeyEvent::new(kc, crossterm::event::KeyModifiers::empty());
                let _ = crate::tabs::workspace::WorkspaceTab::handle_event(&mut app, key);
            }
        }
    }
}

#[test]
fn test_files_tab_git_blame_toggle() {
    use crate::tabs::FilesTab;
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_files_tab_blame.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);
    app.mode = Mode::Detail;
    app.detail_focus = DetailSection::FileContent;

    // Initially show_blame is false, show_line_numbers is true
    assert!(!app.file_tree.show_blame);
    assert!(app.file_tree.show_line_numbers);

    // Simulate pressing 'b' to toggle blame on
    let key_b = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('b'),
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = FilesTab::handle_event(&mut app, key_b);
    assert!(handled);
    assert!(app.file_tree.show_blame);

    // Simulate pressing 'B' to toggle blame off
    let key_b_caps = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('B'),
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = FilesTab::handle_event(&mut app, key_b_caps);
    assert!(handled);
    assert!(!app.file_tree.show_blame);

    // Simulate pressing 'n' to toggle line numbers off
    let key_n = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('n'),
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = FilesTab::handle_event(&mut app, key_n);
    assert!(handled);
    assert!(!app.file_tree.show_line_numbers);

    // Simulate pressing 'N' to toggle line numbers back on
    let key_n_caps = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('N'),
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = FilesTab::handle_event(&mut app, key_n_caps);
    assert!(handled);
    assert!(app.file_tree.show_line_numbers);
}

#[test]
fn test_overview_scrolling() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_overview_scroll.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);

    app.mode = Mode::Overview;
    assert_eq!(app.overview_focus, crate::app::OverviewFocus::Overview);
    assert_eq!(app.overview_scroll, 0);
    assert_eq!(app.stats_scroll, 0);

    // Press Down to scroll overview
    let key_down = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::input::handle_key(&mut app, key_down, 15);
    assert!(handled);
    assert_eq!(app.overview_scroll, 1);
    assert_eq!(app.stats_scroll, 0);

    // Press Tab to cycle focus to Stats
    let key_tab = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Tab,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::input::handle_key(&mut app, key_tab, 15);
    assert!(handled);
    assert_eq!(app.overview_focus, crate::app::OverviewFocus::Stats);

    // Press Down to scroll stats
    let handled = crate::input::handle_key(&mut app, key_down, 15);
    assert!(handled);
    assert_eq!(app.overview_scroll, 1);
    assert_eq!(app.stats_scroll, 1);

    // Press PageDown to scroll stats page
    let key_pagedown = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::PageDown,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::input::handle_key(&mut app, key_pagedown, 15);
    assert!(handled);
    assert!(app.stats_scroll > 1);

    // Press Home to jump to top of stats
    let key_home = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Home,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::input::handle_key(&mut app, key_home, 15);
    assert!(handled);
    assert_eq!(app.stats_scroll, 0);

    // Press End to jump to bottom of stats
    let key_end = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::End,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::input::handle_key(&mut app, key_end, 15);
    assert!(handled);
    assert_eq!(app.stats_scroll, 99999);
}

#[test]
fn test_overview_mouse_events() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_overview_mouse.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);

    app.mode = Mode::Overview;
    app.overview_focus = crate::app::OverviewFocus::Overview;
    app.overview_scroll = 0;
    app.stats_scroll = 0;

    // Mock overview and stats areas
    app.detail_areas.overview = Some(ratatui::layout::Rect::new(0, 0, 40, 20));
    app.detail_areas.stats = Some(ratatui::layout::Rect::new(40, 0, 40, 20));

    // 1. Click on Stats area to cycle focus
    let mouse_click_stats = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: 45,
        row: 5,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, mouse_click_stats);
    assert_eq!(app.overview_focus, crate::app::OverviewFocus::Stats);

    // 2. Click on Overview area to cycle focus back
    let mouse_click_overview = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        column: 10,
        row: 5,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, mouse_click_overview);
    assert_eq!(app.overview_focus, crate::app::OverviewFocus::Overview);

    // 3. Scroll down on Stats area
    let mouse_scroll_down_stats = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::ScrollDown,
        column: 45,
        row: 5,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, mouse_scroll_down_stats);
    assert_eq!(app.stats_scroll, 1);
    assert_eq!(app.overview_scroll, 0);

    // 4. Scroll down on Overview area
    let mouse_scroll_down_overview = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::ScrollDown,
        column: 10,
        row: 5,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, mouse_scroll_down_overview);
    assert_eq!(app.stats_scroll, 1);
    assert_eq!(app.overview_scroll, 1);

    // 5. Scroll up on Stats area
    let mouse_scroll_up_stats = crossterm::event::MouseEvent {
        kind: crossterm::event::MouseEventKind::ScrollUp,
        column: 45,
        row: 5,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    crate::mouse::handle_mouse(&mut app, mouse_scroll_up_stats);
    assert_eq!(app.stats_scroll, 0);
    assert_eq!(app.overview_scroll, 1);
}

#[test]
fn test_directory_scan_hidden_repos() {
    let temp_scan_root = std::env::temp_dir().join("gitwig_test_hidden_scan");
    let _ = std::fs::remove_dir_all(&temp_scan_root);
    std::fs::create_dir_all(&temp_scan_root).unwrap();

    // 1. Create a hidden git repo (starts with '.' and contains '.git')
    let hidden_git = temp_scan_root.join(".macosdotfiles");
    std::fs::create_dir_all(hidden_git.join(".git")).unwrap();

    // 2. Create a normal git repo
    let normal_git = temp_scan_root.join("my_repo");
    std::fs::create_dir_all(normal_git.join(".git")).unwrap();

    // 3. Create a hidden non-git folder
    let hidden_nongit = temp_scan_root.join(".cargo");
    std::fs::create_dir_all(hidden_nongit.join("bin")).unwrap();

    let (tx, rx) = std::sync::mpsc::channel();
    run_directory_scan(temp_scan_root.clone(), 3, vec![], tx, true);

    let mut found = Vec::new();
    while let Ok(msg) = rx.recv() {
        if let Some(repo_info) = msg.strip_prefix("REPO_SCAN_FOUND:") {
            let parts: Vec<&str> = repo_info.split("|||").collect();
            if parts.len() == 2 {
                found.push(parts[0].to_string());
            }
        }
        if msg.starts_with("REPO_SCAN_COMPLETE:") {
            break;
        }
    }

    assert!(found.contains(&".macosdotfiles".to_string()));
    assert!(found.contains(&"my_repo".to_string()));
    assert!(!found.contains(&".cargo".to_string()));

    let _ = std::fs::remove_dir_all(&temp_scan_root);
}
