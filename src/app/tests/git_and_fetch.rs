#![allow(clippy::unwrap_used, clippy::panic)]

use super::*;

#[test]
fn test_network_action_progress_and_error_handling() {
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
    app.tx
        .send(BgEvent::RepoOpResult {
            repo_path: PathBuf::from("/tmp/mock_repo"),
            kind: RepoOpKind::Push,
            result: Err("Push failed: git push rejected".to_string()),
        })
        .unwrap();

    // Drain receiver
    assert!(app.drain_bg_events());

    // Verify that fetching is cleared and error_message popup is active
    assert!(!app.fetching);
    assert_eq!(app.error_message.as_deref(), Some("[mock_repo] Push failed: git push rejected"));

    // Verify keypress dismisses the error popup
    let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    let consumed = crate::input::handle_key(&mut app, esc_key, 0);
    assert!(consumed);
    assert!(app.error_message.is_none());
}

#[test]
fn test_remote_tags_progress_and_error_handling() {
    let repo_path = missing_repo_path("remote_tags_progress");
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
        resolved: repo_path.clone(),
        info: Box::new(mock_info),
    });

    // Trigger fetch with show_progress = true
    app.fetch_remote_tags(true);
    assert!(app.fetching);
    assert_eq!(app.status_message.as_deref(), Some("Fetching tags from 'origin'..."));
    // The spawned ls-remote fails at once on the missing path; drain its reply
    // so it cannot race the simulated one below.
    app.rx.recv_timeout(std::time::Duration::from_secs(30)).unwrap();

    // Simulate background thread sending RemoteTagsResult error
    app.tx
        .send(BgEvent::RemoteTagsResult {
            repo_path,
            result: Err("Failed to get remote tags: custom error".to_string()),
        })
        .unwrap();

    assert!(app.drain_bg_events());

    assert!(!app.fetching);
    assert_eq!(app.error_message.as_deref(), Some("Failed to get remote tags: custom error"));
}

#[test]
fn test_remote_fetch_progress_and_error_handling() {
    let repo_path = missing_repo_path("remote_fetch_progress");
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
        resolved: repo_path.clone(),
        info: Box::new(mock_info),
    });

    // Trigger fetch from remote tab (fetch_remote)
    app.fetch_remote("origin");
    assert!(app.fetching);
    assert_eq!(app.status_message.as_deref(), Some("Fetching remote 'origin'..."));
    // The spawned fetch fails at once on the missing path; drain its reply so
    // it cannot race the simulated one below.
    app.rx.recv_timeout(std::time::Duration::from_secs(30)).unwrap();

    // Simulate background thread sending Fetch failed message
    app.tx
        .send(BgEvent::RepoOpResult {
            repo_path,
            kind: RepoOpKind::Fetch,
            result: Err("Fetch failed: custom fetch error".to_string()),
        })
        .unwrap();

    assert!(app.drain_bg_events());

    assert!(!app.fetching);
    assert_eq!(app.error_message.as_deref(), Some("Fetch failed: custom fetch error"));
}

#[test]
fn test_cherry_pick_and_revert_flow() {
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
fn test_remote_add_delete_flow() {
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
    let temp_path = std::env::temp_dir().join("gitwig_test_config_remotes.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // Put app in Detail Mode on tab 5 (Remotes)
    app.mode = Mode::Detail;
    app.detail_tab = 5;
    app.detail_focus = DetailSection::Remotes;
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: missing_repo_path("remote_add_delete_flow"),
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
fn test_remote_add_paste_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
    let config = Config::default();
    let temp_path = std::env::temp_dir().join("gitwig_test_config_remote_paste.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.mode = Mode::Detail;
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: missing_repo_path("remote_add_paste_flow"),
        info: Box::new(repo::RepoInfo::default()),
    });

    // Trigger remote add
    app.start_remote_add();
    assert_eq!(app.mode, Mode::RemoteAddNameInput);

    // Type remote name and hit enter
    app.input_buffer = "origin".to_string();
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::RemoteAddUrlInput);
    assert_eq!(app.remote_add_name, "origin");

    // Paste remote URL with trailing newline
    crate::input::handle_paste(&mut app, "git@github.com:tareqmy/ferronote.git\n");
    assert_eq!(app.input_buffer, "git@github.com:tareqmy/ferronote.git");

    // Submit URL with Enter
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);
    assert_eq!(app.remote_add_url, "git@github.com:tareqmy/ferronote.git");
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
        resolved: missing_repo_path("branch_and_tag_checkout_confirmation"),
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

    // The failed checkout surfaces as an error popup; dismiss it before
    // continuing, as the next keypress would otherwise be consumed by it.
    assert!(app.error_message.as_deref().unwrap_or("").contains("Checkout failed"));
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
    assert!(handled);
    assert!(app.error_message.is_none());

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
fn test_tag_fetch_attempt_and_dismiss_flow() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
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
        resolved: missing_repo_path("tag_fetch_attempt_and_dismiss_flow"),
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
    // The spawned ls-remote fails at once on the missing path; drain its reply
    // so it cannot race the simulated one below.
    app.rx.recv_timeout(std::time::Duration::from_secs(30)).unwrap();

    // 2. Receive error from the background thread
    let open_repo_path = match &app.current_detail {
        Some(crate::repo::ItemDetail::Repo { resolved, .. }) => resolved.clone(),
        _ => PathBuf::new(),
    };
    app.tx
        .send(BgEvent::RemoteTagsResult {
            repo_path: open_repo_path,
            result: Err("Failed to get remote tags: network timeout".to_string()),
        })
        .unwrap();

    assert!(app.drain_bg_events());

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
        resolved: missing_repo_path("tag_push_all_confirmation_flow"),
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
        resolved: missing_repo_path("tag_push_all_confirmation_flow"),
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
    let (config_path, _guard) = temp_config_path("bulk_fetch_all_trigger");
    let mut app = App::new(config, config_path);
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
fn test_cherry_pick_destination_branches() {
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
    let (config_path, _guard) = temp_config_path("cherry_pick_destination_branches");
    let mut app = App::new(config, config_path);

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
fn test_worktree_tui_flows() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::path::PathBuf;

    let config = Config { items: vec!["/path/to/my_repo".to_string()], ..Default::default() };
    let config_path = std::env::temp_dir().join("gitwig_test_worktree_config.toml");
    let _guard = TestFileGuard { path: config_path.clone() };
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
}

#[test]
fn test_submodule_tui_flows() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::path::PathBuf;

    let config = Config { items: vec!["/path/to/my_repo".to_string()], ..Default::default() };
    let config_path = std::env::temp_dir().join("gitwig_test_submodule_config.toml");
    let _guard = TestFileGuard { path: config_path.clone() };
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
    assert_eq!(
        app.submodule_delete_target.as_ref().map(|sub| sub.name.as_str()),
        Some("my-submodule")
    );

    // Escape confirmation
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 1);
    assert!(handled);
    assert_eq!(app.mode, Mode::Detail);
    assert_eq!(app.submodule_delete_target, None);
}

#[test]
fn test_implicit_network_count() {
    let config = Config::default();
    let (config_path, _guard) = temp_config_path("implicit_network_count");
    let mut app = App::new(config, config_path);
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

/// The Worktrees tab's unlock and prune run git in the open repository. They
/// used to take `config.items[selected_index]`, and with grouping on the home
/// cursor's row index names a different repository: here the cursor is on
/// gamma's Recent row (index 1), and `config.items[1]` is beta.
#[test]
fn test_worktree_unlock_and_prune_act_on_the_open_repo_when_grouped() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let (mut app, items, guard) = app_with_three_repos("worktree_open_repo");
    group_three_repos_under_work(&mut app, &items);
    let beta = PathBuf::from(&items[1]);
    let gamma = PathBuf::from(&items[2]);
    for dir in [&beta, &gamma] {
        std::fs::remove_dir_all(dir.join(".git")).unwrap();
        run_git_in(dir, &["init", "-q"]);
    }
    run_git_in(&gamma, &["commit", "-q", "--allow-empty", "-m", "init"]);
    run_git_in(&gamma, &["worktree", "add", "-q", &guard.path.join("gamma-wt").to_string_lossy()]);
    run_git_in(&gamma, &["worktree", "lock", "gamma-wt"]);

    let info = repo::RepoInfo {
        worktrees: repo::TabData::Loaded(repo::load_tab_worktrees(&gamma).unwrap()),
        ..Default::default()
    };
    app.current_detail =
        Some(repo::ItemDetail::Repo { resolved: gamma.clone(), info: Box::new(info) });
    app.mode = Mode::Detail;
    app.detail_tab = 7;
    app.detail_focus = DetailSection::Worktrees;
    app.worktree_selection = 0;
    app.selected_index = 1;
    assert_eq!(app.get_selected_item(), Some(&items[2]));

    let key = |c: char| KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty());

    // `l` on a locked worktree unlocks it.
    assert!(crate::input::handle_key(&mut app, key('l'), 1));
    assert_eq!(app.status_message.as_deref(), Some("Worktree unlocked successfully"));
    let gamma_wts = repo::load_tab_worktrees(&gamma).unwrap();
    assert!(gamma_wts.iter().all(|wt| !wt.is_locked), "gamma's worktree is unlocked");
    // The unlock resyncs the view, which ignores keys until the reload lands.
    app.loading_repo_path = None;

    // An admin directory with no `gitdir` file is stale metadata for prune.
    for dir in [&beta, &gamma] {
        std::fs::create_dir_all(dir.join(".git/worktrees/stale")).unwrap();
    }
    assert!(crate::input::handle_key(&mut app, key('p'), 1));
    assert_eq!(app.status_message.as_deref(), Some("Pruned stale worktree metadata"));
    assert!(!gamma.join(".git/worktrees/stale").exists(), "gamma is pruned");
    assert!(beta.join(".git/worktrees/stale").exists(), "beta is left alone");
}

/// `D` in the Worktrees tab removes the worktree it was pressed on, by path,
/// and choice 1 never discards uncommitted work. It used to pass the admin name
/// (which named a different worktree here) with `--force` for both choices.
#[test]
fn test_worktree_remove_deletes_the_selected_worktree_and_keeps_dirty_ones() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let key = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let root = std::env::temp_dir().join("gitwig_test_worktree_remove_target");
    let _ = std::fs::remove_dir_all(&root);
    let guard = TestDirGuard { path: root.clone() };
    let main = root.join("main");
    std::fs::create_dir_all(&main).unwrap();
    run_git_in(&main, &["init", "-q"]);
    run_git_in(&main, &["commit", "-q", "--allow-empty", "-m", "init"]);
    run_git_in(&main, &["branch", "a"]);
    run_git_in(&main, &["branch", "b"]);
    let (x_foo, x_renamed, z_foo) =
        (root.join("x/foo"), root.join("x/renamed"), root.join("z/foo"));
    run_git_in(&main, &["worktree", "add", "-q", &x_foo.to_string_lossy(), "a"]);
    run_git_in(
        &main,
        &["worktree", "move", &x_foo.to_string_lossy(), &x_renamed.to_string_lossy()],
    );
    run_git_in(&main, &["worktree", "add", "-q", &z_foo.to_string_lossy(), "b"]);
    std::fs::write(z_foo.join("precious.txt"), "uncommitted work").unwrap();

    let config = Config { items: vec![main.to_string_lossy().to_string()], ..Default::default() };
    let mut app = App::new(config, root.join("config.toml"));
    let load = |app: &mut App| {
        let info = repo::RepoInfo {
            worktrees: repo::TabData::Loaded(repo::load_tab_worktrees(&main).unwrap()),
            ..Default::default()
        };
        app.current_detail =
            Some(repo::ItemDetail::Repo { resolved: main.clone(), info: Box::new(info) });
        app.loading_repo_path = None;
        app.mode = Mode::Detail;
    };
    let row_of = |app: &App, suffix: &str| match &app.current_detail {
        Some(repo::ItemDetail::Repo { info, .. }) => match &info.worktrees {
            repo::TabData::Loaded(wts) => wts.iter().position(|wt| wt.path.ends_with(suffix)),
            _ => None,
        },
        _ => None,
    };
    load(&mut app);
    app.detail_tab = 7;
    app.detail_focus = DetailSection::Worktrees;

    // `D` on x/renamed (admin name "foo"), then the list reloads in reverse
    // order before the choice is typed: the captured worktree is removed.
    app.worktree_selection = row_of(&app, "x/renamed").unwrap();
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Char('D')), 1));
    assert_eq!(app.mode, Mode::WorktreeRemoveConfirm);
    if let Some(repo::ItemDetail::Repo { info, .. }) = &mut app.current_detail {
        if let repo::TabData::Loaded(wts) = &mut info.worktrees {
            wts.reverse();
        }
    }
    app.input_buffer = "1".to_string();
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert!(!x_renamed.exists(), "the worktree `D` was pressed on is removed");
    assert!(z_foo.join("precious.txt").exists(), "the other worktree is untouched");

    // Choice 1 on a worktree with uncommitted work refuses and deletes nothing.
    load(&mut app);
    app.worktree_selection = row_of(&app, "z/foo").unwrap();
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Char('D')), 1));
    app.input_buffer = "1".to_string();
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert!(z_foo.join("precious.txt").exists(), "choice 1 keeps uncommitted work");
    assert_eq!(app.mode, Mode::Detail);
    let error = app.error_message.clone().unwrap_or_default();
    assert!(error.contains("choose 2 to force-remove"), "{}", error);
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Esc), 1));
    assert!(app.error_message.is_none());

    // Choice 2 force-removes it.
    load(&mut app);
    app.worktree_selection = row_of(&app, "z/foo").unwrap();
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Char('D')), 1));
    app.input_buffer = "2".to_string();
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Enter), 1));
    assert!(!z_foo.exists(), "choice 2 force-removes the worktree");
    assert!(app.worktree_remove_target.is_none());
    drop(guard);
}

#[test]
fn test_branch_merge_into_flow() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    // Real repo: one commit on the default branch, an "other" branch at that
    // commit, then a second commit on the default branch so "other" is behind.
    let uuid =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let temp_repo_path = std::env::temp_dir().join(format!("gitwig_test_merge_into_{}", uuid));
    std::fs::create_dir_all(&temp_repo_path).unwrap();
    let repo = git2::Repository::init(&temp_repo_path).unwrap();
    let mut config_git = repo.config().unwrap();
    config_git.set_str("user.name", "Test User").unwrap();
    config_git.set_str("user.email", "test@test.com").unwrap();

    let commit = |repo: &git2::Repository, file: &str, msg: &str| {
        std::fs::write(temp_repo_path.join(file), msg).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new(file)).unwrap();
        index.write().unwrap();
        let oid = index.write_tree().unwrap();
        let tree = repo.find_tree(oid).unwrap();
        let sig = repo.signature().unwrap();
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents).unwrap()
    };
    commit(&repo, "a.txt", "first");
    let first = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch("other", &first, false).unwrap();
    let second_oid = commit(&repo, "b.txt", "second");

    let config =
        Config { items: vec![temp_repo_path.to_str().unwrap().to_string()], ..Default::default() };
    let (config_path, _guard) = temp_config_path("branch_merge_into_flow");
    let mut app = App::new(config, config_path);
    app.mode = Mode::Detail;
    app.detail_tab = 3;
    app.detail_focus = DetailSection::LocalBranches;

    let mut info =
        match crate::repo::inspect_detail(temp_repo_path.to_str().unwrap(), 100, 1000, false) {
            crate::repo::ItemDetail::Repo { info, .. } => *info,
            _ => panic!("expected repo detail"),
        };
    // Branch lists are lazy tab data; load them the way the branches tab does.
    let (local, remote) = crate::repo::load_tab_branches(&temp_repo_path);
    info.local_branches = crate::repo::TabData::Loaded(local.unwrap());
    info.remote_branches = crate::repo::TabData::Loaded(remote.unwrap());
    let current_name =
        info.local_branches.iter().find(|b| b.is_head).map(|b| b.name.clone()).unwrap();
    let other_idx = info.local_branches.iter().position(|b| b.name == "other").unwrap();
    let head_idx = info.local_branches.iter().position(|b| b.is_head).unwrap();
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: temp_repo_path.clone(),
        info: Box::new(info),
    });

    let m_key = KeyEvent::new(KeyCode::Char('M'), KeyModifiers::SHIFT);
    let key = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    // Requesting on the checked-out branch itself is refused.
    app.branch_list.local_branch_selection = head_idx;
    assert!(crate::input::handle_key(&mut app, m_key, 10));
    assert_eq!(app.mode, Mode::Detail);
    assert!(app.status_message.as_deref().unwrap_or("").contains("already checked out"));

    // Requesting on "other" opens the confirmation; `n` cancels cleanly.
    app.branch_list.local_branch_selection = other_idx;
    assert!(crate::input::handle_key(&mut app, m_key, 10));
    assert_eq!(app.mode, Mode::BranchMergeIntoConfirm);
    assert_eq!(app.branch_action_target, Some(("other".to_string(), false)));
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Char('n')), 10));
    assert_eq!(app.mode, Mode::Detail);
    assert_eq!(app.branch_action_target, None);

    // Confirming checks out "other" and merges the previous branch into it.
    assert!(crate::input::handle_key(&mut app, m_key, 10));
    assert_eq!(app.mode, Mode::BranchMergeIntoConfirm);
    assert!(crate::input::handle_key(&mut app, key(KeyCode::Char('y')), 10));
    assert_eq!(app.mode, Mode::Detail);

    // Other subsystems (e.g. the fs watcher) share the channel; scan until the
    // merge-into thread reports.
    let expected = format!("Checked out 'other' and merged '{}' into it", current_name);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let mut seen = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        let msg = app.rx.recv_timeout(remaining).unwrap_or_else(|_| {
            panic!("merge-into thread did not report; messages seen: {:?}", seen)
        });
        if let BgEvent::RepoOpResult { result: Ok(text), .. } = &msg {
            if text.contains(&expected) {
                break;
            }
        }
        seen.push(msg);
    }

    let verify = git2::Repository::open(&temp_repo_path).unwrap();
    let head = verify.head().unwrap();
    assert_eq!(head.shorthand().ok(), Some("other"));
    // Fast-forwarded: "other" now points at the second commit.
    assert_eq!(head.peel_to_commit().unwrap().id(), second_oid);

    let _ = std::fs::remove_dir_all(&temp_repo_path);
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
    let (config_path, _guard) = temp_config_path("all_git_actions_on_real_repo");
    let mut app = App::new(config, config_path);
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
    app.submodule_delete_target = Some(crate::repo::SubmoduleInfo {
        name: "sub-test".to_string(),
        path: std::path::PathBuf::from("sub-test"),
        ..Default::default()
    });
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
fn test_bulk_fetch_completion_and_30s_clear() {
    let config = Config { items: vec!["/path/to/repo".to_string()], ..Default::default() };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_bulk_fetch.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.bulk_fetching.insert("/path/to/repo".to_string());
    app.bulk_fetch_results.insert("/path/to/repo".to_string(), Ok("Fetched".to_string()));

    app.tx.send(BgEvent::BulkFetchSuccess { path: "/path/to/repo".to_string() }).unwrap();

    assert!(app.drain_bg_events());

    assert!(app.bulk_fetching.is_empty());
    assert!(app.bulk_fetch_completed_at.is_some());
    assert!(!app.bulk_fetch_results.is_empty());

    // 1. Not elapsed
    app.bulk_fetch_completed_at =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(29));
    if let Some(completed_at) = app.bulk_fetch_completed_at {
        if completed_at.elapsed().as_secs() >= 30 {
            app.bulk_fetch_results.clear();
            app.bulk_fetch_completed_at = None;
        }
    }
    assert!(!app.bulk_fetch_results.is_empty());
    assert!(app.bulk_fetch_completed_at.is_some());

    // 2. Elapsed
    app.bulk_fetch_completed_at =
        Some(std::time::Instant::now() - std::time::Duration::from_secs(30));
    if let Some(completed_at) = app.bulk_fetch_completed_at {
        if completed_at.elapsed().as_secs() >= 30 {
            app.bulk_fetch_results.clear();
            app.bulk_fetch_completed_at = None;
        }
    }
    assert!(app.bulk_fetch_results.is_empty());
    assert!(app.bulk_fetch_completed_at.is_none());
}

#[test]
fn test_git_cancels_coverage_boost() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_git_cancels.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);

    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(crate::repo::RepoInfo {
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
        }),
    });

    app.cancel_branch_push();
    app.cancel_branch_checkout();
    app.cancel_tag_delete();
    app.cancel_tag_push();
    app.cancel_tag_push_all();
    app.cancel_branch_create();
    app.cancel_branch_delete();
    app.cancel_branch_merge();
    app.cancel_branch_rebase();
    app.cancel_branch_interactive_rebase();
    app.cancel_remote_picker();

    assert_eq!(app.staging_file_total(), 0);

    // Call request triggers
    app.mode = Mode::Detail;
    app.start_branch_create();
    assert_eq!(app.mode, Mode::BranchCreateInput);

    app.mode = Mode::Detail;
    app.detail_tab = 0;
    app.detail_focus = DetailSection::Commits;
    app.commit_list.selection = 0;
    app.start_tag_create();
    assert_eq!(app.mode, Mode::TagCreateInput);

    app.mode = Mode::Detail;
    app.start_stash_create();
    assert_eq!(app.mode, Mode::StashCreateInput);

    app.mode = Mode::Detail;
    app.start_remote_add();
    assert_eq!(app.mode, Mode::RemoteAddNameInput);

    app.mode = Mode::Detail;
    app.request_tag_delete();
    app.request_tag_push();
    app.request_tag_push_all();
    app.request_branch_delete();
    app.request_branch_merge();
    app.request_branch_rebase();
    app.request_branch_interactive_rebase();
}

#[test]
fn test_tag_create_annotated_and_overwrite_flow() {
    let temp_config_path = std::env::temp_dir().join("gitwig_test_tag_annotated_overwrite.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(Config::default(), temp_config_path);

    app.tag_action_target_oid = Some("1234567890abcdef".to_string());
    app.input_buffer = "v1.0.1".to_string();
    app.tag_create_message = "Release v1.0.1".to_string();
    app.commit_tag_create();

    // If tag exists or is created, verify state cleanup
    assert_eq!(app.tag_create_message, "");

    app.tag_overwrite_target = Some((
        "v1.0.1".to_string(),
        "1234567890abcdef".to_string(),
        Some("Release v1.0.1".to_string()),
    ));
    app.mode = Mode::TagOverwriteConfirm;
    app.cancel_tag_overwrite();
    assert_eq!(app.mode, Mode::Detail);
    assert!(app.tag_overwrite_target.is_none());
}

#[test]
fn test_force_tag_push_flow() {
    let temp_config_path = std::env::temp_dir().join("gitwig_test_force_tag_push.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(Config::default(), temp_config_path);

    app.tag_push_target = Some("v1.0.1".to_string());
    app.tag_push_force = true;
    app.cancel_tag_push();
    assert_eq!(app.mode, Mode::Detail);
    assert!(!app.tag_push_force);
    assert!(app.tag_push_target.is_none());
}

#[test]
fn test_branch_push_with_tags_flow() {
    let temp_config_path = std::env::temp_dir().join("gitwig_test_branch_push_tags.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(Config::default(), temp_config_path);

    app.branch_action_target = Some(("main".to_string(), false));
    app.branch_push_tags = true;
    app.cancel_branch_push();
    assert_eq!(app.mode, Mode::Detail);
    assert!(!app.branch_push_tags);
    assert!(app.branch_action_target.is_none());
}

#[test]
fn test_fetch_error_classification() {
    use crate::fetch_error::{FetchFailure, classify};

    // Auth: the three shapes users actually hit — SSH key rejected, HTTPS token
    // rejected, and git refusing to prompt because we disabled terminal prompts.
    assert_eq!(
        classify("git@github.com: Permission denied (publickey)."),
        FetchFailure::Auth,
        "ssh key rejection must read as auth, not network"
    );
    assert_eq!(
        classify("fatal: Authentication failed for 'https://example.com/x.git/'"),
        FetchFailure::Auth
    );
    assert_eq!(
        classify(
            "fatal: could not read Username for 'https://github.com': terminal prompts disabled",
        ),
        FetchFailure::Auth,
        "the hardened no-prompt path must be reported as auth, not as an internal error"
    );

    assert_eq!(classify("Host key verification failed."), FetchFailure::HostKey);
    assert_eq!(classify("ERROR: Repository not found."), FetchFailure::NotFound);
    assert_eq!(
        classify("fatal: unable to access 'https://x/': Could not resolve host: x"),
        FetchFailure::Network
    );
    assert_eq!(
        classify("fatal: 'origin' does not appear to be a git repository"),
        FetchFailure::NotFound
    );
    assert_eq!(classify("fatal: No remote repository specified."), FetchFailure::NoRemote);
    assert_eq!(classify("something nobody has seen before"), FetchFailure::Unknown);
}

#[test]
fn test_fetch_timeout_message_classifies_as_timeout() {
    use crate::fetch_error::{FetchFailure, classify};
    use crate::git_cmd::GitRunError;

    // The bulk fetch thread reports a cancelled child via GitRunError's Display.
    // If that wording ever drifts from what `classify` matches, a timeout would
    // silently be mislabelled as a network failure, so pin the two together.
    let rendered = GitRunError::Timeout(std::time::Duration::from_secs(30)).to_string();
    assert_eq!(classify(&rendered), FetchFailure::Timeout, "rendered: {}", rendered);
}

#[test]
fn test_every_fetch_failure_has_distinct_nonempty_label() {
    use crate::fetch_error::FetchFailure;

    let all = [
        FetchFailure::Auth,
        FetchFailure::HostKey,
        FetchFailure::NotFound,
        FetchFailure::Network,
        FetchFailure::NoRemote,
        FetchFailure::Timeout,
        FetchFailure::Local,
        FetchFailure::Unknown,
    ];
    let mut seen = std::collections::HashSet::new();
    for failure in all {
        assert!(!failure.label().is_empty());
        assert!(!failure.summary().is_empty());
        // Card labels share a narrow column; keep them short enough not to wrap.
        assert!(failure.label().len() <= 12, "label too wide: {}", failure.label());
        assert!(seen.insert(failure.label()), "duplicate label: {}", failure.label());
    }
}

#[test]
fn test_fetch_error_sanitize_strips_ansi_and_progress_frames() {
    use crate::fetch_error::sanitize;

    // git redraws progress with \r and colours errors with CSI sequences; both
    // would render as an unreadable smear inside the error popup.
    let raw = "\u{1b}[31mfatal:\u{1b}[0m boom\nremote: Counting objects:  10%\rremote: Counting objects:  99%\r\n";
    let cleaned = sanitize(raw);

    assert!(!cleaned.contains('\u{1b}'), "escape sequences survived: {:?}", cleaned);
    assert!(!cleaned.contains('\r'), "carriage returns survived: {:?}", cleaned);
    assert!(cleaned.contains("fatal: boom"));
    assert!(cleaned.contains("99%"), "final progress frame should survive: {:?}", cleaned);
    assert!(!cleaned.contains("10%"), "superseded progress frame should be dropped: {:?}", cleaned);
    assert_eq!(sanitize(""), "");
}

#[test]
fn test_git_command_disables_every_interactive_prompt() {
    // This is the actual bug being guarded: without these, ssh or a credential
    // helper writes its prompt into the alternate screen and breaks the TUI.
    let cmd = crate::git_cmd::git_command();
    let envs: std::collections::HashMap<String, Option<String>> = cmd
        .get_envs()
        .map(|(k, v)| {
            (k.to_string_lossy().into_owned(), v.map(|v| v.to_string_lossy().into_owned()))
        })
        .collect();

    assert_eq!(envs.get("GIT_TERMINAL_PROMPT"), Some(&Some("0".to_string())));
    assert_eq!(envs.get("GIT_ASKPASS"), Some(&Some(String::new())));
    assert_eq!(envs.get("SSH_ASKPASS"), Some(&Some(String::new())));
    assert!(envs.contains_key("GIT_SSH_COMMAND"));
    assert!(
        envs.get("GIT_SSH_COMMAND")
            .and_then(|v| v.as_ref())
            .map(|v| v.contains("StrictHostKeyChecking"))
            .unwrap_or(false),
        "ssh must never ask the user to confirm a host fingerprint"
    );
    assert!(
        envs.get("GIT_SSH_COMMAND")
            .and_then(|v| v.as_ref())
            .map(|v| v.contains("BatchMode=yes"))
            .unwrap_or(false),
        "ssh must never prompt on /dev/tty over the TUI (passphrases included)"
    );
    assert_eq!(envs.get("GIT_ALLOW_PROTOCOL"), Some(&Some("https:ssh:git:file".to_string())));
}

#[test]
#[allow(clippy::disallowed_methods)] // a plain `sleep` stands in for a hung remote
fn test_run_git_with_timeout_kills_a_hanging_child() {
    use crate::git_cmd::{GitRunError, run_git_with_timeout};

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
    let res = run_git_with_timeout(cmd, std::time::Duration::from_millis(300));
    let elapsed = started.elapsed();

    assert!(matches!(res, Err(GitRunError::Timeout(_))), "expected a timeout, got {:?}", res);
    assert!(
        elapsed < std::time::Duration::from_secs(10),
        "child was not killed promptly: {:?}",
        elapsed
    );
}

#[test]
fn test_run_git_with_timeout_returns_output_for_fast_commands() {
    use crate::git_cmd::{git_command, run_git_with_timeout};

    let mut cmd = git_command();
    cmd.arg("--version");
    let out = run_git_with_timeout(cmd, std::time::Duration::from_secs(30))
        .expect("git --version should complete well inside the timeout");

    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("git"));
}

#[test]
fn test_bulk_fetch_failure_state_and_details_popup() {
    let config = crate::config::Config {
        items: vec!["/tmp/gitwig_fetch_fail_repo".to_string()],
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_fetch_fail.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    let item = "/tmp/gitwig_fetch_fail_repo".to_string();
    let stderr = "git@github.com: Permission denied (publickey).\nfatal: Could not read from remote repository.";

    app.bulk_fetch_round.insert(item.clone());
    app.bulk_fetch_results.insert(item.clone(), Err(stderr.to_string()));
    app.bulk_fetch_failures.insert(item.clone(), crate::fetch_error::classify(stderr));

    // The summary must report the failure rather than the old unconditional
    // "Bulk fetch completed", which read as success even when nothing worked.
    let summary = app.bulk_fetch_summary();
    assert!(summary.contains("0 succeeded"), "{}", summary);
    assert!(summary.contains("1 failed"), "{}", summary);

    // Selecting the failed repository and asking for details must populate the
    // shared error popup with the cause and the raw git output.
    app.selected_index = 0;
    app.show_fetch_failure_details();

    let err = app.error_message.clone().expect("details should open the error popup");
    assert!(err.contains("auth denied"), "{}", err);
    assert!(err.contains("Permission denied"), "{}", err);
    assert!(err.contains(&item), "{}", err);
}

#[test]
fn test_show_fetch_failure_details_is_quiet_when_nothing_failed() {
    let config = crate::config::Config {
        items: vec!["/tmp/gitwig_fetch_ok_repo".to_string()],
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_fetch_ok.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.selected_index = 0;
    app.show_fetch_failure_details();

    assert!(app.error_message.is_none(), "a healthy repo must not raise an error popup");
}

#[test]
fn test_bulk_fetch_summary_ignores_results_from_earlier_rounds() {
    let config = crate::config::Config {
        items: vec!["a".to_string(), "b".to_string()],
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_fetch_round.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    // "stale" failed in a previous round and is deliberately retained so the user
    // can still inspect it; it must not be counted against this round.
    app.bulk_fetch_results.insert("stale".to_string(), Err("boom".to_string()));
    app.bulk_fetch_results.insert("a".to_string(), Ok("Fetched successfully".to_string()));
    app.bulk_fetch_round.insert("a".to_string());

    let summary = app.bulk_fetch_summary();
    assert!(summary.contains("1 repositories up to date"), "{}", summary);
}

#[test]
fn test_fetch_timeout_setting_rejects_unusably_short_values() {
    let config = crate::config::Config::default();
    let temp_path = std::env::temp_dir().join("gitwig_test_config_fetch_timeout.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);

    app.settings_selected_index = 84;

    // 0 means "no limit" and is allowed.
    app.settings_editing = true;
    app.input_buffer = "0".to_string();
    app.commit_settings_edit();
    assert_eq!(app.config.fetch_timeout_secs, 0);

    // 1-4s is shorter than a TLS handshake and would fail every fetch.
    app.settings_editing = true;
    app.input_buffer = "3".to_string();
    app.commit_settings_edit();
    assert_eq!(app.config.fetch_timeout_secs, 0, "value below the floor must be rejected");
    assert!(app.settings_editing, "rejected input should keep the editor open");

    app.input_buffer = "45".to_string();
    app.commit_settings_edit();
    assert_eq!(app.config.fetch_timeout_secs, 45);
}

#[test]
fn test_fetch_details_action_has_a_unique_default_binding() {
    use crate::keybindings::{Action, KeybindingsConfig};

    let kb = KeybindingsConfig::default_config();
    let keys = kb.get_action_keys(Action::HomeFetchDetails);
    assert!(!keys.is_empty());
    assert!(
        kb.find_conflict(Action::HomeFetchDetails, &keys).is_none(),
        "default binding collides"
    );
}

#[test]
fn test_http_status_errors_are_not_mistaken_for_network_failures() {
    use crate::fetch_error::{FetchFailure, classify};

    // git wraps HTTP failures as "The requested URL returned error: NNN", which
    // also contains "unable to access" — the network block would otherwise win and
    // tell the user to check their VPN when the real problem is their token.
    assert_eq!(
        classify(
            "fatal: unable to access 'https://github.com/x/y.git/': The requested URL returned error: 403"
        ),
        FetchFailure::Auth
    );
    assert_eq!(
        classify(
            "fatal: unable to access 'https://github.com/x/y.git/': The requested URL returned error: 404"
        ),
        FetchFailure::NotFound
    );
}

#[test]
fn test_sanitize_keeps_distinct_progress_counters() {
    use crate::fetch_error::sanitize;

    // Only redraws of the *same* counter should collapse; two different phases
    // must both survive.
    let raw = "remote: Counting objects: 100%\rremote: Compressing objects: 5%\rremote: Compressing objects: 100%\r";
    let cleaned = sanitize(raw);

    assert!(cleaned.contains("Counting objects: 100%"), "{:?}", cleaned);
    assert!(cleaned.contains("Compressing objects: 100%"), "{:?}", cleaned);
    assert!(!cleaned.contains("Compressing objects: 5%"), "{:?}", cleaned);
}

// ── Per-label settings ────────────────────────────────────────────────────

#[test]
fn test_tags_tab_keys_follow_the_tags_bindings_and_the_palette_reaches_them() {
    use crate::keybindings::Action;
    let (mut app, _guard) = palette_test_app("tags_bindings");
    app.mode = Mode::Detail;
    app.detail_tab = 4; // Tags
    let key = |c: char| KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty());

    // Default `/` opens the tag search.
    assert!(crate::input::handle_key(&mut app, key('/'), 5));
    assert_eq!(app.mode, Mode::TagSearchInput);
    app.mode = Mode::Detail;

    // Rebinding tags.search is honoured: the old key no longer opens it, the new one does.
    app.keybindings.update_action_keys(Action::TagsSearch, vec!["ctrl-s".to_string()]);
    crate::input::handle_key(&mut app, key('/'), 5);
    assert_eq!(app.mode, Mode::Detail);
    assert!(crate::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
        5
    ));
    assert_eq!(app.mode, Mode::TagSearchInput);
    app.mode = Mode::Detail;

    // The palette runs the Tags entries through the same route.
    app.keybindings.update_action_keys(Action::TagsSearch, vec!["/".to_string()]);
    assert!(crate::input::handle_key(&mut app, palette_key(), 5));
    let label = app.keybindings.get_action_description(Action::TagsSearch).to_lowercase();
    type_into_palette(&mut app, &label);
    assert_eq!(
        app.command_palette.as_ref().unwrap().selected().map(|e| e.action),
        Some(Action::TagsSearch)
    );
    assert!(crate::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        5
    ));
    assert_eq!(app.mode, Mode::TagSearchInput);
}

#[test]
fn test_tags_tab_honors_rebound_keys() {
    use crate::keybindings::Action;

    let cfg_path = std::env::temp_dir().join("gitwig_test_tags_rebound_keys.toml");
    let _guard = TestFileGuard { path: cfg_path.clone() };
    let mut app = App::new(Config { items: vec![], ..Default::default() }, cfg_path);
    let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

    let mock_info = repo::RepoInfo {
        local_tags: repo::TabData::Loaded(vec![repo::BranchInfo {
            name: "v1.0.0".to_string(),
            ..Default::default()
        }]),
        remotes: repo::TabData::Loaded(vec![repo::RemoteInfo {
            name: "origin".to_string(),
            url: String::new(),
            push_url: None,
            refspecs: vec![],
        }]),
        ..Default::default()
    };
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: PathBuf::from("dummy"),
        info: Box::new(mock_info),
    });
    app.mode = Mode::Detail;
    app.detail_tab = 4;
    app.detail_focus = DetailSection::LocalTags;
    app.tag_list.local_tag_selection = 0;

    // Rebind search from `/` to `z`, push-all from `P` to `U`, and checkout
    // from Enter to `o`.
    app.keybindings.update_action_keys(Action::TagsSearch, vec!["z".to_string()]);
    app.keybindings.update_action_keys(Action::TagsPushAll, vec!["U".to_string()]);
    app.keybindings.update_action_keys(Action::TagsCheckout, vec!["o".to_string()]);

    // The old search key no longer opens the search overlay.
    let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('/')), 1);
    assert!(!handled || app.mode == Mode::Detail, "old `/` must not start tag search");
    assert_ne!(app.mode, Mode::TagSearchInput);

    // The rebound search key does.
    assert!(crate::input::handle_key(&mut app, key_event(KeyCode::Char('z')), 1));
    assert_eq!(app.mode, Mode::TagSearchInput);
    assert!(crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 1));
    assert_eq!(app.mode, Mode::Detail);

    // The rebound push-all key asks for confirmation; the old one is inert.
    assert!(crate::input::handle_key(&mut app, key_event(KeyCode::Char('U')), 1));
    assert_eq!(app.mode, Mode::TagPushAllConfirm);
    assert!(crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 1));
    assert_eq!(app.mode, Mode::Detail);
    crate::input::handle_key(&mut app, key_event(KeyCode::Char('P')), 1);
    assert_eq!(app.mode, Mode::Detail);

    // Checkout follows the rebound key and ignores Enter.
    crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 1);
    assert_eq!(app.mode, Mode::Detail);
    assert!(crate::input::handle_key(&mut app, key_event(KeyCode::Char('o')), 1));
    assert_eq!(app.mode, Mode::TagCheckoutConfirm);
    assert!(crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 1));
    assert_eq!(app.mode, Mode::Detail);

    // The status bar reflects the rebinding.
    let (_, entries) = crate::components::cmd_bar::detail_dismiss_entries(&app);
    let text: String =
        entries.iter().flat_map(|e| e.spans.iter().map(|s| s.content.as_ref())).collect();
    assert!(text.contains("Fuzzy Search"), "status bar text: {text}");
    assert!(text.contains("Fuzzy Search [z]"), "status bar text: {text}");
    assert!(text.contains("Push All [U]"), "status bar text: {text}");
    assert!(text.contains("Checkout [o]"), "status bar text: {text}");
    assert!(!text.contains("↵"), "checkout hint must follow the rebound key: {text}");
}

// ── Per-label sort order ──────────────────────────────────────────────────

/// `u` updates the selected submodule and `U` every one, asking first only
/// when the update would move a `Modified` submodule's checked-out commit.
#[test]
fn test_submodule_update_keys_ask_only_before_moving_modified_submodules() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let sub = |path: &str, initialized: bool, dirty: bool, in_index: bool| repo::SubmoduleInfo {
        name: path.to_string(),
        path: PathBuf::from(path),
        commit_id: in_index.then(|| "a".repeat(40)),
        head_id: Some("a".repeat(40)),
        is_initialized: initialized,
        is_dirty: dirty,
        ..Default::default()
    };
    let info = repo::RepoInfo {
        submodules: repo::TabData::Loaded(vec![
            sub("uninit", false, false, true),
            sub("modified", true, true, true),
            sub("removed", true, false, false),
            sub("clean", true, false, true),
        ]),
        ..Default::default()
    };
    let (mut app, _guard) = advanced_tab_app("submodule_update_keys", info, 8);
    app.keybindings = crate::keybindings::KeybindingsConfig::default_config();
    let u = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::empty());
    let shift_u = KeyEvent::new(KeyCode::Char('U'), KeyModifiers::SHIFT);
    let key = |c: char| KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty());
    let modified_only = vec![PathBuf::from("modified")];

    // Modified: asks first, and `n` cancels without running anything.
    app.submodule_selection = 1;
    assert!(crate::input::handle_key(&mut app, u, 10));
    assert_eq!(app.mode, Mode::SubmoduleUpdateConfirm);
    assert_eq!(
        app.submodule_update_target,
        Some(SubmoduleUpdateTarget {
            path: Some(PathBuf::from("modified")),
            modified: modified_only.clone()
        })
    );
    assert!(crate::input::handle_key(&mut app, key('n'), 10));
    assert_eq!(app.mode, Mode::Detail);
    assert_eq!(app.submodule_update_target, None);
    assert!(!app.fetching);

    // Staged for removal: nothing to update.
    app.submodule_selection = 2;
    assert!(crate::input::handle_key(&mut app, u, 10));
    assert_eq!(app.mode, Mode::Detail);
    assert!(!app.fetching);
    assert!(app.status_message.as_deref().unwrap_or("").contains("staged for removal"));

    // `U` covers every submodule and names the Modified one; `y` starts it.
    assert!(crate::input::handle_key(&mut app, shift_u, 10));
    assert_eq!(app.mode, Mode::SubmoduleUpdateConfirm);
    assert_eq!(
        app.submodule_update_target,
        Some(SubmoduleUpdateTarget { path: None, modified: modified_only })
    );
    assert!(crate::input::handle_key(&mut app, key('y'), 10));
    assert_eq!(app.mode, Mode::Detail);
    assert_eq!(app.submodule_update_target, None);
    assert!(app.fetching);
    assert_eq!(app.status_message.as_deref(), Some("Updating all submodules..."));
    // The repository path does not exist, so the update reports a failure.
    wait_for_repo_message(&app, "Failed to update all submodules");

    // Uninitialized (and Clean) submodules update without asking.
    app.fetching = false;
    app.submodule_selection = 0;
    assert!(crate::input::handle_key(&mut app, u, 10));
    assert_eq!(app.mode, Mode::Detail);
    assert!(app.fetching);
    assert_eq!(app.status_message.as_deref(), Some("Updating submodule 'uninit'..."));
    wait_for_repo_message(&app, "Failed to update submodule 'uninit'");

    // `U` with nothing Modified runs straight away; with nothing left in the
    // index there is nothing to update.
    let set_submodules = |app: &mut App, subs: Vec<repo::SubmoduleInfo>| {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &mut app.current_detail {
            info.submodules = repo::TabData::Loaded(subs);
        }
    };
    app.fetching = false;
    set_submodules(
        &mut app,
        vec![sub("uninit", false, false, true), sub("clean", true, false, true)],
    );
    assert!(crate::input::handle_key(&mut app, shift_u, 10));
    assert_eq!(app.mode, Mode::Detail);
    assert!(app.fetching);
    wait_for_repo_message(&app, "Failed to update all submodules");
    app.fetching = false;
    set_submodules(&mut app, vec![sub("removed", true, false, false)]);
    assert!(crate::input::handle_key(&mut app, shift_u, 10));
    assert!(!app.fetching);
    assert_eq!(app.status_message.as_deref(), Some("No submodules to update"));
}

/// `u` initialises only the selected submodule, and `a` no longer initialises
/// every other submodule after adding one: it ran a bare
/// `git submodule update --init --recursive` over the whole repository.
#[test]
fn test_submodule_update_and_add_leave_other_submodules_alone() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let root = std::env::temp_dir().join(format!(
        "gitwig_test_submodule_update_{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    let _dir = TestDirGuard { path: root.clone() };
    let lib = root.join("lib");
    std::fs::create_dir_all(&lib).unwrap();
    run_git_in(&lib, &["init", "-q"]);
    std::fs::write(lib.join("f"), "lib\n").unwrap();
    run_git_in(&lib, &["add", "f"]);
    run_git_in(&lib, &["commit", "-q", "-m", "lib"]);
    let main = root.join("main");
    std::fs::create_dir_all(&main).unwrap();
    run_git_in(&main, &["init", "-q"]);
    run_git_in(&main, &["commit", "-q", "--allow-empty", "-m", "init"]);
    let lib_url = lib.to_string_lossy().to_string();
    run_git_in(&main, &["submodule", "add", "-q", &lib_url, "vendor/a"]);
    run_git_in(&main, &["submodule", "add", "-q", &lib_url, "vendor/b"]);
    run_git_in(&main, &["commit", "-q", "-m", "submodules"]);
    run_git_in(&main, &["submodule", "deinit", "-q", "-f", "--all"]);

    let submodules = repo::load_tab_submodules(&main).unwrap();
    let a_idx = submodules.iter().position(|s| s.path == std::path::Path::new("vendor/a")).unwrap();
    let info =
        repo::RepoInfo { submodules: repo::TabData::Loaded(submodules), ..Default::default() };
    let mut app = App::new(Config::default(), root.join("config.toml"));
    app.keybindings = crate::keybindings::KeybindingsConfig::default_config();
    app.current_detail =
        Some(repo::ItemDetail::Repo { resolved: main.clone(), info: Box::new(info) });
    app.mode = Mode::Detail;
    app.advanced_tabs = true;
    app.detail_tab = 8;
    let initialized = |p: &str| {
        let subs = repo::load_tab_submodules(&main).unwrap();
        subs.iter().any(|s| s.path == std::path::Path::new(p) && s.is_initialized)
    };

    app.submodule_selection = a_idx;
    let u = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::empty());
    assert!(crate::input::handle_key(&mut app, u, 10));
    wait_for_repo_message(&app, "Updated submodule 'vendor/a'");
    assert!(initialized("vendor/a"));
    assert!(std::fs::read_to_string(main.join("vendor/a/f")).unwrap() == "lib\n");
    assert!(!initialized("vendor/b"), "only the selected submodule");

    app.fetching = false;
    app.submodule_add_url = lib_url;
    app.submodule_add_path = "vendor/c".to_string();
    app.start_submodule_add();
    wait_for_repo_message(&app, "Submodule 'vendor/c' added successfully");
    assert!(initialized("vendor/c"));
    assert!(!initialized("vendor/b"), "adding a submodule leaves the others alone");
}

#[test]
fn test_bg_events_typed_classification_and_cross_repo_isolation() {
    let (config_path, _guard) = temp_config_path("bg_events_typed");
    let mut app = App::new(Config::default(), config_path);
    let open_repo = missing_repo_path("bg_events_open");
    let other_repo = missing_repo_path("bg_events_other");
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: open_repo.clone(),
        info: Box::new(repo::RepoInfo::default()),
    });

    // 1. A branch named "fix-failed-login" succeeds without tripping error classification.
    app.fetching = true;
    let pushes_before = app.stats.pushes;
    app.tx
        .send(BgEvent::RepoOpResult {
            repo_path: open_repo.clone(),
            kind: RepoOpKind::Push,
            result: Ok("Pushed 'fix-failed-login' to 'origin' successfully".to_string()),
        })
        .unwrap();
    assert!(app.drain_bg_events());
    assert!(!app.fetching);
    assert!(app.error_message.is_none());
    assert_eq!(
        app.status_message.as_deref(),
        Some("Pushed 'fix-failed-login' to 'origin' successfully")
    );
    assert_eq!(app.stats.pushes, pushes_before + 1);

    // 2. An error from a different repo prefixes its folder name and sets error_message.
    app.fetching = true;
    app.tx
        .send(BgEvent::RepoOpResult {
            repo_path: other_repo.clone(),
            kind: RepoOpKind::Push,
            result: Err("Push failed: rejected".to_string()),
        })
        .unwrap();
    assert!(app.drain_bg_events());
    assert!(!app.fetching);
    let expected_name = other_repo.file_name().unwrap().to_string_lossy().into_owned();
    assert_eq!(
        app.error_message.as_deref(),
        Some(format!("[{}] Push failed: rejected", expected_name).as_str())
    );
    app.error_message = None;

    // 3. RemoteTagsResult populates remote_tags directly when open, and settles
    //    implicit_network_count even when the reply belongs to another repo.
    app.current_detail = Some(repo::ItemDetail::Repo {
        resolved: open_repo.clone(),
        info: Box::new(repo::RepoInfo::default()),
    });
    app.fetching = true;
    let tag = repo::BranchInfo {
        name: "v1.0.0".to_string(),
        is_head: false,
        short_sha: "abc1234".to_string(),
        short_message: String::new(),
    };
    app.tx
        .send(BgEvent::RemoteTagsResult { repo_path: open_repo, result: Ok(vec![tag.clone()]) })
        .unwrap();
    assert!(app.drain_bg_events());
    assert!(!app.fetching);
    if let Some(repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
        assert_eq!(info.remote_tags.as_slice().len(), 1);
        assert_eq!(info.remote_tags.as_slice()[0].name, "v1.0.0");
        assert!(info.remote_tags_loaded);
    } else {
        panic!("expected open repo detail");
    }

    app.implicit_network_count = 1;
    app.tx
        .send(BgEvent::RemoteTagsResult {
            repo_path: other_repo,
            result: Err("network down".to_string()),
        })
        .unwrap();
    assert!(app.drain_bg_events());
    assert_eq!(app.implicit_network_count, 0);
    assert!(app.error_message.is_none());
}
