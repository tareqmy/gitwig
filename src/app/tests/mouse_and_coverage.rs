#![allow(clippy::unwrap_used, clippy::panic)]

use super::*;

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
fn test_commit_popup_mouse_resize() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Rect;

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
fn test_update_click_trigger() {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    let config = Config::default();
    let (config_path, _guard) = temp_config_path("update_click_trigger");
    let mut app = App::new(config, config_path);
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
fn test_resize_focused_panel_keys() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    let config = Config { items: vec!["/path/to/repo_a".to_string()], ..Default::default() };
    // Own config dir: the shared $TMPDIR keybindings.toml may hold stale
    // defaults from older builds, which would shadow the bindings under test.
    let temp_dir = std::env::temp_dir().join("gitwig_test_resize_panel");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();
    let temp_path = temp_dir.join("config.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };
    let mut app = App::new(config, temp_path);
    app.mode = Mode::Detail;

    let grow = KeyEvent::new(KeyCode::Char('+'), KeyModifiers::SHIFT);
    let shrink = KeyEvent::new(KeyCode::Char('-'), KeyModifiers::empty());

    // Workspace tab, commits panel owns the top (pct) side of the main split.
    app.detail_tab = 0;
    app.detail_focus = DetailSection::Commits;
    let base = app.workspace_main_split_pct;
    assert!(crate::input::handle_key(&mut app, grow, 10));
    assert_eq!(app.workspace_main_split_pct, base + 5);
    assert!(crate::input::handle_key(&mut app, shrink, 10));
    assert_eq!(app.workspace_main_split_pct, base);

    // '=' (unshifted '+' on many layouts) is deliberately not bound.
    let eq_key = KeyEvent::new(KeyCode::Char('='), KeyModifiers::empty());
    assert!(crate::input::handle_key(&mut app, eq_key, 10));
    assert_eq!(app.workspace_main_split_pct, base);

    // The diff panel sits on the complement side: growing it shrinks the pct.
    app.detail_focus = DetailSection::StagingDetails;
    let base = app.inspect_horizontal_split_pct;
    assert!(crate::input::handle_key(&mut app, grow, 10));
    assert_eq!(app.inspect_horizontal_split_pct, base - 5);
    assert!(crate::input::handle_key(&mut app, shrink, 10));
    assert_eq!(app.inspect_horizontal_split_pct, base);

    // The same focus in the stashes tab resizes the stashes split instead.
    app.detail_tab = 6;
    let base = app.stashes_horizontal_split_pct;
    assert!(crate::input::handle_key(&mut app, grow, 10));
    assert_eq!(app.stashes_horizontal_split_pct, base - 5);
    app.stashes_horizontal_split_pct = base;

    // Clamped to the same 15..=85 range the mouse drag enforces.
    app.detail_tab = 1;
    app.detail_focus = DetailSection::Files;
    app.files_horizontal_split_pct = 83;
    assert!(crate::input::handle_key(&mut app, grow, 10));
    assert_eq!(app.files_horizontal_split_pct, 85);
    assert!(crate::input::handle_key(&mut app, grow, 10));
    assert_eq!(app.files_horizontal_split_pct, 85);
    app.files_horizontal_split_pct = 17;
    assert!(crate::input::handle_key(&mut app, shrink, 10));
    assert_eq!(app.files_horizontal_split_pct, 15);
    assert!(crate::input::handle_key(&mut app, shrink, 10));
    assert_eq!(app.files_horizontal_split_pct, 15);

    // A focus without an adjustable split is a harmless no-op.
    app.detail_tab = 4;
    app.detail_focus = DetailSection::LocalTags;
    let snapshot = (
        app.workspace_main_split_pct,
        app.inspect_horizontal_split_pct,
        app.files_horizontal_split_pct,
        app.branches_horizontal_split_pct,
    );
    assert!(crate::input::handle_key(&mut app, grow, 10));
    assert_eq!(
        snapshot,
        (
            app.workspace_main_split_pct,
            app.inspect_horizontal_split_pct,
            app.files_horizontal_split_pct,
            app.branches_horizontal_split_pct,
        )
    );
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
    app.submodule_delete_target = Some(crate::repo::SubmoduleInfo {
        name: "submodule_a".to_string(),
        path: std::path::PathBuf::from("submodule_a"),
        ..Default::default()
    });

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
        Mode::TagOverwriteConfirm,
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

/// Clicks on the Submodules and Reflog lists did nothing, and clicks on a
/// scrolled list picked the row at that height in an unscrolled one.
#[test]
fn test_list_mouse_clicks_map_through_the_scroll_offset() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let at = |kind: MouseEventKind, row: u16| MouseEvent {
        kind,
        column: 10,
        row,
        modifiers: crossterm::event::KeyModifiers::empty(),
    };
    let click = |row: u16| at(MouseEventKind::Down(MouseButton::Left), row);
    let submodule = |i: usize| crate::repo::SubmoduleInfo {
        name: format!("lib{}", i),
        path: PathBuf::from(format!("lib{}", i)),
        ..Default::default()
    };
    // Lists drawn at rows 5..25, the table (header + margin first) at 6..24.
    let outer = ratatui::layout::Rect::new(0, 5, 80, 20);
    let inner = ratatui::layout::Rect::new(1, 6, 78, 18);

    let info = crate::repo::RepoInfo {
        submodules: crate::repo::TabData::Loaded((0..30).map(submodule).collect()),
        ..Default::default()
    };
    let (mut app, _guard) = advanced_tab_app("submodule_mouse", info, 8);
    app.detail_areas.submodules = Some(outer);
    app.detail_areas.submodules_inner = Some(inner);
    app.detail_areas.submodules_offset = 10;
    crate::mouse::handle_mouse(&mut app, click(8)); // first body row
    assert_eq!(app.submodule_selection, 10);
    assert_eq!(app.detail_focus, DetailSection::Submodules);
    crate::mouse::handle_mouse(&mut app, click(11));
    assert_eq!(app.submodule_selection, 13);
    crate::mouse::handle_mouse(&mut app, at(MouseEventKind::ScrollDown, 11));
    assert_eq!(app.submodule_selection, 14);
    crate::mouse::handle_mouse(&mut app, click(6)); // the header selects nothing
    assert_eq!(app.submodule_selection, 14);

    let info = crate::repo::RepoInfo {
        reflog: crate::repo::TabData::Loaded((0..5).map(test_reflog_entry).collect()),
        ..Default::default()
    };
    let (mut app, _guard) = advanced_tab_app("reflog_mouse", info, 9);
    app.detail_areas.reflog = Some(outer);
    app.detail_areas.reflog_inner = Some(inner);
    crate::mouse::handle_mouse(&mut app, click(10));
    assert_eq!(app.reflog_selection, 2);
    assert_eq!(app.detail_focus, DetailSection::Reflog);
    crate::mouse::handle_mouse(&mut app, click(20)); // below the last row
    assert_eq!(app.reflog_selection, 2);
    crate::mouse::handle_mouse(&mut app, at(MouseEventKind::ScrollUp, 10));
    assert_eq!(app.reflog_selection, 1);
}

#[test]
fn test_app_actions_coverage() {
    let config = Config {
        items: vec!["/dummy/path/a".to_string()],
        poll_interval_ms: 100,
        max_commits: 0,
        page_size: 10,
        sort_by: SortOrder::Custom,
        ..Default::default()
    };
    let temp_path = std::env::temp_dir().join("gitwig_test_config_actions_cov.toml");
    let _guard = TestFileGuard { path: temp_path.clone() };

    let mut app = App::new(config, temp_path);

    // Test basic edit/add state changes
    app.start_add();
    app.start_edit();
    app.request_delete();

    app.input_buffer = "/dummy/path/b".to_string();
    app.commit_add();

    app.input_buffer = "  /dummy/path/c  ".to_string();
    app.commit_add_with_labels(app.input_buffer.clone(), vec!["label1".to_string()]);

    app.input_buffer = "label-input".to_string();
    app.commit_add_label_input();

    app.add_repo_path("/dummy/path/d".to_string());

    app.input_buffer = "/dummy/path_edited".to_string();
    app.commit_edit();

    app.confirm_delete();

    app.start_edit_labels();
    app.commit_edit_labels();

    app.start_repo_scan();
    app.start_bulk_repo_scan();

    let matches = app.get_scan_matches();
    assert!(matches.is_empty());

    app.start_branch_search();
    let branch_matches = app.get_branch_search_matches();
    assert!(branch_matches.is_empty());

    app.start_file_search();
    let file_matches = app.get_file_search_matches();
    assert!(file_matches.is_empty());

    app.start_commit_fuzzy_search();
    let commit_matches = app.get_commit_fuzzy_matches();
    assert!(commit_matches.is_empty());

    app.start_tag_search();
    let tag_matches = app.get_tag_search_matches();
    assert!(tag_matches.is_empty());

    app.trigger_global_search();
    app.select_global_search_result();

    app.auto_discover_add("/dummy/path/e".to_string());

    // Workspace action coverage boost
    app.request_revert();
    app.confirm_revert();
    app.cancel_revert();
    app.cancel_commit_search();
    app.cancel_commit();
}

#[test]
fn test_navigation_boilerplates_coverage_boost() {
    let config = Config::default();
    let temp_config_path = std::env::temp_dir().join("gitwig_test_nav_boilerplates.toml");
    let _guard = TestFileGuard { path: temp_config_path.clone() };
    let mut app = App::new(config, temp_config_path);

    // 1. local_branch
    app.local_branch_up();
    app.local_branch_down();
    app.local_branch_page_up(2);
    app.local_branch_page_down(2);
    app.local_branch_to_top();
    app.local_branch_to_bottom();

    // 2. remote_branch
    app.remote_branch_up();
    app.remote_branch_down();
    app.remote_branch_page_up(2);
    app.remote_branch_page_down(2);
    app.remote_branch_to_top();
    app.remote_branch_to_bottom();

    // 3. file_list
    app.file_list_up();
    app.file_list_down();
    app.file_list_page_up(2);
    app.file_list_page_down(2);
    app.file_list_to_top();
    app.file_list_to_bottom();

    // 4. detail_commit
    app.detail_commit_up();
    app.detail_commit_down();
    app.detail_commit_page_up(2);
    app.detail_commit_page_down(2);
    app.detail_commit_to_top();
    app.detail_commit_to_bottom();

    // 5. detail_file / staging_file / conflict_file
    app.detail_file_up();
    app.detail_file_down();
    app.staging_file_up();
    app.staging_file_down();
    app.conflict_file_up();
    app.conflict_file_down();

    // 6. diff_hunk / diff_line
    app.diff_hunk_up();
    app.diff_hunk_down();
    app.diff_line_up();
    app.diff_line_down();

    // 7. overview_scroll
    app.overview_scroll_up();
    app.overview_scroll_down();
    app.overview_scroll_page_up(2);
    app.overview_scroll_page_down(2);
    app.overview_scroll_to_top();
    app.overview_scroll_to_bottom();

    // 8. file_content_scroll
    app.file_content_scroll_up();
    app.file_content_scroll_down();
    app.file_content_scroll_page_up(2);
    app.file_content_scroll_page_down(2);
    app.file_content_scroll_to_top();
    app.file_content_scroll_to_bottom();

    // 9. graph_select
    app.graph_select_up();
    app.graph_select_down();
    app.graph_select_page_up(2);
    app.graph_select_page_down(2);
    app.graph_select_to_top();
    app.graph_select_to_bottom();

    // 10. scrolls
    app.commit_details_scroll_up();
    app.commit_details_scroll_down();
    app.commit_input_scroll_up();
    app.commit_input_scroll_down();

    // 11. remote_picker / local_tag
    app.remote_picker_up();
    app.remote_picker_down();
    app.local_tag_up();
    app.local_tag_down();
    app.local_tag_page_up(2);
    app.local_tag_page_down(2);
    app.local_tag_to_top();
    app.local_tag_to_bottom();

    // 12. remote / stash
    app.remote_up();
    app.remote_down();
    app.remote_page_up(2);
    app.remote_page_down(2);
    app.remote_to_top();
    app.remote_to_bottom();

    app.stash_up();
    app.stash_down();
    app.stash_page_up(2);
    app.stash_page_down(2);
    app.stash_to_top();
    app.stash_to_bottom();

    // 13. stash_file
    app.stash_file_up();
    app.stash_file_down();
    app.stash_file_page_up(2);
    app.stash_file_page_down(2);
    app.stash_file_to_top();
    app.stash_file_to_bottom();

    // 14. help / legend scroll
    app.help_scroll_up();
    app.help_scroll_down();
    app.help_scroll_page_up(2);
    app.help_scroll_page_down(2);
    app.help_scroll_to_top();
    app.help_scroll_to_bottom();
    app.legend_scroll_up();
    app.legend_scroll_down();
    app.legend_scroll_page_up(2);
    app.legend_scroll_page_down(2);
    app.legend_scroll_to_top();
    app.legend_scroll_to_bottom();
}

// ---------------------------------------------------------------------------
// Remote fetch failure handling
// ---------------------------------------------------------------------------
