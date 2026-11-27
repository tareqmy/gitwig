#![allow(clippy::unwrap_used, clippy::panic)]

use super::*;

/// A relative config path would put the files `App::new` writes into the
/// checkout, so building a test `App` with one fails before anything is written.
#[test]
#[should_panic(expected = "relative config path")]
fn test_app_refuses_relative_config_path_in_tests() {
    let _ = App::new(Config::default(), PathBuf::from("config.toml"));
}

#[test]
fn test_sorting_logic() {
    let config = Config {
        items: vec!["z_repo".to_string(), "a_repo".to_string(), "m_repo".to_string()],
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
    app.state.visits.insert("a_repo".to_string(), 10);
    app.state.visits.insert("z_repo".to_string(), 20);
    app.state.visits.insert("m_repo".to_string(), 5);

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

    // Case 1: git_only is always enabled — only git repos are included
    app.input_buffer = temp_dir.to_string_lossy().to_string();
    app.commit_bulk_add();

    // Should include repo_a and repo_c, but NOT repo_b (no .git dir)
    assert_eq!(app.config.items.len(), 2);
    assert!(app.config.items.iter().any(|item| item.ends_with("repo_a")));
    assert!(app.config.items.iter().any(|item| item.ends_with("repo_c")));
    assert!(!app.config.items.iter().any(|item| item.ends_with("repo_b")));

    // Clean up
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_add_repo_with_labels_flow() {
    let config = Config {
        items: vec![],
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
    let temp_dir = std::env::temp_dir().join("gitwig_test_labels_flow");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let temp_path = temp_dir.join("config_labels_test.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Test 1: Single add with labels flow
    app.pending_add_repo = Some("/path/to/my_new_repo".to_string());
    app.input_buffer = "work, personal, rust".to_string();
    app.mode = Mode::AddRepoLabelInput;
    app.commit_add_label_input();

    assert_eq!(app.config.items.len(), 1);
    assert_eq!(app.config.items[0], "/path/to/my_new_repo");
    let labels = app.config.labels.get("/path/to/my_new_repo").unwrap();
    assert_eq!(labels, &vec!["work".to_string(), "personal".to_string(), "rust".to_string()]);
    assert_eq!(app.mode, Mode::Normal);

    // Test 2: Bulk add with labels flow
    let bulk_sub_a = temp_dir.join("bulk_sub_a");
    std::fs::create_dir_all(bulk_sub_a.join(".git")).unwrap();

    app.pending_bulk_add_repo = Some(temp_dir.to_string_lossy().to_string());
    app.input_buffer = "shared, source".to_string();
    app.mode = Mode::BulkAddRepoLabelInput;
    app.commit_bulk_add_label_input();

    // Verify sub repo was added and has correct labels
    let target_sub_path = temp_dir.join("bulk_sub_a").to_string_lossy().to_string();
    assert!(app.config.items.iter().any(|x| x == &target_sub_path));
    let labels_bulk = app.config.labels.get(&target_sub_path).unwrap();
    assert_eq!(labels_bulk, &vec!["shared".to_string(), "source".to_string()]);
    assert_eq!(app.mode, Mode::Normal);
    assert!(app.input_buffer.is_empty());

    // Test 3: Clone add with labels flow
    app.pending_add_repo = Some("/path/to/my_cloned_repo".to_string());
    app.input_buffer = "cloned, external".to_string();
    app.mode = Mode::CloneRepoLabelInput;
    app.commit_add_label_input();

    assert!(app.config.items.iter().any(|x| x == "/path/to/my_cloned_repo"));
    let labels_clone = app.config.labels.get("/path/to/my_cloned_repo").unwrap();
    assert_eq!(labels_clone, &vec!["cloned".to_string(), "external".to_string()]);
    assert_eq!(app.mode, Mode::Normal);

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
fn test_repo_search_filtering() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec!["z_repo".to_string(), "a_repo".to_string(), "m_repo".to_string()],
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
fn test_scan_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config {
        items: vec![],
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
fn test_yank_selected_repo_path() {
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
    let (config_path, _guard) = temp_config_path("yank_selected_repo_path");
    let mut app = App::new(config, config_path);
    app.selected_index = 0;

    app.yank_selected_repo_path();
    let success = app.status_message.as_ref().is_some_and(|m| m.contains("Copied path"));
    let failure = app.error_message.as_ref().is_some_and(|m| m.contains("Failed to copy"));
    assert!(success || failure);
}

#[test]
fn test_multi_select_toggle() {
    let config = Config {
        items: vec!["/path/to/repo1".to_string(), "/path/to/repo2".to_string()],
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
    let (config_path, _guard) = temp_config_path("multi_select_toggle");
    let mut app = App::new(config, config_path);
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
fn test_repository_labels_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let config = Config {
        items: vec!["/path/to/repo1".to_string(), "/path/to/repo2".to_string()],
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
    assert_eq!(app.config.view_mode, crate::config::HomeViewMode::Normal);

    // Simulate event toggle
    let event = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('v'),
        crossterm::event::KeyModifiers::empty(),
    );
    let _ = crate::tabs::HomeTab::handle_event(&mut app, event, 10);
    assert_eq!(app.config.view_mode, crate::config::HomeViewMode::Compact);

    let _ = crate::tabs::HomeTab::handle_event(&mut app, event, 10);
    assert_eq!(app.config.view_mode, crate::config::HomeViewMode::Tile);
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
fn test_repo_jump_flow_filters_stale() {
    let config = Config {
        items: vec![
            "/path/to/alpha".to_string(),
            "/path/to/beta".to_string(),
            "/path/to/gamma".to_string(),
        ],
        show_stale_projects: false,
        stale_threshold_months: 1,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_repo_jump_stale.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Mock summary with a very old commit time for "/path/to/beta" to make it stale
    let mut summary = crate::repo::RepoSummary::default();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    summary.last_commit_time = Some((now - 7776000) as i64); // 3 months ago

    app.statuses[1] = crate::repo::ItemStatus::GitRepo(Some(summary));

    // Since show_stale_projects is false, "/path/to/beta" (index 1) should be hidden, and not searchable
    let matches = app.get_jump_matches();
    // Should only contain "/path/to/alpha" (index 0) and "/path/to/gamma" (index 2)
    assert_eq!(matches.len(), 2);
    assert!(matches.iter().all(|m| m.0 != 1));
}

#[test]
fn test_mru_group_flow() {
    let temp_dir = std::env::temp_dir().join("gitwig_test_mru_repos");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let alpha = temp_dir.join("alpha");
    let beta = temp_dir.join("beta");
    let gamma = temp_dir.join("gamma");

    std::fs::create_dir_all(alpha.join(".git")).unwrap();
    std::fs::create_dir_all(beta.join(".git")).unwrap();
    std::fs::create_dir_all(gamma.join(".git")).unwrap();

    let config = Config {
        items: vec![
            alpha.to_string_lossy().to_string(),
            beta.to_string_lossy().to_string(),
            gamma.to_string_lossy().to_string(),
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

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Opening a repository records a visit, which lifts it into the Recent group
/// and shifts the rows below. The Detail header and the home cursor must stay
/// on the repository that was opened, not on whatever row slid into the old
/// cursor index.
#[test]
fn test_open_detail_title_and_cursor_follow_the_opened_repo() {
    let (mut app, items, _guard) = app_with_three_repos("open_detail_title");
    assert_eq!(app.get_home_rows().len(), 3);

    // gamma is the third row; after the visit the rows are
    // [Recent, gamma, alpha, beta, gamma], so index 2 would be alpha.
    app.selected_index = 2;
    app.open_detail();

    assert_eq!(app.get_home_rows().len(), 5);
    assert_eq!(app.active_repo_item(), Some(&items[2]));
    assert_eq!(app.get_selected_item(), Some(&items[2]));
    assert_eq!(app.selected_index, 4, "cursor stays in the group it was opened from");
}

/// `open_repo` opens the entry it is given, whatever row the home cursor is
/// on. The Worktrees tab used to write a `config.items` index into the
/// home-row cursor and open whichever row that landed on.
#[test]
fn test_open_repo_opens_the_given_item_not_the_cursor_row() {
    let (mut app, items, _guard) = app_with_three_repos("open_repo_item");
    // Label groups reorder the rows: [apple, gamma, zeta, alpha, Unlabeled, beta].
    app.config.labels.insert(items[0].clone(), vec!["zeta".to_string()]);
    app.config.labels.insert(items[2].clone(), vec!["apple".to_string()]);
    app.selected_index = 1;
    assert_eq!(app.get_selected_item(), Some(&items[2]));

    app.open_repo(items[1].clone());

    assert_eq!(app.loading_repo_path.as_ref(), Some(&items[1]));
    assert_eq!(app.active_repo_item(), Some(&items[1]));
    assert_eq!(app.get_selected_item(), Some(&items[1]));
}

/// The `.git` guard in `open_repo` must see the expanded path: an item
/// written as `~/repo` used to fall into `NotGitRepo` every time, even
/// though the home list loaded its status fine.
#[test]
fn test_open_repo_expands_tilde_before_the_git_guard() {
    let Some(home) = dirs::home_dir() else {
        return;
    };
    let name = "gitwig_test_open_repo_tilde";
    let dir = home.join(name);
    let _ = std::fs::remove_dir_all(&dir);
    // `~` can only be exercised with a fixture under the real home directory;
    // skip where the home directory is not writable (sandboxed runs).
    if std::fs::create_dir_all(dir.join(".git")).is_err() {
        return;
    }
    let _guard = TestDirGuard { path: dir.clone() };

    let tilde_item = format!("~/{}", name);
    let config = Config { items: vec![tilde_item.clone()], ..Default::default() };
    let mut app = App::new(config, dir.join("config.toml"));

    app.open_repo(tilde_item.clone());
    assert_ne!(app.mode, Mode::NotGitRepo, "a `~` item with a .git directory is a repository");
    assert_eq!(app.loading_repo_path.as_ref(), Some(&tilde_item));
    assert!(app.state.visits.contains_key(&tilde_item));

    // The guard still rejects a directory without `.git`, tilde or not.
    let plain = home.join("gitwig_test_open_repo_tilde_plain");
    let _ = std::fs::remove_dir_all(&plain);
    std::fs::create_dir_all(&plain).unwrap();
    let _plain_guard = TestDirGuard { path: plain };
    app.mode = Mode::Normal;
    app.open_repo("~/gitwig_test_open_repo_tilde_plain".to_string());
    assert_eq!(app.mode, Mode::NotGitRepo);
}

/// Once a repository is open, "which repository" comes from the open
/// snapshot, whatever the home cursor points at. Items stored with `~` match
/// the expanded path the snapshot carries.
#[test]
fn test_active_repo_item_follows_the_open_repo_not_the_cursor() {
    let (mut app, mut items, _guard) = app_with_three_repos("active_repo_item");
    items[1] = "~/gitwig_test_tilde_repo".to_string();
    app.config.items = items.clone();
    app.config.show_grouping = false;

    app.selected_index = 0;
    assert_eq!(app.active_repo_item(), Some(&items[0]), "home screen: the cursor");

    app.loading_repo_path = Some(items[1].clone());
    assert_eq!(app.active_repo_item(), Some(&items[1]), "loading: the repo being opened");

    app.loading_repo_path = None;
    app.current_detail =
        Some(crate::repo::ItemDetail::Directory { resolved: crate::repo::expand_tilde(&items[1]) });
    assert_eq!(app.active_repo_item(), Some(&items[1]), "open: the snapshot");

    app.loading_repo_path = Some(items[2].clone());
    assert_eq!(app.active_repo_item(), Some(&items[2]), "a load supersedes the old snapshot");
    app.loading_repo_path = None;

    app.current_detail = Some(crate::repo::ItemDetail::Directory {
        resolved: std::path::PathBuf::from("/untracked/elsewhere"),
    });
    assert_eq!(app.active_repo_item(), None, "never falls back to the cursor while open");
}

/// Sorting, pinning and starring reorder the home rows. The cursor must be
/// put back by home row, in the group it was on; these used to write the
/// repository's filtered-list position into the home-row cursor, which with
/// grouping on lands on a neighbour or a group header.
#[test]
fn test_sort_pin_and_star_keep_the_cursor_on_the_repo_when_grouped() {
    let (mut app, items, _guard) = app_with_three_repos("reorder_cursor");
    group_three_repos_under_work(&mut app, &items);
    app.selected_index = 3;
    let alpha_in_work = Some((items[0].clone(), "work".to_string()));
    assert_eq!(app.home_cursor(), alpha_in_work);

    app.cycle_sort_order();
    assert_eq!(app.config.sort_by, SortOrder::Alphabetical);
    assert_eq!(app.home_cursor(), alpha_in_work, "cycle sort order");

    // [Recent, gamma, work, gamma, beta, alpha]
    app.toggle_sort_reverse();
    assert_eq!(app.selected_index, 5);
    assert_eq!(app.home_cursor(), alpha_in_work, "toggle sort direction");

    // [Recent, gamma, work, alpha, gamma, beta]
    app.toggle_pin_selected();
    assert!(app.config.pinned.contains(&items[0]));
    assert_eq!(app.home_cursor(), alpha_in_work, "pin");

    // [Recent, gamma, Starred, alpha, work, alpha, gamma, beta]
    app.toggle_star_selected();
    assert!(app.config.starred.contains(&items[0]));
    assert_eq!(app.home_cursor(), alpha_in_work, "star keeps the group it was starred from");

    // Unstarring from the Starred group removes that row; the cursor follows
    // the repository to its remaining one.
    app.select_home_row(&items[0], Some("Starred"));
    assert_eq!(app.selected_index, 3);
    app.toggle_star_selected();
    assert_eq!(app.home_cursor(), alpha_in_work, "unstar from the Starred group");
}

/// Changing the sort of the open label view (Label Settings) re-sorts through
/// `resort_if_sort_changed`, which must keep the cursor on the same row.
#[test]
fn test_label_sort_change_keeps_the_cursor_on_the_repo_when_grouped() {
    let (mut app, items, _guard) = app_with_three_repos("label_sort_cursor");
    group_three_repos_under_work(&mut app, &items);
    app.state.active_label_filter = Some("work".to_string());
    app.selected_index = 3;
    assert_eq!(app.home_cursor(), Some((items[0].clone(), "work".to_string())));

    let before = app.effective_sort();
    app.config.label_configs.insert(
        "work".to_string(),
        crate::config::LabelConfig { sort_reverse: Some(true), ..Default::default() },
    );
    app.resort_if_sort_changed(before);

    // [Recent, gamma, work, gamma, beta, alpha]
    assert_eq!(app.selected_index, 5);
    assert_eq!(app.home_cursor(), Some((items[0].clone(), "work".to_string())));
}

/// Each add flow selects the new repository by its home row, expanding its
/// label group when collapsed. They used to write its `config.items`
/// position into the home-row cursor.
#[test]
fn test_adding_repos_selects_the_new_row_when_grouped() {
    let (mut app, items, guard) = app_with_three_repos("add_cursor");
    app.config.labels.insert(items[0].clone(), vec!["work".to_string()]);
    app.state.visits.insert(items[2].clone(), 1);
    // [Recent, gamma, work, alpha, Unlabeled, beta, gamma]

    let delta = guard.path.join("delta").to_string_lossy().to_string();
    app.commit_add_with_labels(delta.clone(), vec!["work".to_string()]);
    assert_eq!(app.home_cursor(), Some((delta, "work".to_string())), "add with labels");

    app.collapsed_groups.insert("Unlabeled".to_string());
    let epsilon = guard.path.join("epsilon").to_string_lossy().to_string();
    app.add_repo_path(epsilon.clone());
    assert!(!app.collapsed_groups.contains("Unlabeled"), "the new repo's group is expanded");
    assert_eq!(app.home_cursor(), Some((epsilon, "Unlabeled".to_string())), "add path");

    let bulk_dir = guard.path.join("more");
    for name in ["one", "two"] {
        std::fs::create_dir_all(bulk_dir.join(name).join(".git")).unwrap();
    }
    let bulk_dir = bulk_dir.to_string_lossy().to_string();
    app.collapsed_groups.insert("zeta".to_string());
    app.bulk_add_path_with_labels(bulk_dir.clone(), vec!["zeta".to_string()]);
    let one = format!("{}{}one", bulk_dir, std::path::MAIN_SEPARATOR);
    assert!(!app.collapsed_groups.contains("zeta"));
    assert_eq!(app.home_cursor(), Some((one, "zeta".to_string())), "bulk add");
}

/// Editing a repository's path keeps the cursor on it, in the same group.
#[test]
fn test_editing_a_repo_path_keeps_the_cursor_on_it_when_grouped() {
    let (mut app, items, guard) = app_with_three_repos("edit_cursor");
    app.state.visits.insert(items[2].clone(), 1);
    // [Recent, gamma, alpha, beta, gamma]
    app.selected_index = 4;
    assert_eq!(app.get_selected_item(), Some(&items[2]));

    let renamed = guard.path.join("gamma-renamed").to_string_lossy().to_string();
    app.set_input_buffer(renamed.clone());
    app.commit_edit();

    assert_eq!(app.selected_index, 4);
    assert_eq!(app.home_cursor(), Some((renamed, String::new())));
}

/// Editing a repository's path moves everything keyed by the old path to the
/// new one — labels, star, pin, per-repo settings, commit history and batch
/// selection — so the repository stays in its label group instead of
/// dropping into Unlabeled, and config.toml keeps nothing under the old path.
#[test]
fn test_editing_a_repo_path_migrates_everything_keyed_by_it() {
    let (mut app, items, guard) = app_with_three_repos("edit_migrates_keys");
    let old = items[2].clone();
    let labels = vec!["work".to_string()];
    let repo_cfg =
        RepoConfig { note: Some("deploys on Fridays".to_string()), ..Default::default() };
    app.config.labels.insert(old.clone(), labels.clone());
    app.config.starred.insert(old.clone());
    app.config.pinned.insert(old.clone());
    app.config.repo_configs.insert(old.clone(), repo_cfg.clone());
    app.state.record_commit_message(&old, "feat: first");
    app.multi_selected.insert(old.clone());
    app.select_home_row(&old, Some("work"));
    assert_eq!(app.home_cursor(), Some((old.clone(), "work".to_string())));

    let renamed = guard.path.join("delta").to_string_lossy().to_string();
    app.set_input_buffer(renamed.clone());
    app.commit_edit();

    assert_eq!(app.home_cursor(), Some((renamed.clone(), "work".to_string())));
    assert_eq!(app.config.labels.get(&renamed), Some(&labels));
    assert!(app.config.starred.contains(&renamed));
    assert!(app.config.pinned.contains(&renamed));
    assert_eq!(app.config.repo_configs.get(&renamed), Some(&repo_cfg));
    assert_eq!(app.state.commit_history_for(&renamed), vec!["feat: first".to_string()]);
    assert!(app.multi_selected.contains(&renamed));

    assert!(!app.config.labels.contains_key(&old));
    assert!(!app.config.starred.contains(&old));
    assert!(!app.config.pinned.contains(&old));
    assert!(!app.config.repo_configs.contains_key(&old));
    assert!(!app.state.commit_history.contains_key(&old));
    assert!(!app.multi_selected.contains(&old));

    // The saved files agree, with no orphaned entries left under the old path.
    let config_path = guard.path.join("config.toml");
    assert!(!std::fs::read_to_string(&config_path).unwrap().contains(&old));
    let (config, state, _, _) = crate::config::load_config(Some(config_path)).unwrap();
    assert_eq!(config.labels.get(&renamed), Some(&labels));
    assert!(config.starred.contains(&renamed));
    assert_eq!(config.repo_configs.get(&renamed), Some(&repo_cfg));
    assert_eq!(state.commit_history_for(&renamed), vec!["feat: first".to_string()]);
    assert!(!state.commit_history.contains_key(&old));
}

/// Editing a repository's path to one another entry already tracks — as
/// written or spelled differently — is refused and changes nothing: it would
/// duplicate the entry in `config.items`, and the migration would overwrite
/// the other repository's labels and settings with this one's. Another
/// spelling of the edited repository's own path is still a valid edit.
#[test]
fn test_editing_a_repo_path_to_another_tracked_repo_is_refused() {
    fn edit(app: &mut App, from: &str, group: &str, to: &str) {
        app.select_home_row(from, Some(group));
        app.start_edit();
        app.set_input_buffer(to.to_string());
        app.status_message = None;
        app.commit_edit();
    }

    let (mut app, items, guard) = app_with_three_repos("edit_path_clash");
    let (beta, gamma) = (items[1].clone(), items[2].clone());
    let beta_labels = vec!["b".to_string()];
    let gamma_labels = vec!["g".to_string()];
    let beta_cfg = RepoConfig { note: Some("beta".to_string()), ..Default::default() };
    let gamma_cfg = RepoConfig { note: Some("gamma".to_string()), ..Default::default() };
    app.config.labels.insert(beta.clone(), beta_labels.clone());
    app.config.labels.insert(gamma.clone(), gamma_labels.clone());
    app.config.repo_configs.insert(beta.clone(), beta_cfg.clone());
    app.config.repo_configs.insert(gamma.clone(), gamma_cfg.clone());
    let items_before = app.config.items.clone();
    let config_path = guard.path.join("config.toml");
    let saved_before = std::fs::read_to_string(&config_path).ok();

    // beta's path as written, with a trailing slash, and canonicalized
    // (`/private/var/...` for macOS temp dirs).
    let beta_canonical = std::fs::canonicalize(&beta).unwrap().to_string_lossy().to_string();
    for clash in [beta.clone(), format!("{}/", beta), beta_canonical] {
        edit(&mut app, &gamma, "g", &clash);
        assert_eq!(app.status_message.as_deref(), Some("Repository already added"), "{}", clash);
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.input_buffer.is_empty());
        assert_eq!(app.config.items, items_before);
        assert_eq!(app.config.labels.get(&beta), Some(&beta_labels));
        assert_eq!(app.config.labels.get(&gamma), Some(&gamma_labels));
        assert_eq!(app.config.repo_configs.get(&beta), Some(&beta_cfg));
        assert_eq!(app.config.repo_configs.get(&gamma), Some(&gamma_cfg));
        assert_eq!(app.home_cursor(), Some((gamma.clone(), "g".to_string())));
        assert_eq!(std::fs::read_to_string(&config_path).ok(), saved_before, "nothing saved");
    }

    // gamma's own path, unchanged and then with a trailing slash, is accepted.
    let gamma_slash = format!("{}/", gamma);
    for (from, to) in [(&gamma, &gamma), (&gamma, &gamma_slash)] {
        edit(&mut app, from, "g", to);
        assert_eq!(app.status_message.as_deref(), Some("Saved"), "{}", to);
        assert_eq!(app.config.labels.get(to), Some(&gamma_labels));
        assert_eq!(app.config.repo_configs.get(to), Some(&gamma_cfg));
    }
    assert_eq!(app.config.items.len(), 3);
    assert!(app.config.items.contains(&gamma_slash));
    assert!(!app.config.items.contains(&gamma));

    // `~/x` and its expanded form are one repository: renaming between them
    // is fine for that repository and refused for any other.
    if let Some(home) = dirs::home_dir() {
        let tilde = "~/gitwig_test_edit_path_clash".to_string();
        let expanded = home.join("gitwig_test_edit_path_clash").to_string_lossy().to_string();
        edit(&mut app, &gamma_slash, "g", &tilde);
        assert_eq!(app.status_message.as_deref(), Some("Saved"));
        edit(&mut app, &tilde, "g", &expanded);
        assert_eq!(app.status_message.as_deref(), Some("Saved"));
        assert_eq!(app.config.labels.get(&expanded), Some(&gamma_labels));

        edit(&mut app, &beta, "b", &tilde);
        assert_eq!(app.status_message.as_deref(), Some("Repository already added"));
        assert_eq!(app.config.items.len(), 3);
        assert!(app.config.items.contains(&beta));
        assert_eq!(app.config.labels.get(&beta), Some(&beta_labels));
        assert_eq!(app.config.labels.get(&expanded), Some(&gamma_labels));
        assert_eq!(app.config.repo_configs.get(&expanded), Some(&gamma_cfg));
    }

    // Nothing was lost along the way.
    assert_eq!(app.config.repo_configs.get(&beta), Some(&beta_cfg));
}

/// A repository found in the background must not move the cursor off the
/// repository the user had selected.
#[test]
fn test_auto_discovered_repo_leaves_the_cursor_on_the_selected_repo() {
    let (mut app, items, guard) = app_with_three_repos("auto_discover_cursor");
    app.config.sort_by = SortOrder::Alphabetical;
    app.config.labels.insert(items[0].clone(), vec!["work".to_string()]);
    // [work, alpha, Unlabeled, beta, gamma]
    app.selected_index = 3;
    assert_eq!(app.get_selected_item(), Some(&items[1]));

    // Sorts ahead of beta, so its row takes the cursor's index.
    app.auto_discover_add(guard.path.join("aardvark").to_string_lossy().to_string());

    assert_eq!(app.selected_index, 4);
    assert_eq!(app.home_cursor(), Some((items[1].clone(), "Unlabeled".to_string())));
}

/// Enter on a worktree opened it but left `advanced_tabs` set with tab 0:
/// no tab was highlighted, Shift+Tab computed `0 - 7` (a panic in debug
/// builds) and `q` needed pressing twice. Its new home row was pushed to
/// `config.items` without a status, so it read "missing". A worktree already
/// tracked under another spelling was added again.
#[test]
fn test_opening_a_worktree_starts_in_the_primary_tabs_with_a_real_home_row() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let root = std::env::temp_dir().join("gitwig_test_open_worktree");
    let _ = std::fs::remove_dir_all(&root);
    let _guard = TestDirGuard { path: root.clone() };
    let main = root.join("main");
    let wt = root.join("wt");
    std::fs::create_dir_all(&main).unwrap();
    run_git_in(&main, &["init", "-q"]);
    run_git_in(&main, &["commit", "-q", "--allow-empty", "-m", "init"]);
    run_git_in(&main, &["worktree", "add", "-q", "-b", "wt-branch", &wt.to_string_lossy()]);

    let main_item = main.to_string_lossy().to_string();
    let config = Config { items: vec![main_item.clone()], ..Default::default() };
    let mut app = App::new(config, root.join("config.toml"));
    let open_worktrees_tab = |app: &mut App| {
        let info = repo::RepoInfo {
            worktrees: repo::TabData::Loaded(repo::load_tab_worktrees(&main).unwrap()),
            ..Default::default()
        };
        app.current_detail =
            Some(repo::ItemDetail::Repo { resolved: main.clone(), info: Box::new(info) });
        app.loading_repo_path = None;
        app.mode = Mode::Detail;
        app.advanced_tabs = true;
        app.detail_tab = 7;
        app.detail_focus = DetailSection::Worktrees;
        app.worktree_selection = 0;
    };

    open_worktrees_tab(&mut app);
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert!(!app.advanced_tabs, "the worktree opens in the Primary tab group");
    assert_eq!(app.detail_tab, 0);
    assert_eq!(app.config.items.len(), 2);
    assert_eq!(app.statuses.len(), app.config.items.len(), "the new row has a status");
    let wt_idx = app.config.items.iter().position(|i| std::path::Path::new(i).ends_with("wt"));
    let wt_idx = wt_idx.expect("the worktree is tracked");
    assert!(
        !matches!(app.statuses[wt_idx], repo::ItemStatus::Missing),
        "the worktree's row is not 'missing'"
    );

    // Tab cycling from the opened view stays in the Primary group.
    app.loading_repo_path = None;
    assert!(crate::input::handle_key(&mut app, key(KeyCode::BackTab), 1));
    assert_eq!(app.detail_tab, 6);
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Tab), 1));
    assert_eq!(app.detail_tab, 0);

    // Even a stale Advanced flag with a Primary index cannot underflow.
    app.advanced_tabs = true;
    app.detail_tab = 0;
    assert!(crate::input::handle_key(&mut app, key(KeyCode::BackTab), 1));
    assert!((7..=11).contains(&app.detail_tab));
    app.detail_tab = 0;
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Tab), 1));
    assert!((7..=11).contains(&app.detail_tab));

    // Tracked under another spelling: opened, not added a second time.
    let other_spelling = format!("{}/", app.config.items[wt_idx]);
    app.config.items[wt_idx] = other_spelling.clone();
    open_worktrees_tab(&mut app);
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert_eq!(app.config.items.len(), 2, "no duplicate entry");
    assert_eq!(app.active_repo_item(), Some(&other_spelling));
}

/// Sorted by Latest Changes, the home list only re-sorted when something else
/// did (changing the sort, adding a repository), so a new commit left its
/// repository where it was; and a refresh that landed after a re-sort was
/// dropped, since its row indices no longer matched.
#[test]
fn test_latest_changes_sort_follows_refreshed_commits() {
    let root = std::env::temp_dir().join("gitwig_test_latest_changes_sort");
    let _ = std::fs::remove_dir_all(&root);
    let _guard = TestDirGuard { path: root.clone() };
    let commit_at = |dir: &std::path::Path, date: &str| {
        let out = crate::git_cmd::git_command()
            .args(["-c", "user.name=Gitwig Test", "-c", "user.email=test@gitwig.invalid"])
            .args(["-c", "commit.gpgsign=false", "commit", "-q", "--allow-empty", "-m", date])
            .env("GIT_AUTHOR_DATE", date)
            .env("GIT_COMMITTER_DATE", date)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    };
    let [alpha, beta, gamma] = ["alpha", "beta", "gamma"].map(|name| {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        run_git_in(&dir, &["init", "-q"]);
        dir.to_string_lossy().to_string()
    });
    commit_at(std::path::Path::new(&alpha), "2020-01-01T00:00:00Z");
    commit_at(std::path::Path::new(&beta), "2021-01-01T00:00:00Z");
    commit_at(std::path::Path::new(&gamma), "2022-01-01T00:00:00Z");

    let config = Config {
        items: vec![alpha.clone(), beta.clone(), gamma.clone()],
        sort_by: SortOrder::LatestChanges,
        ..Default::default()
    };
    let mut app = App::new(config, root.join("config.toml"));
    assert_eq!(app.config.items, [gamma.clone(), beta.clone(), alpha.clone()]);
    app.select_home_row(&beta, None);
    assert_eq!(app.home_cursor().map(|(path, _)| path), Some(beta.clone()));

    // A refresh started now carries today's row indices.
    let stale_rows: Vec<(usize, String)> = app.config.items.iter().cloned().enumerate().collect();

    // alpha gets the newest commit; the next refresh moves it to the top.
    commit_at(std::path::Path::new(&alpha), "2023-01-01T00:00:00Z");
    let refresh = app
        .config
        .items
        .iter()
        .enumerate()
        .map(|(idx, path)| (idx, path.clone(), repo::inspect_summary(path)))
        .collect();
    app.status_refresh_tx.send(refresh).unwrap();
    assert!(app.drain_status_refreshes());
    assert_eq!(app.config.items, [alpha.clone(), gamma.clone(), beta.clone()]);
    assert_eq!(app.statuses.len(), app.config.items.len());
    assert_eq!(
        app.home_cursor().map(|(path, _)| path),
        Some(beta.clone()),
        "the cursor stays on its repository"
    );

    // A batch using the indices from before that re-sort still reaches the
    // right repositories: beta, at row 1 then, is at row 2 now.
    let late: Vec<(usize, String, repo::ItemStatus)> = stale_rows
        .into_iter()
        .map(|(idx, path)| {
            let status =
                if path == beta { repo::ItemStatus::Missing } else { repo::inspect_summary(&path) };
            (idx, path, status)
        })
        .collect();
    app.status_refresh_tx.send(late).unwrap();
    app.drain_status_refreshes();
    let status_of = |app: &App, path: &str| {
        let idx = app.config.items.iter().position(|item| item == path).unwrap();
        app.statuses[idx].clone()
    };
    assert!(matches!(status_of(&app, &beta), repo::ItemStatus::Missing));
    assert!(matches!(status_of(&app, &gamma), repo::ItemStatus::GitRepo(Some(_))));

    // Other sort orders are left alone by refreshes.
    app.config.sort_by = SortOrder::Alphabetical;
    app.sort_items_in_place();
    commit_at(std::path::Path::new(&gamma), "2024-01-01T00:00:00Z");
    let refresh = app
        .config
        .items
        .iter()
        .enumerate()
        .map(|(idx, path)| (idx, path.clone(), repo::inspect_summary(path)))
        .collect();
    app.status_refresh_tx.send(refresh).unwrap();
    app.drain_status_refreshes();
    assert_eq!(app.config.items, [alpha, beta, gamma]);
}

/// After `git rebase -i` or `git mergetool` the open repository's home card
/// is refreshed. The slot used to be `statuses[selected_index]`, filled from
/// `config.items[selected_index]`: with grouping on, another repository's.
#[test]
fn test_refresh_active_repo_status_updates_the_open_repo_when_grouped() {
    let (mut app, items, _guard) = app_with_three_repos("active_repo_status");
    group_three_repos_under_work(&mut app, &items);
    app.selected_index = 1;
    app.current_detail = Some(repo::ItemDetail::Directory { resolved: PathBuf::from(&items[2]) });
    app.statuses = vec![repo::ItemStatus::Missing; 3];

    app.refresh_active_repo_status();

    assert!(matches!(app.statuses[2], repo::ItemStatus::GitRepo(_)), "gamma is refreshed");
    assert!(matches!(app.statuses[0], repo::ItemStatus::Missing));
    assert!(matches!(app.statuses[1], repo::ItemStatus::Missing), "beta is left alone");
}

/// The git app and the external shell start in the repository under the home
/// cursor. They used to take `config.items[selected_index]`, which with
/// grouping on is a different repository, and a group header's index still
/// named one.
#[test]
fn test_git_app_and_shell_open_the_repo_under_the_home_cursor_when_grouped() {
    let (mut app, items, _guard) = app_with_three_repos("home_cursor_targets");
    group_three_repos_under_work(&mut app, &items);

    app.selected_index = 1;
    assert_eq!(app.git_app_dir(), Some(repo::expand_tilde(&items[2])));
    assert_eq!(app.take_shell_targets(), vec![items[2].clone()]);

    // The `work` group header.
    app.selected_index = 2;
    assert_eq!(app.git_app_dir(), None);
    assert!(app.take_shell_targets().is_empty());

    app.multi_selected.insert(items[0].clone());
    assert_eq!(app.take_shell_targets(), vec![items[0].clone()]);
    assert!(app.multi_selected.is_empty(), "the multi-selection is consumed");

    // A search narrows the flat list too: row 0 is gamma, `config.items[0]` alpha.
    app.config.show_grouping = false;
    app.repo_search_query = Some("gamma".to_string());
    app.selected_index = 0;
    assert_eq!(app.git_app_dir(), Some(repo::expand_tilde(&items[2])));
    assert_eq!(app.take_shell_targets(), vec![items[2].clone()]);
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

    // Drain any startup background messages first
    while app.status_refresh_rx.try_recv().is_ok() {}

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
fn test_cycle_global_filter() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let config = Config { items: vec!["/path/to/repo_a".to_string()], ..Default::default() };
    let temp_path = std::env::temp_dir().join("gitwig_test_cycle_global_filter.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Tab walks repos → dirty → ahead → stale → archived → repos.
    let tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::empty());
    assert_eq!(app.global_filter, None);
    assert!(crate::input::handle_key(&mut app, tab, 1));
    assert_eq!(app.global_filter, Some(GlobalFilter::Dirty));
    assert!(crate::input::handle_key(&mut app, tab, 1));
    assert_eq!(app.global_filter, Some(GlobalFilter::Ahead));
    assert!(crate::input::handle_key(&mut app, tab, 1));
    assert_eq!(app.global_filter, Some(GlobalFilter::Stale));
    assert!(crate::input::handle_key(&mut app, tab, 1));
    assert_eq!(app.global_filter, Some(GlobalFilter::Archived));
    assert!(crate::input::handle_key(&mut app, tab, 1));
    assert_eq!(app.global_filter, None);

    // Shift+Tab (BackTab) walks the same ring in reverse.
    let back_tab = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
    assert!(crate::input::handle_key(&mut app, back_tab, 1));
    assert_eq!(app.global_filter, Some(GlobalFilter::Archived));
    assert!(crate::input::handle_key(&mut app, back_tab, 1));
    assert_eq!(app.global_filter, Some(GlobalFilter::Stale));
    assert!(crate::input::handle_key(&mut app, back_tab, 1));
    assert_eq!(app.global_filter, Some(GlobalFilter::Ahead));
    assert!(crate::input::handle_key(&mut app, back_tab, 1));
    assert_eq!(app.global_filter, Some(GlobalFilter::Dirty));
    assert!(crate::input::handle_key(&mut app, back_tab, 1));
    assert_eq!(app.global_filter, None);

    // Cycling resets the selection to the top of the newly filtered list.
    app.selected_index = 5;
    app.scroll_top = 3;
    app.cycle_global_filter(false);
    assert_eq!(app.global_filter, Some(GlobalFilter::Dirty));
    assert_eq!(app.selected_index, 0);
    assert_eq!(app.scroll_top, 0);
}

#[test]
fn test_label_filter_project_view() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let mut config = Config {
        items: vec![
            "/path/to/frontend".to_string(),
            "/path/to/backend".to_string(),
            "/path/to/tools".to_string(),
        ],
        ..Default::default()
    };
    config.labels.insert("/path/to/frontend".to_string(), vec!["web".to_string()]);
    config
        .labels
        .insert("/path/to/backend".to_string(), vec!["web".to_string(), "api".to_string()]);
    let temp_path = std::env::temp_dir().join("gitwig_test_label_filter.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    let key = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    // `L` opens the picker.
    let l_key = KeyEvent::new(KeyCode::Char('L'), KeyModifiers::SHIFT);
    assert!(crate::input::handle_key(&mut app, l_key, 1));
    assert_eq!(app.mode, Mode::LabelPicker);

    // Rows: "All repositories" first, then labels alphabetically with counts.
    let matches = app.get_label_matches();
    assert_eq!(matches.len(), 3);
    assert_eq!(matches[0], (None, 3));
    assert_eq!(matches[1], (Some("api".to_string()), 1));
    assert_eq!(matches[2], (Some("web".to_string()), 2));

    // Typing narrows the label rows (the All row stays).
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Char('w')), 1));
    let matches = app.get_label_matches();
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[1].0.as_deref(), Some("web"));

    // Enter on "web" applies the sticky filter and closes the picker.
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Down), 1));
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.state.active_label_filter.as_deref(), Some("web"));
    assert_eq!(app.get_filtered_items().len(), 2);

    // Esc does NOT clear the label filter — it is sticky by design.
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Esc), 1));
    assert_eq!(app.state.active_label_filter.as_deref(), Some("web"));

    // Composes with the summary filter as an intersection.
    let dirty = crate::repo::RepoSummary {
        branch: Some("main".to_string()),
        staged: 1,
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
    let clean = crate::repo::RepoSummary { staged: 0, ..dirty.clone() };
    app.statuses = vec![
        crate::repo::ItemStatus::GitRepo(Some(dirty)), // frontend: web, dirty
        crate::repo::ItemStatus::GitRepo(Some(clean.clone())), // backend: web+api, clean
        crate::repo::ItemStatus::GitRepo(Some(clean)), // tools: unlabeled, clean
    ];
    app.global_filter = Some(GlobalFilter::Dirty);
    let visible = app.get_filtered_items();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].1, "/path/to/frontend");
    app.global_filter = None;

    // Re-selecting the active label in the picker toggles the filter off.
    assert!(crate::input::handle_key(&mut app, l_key, 1));
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Down), 1));
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Down), 1));
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert_eq!(app.state.active_label_filter, None);
    assert_eq!(app.get_filtered_items().len(), 3);

    // The "All repositories" row clears an active filter too.
    app.state.active_label_filter = Some("web".to_string());
    assert!(crate::input::handle_key(&mut app, l_key, 1));
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert_eq!(app.state.active_label_filter, None);

    // Esc closes the picker without touching the filter.
    app.state.active_label_filter = Some("web".to_string());
    assert!(crate::input::handle_key(&mut app, l_key, 1));
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Esc), 1));
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.state.active_label_filter.as_deref(), Some("web"));

    // Auto-clear when the active label no longer exists on any repository.
    app.state.active_label_filter = Some("api".to_string());
    app.config.labels.insert("/path/to/backend".to_string(), vec!["web".to_string()]);
    assert!(app.clear_label_filter_if_orphaned());
    assert_eq!(app.state.active_label_filter, None);

    // The filter persists through a state serialization roundtrip.
    app.state.active_label_filter = Some("web".to_string());
    let serialized = toml::to_string(&app.state).unwrap();
    let loaded: AppState = toml::from_str(&serialized).unwrap();
    assert_eq!(loaded.active_label_filter.as_deref(), Some("web"));
}

#[test]
fn test_quick_labels_fill_slots_fifo_and_evict_oldest_past_nine() {
    let mut config = Config {
        items: (0..12).map(|i| format!("/path/to/repo{}", i)).collect(),
        ..Default::default()
    };
    for i in 0..12 {
        config.labels.insert(format!("/path/to/repo{}", i), vec![format!("l{}", i)]);
    }
    config.labels.entry("/path/to/repo0".to_string()).or_default().push("l1".to_string());
    let temp_dir = std::env::temp_dir().join(format!(
        "gitwig_test_quick_labels_fifo_{}_{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let _guard = TestDirGuard { path: temp_dir.clone() };
    let mut app = App::new(config, temp_dir.join("config.toml"));
    // The constructor's first-run version check writes config.toml once;
    // viewing labels afterwards must never rewrite it.
    let config_before = std::fs::read_to_string(&app.config_path).unwrap();

    // Initially clean: labels exist, but none has been viewed yet.
    assert!(app.quick_labels().is_empty());
    assert_eq!(app.quick_label_slot("l0"), None);

    // The first label viewed takes slot 1, the next takes slot 2, and so on.
    app.select_label_filter(Some("l0".to_string()));
    app.select_label_filter(Some("l1".to_string()));
    let names =
        |app: &App| -> Vec<String> { app.quick_labels().into_iter().map(|(l, _)| l).collect() };
    assert_eq!(names(&app), vec!["l0", "l1"]);
    assert_eq!(app.quick_labels()[1], ("l1".to_string(), 2));
    assert_eq!(app.quick_label_slot("l1"), Some(1));

    // Viewing a label that already holds a slot never reorders the strip.
    app.select_label_filter(Some("l1".to_string()));
    app.select_label_filter(Some("l0".to_string()));
    assert_eq!(names(&app), vec!["l0", "l1"]);

    // Clearing the filter (toggle off) keeps the slots intact.
    app.select_label_filter(Some("l0".to_string()));
    assert_eq!(app.state.active_label_filter, None);
    assert_eq!(names(&app), vec!["l0", "l1"]);

    // Slots fill up to nine; the tenth distinct label evicts the oldest
    // (slot 1) and everything shifts down one, so it lands in slot 9.
    for i in 2..9 {
        app.select_label_filter(Some(format!("l{}", i)));
    }
    assert_eq!(names(&app), vec!["l0", "l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8"]);
    app.select_label_filter(Some("l9".to_string()));
    assert_eq!(app.state.label_slots.len(), crate::keybindings::HOME_LABEL_SLOTS);
    assert_eq!(names(&app), vec!["l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9"]);
    app.select_label_filter(Some("l10".to_string()));
    assert_eq!(names(&app), vec!["l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9", "l10"]);

    // The slots persist in state.toml in slot order — and viewing labels is
    // usage, so config.toml is never written for it.
    let saved = std::fs::read_to_string(app.state_path()).expect("state persisted");
    assert!(saved.contains("label_slots = "), "saved: {}", saved);
    let reloaded: AppState = toml::from_str(&saved).expect("state parses");
    assert_eq!(reloaded.label_slots, app.state.label_slots);
    assert_eq!(
        std::fs::read_to_string(&app.config_path).unwrap(),
        config_before,
        "label views must not rewrite config.toml"
    );

    // A label nobody carries any more drops out of the strip and, on the
    // next save, out of the stored slots.
    app.config.labels.remove("/path/to/repo10");
    assert_eq!(names(&app).last().map(String::as_str), Some("l9"));
    app.prune_label_slots();
    assert!(!app.state.label_slots.contains(&"l10".to_string()));

    // A fresh state starts with no slots at all.
    assert!(AppState::default().label_slots.is_empty());
}

#[test]
fn test_usage_state_is_saved_beside_the_config_without_touching_it() {
    let temp_dir = std::env::temp_dir().join(format!(
        "gitwig_test_state_split_{}_{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let _guard = TestDirGuard { path: temp_dir.clone() };
    let config_path = temp_dir.join("config.toml");
    let config = Config {
        items: vec!["/path/to/frontend".to_string(), "/path/to/backend".to_string()],
        ..Default::default()
    };
    let mut app = App::new(config, config_path.clone());
    assert_eq!(app.state_path(), temp_dir.join("state.toml"));
    // The constructor's first-run version check writes config.toml once;
    // everything after this must leave it byte-for-byte alone.
    let config_before = std::fs::read_to_string(&config_path).unwrap();
    let state_before = std::fs::read_to_string(app.state_path()).unwrap_or_default();

    // Recording a visit and a commit message only writes state.toml.
    app.state.visits.insert("/path/to/frontend".to_string(), 1234);
    app.state.record_commit_message("/path/to/frontend", "feat: first");
    app.persist_state("Opened repository");
    assert_eq!(app.status_message.as_deref(), Some("Opened repository"));
    assert_ne!(std::fs::read_to_string(app.state_path()).unwrap(), state_before);
    assert_eq!(std::fs::read_to_string(&config_path).unwrap(), config_before);

    let (reloaded, warning) = crate::state::load_state(&app.state_path());
    assert!(warning.is_none());
    assert_eq!(reloaded.visits.get("/path/to/frontend"), Some(&1234));
    assert_eq!(reloaded.commit_history_for("/path/to/frontend"), vec!["feat: first".to_string()]);

    // A real settings save writes config.toml, and it stays free of state keys.
    app.persist("Saved");
    let saved = std::fs::read_to_string(&config_path).unwrap();
    assert!(saved.contains("items = "));
    for key in ["visits", "commit_history", "label_slots", "active_label_filter"] {
        assert!(!saved.contains(key), "config.toml must not contain `{}`:\n{}", key, saved);
    }

    // Loading through the real loader hands the state back to the app.
    let (config, state, path, warning) = crate::config::load_config(Some(config_path)).unwrap();
    assert!(warning.is_none());
    assert_eq!(config.items.len(), 2);
    assert_eq!(state.visits.get("/path/to/frontend"), Some(&1234));
    let app = App::with_state(config, state, path);
    assert_eq!(app.state.commit_history_for("/path/to/frontend"), vec!["feat: first".to_string()]);
}

#[test]
fn test_home_quick_label_keys_apply_toggle_and_ignore_empty_slots() {
    let mut config = Config {
        items: vec![
            "/path/to/frontend".to_string(),
            "/path/to/backend".to_string(),
            "/path/to/tools".to_string(),
        ],
        ..Default::default()
    };
    config.labels.insert("/path/to/frontend".to_string(), vec!["web".to_string()]);
    config
        .labels
        .insert("/path/to/backend".to_string(), vec!["web".to_string(), "api".to_string()]);
    let temp_path = std::env::temp_dir().join("gitwig_test_quick_label_keys.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key = |c: char| KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty());

    // Nothing viewed yet: every slot is empty, so digits are a no-op with a hint.
    assert!(crate::input::handle_key(&mut app, key('1'), 1));
    assert_eq!(app.state.active_label_filter, None);
    assert_eq!(app.status_message.as_deref(), Some("No label in quick slot 1"));

    // Viewing labels through the filter fills the slots in order:
    // web (viewed first) → slot 1, api (viewed second) → slot 2.
    app.select_label_filter(Some("web".to_string()));
    app.select_label_filter(Some("api".to_string()));
    app.select_label_filter(None);
    assert_eq!(app.state.active_label_filter, None);
    assert_eq!(app.state.label_slots, vec!["web".to_string(), "api".to_string()]);

    // Slot 2 applies "api"; the slots stay exactly where they were.
    assert!(crate::input::handle_key(&mut app, key('2'), 1));
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.state.active_label_filter.as_deref(), Some("api"));
    assert_eq!(app.get_filtered_items().len(), 1);
    assert_eq!(app.state.label_slots, vec!["web".to_string(), "api".to_string()]);

    // Slot 1 switches to "web": the filter is replaced, not stacked.
    assert!(crate::input::handle_key(&mut app, key('1'), 1));
    assert_eq!(app.state.active_label_filter.as_deref(), Some("web"));
    assert_eq!(app.get_filtered_items().len(), 2);
    assert_eq!(app.state.label_slots, vec!["web".to_string(), "api".to_string()]);

    // Pressing the active slot again clears the filter, like the picker.
    assert!(crate::input::handle_key(&mut app, key('1'), 1));
    assert_eq!(app.state.active_label_filter, None);
    assert_eq!(app.get_filtered_items().len(), 3);

    // An unfilled slot leaves the filter alone and explains why.
    app.status_message = None;
    assert!(crate::input::handle_key(&mut app, key('9'), 1));
    assert_eq!(app.state.active_label_filter, None);
    assert_eq!(app.status_message.as_deref(), Some("No label in quick slot 9"));

    // Quick-label keys are a home-screen concept: in the detail view the
    // same digits still switch tabs and never touch the label filter.
    app.mode = Mode::Detail;
    app.detail_tab = 0;
    assert!(crate::input::handle_key(&mut app, key('2'), 1));
    assert_eq!(app.detail_tab, 1);
    assert_eq!(app.state.active_label_filter, None);
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
    run_directory_scan(temp_scan_root.clone(), 3, vec![], tx, false);

    let mut found = Vec::new();
    while let Ok(msg) = rx.recv() {
        match msg {
            BgEvent::RepoScanFound { name, .. } => found.push(name),
            BgEvent::RepoScanComplete => break,
            _ => {}
        }
    }

    assert!(found.contains(&".macosdotfiles".to_string()));
    assert!(found.contains(&"my_repo".to_string()));
    assert!(!found.contains(&".cargo".to_string()));

    let _ = std::fs::remove_dir_all(&temp_scan_root);
}

#[test]
fn test_bulk_scan_flow() {
    let temp_scan_root = std::env::temp_dir().join("gitwig_test_bulk_scan_root");
    let _ = std::fs::remove_dir_all(&temp_scan_root);
    std::fs::create_dir_all(&temp_scan_root).unwrap();

    // Create a folder 'folder_x' containing 'a' (git), 'b' (git), and 'c' (nongit)
    let folder_x = temp_scan_root.join("folder_x");
    std::fs::create_dir_all(folder_x.join("a").join(".git")).unwrap();
    std::fs::create_dir_all(folder_x.join("b").join(".git")).unwrap();
    std::fs::create_dir_all(folder_x.join("c")).unwrap();

    // Create another folder 'folder_y' containing no git repositories
    let folder_y = temp_scan_root.join("folder_y");
    std::fs::create_dir_all(folder_y.join("d")).unwrap();

    let (tx, rx) = std::sync::mpsc::channel();
    run_directory_scan(temp_scan_root.clone(), 3, vec![], tx, true);

    let mut found = Vec::new();
    while let Ok(msg) = rx.recv() {
        match msg {
            BgEvent::RepoScanFound { name, .. } => found.push(name),
            BgEvent::RepoScanComplete => break,
            _ => {}
        }
    }

    // Should find folder_x (since it contains immediate git repo subfolders a and b)
    assert!(found.contains(&"folder_x".to_string()));
    // Should NOT find folder_y (since it doesn't contain any git repo subfolders)
    assert!(!found.contains(&"folder_y".to_string()));
    // Should NOT find a or b directly (since bulk scan only finds parents containing git repos)
    assert!(!found.contains(&"a".to_string()));
    assert!(!found.contains(&"b".to_string()));

    let _ = std::fs::remove_dir_all(&temp_scan_root);
}

#[test]
fn test_automatic_workspace_sync() {
    let watch_root = std::env::temp_dir().join("gitwig_watch_test");
    let _ = std::fs::remove_dir_all(&watch_root);
    std::fs::create_dir_all(&watch_root).unwrap();

    // Create a new git repository inside watched root
    let new_repo = watch_root.join("new_auto_repo");
    std::fs::create_dir_all(new_repo.join(".git")).unwrap();

    let config =
        Config { watch_dirs: vec![watch_root.to_string_lossy().to_string()], ..Default::default() };

    let mut app = App::new(config, std::env::temp_dir().join("dummy_config.toml"));

    // Verify watch dirs are registered (watcher exists)
    app.setup_watcher();
    assert!(app.watcher.is_some());

    // Send a RefreshRepo event to simulate notify trigger
    app.tx
        .send(BgEvent::RefreshRepo { repo_root: new_repo.to_string_lossy().into_owned() })
        .unwrap();

    // Wait a brief moment and process events
    let start = std::time::Instant::now();
    while start.elapsed().as_millis() < 500 {
        if app.drain_bg_events() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    assert!(
        app.config.items.contains(&new_repo.to_string_lossy().to_string()),
        "The repository should be automatically added to config"
    );

    let _ = std::fs::remove_dir_all(&watch_root);
}

#[test]
fn test_startup_deferred_loading() {
    let config = Config {
        items: vec!["/dummy/path/a".to_string(), "/dummy/path/b".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_deferred.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };

    let app = App::new(config, temp_path);

    // Check that at startup, the status array is immediately initialized with Loading
    assert_eq!(app.statuses.len(), 2);
    assert!(matches!(app.statuses[0], repo::ItemStatus::Loading));
    assert!(matches!(app.statuses[1], repo::ItemStatus::Loading));

    // Since App::new spawns a background thread to load them, they should be sent via the status_refresh_tx
    let mut received = false;
    for _ in 0..50 {
        if let Ok(updates) =
            app.status_refresh_rx.recv_timeout(std::time::Duration::from_millis(50))
        {
            // Because we chunk/worker them, they might be sent in chunks or all together depending on timing.
            // Let's assert that we got some updates.
            assert!(!updates.is_empty());
            for update in updates {
                assert!(matches!(update.2, repo::ItemStatus::Missing));
            }
            received = true;
            break;
        }
    }
    assert!(received, "Should receive background status updates");
}

/// The startup prompt must fire only when there is something to add: inside a
/// repository, not already tracked, and not switched off in config.
#[test]
fn test_detect_cwd_repo_startup_prompt() {
    let base = std::env::temp_dir().join("gitwig_test_cwd_repo_prompt");
    let _ = std::fs::remove_dir_all(&base);
    let repo_root = base.join("myrepo");
    let nested = repo_root.join("src").join("deep");
    std::fs::create_dir_all(&nested).expect("temp dirs should be creatable");
    std::fs::create_dir_all(repo_root.join(".git")).expect(".git should be creatable");
    let plain_dir = base.join("not_a_repo");
    std::fs::create_dir_all(&plain_dir).expect("temp dir should be creatable");

    let cfg_path = std::env::temp_dir().join("gitwig_test_cwd_repo_prompt.toml");
    let _guard = TestFileGuard { path: cfg_path.clone() };
    let new_app = || App::new(Config { items: vec![], ..Default::default() }, cfg_path.clone());

    // Launched from a nested directory: the prompt offers the repo *root*.
    let mut app = new_app();
    app.detect_cwd_repo(&nested);
    assert_eq!(app.mode, Mode::AddCwdRepoConfirm);
    assert_eq!(
        App::canonical_path(std::path::Path::new(
            app.pending_cwd_repo.as_deref().expect("a repo should be pending")
        )),
        App::canonical_path(&repo_root),
        "the prompt should offer the repository root, not the launch directory"
    );

    // Accepting adds it and returns to the list.
    app.confirm_add_cwd_repo();
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.config.items.len(), 1);
    assert!(app.pending_cwd_repo.is_none());

    // Already tracked: no prompt the next time.
    let tracked = app.config.items[0].clone();
    let mut app = App::new(Config { items: vec![tracked], ..Default::default() }, cfg_path.clone());
    app.detect_cwd_repo(&nested);
    assert_eq!(app.mode, Mode::Normal, "an already-tracked repo must not prompt");
    assert!(app.pending_cwd_repo.is_none());

    // Outside any repository: no prompt.
    let mut app = new_app();
    app.detect_cwd_repo(&plain_dir);
    assert_eq!(app.mode, Mode::Normal);

    // Declining clears the pending path.
    let mut app = new_app();
    app.detect_cwd_repo(&nested);
    assert_eq!(app.mode, Mode::AddCwdRepoConfirm);
    app.dismiss_cwd_repo();
    assert_eq!(app.mode, Mode::Normal);
    assert!(app.pending_cwd_repo.is_none());
    assert!(app.config.items.is_empty(), "declining must not add the repo");

    // Switched off in config: never prompts.
    let mut app = App::new(
        Config { items: vec![], prompt_cwd_repo: false, ..Default::default() },
        cfg_path.clone(),
    );
    app.detect_cwd_repo(&nested);
    assert_eq!(app.mode, Mode::Normal, "prompt_cwd_repo = false must suppress the prompt");

    let _ = std::fs::remove_dir_all(&base);
}

/// `.git` as a *file* marks a linked worktree or submodule; it must be
/// discovered the same as a `.git` directory.
#[test]
fn test_discover_repo_root_handles_gitfile_and_missing() {
    let base = std::env::temp_dir().join("gitwig_test_discover_root");
    let _ = std::fs::remove_dir_all(&base);
    let wt = base.join("worktree");
    std::fs::create_dir_all(wt.join("sub")).expect("temp dirs should be creatable");
    std::fs::write(wt.join(".git"), "gitdir: /elsewhere/.git/worktrees/wt")
        .expect(".git file should be writable");

    assert_eq!(
        crate::repo::discover_repo_root(&wt.join("sub")).map(|p| App::canonical_path(&p)),
        Some(App::canonical_path(&wt)),
        "a .git file should mark the root"
    );

    let orphan = base.join("orphan");
    std::fs::create_dir_all(&orphan).expect("temp dir should be creatable");
    // Nothing above a temp dir is a repo, so discovery walks to the filesystem
    // root and gives up rather than looping.
    let found = crate::repo::discover_repo_root(&orphan);
    assert!(
        found.is_none() || !App::canonical_path(&found.unwrap_or_default()).starts_with(&orphan),
        "a directory outside any repo must not report itself as a root"
    );

    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn test_label_sort_override_applies_only_while_filter_active() {
    let (mut app, path) = label_sort_app(
        "lbl_sort_view",
        crate::config::LabelConfig { sort_by: Some(SortOrder::Alphabetical), ..Default::default() },
    );
    let _guard = TestFileGuard { path };

    // No filter: the global Custom order is untouched.
    assert_eq!(app.effective_sort_by(), SortOrder::Custom);
    assert_eq!(app.config.items, vec!["/c", "/a", "/b", "/z"]);
    assert!(app.sort_override_label().is_none());

    // Entering the label view applies the label's sort to the list.
    app.select_label_filter(Some("work".to_string()));
    assert_eq!(app.effective_sort_by(), SortOrder::Alphabetical);
    assert_eq!(app.sort_override_label(), Some("work"));
    assert_eq!(app.config.items, vec!["/a", "/b", "/c", "/z"]);
    assert_eq!(app.config.sort_by, SortOrder::Custom, "global sort must not change");
    assert!(crate::ui::draw::sort_caption(&app).contains("Sort: Alphabetical · work"));

    // Leaving the view restores the custom order.
    app.select_label_filter(Some("work".to_string()));
    assert!(app.state.active_label_filter.is_none());
    assert_eq!(app.config.items, vec!["/c", "/a", "/b", "/z"]);
    assert!(!crate::ui::draw::sort_caption(&app).contains("work"));

    // A label without a sort override inherits the global sort.
    app.config.label_configs.insert("work".to_string(), crate::config::LabelConfig::default());
    app.select_label_filter(Some("work".to_string()));
    assert_eq!(app.effective_sort_by(), SortOrder::Custom);
    assert_eq!(app.config.items, vec!["/c", "/a", "/b", "/z"]);
}

#[test]
fn test_label_sort_reverse_override_resolves_independently() {
    let (mut app, path) = label_sort_app(
        "lbl_sort_rev",
        crate::config::LabelConfig { sort_reverse: Some(true), ..Default::default() },
    );
    let _guard = TestFileGuard { path };
    app.config.sort_by = SortOrder::Alphabetical;
    app.sort_items_in_place();
    assert_eq!(app.config.items, vec!["/a", "/b", "/c", "/z"]);

    // The label only overrides the direction; the mode still comes from global.
    app.select_label_filter(Some("work".to_string()));
    assert_eq!(app.effective_sort(), (SortOrder::Alphabetical, true));
    assert_eq!(app.config.items, vec!["/z", "/c", "/b", "/a"]);
    assert!(crate::ui::draw::sort_caption(&app).contains("(Rev) · work"));
}

#[test]
fn test_sort_keys_edit_label_override_when_it_owns_the_sort() {
    let (mut app, path) = label_sort_app(
        "lbl_sort_keys",
        crate::config::LabelConfig {
            sort_by: Some(SortOrder::Alphabetical),
            sort_reverse: None,
            ..Default::default()
        },
    );
    let _guard = TestFileGuard { path };
    app.select_label_filter(Some("work".to_string()));

    // `o` cycles the label's mode, leaving the global alone.
    app.cycle_sort_order();
    assert_eq!(app.config.label_configs["work"].sort_by, Some(SortOrder::RecentVisit));
    assert_eq!(app.config.sort_by, SortOrder::Custom);
    assert_eq!(app.status_message.as_deref(), Some("Sort mode updated for label 'work'"));

    // `O` with no label direction override flips the global direction.
    app.toggle_sort_reverse();
    assert_eq!(app.config.label_configs["work"].sort_reverse, None);
    assert!(app.config.sort_reverse);
    assert_eq!(app.status_message.as_deref(), Some("Sort direction updated"));

    // Once the label owns the direction, `O` flips that instead.
    app.config.label_configs.get_mut("work").unwrap().sort_reverse = Some(false);
    app.toggle_sort_reverse();
    assert_eq!(app.config.label_configs["work"].sort_reverse, Some(true));
    assert!(app.config.sort_reverse, "global direction must not change");

    // Outside the label view the keys act on the global settings again.
    app.select_label_filter(None);
    app.cycle_sort_order();
    assert_eq!(app.config.sort_by, SortOrder::Alphabetical);
    assert_eq!(app.config.label_configs["work"].sort_by, Some(SortOrder::RecentVisit));
}

/// A thin rule closes the pinned block the sort keeps at the top of the list
/// and of each label group. Only a block opening its group gets one; the
/// Recent and Starred groups, which interleave pinned and unpinned rows, and a
/// group without pinned rows do not.
#[test]
fn test_home_pin_separators_close_the_pinned_block() {
    let items: Vec<String> =
        ["/p/a", "/p/b", "/p/c", "/p/d", "/p/e"].iter().map(|s| s.to_string()).collect();
    let mut config = Config {
        items: items.clone(),
        sort_by: SortOrder::Custom,
        show_grouping: false,
        ..Default::default()
    };
    config.pinned.insert(items[0].clone());
    config.pinned.insert(items[1].clone());
    config.starred.insert(items[0].clone());
    config.starred.insert(items[2].clone());
    let temp_path = std::env::temp_dir().join("gitwig_test_home_pin_separators.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Flat list: a, b pinned, then c, d, e.
    let rows = app.get_home_rows();
    assert_eq!(rows.len(), 5);
    assert_eq!(app.home_pin_separators(&rows), [false, true, false, false, false]);

    // Grouped: Starred (a, c: pinned then unpinned, no rule), then the label
    // groups: `pet` (a, b pinned | c), `solo` (d, e: no pinned rows) and
    // `all` (a, b: everything pinned, so nothing to set apart).
    app.config.show_grouping = true;
    for (item, labels) in [
        (&items[0], vec!["pet", "all"]),
        (&items[1], vec!["pet", "all"]),
        (&items[2], vec!["pet"]),
        (&items[3], vec!["solo"]),
        (&items[4], vec!["solo"]),
    ] {
        app.config.labels.insert(item.clone(), labels.into_iter().map(String::from).collect());
    }
    let rows = app.get_home_rows();
    let separators = app.home_pin_separators(&rows);
    let described: Vec<(String, bool)> = rows
        .iter()
        .zip(&separators)
        .map(|(row, &sep)| match row {
            HomeRow::GroupHeader { name, .. } => (format!("[{name}]"), sep),
            HomeRow::Repo { path, primary_label, .. } => (format!("{primary_label}:{path}"), sep),
        })
        .collect();
    let expected: Vec<(String, bool)> = [
        ("[Starred]", false),
        ("Starred:/p/a", false),
        ("Starred:/p/c", false),
        ("[all]", false),
        ("all:/p/a", false),
        ("all:/p/b", false),
        ("[pet]", false),
        ("pet:/p/a", false),
        ("pet:/p/b", true),
        ("pet:/p/c", false),
        ("[solo]", false),
        ("solo:/p/d", false),
        ("solo:/p/e", false),
    ]
    .iter()
    .map(|(row, sep)| (row.to_string(), *sep))
    .collect();
    assert_eq!(described, expected);

    // With the separators setting off, no row closes a pinned block with a rule.
    app.config.show_separators = false;
    assert!(app.home_pin_separators(&rows).iter().all(|&sep| !sep));
}

/// A label's `view_mode` replaces the global layout only while that label's
/// filter is active, `v` then cycles the label's value instead of the global,
/// and the Label Settings row walks default → Normal → Compact → Tile → default.
#[test]
fn test_label_view_mode_applies_in_project_view() {
    use crate::config::HomeViewMode;
    use crate::popups::label_settings::LabelSettingsPopup;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let key = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let (mut app, path) = label_test_app(
        "lbl_view_mode",
        vec!["web"],
        None,
        vec![(
            "web",
            crate::config::LabelConfig {
                view_mode: Some(HomeViewMode::Tile),
                ..Default::default()
            },
        )],
    );
    let _guard = TestFileGuard { path };
    app.config.tile_columns = 3;
    assert_eq!(app.config.view_mode, HomeViewMode::Normal);
    assert_eq!(app.effective_view_mode(), HomeViewMode::Normal, "no filter: the global layout");
    assert_eq!(app.get_tile_cols(), 1);

    app.select_label_filter(Some("web".to_string()));
    assert_eq!(app.effective_view_mode(), HomeViewMode::Tile, "in the label's view: its layout");
    assert_eq!(app.get_tile_cols(), 3, "the grid follows the label's layout");

    // `v` cycles the label's layout, leaving the global one alone.
    app.mode = Mode::Normal;
    let _ = crate::tabs::HomeTab::handle_event(&mut app, key(KeyCode::Char('v')), 10);
    assert_eq!(app.config.label_configs["web"].view_mode, Some(HomeViewMode::Normal));
    assert_eq!(app.effective_view_mode(), HomeViewMode::Normal);
    assert_eq!(app.config.view_mode, HomeViewMode::Normal);

    // Leaving the view restores the global layout, and `v` edits that again.
    app.select_label_filter(None);
    let _ = crate::tabs::HomeTab::handle_event(&mut app, key(KeyCode::Char('v')), 10);
    assert_eq!(app.config.view_mode, HomeViewMode::Compact);
    assert_eq!(app.config.label_configs["web"].view_mode, Some(HomeViewMode::Normal));

    // The Label Settings row (8) cycles through every layout and back to default.
    app.mode = Mode::LabelSettings;
    app.label_settings_target = Some("web".to_string());
    app.label_settings_selected_index = 8;
    let mut seen = Vec::new();
    for _ in 0..4 {
        LabelSettingsPopup::handle_event(&mut app, key(KeyCode::Right));
        seen.push(app.config.label_configs["web"].view_mode);
    }
    assert_eq!(
        seen,
        [Some(HomeViewMode::Compact), Some(HomeViewMode::Tile), None, Some(HomeViewMode::Normal)]
    );
    LabelSettingsPopup::handle_event(&mut app, key(KeyCode::Left));
    assert_eq!(app.config.label_configs["web"].view_mode, None);
    LabelSettingsPopup::handle_event(&mut app, key(KeyCode::Left));
    assert_eq!(app.config.label_configs["web"].view_mode, Some(HomeViewMode::Tile));

    // Down from the last row wraps to the first.
    LabelSettingsPopup::handle_event(&mut app, key(KeyCode::Down));
    assert_eq!(app.label_settings_selected_index, 0);

    // And it survives a config round trip.
    let serialized = toml::to_string_pretty(&app.config).unwrap();
    assert!(serialized.contains("view_mode = \"tile\""), "{serialized}");
    let parsed: Config = toml::from_str(&serialized).unwrap();
    assert_eq!(parsed.label_configs["web"].view_mode, Some(HomeViewMode::Tile));
}

#[test]
fn test_watcher_resolve_refresh_roots_worktree_and_filters() {
    use super::{WatchedRepo, resolve_refresh_roots};

    let targets = vec![
        WatchedRepo {
            repo_root: "/ws/main".to_string(),
            git_dir: PathBuf::from("/ws/main/.git"),
            common_dir: PathBuf::from("/ws/main/.git"),
        },
        WatchedRepo {
            repo_root: "/ws/wt1".to_string(),
            git_dir: PathBuf::from("/ws/main/.git/worktrees/wt1"),
            common_dir: PathBuf::from("/ws/main/.git"),
        },
    ];
    let excludes = vec!["node_modules".to_string()];

    // `.git/objects` and `.lock` files are ignored.
    assert!(
        resolve_refresh_roots(
            std::path::Path::new("/ws/main/.git/objects/ab/cdef"),
            &targets,
            &excludes
        )
        .is_empty()
    );
    assert!(
        resolve_refresh_roots(
            std::path::Path::new("/ws/main/.git/index.lock"),
            &targets,
            &excludes
        )
        .is_empty()
    );

    // Worktree HEAD/index events map only to that worktree's repo_root.
    assert_eq!(
        resolve_refresh_roots(
            std::path::Path::new("/ws/main/.git/worktrees/wt1/HEAD"),
            &targets,
            &excludes
        ),
        vec!["/ws/wt1".to_string()]
    );

    // Shared `refs/` events notify both the main checkout and linked worktree.
    assert_eq!(
        resolve_refresh_roots(
            std::path::Path::new("/ws/main/.git/refs/heads/feature"),
            &targets,
            &excludes
        ),
        vec!["/ws/main".to_string(), "/ws/wt1".to_string()]
    );

    // Untracked repos discovered inside watch_dirs still fall back via `/.git`.
    assert_eq!(
        resolve_refresh_roots(
            std::path::Path::new("/watched_root/brand_new/.git/HEAD"),
            &targets,
            &excludes
        ),
        vec!["/watched_root/brand_new".to_string()]
    );
}

#[test]
fn test_archive_toggle_filter_counts_and_lifecycle() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let (cfg_path, _guard) = temp_config_path("archive_toggle_filter_counts");
    let dir_a = std::env::temp_dir().join("gitwig_test_arch_a");
    let dir_b = std::env::temp_dir().join("gitwig_test_arch_b");
    let _ = std::fs::create_dir_all(&dir_a);
    let _ = std::fs::create_dir_all(&dir_b);
    let repo_a = dir_a.to_string_lossy().into_owned();
    let repo_b = dir_b.to_string_lossy().into_owned();
    let repo_b_new = format!("{}_renamed", repo_b);

    let config = Config { items: vec![repo_a.clone(), repo_b.clone()], ..Default::default() };
    let mut app = App::new(config, cfg_path);
    app.config.show_stale_projects = true;

    let now_secs =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
            as i64;
    let dirty_summary = repo::RepoSummary {
        branch: Some("main".to_string()),
        modified: 1,
        ahead: 1,
        last_commit_time: Some(now_secs),
        ..Default::default()
    };
    let clean_summary = repo::RepoSummary {
        branch: Some("main".to_string()),
        last_commit_time: Some(now_secs),
        ..Default::default()
    };
    app.statuses = vec![
        repo::ItemStatus::GitRepo(Some(dirty_summary)),
        repo::ItemStatus::GitRepo(Some(clean_summary)),
    ];

    // Before archiving: repo_a counts toward repos/dirty/ahead, 0 archived.
    let counts = crate::ui::draw::summary_counts(&app);
    assert_eq!(counts.total_repos, 2);
    assert_eq!(counts.dirty, 1);
    assert_eq!(counts.ahead, 1);
    assert_eq!(counts.archived, 0);

    // Press `z` to archive the selected repo (repo_a).
    let z_key = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::empty());
    assert!(crate::input::handle_key(&mut app, z_key, 10));
    assert!(app.config.archived.contains(&repo_a));
    assert_eq!(app.status_message.as_deref(), Some("Archived repository"));

    // Archived repo drops out of `repos`/`dirty`/`ahead` and increments `archived`.
    let counts = crate::ui::draw::summary_counts(&app);
    assert_eq!(counts.total_repos, 1);
    assert_eq!(counts.dirty, 0);
    assert_eq!(counts.ahead, 0);
    assert_eq!(counts.archived, 1);
    assert_eq!(app.get_filtered_items().len(), 1);

    app.global_filter = Some(GlobalFilter::Dirty);
    assert!(app.get_filtered_items().is_empty());
    app.global_filter = Some(GlobalFilter::Archived);
    let archived_items = app.get_filtered_items();
    assert_eq!(archived_items.len(), 1);
    assert_eq!(archived_items[0].1, &repo_a);

    // `bulk_fetch_all` skips archived repos.
    app.bulk_fetch_all();
    assert!(!app.bulk_fetching.contains(&repo_a));

    // Pressing `z` in the Archived tab unarchives repo_a.
    app.selected_index = 0;
    assert!(crate::input::handle_key(&mut app, z_key, 10));
    assert!(!app.config.archived.contains(&repo_a));
    assert_eq!(app.status_message.as_deref(), Some("Unarchived repository"));
    assert!(app.get_filtered_items().is_empty());

    // Archive repo_b and test rename migration + delete cleanup.
    app.global_filter = None;
    app.config.archived.insert(repo_b.clone());
    app.migrate_repo_path(&repo_b, &repo_b_new);
    assert!(!app.config.archived.contains(&repo_b));
    assert!(app.config.archived.contains(&repo_b_new));

    app.config.items[1] = repo_b_new.clone();
    app.multi_selected.insert(repo_b_new.clone());
    app.confirm_delete();
    assert!(!app.config.archived.contains(&repo_b_new));

    let _ = std::fs::remove_dir_all(&dir_a);
    let _ = std::fs::remove_dir_all(&dir_b);
}
