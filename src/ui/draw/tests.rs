#![allow(clippy::unwrap_used, clippy::panic)]

use ratatui::layout::Rect;
use ratatui::style::Modifier;

use super::helpers::{wrap_excludes, wrap_str};
use super::home::*;
use crate::app::{App, DetailSection, Mode};
use crate::components::cmd_bar::{detail_dismiss_entries, inspect_dismiss_entries};
use crate::config::{Config, ScanConfig, SortOrder, ThemeConfig};
use crate::repo::{FileEntry, ItemDetail, RepoInfo};
use crate::repo::{ItemStatus, RepoState, RepoSummary};
use crate::ui::{DANGER, SUCCESS, WARNING};
use std::path::PathBuf;

#[test]
fn test_inspect_status_bar_shortcuts() {
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
    let (config_path, _guard) =
        crate::test_support::temp_config_path("inspect_status_bar_shortcuts");
    let mut app = App::new(config, config_path);

    // 1. Setup dirty working tree detail
    let mut info = RepoInfo::default();
    info.changes.staged.push(FileEntry { path: "file.txt".to_string(), label: "M" });

    app.current_detail =
        Some(ItemDetail::Repo { resolved: PathBuf::from("/dummy"), info: Box::new(info) });
    app.commit_list.selection = 0; // selection = uncommitted
    app.in_logs_ui = false;

    // A) Staged focus -> Unstage File [↵]
    app.detail_focus = DetailSection::Staged;
    let (_, entries) = inspect_dismiss_entries(&app);
    let entry_labels: Vec<String> = entries
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels.iter().any(|label| label.contains("Unstage File [↵]")));
    assert!(entry_labels.iter().any(|label| label.contains("Unstage All [a]")));
    assert!(entry_labels.iter().any(|label| label.contains("Discard [x]")));
    assert!(entry_labels.iter().any(|label| label.contains("Discard All [X]")));
    assert!(entry_labels.iter().any(|label| label.contains("Commit/Amend [c/C]")));

    // B) Unstaged focus -> Stage File [↵]
    app.detail_focus = DetailSection::Unstaged;
    let (_, entries) = inspect_dismiss_entries(&app);
    let entry_labels: Vec<String> = entries
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels.iter().any(|label| label.contains("Stage File [↵]")));
    assert!(entry_labels.iter().any(|label| label.contains("Stage All [a]")));
    assert!(entry_labels.iter().any(|label| label.contains("Discard [x]")));
    assert!(entry_labels.iter().any(|label| label.contains("Discard All [X]")));

    // C) StagingDetails with last_staging_focus == Staged -> Unstage Hunk [↵]
    app.detail_focus = DetailSection::StagingDetails;
    app.last_staging_focus = DetailSection::Staged;
    let (_, entries) = inspect_dismiss_entries(&app);
    let entry_labels: Vec<String> = entries
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels.iter().any(|label| label.contains("Unstage Hunk [↵/u]")));

    // D) StagingDetails with last_staging_focus == Unstaged -> Stage Hunk [↵]
    app.detail_focus = DetailSection::StagingDetails;
    app.last_staging_focus = DetailSection::Unstaged;
    let (_, entries) = inspect_dismiss_entries(&app);
    let entry_labels: Vec<String> = entries
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels.iter().any(|label| label.contains("Stage Hunk [↵/s]")));
    assert!(entry_labels.iter().any(|label| label.contains("Discard Hunk [x/Del]")));

    // D2) StagingDetails with last_staging_focus == Unstaged and diff_line_mode == true -> Stage Line [↵/s] & Discard Line [x/Del] & Hunk Mode [l]
    app.diff.diff_line_mode = true;
    let (_, entries) = inspect_dismiss_entries(&app);
    let entry_labels: Vec<String> = entries
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels.iter().any(|label| label.contains("Stage Line [↵/s]")));
    assert!(entry_labels.iter().any(|label| label.contains("Discard Line [x/Del]")));
    assert!(entry_labels.iter().any(|label| label.contains("Hunk Mode [l]")));

    // D3) Full screen diff mode -> Commit [c]
    app.inspect_full_diff = true;
    let (_, entries_full) = inspect_dismiss_entries(&app);
    let entry_labels_full: Vec<String> = entries_full
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels_full.iter().any(|label| label.contains("Commit/Amend [c/C]")));

    // E) If in_logs_ui is true, it should NOT render any staging entry
    app.inspect_full_diff = false;
    app.in_logs_ui = true;
    let (_, entries) = inspect_dismiss_entries(&app);
    let entry_labels: Vec<String> = entries
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(!entry_labels.iter().any(|label| label.contains("Stage") || label.contains("Unstage")));

    // F) Conflicts focus in Inspect mode -> Accept Ours [o] etc.
    app.in_logs_ui = false;
    app.detail_focus = DetailSection::Conflicts;
    let (_, entries_c) = inspect_dismiss_entries(&app);
    let entry_labels_c: Vec<String> = entries_c
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels_c.iter().any(|label| label.contains("Accept Ours [o]")));
    assert!(entry_labels_c.iter().any(|label| label.contains("Accept Theirs [t]")));
    assert!(entry_labels_c.iter().any(|label| label.contains("Mark Resolved [r]")));
    assert!(entry_labels_c.iter().any(|label| label.contains("Abort Merge [A]")));
    assert!(entry_labels_c.iter().any(|label| label.contains("Continue Merge [C]")));
    assert!(entry_labels_c.iter().any(|label| label.contains("Inspect [↵/→]")));

    // G) ConflictDiff focus in Inspect mode -> Accept Ours [o] etc.
    app.detail_focus = DetailSection::ConflictDiff;
    let (_, entries_cd) = inspect_dismiss_entries(&app);
    let entry_labels_cd: Vec<String> = entries_cd
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels_cd.iter().any(|label| label.contains("Accept Ours [o]")));
    assert!(entry_labels_cd.iter().any(|label| label.contains("Accept Theirs [t]")));
    assert!(entry_labels_cd.iter().any(|label| label.contains("Mark Resolved [r]")));
    assert!(entry_labels_cd.iter().any(|label| label.contains("Abort Merge [A]")));
    assert!(entry_labels_cd.iter().any(|label| label.contains("Continue Merge [C]")));
    assert!(entry_labels_cd.iter().any(|label| label.contains("Scroll Diff [↑↓/⇟⇞]")));
}

#[test]
fn test_detail_dismiss_entries_shortcuts() {
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
    let (config_path, _guard) =
        crate::test_support::temp_config_path("detail_dismiss_entries_shortcuts");
    let mut app = App::new(config, config_path);

    // Tab 0: Workspace, Commits focus (default)
    app.detail_tab = 0;
    app.detail_focus = DetailSection::Commits;
    let (_, entries_w) = detail_dismiss_entries(&app);
    let entry_labels_w: Vec<String> = entries_w
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels_w.iter().any(|label| label.contains("Inspect [↵/→]")));
    assert!(entry_labels_w.iter().any(|label| label.contains("Tag [t/T]")));
    assert!(entry_labels_w.iter().any(|label| label.contains("Checkout [o]")));
    assert!(entry_labels_w.iter().any(|label| label.contains("Cherry-pick [p/P]")));
    assert!(entry_labels_w.iter().any(|label| label.contains("Load More [G]")));
    assert!(entry_labels_w.iter().any(|label| label.contains("Yank Hash [y/Y]")));

    // The captions follow the live keybindings, not a stale literal.
    app.keybindings.update_action_keys(
        crate::keybindings::Action::WorkspaceCreateTag,
        vec!["ctrl-t".to_string()],
    );
    let (_, entries_rebound) = detail_dismiss_entries(&app);
    let entry_labels_rebound: Vec<String> = entries_rebound
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels_rebound.iter().any(|label| label.contains("Tag [ctrl-t]")));
    assert!(!entry_labels_rebound.iter().any(|label| label.contains("Tag [t/T]")));
    app.keybindings.update_action_keys(
        crate::keybindings::Action::WorkspaceCreateTag,
        vec!["t".to_string(), "T".to_string()],
    );

    // Setup uncommitted changes mock for Tab 0 uncommitted shortcuts
    let mut info = RepoInfo::default();
    info.changes.staged.push(FileEntry { path: "file.txt".to_string(), label: "M" });
    info.changes.unstaged.push(FileEntry { path: "other.txt".to_string(), label: "M" });
    app.current_detail =
        Some(ItemDetail::Repo { resolved: PathBuf::from("/dummy"), info: Box::new(info) });
    app.commit_list.selection = 0; // selection = uncommitted

    // Tab 0: Workspace, Staged focus
    app.detail_focus = DetailSection::Staged;
    let (_, entries_s) = detail_dismiss_entries(&app);
    let entry_labels_s: Vec<String> = entries_s
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels_s.iter().any(|label| label.contains("Inspect [→]")));
    assert!(entry_labels_s.iter().any(|label| label.contains("Unstage All [a/A]")));
    assert!(entry_labels_s.iter().any(|label| label.contains("Discard All [X]")));
    assert!(!entry_labels_s.iter().any(|label| label.contains("Tag [t/T]")));

    // Tab 0: Workspace, Unstaged focus
    app.detail_focus = DetailSection::Unstaged;
    let (_, entries_u) = detail_dismiss_entries(&app);
    let entry_labels_u: Vec<String> = entries_u
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels_u.iter().any(|label| label.contains("Stage All [a/A]")));
    assert!(entry_labels_u.iter().any(|label| label.contains("Discard All [X]")));

    // Tab 0: Workspace, StagingDetails focus
    app.detail_focus = DetailSection::StagingDetails;
    let (_, entries_sd) = detail_dismiss_entries(&app);
    let entry_labels_sd: Vec<String> = entries_sd
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels_sd.iter().any(|label| label.contains("Full Screen [→]")));
    assert!(entry_labels_sd.iter().any(|label| label.contains("Line Mode [l/L]")));
    assert!(entry_labels_sd.iter().any(|label| label.contains("Stage [s/S]")));
    assert!(entry_labels_sd.iter().any(|label| label.contains("Unstage [u/U]")));
    assert!(!entry_labels_sd.iter().any(|label| label.contains("Stash [s/S]")));
    assert!(!entry_labels_sd.iter().any(|label| label.contains("Tag [t/T]")));

    // Tab 1: Files - Files Focus
    app.detail_tab = 1;
    app.detail_focus = DetailSection::Files;
    let (_, entries_f1) = detail_dismiss_entries(&app);
    let entry_labels_f1: Vec<String> = entries_f1
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels_f1.iter().any(|label| label.contains("Fuzzy Find [/]")));
    assert!(entry_labels_f1.iter().any(|label| label.contains("Expand/Toggle [→/↵/>]")));
    assert!(entry_labels_f1.iter().any(|label| label.contains("Collapse All [←]")));
    assert!(entry_labels_f1.iter().any(|label| label.contains("History [H]")));
    assert!(entry_labels_f1.iter().any(|label| label.contains("Discard [x/X]")));
    assert!(!entry_labels_f1.iter().any(|label| label.contains("Open in Editor [e/o]")));

    // Add a file tree item that is a directory
    app.file_tree.visible_files.push(crate::app::FileTreeItem {
        name: "src".to_string(),
        full_path: "src".to_string(),
        is_dir: true,
        depth: 0,
        is_expanded: true,
    });
    app.file_tree.file_list_selection = 0;
    let (_, entries_f1_dir) = detail_dismiss_entries(&app);
    let entry_labels_f1_dir: Vec<String> = entries_f1_dir
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(!entry_labels_f1_dir.iter().any(|label| label.contains("Open in Editor [e/o]")));

    // Add a file tree item that is a file
    app.file_tree.visible_files.push(crate::app::FileTreeItem {
        name: "main.rs".to_string(),
        full_path: "src/main.rs".to_string(),
        is_dir: false,
        depth: 1,
        is_expanded: false,
    });
    app.file_tree.file_list_selection = 1;
    let (_, entries_f1_file) = detail_dismiss_entries(&app);
    let entry_labels_f1_file: Vec<String> = entries_f1_file
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels_f1_file.iter().any(|label| label.contains("Open in Editor [e/o]")));

    // Tab 1: Files - FileContent Focus
    app.detail_focus = DetailSection::FileContent;
    let (_, entries_f2) = detail_dismiss_entries(&app);
    let entry_labels_f2: Vec<String> = entries_f2
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(!entry_labels_f2.iter().any(|label| label.contains("Fuzzy Find [f]")));
    assert!(!entry_labels_f2.iter().any(|label| label.contains("Expand/Collapse [←/→]")));
    assert!(!entry_labels_f2.iter().any(|label| label.contains("History [H]")));
    assert!(entry_labels_f2.iter().any(|label| label.contains("Full Screen [→]")));
    assert!(entry_labels_f2.iter().any(|label| label.contains("Open in Editor [e/o]")));

    app.inspect_full_diff = true;
    let (_, entries_f2_full) = detail_dismiss_entries(&app);
    let entry_labels_f2_full: Vec<String> = entries_f2_full
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels_f2_full.iter().any(|label| label.contains("Exit Full Screen [←/⎋/q/Q]")));
    app.inspect_full_diff = false;

    // Tab 3: Branches - LocalBranches Focus
    app.detail_tab = 3;
    app.detail_focus = DetailSection::LocalBranches;
    let (_, entries_b1) = detail_dismiss_entries(&app);
    let entry_labels_b1: Vec<String> = entries_b1
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels_b1.iter().any(|label| label.contains("Fuzzy Search [/]")));
    assert!(entry_labels_b1.iter().any(|label| label.contains("Fetch [f/F]")));
    assert!(entry_labels_b1.iter().any(|label| label.contains("Add Remote [a/A]")));
    assert!(entry_labels_b1.iter().any(|label| label.contains("Create [c/C]")));
    assert!(entry_labels_b1.iter().any(|label| label.contains("Interactive Rebase [i/I]")));
    assert!(entry_labels_b1.iter().any(|label| label.contains("Merge Into [M]")));
    assert!(entry_labels_b1.iter().any(|label| label.contains("Pull [p]")));
    assert!(entry_labels_b1.iter().any(|label| label.contains("Push [P]")));

    // Tab 3: Branches - RemoteBranches Focus
    app.detail_focus = DetailSection::RemoteBranches;
    let (_, entries_b2) = detail_dismiss_entries(&app);
    let entry_labels_b2: Vec<String> = entries_b2
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    // Fetch and Add Remote are not focus-gated (src/components/branch_list.rs),
    // so they stay visible from the remote panel too.
    assert!(entry_labels_b2.iter().any(|label| label.contains("Fetch [f/F]")));
    assert!(entry_labels_b2.iter().any(|label| label.contains("Add Remote [a/A]")));
    assert!(!entry_labels_b2.iter().any(|label| label.contains("Merge Into [M]")));
    assert!(!entry_labels_b2.iter().any(|label| label.contains("Pull [p]")));
    assert!(!entry_labels_b2.iter().any(|label| label.contains("Push [P]")));

    // Tab 6: Stashes - Stashes Focus
    app.detail_tab = 6;
    app.detail_focus = DetailSection::Stashes;
    let (_, entries_s1) = detail_dismiss_entries(&app);
    let entry_labels_s1: Vec<String> = entries_s1
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels_s1.iter().any(|label| label.contains("Apply [a/A]")));
    assert!(entry_labels_s1.iter().any(|label| label.contains("Delete [D]")));

    // Tab 6: Stashes - StashedFiles Focus
    app.detail_focus = DetailSection::StashedFiles;
    let (_, entries_s2) = detail_dismiss_entries(&app);
    let entry_labels_s2: Vec<String> = entries_s2
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(!entry_labels_s2.iter().any(|label| label.contains("Apply [a/A]")));
    assert!(!entry_labels_s2.iter().any(|label| label.contains("Delete [D]")));

    // Tab 4: Tags
    app.detail_tab = 4;
    let (_, entries_tags) = detail_dismiss_entries(&app);
    let entry_labels_tags: Vec<String> = entries_tags
        .iter()
        .map(|entry| entry.spans.iter().map(|s| s.content.as_ref()).collect::<Vec<&str>>().join(""))
        .collect();
    assert!(entry_labels_tags.iter().any(|label| label.contains("Fuzzy Search [/]")));
    assert!(entry_labels_tags.iter().any(|label| label.contains("Fetch [f/F]")));
    assert!(entry_labels_tags.iter().any(|label| label.contains("Checkout [↵]")));
    assert!(entry_labels_tags.iter().any(|label| label.contains("Push All [P]")));
}

#[test]
fn test_wrap_str() {
    let wrapped = wrap_str("node_modules,target,build,dist", 10);
    assert_eq!(
        wrapped,
        vec!["node_modul".to_string(), "es,target,".to_string(), "build,dist".to_string(),]
    );

    let wrapped_empty = wrap_str("", 10);
    assert_eq!(wrapped_empty, vec!["".to_string()]);
}

#[test]
fn test_wrap_excludes() {
    // Items that fit together on one line stay together
    let w = wrap_excludes("node_modules,target", 30);
    assert_eq!(w, vec!["node_modules,target"]);

    // Items that overflow wrap to the next line
    let w = wrap_excludes("node_modules,target,build,dist", 20);
    // "node_modules," = 13, "target," = 7 → 20 fits on line 1
    // "build," = 6, "dist" = 4 → fits on line 2
    assert_eq!(w, vec!["node_modules,target,", "build,dist"]);

    // Single very long item gets hard-wrapped
    let w = wrap_excludes("averylongnamehere", 10);
    assert_eq!(w, vec!["averylongn", "amehere"]);

    // Empty string returns a single empty line
    let w = wrap_excludes("", 20);
    assert_eq!(w, vec![""]);
}

#[test]
fn test_settings_val_str() {
    let config =
        Config { editor: "vim".to_string(), ssh_strict_host_checking: true, ..Default::default() };
    let (config_path, _guard) = crate::test_support::temp_config_path("settings_val_str");
    let mut app = App::new(config, config_path);

    // Setting index 55: SSH Strict Host Checking
    let val_ssh = crate::popups::settings::get_val_str(&app, 55);
    assert_eq!(val_ssh, "true");

    // Setting index 56: Editor Command
    app.settings_selected_index = 56;
    app.settings_editing = false;
    let val_editor = crate::popups::settings::get_val_str(&app, 56);
    assert_eq!(val_editor, "vim");

    // During editing
    app.settings_editing = true;
    app.set_input_buffer("nano".to_string());
    let val_editor_edit = crate::popups::settings::get_val_str(&app, 56);
    assert_eq!(val_editor_edit, "nano█");
}

#[test]
fn test_repo_indicator_line_divergence() {
    let config = Config::default();
    let (config_path, _guard) =
        crate::test_support::temp_config_path("repo_indicator_line_divergence");
    let app = App::new(config, config_path);

    // Case 1: Ahead <= 3 (Green)
    let summary1 = RepoSummary {
        ahead: 2,
        behind: 0,
        staged: 0,
        modified: 0,
        untracked: 0,
        conflicted: 0,
        branch: None,
        state: RepoState::Clean,
        last_commit_time: None,
    };
    let line1 = repo_indicator_line(&app, &summary1);
    let ahead_span = line1.spans.iter().find(|s| s.content.contains(app.sym("up"))).unwrap();
    assert_eq!(ahead_span.style.fg, Some(SUCCESS()));

    // Case 2: Ahead <= 10 (Yellow)
    let summary2 = RepoSummary {
        ahead: 7,
        behind: 0,
        staged: 0,
        modified: 0,
        untracked: 0,
        conflicted: 0,
        branch: None,
        state: RepoState::Clean,
        last_commit_time: None,
    };
    let line2 = repo_indicator_line(&app, &summary2);
    let ahead_span2 = line2.spans.iter().find(|s| s.content.contains(app.sym("up"))).unwrap();
    assert_eq!(ahead_span2.style.fg, Some(WARNING()));

    // Case 3: Behind <= 5 (Yellow)
    let summary3 = RepoSummary {
        ahead: 0,
        behind: 3,
        staged: 0,
        modified: 0,
        untracked: 0,
        conflicted: 0,
        branch: None,
        state: RepoState::Clean,
        last_commit_time: None,
    };
    let line3 = repo_indicator_line(&app, &summary3);
    let behind_span = line3.spans.iter().find(|s| s.content.contains(app.sym("down"))).unwrap();
    assert_eq!(behind_span.style.fg, Some(WARNING()));

    // Case 4: Behind > 5 (Red)
    let summary4 = RepoSummary {
        ahead: 0,
        behind: 8,
        staged: 0,
        modified: 0,
        untracked: 0,
        conflicted: 0,
        branch: None,
        state: RepoState::Clean,
        last_commit_time: None,
    };
    let line4 = repo_indicator_line(&app, &summary4);
    let behind_span2 = line4.spans.iter().find(|s| s.content.contains(app.sym("down"))).unwrap();
    assert_eq!(behind_span2.style.fg, Some(DANGER()));
}

#[test]
fn test_draw_quick_label_bar_marks_active_chip() {
    let mut config = Config {
        items: vec!["/path/to/repo_a".to_string(), "/path/to/repo_b".to_string()],
        ..Default::default()
    };
    config.labels.insert("/path/to/repo_a".to_string(), vec!["web".to_string()]);
    config.labels.insert("/path/to/repo_b".to_string(), vec!["web".to_string(), "api".to_string()]);
    let (config_path, _guard) =
        crate::test_support::temp_config_path("draw_quick_label_bar_marks_active_chip");
    let mut app = App::new(config, config_path);
    // Viewed api first, then web → slot 1 = api, slot 2 = web.
    app.state.label_slots = vec!["api".to_string(), "web".to_string()];
    app.state.active_label_filter = Some("api".to_string());

    let backend = ratatui::backend::TestBackend::new(80, 1);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_quick_label_bar(f, Rect::new(0, 0, 80, 1), &app)).unwrap();

    let buffer = terminal.backend().buffer();
    let text: String = (0..80).map(|x| buffer[(x, 0)].symbol()).collect();
    let trimmed = text.trim();
    // Slots keep first-view order: api before web.
    assert!(trimmed.contains("1 api"), "Buffer contents: {}", trimmed);
    assert!(trimmed.contains("2 web"), "Buffer contents: {}", trimmed);
    assert!(text.find("1 api").unwrap() < text.find("2 web").unwrap());

    // The active label's chip is the highlighted (reversed) one.
    let api_x = text.find("api").unwrap() as u16;
    let web_x = text.find("web").unwrap() as u16;
    assert!(buffer[(api_x, 0)].style().add_modifier.contains(Modifier::REVERSED));
    assert!(!buffer[(web_x, 0)].style().add_modifier.contains(Modifier::REVERSED));

    // The parts helper describes exactly what was drawn (click math relies on it).
    let parts = quick_label_parts(&app);
    assert_eq!(
        parts,
        vec![(" 1 ".to_string(), "api ".to_string()), (" 2 ".to_string(), "web ".to_string())]
    );

    // With no viewed labels the header layout has no chip row; with some it does.
    // Either way the closing rule (carrying the sort caption) sits right above the body.
    let area = Rect::new(0, 0, 80, 20);
    let header = home_header_layout(area, &app);
    // Row 1 is the rule between the summary bar and the chips.
    assert_eq!(header.chips, Some(Rect::new(0, 2, 80, 1)));
    assert_eq!(header.sort_rule, Rect::new(0, 3, 80, 1));
    assert_eq!(header.body.y, 4);
    app.state.label_slots.clear();
    let header = home_header_layout(area, &app);
    assert_eq!(header.chips, None);
    assert_eq!(header.sort_rule, Rect::new(0, 1, 80, 1));
    assert_eq!(header.body.y, 2);

    // The sort caption is set into the closing rule, not the outer frame.
    app.config.sort_by = SortOrder::Alphabetical;
    app.config.sort_reverse = true;
    let backend = ratatui::backend::TestBackend::new(80, 1);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal.draw(|f| draw_sort_rule(f, Rect::new(0, 0, 80, 1), &app)).unwrap();
    let buffer = terminal.backend().buffer();
    let rule: String = (0..80).map(|x| buffer[(x, 0)].symbol()).collect();
    assert!(rule.contains("Sort: Alphabetical (Rev)"), "rule: {}", rule);
    let dash = app.sym("rule");
    assert!(rule.starts_with(&format!("{dash}{dash}")), "rule: {}", rule);
    assert!(buffer[(0, 0)].style().add_modifier.contains(Modifier::DIM));

    // With the separators setting off the sort row keeps its caption but
    // loses the dashes, and the rule row between summary and chips goes.
    app.config.show_separators = false;
    terminal.draw(|f| draw_sort_rule(f, Rect::new(0, 0, 80, 1), &app)).unwrap();
    let buffer = terminal.backend().buffer();
    let rule: String = (0..80).map(|x| buffer[(x, 0)].symbol()).collect();
    assert!(rule.contains("Sort: Alphabetical (Rev)"), "rule: {}", rule);
    assert!(!rule.contains(dash), "rule: {}", rule);
    app.state.label_slots.push("api".to_string());
    let header = home_header_layout(area, &app);
    assert_eq!(header.chips, Some(Rect::new(0, 1, 80, 1)));
    assert_eq!(header.sort_rule, Rect::new(0, 2, 80, 1));
    assert_eq!(header.body.y, 3);
}

#[test]
fn test_draw_global_summary_bar() {
    let config = Config {
        items: vec!["/path/to/repo_a".to_string(), "/path/to/repo_b".to_string()],
        ..Default::default()
    };
    let (config_path, _guard) = crate::test_support::temp_config_path("draw_global_summary_bar");
    let mut app = App::new(config, config_path);

    let summary_clean = RepoSummary {
        branch: Some("main".to_string()),
        staged: 0,
        modified: 0,
        untracked: 0,
        conflicted: 0,
        ahead: 0,
        behind: 0,
        state: RepoState::Clean,
        last_commit_time: Some(
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
                as i64,
        ),
    };

    let summary_dirty = RepoSummary {
        branch: Some("main".to_string()),
        staged: 1,
        modified: 0,
        untracked: 0,
        conflicted: 0,
        ahead: 2,
        behind: 0,
        state: RepoState::Clean,
        last_commit_time: Some(0), // very stale
    };

    app.statuses =
        vec![ItemStatus::GitRepo(Some(summary_clean)), ItemStatus::GitRepo(Some(summary_dirty))];

    let backend = ratatui::backend::TestBackend::new(80, 1);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            draw_global_summary_bar(f, Rect::new(0, 0, 80, 1), &app);
        })
        .unwrap();

    // Verify correct rendering of stats in the buffer
    let buffer = terminal.backend().buffer();
    let text: String = (0..80).map(|x| buffer[(x, 0)].symbol()).collect();
    let trimmed = text.trim();
    assert!(trimmed.contains("2 repos"), "Buffer contents: {}", trimmed);
    assert!(trimmed.contains("1 dirty"), "Buffer contents: {}", trimmed);
    assert!(trimmed.contains("1 ahead"), "Buffer contents: {}", trimmed);
    assert!(trimmed.contains("1 stale"), "Buffer contents: {}", trimmed);
    assert!(trimmed.contains("0 archived"), "Buffer contents: {}", trimmed);
    assert!(trimmed.contains(SUMMARY_TAB_DIVIDER), "Buffer contents: {}", trimmed);

    // With no filter, the repos tab is the active (reversed) one.
    let repos_x = text.find("repos").unwrap();
    assert!(
        buffer[(repos_x as u16, 0)].style().add_modifier.contains(Modifier::REVERSED),
        "repos tab should render as the active tab"
    );

    // Activating a filter moves the reversed highlight to that tab.
    app.global_filter = Some(crate::app::GlobalFilter::Dirty);
    terminal
        .draw(|f| {
            draw_global_summary_bar(f, Rect::new(0, 0, 80, 1), &app);
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    let text: String = (0..80).map(|x| buffer[(x, 0)].symbol()).collect();
    let dirty_x = text.find("dirty").unwrap();
    let repos_x = text.find("repos").unwrap();
    assert!(
        buffer[(dirty_x as u16, 0)].style().add_modifier.contains(Modifier::REVERSED),
        "dirty tab should render as the active tab when the Dirty filter is on"
    );
    assert!(
        !buffer[(repos_x as u16, 0)].style().add_modifier.contains(Modifier::REVERSED),
        "repos tab should render inactive when the Dirty filter is on"
    );
}

#[test]
fn test_filtered_empty_view_keeps_summary_bar() {
    let config = Config { items: vec!["/path/to/repo_a".to_string()], ..Default::default() };
    let (config_path, _guard) =
        crate::test_support::temp_config_path("filtered_empty_view_keeps_summary_bar");
    let mut app = App::new(config, config_path);

    let clean = RepoSummary {
        branch: Some("main".to_string()),
        staged: 0,
        modified: 0,
        untracked: 0,
        conflicted: 0,
        ahead: 0,
        behind: 0,
        state: RepoState::Clean,
        last_commit_time: Some(
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
                as i64,
        ),
    };
    app.statuses = vec![ItemStatus::GitRepo(Some(clean))];

    // The dirty tab matches nothing.
    app.global_filter = Some(crate::app::GlobalFilter::Dirty);
    assert_eq!(app.get_items_len(), 0);

    let backend = ratatui::backend::TestBackend::new(80, 20);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    let mut detail_areas = crate::ui_detail::DetailAreas::default();
    let mut main_areas = Vec::new();
    let mut global_summary_area = None;

    let mut render = |app: &App, summary_area: &mut Option<Rect>| {
        terminal
            .draw(|f| {
                let size = f.area();
                // Same geometry as the run loop: the frame takes the outer
                // rect and content draws inside its one-cell border.
                let inner = size.inner(ratatui::layout::Margin { vertical: 1, horizontal: 1 });
                super::draw(
                    f,
                    app,
                    size,
                    inner,
                    1,
                    &mut detail_areas,
                    &mut main_areas,
                    summary_area,
                    &mut None,
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer().clone();
        let mut text = String::new();
        for y in 0..20 {
            for x in 0..80 {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        (text, buffer)
    };

    let (text, _) = render(&app, &mut global_summary_area);
    // The summary tab bar stays rendered and clickable, the onboarding
    // screen does not appear, and the message names the empty tab.
    assert!(global_summary_area.is_some());
    assert!(text.contains("dirty"), "summary bar missing:\n{}", text);
    assert!(!text.contains("Gitwig Onboarding"), "onboarding shown:\n{}", text);
    assert!(text.contains("No repositories in the dirty tab"), "message missing:\n{}", text);

    // The active ahead tab stays visible at zero count: reversed, in a
    // theme color rather than the old hardcoded gray.
    app.global_filter = Some(crate::app::GlobalFilter::Ahead);
    let (text, buffer) = render(&app, &mut global_summary_area);
    assert!(text.contains("No repositories in the ahead tab"), "message missing:\n{}", text);
    // Locate the tab by scanning buffer cells (a byte-offset str::find would
    // drift past the multi-byte border and divider glyphs on this row).
    let bar_y = global_summary_area.unwrap().y;
    let ahead_col = (0..76u16)
        .find(|&x| (0..5).map(|i| buffer[(x + i, bar_y)].symbol()).collect::<String>() == "ahead")
        .expect("ahead tab not rendered in summary bar");
    let ahead_cell_style = buffer[(ahead_col, bar_y)].style();
    assert!(ahead_cell_style.add_modifier.contains(Modifier::REVERSED));
    assert_eq!(ahead_cell_style.fg, Some(SUCCESS()));

    // An empty label filter names the label instead, and the frame's top
    // border carries the label badge so the active project stays visible.
    app.global_filter = None;
    app.state.active_label_filter = Some("ghost".to_string());
    assert_eq!(app.get_items_len(), 0);
    let (text, buffer) = render(&app, &mut global_summary_area);
    assert!(global_summary_area.is_some());
    assert!(!text.contains("Gitwig Onboarding"), "onboarding shown:\n{}", text);
    assert!(text.contains("No repositories with label 'ghost'"), "message missing:\n{}", text);
    let bar_y = global_summary_area.unwrap().y;
    let bar_row: String = (0..80).map(|x| buffer[(x, bar_y)].symbol()).collect();
    assert!(!bar_row.contains("ghost"), "badge should not sit in the summary bar:\n{}", bar_row);
    let top_row: String = (0..80).map(|x| buffer[(x, 0)].symbol()).collect();
    assert!(top_row.contains("ghost"), "label badge missing from frame border:\n{}", top_row);
    let badge = app.label_badge_area.get().expect("badge rect recorded");
    assert_eq!(badge.y, 0);
    assert!(
        badge.x > 0 && badge.x + badge.width < 80,
        "badge {:?} should sit inside the border",
        badge
    );
    assert!(buffer[(badge.x, 0)].style().add_modifier.contains(Modifier::REVERSED));
    // Frame drawn without the home header (e.g. no repositories): no badge rect.
    app.config.items.clear();
    app.statuses.clear();
    let _ = render(&app, &mut global_summary_area);
    assert!(global_summary_area.is_none());
    assert!(app.label_badge_area.get().is_none());
}

#[test]
fn test_compact_branch_col_width_adapts() {
    let config = Config {
        items: vec!["/path/to/repo_a".to_string(), "/path/to/repo_b".to_string()],
        ..Default::default()
    };
    let (config_path, _guard) =
        crate::test_support::temp_config_path("compact_branch_col_width_adapts");
    let mut app = App::new(config, config_path);

    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        as i64;
    let summary = |branch: &str| {
        ItemStatus::GitRepo(Some(RepoSummary {
            branch: Some(branch.to_string()),
            staged: 0,
            modified: 0,
            untracked: 0,
            conflicted: 0,
            ahead: 0,
            behind: 0,
            state: RepoState::Clean,
            last_commit_time: Some(now),
        }))
    };

    // Short branches keep the legacy 28-column width.
    app.statuses = vec![summary("main"), summary("develop")];
    assert_eq!(compact_branch_col_width(&app, 200), 28);

    // A long branch name widens the column when the terminal has room.
    let long_branch = "feature/very-long-descriptive-branch-name";
    app.statuses = vec![summary("main"), summary(long_branch)];
    let width = compact_branch_col_width(&app, 200);
    assert!(
        width as usize > 2 + long_branch.chars().count(),
        "column ({}) should fit the branch plus its symbol",
        width
    );

    // On a narrow terminal the cap protects the repository column, and the
    // width never drops below the legacy minimum.
    assert_eq!(compact_branch_col_width(&app, 80), 28);
    assert_eq!(compact_branch_col_width(&app, 0), 28);
}

/// The PR line-comment wizard's modes were missing from the detail mode set,
/// so pressing `n` drew the home list and never the wizard popup.
#[test]
fn test_forge_comment_wizard_draws_its_popup_over_the_detail_view() {
    let config = Config { items: vec!["/path/to/repo_a".to_string()], ..Default::default() };
    let (config_path, _guard) = crate::test_support::temp_config_path(
        "forge_comment_wizard_draws_its_popup_over_the_detail_view",
    );
    let mut app = App::new(config, config_path);
    app.current_detail = Some(ItemDetail::Repo {
        resolved: PathBuf::from("/path/to/repo_a"),
        info: Box::new(RepoInfo::default()),
    });
    app.advanced_tabs = true;
    app.detail_tab = 11;
    let backend = ratatui::backend::TestBackend::new(120, 40);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    for (mode, step) in [
        (Mode::ForgeCommentPathInput, "Step 1"),
        (Mode::ForgeCommentLineInput, "Step 2"),
        (Mode::ForgeCommentBodyInput, "Step 3"),
    ] {
        app.mode = mode;
        let mut detail_areas = crate::ui_detail::DetailAreas::default();
        terminal
            .draw(|f| {
                let size = f.area();
                super::draw(
                    f,
                    &app,
                    size,
                    size,
                    1,
                    &mut detail_areas,
                    &mut Vec::new(),
                    &mut None,
                    &mut None,
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let text: String = (0..40)
            .map(|y| (0..120).map(|x| buffer[(x, y)].symbol()).collect::<String>() + "\n")
            .collect();
        assert!(text.contains("Add PR Line Comment"), "{:?}: no popup in\n{}", mode, text);
        assert!(text.contains(step), "{:?}: {} missing in\n{}", mode, step, text);
        assert!(text.contains("PRs [5]"), "{:?}: detail view missing in\n{}", mode, text);
    }
}

#[test]
fn test_draw_partial_uncommitted_badge() {
    let config = Config { items: vec!["/path/to/repo_a".to_string()], ..Default::default() };
    let (config_path, _guard) =
        crate::test_support::temp_config_path("draw_partial_uncommitted_badge");
    let mut app = App::new(config, config_path);

    // Has both staged and unstaged (modified) changes -> should show PARTIAL badge
    let summary_partial = RepoSummary {
        branch: Some("main".to_string()),
        staged: 1,
        modified: 1,
        untracked: 0,
        conflicted: 0,
        ahead: 0,
        behind: 0,
        state: RepoState::Clean,
        last_commit_time: None,
    };

    app.statuses = vec![ItemStatus::GitRepo(Some(summary_partial))];

    let backend = ratatui::backend::TestBackend::new(80, 5);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let chunks = vec![Rect::new(0, 0, 80, 4)];
            draw_items(f, &app, &chunks);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let mut found_partial = false;
    for y in 0..5 {
        let row_text: String = (0..80).map(|x| buffer[(x, y)].symbol()).collect();
        if row_text.contains("PARTIAL") {
            found_partial = true;
            break;
        }
    }
    assert!(found_partial, "Expected to find PARTIAL badge in card rendering");
}

#[test]
fn test_actual_rendering_all_modes_and_tabs() {
    // Create a unique temporary directory
    let uuid =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let temp_repo_path = std::env::temp_dir().join(format!("gitwig_test_render_repo_{}", uuid));
    let _ = std::fs::create_dir_all(&temp_repo_path);

    // Initialize Git repo
    let repo = git2::Repository::init(&temp_repo_path).unwrap();

    // Configure user
    let mut config_git = repo.config().unwrap();
    config_git.set_str("user.name", "Test User").unwrap();
    config_git.set_str("user.email", "test@test.com").unwrap();

    // Create committed file
    let file1_path = temp_repo_path.join("file1.rs");
    std::fs::write(&file1_path, "fn main() {\n    // initial version\n}\n").unwrap();

    // Stage and commit
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("file1.rs")).unwrap();
    index.write().unwrap();

    let oid = index.write_tree().unwrap();
    let tree = repo.find_tree(oid).unwrap();
    let signature = repo.signature().unwrap();
    repo.commit(Some("HEAD"), &signature, &signature, "Initial commit", &tree, &[]).unwrap();

    // Modify file1.rs to create unstaged changes
    std::fs::write(&file1_path, "fn main() {\n    println!(\"Modified\");\n}\n").unwrap();

    // Create untracked file
    let file2_path = temp_repo_path.join("untracked.rs");
    std::fs::write(&file2_path, "// untracked file content\n").unwrap();

    let repo_path_str = temp_repo_path.to_str().unwrap().to_string();
    let detail = crate::repo::inspect_detail(&repo_path_str, 100, 1000, false);

    let mut mock_info = match detail {
        crate::repo::ItemDetail::Repo { info, .. } => *info,
        _ => crate::repo::RepoInfo::default(),
    };

    // Enrich mock_info with mock stashes, worktrees, submodules, etc. so they render nicely too
    mock_info.stashes = crate::repo::TabData::Loaded(vec![crate::repo::StashInfo {
        index: 0,
        message: "stash msg".to_string(),
        commit_id: "abc1234".to_string(),
        files: vec![],
    }]);
    mock_info.worktrees = crate::repo::TabData::Loaded(vec![crate::repo::WorktreeInfo {
        name: "wt".to_string(),
        path: temp_repo_path.join("wt"),
        branch: Some("feature".to_string()),
        is_locked: false,
        lock_reason: None,
    }]);
    mock_info.submodules = crate::repo::TabData::Loaded(vec![crate::repo::SubmoduleInfo {
        name: "sub".to_string(),
        path: temp_repo_path.join("sub"),
        url: "http://url".to_string(),
        head_id: Some("abc1234".to_string()),
        ..Default::default()
    }]);
    mock_info.reflog = crate::repo::TabData::Loaded(vec![crate::repo::ReflogEntry {
        index: 0,
        target_oid: "abc1234".to_string(),
        selector: "reflog@{0}".to_string(),
        command: "commit".to_string(),
        message: "feat: summary".to_string(),
        when: "10 mins ago".to_string(),
        date: "2026-07-02".to_string(),
    }]);
    mock_info.committer_stats = crate::repo::TabData::Loaded(vec![crate::repo::CommitterStat {
        name: "Author".to_string(),
        email: "author@author.com".to_string(),
        count: 1,
    }]);

    // Add staged/unstaged/conflicted/untracked file changes
    mock_info.changes.staged =
        vec![crate::repo::FileEntry { path: "staged_file.rs".to_string(), label: "A" }];
    mock_info.changes.unstaged =
        vec![crate::repo::FileEntry { path: "unstaged_file.rs".to_string(), label: "M" }];
    mock_info.changes.conflicted =
        vec![crate::repo::FileEntry { path: "conflicted_file.rs".to_string(), label: "U" }];
    mock_info.changes.untracked =
        vec![crate::repo::FileEntry { path: "untracked_file.rs".to_string(), label: "?" }];

    // Add local/remote branches, tags, and remotes
    mock_info.local_branches = crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
        name: "main".to_string(),
        is_head: true,
        short_sha: "abc1234".to_string(),
        short_message: "feat: commit 1".to_string(),
    }]);
    mock_info.remote_branches = crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
        name: "origin/main".to_string(),
        is_head: false,
        short_sha: "abc1234".to_string(),
        short_message: "feat: commit 1".to_string(),
    }]);
    mock_info.local_tags = crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
        name: "v1.0.0".to_string(),
        is_head: false,
        short_sha: "abc1234".to_string(),
        short_message: "feat: commit 1".to_string(),
    }]);
    mock_info.remote_tags = crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
        name: "v1.0.0".to_string(),
        is_head: false,
        short_sha: "abc1234".to_string(),
        short_message: "feat: commit 1".to_string(),
    }]);
    mock_info.remotes = crate::repo::TabData::Loaded(vec![crate::repo::RemoteInfo {
        name: "origin".to_string(),
        url: "http://url".to_string(),
        push_url: Some("http://url".to_string()),
        refspecs: vec![],
    }]);

    // Add rich commits & graph commits
    mock_info.commits = vec![
        crate::repo::CommitEntry {
            id: "abc1234".to_string(),
            oid: "abc1234abc1234abc1234abc1234abc1234abc1234".to_string(),
            author: "Author 1".to_string(),
            when: "10 mins ago".to_string(),
            date: "2026-07-02".to_string(),
            summary: "feat: commit 1".to_string(),
            message: "feat: commit 1\n\nbody".to_string(),
            refs: vec!["HEAD".to_string(), "main".to_string()],
            files: vec![],
            signature_status: "G".to_string(),
        },
        crate::repo::CommitEntry {
            id: "def5678".to_string(),
            oid: "def5678def5678def5678def5678def5678def5678".to_string(),
            author: "Author 2".to_string(),
            when: "20 mins ago".to_string(),
            date: "2026-07-02".to_string(),
            summary: "feat: commit 2".to_string(),
            message: "feat: commit 2\n\nbody".to_string(),
            refs: vec![],
            files: vec![],
            signature_status: "N".to_string(),
        },
    ];
    mock_info.graph_lines = crate::repo::TabData::Loaded(vec![
        crate::repo::GraphLine {
            graph: "*".to_string(),
            commit: Some(crate::repo::GraphCommit {
                oid: "abc1234".to_string(),
                decoration: " (HEAD -> main)".to_string(),
                summary: "feat: commit 1".to_string(),
                author: "Author 1".to_string(),
                date: "2026-07-02".to_string(),
                signature_status: "G".to_string(),
            }),
        },
        crate::repo::GraphLine {
            graph: "|*".to_string(),
            commit: Some(crate::repo::GraphCommit {
                oid: "def5678".to_string(),
                decoration: "".to_string(),
                summary: "feat: commit 2".to_string(),
                author: "Author 2".to_string(),
                date: "2026-07-02".to_string(),
                signature_status: "N".to_string(),
            }),
        },
    ]);

    let config = Config { items: vec![repo_path_str], ..Default::default() };
    let mut app = App::new(config, temp_repo_path.join("dummy_path.toml"));

    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: temp_repo_path.clone(),
        info: Box::new(mock_info.clone()),
    });

    app.file_tree.visible_files = vec![crate::app::FileTreeItem {
        name: "file1.rs".to_string(),
        full_path: "file1.rs".to_string(),
        is_dir: false,
        depth: 0,
        is_expanded: false,
    }];

    // Populate action/confirm targets
    app.branch_action_target = Some(("feature".to_string(), false));
    app.tag_delete_target = Some(("v1.0.0".to_string(), false));
    app.tag_push_target = Some("v1.0.0".to_string());
    app.discard_target = Some(("staged_file.rs".to_string(), false));
    app.cherry_pick_target = Some(("abc1234".to_string(), "feat: commit".to_string()));
    app.revert_target = Some(("abc1234".to_string(), "feat: commit".to_string()));
    app.stash_action_target = Some(("stash@{0}".to_string(), "wip".to_string()));
    app.remote_action_target = Some("origin".to_string());
    app.submodule_delete_target = Some(crate::repo::SubmoduleInfo {
        name: "sub".to_string(),
        path: std::path::PathBuf::from("sub"),
        ..Default::default()
    });
    app.submodule_update_target = Some(crate::app::SubmoduleUpdateTarget {
        path: None,
        modified: vec![std::path::PathBuf::from("sub")],
    });
    app.forge_checkout_target = Some(crate::app::ForgeCheckoutTarget::PullRequest {
        number: 42,
        title: "Add retry".to_string(),
    });

    let backend = ratatui::backend::TestBackend::new(120, 40);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    let mut detail_areas = crate::ui_detail::DetailAreas::default();
    let mut main_areas = Vec::new();
    let mut global_summary_area = None;

    // 1. Loop through all detail tabs
    app.mode = Mode::Detail;
    for tab in 0..10 {
        app.detail_tab = tab;
        terminal
            .draw(|f| {
                let size = f.area();
                super::draw(
                    f,
                    &app,
                    size,
                    size,
                    1,
                    &mut detail_areas,
                    &mut main_areas,
                    &mut global_summary_area,
                    &mut None,
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        assert!(buffer.area.width > 0);
    }

    // 1b. Render detail tabs in other TabData states (NotLoaded, Loading, Error)
    let tab_data_states: Vec<crate::repo::TabData<()>> = vec![
        crate::repo::TabData::NotLoaded,
        crate::repo::TabData::Loading,
        crate::repo::TabData::Error("Mock Error Msg".to_string()),
    ];
    for state in &tab_data_states {
        macro_rules! map_state {
            ($s:expr) => {
                match $s {
                    crate::repo::TabData::NotLoaded => crate::repo::TabData::NotLoaded,
                    crate::repo::TabData::Loading => crate::repo::TabData::Loading,
                    crate::repo::TabData::Error(e) => crate::repo::TabData::Error(e.clone()),
                    _ => crate::repo::TabData::NotLoaded,
                }
            };
        }
        mock_info.remotes = map_state!(state);
        mock_info.graph_lines = map_state!(state);
        mock_info.local_branches = map_state!(state);
        mock_info.remote_branches = map_state!(state);
        mock_info.local_tags = map_state!(state);
        mock_info.remote_tags = map_state!(state);
        mock_info.files = map_state!(state);
        mock_info.stashes = map_state!(state);
        mock_info.worktrees = map_state!(state);
        mock_info.submodules = map_state!(state);
        mock_info.reflog = map_state!(state);
        mock_info.committer_stats = map_state!(state);

        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: temp_repo_path.clone(),
            info: Box::new(mock_info.clone()),
        });

        for tab in 0..10 {
            app.detail_tab = tab;
            terminal
                .draw(|f| {
                    let size = f.area();
                    super::draw(
                        f,
                        &app,
                        size,
                        size,
                        1,
                        &mut detail_areas,
                        &mut main_areas,
                        &mut global_summary_area,
                        &mut None,
                    );
                })
                .unwrap();
        }
    }

    // 2. Render all modes
    let other_modes = vec![
        Mode::Normal,
        Mode::Editing,
        Mode::Overview,
        Mode::Bisect,
        Mode::RepoSettings,
        Mode::RepoJump,
        Mode::LabelPicker,
        Mode::LabelSettings,
        Mode::RepoScanPicker,
        Mode::BulkAddScanPicker,
        Mode::BranchSearchInput,
        Mode::FileSearchInput,
        Mode::CommitFuzzySearch,
        Mode::TagSearchInput,
        Mode::FileHistory,
        Mode::Settings,
        Mode::DebugLogs,
        Mode::Help,
        Mode::About,
        Mode::Legend,
        Mode::ConfirmDelete,
        Mode::ConfirmArchiveLabel,
        Mode::DetailHelp,
        Mode::CommitInput,
        Mode::BranchCreateInput,
        Mode::TagCreateInput,
        Mode::BranchDeleteConfirm,
        Mode::BranchPushConfirm,
        Mode::TagDeleteConfirm,
        Mode::TagOverwriteConfirm,
        Mode::TagPushConfirm,
        Mode::TagPushAllConfirm,
        Mode::StashDeleteConfirm,
        Mode::StashApplyConfirm,
        Mode::StashCreateInput,
        Mode::StashingUI,
        Mode::RemotePicker,
        Mode::BranchMergeConfirm,
        Mode::BranchMergeIntoConfirm,
        Mode::BranchRebaseConfirm,
        Mode::BranchInteractiveRebaseConfirm,
        Mode::DiscardChangesConfirm,
        Mode::Inspect,
        Mode::ImportUrlInput,
        Mode::ImportDestInput,
        Mode::ImportNameInput,
        Mode::BulkAddInput,
        Mode::SearchColumnPicker,
        Mode::RemoteAddNameInput,
        Mode::RemoteAddUrlInput,
        Mode::RemoteDeleteConfirm,
        Mode::Logs,
        Mode::LogsSearchInput,
        Mode::BranchCheckoutConfirm,
        Mode::TagCheckoutConfirm,
        Mode::CommitCheckoutConfirm,
        Mode::RepoSearchInput,
        Mode::MergeAbortConfirm,
        Mode::MergeContinueConfirm,
        Mode::CherryPickConfirm,
        Mode::RevertConfirm,
        Mode::LabelInput,
        Mode::AddRepoLabelInput,
        Mode::BulkAddRepoLabelInput,
        Mode::CloneRepoLabelInput,
        Mode::UpdateConfirm,
        Mode::WorktreeAddBranchInput,
        Mode::WorktreeAddPathInput,
        Mode::WorktreeLockReasonInput,
        Mode::WorktreeRemoveConfirm,
        Mode::SubmoduleAddUrlInput,
        Mode::SubmoduleAddPathInput,
        Mode::SubmoduleDeleteConfirm,
        Mode::SubmoduleUpdateConfirm,
        Mode::ForgeCheckoutConfirm,
        Mode::NotGitRepo,
    ];

    for mode in &other_modes {
        app.mode = *mode;
        terminal
            .draw(|f| {
                let size = f.area();
                super::draw(
                    f,
                    &app,
                    size,
                    size,
                    1,
                    &mut detail_areas,
                    &mut main_areas,
                    &mut global_summary_area,
                    &mut None,
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        assert!(buffer.area.width > 0);
    }

    // Draw loading popup
    app.fetching = true;
    terminal
        .draw(|f| {
            let size = f.area();
            super::draw(
                f,
                &app,
                size,
                size,
                1,
                &mut detail_areas,
                &mut main_areas,
                &mut global_summary_area,
                &mut None,
            );
        })
        .unwrap();
    app.fetching = false;

    // Draw error popup
    app.error_message = Some("An error occurred".to_string());
    terminal
        .draw(|f| {
            let size = f.area();
            super::draw(
                f,
                &app,
                size,
                size,
                1,
                &mut detail_areas,
                &mut main_areas,
                &mut global_summary_area,
                &mut None,
            );
        })
        .unwrap();
    app.error_message = None;

    // 3. Dispatch mock events to handle_key for all modes and detail focuses
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let dummy_keys = vec![
        KeyEvent::new(KeyCode::Up, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Down, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('F'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('o'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()),
        KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()),
        KeyEvent::new(KeyCode::BackTab, KeyModifiers::empty()),
    ];

    // Loop through tab focuses for Detail mode
    app.mode = Mode::Detail;
    for tab in 0..10 {
        app.detail_tab = tab;
        let focuses = vec![
            DetailSection::Staged,
            DetailSection::Unstaged,
            DetailSection::Conflicts,
            DetailSection::CommitDetails,
            DetailSection::StagingDetails,
            DetailSection::ConflictDiff,
            DetailSection::Commits,
            DetailSection::FileContent,
            DetailSection::LocalBranches,
            DetailSection::RemoteBranches,
            DetailSection::LocalTags,
            DetailSection::RemoteTags,
            DetailSection::Remotes,
            DetailSection::Stashes,
            DetailSection::StashedFiles,
            DetailSection::Worktrees,
            DetailSection::Submodules,
            DetailSection::Reflog,
        ];
        for focus in focuses {
            app.detail_focus = focus;
            for key in &dummy_keys {
                let _ = crate::input::handle_key(&mut app, *key, 1);
            }
        }
    }

    // Other modes event dispatch
    for mode in &other_modes {
        app.mode = *mode;
        for key in &dummy_keys {
            let _ = crate::input::handle_key(&mut app, *key, 1);
        }
    }
    let _ = std::fs::remove_dir_all(&temp_repo_path);
}

#[test]
fn test_command_palette_draws_over_home_and_detail_frames() {
    let config = Config { items: vec!["/path/to/repo".to_string()], ..Default::default() };
    let (config_path, _guard) =
        crate::test_support::temp_config_path("command_palette_draws_over_home_and_detail_frames");
    let mut app = App::new(config, config_path);
    let backend = ratatui::backend::TestBackend::new(120, 40);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    let mut detail_areas = crate::ui_detail::DetailAreas::default();
    let mut main_areas = Vec::new();
    let mut global_summary_area = None;

    let mut render = |app: &App| -> String {
        terminal
            .draw(|f| {
                let size = f.area();
                super::draw(
                    f,
                    app,
                    size,
                    size,
                    1,
                    &mut detail_areas,
                    &mut main_areas,
                    &mut global_summary_area,
                    &mut None,
                );
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..40)
            .map(|y| (0..120).map(|x| buffer[(x, y)].symbol().to_string()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    };

    for mode in [Mode::Normal, Mode::Detail] {
        app.mode = mode;
        app.close_command_palette();
        assert!(!render(&app).contains("Command Palette"), "closed palette drawn in {:?}", mode);
        app.open_command_palette();
        let frame = render(&app);
        assert!(frame.contains("Command Palette"), "{:?} frame:\n{}", mode, frame);
        assert!(frame.contains("Search Actions"), "{:?} frame:\n{}", mode, frame);
        // The status bar underneath switches to the palette hints.
        assert!(frame.contains("Command palette"), "{:?} frame:\n{}", mode, frame);
    }

    // An error popup still outranks the palette.
    app.error_message = Some("Palette test error".to_string());
    let frame = render(&app);
    assert!(frame.contains("Palette test error"));
}

#[test]
fn test_repo_theme_persists_in_detail_search_popups() {
    let (config_path, _guard) =
        crate::test_support::temp_config_path("repo_theme_detail_search_popups");
    let repo_path =
        config_path.parent().unwrap().join("themed_repo").to_string_lossy().into_owned();
    let config = Config { items: vec![repo_path.clone()], ..Default::default() };
    let mut app = App::new(config, config_path);
    app.repo_theme_cache.insert(
        repo_path.clone(),
        crate::config::ThemeConfig { accent: "#ab12cd".to_string(), ..Default::default() },
    );
    app.current_detail = Some(crate::repo::ItemDetail::Repo {
        resolved: std::path::PathBuf::from(&repo_path),
        info: Box::new(crate::repo::RepoInfo::default()),
    });

    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 40)).unwrap();
    let mut detail_areas = crate::ui_detail::DetailAreas::default();
    let mut main_areas = Vec::new();
    let mut global_summary_area = None;

    for mode in [Mode::BranchSearchInput, Mode::CommitHistoryPicker] {
        app.mode = mode;
        terminal
            .draw(|f| {
                let size = f.area();
                super::draw(
                    f,
                    &app,
                    size,
                    size,
                    1,
                    &mut detail_areas,
                    &mut main_areas,
                    &mut global_summary_area,
                    &mut None,
                );
            })
            .unwrap();

        let has_repo_accent = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .any(|cell| cell.fg == ratatui::style::Color::Rgb(0xab, 0x12, 0xcd));
        assert!(has_repo_accent, "repo accent missing during {:?}", mode);
        assert_ne!(
            crate::ui::style::ACCENT(),
            ratatui::style::Color::Rgb(0xab, 0x12, 0xcd),
            "ThemeGuard should restore theme after draw in {:?}",
            mode
        );
    }
}

#[test]
fn test_bisect_banner_and_popup_rendering() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let (config_path, _guard) = crate::test_support::temp_config_path("bisect_banner_popup_draw");
    let repo_path = PathBuf::from("/nonexistent/gitwig_bisect_draw");
    let mut app = App::new(
        Config { items: vec![repo_path.to_string_lossy().into_owned()], ..Default::default() },
        config_path,
    );

    let info = RepoInfo {
        branch: Some("HEAD".to_string()),
        bisect: crate::repo::BisectStatus {
            active: true,
            start_ref: Some("main".to_string()),
            current_oid: Some("1111111111111111111111111111111111111111".to_string()),
            current_short_oid: Some("1111111".to_string()),
            current_summary: Some("midpoint commit".to_string()),
            bad_oid: Some("2222222222222222222222222222222222222222".to_string()),
            good_oids: vec!["3333333333333333333333333333333333333333".to_string()],
            skip_oids: Vec::new(),
            remaining_revisions: Some(2),
            remaining_steps: Some(1),
            culprit: None,
            log: vec![crate::repo::BisectLogEntry {
                oid: "2222222222222222222222222222222222222222".to_string(),
                short_oid: "2222222".to_string(),
                verdict: "bad".to_string(),
                summary: "broken commit".to_string(),
            }],
        },
        ..Default::default()
    };
    app.current_detail = Some(ItemDetail::Repo { resolved: repo_path, info: Box::new(info) });

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut detail_areas = crate::ui_detail::DetailAreas::default();
    let mut main_areas = Vec::new();
    let mut global_summary_area = None;

    let render_text = |terminal: &Terminal<TestBackend>| -> String {
        let buf = terminal.backend().buffer();
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width).map(|x| buf.cell((x, y)).unwrap().symbol()).collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Active Detail view renders the 1-line bisect banner.
    app.mode = Mode::Detail;
    terminal
        .draw(|f| {
            let size = f.area();
            super::draw(
                f,
                &app,
                size,
                size,
                1,
                &mut detail_areas,
                &mut main_areas,
                &mut global_summary_area,
                &mut None,
            );
        })
        .unwrap();
    let detail_out = render_text(&terminal);
    assert!(detail_out.contains("BISECT"));
    assert!(detail_out.contains("~2 revs (~1 steps)"));

    // Active Mode::Bisect renders the interactive popup with log entry.
    app.mode = Mode::Bisect;
    terminal
        .draw(|f| {
            let size = f.area();
            super::draw(
                f,
                &app,
                size,
                size,
                1,
                &mut detail_areas,
                &mut main_areas,
                &mut global_summary_area,
                &mut None,
            );
        })
        .unwrap();
    let popup_out = render_text(&terminal);
    assert!(popup_out.contains("Git Bisect"));
    assert!(popup_out.contains("[BAD]"));
    assert!(popup_out.contains("broken commit"));
}

#[test]
fn test_draw_archive_label_popup_and_empty_state() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let (cfg_path, _guard) = crate::test_support::temp_config_path("draw_archive_label");
    let items: Vec<String> = (1..=7).map(|i| format!("/tmp/gitwig_lbl_repo_{}", i)).collect();
    let mut labels = std::collections::HashMap::new();
    for p in &items {
        labels.insert(p.clone(), vec!["work".to_string()]);
    }
    let config = Config { items: items.clone(), labels, ..Default::default() };
    let mut app = App::new(config, cfg_path);
    app.label_archive_target = Some(crate::app::LabelArchiveTarget {
        label: "work".to_string(),
        repos: items.clone(),
        unarchive: false,
    });
    app.mode = Mode::ConfirmArchiveLabel;

    let backend = TestBackend::new(90, 28);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut detail_areas = crate::ui_detail::DetailAreas::default();
    let mut main_areas = Vec::new();
    let mut global_summary_area = None;

    let render_text = |terminal: &Terminal<TestBackend>| -> String {
        let buf = terminal.backend().buffer();
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width).map(|x| buf.cell((x, y)).unwrap().symbol()).collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    terminal
        .draw(|f| {
            let size = f.area();
            super::draw(
                f,
                &app,
                size,
                size,
                1,
                &mut detail_areas,
                &mut main_areas,
                &mut global_summary_area,
                &mut None,
            );
        })
        .unwrap();
    let popup_out = render_text(&terminal);
    assert!(popup_out.contains("Archive Label Repositories"), "Buffer:\n{}", popup_out);
    assert!(popup_out.contains("7 shown repositories"), "Buffer:\n{}", popup_out);
    assert!(popup_out.contains("work"), "Buffer:\n{}", popup_out);
    assert!(popup_out.contains("gitwig_lbl_repo_1"), "Buffer:\n{}", popup_out);
    assert!(popup_out.contains("gitwig_lbl_repo_5"), "Buffer:\n{}", popup_out);
    assert!(popup_out.contains("... and 2 more"), "Buffer:\n{}", popup_out);

    // When all repos for the active label are archived, empty state reports it cleanly.
    app.mode = Mode::Normal;
    app.label_archive_target = None;
    app.state.active_label_filter = Some("work".to_string());
    for p in items {
        app.config.archived.insert(p);
    }
    terminal
        .draw(|f| {
            let size = f.area();
            super::draw(
                f,
                &app,
                size,
                size,
                1,
                &mut detail_areas,
                &mut main_areas,
                &mut global_summary_area,
                &mut None,
            );
        })
        .unwrap();
    let empty_out = render_text(&terminal);
    assert!(
        empty_out.contains("All repositories with label 'work' are currently archived."),
        "Buffer:\n{}",
        empty_out
    );
}
