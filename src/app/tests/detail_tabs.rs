#![allow(clippy::unwrap_used, clippy::panic)]

use super::*;

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
        resolved: missing_repo_path("logs_search_picker_flow"),
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
    // The resyncs below reload a real repository, so give them a throwaway one.
    let (repo_path, _repo_guard) = temp_repo_with_file("detail_view_sync_repo", "file.txt");
    let config = Config {
        items: vec![repo_path.to_string_lossy().to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
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
        resolved: repo_path.clone(),
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
    app.current_detail =
        Some(crate::repo::ItemDetail::Repo { resolved: repo_path, info: Box::new(mock_info_2) });

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
fn test_normal_mode_right_arrow_detail() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let temp_repo = std::env::temp_dir().join("gitwig_test_right_arrow_repo");
    let _ = std::fs::remove_dir_all(&temp_repo);
    std::fs::create_dir_all(&temp_repo).unwrap();
    std::fs::create_dir_all(temp_repo.join(".git")).unwrap();
    let repo_path = temp_repo.to_string_lossy().to_string();

    let config = Config {
        items: vec![repo_path.clone()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
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
    assert_eq!(app.loading_repo_path.as_ref(), Some(&repo_path));

    // Wait for background thread message
    let (path, detail) = app.detail_rx.recv().unwrap();
    assert_eq!(path, repo_path);

    // Manually apply to verify state transition
    app.current_detail = Some(detail);
    app.loading_repo_path = None;

    assert_eq!(app.loading_repo_path, None);
    assert!(app.current_detail.is_some());

    let _ = std::fs::remove_dir_all(&temp_repo);
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
fn test_detail_cache_ttl_behavior() {
    let temp_dir = std::env::temp_dir();
    let repo_path = temp_dir.join("test_detail_cache_ttl_repo");
    let _ = std::fs::remove_dir_all(&repo_path);
    std::fs::create_dir_all(&repo_path).unwrap();
    std::fs::create_dir_all(repo_path.join(".git")).unwrap();

    // Initialize App
    let config = Config {
        items: vec![repo_path.to_string_lossy().to_string()],
        poll_interval_ms: 100,
        max_commits: 200,
        graph_max_commits: 1000,

        page_size: 10,
        sort_by: SortOrder::Custom,
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

    let (config_path, _guard) = temp_config_path("detail_cache_ttl_behavior");
    let mut app = App::new(config, config_path);

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

    let (config_path, _guard) = temp_config_path("tab_ttl_behavior");
    let mut app = App::new(config, config_path);

    // Set up mock current detail
    let mock_info = crate::repo::RepoInfo {
        commits: vec![],
        files: crate::repo::TabData::Loaded(vec!["file1.txt".to_string()]),
        tab_loaded_at: [None; 12],
        tab_loading: [false; 12],
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
fn test_pending_terminal_trigger() {
    let config = Config {
        items: vec!["/dummy/repo".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
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
    let (config_path, _guard) = temp_config_path("pending_terminal_trigger");
    let mut app = App::new(config, config_path);
    app.selected_index = 0;

    // `T` (external shell) keeps the pre-panel suspend-and-spawn behavior.
    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('T'),
        crossterm::event::KeyModifiers::SHIFT,
    );
    let handled = crate::input::handle_key(&mut app, key, 1);
    assert!(handled);
    assert!(app.pending_terminal);
}

#[test]
fn test_terminal_panel_open_without_repo() {
    let config = Config { items: vec![], ..Default::default() };
    let (config_path, _guard) = temp_config_path("terminal_panel_open_without_repo");
    let mut app = App::new(config, config_path);

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('t'),
        crossterm::event::KeyModifiers::empty(),
    );
    assert!(crate::input::handle_key(&mut app, key, 1));
    assert!(app.terminal_panel.session.is_none());
    assert!(!app.terminal_panel.visible);
    assert!(!app.terminal_focused);
    assert_eq!(app.status_message.as_deref(), Some("No repository selected"));
}

#[test]
fn test_terminal_panel_focused_swallows_keys() {
    let config = Config { items: vec!["/dummy/repo".to_string()], ..Default::default() };
    let (config_path, _guard) = temp_config_path("terminal_panel_focused_swallows_keys");
    let mut app = App::new(config, config_path);
    app.terminal_panel.visible = true;
    app.terminal_focused = true;

    // With no live session, the first key closes the dead panel — and must
    // not fall through to the home handler (`q`, `t`, etc. do nothing else).
    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('q'),
        crossterm::event::KeyModifiers::empty(),
    );
    assert!(crate::input::handle_key(&mut app, key, 1));
    assert!(!app.terminal_panel.visible);
    assert!(!app.terminal_focused);
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn test_terminal_panel_toggle_key_hides_while_focused() {
    let config = Config { items: vec!["/dummy/repo".to_string()], ..Default::default() };
    let (config_path, _guard) = temp_config_path("terminal_panel_toggle_key_hides_while_focused");
    let mut app = App::new(config, config_path);
    app.terminal_panel.visible = true;
    app.terminal_focused = true;

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('t'),
        crossterm::event::KeyModifiers::CONTROL,
    );
    assert!(crate::input::handle_key(&mut app, key, 1));
    assert!(!app.terminal_panel.visible);
    assert!(!app.terminal_focused);
}

#[test]
fn test_terminal_panel_global_quit_wins_while_focused() {
    let config = Config { items: vec!["/dummy/repo".to_string()], ..Default::default() };
    let (config_path, _guard) = temp_config_path("terminal_panel_global_quit_wins_while_focused");
    let mut app = App::new(config, config_path);
    app.terminal_panel.visible = true;
    app.terminal_focused = true;

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('q'),
        crossterm::event::KeyModifiers::CONTROL,
    );
    assert!(!crate::input::handle_key(&mut app, key, 1));
}

#[test]
fn test_terminal_panel_error_popup_wins_while_focused() {
    let config = Config { items: vec!["/dummy/repo".to_string()], ..Default::default() };
    let (config_path, _guard) = temp_config_path("terminal_panel_error_popup_wins_while_focused");
    let mut app = App::new(config, config_path);
    app.terminal_panel.visible = true;
    app.terminal_focused = true;
    app.error_message = Some("boom".to_string());

    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::empty(),
    );
    assert!(crate::input::handle_key(&mut app, key, 1));
    assert!(app.error_message.is_none());
    // The key dismissed the error; the panel state is untouched.
    assert!(app.terminal_panel.visible);
    assert!(app.terminal_focused);
}

#[test]
fn test_terminal_panel_geometry() {
    let config = Config { items: vec![], ..Default::default() };
    let (config_path, _guard) = temp_config_path("terminal_panel_geometry");
    let mut app = App::new(config, config_path);

    // Hidden panel takes no height and yields no grid.
    assert_eq!(app.terminal_panel_outer_height(40), 0);
    assert!(app.terminal_grid_size(ratatui::layout::Rect::new(0, 0, 80, 40)).is_none());

    app.terminal_panel.visible = true;
    app.terminal_panel.height = 12;
    assert_eq!(app.terminal_panel_outer_height(40), 12);
    // Grid excludes the 2 border rows/cols.
    assert_eq!(app.terminal_grid_size(ratatui::layout::Rect::new(0, 0, 80, 40)), Some((10, 78)));

    // A tiny window clamps the panel but never below the 5-row minimum.
    assert_eq!(app.terminal_panel_outer_height(8), 5);
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
fn test_file_history_view_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    // Opening the history reads it from a real repository.
    let (repo_path, _repo_guard) = temp_repo_with_file("file_history_repo", "Cargo.toml");

    let config = Config {
        items: vec![repo_path.to_string_lossy().to_string()],
        poll_interval_ms: 100,
        max_commits: 10,
        page_size: 10,
        sort_by: SortOrder::Custom,
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
    app.current_detail =
        Some(crate::repo::ItemDetail::Repo { resolved: repo_path, info: Box::new(mock_info) });

    // Open file history (loads asynchronously via BgEvent::FileHistoryLoaded)
    app.open_file_history();
    assert_eq!(app.mode, Mode::FileHistory);
    assert_eq!(app.file_history_path, "Cargo.toml");
    assert_eq!(app.file_history_selection, 0);
    assert_eq!(app.file_history_focus, 0); // Focus on revisions list
    assert!(app.file_history_loading);

    let start = std::time::Instant::now();
    while app.file_history_loading && start.elapsed().as_secs() < 5 {
        app.drain_bg_events();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(!app.file_history_loading, "file history should finish loading");
    assert!(!app.file_history_revisions.is_empty(), "Cargo.toml should have git history");

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
    let (config_path, _guard) = temp_config_path("files_tab_editor_shortcut");
    let mut app = App::new(config, config_path);

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
        resolved: missing_repo_path("files_tab_editor_shortcut"),
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
fn test_picker_list_navigation_keys() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // One repo carrying 15 labels: the picker shows 16 rows (All + 15), which
    // gives PageUp/PageDown (page_size 10) room to move and clamp.
    let mut config = Config { items: vec!["/path/to/repo_a".to_string()], ..Default::default() };
    let labels: Vec<String> = (0..15).map(|i| format!("label{:02}", i)).collect();
    config.labels.insert("/path/to/repo_a".to_string(), labels);
    let temp_path = std::env::temp_dir().join("gitwig_test_picker_nav.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);
    assert_eq!(app.config.page_size, 10);

    let key = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    // Label picker: End / Home / PageDown / PageUp navigate and clamp.
    assert!(crate::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('L'), KeyModifiers::SHIFT),
        1
    ));
    assert_eq!(app.mode, Mode::LabelPicker);
    assert_eq!(app.get_label_matches().len(), 16);

    assert!(crate::input::handle_key(&mut app, key(KeyCode::End), 1));
    assert_eq!(app.label_picker_selection, 15);
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Home), 1));
    assert_eq!(app.label_picker_selection, 0);
    assert!(crate::input::handle_key(&mut app, key(KeyCode::PageDown), 1));
    assert_eq!(app.label_picker_selection, 10);
    assert!(crate::input::handle_key(&mut app, key(KeyCode::PageDown), 1));
    assert_eq!(app.label_picker_selection, 15); // clamped to the last row
    assert!(crate::input::handle_key(&mut app, key(KeyCode::PageUp), 1));
    assert_eq!(app.label_picker_selection, 5);
    assert!(crate::input::handle_key(&mut app, key(KeyCode::PageUp), 1));
    assert_eq!(app.label_picker_selection, 0);
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Esc), 1));

    // Repo scan picker (add repo): same keys drive repo_scan_selection.
    app.scanned_repos =
        (0..12).map(|i| (format!("repo{:02}", i), format!("/scan/repo{:02}", i))).collect();
    app.mode = Mode::RepoScanPicker;
    app.repo_scan_selection = 0;
    assert!(crate::input::handle_key(&mut app, key(KeyCode::End), 1));
    assert_eq!(app.repo_scan_selection, 11);
    assert!(crate::input::handle_key(&mut app, key(KeyCode::PageUp), 1));
    assert_eq!(app.repo_scan_selection, 1);
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Home), 1));
    assert_eq!(app.repo_scan_selection, 0);
    assert!(crate::input::handle_key(&mut app, key(KeyCode::PageDown), 1));
    assert_eq!(app.repo_scan_selection, 10);
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Esc), 1));
    assert_eq!(app.mode, Mode::Normal);
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

    // 2. Press Enter: asks to check out the target OID first.
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::CommitCheckoutConfirm);
    assert_eq!(
        app.commit_action_target_oid.as_deref(),
        Some("abcdef1234567890abcdef1234567890abcdef12")
    );
    assert!(!app.fetching);

    // 3. Confirming runs it (and reports the failure: the repository is fake).
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('y')), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);
    assert!(app.error_message.as_deref().unwrap_or("").contains("Failed to checkout commit"));
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
        path: None,
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
            path: None,
        },
        crate::repo::FileRevision {
            commit_oid: "def5678def5678def5678def5678def5678def5678".to_string(),
            author: "Author2".to_string(),
            when: "yesterday".to_string(),
            date: "2026-07-01".to_string(),
            summary: "fix: bug".to_string(),
            path: None,
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
            path: None,
        },
        crate::repo::FileRevision {
            commit_oid: "def5678def5678def5678def5678def5678def5678".to_string(),
            author: "Author2".to_string(),
            when: "yesterday".to_string(),
            date: "2026-07-01".to_string(),
            summary: "fix: bug".to_string(),
            path: None,
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
fn test_global_code_search() {
    let temp_dir = std::env::temp_dir().join("gitwig_global_search_test");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let file1 = temp_dir.join("file1.txt");
    std::fs::write(&file1, "hello world\nthis is a gitwig test\nrust coding").unwrap();

    let file2 = temp_dir.join("file2.txt");
    std::fs::write(&file2, "unrelated text\nkeep coding").unwrap();

    let config =
        Config { items: vec![temp_dir.to_string_lossy().to_string()], ..Default::default() };
    let mut app = App::new(config, std::env::temp_dir().join("dummy_config.toml"));

    // Set search query in input_buffer
    app.input_buffer = "gitwig".to_string();
    app.trigger_global_search();

    // Poll the receiver for results (with a timeout of 1000ms max)
    let start = std::time::Instant::now();
    let mut results = Vec::new();
    while start.elapsed().as_millis() < 1000 {
        if let Ok(res) = app.global_search_rx.try_recv() {
            results = res;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    assert!(!results.is_empty(), "Search results should not be empty");
    assert_eq!(results[0].repo_name, temp_dir.file_name().unwrap().to_string_lossy().to_string());
    assert_eq!(results[0].file_rel_path, "file1.txt");
    assert_eq!(results[0].line_number, 2);
    assert_eq!(results[0].line_content, "this is a gitwig test");

    // Apply the snapshot of results to app
    app.global_search_results = results;
    app.global_search_selection = 0;
    app.global_search_focus_input = false;

    // Test mouse and keyboard navigation actions on search results
    app.global_search_results = vec![
        SearchResult {
            repo_name: "a".to_string(),
            repo_path: "p".to_string(),
            file_rel_path: "f".to_string(),
            line_number: 1,
            line_content: "c".to_string(),
        },
        SearchResult {
            repo_name: "a".to_string(),
            repo_path: "p".to_string(),
            file_rel_path: "f".to_string(),
            line_number: 2,
            line_content: "c".to_string(),
        },
        SearchResult {
            repo_name: "a".to_string(),
            repo_path: "p".to_string(),
            file_rel_path: "f".to_string(),
            line_number: 3,
            line_content: "c".to_string(),
        },
    ];
    app.mode = Mode::GlobalSearch;
    app.config.page_size = 2;

    // Test PageDown
    crate::input::handle_key(
        &mut app,
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::PageDown,
            crossterm::event::KeyModifiers::empty(),
        ),
        0,
    );
    assert_eq!(app.global_search_selection, 2);

    // Test PageUp
    crate::input::handle_key(
        &mut app,
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::PageUp,
            crossterm::event::KeyModifiers::empty(),
        ),
        0,
    );
    assert_eq!(app.global_search_selection, 0);

    // Test End
    crate::input::handle_key(
        &mut app,
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::End,
            crossterm::event::KeyModifiers::empty(),
        ),
        0,
    );
    assert_eq!(app.global_search_selection, 2);

    // Test Home
    crate::input::handle_key(
        &mut app,
        crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Home,
            crossterm::event::KeyModifiers::empty(),
        ),
        0,
    );
    assert_eq!(app.global_search_selection, 0);

    // Test Mouse ScrollDown
    crate::mouse::handle_mouse(
        &mut app,
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::empty(),
        },
    );
    assert_eq!(app.global_search_selection, 1);

    // Test Mouse ScrollUp
    crate::mouse::handle_mouse(
        &mut app,
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: crossterm::event::KeyModifiers::empty(),
        },
    );
    assert_eq!(app.global_search_selection, 0);

    // Set selection back to 0 and matching repo path to test selection jump
    app.global_search_results[0].repo_path = temp_dir.to_string_lossy().to_string();
    app.select_global_search_result();

    // Verify app mode changed to Mode::Detail
    assert_eq!(app.mode, Mode::Detail);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// The Issues (10) and PRs (11) tabs could stay on "Loading..." for good:
/// finished loads only cleared `tab_loading` below index 10, so after `R`, a
/// checkout or a background refresh the tabs were never loaded again, and a
/// background detail refresh dropped their lists. PR line comments share
/// index 11 but must not end the PR list's load.
#[test]
fn test_forge_tab_loads_finish_so_the_tabs_can_reload() {
    let (mut app, items, _guard) = app_with_three_repos("forge_tab_loads");
    let path = items[1].clone();
    let issue = crate::repo::ForgeIssue {
        number: 1,
        title: "Issue 1".to_string(),
        state: "OPEN".to_string(),
        author: "user1".to_string(),
        assignees: vec![],
        url: "https://github.com/org/repo/issues/1".to_string(),
    };
    let mut info = crate::repo::RepoInfo {
        forge_issues: crate::repo::TabData::Loading,
        forge_prs: crate::repo::TabData::Loading,
        ..Default::default()
    };
    info.tab_loading[10] = true;
    info.tab_loading[11] = true;
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from(&path),
        info: Box::new(info),
    });
    let forge = |app: &App| match &app.current_detail {
        Some(crate::repo::ItemDetail::Repo { info, .. }) => info.as_ref().clone(),
        _ => panic!("no repository open"),
    };

    // Line comments arriving on index 11 leave the PR list's load running.
    let comments = crate::repo::TabPayload::PRComments { pr_number: 7, result: Ok(vec![]) };
    app.tab_tx.send((path.clone(), 11, comments)).unwrap();
    assert!(app.drain_tab_payloads());
    assert!(forge(&app).tab_loading[11], "comments do not end the PR list load");

    let issues = crate::repo::TabPayload::ForgeIssues(Ok(vec![issue]));
    app.tab_tx.send((path.clone(), 10, issues)).unwrap();
    app.tab_tx.send((path.clone(), 11, crate::repo::TabPayload::ForgePRs(Ok(vec![])))).unwrap();
    app.tab_tx
        .send((path.clone(), 99, crate::repo::TabPayload::Overview(Ok((vec![], false)))))
        .unwrap();
    assert!(app.drain_tab_payloads());
    let info = forge(&app);
    for tab in [10, 11] {
        assert!(!info.tab_loading[tab], "tab {} is no longer loading", tab);
        assert!(info.tab_loaded_at[tab].is_some(), "tab {} records its load time", tab);
    }
    assert_eq!(info.forge_issues.len(), 1);
    assert!(info.forge_prs.is_loaded());

    // A background detail refresh keeps the loaded forge lists.
    app.apply_detail_snapshot(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from(&path),
        info: Box::default(),
    });
    assert_eq!(forge(&app).forge_issues.len(), 1);
    assert!(forge(&app).forge_prs.is_loaded());

    // Leaving mid-load drops the result; reopening from the cache must not
    // restore the tab as loading forever.
    let mut stale = forge(&app);
    stale.forge_issues = crate::repo::TabData::Loading;
    stale.tab_loading[10] = true;
    app.detail_cache.insert(
        path.clone(),
        DetailCache {
            detail: crate::repo::ItemDetail::Repo {
                resolved: PathBuf::from(&path),
                info: Box::new(stale),
            },
            loaded_at: std::time::Instant::now(),
        },
    );
    app.current_detail = None;
    app.open_repo(path.clone());
    let info = forge(&app);
    assert!(!info.tab_loading[10]);
    assert!(info.forge_issues.is_not_loaded(), "the tab loads again when shown");
    assert!(info.forge_prs.is_loaded(), "finished loads are kept");
}

/// The PR line-comment wizard ignored every key but Esc and Enter, so nothing
/// typed reached it, and three Enters posted an empty comment on line 1. Each
/// step now takes typing and keeps itself open with an error on bad input.
#[test]
fn test_forge_comment_wizard_takes_typing_and_validates_each_step() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let type_text = |app: &mut App, text: &str| {
        for c in text.chars() {
            assert!(crate::input::handle_key(app, key(KeyCode::Char(c)), 1));
        }
    };

    let config_path = std::env::temp_dir().join("gitwig_test_forge_comment_wizard.toml");
    let _guard = TestFileGuard { path: config_path.clone() };
    let mut app = App::new(Config::default(), config_path);
    let pr = crate::repo::ForgePR {
        number: 42,
        title: "Add retry".to_string(),
        state: "OPEN".to_string(),
        author: "alice".to_string(),
        assignees: vec![],
        url: "https://github.com/o/r/pull/42".to_string(),
        head_ref: "retry".to_string(),
        head_ref_oid: "abc123".to_string(),
        body: String::new(),
        status_checks: vec![],
        reviews: vec![],
    };
    let info = crate::repo::RepoInfo {
        forge_prs: crate::repo::TabData::Loaded(vec![pr]),
        ..Default::default()
    };
    // A path that does not exist, so the final post cannot reach any `gh`.
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/nonexistent/gitwig_test_forge_comment_repo"),
        info: Box::new(info),
    });
    app.mode = Mode::Detail;
    app.advanced_tabs = true;
    app.detail_tab = 11;
    app.detail_focus = DetailSection::ForgePRs;
    app.forge_pr_comments = Some(Ok(vec![]));

    assert!(crate::input::handle_key(&mut app, key(KeyCode::Char('n')), 1));
    assert_eq!(app.mode, Mode::ForgeCommentPathInput);

    // An empty path is refused; the step stays open.
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert!(app.error_message.is_some());
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Esc), 1));
    assert_eq!(app.mode, Mode::ForgeCommentPathInput, "dismissing the error keeps the step");

    type_text(&mut app, "src/lib.rsx");
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Backspace), 1));
    assert_eq!(app.input_buffer, "src/lib.rs");
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert_eq!(app.mode, Mode::ForgeCommentLineInput);
    assert_eq!(app.forge_comment_path, "src/lib.rs");

    // "1O" and 0 are not line numbers; the typed text is kept for fixing.
    type_text(&mut app, "1O");
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert_eq!(app.mode, Mode::ForgeCommentLineInput);
    assert!(app.error_message.is_some());
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Esc), 1));
    assert_eq!(app.input_buffer, "1O");
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Backspace), 1));
    type_text(&mut app, "0");
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert_eq!(app.mode, Mode::ForgeCommentBodyInput);
    assert_eq!(app.forge_comment_line, 10);

    // An empty body is refused; typed text, `@` and `q` included, is taken.
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert!(app.error_message.is_some());
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Esc), 1));
    type_text(&mut app, "@alice q?");
    assert_eq!(app.input_buffer, "@alice q?");
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert_eq!(app.mode, Mode::Detail);
    assert!(app.fetching, "the comment is being posted");
}

/// A failed PR line-comment load used to clear the comments back to `None`,
/// which the Detail loop reads as "never requested": it re-ran `gh api` on
/// every frame (about 59 calls in 6 s) and the panel only ever showed
/// "Loading line comments...". The failure is now kept, shown, and not retried
/// until the selection changes or the tab is resynced.
#[test]
fn test_failed_pr_comment_load_is_kept_and_not_retried_every_frame() {
    let config_path = std::env::temp_dir().join("gitwig_test_pr_comment_failure.toml");
    let _guard = TestFileGuard { path: config_path.clone() };
    let mut app = App::new(Config::default(), config_path);
    let path = "/nonexistent/gitwig_test_pr_comment_repo".to_string();
    let pr = crate::repo::ForgePR {
        number: 42,
        title: "Add retry".to_string(),
        state: "OPEN".to_string(),
        author: "alice".to_string(),
        assignees: vec![],
        url: "https://github.com/o/r/pull/42".to_string(),
        head_ref: "retry".to_string(),
        head_ref_oid: "abc123".to_string(),
        body: String::new(),
        status_checks: vec![],
        reviews: vec![],
    };
    let mut info = crate::repo::RepoInfo {
        forge_prs: crate::repo::TabData::Loaded(vec![pr]),
        ..Default::default()
    };
    // The PR list is fresh, so the per-frame trigger only concerns comments.
    info.tab_loaded_at[11] = Some(std::time::Instant::now());
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from(&path),
        info: Box::new(info),
    });
    app.mode = Mode::Detail;
    app.advanced_tabs = true;
    app.detail_tab = 11;
    app.forge_pr_comments_loading = true;
    app.forge_pr_comments_pr = Some(42);

    let failure = Err("HTTP 403: Resource not accessible".to_string());
    let payload = crate::repo::TabPayload::PRComments { pr_number: 42, result: failure };
    app.tab_tx.send((path, 11, payload)).unwrap();
    assert!(app.drain_tab_payloads());
    assert!(!app.forge_pr_comments_loading);
    assert!(matches!(&app.forge_pr_comments, Some(Err(e)) if e.contains("403")));

    for _ in 0..5 {
        app.trigger_tab_load_if_needed(11);
        assert!(!app.forge_pr_comments_loading, "a failed load is not requested again");
    }
    assert!(matches!(app.forge_pr_comments, Some(Err(_))), "the failure stays visible");
}

/// Line comments of one PR could show under another: a mouse click or scroll
/// changed the selection without reloading them, and a slow response for the
/// previously selected PR could overwrite the one for the current PR.
#[test]
fn test_pr_line_comments_follow_the_selected_pr() {
    let config_path = std::env::temp_dir().join("gitwig_test_pr_comments_follow.toml");
    let _guard = TestFileGuard { path: config_path.clone() };
    let mut app = App::new(Config::default(), config_path);
    // A path that does not exist, so a triggered load cannot reach any `gh`.
    let path = "/nonexistent/gitwig_test_pr_comments_follow".to_string();
    let pr = |number: u32| crate::repo::ForgePR {
        number,
        title: format!("PR {}", number),
        state: "OPEN".to_string(),
        author: "alice".to_string(),
        assignees: vec![],
        url: format!("https://github.com/o/r/pull/{}", number),
        head_ref: format!("branch-{}", number),
        head_ref_oid: "abc".to_string(),
        body: String::new(),
        status_checks: vec![],
        reviews: vec![],
    };
    let comment = |body: &str| crate::repo::ForgePRComment {
        path: "a.rs".to_string(),
        line: Some(1),
        body: body.to_string(),
        author: "bob".to_string(),
        commit_id: "abc".to_string(),
    };
    let mut info = crate::repo::RepoInfo {
        forge_prs: crate::repo::TabData::Loaded(vec![pr(42), pr(43)]),
        ..Default::default()
    };
    info.tab_loaded_at[11] = Some(std::time::Instant::now());
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from(&path),
        info: Box::new(info),
    });
    app.mode = Mode::Detail;
    app.advanced_tabs = true;
    app.detail_tab = 11;

    // #43 is selected and its load is running; #42's slower load lands late.
    app.forge_pr_selection = 1;
    app.forge_pr_comments_pr = Some(43);
    app.forge_pr_comments_loading = true;
    let late = crate::repo::TabPayload::PRComments {
        pr_number: 42,
        result: Ok(vec![comment("about 42")]),
    };
    app.tab_tx.send((path.clone(), 11, late)).unwrap();
    app.drain_tab_payloads();
    assert!(app.forge_pr_comments.is_none(), "#42's comments are dropped");
    assert!(app.forge_pr_comments_loading, "#43's load is still awaited");
    let current = crate::repo::TabPayload::PRComments {
        pr_number: 43,
        result: Ok(vec![comment("about 43")]),
    };
    app.tab_tx.send((path.clone(), 11, current)).unwrap();
    app.drain_tab_payloads();
    assert!(matches!(&app.forge_pr_comments, Some(Ok(c)) if c[0].body == "about 43"));

    // The mouse moves the selection back to #42 without a reload: the next
    // frame's trigger requests #42's comments.
    app.forge_pr_selection = 0;
    app.trigger_tab_load_if_needed(11);
    assert_eq!(app.forge_pr_comments_pr, Some(42));
    assert!(app.forge_pr_comments_loading);
    assert!(app.forge_pr_comments.is_none(), "#43's comments are no longer held");
}

/// `docs/panels.md` has always listed `J` / `K` next to `j` / `k` for the
/// detail tabs (Logs and File History accept them); the detail tabs did not.
#[test]
fn test_detail_lists_move_with_shift_j_and_k() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let info = crate::repo::RepoInfo {
        worktrees: crate::repo::TabData::Loaded(vec![test_worktree("a"), test_worktree("b")]),
        ..Default::default()
    };
    let (mut app, _guard) = advanced_tab_app("shift_jk", info, 7);
    app.detail_focus = DetailSection::Worktrees;
    let key = |c: char| KeyEvent::new(KeyCode::Char(c), KeyModifiers::SHIFT);
    assert!(crate::input::handle_key(&mut app, key('J'), 1));
    assert_eq!(app.worktree_selection, 1);
    assert!(crate::input::handle_key(&mut app, key('K'), 1));
    assert_eq!(app.worktree_selection, 0);
}

/// Checkouts from the Reflog, Issues and PRs tabs ran on the first `Enter`;
/// they now ask first, like the Workspace tab's commit checkout.
#[test]
fn test_reflog_issue_and_pr_checkouts_ask_first() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    // Reflog: Enter asks to check out the entry's commit; Esc cancels.
    let info = crate::repo::RepoInfo {
        reflog: crate::repo::TabData::Loaded(vec![test_reflog_entry(0), test_reflog_entry(1)]),
        ..Default::default()
    };
    let (mut app, _guard) = advanced_tab_app("reflog_confirm", info, 9);
    app.reflog_selection = 1;
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert_eq!(app.mode, Mode::CommitCheckoutConfirm);
    assert_eq!(app.commit_action_target_oid, Some(format!("{:040x}", 2)));
    assert!(!app.fetching, "nothing runs before the answer");
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Esc), 1));
    assert_eq!(app.mode, Mode::Detail);
    assert!(app.commit_action_target_oid.is_none());

    // Issues: Enter asks; `n` cancels.
    let issue = crate::repo::ForgeIssue {
        number: 12,
        title: "Login fails".to_string(),
        state: "OPEN".to_string(),
        author: "alice".to_string(),
        assignees: vec![],
        url: "https://github.com/o/r/issues/12".to_string(),
    };
    let info = crate::repo::RepoInfo {
        forge_issues: crate::repo::TabData::Loaded(vec![issue]),
        ..Default::default()
    };
    let (mut app, _guard) = advanced_tab_app("issue_confirm", info, 10);
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert_eq!(app.mode, Mode::ForgeCheckoutConfirm);
    assert_eq!(
        app.forge_checkout_target,
        Some(crate::app::ForgeCheckoutTarget::Issue {
            number: 12,
            title: "Login fails".to_string()
        })
    );
    assert!(!app.fetching);
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Char('n')), 1));
    assert_eq!(app.mode, Mode::Detail);
    assert!(app.forge_checkout_target.is_none());

    // PRs: Enter asks; Enter again (not destructive) confirms and starts it.
    let pr = crate::repo::ForgePR {
        number: 42,
        title: "Add retry".to_string(),
        state: "OPEN".to_string(),
        author: "alice".to_string(),
        assignees: vec![],
        url: "https://github.com/o/r/pull/42".to_string(),
        head_ref: "retry".to_string(),
        head_ref_oid: "abc".to_string(),
        body: String::new(),
        status_checks: vec![],
        reviews: vec![],
    };
    let info = crate::repo::RepoInfo {
        forge_prs: crate::repo::TabData::Loaded(vec![pr]),
        ..Default::default()
    };
    let (mut app, _guard) = advanced_tab_app("pr_confirm", info, 11);
    app.forge_pr_comments_pr = Some(42);
    app.forge_pr_comments = Some(Ok(vec![]));
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert_eq!(app.mode, Mode::ForgeCheckoutConfirm);
    assert!(!app.fetching);
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert_eq!(app.mode, Mode::Detail);
    assert!(app.fetching, "the checkout runs once confirmed");
    assert!(app.forge_checkout_target.is_none());
}

/// The Worktrees and Submodules lists were drawn without scrolling, so a
/// selection past the panel's height was invisible.
#[test]
fn test_worktree_and_submodule_lists_scroll_to_the_selection() {
    let config_path = std::env::temp_dir().join("gitwig_test_list_scroll.toml");
    let _guard = TestFileGuard { path: config_path.clone() };
    let app = App::new(Config::default(), config_path);
    let text_of = |terminal: &ratatui::Terminal<ratatui::backend::TestBackend>| {
        let buffer = terminal.backend().buffer();
        (0..20)
            .map(|y| (0..100).map(|x| buffer[(x, y)].symbol()).collect::<String>() + "\n")
            .collect::<String>()
    };

    let info = crate::repo::RepoInfo {
        worktrees: crate::repo::TabData::Loaded(
            (0..40).map(|i| test_worktree(&format!("wt{:02}", i))).collect(),
        ),
        submodules: crate::repo::TabData::Loaded(
            (0..40)
                .map(|i| crate::repo::SubmoduleInfo {
                    name: format!("sub{:02}", i),
                    path: PathBuf::from(format!("sub{:02}", i)),
                    // #39's removal is staged: in HEAD, not in the index.
                    commit_id: (i != 39).then(|| "a".repeat(40)),
                    head_id: Some("a".repeat(40)),
                    is_initialized: true,
                    ..Default::default()
                })
                .collect(),
        ),
        ..Default::default()
    };
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(100, 20)).unwrap();
    let mut areas = crate::ui_detail::DetailAreas::default();
    terminal
        .draw(|f| {
            crate::components::worktree_list::draw_worktrees_view(
                f,
                &info,
                DetailSection::Worktrees,
                35,
                &mut areas,
                &app,
                ratatui::layout::Rect::new(0, 0, 100, 20),
            )
        })
        .unwrap();
    assert!(text_of(&terminal).contains("wt35"), "{}", text_of(&terminal));
    assert!(areas.worktrees_offset > 0);

    terminal
        .draw(|f| {
            crate::components::submodule_list::draw_submodules_view(
                f,
                &info,
                DetailSection::Submodules,
                39,
                &mut areas,
                &app,
                ratatui::layout::Rect::new(0, 0, 100, 20),
            )
        })
        .unwrap();
    let text = text_of(&terminal);
    assert!(text.contains("sub39"), "{}", text);
    assert!(text.contains("Removal staged"), "{}", text);
    assert!(areas.submodules_offset > 0);
}

#[test]
fn test_forge_tab_event_handling() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_forge_dummy.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);
    app.mode = Mode::Detail;
    app.detail_tab = 10; // Forge Tab
    app.detail_focus = DetailSection::ForgeIssues;

    let mock_info = crate::repo::RepoInfo {
        forge_issues: crate::repo::TabData::Loaded(vec![
            crate::repo::ForgeIssue {
                number: 1,
                title: "Issue 1".to_string(),
                state: "OPEN".to_string(),
                author: "user1".to_string(),
                assignees: vec!["user2".to_string()],
                url: "https://github.com/org/repo/issues/1".to_string(),
            },
            crate::repo::ForgeIssue {
                number: 2,
                title: "Issue 2".to_string(),
                state: "CLOSED".to_string(),
                author: "user3".to_string(),
                assignees: vec![],
                url: "https://github.com/org/repo/issues/2".to_string(),
            },
        ]),
        tab_loaded_at: [None; 12],
        tab_loading: [false; 12],
        ..Default::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    // Test selection movement keys
    assert_eq!(app.forge_issue_selection, 0);

    // Move down
    let key_down = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::tabs::route_detail_event(&mut app, key_down);
    assert!(handled);
    assert_eq!(app.forge_issue_selection, 1);

    // Move up
    let key_up = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Up,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::tabs::route_detail_event(&mut app, key_up);
    assert!(handled);
    assert_eq!(app.forge_issue_selection, 0);

    // Home key
    let key_home = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Home,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::tabs::route_detail_event(&mut app, key_home);
    assert!(handled);
    assert_eq!(app.forge_issue_selection, 0);

    // End key
    let key_end = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::End,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::tabs::route_detail_event(&mut app, key_end);
    assert!(handled);
    assert_eq!(app.forge_issue_selection, 1);

    // Toggle assigned only filter
    assert!(app.forge_issues_assigned_only);
    let key_a = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('a'),
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::tabs::route_detail_event(&mut app, key_a);
    assert!(handled);
    assert!(!app.forge_issues_assigned_only);
}

#[test]
fn test_forge_pr_tab_event_handling() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_forge_prs_dummy.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);
    app.mode = Mode::Detail;
    app.detail_tab = 11; // PRs Tab
    app.detail_focus = DetailSection::ForgePRs;

    let mock_info = crate::repo::RepoInfo {
        forge_prs: crate::repo::TabData::Loaded(vec![
            crate::repo::ForgePR {
                number: 101,
                title: "PR 1".to_string(),
                state: "OPEN".to_string(),
                author: "user1".to_string(),
                assignees: vec!["user2".to_string()],
                url: "https://github.com/org/repo/pull/101".to_string(),
                head_ref: "feature/pr-1".to_string(),
                head_ref_oid: "sha101".to_string(),
                body: "Body 1".to_string(),
                status_checks: vec![],
                reviews: vec![],
            },
            crate::repo::ForgePR {
                number: 102,
                title: "PR 2".to_string(),
                state: "MERGED".to_string(),
                author: "user3".to_string(),
                assignees: vec![],
                url: "https://github.com/org/repo/pull/102".to_string(),
                head_ref: "feature/pr-2".to_string(),
                head_ref_oid: "sha102".to_string(),
                body: "Body 2".to_string(),
                status_checks: vec![],
                reviews: vec![],
            },
        ]),
        tab_loaded_at: [None; 12],
        tab_loading: [false; 12],
        ..Default::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    // Test selection movement keys
    assert_eq!(app.forge_pr_selection, 0);

    // Move down
    let key_down = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Down,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::tabs::route_detail_event(&mut app, key_down);
    assert!(handled);
    assert_eq!(app.forge_pr_selection, 1);

    // Move up
    let key_up = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Up,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::tabs::route_detail_event(&mut app, key_up);
    assert!(handled);
    assert_eq!(app.forge_pr_selection, 0);

    // Pressing 'n' should start the comment path input wizard
    let key_n = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('n'),
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::tabs::route_detail_event(&mut app, key_n);
    assert!(handled);
    assert_eq!(app.mode, Mode::ForgeCommentPathInput);

    // Simulate typing a file path: "src/main.rs"
    app.input_buffer = "src/main.rs".to_string();
    let key_enter = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::input::handle_key(&mut app, key_enter, 1);
    assert!(handled);
    assert_eq!(app.forge_comment_path, "src/main.rs");
    assert_eq!(app.mode, Mode::ForgeCommentLineInput);

    // Simulate typing a line number: "10"
    app.input_buffer = "10".to_string();
    let handled = crate::input::handle_key(&mut app, key_enter, 1);
    assert!(handled);
    assert_eq!(app.forge_comment_line, 10);
    assert_eq!(app.mode, Mode::ForgeCommentBodyInput);

    // Simulate typing comment body: "Looks good"
    app.input_buffer = "Looks good".to_string();
    let handled = crate::input::handle_key(&mut app, key_enter, 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);
}

#[test]
fn test_tab_groups_primary_advanced() {
    let config = Config::default();
    let temp_dir = std::env::temp_dir().join("gitwig_test_tab_groups_dir");
    let _ = std::fs::create_dir_all(&temp_dir);
    let temp_config_path = temp_dir.join("config.toml");
    let _guard = TestDirGuard { path: temp_dir };
    let mut app = App::new(config, temp_config_path);
    app.mode = Mode::Detail;

    let mock_info = crate::repo::RepoInfo {
        tab_loaded_at: [None; 12],
        tab_loading: [false; 12],
        ..Default::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(mock_info),
    });

    // 1. Initial State: Primary tabs, Workspace (0)
    assert!(!app.advanced_tabs);
    assert_eq!(app.detail_tab, 0);

    // 2. Cycle Forward (should go to Files (1))
    let key_tab = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Tab,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::tabs::route_detail_event(&mut app, key_tab);
    assert!(handled);
    assert_eq!(app.detail_tab, 1);

    // 3. Jump to Tab 7 (Stashes - index 6)
    let key_7 = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('7'),
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::tabs::route_detail_event(&mut app, key_7);
    assert!(handled);
    assert_eq!(app.detail_tab, 6);

    // 4. Cycle Forward from index 6 (should wrap back to 0)
    let handled = crate::tabs::route_detail_event(&mut app, key_tab);
    assert!(handled);
    assert_eq!(app.detail_tab, 0);

    // 5. Toggle Advanced Tabs (by pressing Z)
    let key_z = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('Z'),
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::tabs::route_detail_event(&mut app, key_z);
    assert!(handled);
    assert!(app.advanced_tabs);
    assert_eq!(app.detail_tab, 7); // Worktrees is index 7

    // 6. Jump to Tab 3 of Advanced group (Reflog - index 9)
    let key_3 = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('3'),
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::tabs::route_detail_event(&mut app, key_3);
    assert!(handled);
    assert_eq!(app.detail_tab, 9);

    // 7. Cycle Forward from index 9 (should go to index 10 - Forge)
    let handled = crate::tabs::route_detail_event(&mut app, key_tab);
    assert!(handled);
    assert_eq!(app.detail_tab, 10);

    // 8. Cycle Forward from index 10 (should go to index 11 - PRs)
    let handled = crate::tabs::route_detail_event(&mut app, key_tab);
    assert!(handled);
    assert_eq!(app.detail_tab, 11);

    // 9. Cycle Forward from index 11 (should wrap back to 7 - Worktrees)
    let handled = crate::tabs::route_detail_event(&mut app, key_tab);
    assert!(handled);
    assert_eq!(app.detail_tab, 7);

    // 9. Escape in Advanced mode should return to Primary
    let key_esc = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Esc,
        crossterm::event::KeyModifiers::empty(),
    );
    let handled = crate::tabs::route_detail_event(&mut app, key_esc);
    assert!(handled);
    assert!(!app.advanced_tabs);
    assert_eq!(app.detail_tab, 0);
}

#[test]
fn test_file_history_and_blame_bg_events_apply_and_drop_stale() {
    let (cfg_path, _guard) = temp_config_path("p1_history_blame_bg");
    let mut app = App::new(Config::default(), cfg_path);
    let open_repo = PathBuf::from("/tmp/repo-open");
    let other_repo = PathBuf::from("/tmp/repo-other");

    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: open_repo.clone(),
        info: Box::new(repo::RepoInfo::default()),
    });

    // Stale FileHistoryLoaded when mode or path does not match is ignored
    app.mode = Mode::FileHistory;
    app.file_history_path = "src/main.rs".to_string();
    app.file_history_loading = true;
    app.tx
        .send(BgEvent::FileHistoryLoaded {
            repo_path: open_repo.clone(),
            file_path: "src/other.rs".to_string(),
            result: Ok(vec![repo::FileRevision {
                commit_oid: "1111111".to_string(),
                ..Default::default()
            }]),
        })
        .unwrap();
    assert!(app.drain_bg_events());
    assert!(app.file_history_loading);
    assert!(app.file_history_revisions.is_empty());

    // Matching FileHistoryLoaded populates revisions and clears loading
    app.tx
        .send(BgEvent::FileHistoryLoaded {
            repo_path: open_repo.clone(),
            file_path: "src/main.rs".to_string(),
            result: Ok(vec![repo::FileRevision {
                commit_oid: "2222222".to_string(),
                summary: "feat: async history".to_string(),
                ..Default::default()
            }]),
        })
        .unwrap();
    assert!(app.drain_bg_events());
    assert!(!app.file_history_loading);
    assert_eq!(app.file_history_revisions.len(), 1);
    assert_eq!(app.file_history_revisions[0].commit_oid, "2222222");

    // Stale FileBlameLoaded for another repo or file is ignored
    app.file_tree.show_blame = true;
    app.file_tree.blamed_file_path = Some("src/main.rs".to_string());
    app.file_tree.blame_loading = true;
    app.tx
        .send(BgEvent::FileBlameLoaded {
            repo_path: other_repo,
            file_path: "src/main.rs".to_string(),
            result: Ok(vec![repo::BlameEntry {
                line_no: 1,
                commit_id: "aaaaaaa".to_string(),
                author: "Other".to_string(),
                date: "2026-01-01".to_string(),
            }]),
        })
        .unwrap();
    assert!(app.drain_bg_events());
    assert!(app.file_tree.blame_loading);
    assert!(app.file_tree.file_blame.is_none());

    // Matching FileBlameLoaded stores entries and clears blame_loading
    app.tx
        .send(BgEvent::FileBlameLoaded {
            repo_path: open_repo,
            file_path: "src/main.rs".to_string(),
            result: Ok(vec![repo::BlameEntry {
                line_no: 1,
                commit_id: "bbbbbbb".to_string(),
                author: "Álvaro".to_string(),
                date: "2026-01-02".to_string(),
            }]),
        })
        .unwrap();
    assert!(app.drain_bg_events());
    assert!(!app.file_tree.blame_loading);
    let blame = app.file_tree.file_blame.as_ref().expect("blame should be populated");
    assert_eq!(blame.len(), 1);
    assert_eq!(blame[0].commit_id, "bbbbbbb");
}
