#![allow(clippy::unwrap_used, clippy::panic)]

use super::*;

#[test]
fn test_set_error_logging() {
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
    let temp_path = std::env::temp_dir().join("gitwig_test_config_set_error.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    let test_error_msg = "Test error message for debugging".to_string();
    app.set_error(test_error_msg.clone());

    assert_eq!(app.error_message.as_ref(), Some(&test_error_msg));

    // Check if debug log contains the message
    let logs = crate::debug_log::get_logs();
    assert!(logs.iter().any(|log| log.contains("ERROR") && log.contains(&test_error_msg)));

    // Raw ssh/git stderr is sanitized before display: ANSI escapes are
    // stripped and carriage returns flattened so the popup can't tear the
    // frame with control characters.
    app.set_error("\x1b[31mPermission denied\x1b[0m (publickey).\rretry".to_string());
    assert_eq!(app.error_message.as_deref(), Some("Permission denied (publickey).\nretry"));
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
    assert_eq!(app.settings_selected_index, 9);
    assert!(!app.settings_editing);
    assert!(app.settings_focus_sidebar);

    // Focus right content pane via 'w'
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('w')), 10);
    assert!(handled);
    assert!(!app.settings_focus_sidebar);
    assert_eq!(app.settings_selected_index, 9);

    // Go down to Poll Interval (index 0): 9 -> 56 -> 55 -> 0
    for _ in 0..3 {
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    }
    assert_eq!(app.settings_selected_index, 0);

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

    // Go down to Scan Excludes (index 8) — Scan Git Only setting has been removed
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
    assert_eq!(app.settings_selected_index, 9);

    // Navigate to Page Size (index 7) by pressing End and then Up twice (due to new stale settings added at the end)
    crate::input::handle_key(&mut app, key_event(KeyCode::End), 10);
    assert_eq!(app.settings_selected_index, 81);
    crate::input::handle_key(&mut app, key_event(KeyCode::Up), 10);
    assert_eq!(app.settings_selected_index, 80);
    crate::input::handle_key(&mut app, key_event(KeyCode::Up), 10);
    assert_eq!(app.settings_selected_index, 7);

    // Edit Page Size
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(app.settings_editing);
    app.input_buffer = "15".to_string();
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(!app.settings_editing);
    assert_eq!(app.config.page_size, 15);

    // Go down to Preferred Git Client (index 9) by pressing Home
    crate::input::handle_key(&mut app, key_event(KeyCode::Home), 10);
    assert_eq!(app.settings_selected_index, 9);

    // Edit Preferred Git Client
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(app.settings_editing);
    app.input_buffer = "lazygit".to_string();
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(!app.settings_editing);
    assert_eq!(app.config.git_app, "lazygit");

    // Go to Compatibility Mode (index 12): first press End to go to 81, then Up 6 times
    crate::input::handle_key(&mut app, key_event(KeyCode::End), 10);
    for _ in 0..6 {
        crate::input::handle_key(&mut app, key_event(KeyCode::Up), 10);
    }
    assert_eq!(app.settings_selected_index, 12);
    assert!(!app.config.compatibility_mode);

    // Toggle Compatibility Mode
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(app.config.compatibility_mode);
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(!app.config.compatibility_mode);

    // Go to Resync on Tab Change (index 13) by going Up 3 times (12 -> 66 -> 65 -> 13)
    for _ in 0..3 {
        crate::input::handle_key(&mut app, key_event(KeyCode::Up), 10);
    }
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
    assert_eq!(app.settings_selected_index, 9);
    for _ in 0..4 {
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    }
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
    assert_eq!(app.settings_selected_index, 61); // Category 2 end is 61

    crate::input::handle_key(&mut app, key_event(KeyCode::PageDown), 10);
    assert_eq!(app.settings_selected_index, 61);

    // Press Esc to focus sidebar
    crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
    assert!(app.settings_focus_sidebar);

    // Press Esc on sidebar to exit settings
    crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn test_paste_sanitization_and_commit() {
    let config = Config::default();
    let temp_path = std::env::temp_dir().join("gitwig_test_config_paste_sanitization.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Single line mode: BranchCreateInput
    app.mode = Mode::BranchCreateInput;
    app.input_str("feature/my-branch\r\n");
    assert_eq!(app.input_buffer, "feature/my-branch");

    // Multi line mode: CommitInput editing
    app.mode = Mode::CommitInput;
    app.commit_popup.editing = true;
    app.input_str("feat: add feature\n\nDetailed body description");
    assert_eq!(app.commit_popup.input_buffer, "feat: add feature\n\nDetailed body description");
}

#[test]
fn test_typing_q_in_text_input_does_not_cancel() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config::default();
    let temp_path = std::env::temp_dir().join("gitwig_test_config_typing_q.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.mode = Mode::RemoteAddUrlInput;

    // Type characters "tareq" one by one
    for c in "tareq".chars() {
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char(c)), 10);
        assert!(handled);
    }

    // Modal should NOT have cancelled or switched mode
    assert_eq!(app.mode, Mode::RemoteAddUrlInput);
    assert_eq!(app.input_buffer, "tareq");
}

#[test]
fn test_initial_setup_and_migration() {
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
fn test_dynamic_status_height() {
    let config = Config::default();
    let (config_path, _guard) = temp_config_path("dynamic_status_height");
    let mut app = App::new(config, config_path);

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
    let (config_path, _guard) = temp_config_path("help_overlay_wrapping");
    let app = App::new(config, config_path);

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
    let (config_path, _guard) = temp_config_path("cancel_selections");
    let mut app = App::new(config, config_path);
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
fn test_settings_panel_organization() {
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

    // Enter Overview mode via 'O'
    let o_press = key_event(KeyCode::Char('O'));
    let handled = crate::input::handle_key(&mut app, o_press, 1);
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

    // Go down to Auto Fetch (index 4)
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

    // Enter to edit the per-repo auto-fetch interval, type "45" and confirm
    let handled = crate::input::handle_key(&mut app, enter_press, 1);
    assert!(handled);
    assert!(app.repo_settings_editing);
    for c in "45".chars() {
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char(c)), 1);
        assert!(handled);
    }
    assert_eq!(app.repo_settings_input, "45");
    let handled = crate::input::handle_key(&mut app, enter_press, 1);
    assert!(handled);
    assert!(!app.repo_settings_editing);
    let repo_cfg = app.config.repo_configs.get("/path/to/custom_repo").unwrap();
    assert_eq!(repo_cfg.auto_fetch_interval_mins, Some(45));

    // Down to 5 (Editor Command)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Down), 1);
    assert!(handled);
    assert_eq!(app.repo_settings_selected_index, 5);

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

    // Down to 6 (User Note)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Down), 1);
    assert!(handled);
    assert_eq!(app.repo_settings_selected_index, 6);

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
    let (config_path, _guard) = temp_config_path("cargo_install_detection");
    let app = App::new(config, config_path);
    // Normal test execution runs within target/debug/deps, so it is not a Cargo, Homebrew, or Chocolatey installation folder
    assert!(!app.is_cargo_install());
    assert!(!app.is_msi_install());
    assert!(!app.is_homebrew_install());
    assert!(!app.is_chocolatey_install());
    assert!(!app.is_nix_install());
    assert!(app.can_self_update());
}

#[test]
fn test_repo_settings_fallbacks() {
    let mut repo_configs = std::collections::HashMap::new();
    let repo_cfg = RepoConfig {
        page_size: Some(15),
        max_commits: Some(200),
        resync_on_tab_change: Some(false),
        auto_fetch_interval_mins: Some(0),
        ..Default::default()
    };
    repo_configs.insert("/path/to/repo_a".to_string(), repo_cfg);

    let config = Config {
        page_size: 10,
        max_commits: 100,
        resync_on_tab_change: true,
        auto_fetch_interval_mins: 25,
        items: vec!["/path/to/repo_a".to_string(), "/path/to/repo_b".to_string()],
        repo_configs,
        ..Default::default()
    };

    let (config_path, _guard) = temp_config_path("repo_settings_fallbacks");
    let mut app = App::new(config, config_path);

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

    // Auto-fetch cadence: repo_a is disabled by its override, repo_b inherits the global interval
    assert_eq!(app.effective_auto_fetch_interval_mins("/path/to/repo_a"), 0);
    assert_eq!(app.effective_auto_fetch_interval_mins("/path/to/repo_b"), 25);
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
    assert_eq!(app.settings_selected_index, 16); // Quit / Close Dialog
    assert!(!app.settings_editing);

    // Navigate down to Toggle Status Bar (index 14)
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert_eq!(app.settings_selected_index, 15);
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert_eq!(app.settings_selected_index, 14);

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
fn test_manual_update_check_flow() {
    let config = Config::default();
    let (config_path, _guard) = temp_config_path("manual_update_check_flow");
    let mut app = App::new(config, config_path);
    assert!(!app.update_check_manual);

    app.trigger_update_check();
    assert!(app.update_check_manual);
    assert_eq!(app.status_message.as_deref(), Some("Checking for updates..."));
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
    app.repo_settings_selected_index = 5; // editor
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
fn test_settings_watch_dirs_editing() {
    let config = Config { watch_dirs: vec!["~/old_watch".to_string()], ..Default::default() };

    let temp_path = std::env::temp_dir().join("gitwig_test_config_settings_watch.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.mode = Mode::Settings;
    app.settings_selected_index = 61; // Watch directories
    app.settings_focus_sidebar = false;
    app.settings_editing = false;

    // Start editing
    app.toggle_or_edit_setting();
    assert!(app.settings_editing);
    assert_eq!(app.input_buffer, "~/old_watch");

    // Modify input buffer
    app.input_buffer = "~/new_watch,~/another".to_string();

    // Commit edit
    app.commit_settings_edit();
    assert!(!app.settings_editing);
    assert_eq!(app.config.watch_dirs, vec!["~/new_watch".to_string(), "~/another".to_string()]);
}

#[test]
fn test_settings_show_system_stats_toggling() {
    let config = Config { show_system_stats: false, ..Default::default() };

    let temp_path = std::env::temp_dir().join("gitwig_test_config_settings_stats.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.mode = Mode::Settings;
    app.settings_selected_index = 62; // Show CPU/MEM in Status Bar
    app.settings_focus_sidebar = false;
    app.settings_editing = false;

    // Toggle setting (simulates Enter/Space key press)
    app.toggle_or_edit_setting();
    assert!(app.config.show_system_stats);

    // Toggle again
    app.toggle_or_edit_setting();
    assert!(!app.config.show_system_stats);
}

#[test]
fn test_settings_enable_commit_signatures_toggling() {
    let config = Config { enable_commit_signatures: false, ..Default::default() };

    let temp_path = std::env::temp_dir().join("gitwig_test_config_settings_signatures.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.mode = Mode::Settings;
    app.settings_selected_index = 63; // Enable Commit Signatures
    app.settings_focus_sidebar = false;
    app.settings_editing = false;

    // Toggle setting
    app.toggle_or_edit_setting();
    assert!(app.config.enable_commit_signatures);

    // Toggle again
    app.toggle_or_edit_setting();
    assert!(!app.config.enable_commit_signatures);
}

#[test]
fn test_settings_graph_max_commits_editing() {
    let config = Config { graph_max_commits: 1000, ..Default::default() };

    let temp_path = std::env::temp_dir().join("gitwig_test_config_settings_graph_commits.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.mode = Mode::Settings;
    app.settings_selected_index = 64; // Graph Max Commits
    app.settings_focus_sidebar = false;
    app.settings_editing = false;

    // Start editing
    app.toggle_or_edit_setting();
    assert!(app.settings_editing);
    assert_eq!(app.input_buffer, "1000");

    // Modify input buffer
    app.input_buffer = "250".to_string();

    // Commit edit
    app.commit_settings_edit();
    assert!(!app.settings_editing);
    assert_eq!(app.config.graph_max_commits, 250);
}

#[test]
fn test_settings_detail_cache_ttl_editing() {
    let config = Config { detail_cache_ttl_secs: 30, ..Default::default() };

    let temp_path = std::env::temp_dir().join("gitwig_test_config_settings_detail_ttl.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.mode = Mode::Settings;
    app.settings_selected_index = 65; // Detail Cache TTL
    app.settings_focus_sidebar = false;
    app.settings_editing = false;

    // Start editing
    app.toggle_or_edit_setting();
    assert!(app.settings_editing);
    assert_eq!(app.input_buffer, "30");

    // Modify input buffer
    app.input_buffer = "15".to_string();

    // Commit edit
    app.commit_settings_edit();
    assert!(!app.settings_editing);
    assert_eq!(app.config.detail_cache_ttl_secs, 15);
}

#[test]
fn test_settings_tab_ttl_editing() {
    let config = Config { tab_ttl_secs: 60, ..Default::default() };

    let temp_path = std::env::temp_dir().join("gitwig_test_config_settings_tab_ttl.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.mode = Mode::Settings;
    app.settings_selected_index = 66; // Tab Cache TTL
    app.settings_focus_sidebar = false;
    app.settings_editing = false;

    // Start editing
    app.toggle_or_edit_setting();
    assert!(app.settings_editing);
    assert_eq!(app.input_buffer, "60");

    // Modify input buffer
    app.input_buffer = "45".to_string();

    // Commit edit
    app.commit_settings_edit();
    assert!(!app.settings_editing);
    assert_eq!(app.config.tab_ttl_secs, 45);
}

#[test]
fn test_settings_compact_view_toggling() {
    let config = Config { view_mode: crate::config::HomeViewMode::Normal, ..Default::default() };

    let temp_path = std::env::temp_dir().join("gitwig_test_config_settings_compact.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.mode = Mode::Settings;
    app.settings_selected_index = 67; // Compact Layout View
    app.settings_focus_sidebar = false;
    app.settings_editing = false;

    // Toggle setting
    app.toggle_or_edit_setting();
    assert_eq!(app.config.view_mode, crate::config::HomeViewMode::Compact);

    // Toggle again
    app.toggle_or_edit_setting();
    assert_eq!(app.config.view_mode, crate::config::HomeViewMode::Tile);
}

#[test]
fn test_settings_stale_behavior_and_toggling() {
    let config = Config {
        stale_threshold_months: 1,
        show_stale_projects: true,
        items: vec!["repo_a".to_string(), "repo_b".to_string()],
        ..Default::default()
    };

    let temp_path = std::env::temp_dir().join("gitwig_test_config_settings_stale.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // 1. Validate stale_threshold_months editing
    app.mode = Mode::Settings;
    app.settings_selected_index = 80; // Stale Threshold (months)
    app.settings_focus_sidebar = false;
    app.settings_editing = false;

    app.toggle_or_edit_setting();
    assert!(app.settings_editing);
    assert_eq!(app.input_buffer, "1");

    // Commit valid value (2 months)
    app.input_buffer = "2".to_string();
    app.commit_settings_edit();
    assert!(!app.settings_editing);
    assert_eq!(app.config.stale_threshold_months, 2);

    // Try committing invalid value (0 months)
    app.toggle_or_edit_setting();
    app.input_buffer = "0".to_string();
    app.commit_settings_edit();
    assert!(app.settings_editing); // still editing
    assert!(app.status_message.as_ref().unwrap().contains("must be at least 1"));

    // 2. Validate show_stale_projects toggling
    app.settings_selected_index = 81; // Show Stale Projects
    app.settings_editing = false;
    assert!(app.config.show_stale_projects);

    app.toggle_or_edit_setting();
    assert!(!app.config.show_stale_projects);

    app.toggle_or_edit_setting();
    assert!(app.config.show_stale_projects);

    // 3. Validate get_filtered_items stale filtering
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let summary_stale = crate::repo::RepoSummary {
        branch: Some("main".to_string()),
        staged: 0,
        modified: 0,
        untracked: 0,
        conflicted: 0,
        ahead: 0,
        behind: 0,
        state: crate::repo::RepoState::Clean,
        last_commit_time: Some(now - 45 * 24 * 60 * 60), // 45 days ago (stale for 1 month threshold, clean for 2 months threshold)
    };

    let summary_fresh = crate::repo::RepoSummary {
        branch: Some("main".to_string()),
        staged: 0,
        modified: 0,
        untracked: 0,
        conflicted: 0,
        ahead: 0,
        behind: 0,
        state: crate::repo::RepoState::Clean,
        last_commit_time: Some(now - 5 * 24 * 60 * 60), // 5 days ago (fresh)
    };

    app.statuses = vec![
        crate::repo::ItemStatus::GitRepo(Some(summary_stale)),
        crate::repo::ItemStatus::GitRepo(Some(summary_fresh)),
    ];

    // Threshold = 2 months (60 days), so 45 days is NOT stale
    app.config.stale_threshold_months = 2;
    app.config.show_stale_projects = false;
    let items = app.get_filtered_items();
    assert_eq!(items.len(), 2); // Both shown because neither is stale

    // Threshold = 1 month (30 days), so 45 days is stale. show_stale_projects = false
    app.config.stale_threshold_months = 1;
    let items = app.get_filtered_items();
    assert_eq!(items.len(), 1); // Stale project hidden
    assert_eq!(items[0].1, "repo_b"); // Only fresh repo_b shown

    // show_stale_projects = true
    app.config.show_stale_projects = true;
    let items = app.get_filtered_items();
    assert_eq!(items.len(), 2); // Both shown

    // Global Filter = Stale, show_stale_projects = false
    app.config.show_stale_projects = false;
    app.global_filter = Some(GlobalFilter::Stale);
    let items = app.get_filtered_items();
    assert_eq!(items.len(), 1); // Stale filter override works
    assert_eq!(items[0].1, "repo_a"); // Only stale repo_a shown
}

#[test]
fn test_keybindings_view_conflict_checking() {
    let defaults = crate::keybindings::KeybindingsConfig::default_config();

    // 1. Proposed binding is unique and does not conflict
    let conflict =
        defaults.find_conflict(crate::keybindings::Action::HomeMoveDown, &["ctrl-d".to_string()]);
    assert!(conflict.is_none());

    // 2. Conflict within the same view (Home view context)
    // HomeMoveDown is mapped to 'down' / 'j' by default. Let's propose 'j' for HomeMoveUp
    let conflict =
        defaults.find_conflict(crate::keybindings::Action::HomeMoveUp, &["j".to_string()]);
    assert_eq!(conflict, Some(crate::keybindings::Action::HomeMoveDown));

    // 3. No conflict across different views (Home view action vs Detail view action)
    // HomeMoveDown is 'down' / 'j'. CycleFocusForward is 'w'.
    // Let's propose 'j' for CycleFocusForward (a Detail action). Should NOT conflict since they are in separate views.
    let conflict =
        defaults.find_conflict(crate::keybindings::Action::CycleFocusForward, &["j".to_string()]);
    assert!(conflict.is_none());

    // 4. Proposing a global key mapping conflicts with both contexts
    // '?' is default for DetailHelp (Detail view action). Proposing '?' for global Help should conflict.
    let conflict = defaults.find_conflict(crate::keybindings::Action::Help, &["?".to_string()]);
    assert_eq!(conflict, Some(crate::keybindings::Action::DetailHelp));
}

#[test]
fn test_keybindings_conflict_revert_settings() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config::default();

    let temp_dir = std::env::temp_dir().join("gitwig_test_dir_keybindings_conflict");
    let _ = std::fs::create_dir_all(&temp_dir);
    let temp_path = temp_dir.join("config.toml");

    let mut app = App::new(config, temp_path);

    // Enter settings
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('s')), 10);
    assert_eq!(app.mode, Mode::Settings);

    // Jump to keybindings category
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('5')), 10);
    assert_eq!(app.settings_selected_index, 16); // Quit / Close Dialog

    // Navigate to Help (index 15)
    crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
    assert_eq!(app.settings_selected_index, 15);

    // Start editing Help key (default is '?')
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(app.settings_editing);

    // Propose 'ctrl-q' (which is already bound to Quit/Close in the same section)
    app.input_buffer.clear();
    for c in "ctrl-q".chars() {
        crate::input::handle_key(&mut app, key_event(KeyCode::Char(c)), 10);
    }
    assert_eq!(app.input_buffer, "ctrl-q");

    // Commit change -> should fail, show conflict warning, and revert!
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);

    // Should NOT be editing anymore (reverted)
    assert!(!app.settings_editing);
    assert!(app.input_buffer.is_empty());

    // Status message should indicate conflict and mention both the action and its section
    let status = app.status_message.clone().unwrap_or_default();
    assert!(status.contains("Conflict: key already bound to 'Quit / Close Dialog'"));
    assert!(status.contains("in 'Global & Navigation'"));

    // Verify keybind was NOT updated (still '?')
    assert!(app.is_bound(crate::keybindings::Action::Help, key_event(KeyCode::Char('?'))));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_clear_debug_logs_popup() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let config = Config::default();
    let temp_dir = std::env::temp_dir().join("gitwig_test_clear_logs");
    let _ = std::fs::create_dir_all(&temp_dir);
    let temp_path = temp_dir.join("config.toml");
    let mut app = App::new(config, temp_path.clone());

    // Write a test log
    crate::debug_log::info("Test Log Entry");
    assert!(crate::debug_log::get_logs().iter().any(|l| l.contains("Test Log Entry")));

    // Change app mode to DebugLogs
    app.mode = Mode::DebugLogs;
    app.debug_log_scroll = 5;

    // Simulate pressing 'c' key in DebugLogs mode
    crate::popups::debug::DebugLogsPopup::handle_event(&mut app, key_event(KeyCode::Char('c')));

    // The specific test log should be gone now (other concurrent tests might have added new logs)
    assert!(!crate::debug_log::get_logs().iter().any(|l| l.contains("Test Log Entry")));
    // Scroll should reset to 0
    assert_eq!(app.debug_log_scroll, 0);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_skip_key_logging_in_commit_input() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let config = Config::default();
    let temp_dir = std::env::temp_dir().join("gitwig_test_skip_key_logging");
    let _ = std::fs::create_dir_all(&temp_dir);
    let temp_path = temp_dir.join("config.toml");
    let mut app = App::new(config, temp_path.clone());

    // Clear logs first so we have a clean state to verify
    crate::debug_log::clear();

    // 1. In CommitInput mode, key presses should NOT be logged
    app.mode = Mode::CommitInput;
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('x')), 10);
    assert!(!crate::debug_log::get_logs().iter().any(|l| l.contains("Key pressed: Char('x')")));

    // 2. In Normal mode, key presses SHOULD be logged
    app.mode = Mode::Normal;
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('y')), 10);
    assert!(crate::debug_log::get_logs().iter().any(|l| l.contains("Key pressed: Char('y')")));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_debug_logs_fuzzy_search() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let config = Config::default();
    let temp_dir = std::env::temp_dir().join("gitwig_test_debug_search");
    let _ = std::fs::create_dir_all(&temp_dir);
    let temp_path = temp_dir.join("config.toml");
    let mut app = App::new(config, temp_path.clone());

    // Populate with known log entries
    crate::debug_log::clear();
    crate::debug_log::info("Alpha Log Entry");
    crate::debug_log::info("Beta Log Entry");
    crate::debug_log::info("Gamma Log Entry");

    app.mode = Mode::DebugLogs;

    // Verify initial state: no search query, search is not active
    assert!(app.debug_log_search_query.is_none());
    assert!(!app.debug_log_search_editing);

    // 1. Press '/' to enter search mode
    crate::popups::debug::DebugLogsPopup::handle_event(&mut app, key_event(KeyCode::Char('/')));
    assert!(app.debug_log_search_query.is_some());
    assert!(app.debug_log_search_editing);
    assert_eq!(app.debug_log_search_query.as_deref(), Some(""));

    // 2. Type "be" to filter logs
    crate::popups::debug::DebugLogsPopup::handle_event(&mut app, key_event(KeyCode::Char('b')));
    crate::popups::debug::DebugLogsPopup::handle_event(&mut app, key_event(KeyCode::Char('e')));
    assert_eq!(app.debug_log_search_query.as_deref(), Some("be"));

    // 3. Press Enter to lock/exit editing but keep filter
    crate::popups::debug::DebugLogsPopup::handle_event(&mut app, key_event(KeyCode::Enter));
    assert!(app.debug_log_search_query.is_some());
    assert!(!app.debug_log_search_editing);

    // 4. Press Esc to clear filter
    crate::popups::debug::DebugLogsPopup::handle_event(&mut app, key_event(KeyCode::Esc));
    assert!(app.debug_log_search_query.is_none());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_keybindings_backfill_adds_actions_missing_from_an_older_file() {
    use crate::keybindings::{Action, KeybindingsConfig};

    let dir = std::env::temp_dir().join(format!(
        "gitwig_kb_backfill_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let _guard = TestDirGuard { path: dir.clone() };

    // An older config that predates HomeFetchDetails, with a customised binding
    // that the migration must not touch.
    std::fs::write(
        dir.join("keybindings.toml"),
        "[home.fetch_all]\nkeys = [\"G\"]\ndescription = \"Fetch all\"\n",
    )
    .unwrap();

    let loaded = KeybindingsConfig::load(&dir);

    assert_eq!(
        loaded.get_action_keys(Action::HomeFetchAll),
        vec!["G".to_string()],
        "user's own binding must survive the migration"
    );
    assert!(
        !loaded.get_action_keys(Action::HomeFetchDetails).is_empty(),
        "newly added action must be backfilled instead of rendering as '-'"
    );
}

#[test]
fn test_label_settings_repo_beats_label_beats_global() {
    // Global page_size = 10, label "work" sets 20, repo override sets 30.
    let repo_cfg = RepoConfig { page_size: Some(30), ..Default::default() };
    let label_cfg = crate::config::LabelConfig { page_size: Some(20), ..Default::default() };
    let (mut app, _p) =
        label_test_app("lbl_precedence", vec!["work"], Some(repo_cfg), vec![("work", label_cfg)]);
    app.config.page_size = 10;
    app.selected_index = 0;
    // Repo override wins.
    assert_eq!(app.get_current_page_size(), 30);

    // Drop the repo override: the label value applies.
    app.config.repo_configs.clear();
    assert_eq!(app.get_current_page_size(), 20);

    // Drop the label config: fall back to the global default.
    app.config.label_configs.clear();
    assert_eq!(app.get_current_page_size(), 10);
}

#[test]
fn test_label_settings_first_label_with_value_wins() {
    // Repo carries [a, b] in that order. Only b sets max_commits → b applies.
    // Then a also sets it → a (first) wins.
    let cfg_b = crate::config::LabelConfig { max_commits: Some(50), ..Default::default() };
    let (mut app, _p) =
        label_test_app("lbl_order", vec!["a", "b"], None, vec![("b", cfg_b.clone())]);
    app.config.max_commits = 500;
    app.selected_index = 0;
    assert_eq!(app.get_current_max_commits(), 50);

    app.config
        .label_configs
        .insert("a".to_string(), crate::config::LabelConfig { max_commits: Some(7), ..cfg_b });
    assert_eq!(app.get_current_max_commits(), 7);
}

#[test]
fn test_label_auto_fetch_disabled_via_label() {
    // A label can turn off auto-fetch for the whole group with Some(0).
    let label_cfg =
        crate::config::LabelConfig { auto_fetch_interval_mins: Some(0), ..Default::default() };
    let (mut app, _p) =
        label_test_app("lbl_autofetch", vec!["archive"], None, vec![("archive", label_cfg)]);
    app.config.auto_fetch_interval_mins = 10;
    assert_eq!(app.effective_auto_fetch_interval_mins("/repo"), 0);

    // A repo with no matching label still inherits the global cadence.
    assert_eq!(app.effective_auto_fetch_interval_mins("/other"), 10);
}

#[test]
fn test_effective_theme_name_resolution() {
    let repo_cfg = RepoConfig { theme: Some("dracula".to_string()), ..Default::default() };
    let label_cfg =
        crate::config::LabelConfig { theme: Some("nord".to_string()), ..Default::default() };
    let (mut app, _p) =
        label_test_app("lbl_theme", vec!["work"], Some(repo_cfg), vec![("work", label_cfg)]);
    // Repo override wins.
    assert_eq!(app.effective_theme_name("/repo").as_deref(), Some("dracula"));

    // Without a repo override, the label theme applies.
    app.config.repo_configs.clear();
    assert_eq!(app.effective_theme_name("/repo").as_deref(), Some("nord"));

    // With neither, there is no per-repo theme (global default renders).
    app.config.label_configs.clear();
    assert_eq!(app.effective_theme_name("/repo"), None);
}

#[test]
fn test_effective_editor_resolution() {
    let label_cfg =
        crate::config::LabelConfig { editor: Some("nano".to_string()), ..Default::default() };
    let (mut app, _p) = label_test_app("lbl_editor", vec!["work"], None, vec![("work", label_cfg)]);
    app.config.editor = "vim".to_string();
    assert_eq!(app.effective_editor("/repo"), "nano");

    // Repo override beats the label.
    app.config.repo_configs.insert(
        "/repo".to_string(),
        RepoConfig { editor: Some("hx".to_string()), ..Default::default() },
    );
    assert_eq!(app.effective_editor("/repo"), "hx");

    // Unlabeled repo falls back to global.
    assert_eq!(app.effective_editor("/other"), "vim");
}

#[test]
fn test_prune_orphaned_label_configs_on_persist() {
    // "work" is live (carried by /repo); "dead" is carried by nobody.
    let (mut app, path) = label_test_app(
        "lbl_prune",
        vec!["work"],
        None,
        vec![
            ("work", crate::config::LabelConfig { page_size: Some(5), ..Default::default() }),
            ("dead", crate::config::LabelConfig { page_size: Some(9), ..Default::default() }),
        ],
    );
    let _guard = TestFileGuard { path };
    app.persist("saved");
    assert!(app.config.label_configs.contains_key("work"));
    assert!(!app.config.label_configs.contains_key("dead"), "orphaned label config must be pruned");
}

#[test]
fn test_label_config_toml_roundtrip() {
    let mut config = Config { items: vec!["/repo".to_string()], ..Default::default() };
    config.label_configs.insert(
        "work".to_string(),
        crate::config::LabelConfig {
            theme: Some("nord".to_string()),
            page_size: Some(25),
            auto_fetch_interval_mins: Some(0),
            resync_on_tab_change: Some(true),
            ..Default::default()
        },
    );
    let serialized = toml::to_string_pretty(&config).unwrap();
    assert!(serialized.contains("[label_configs.work]"));
    let parsed: Config = toml::from_str(&serialized).unwrap();
    let lc = parsed.label_configs.get("work").unwrap();
    assert_eq!(lc.theme.as_deref(), Some("nord"));
    assert_eq!(lc.page_size, Some(25));
    assert_eq!(lc.auto_fetch_interval_mins, Some(0));
    assert_eq!(lc.resync_on_tab_change, Some(true));
    assert_eq!(lc.max_commits, None);
}

#[test]
fn test_label_settings_popup_open_from_picker() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let (mut app, _p) = label_test_app(
        "lbl_open",
        vec!["web"],
        None,
        vec![("web", crate::config::LabelConfig::default())],
    );
    app.mode = Mode::LabelPicker;
    // Matches are [(None, "All"), (Some("web"), 1)]; highlight the label row.
    app.label_picker_selection = 1;

    let right = KeyEvent::new(KeyCode::Right, KeyModifiers::empty());
    crate::input::handle_key(&mut app, right, 1);
    assert_eq!(app.mode, Mode::LabelSettings);
    assert_eq!(app.label_settings_target.as_deref(), Some("web"));

    // Right on the "All repositories" row does not open settings.
    app.mode = Mode::LabelPicker;
    app.label_settings_target = None;
    app.label_picker_selection = 0;
    crate::input::handle_key(&mut app, right, 1);
    assert_eq!(app.mode, Mode::LabelPicker);
    assert!(app.label_settings_target.is_none());
}

#[test]
fn test_label_settings_popup_edit_and_toggle() {
    use crate::popups::label_settings::LabelSettingsPopup;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let (mut app, path) = label_test_app("lbl_edit", vec!["web"], None, vec![]);
    let _guard = TestFileGuard { path };
    app.mode = Mode::LabelSettings;
    app.label_settings_target = Some("web".to_string());
    app.label_settings_selected_index = 0;

    // Navigate down to Resync (row 3) and toggle it right: None → Yes.
    for _ in 0..3 {
        LabelSettingsPopup::handle_event(
            &mut app,
            KeyEvent::new(KeyCode::Down, KeyModifiers::empty()),
        );
    }
    assert_eq!(app.label_settings_selected_index, 3);
    LabelSettingsPopup::handle_event(
        &mut app,
        KeyEvent::new(KeyCode::Right, KeyModifiers::empty()),
    );
    assert_eq!(app.config.label_configs.get("web").unwrap().resync_on_tab_change, Some(true));

    // Edit Page Size (row 1): open editor, type 42, confirm.
    app.label_settings_selected_index = 1;
    LabelSettingsPopup::handle_event(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
    );
    assert!(app.label_settings_editing);
    for c in ['4', '2'] {
        LabelSettingsPopup::handle_event(
            &mut app,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()),
        );
    }
    LabelSettingsPopup::handle_event(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
    );
    assert!(!app.label_settings_editing);
    assert_eq!(app.config.label_configs.get("web").unwrap().page_size, Some(42));

    // Esc returns to the label picker.
    LabelSettingsPopup::handle_event(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.mode, Mode::LabelPicker);
}

#[test]
fn test_label_settings_theme_cycle_reaches_all_themes() {
    // Regression: cycling the Theme row must reach every installed theme, not
    // just the ones sorting before "default". A duplicate "default" in the
    // cycle list used to snap `position` back to index 0 mid-cycle, so themes
    // sorting after "default" (like "zzz") were unreachable.
    use crate::popups::label_settings::LabelSettingsPopup;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let dir = std::env::temp_dir().join(format!("gitwig_test_theme_cycle_{}", std::process::id()));
    let themes_dir = dir.join("themes");
    std::fs::create_dir_all(&themes_dir).unwrap();
    // Names chosen to straddle "default" alphabetically: alpha < default < zzz.
    for name in ["default", "alpha", "zzz"] {
        std::fs::write(themes_dir.join(format!("{}.theme", name)), "accent = \"cyan\"\n").unwrap();
    }
    let _guard = TestDirGuard { path: dir.clone() };

    let mut config =
        Config { items: vec!["/repo".to_string()], show_grouping: false, ..Default::default() };
    config.labels.insert("/repo".to_string(), vec!["web".to_string()]);
    let mut app = App::new(config, dir.join("config.toml"));
    app.mode = Mode::LabelSettings;
    app.label_settings_target = Some("web".to_string());
    app.label_settings_selected_index = 0; // Theme row.

    // Cycle right several times, recording each theme the label lands on.
    let mut seen = std::collections::HashSet::new();
    for _ in 0..6 {
        LabelSettingsPopup::handle_event(
            &mut app,
            KeyEvent::new(KeyCode::Right, KeyModifiers::empty()),
        );
        let theme = app
            .config
            .label_configs
            .get("web")
            .and_then(|lc| lc.theme.clone())
            .unwrap_or_else(|| "default".to_string());
        seen.insert(theme);
    }

    // All three themes must be reachable — crucially "zzz", which sorts after
    // "default" and was unreachable before the fix.
    assert!(seen.contains("alpha"), "reachable themes: {:?}", seen);
    assert!(seen.contains("default"), "reachable themes: {:?}", seen);
    assert!(seen.contains("zzz"), "reachable themes: {:?}", seen);
}

#[test]
fn test_label_theme_cache_populated_for_active_filter() {
    // A label that sets a theme gets its ThemeConfig cached (keyed by label) so
    // the home list can be tinted while that label filter is active.
    let dir = std::env::temp_dir().join(format!("gitwig_test_label_theme_{}", std::process::id()));
    let themes_dir = dir.join("themes");
    std::fs::create_dir_all(&themes_dir).unwrap();
    std::fs::write(themes_dir.join("nord.theme"), "accent = \"lightblue\"\n").unwrap();
    let _guard = TestDirGuard { path: dir.clone() };

    let mut config =
        Config { items: vec!["/repo".to_string()], show_grouping: false, ..Default::default() };
    config.labels.insert("/repo".to_string(), vec!["web".to_string()]);
    config.label_configs.insert(
        "web".to_string(),
        crate::config::LabelConfig { theme: Some("nord".to_string()), ..Default::default() },
    );
    let mut app = App::new(config, dir.join("config.toml"));
    app.resolve_repo_themes();

    let cached = app.label_theme_cache.get("web").expect("web label theme should be cached");
    assert_eq!(cached.accent, "lightblue");

    // A label with no theme set is absent from the cache.
    app.config.label_configs.insert("plain".to_string(), crate::config::LabelConfig::default());
    app.resolve_repo_themes();
    assert!(!app.label_theme_cache.contains_key("plain"));
}

/// The Settings row for the startup prompt must toggle the real config flag.
/// Settings ids are hand-assigned and the handler ends in an `idx >= 14`
/// catch-all that routes to keybindings, so a new id has to be both unused and
/// below that threshold — this pins the wiring.
#[test]
fn test_settings_toggles_prompt_cwd_repo() {
    let cfg_path = std::env::temp_dir().join("gitwig_test_settings_prompt_cwd.toml");
    let _guard = TestFileGuard { path: cfg_path.clone() };
    let mut app = App::new(Config { items: vec![], ..Default::default() }, cfg_path);

    assert!(app.config.prompt_cwd_repo, "the prompt should default to on");

    app.settings_selected_index = 11;
    app.toggle_or_edit_setting();
    assert!(!app.config.prompt_cwd_repo, "selecting the row should turn the prompt off");
    assert!(
        !app.settings_editing,
        "a boolean row toggles in place; it must not drop into text-edit mode"
    );

    app.toggle_or_edit_setting();
    assert!(app.config.prompt_cwd_repo, "selecting it again should turn the prompt back on");
}

/// The reported case: editing a repo's labels and removing one from the middle
/// of the comma-separated list. Before caret support the field was append-only,
/// so `Left` did nothing and only the last label could be trimmed.
#[test]
fn test_label_input_caret_edits_middle_of_list() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let cfg_path = std::env::temp_dir().join("gitwig_test_label_caret.toml");
    let _guard = TestFileGuard { path: cfg_path.clone() };
    let mut app =
        App::new(Config { items: vec!["/dummy/repo".to_string()], ..Default::default() }, cfg_path);
    app.mode = Mode::LabelInput;
    app.set_input_buffer("work,rust,cli".to_string());
    assert_eq!(
        app.input_cursor_clamped(),
        13,
        "a pre-filled field opens with the caret at the end"
    );

    let press = |app: &mut App, code| {
        crate::input::handle_key(app, KeyEvent::new(code, KeyModifiers::empty()), 0)
    };

    // Step back over "cli" and its comma, then delete "rust," with backspace.
    for _ in 0..4 {
        assert!(press(&mut app, KeyCode::Left), "Left must be consumed by the input");
    }
    assert_eq!(app.input_cursor_clamped(), 9);
    for _ in 0..5 {
        press(&mut app, KeyCode::Backspace);
    }
    assert_eq!(app.input_buffer, "work,cli", "the middle label should be gone");
    assert_eq!(app.input_cursor_clamped(), 4, "caret stays where the text was removed");

    // Typing inserts at the caret rather than appending.
    for c in ",go".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    assert_eq!(app.input_buffer, "work,go,cli", "a new label lands mid-list, not at the end");

    // Home/End jump the caret; Delete removes forwards.
    assert!(press(&mut app, KeyCode::Home));
    assert_eq!(app.input_cursor_clamped(), 0);
    press(&mut app, KeyCode::Delete);
    assert_eq!(app.input_buffer, "ork,go,cli", "Delete removes the character under the caret");
    assert!(press(&mut app, KeyCode::End));
    assert_eq!(app.input_cursor_clamped(), app.input_buffer.chars().count());

    // Backspace at the start and Delete at the end are no-ops, not panics.
    press(&mut app, KeyCode::Delete);
    assert_eq!(app.input_buffer, "ork,go,cli");
    press(&mut app, KeyCode::Home);
    press(&mut app, KeyCode::Backspace);
    assert_eq!(app.input_buffer, "ork,go,cli");
}

/// The caret counts characters, so multi-byte labels must not split a `char`
/// boundary (which would panic on `String::remove`/`insert`).
#[test]
fn test_input_caret_is_utf8_safe() {
    let cfg_path = std::env::temp_dir().join("gitwig_test_caret_utf8.toml");
    let _guard = TestFileGuard { path: cfg_path.clone() };
    let mut app = App::new(Config { items: vec![], ..Default::default() }, cfg_path);
    app.mode = Mode::LabelInput;
    app.set_input_buffer("café,naïve,日本".to_string());

    app.input_home();
    for _ in 0..4 {
        app.input_right();
    }
    app.input_backspace();
    assert_eq!(app.input_buffer, "caf,naïve,日本", "the accented char is removed whole");

    app.input_end();
    app.input_backspace();
    assert_eq!(app.input_buffer, "caf,naïve,日");

    app.input_home();
    app.input_char('é');
    assert_eq!(app.input_buffer, "écaf,naïve,日");
}

/// `input_buffer` is assigned directly from well over a hundred places. When
/// that happens the caret must land at the end of the new text instead of
/// pointing into the old one.
#[test]
fn test_caret_snaps_to_end_when_buffer_replaced_directly() {
    let cfg_path = std::env::temp_dir().join("gitwig_test_caret_external.toml");
    let _guard = TestFileGuard { path: cfg_path.clone() };
    let mut app = App::new(Config { items: vec![], ..Default::default() }, cfg_path);
    app.mode = Mode::LabelInput;

    // Caret parked at 0 by a cleared field, then a pre-filled value arrives.
    app.cancel_input();
    app.mode = Mode::LabelInput;
    app.input_buffer = "alpha,beta".to_string();
    assert_eq!(app.input_cursor_clamped(), 10, "caret should follow an external assignment");

    app.input_backspace();
    assert_eq!(app.input_buffer, "alpha,bet", "backspace must act on the end, not the start");
}

/// Fuzzy pickers use the arrow keys to move their selection. Caret handling
/// must not swallow those.
#[test]
fn test_caret_keys_do_not_hijack_picker_arrows() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let cfg_path = std::env::temp_dir().join("gitwig_test_caret_pickers.toml");
    let _guard = TestFileGuard { path: cfg_path.clone() };
    let mut app = App::new(Config { items: vec![], ..Default::default() }, cfg_path);

    for mode in [Mode::RepoJump, Mode::LabelPicker, Mode::GlobalSearch, Mode::RepoSearchInput] {
        app.mode = mode;
        app.set_input_buffer("abc".to_string());
        app.input_cursor = 3;
        crate::input::handle_key(&mut app, KeyEvent::new(KeyCode::Left, KeyModifiers::empty()), 0);
        assert_eq!(
            app.input_cursor_clamped(),
            3,
            "{:?} uses arrows for selection; the caret must not move",
            mode
        );
    }
}

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// ── Command palette ──────────────────────────────────────────────────────────

#[test]
fn test_command_palette_opens_on_home_and_runs_the_highlighted_action() {
    use crate::keybindings::Action;
    let (mut app, _guard) = palette_test_app("home_run");

    assert!(app.command_palette.is_none());
    assert!(crate::input::handle_key(&mut app, palette_key(), 5));
    let palette = app.command_palette.as_ref().expect("palette opens on the home screen");
    assert_eq!(app.mode, Mode::Normal, "the palette is an overlay, not a mode");

    // Home actions come first, then the globals; movement keys are left out.
    assert_eq!(palette.entries.first().map(|e| e.group), Some("Home"));
    assert_eq!(palette.entries.last().map(|e| e.group), Some("Global"));
    assert!(palette.entries.iter().any(|e| e.action == Action::HomeOpenSettings));
    assert!(palette.entries.iter().any(|e| e.action == Action::Help));
    assert!(!palette.entries.iter().any(|e| e.action == Action::HomeMoveDown));
    assert!(!palette.entries.iter().any(|e| e.action == Action::BranchesCheckout));
    // Every entry shows its description and its current key.
    let settings = palette.entries.iter().find(|e| e.action == Action::HomeOpenSettings).unwrap();
    assert_eq!(settings.label, app.keybindings.get_action_description(Action::HomeOpenSettings));
    assert_eq!(settings.keys.as_deref(), Some("s"));
    let label = settings.label.clone();

    // Typing narrows the list and Enter runs the top match exactly like `s` would.
    type_into_palette(&mut app, &label.to_lowercase());
    let palette = app.command_palette.as_ref().unwrap();
    assert_eq!(palette.query, label.to_lowercase());
    assert_eq!(palette.selected().map(|e| e.action), Some(Action::HomeOpenSettings));
    assert!(crate::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        5
    ));
    assert!(app.command_palette.is_none(), "the palette closes before running");
    assert_eq!(app.mode, Mode::Settings);
    assert!(app.forced_action.is_none(), "the forced action never outlives the dispatch");
}

#[test]
fn test_command_palette_closes_on_esc_or_its_own_key_without_side_effects() {
    let (mut app, _guard) = palette_test_app("close");

    assert!(crate::input::handle_key(&mut app, palette_key(), 5));
    type_into_palette(&mut app, "quit");
    assert!(crate::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        5
    ));
    assert!(app.command_palette.is_none());
    assert_eq!(app.mode, Mode::Normal);
    assert_eq!(app.selected_index, 0);

    // ctrl-p toggles it shut too, and a fresh open starts with an empty query.
    assert!(crate::input::handle_key(&mut app, palette_key(), 5));
    type_into_palette(&mut app, "abc");
    assert!(crate::input::handle_key(&mut app, palette_key(), 5));
    assert!(app.command_palette.is_none());
    assert!(crate::input::handle_key(&mut app, palette_key(), 5));
    assert_eq!(app.command_palette.as_ref().unwrap().query, "");

    // Enter on an empty match list closes without running anything.
    type_into_palette(&mut app, "zzzz-no-such-action");
    assert!(app.command_palette.as_ref().unwrap().matches().is_empty());
    assert!(crate::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        5
    ));
    assert!(app.command_palette.is_none());
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn test_command_palette_navigation_keys_move_the_selection() {
    let (mut app, _guard) = palette_test_app("nav");
    app.config.page_size = 3;
    assert!(crate::input::handle_key(&mut app, palette_key(), 5));
    let total = app.command_palette.as_ref().unwrap().entries.len();
    let key = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    crate::input::handle_key(&mut app, key(KeyCode::Down), 5);
    crate::input::handle_key(&mut app, key(KeyCode::Down), 5);
    assert_eq!(app.command_palette.as_ref().unwrap().selection, 2);
    crate::input::handle_key(&mut app, key(KeyCode::Up), 5);
    assert_eq!(app.command_palette.as_ref().unwrap().selection, 1);
    crate::input::handle_key(&mut app, key(KeyCode::PageDown), 5);
    assert_eq!(app.command_palette.as_ref().unwrap().selection, 4);
    crate::input::handle_key(&mut app, key(KeyCode::End), 5);
    assert_eq!(app.command_palette.as_ref().unwrap().selection, total - 1);
    crate::input::handle_key(&mut app, key(KeyCode::Down), 5);
    assert_eq!(app.command_palette.as_ref().unwrap().selection, total - 1, "clamped at the end");
    crate::input::handle_key(&mut app, key(KeyCode::Home), 5);
    assert_eq!(app.command_palette.as_ref().unwrap().selection, 0);
    crate::input::handle_key(&mut app, key(KeyCode::Up), 5);
    assert_eq!(app.command_palette.as_ref().unwrap().selection, 0, "clamped at the top");

    // Typing resets the highlight to the best match; Backspace edits the query.
    crate::input::handle_key(&mut app, key(KeyCode::End), 5);
    type_into_palette(&mut app, "ab");
    assert_eq!(app.command_palette.as_ref().unwrap().selection, 0);
    crate::input::handle_key(&mut app, key(KeyCode::Backspace), 5);
    assert_eq!(app.command_palette.as_ref().unwrap().query, "a");
}

#[test]
fn test_command_palette_in_detail_view_lists_the_active_tab_and_runs_detail_actions() {
    use crate::keybindings::Action;
    let (mut app, _guard) = palette_test_app("detail");
    app.mode = Mode::Detail;
    app.detail_tab = 3; // Branches

    assert!(crate::input::handle_key(&mut app, palette_key(), 5));
    let palette = app.command_palette.as_ref().unwrap();
    assert_eq!(app.mode, Mode::Detail);
    assert_eq!(palette.entries.first().map(|e| e.group), Some("Branches"));
    assert!(palette.entries.iter().any(|e| e.action == Action::BranchesCheckout));
    assert!(palette.entries.iter().any(|e| e.action == Action::CycleTabForward));
    assert!(palette.entries.iter().any(|e| e.action == Action::DetailHelp));
    assert!(!palette.entries.iter().any(|e| e.action == Action::HomeAddRepo));
    assert!(!palette.entries.iter().any(|e| e.action == Action::TagsCheckout));
    assert!(!palette.entries.iter().any(|e| e.action == Action::Help));

    // Running "cycle tab forward" goes through the detail router like the key.
    let label = app.keybindings.get_action_description(Action::CycleTabForward).to_lowercase();
    type_into_palette(&mut app, &label);
    assert_eq!(
        app.command_palette.as_ref().unwrap().selected().map(|e| e.action),
        Some(Action::CycleTabForward)
    );
    assert!(crate::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        5
    ));
    assert!(app.command_palette.is_none());
    assert_eq!(app.detail_tab, 4);
    assert_eq!(app.mode, Mode::Detail);

    // The Graph tab has no actions of its own: only the repository and global groups remain.
    app.detail_tab = 2;
    assert!(crate::input::handle_key(&mut app, palette_key(), 5));
    let palette = app.command_palette.as_ref().unwrap();
    assert_eq!(palette.entries.first().map(|e| e.group), Some("Repository"));
    assert!(palette.entries.iter().all(|e| e.group == "Repository" || e.group == "Global"));
}

#[test]
fn test_command_palette_runs_rebound_and_unparsable_bindings() {
    use crate::keybindings::Action;
    let (mut app, _guard) = palette_test_app("unbound");

    // Rebound: the palette follows the new key.
    app.keybindings.update_action_keys(Action::HomeOpenSettings, vec!["ctrl-x".to_string()]);
    assert!(crate::input::handle_key(&mut app, palette_key(), 5));
    let entry = app
        .command_palette
        .as_ref()
        .unwrap()
        .entries
        .iter()
        .find(|e| e.action == Action::HomeOpenSettings)
        .unwrap()
        .clone();
    assert_eq!(entry.keys.as_deref(), Some("ctrl-x"));
    type_into_palette(&mut app, &entry.label.to_lowercase());
    assert!(crate::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        5
    ));
    assert_eq!(app.mode, Mode::Settings);

    // A binding the key parser rejects (an empty list would just restore the
    // defaults) still runs from the palette: the dispatcher sees a null key and
    // the forced action does the routing.
    app.mode = Mode::Normal;
    app.keybindings.home.open_settings =
        Some(crate::keybindings::Keybind::new(&["not-a-key"], "Open settings"));
    assert_eq!(app.keybindings.get_action_keys(Action::HomeOpenSettings), vec!["not-a-key"]);
    assert!(crate::keybindings::parse_key("not-a-key").is_none());
    assert!(crate::input::handle_key(&mut app, palette_key(), 5));
    let entry = app
        .command_palette
        .as_ref()
        .unwrap()
        .entries
        .iter()
        .find(|e| e.action == Action::HomeOpenSettings)
        .unwrap()
        .clone();
    assert_eq!(entry.keys.as_deref(), Some("not-a-key"));
    type_into_palette(&mut app, "open settings");
    assert!(crate::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        5
    ));
    assert_eq!(app.mode, Mode::Settings);
    assert!(app.forced_action.is_none());
}

#[test]
fn test_command_palette_is_not_available_in_text_inputs_or_other_modes() {
    let (mut app, _guard) = palette_test_app("gated");

    // A text field keeps ctrl-p for itself.
    app.mode = Mode::RepoSearchInput;
    crate::input::handle_key(&mut app, palette_key(), 5);
    assert!(app.command_palette.is_none());

    // Modes without a palette explain instead of opening one.
    app.mode = Mode::Settings;
    app.status_message = None;
    crate::input::handle_key(&mut app, palette_key(), 5);
    assert!(app.command_palette.is_none());
    assert!(app.status_message.as_deref().unwrap_or("").contains("command palette"));

    // Mouse input is ignored while the palette is open.
    app.mode = Mode::Normal;
    app.status_message = None;
    assert!(crate::input::handle_key(&mut app, palette_key(), 5));
    app.main_areas =
        vec![ratatui::layout::Rect::new(0, 0, 50, 4), ratatui::layout::Rect::new(0, 4, 50, 4)];
    crate::mouse::handle_mouse(
        &mut app,
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 5,
            row: 5,
            modifiers: KeyModifiers::empty(),
        },
    );
    assert!(app.command_palette.is_some());
    assert_eq!(app.selected_index, 0);
}

#[test]
fn test_command_palette_quit_entry_ends_the_app() {
    use crate::keybindings::Action;
    let (mut app, _guard) = palette_test_app("quit");
    assert!(crate::input::handle_key(&mut app, palette_key(), 5));
    let label = app.keybindings.get_action_description(Action::Close).to_lowercase();
    type_into_palette(&mut app, &label);
    assert_eq!(
        app.command_palette.as_ref().unwrap().selected().map(|e| e.action),
        Some(Action::Close)
    );
    let keep_running =
        crate::input::handle_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()), 5);
    assert!(!keep_running, "running Quit from the palette returns the quit signal");
}

#[test]
fn test_command_palette_every_entry_dispatches_without_panicking() {
    use crate::keybindings::Action;
    // Fresh app per entry so one action's side effects (mode changes, pending
    // flags, confirmations) never leak into the next. The two terminal actions
    // spawn a real shell in a PTY and the update check reaches the network, so
    // those three are exercised by their own tests instead.
    let skip = [Action::HomeOpenTerminal, Action::ToggleTerminalPanel, Action::HomeCheckUpdate];
    let (_, _guard) = palette_test_app("smoke_dir");
    let temp_dir = _guard.path.clone();
    let fresh = |mode: Mode, tab: usize| -> App {
        let config = Config {
            items: vec!["/path/to/frontend".to_string(), "/path/to/backend".to_string()],
            ..Default::default()
        };
        let mut app = App::new(config, temp_dir.join("config.toml"));
        app.mode = mode;
        app.detail_tab = tab;
        app
    };

    let mut contexts = vec![(Mode::Normal, 0)];
    contexts.extend((0..=11).map(|tab| (Mode::Detail, tab)));
    let mut ran = 0;
    for (mode, tab) in contexts {
        let mut probe = fresh(mode, tab);
        probe.open_command_palette();
        let entries = probe.command_palette.as_ref().unwrap().entries.clone();
        assert!(!entries.is_empty(), "no entries for {:?} tab {}", mode, tab);
        for entry in entries {
            if skip.contains(&entry.action) {
                continue;
            }
            let mut app = fresh(mode, tab);
            app.open_command_palette();
            app.command_palette.as_mut().unwrap().query = entry.label.clone();
            let keep_running = crate::input::handle_key(
                &mut app,
                KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
                5,
            );
            assert_eq!(
                keep_running,
                entry.action != Action::Close,
                "{:?} in {:?} tab {} returned the wrong run signal",
                entry.action,
                mode,
                tab
            );
            assert!(app.command_palette.is_none(), "{:?} left the palette open", entry.action);
            assert!(app.forced_action.is_none(), "{:?} leaked forced_action", entry.action);
            ran += 1;
        }
    }
    assert!(ran > 80, "expected the palette to expose many actions, ran {}", ran);
}

#[test]
fn test_command_palette_status_bar_shows_its_hints_while_open() {
    let (mut app, _guard) = palette_test_app("status_bar");
    let render = |app: &App| -> String {
        let backend = ratatui::backend::TestBackend::new(120, 3);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                crate::components::cmd_bar::draw_status_bar(
                    f,
                    app,
                    ratatui::layout::Rect::new(0, 0, 120, 3),
                )
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..3)
            .map(|y| (0..120).map(|x| buffer[(x, y)].symbol().to_string()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    };

    assert!(!render(&app).contains("Command palette"));
    app.open_command_palette();
    let bar = render(&app);
    assert!(bar.contains("Command palette"), "bar:\n{}", bar);
    for hint in ["Select", "Run", "Enter"] {
        assert!(bar.contains(hint), "missing {:?} in bar:\n{}", hint, bar);
    }
    // Trailing entries fold into "More" when the bar runs out of width.
    assert!(bar.contains("Cancel") || bar.contains("More"), "bar:\n{}", bar);
    app.close_command_palette();
    assert!(!render(&app).contains("Command palette"));
}

#[test]
fn test_command_palette_is_listed_in_both_help_overlays() {
    let (mut app, _guard) = palette_test_app("help");
    let flatten = |lines: Vec<ratatui::text::Line<'_>>| -> String {
        lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.to_string()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase()
    };

    let home_help = flatten(crate::popups::help::get_help_lines(&app, 100));
    assert!(home_help.contains("command palette"), "home help:\n{}", home_help);
    assert!(home_help.contains("ctrl-p"), "home help:\n{}", home_help);

    app.mode = Mode::Detail;
    let detail_help = flatten(crate::popups::detail_help::get_detail_help_lines(&app, 100));
    assert!(detail_help.contains("command palette"), "detail help:\n{}", detail_help);
    assert!(detail_help.contains("ctrl-p"), "detail help:\n{}", detail_help);

    // A rebinding shows up in both overlays too.
    app.keybindings
        .update_action_keys(crate::keybindings::Action::CommandPalette, vec!["ctrl-k".to_string()]);
    let detail_help = flatten(crate::popups::detail_help::get_detail_help_lines(&app, 100));
    assert!(detail_help.contains("ctrl-k") && !detail_help.contains("ctrl-p"));
    app.mode = Mode::Normal;
    let home_help = flatten(crate::popups::help::get_help_lines(&app, 100));
    assert!(home_help.contains("ctrl-k") && !home_help.contains("ctrl-p"));
}

#[test]
fn test_command_palette_keys_follow_compatibility_mode() {
    use crate::keybindings::Action;
    let (mut app, _guard) = palette_test_app("compat");
    let open_detail_keys = |app: &mut App| -> Option<String> {
        app.open_command_palette();
        let keys = app
            .command_palette
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .find(|e| e.action == Action::HomeOpenDetail)
            .and_then(|e| e.keys.clone());
        app.close_command_palette();
        keys
    };
    app.config.compatibility_mode = false;
    assert_eq!(open_detail_keys(&mut app).as_deref(), Some("↵/right"));
    app.config.compatibility_mode = true;
    assert_eq!(open_detail_keys(&mut app).as_deref(), Some("Enter/right"));
}

#[test]
fn test_command_palette_action_is_registered_like_every_other_global() {
    use crate::keybindings::{Action, KeybindingsConfig};
    assert_eq!(Action::from_index(261), Some(Action::CommandPalette));
    assert_eq!(Action::CommandPalette.to_index(), 261);
    assert_eq!(KeybindingsConfig::get_default_keys(Action::CommandPalette), vec!["ctrl-p"]);
    let defaults = KeybindingsConfig::default_config();
    assert!(!defaults.get_action_description(Action::CommandPalette).is_empty());
    // As a global action it conflicts with home and detail bindings alike.
    assert_eq!(
        defaults.find_conflict(Action::HomeOpenSettings, &["ctrl-p".to_string()]),
        Some(Action::CommandPalette)
    );
    assert_eq!(
        defaults.find_conflict(Action::RefreshDetail, &["ctrl-p".to_string()]),
        Some(Action::CommandPalette)
    );
    // The Settings → Keybindings page lists it under Global & Navigation, and
    // every index on that page still resolves to an action.
    let globals = crate::popups::settings::GLOBAL_NAV_SETTING_INDICES;
    assert!(globals.contains(&261));
    for &idx in globals {
        assert!(Action::from_index(idx).is_some(), "settings index {} resolves to nothing", idx);
    }
}

#[test]
fn test_label_settings_popup_cycles_sort_rows() {
    use crate::popups::label_settings::LabelSettingsPopup;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let (mut app, path) = label_sort_app("lbl_sort_popup", crate::config::LabelConfig::default());
    let _guard = TestFileGuard { path };
    app.state.active_label_filter = Some("work".to_string());
    app.mode = Mode::LabelSettings;
    app.label_settings_target = Some("work".to_string());
    let right = KeyEvent::new(KeyCode::Right, KeyModifiers::empty());
    let left = KeyEvent::new(KeyCode::Left, KeyModifiers::empty());
    let sort_by = |app: &App| app.config.label_configs["work"].sort_by;
    let sort_rev = |app: &App| app.config.label_configs["work"].sort_reverse;

    // Up from the top wraps to the last row (View Mode); up again is Sort
    // Reverse, then Sort By.
    LabelSettingsPopup::handle_event(&mut app, KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
    assert_eq!(app.label_settings_selected_index, 8);
    LabelSettingsPopup::handle_event(&mut app, KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
    assert_eq!(app.label_settings_selected_index, 7);
    LabelSettingsPopup::handle_event(&mut app, KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
    assert_eq!(app.label_settings_selected_index, 6);

    // Sort By ring: default → Custom → Alphabetical → … → Latest Changes → default.
    LabelSettingsPopup::handle_event(&mut app, right);
    assert_eq!(sort_by(&app), Some(SortOrder::Custom));
    LabelSettingsPopup::handle_event(&mut app, right);
    assert_eq!(sort_by(&app), Some(SortOrder::Alphabetical));
    // The label view is active, so the list re-sorts immediately.
    assert_eq!(app.config.items, vec!["/a", "/b", "/c", "/z"]);
    LabelSettingsPopup::handle_event(&mut app, left);
    assert_eq!(sort_by(&app), Some(SortOrder::Custom));
    LabelSettingsPopup::handle_event(&mut app, left);
    assert_eq!(sort_by(&app), None);
    assert_eq!(app.config.items, vec!["/c", "/a", "/b", "/z"]);
    LabelSettingsPopup::handle_event(&mut app, left);
    assert_eq!(sort_by(&app), Some(SortOrder::LatestChanges));
    LabelSettingsPopup::handle_event(&mut app, right);
    assert_eq!(sort_by(&app), None);

    // Sort Reverse ring: default → yes → no → default; Enter steps right too.
    app.label_settings_selected_index = 7;
    LabelSettingsPopup::handle_event(&mut app, right);
    assert_eq!(sort_rev(&app), Some(true));
    LabelSettingsPopup::handle_event(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
    );
    assert_eq!(sort_rev(&app), Some(false));
    LabelSettingsPopup::handle_event(&mut app, right);
    assert_eq!(sort_rev(&app), None);
    LabelSettingsPopup::handle_event(&mut app, left);
    assert_eq!(sort_rev(&app), Some(false));
    assert!(!app.label_settings_editing, "sort rows never enter text editing");
}

#[test]
fn test_label_config_sort_round_trips_through_toml() {
    let mut config = Config::default();
    config.label_configs.insert(
        "work".to_string(),
        crate::config::LabelConfig {
            sort_by: Some(SortOrder::RecentVisit),
            sort_reverse: Some(true),
            ..Default::default()
        },
    );
    let serialized = toml::to_string(&config).unwrap();
    assert!(serialized.contains("sort_by = \"recent_visit\""), "{serialized}");
    let parsed: Config = toml::from_str(&serialized).unwrap();
    let lc = &parsed.label_configs["work"];
    assert_eq!(lc.sort_by, Some(SortOrder::RecentVisit));
    assert_eq!(lc.sort_reverse, Some(true));

    // Older files without the keys still load, inheriting the global sort.
    let legacy: Config =
        toml::from_str("items = []\n[label_configs.work]\ntheme = \"nord\"\n").unwrap();
    assert_eq!(legacy.label_configs["work"].sort_by, None);
    assert_eq!(legacy.label_configs["work"].sort_reverse, None);
}

#[test]
fn test_is_nix_store_path() {
    assert!(super::is_nix_store_path(std::path::Path::new(
        "/nix/store/abc123-gitwig-2.6.7/bin/gitwig"
    )));
    assert!(!super::is_nix_store_path(std::path::Path::new("/usr/local/bin/gitwig")));
}

#[test]
fn test_split_command_words() {
    assert_eq!(super::split_command_words("vim"), vec!["vim"]);
    assert_eq!(super::split_command_words("code --wait"), vec!["code", "--wait"]);
    assert_eq!(super::split_command_words("nvim -u NONE"), vec!["nvim", "-u", "NONE"]);
    assert_eq!(
        super::split_command_words("\"/Applications/Visual Studio Code.app/bin/code\" --wait"),
        vec!["/Applications/Visual Studio Code.app/bin/code", "--wait"]
    );
    assert_eq!(super::split_command_words("'/my editor/bin' -f"), vec!["/my editor/bin", "-f"]);
    assert!(super::split_command_words("   ").is_empty());
}
