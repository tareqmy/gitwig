#![allow(clippy::unwrap_used, clippy::panic)]

use super::*;

#[test]
fn test_stash_creation_flow() {
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
fn test_commit_input_scroll() {
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
fn test_commit_amend_flow() {
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
fn test_workspace_tab_right_arrow_inspect() {
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
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: missing_repo_path("workspace_tab_right_arrow_inspect"),
        info: Box::new(info),
    });
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
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: missing_repo_path("commit_enter_key_inspect"),
        info: Box::new(info),
    });
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
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: missing_repo_path("inspect_commit_shortcut"),
        info: Box::new(info),
    });
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
    let temp_dir = std::env::temp_dir().join("gitwig_test_workspace_all_changes_dir");
    let _ = std::fs::create_dir_all(&temp_dir);
    let temp_path = temp_dir.join("config.toml");
    let _guard = TestDirGuard { path: temp_dir };
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
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: missing_repo_path("workspace_all_changes_shortcuts"),
        info: Box::new(info),
    });
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
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: missing_repo_path("inspect_workspace_all_changes_shortcuts"),
        info: Box::new(info),
    });
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
    let _guard = TestDirGuard { path: temp_path.clone() };
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
    assert_eq!(app.detail_focus, DetailSection::Staged, "error: {:?}", app.error_message);

    // 2. Unstage All Focus Transition (Staged -> Unstaged)
    app.detail_focus = DetailSection::Staged;
    app.unstage_all_changes();
    assert_eq!(app.detail_focus, DetailSection::Unstaged, "error: {:?}", app.error_message);
}

#[test]
fn test_workspace_tab_focus_cycle_skips_empty_panels() {
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
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: missing_repo_path("workspace_tab_focus_cycle_skips_empty_panels"),
        info: Box::new(info),
    });
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
        resolved: missing_repo_path("workspace_tab_focus_cycle_skips_empty_panels"),
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
fn test_inspect_full_screen_diff_toggle() {
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
fn test_yank_selected_commit_hash() {
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
    let (config_path, _guard) = temp_config_path("yank_selected_commit_hash");
    let mut app = App::new(config, config_path);

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

    // Try yanking. Clipboard access fails in headless envs (CI); success lands
    // in the status bar while failure goes to the error popup via set_error.
    app.yank_selected_commit_hash();
    let success = app.status_message.as_ref().is_some_and(|m| m.contains("Copied hash abc1234"));
    let failure = app.error_message.as_ref().is_some_and(|m| m.contains("Failed to copy"));
    assert!(success || failure);
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

    // Test Ctrl+U in editing mode -> clears buffer
    app.commit_popup.input_buffer = "hello world".to_string();
    app.commit_popup.cursor_idx = 5;
    let ctrl_u = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL);
    let handled = crate::input::handle_key(&mut app, ctrl_u, 0);
    assert!(handled);
    assert!(app.commit_popup.input_buffer.is_empty());
    assert_eq!(app.commit_popup.cursor_idx, 0);

    // Enter confirm mode and test 'x' in confirm mode -> clears buffer
    app.commit_popup.input_buffer = "some message".to_string();
    app.commit_popup.editing = false;
    let key_x = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty());
    let handled = crate::input::handle_key(&mut app, key_x, 0);
    assert!(handled);
    assert!(app.commit_popup.input_buffer.is_empty());

    // Switch editing back to true for next steps
    app.commit_popup.editing = true;

    // Test Esc in editing mode -> closes popup / returns to Detail mode
    let key_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    let handled = crate::input::handle_key(&mut app, key_esc, 0);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);
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
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: missing_repo_path("workspace_conflicts_shortcuts"),
        info: Box::new(info),
    });
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
fn test_conflict_diff_mergetool_and_continue_shortcuts() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config { items: vec!["a_repo".to_string()], ..Default::default() };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_conflict_diff.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    let mut changes = crate::repo::WorktreeChanges::default();
    changes.conflicted.push(crate::repo::FileEntry { path: "dummy.txt".to_string(), label: "C" });
    let info = crate::repo::RepoInfo {
        branch: Some("main".to_string()),
        summary: crate::repo::RepoSummary { modified: 1, ..Default::default() },
        changes,
        ..crate::repo::RepoInfo::default()
    };
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: missing_repo_path("conflict_diff_mergetool_and_continue_shortcuts"),
        info: Box::new(info),
    });
    app.commit_list.selection = 0; // Selected uncommitted commit
    assert!(app.is_uncommitted_selected());

    // Mode::Detail with the ConflictDiff panel focused
    app.mode = Mode::Detail;
    app.detail_tab = 0;
    app.detail_focus = DetailSection::ConflictDiff;

    // 'M' -> queue the selected conflicted file for the external mergetool
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('M')), 10);
    assert!(handled);
    assert_eq!(app.pending_mergetool_file.take(), Some("dummy.txt".to_string()));

    // 'C' -> Mode::MergeContinueConfirm (not commit amend)
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('C')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::MergeContinueConfirm);

    // Mode::Inspect with the ConflictDiff panel focused
    app.mode = Mode::Inspect;
    app.detail_focus = DetailSection::ConflictDiff;

    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('M')), 10);
    assert!(handled);
    assert_eq!(app.pending_mergetool_file.take(), Some("dummy.txt".to_string()));

    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('C')), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::MergeContinueConfirm);
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

    app.diff.file_diff = vec![
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::Header,
            content: "@@ -1,3 +1,4 @@".to_string(),
            old_lineno: None,
            new_lineno: None,
            hunk_idx: Some(0),
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::Context,
            content: " fn main() {".to_string(),
            old_lineno: Some(1),
            new_lineno: Some(1),
            hunk_idx: Some(0),
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::Added,
            content: "+    println!(\"hello\");".to_string(),
            old_lineno: None,
            new_lineno: Some(2),
            hunk_idx: Some(0),
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::Header,
            content: "@@ -10,3 +10,4 @@".to_string(),
            old_lineno: None,
            new_lineno: None,
            hunk_idx: Some(1),
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::Removed,
            content: "-    println!(\"old\");".to_string(),
            old_lineno: Some(10),
            new_lineno: None,
            hunk_idx: Some(1),
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
            old_lineno: None,
            new_lineno: None,
            hunk_idx: None,
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::ConflictOurs,
            content: "ours".to_string(),
            old_lineno: None,
            new_lineno: None,
            hunk_idx: None,
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::ConflictSeparator,
            content: "=======".to_string(),
            old_lineno: None,
            new_lineno: None,
            hunk_idx: None,
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::ConflictTheirs,
            content: "theirs".to_string(),
            old_lineno: None,
            new_lineno: None,
            hunk_idx: None,
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::ConflictSeparator,
            content: ">>>>>>> THEIRS".to_string(),
            old_lineno: None,
            new_lineno: None,
            hunk_idx: None,
        },
    ];

    app.resolve_conflict_ours();
    app.resolve_conflict_theirs();
    app.mark_conflict_resolved();
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
        let _ =
            commit_popup.event(&Event::Key(key), &crate::keybindings::KeybindingsConfig::default());
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
        let _ =
            commit_popup.event(&Event::Key(key), &crate::keybindings::KeybindingsConfig::default());
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
        let _ = generic_popup
            .event(&Event::Key(key), &crate::keybindings::KeybindingsConfig::default());
    }
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
            old_lineno: None,
            new_lineno: None,
            hunk_idx: Some(0),
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::Added,
            content: "+added line".to_string(),
            old_lineno: None,
            new_lineno: Some(1),
            hunk_idx: Some(0),
        },
        crate::repo::DiffLine {
            kind: crate::repo::DiffLineKind::Removed,
            content: "-removed line".to_string(),
            old_lineno: Some(1),
            new_lineno: None,
            hunk_idx: Some(0),
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

    // Direct tests for cancel methods
    app.cancel_cherry_pick();
    app.cancel_revert();
    app.cancel_discard_changes();
    app.cancel_commit_search();
    app.cancel_commit();
}

/// The stashing panel lists each file once, `Space` / `a` pick files, `u`
/// drops untracked picks, Esc from the name prompt returns to the panel, and
/// saving stashes only the picked files.
#[test]
fn test_stashing_panel_stashes_only_the_picked_files() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let (dir, _repo) =
        temp_repo_with_files("stash_panel_picks", &[("a.txt", "a\n"), ("b.txt", "b\n")]);
    std::fs::write(dir.join("a.txt"), "a changed\n").unwrap();
    std::fs::write(dir.join("b.txt"), "b staged\n").unwrap();
    run_git_in(&dir, &["add", "b.txt"]);
    std::fs::write(dir.join("b.txt"), "b staged then changed\n").unwrap();
    std::fs::write(dir.join("n.txt"), "new\n").unwrap();

    let (config_path, _cfg) = temp_config_path("stash_panel_picks");
    let mut app = App::new(Config::default(), config_path);
    app.keybindings = crate::keybindings::KeybindingsConfig::default_config();
    load_workspace(&mut app, &dir);
    let press = |app: &mut App, code: KeyCode| {
        assert!(crate::input::handle_key(app, KeyEvent::new(code, KeyModifiers::empty()), 10));
    };

    app.detail_focus = DetailSection::Unstaged;
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(app.mode, Mode::StashingUI);
    let rows: Vec<(String, StashFileKind)> =
        app.stash_panel_rows().into_iter().map(|r| (r.path, r.kind)).collect();
    assert_eq!(
        rows,
        [
            ("b.txt".to_string(), StashFileKind::StagedAndUnstaged),
            ("a.txt".to_string(), StashFileKind::Unstaged),
            ("n.txt".to_string(), StashFileKind::Untracked),
        ]
    );

    // Space picks the file under the cursor; `u` drops untracked files and
    // their picks, keeping the cursor on a row.
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(app.stash_panel_paths(), ["a.txt", "n.txt"]);
    press(&mut app, KeyCode::Char('u'));
    assert!(!app.stash_untracked);
    assert_eq!(app.stash_panel_paths(), ["a.txt"]);
    assert_eq!(app.stashing_ui_selection, 1);

    // `a` picks everything, then nothing; Space unpicks and re-picks.
    press(&mut app, KeyCode::Char('a'));
    assert_eq!(app.stash_panel_paths(), ["b.txt", "a.txt"]);
    press(&mut app, KeyCode::Char('a'));
    assert!(app.stash_panel_paths().is_empty());
    press(&mut app, KeyCode::Char(' '));
    assert_eq!(app.stash_panel_paths(), ["a.txt"]);

    // Cancelling the name prompt goes back to the panel with the picks kept.
    press(&mut app, KeyCode::Char('s'));
    assert_eq!(app.mode, Mode::StashCreateInput);
    assert!(app.stash_from_panel);
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.mode, Mode::StashingUI);
    assert_eq!(app.stash_panel_paths(), ["a.txt"]);

    press(&mut app, KeyCode::Char('s'));
    for c in "only a".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.error_message, None);
    assert_eq!(app.mode, Mode::Detail);
    assert_eq!(app.status_message.as_deref(), Some("Created stash 'only a' of a.txt"));
    assert_eq!(app.stats.stashes, 1);
    assert!(!app.stash_from_panel);

    assert_eq!(git_output_in(&dir, &["stash", "show", "--name-only", "stash@{0}"]), "a.txt\n");
    assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap(), "a\n");
    assert_eq!(std::fs::read_to_string(dir.join("b.txt")).unwrap(), "b staged then changed\n");
    assert!(dir.join("n.txt").exists());
}

/// A stash prompt opened from the Stashes tab stashes everything, whatever was
/// picked in the panel earlier, and Esc returns to the tab.
#[test]
fn test_stashes_tab_prompt_ignores_panel_picks() {
    let (config_path, _cfg) = temp_config_path("stash_tab_prompt");
    let mut app = App::new(Config::default(), config_path);
    app.stash_marked.insert("a.txt".to_string());
    app.start_panel_stash_create();
    assert!(app.stash_from_panel);
    app.start_stash_create();
    assert!(!app.stash_from_panel);
    app.queue.push(crate::queue::InternalEvent::InputEsc);
    app.drain_queue();
    assert_eq!(app.mode, Mode::Detail);
}

/// `z` in an Unstaged diff stashes the selected hunk (Workspace diff pane) or,
/// in line mode, the selected line (full-screen diff), and takes it out of
/// the working tree.
#[test]
fn test_z_stashes_the_selected_hunk_or_line() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let original: String = (1..=20).map(|i| format!("Line {}\n", i)).collect();
    let (dir, _repo) = temp_repo_with_files("stash_hunk_key", &[("f.txt", &original)]);
    let edited = original
        .replace("Line 2\n", "Line 2 modified\n")
        .replace("Line 18\n", "Line 18 modified\n");
    std::fs::write(dir.join("f.txt"), &edited).unwrap();

    let (config_path, _cfg) = temp_config_path("stash_hunk_key");
    let mut app = App::new(Config::default(), config_path);
    app.keybindings = crate::keybindings::KeybindingsConfig::default_config();
    let open_diff = |app: &mut App| {
        load_workspace(app, &dir);
        app.detail_focus = DetailSection::StagingDetails;
        app.last_staging_focus = DetailSection::Unstaged;
        app.status_list.staging_file_selection = 0;
        app.diff.file_diff = repo::get_worktree_file_diff(&dir, "f.txt", false);
    };
    let z = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::empty());

    open_diff(&mut app);
    app.diff.diff_hunk_selection = 1;
    assert!(crate::input::handle_key(&mut app, z, 10));
    assert_eq!(app.error_message, None);
    assert_eq!(app.status_message.as_deref(), Some("Stashed hunk at line 15 of f.txt"));
    assert_eq!(app.stats.stashes, 1);
    assert_eq!(
        std::fs::read_to_string(dir.join("f.txt")).unwrap(),
        original.replace("Line 2\n", "Line 2 modified\n")
    );
    let shown = git_output_in(&dir, &["stash", "show", "-p", "stash@{0}"]);
    assert!(shown.contains("+Line 18 modified") && !shown.contains("Line 2 modified"));

    // Line mode in the full-screen diff: only the added line is stashed, so
    // the working tree keeps the removal of "Line 2".
    open_diff(&mut app);
    app.mode = Mode::Inspect;
    app.diff.diff_line_mode = true;
    app.diff.diff_hunk_selection = 0;
    app.diff.diff_line_selection =
        app.diff.file_diff.iter().position(|l| l.content == "Line 2 modified").unwrap();
    assert!(crate::input::handle_key(&mut app, z, 10));
    assert_eq!(app.error_message, None);
    assert_eq!(app.status_message.as_deref(), Some("Stashed line 2 of f.txt"));
    assert_eq!(app.stats.stashes, 2);
    assert_eq!(
        std::fs::read_to_string(dir.join("f.txt")).unwrap(),
        original.replace("Line 2\n", "")
    );
    assert_eq!(git_output_in(&dir, &["stash", "list"]).lines().count(), 2);
}

/// Only a modified file can be stashed by hunk or line; a new file is refused
/// before git runs.
#[test]
fn test_z_refuses_a_new_file() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let (config_path, _cfg) = temp_config_path("stash_hunk_new_file");
    let mut app = App::new(Config::default(), config_path);
    app.keybindings = crate::keybindings::KeybindingsConfig::default_config();
    let mut info = repo::RepoInfo::default();
    info.changes.unstaged = vec![repo::FileEntry { path: "n.txt".to_string(), label: "N" }];
    info.changes.untracked = vec![repo::FileEntry { path: "n.txt".to_string(), label: "?" }];
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: missing_repo_path("stash_hunk_new_file"),
        info: Box::new(info),
    });
    app.mode = Mode::Detail;
    app.detail_tab = 0;
    app.detail_focus = DetailSection::StagingDetails;
    app.last_staging_focus = DetailSection::Unstaged;
    app.diff.file_diff = vec![repo::DiffLine {
        kind: repo::DiffLineKind::Header,
        content: "@@ -0,0 +1 @@".to_string(),
        old_lineno: None,
        new_lineno: None,
        hunk_idx: Some(0),
    }];

    let z = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::empty());
    assert!(crate::input::handle_key(&mut app, z, 10));
    let err = app.error_message.clone().unwrap_or_default();
    assert!(err.contains("Only a modified file") && err.contains("'n.txt' is a new file"), "{err}");
    assert_eq!(app.stats.stashes, 0);
}

/// The diff panes advertise `z` for an unstaged file, as a hunk or a line
/// stash depending on the mode, and the stashing panel lists its pick keys.
#[test]
fn test_partial_stash_status_bar_hints() {
    use crate::components::cmd_bar::{detail_dismiss_entries, inspect_dismiss_entries};
    let (config_path, _cfg) = temp_config_path("stash_hints");
    let mut app = App::new(Config::default(), config_path);
    app.keybindings = crate::keybindings::KeybindingsConfig::default_config();
    let mut info = repo::RepoInfo::default();
    info.changes.unstaged = vec![repo::FileEntry { path: "f.txt".to_string(), label: "M" }];
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: missing_repo_path("stash_hints"),
        info: Box::new(info),
    });
    app.mode = Mode::Detail;
    app.detail_tab = 0;
    app.detail_focus = DetailSection::StagingDetails;
    let labels = |entries: Vec<crate::components::cmd_bar::StatusEntry>| -> Vec<String> {
        entries
            .iter()
            .map(|e| e.spans.iter().map(|s| s.content.as_ref()).collect::<String>())
            .collect()
    };

    app.last_staging_focus = DetailSection::Staged;
    assert!(!labels(detail_dismiss_entries(&app).1).iter().any(|l| l.contains("Stash Hunk")));

    app.last_staging_focus = DetailSection::Unstaged;
    assert!(labels(detail_dismiss_entries(&app).1).iter().any(|l| l.contains("Stash Hunk [z]")));
    assert!(labels(inspect_dismiss_entries(&app).1).iter().any(|l| l.contains("Stash Hunk [z]")));
    app.diff.diff_line_mode = true;
    assert!(labels(detail_dismiss_entries(&app).1).iter().any(|l| l.contains("Stash Line [z]")));
    app.inspect_full_diff = true;
    assert!(labels(inspect_dismiss_entries(&app).1).iter().any(|l| l.contains("Stash Line [z]")));

    // A rebinding is what the hint shows.
    app.keybindings
        .update_action_keys(crate::keybindings::Action::DiffStash, vec!["ctrl-z".into()]);
    assert!(!labels(detail_dismiss_entries(&app).1).iter().any(|l| l.contains("Stash Line [z]")));
}

/// Rows put a conflicted file first and never repeat a path.
#[test]
fn test_stash_file_rows_order_and_untracked_toggle() {
    let entry = |path: &str, label: &'static str| repo::FileEntry { path: path.to_string(), label };
    let changes = repo::WorktreeChanges {
        conflicted: vec![entry("c.txt", "C")],
        staged: vec![entry("s.txt", "N")],
        unstaged: vec![entry("s.txt", "M"), entry("u.txt", "D"), entry("new.txt", "N")],
        untracked: vec![entry("new.txt", "?")],
    };
    let rows = |include_untracked| -> Vec<(String, &'static str, StashFileKind)> {
        stash_file_rows(&changes, include_untracked)
            .into_iter()
            .map(|r| (r.path, r.label, r.kind))
            .collect()
    };
    assert_eq!(
        rows(true),
        [
            ("c.txt".to_string(), "C", StashFileKind::Conflicted),
            ("s.txt".to_string(), "N", StashFileKind::StagedAndUnstaged),
            ("u.txt".to_string(), "D", StashFileKind::Unstaged),
            ("new.txt".to_string(), "?", StashFileKind::Untracked),
        ]
    );
    assert_eq!(rows(false).len(), 3);
    assert_eq!(StashFileKind::StagedAndUnstaged.caption(), "staged + unstaged");
}

/// The stashing panel says when every file goes, and once files are picked it
/// counts them and checks them off; the name prompt's title names the count.
#[test]
fn test_stashing_panel_draws_picks() {
    let (config_path, _cfg) = temp_config_path("stash_panel_draw");
    let mut app = App::new(Config::default(), config_path);
    let mut info = repo::RepoInfo::default();
    info.changes.unstaged = vec![
        repo::FileEntry { path: "a.txt".to_string(), label: "M" },
        repo::FileEntry { path: "b.txt".to_string(), label: "M" },
    ];
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: missing_repo_path("stash_panel_draw"),
        info: Box::new(info.clone()),
    });
    let render = |app: &App, prompt: bool| -> String {
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(140, 30)).unwrap();
        terminal
            .draw(|f| {
                if prompt {
                    crate::popups::stash_msg::draw_stash_create_popup(f, "", f.area(), app);
                } else {
                    crate::popups::stashing_ui::draw_stashing_ui(f, &info, app, f.area());
                }
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        buffer.content.iter().map(|c| c.symbol()).collect()
    };

    app.open_stashing_ui();
    let all = render(&app, false);
    assert!(all.contains("Files to Stash (all 2)"), "{all}");
    assert!(all.contains("Stashes every listed file") && !all.contains("[x]"));

    app.stash_panel_toggle_file();
    let picked = render(&app, false);
    assert!(picked.contains("Files to Stash (1 of 2 picked)"), "{picked}");
    assert!(picked.contains("[x] [M] a.txt") && picked.contains("[ ] [M] b.txt"));

    app.start_panel_stash_create();
    assert!(render(&app, true).contains("Stash 1 Picked File"));
}
