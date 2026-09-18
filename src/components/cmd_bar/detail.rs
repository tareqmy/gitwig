//! Detail pane view status bar entry generation.

use super::{StatusEntry, compact_action_keys};
use crate::app::{App, DetailSection, Mode};
use crate::keybindings::Action;
use crate::ui::style::{ACCENT, DANGER, WARNING, accent_style, muted_style, primary_style};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

pub(crate) fn detail_dismiss_entries(app: &App) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let mut message_spans = None;
    if let Some(msg) = &app.status_message {
        message_spans = Some(vec![Span::styled(format!("{} ", msg), accent_style())]);
    }

    // Every configurable action is rendered from the live keybindings so a
    // rebinding never leaves the status bar lying. Keys that stay literal
    // below are matched as literal key codes by their handlers (noted inline).
    let compat = app.config.compatibility_mode;
    let k = |action: Action| compact_action_keys(&app.keybindings, action, compat);

    let ws_checkout = k(Action::WorkspaceCheckout);
    let ws_tag = k(Action::WorkspaceCreateTag);
    let ws_branch = k(Action::WorkspaceCreateBranch);
    let ws_irebase = k(Action::WorkspaceInteractiveRebase);
    let ws_cherry_pick = k(Action::WorkspaceCherryPick);
    let ws_revert = k(Action::WorkspaceRevert);
    let ws_fuzzy = k(Action::WorkspaceFuzzySearch);
    let ws_columns = k(Action::WorkspaceColumnPicker);
    let ws_logs = k(Action::WorkspaceLogsView);
    let tags_checkout = k(Action::TagsCheckout);
    let tags_search = k(Action::TagsSearch);
    let tags_fetch = k(Action::TagsFetch);
    let tags_push = k(Action::TagsPush);
    let tags_push_all = k(Action::TagsPushAll);
    let tags_delete = k(Action::TagsDelete);
    let ws_load_more = k(Action::WorkspaceLoadMore);
    let ws_yank = k(Action::WorkspaceYankHash);
    let ws_stash = k(Action::WorkspaceStashUI);
    let ws_commit = k(Action::WorkspaceCommit);
    let ws_commit_amend = format!("{}/{}", ws_commit, k(Action::WorkspaceCommitAmend));
    let ws_stage = k(Action::WorkspaceStage);
    let ws_stage_all = k(Action::WorkspaceStageAll);
    let ws_discard = k(Action::WorkspaceDiscard);
    let ws_discard_all = k(Action::WorkspaceDiscardAll);
    // Inspect from the commits list is the HomeOpenDetail binding
    // (src/tabs/workspace.rs) plus a literal → in the commit list component.
    let ws_inspect = k(Action::HomeOpenDetail);
    let diff_line_mode = k(Action::DiffLineMode);
    let diff_stage = k(Action::DiffStage);
    let diff_unstage = k(Action::DiffUnstage);
    let conflict_ours = k(Action::ConflictOurs);
    let conflict_theirs = k(Action::ConflictTheirs);
    let conflict_resolve = k(Action::ConflictResolve);
    let conflict_mergetool = k(Action::ConflictMergeTool);
    let conflict_abort = k(Action::ConflictAbort);
    let conflict_continue = k(Action::ConflictContinue);
    // The file tree matches → / ↵ (toggle folder), ← (collapse all) and
    // x / X (discard) as literal key codes (src/components/file_tree.rs).
    let files_expand = format!("→/↵/{}", k(Action::FilesExpand));
    let files_collapse = k(Action::FilesCollapse);
    let files_search = k(Action::FilesSearch);
    let files_history = k(Action::FilesHistory);
    let files_line_numbers = k(Action::FilesLineNumbers);
    let files_blame = k(Action::FilesBlame);
    let files_editor = k(Action::FilesEditor);
    let files_full_screen = k(Action::FilesFullScreen);
    // Leaving the full-screen viewer is a literal ← plus the CloseDetail
    // binding (src/tabs/files.rs).
    let files_exit_full_screen = format!("←/{}", k(Action::CloseDetail));
    let branches_checkout = k(Action::BranchesCheckout);
    let branches_create = k(Action::BranchesCreate);
    let branches_delete = k(Action::BranchesDelete);
    let branches_merge = k(Action::BranchesMerge);
    let branches_merge_into = k(Action::BranchesMergeInto);
    let branches_rebase = k(Action::BranchesRebase);
    let branches_irebase = k(Action::BranchesInteractiveRebase);
    let branches_search = k(Action::BranchesSearch);
    let branches_pull = k(Action::BranchesPull);
    let branches_push = k(Action::BranchesPush);
    let remotes_fetch = k(Action::RemotesFetch);
    let remotes_add = k(Action::RemotesAdd);
    let remotes_delete = k(Action::RemotesDelete);
    let stashes_apply = k(Action::StashesApply);
    let stashes_delete = k(Action::StashesDelete);
    let stashes_create = k(Action::StashesCreate);
    let worktrees_add = k(Action::WorktreesAdd);
    let worktrees_delete = k(Action::WorktreesDelete);
    let worktrees_lock = k(Action::WorktreesLock);
    let worktrees_prune = k(Action::WorktreesPrune);
    let worktrees_open = k(Action::WorktreesOpen);
    let submodules_add = k(Action::SubmodulesAdd);
    let submodules_delete = k(Action::SubmodulesDelete);
    let reflog_checkout = k(Action::ReflogCheckout);
    let forge_checkout = k(Action::ForgeCheckout);
    let forge_open_browser = k(Action::ForgeOpenBrowser);
    let forge_toggle_assigned = k(Action::ForgeToggleAssigned);
    let forge_add_comment = k(Action::ForgeAddComment);

    // The "Tabs", "Home", "Cycle Focus", "Resize", "Resync" and "Help" keys
    // are supplied by the rewrite loop below, so their literal here is "".
    let entries_data: Vec<(&str, &str)> = match app.detail_tab {
        0 => {
            let mut v = vec![("Home", ""), ("Tabs", ""), ("Cycle Focus", ""), ("Resize", "")];
            if app.detail_focus == DetailSection::CommitDetails {
                v.push(("Scroll Info", "↑↓"));
                v.push(("Inspect", "→"));
            } else if app.detail_focus == DetailSection::Staged
                || app.detail_focus == DetailSection::Unstaged
            {
                v.push(("Navigate/Scroll", "↑↓"));
                v.push(("Page", "⇟/⇞"));
                v.push(("Jump", "Home/End"));
                if app.is_uncommitted_selected() {
                    v.push(("Stage/Unstage", &ws_stage));
                    if app.detail_focus == DetailSection::Unstaged {
                        v.push(("Stage All", &ws_stage_all));
                    } else if app.detail_focus == DetailSection::Staged {
                        v.push(("Unstage All", &ws_stage_all));
                    }
                    v.push(("Discard", &ws_discard));
                    v.push(("Discard All", &ws_discard_all));
                    v.push(("Stash", &ws_stash));
                }
                v.push(("Inspect", "→"));
            } else if app.detail_focus == DetailSection::StagingDetails {
                v.push(("Navigate/Scroll", "↑↓"));
                v.push(("Page", "⇟/⇞"));
                v.push(("Jump", "Home/End"));
                if app.is_uncommitted_selected() {
                    v.push(("Line Mode", &diff_line_mode));
                    v.push(("Stage/Unstage Hunk", &ws_stage));
                    v.push(("Stage", &diff_stage));
                    v.push(("Unstage", &diff_unstage));
                    v.push(("Discard", &ws_discard));
                    v.push(("Discard All", &ws_discard_all));
                }
                v.push(("Full Screen", &files_full_screen));
            } else if app.detail_focus == DetailSection::Conflicts {
                v.push(("Navigate/Scroll", "↑↓"));
                v.push(("Page", "⇟/⇞"));
                v.push(("Jump", "Home/End"));
                if app.is_uncommitted_selected() {
                    v.push(("Accept Ours", &conflict_ours));
                    v.push(("Accept Theirs", &conflict_theirs));
                    v.push(("Mark Resolved", &conflict_resolve));
                    v.push(("Mergetool", &conflict_mergetool));
                    v.push(("Abort Merge", &conflict_abort));
                    v.push(("Continue Merge", &conflict_continue));
                }
                v.push(("Inspect", "↵/→"));
            } else if app.detail_focus == DetailSection::ConflictDiff {
                v.push(("Scroll Diff", "↑↓/⇟⇞"));
                if app.is_uncommitted_selected() {
                    v.push(("Accept Ours", &conflict_ours));
                    v.push(("Accept Theirs", &conflict_theirs));
                    v.push(("Mark Resolved", &conflict_resolve));
                    v.push(("Mergetool", &conflict_mergetool));
                    v.push(("Abort Merge", &conflict_abort));
                    v.push(("Continue Merge", &conflict_continue));
                }
                v.push(("Home", ""));
            } else {
                v.push(("Navigate/Scroll", "↑↓"));
                v.push(("Page", "⇟/⇞"));
                v.push(("Jump", "Home/End"));
                v.push(("Inspect", &ws_inspect));
                v.push(("Checkout", &ws_checkout));
                v.push(("Tag", &ws_tag));
                v.push(("Branch", &ws_branch));
                v.push(("Interactive Rebase", &ws_irebase));
                v.push(("Cherry-pick", &ws_cherry_pick));
                v.push(("Revert", &ws_revert));
                v.push(("Fuzzy Search", &ws_fuzzy));
                v.push(("Search Columns", &ws_columns));
                v.push(("Logs UI", &ws_logs));
                v.push(("Load More", &ws_load_more));
                v.push(("Yank Hash", &ws_yank));
                if app.has_uncommitted_changes() {
                    v.push(("Stash", &ws_stash));
                }
            }
            if app.detail_focus != DetailSection::Conflicts
                && app.detail_focus != DetailSection::ConflictDiff
            {
                v.push(("Commit/Amend", &ws_commit_amend));
            } else {
                v.push(("Commit", &ws_commit));
            }
            v.push(("Resync", ""));
            v.push(("Help", ""));
            v
        }
        1 => {
            let mut v = vec![
                ("Home", ""),
                ("Tabs", ""),
                ("Cycle Focus", ""),
                ("Resize", ""),
                ("Navigate/Scroll", "↑↓"),
                ("Page", "⇟/⇞"),
                ("Jump", "Home/End"),
            ];
            if app.detail_focus == DetailSection::Files {
                v.push(("Expand/Toggle", &files_expand));
                v.push(("Collapse", &files_collapse));
                v.push(("Collapse All", "←"));
                v.push(("Fuzzy Find", &files_search));
                v.push(("History", &files_history));
                // Literal x / X in src/components/file_tree.rs.
                v.push(("Discard", "x/X"));
            } else if app.detail_focus == DetailSection::FileContent {
                if app.inspect_full_diff {
                    v.push(("Exit Full Screen", &files_exit_full_screen));
                    let line_no_label =
                        if app.file_tree.show_line_numbers { "Hide Lines" } else { "Show Lines" };
                    v.push((line_no_label, &files_line_numbers));
                    let blame_label =
                        if app.file_tree.show_blame { "Hide Blame" } else { "Show Blame" };
                    v.push((blame_label, &files_blame));
                } else {
                    v.push(("Full Screen", &files_full_screen));
                }
            }
            if let Some(item) = app.file_tree.visible_files.get(app.file_tree.file_list_selection) {
                if !item.is_dir {
                    v.push(("Open in Editor", &files_editor));
                }
            }
            v.push(("Resync", ""));
            v.push(("Help", ""));
            v
        }
        2 => vec![
            ("Home", ""),
            ("Tabs", ""),
            ("Scroll", "↑↓"),
            ("Page", "⇟/⇞"),
            ("Jump", "Home/End"),
            ("Inspect", "↵"),
            ("Yank Hash", "y"),
            ("Resync", ""),
            ("Help", ""),
        ],
        3 => {
            let mut v = vec![
                ("Home", ""),
                ("Tabs", ""),
                ("Cycle Focus", ""),
                ("Resize", ""),
                ("Checkout", branches_checkout.as_str()),
                ("Create", branches_create.as_str()),
                ("Delete", branches_delete.as_str()),
                ("Merge", branches_merge.as_str()),
                ("Rebase", branches_rebase.as_str()),
                ("Interactive Rebase", branches_irebase.as_str()),
            ];
            if app.detail_focus == DetailSection::LocalBranches {
                v.push(("Merge Into", &branches_merge_into));
                v.push(("Pull", &branches_pull));
                v.push(("Push", &branches_push));
            }
            v.push(("Fuzzy Search", &branches_search));
            // Fetch and Add Remote are literal f / F and a / A in
            // src/components/branch_list.rs and work from either panel.
            v.push(("Fetch", "f/F"));
            v.push(("Add Remote", "a/A"));
            v.push(("Navigate", "↑↓"));
            v.push(("Page", "⇟/⇞"));
            v.push(("Jump", "Home/End"));
            v.push(("Focus L/R", "←/→"));
            v.push(("Resync", ""));
            v.push(("Help", ""));
            v
        }
        4 => vec![
            ("Home", ""),
            ("Tabs", ""),
            ("Cycle Focus", ""),
            ("Checkout", tags_checkout.as_str()),
            ("Navigate", "↑↓"),
            ("Page", "⇟/⇞"),
            ("Jump", "Home/End"),
            ("Fuzzy Search", tags_search.as_str()),
            ("Fetch", tags_fetch.as_str()),
            ("Push", tags_push.as_str()),
            ("Push All", tags_push_all.as_str()),
            ("Delete", tags_delete.as_str()),
            ("Resync", ""),
            ("Help", ""),
        ],
        5 => vec![
            ("Home", ""),
            ("Tabs", ""),
            ("Navigate", "↑↓"),
            ("Page", "⇟/⇞"),
            ("Jump", "Home/End"),
            ("Fetch", remotes_fetch.as_str()),
            ("Add", remotes_add.as_str()),
            ("Delete", remotes_delete.as_str()),
            ("Resync", ""),
            ("Help", ""),
        ],
        6 => {
            let mut v = vec![
                ("Home", ""),
                ("Tabs", ""),
                ("Cycle Focus", ""),
                ("Resize", ""),
                ("Navigate", "↑↓"),
                ("Page", "⇟/⇞"),
                ("Jump", "Home/End"),
            ];
            if app.detail_focus == DetailSection::Stashes {
                v.push(("Apply", &stashes_apply));
                v.push(("Delete", &stashes_delete));
                v.push(("Stash New", &stashes_create));
            }
            v.push(("Resync", ""));
            v.push(("Help", ""));
            v
        }
        7 => vec![
            ("Home", ""),
            ("Tabs", ""),
            ("Navigate", "↑↓"),
            ("Page", "⇟/⇞"),
            ("Jump", "Home/End"),
            ("Add", worktrees_add.as_str()),
            ("Delete", worktrees_delete.as_str()),
            ("Lock/Unlock", worktrees_lock.as_str()),
            ("Prune", worktrees_prune.as_str()),
            ("Open", worktrees_open.as_str()),
            ("Resync", ""),
            ("Help", ""),
        ],
        8 => vec![
            ("Home", ""),
            ("Tabs", ""),
            ("Navigate", "↑↓"),
            ("Page", "⇟/⇞"),
            ("Jump", "Home/End"),
            ("Add", submodules_add.as_str()),
            ("Delete", submodules_delete.as_str()),
            ("Resync", ""),
            ("Help", ""),
        ],
        9 => vec![
            ("Home", ""),
            ("Tabs", ""),
            ("Navigate", "↑↓"),
            ("Page", "⇟/⇞"),
            ("Jump", "Home/End"),
            ("Checkout Commit", reflog_checkout.as_str()),
            ("Resync", ""),
            ("Help", ""),
        ],
        10 => vec![
            ("Home", ""),
            ("Tabs", ""),
            ("Cycle Focus", ""),
            ("Resize", ""),
            ("Navigate", "↑↓"),
            ("Page", "⇟/⇞"),
            ("Jump", "Home/End"),
            ("Checkout Branch", forge_checkout.as_str()),
            ("Open Browser", forge_open_browser.as_str()),
            ("Toggle Assigned", forge_toggle_assigned.as_str()),
            ("Resync", ""),
            ("Help", ""),
        ],
        11 => vec![
            ("Home", ""),
            ("Tabs", ""),
            ("Cycle Focus", ""),
            ("Resize", ""),
            ("Navigate", "↑↓"),
            ("Page", "⇟/⇞"),
            ("Jump", "Home/End"),
            ("Checkout PR Branch", forge_checkout.as_str()),
            ("Open Browser", forge_open_browser.as_str()),
            ("Add Comment", forge_add_comment.as_str()),
            ("Resync", ""),
            ("Help", ""),
        ],
        _ => vec![("Home", ""), ("Tabs", ""), ("Resync", ""), ("Help", "")],
    };
    let home_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::CloseDetail, compat);
    let cycle_focus_key = format!(
        "{}/{}",
        app.keybindings.format_action_keys(crate::keybindings::Action::CycleFocusForward, compat),
        app.keybindings.format_action_keys(crate::keybindings::Action::CycleFocusBackward, compat)
    );
    let resync_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::RefreshDetail, compat);
    let resize_key = format!(
        "{}/{}",
        app.keybindings.format_action_keys(crate::keybindings::Action::GrowPanel, compat),
        app.keybindings.format_action_keys(crate::keybindings::Action::ShrinkPanel, compat)
    );
    let help_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::DetailHelp, compat);
    let toggle_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::ToggleAdvancedTabs, compat);

    let home_key_ref = home_key.as_str();
    let cycle_focus_key_ref = cycle_focus_key.as_str();
    let resync_key_ref = resync_key.as_str();
    let resize_key_ref = resize_key.as_str();
    let help_key_ref = help_key.as_str();
    let toggle_key_ref = toggle_key.as_str();

    let mut final_entries = Vec::new();
    for (label, key) in entries_data {
        if label == "Tabs" {
            final_entries.push(("Tabs", if app.advanced_tabs { "1-5" } else { "1-7" }));
            final_entries
                .push((if app.advanced_tabs { "Primary" } else { "Advanced" }, toggle_key_ref));
        } else if label == "Home" {
            final_entries.push(("Home", home_key_ref));
        } else if label == "Cycle Focus" {
            final_entries.push(("Cycle Focus", cycle_focus_key_ref));
        } else if label == "Resize" {
            final_entries.push(("Resize", resize_key_ref));
        } else if label == "Resync" {
            final_entries.push(("Resync", resync_key_ref));
        } else if label == "Help" {
            final_entries.push(("Help", help_key_ref));
        } else {
            final_entries.push((label, key));
        }
    }
    let entries = super::build_status_entries(&final_entries);
    (message_spans, entries)
}

pub(crate) fn inspect_dismiss_entries(app: &App) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let mut message_spans = None;
    if let Some(msg) = &app.status_message {
        message_spans = Some(vec![Span::styled(format!("{} ", msg), accent_style())]);
    }

    let mut entries_data = Vec::new();

    // Leaving the full-screen diff is a literal ← (src/popups/inspect.rs)
    // plus the CloseDetail binding, so the caption follows the binding.
    let compat = app.config.compatibility_mode;
    let exit_full_screen_key = format!(
        "←/{}",
        app.keybindings.format_action_keys(crate::keybindings::Action::CloseDetail, compat)
    );

    if app.detail_focus == DetailSection::ConflictDiff {
        let exit_label = if app.inspect_full_diff { "Exit Full Screen" } else { "Workspace" };
        let exit_key = if app.inspect_full_diff { exit_full_screen_key.as_str() } else { "⎋/q" };
        entries_data.push((exit_label, exit_key));
        if app.is_uncommitted_selected() {
            entries_data.push(("Accept Ours", "o"));
            entries_data.push(("Accept Theirs", "t"));
            entries_data.push(("Mark Resolved", "r"));
            entries_data.push(("Mergetool", "M"));
            entries_data.push(("Abort Merge", "A"));
            entries_data.push(("Continue Merge", "C"));
        }
        if app.inspect_full_diff {
            entries_data.push(("Scroll Diff", "↑↓"));
        } else {
            entries_data.push(("Scroll Diff", "↑↓/⇟⇞"));
        }
        entries_data.push(("Help", "?"));
    } else if app.detail_focus == DetailSection::Conflicts {
        let exit_label = if app.in_logs_ui { "Logs UI" } else { "Workspace" };
        entries_data.push((exit_label, "⎋/q"));
        entries_data.push(("Cycle Focus", "w/W"));
        if app.is_uncommitted_selected() {
            entries_data.push(("Accept Ours", "o"));
            entries_data.push(("Accept Theirs", "t"));
            entries_data.push(("Mark Resolved", "r"));
            entries_data.push(("Mergetool", "M"));
            entries_data.push(("Abort Merge", "A"));
            entries_data.push(("Continue Merge", "C"));
        }
        entries_data.push(("Inspect", "↵/→"));
        entries_data.push(("Select File", "↑↓"));
        entries_data.push(("Help", "?"));
    } else if app.inspect_full_diff {
        entries_data.push(("Exit Full Screen", exit_full_screen_key.as_str()));

        if app.is_uncommitted_selected() {
            if app.diff.diff_line_mode {
                entries_data.push(("Hunk Mode", "l"));
                if app.last_staging_focus == DetailSection::Staged {
                    entries_data.push(("Unstage Line", "↵"));
                } else if app.last_staging_focus == DetailSection::Unstaged {
                    entries_data.push(("Stage Line", "↵"));
                    entries_data.push(("Discard Line", "x/Del"));
                }
            } else {
                entries_data.push(("Line Mode", "l"));
                if app.last_staging_focus == DetailSection::Staged {
                    entries_data.push(("Unstage Hunk", "↵"));
                } else if app.last_staging_focus == DetailSection::Unstaged {
                    entries_data.push(("Stage Hunk", "↵"));
                    entries_data.push(("Discard Hunk", "x/Del"));
                }
            }
            entries_data.push(("Commit/Amend", "c/C"));
        }
        entries_data.push(("Scroll Diff", "↑↓"));
        entries_data.push(("Help", "?"));
    } else {
        let exit_label = if app.in_logs_ui { "Logs UI" } else { "Workspace" };
        entries_data.push((exit_label, "⎋/q"));
        entries_data.push(("Cycle Focus", "w/W"));

        if app.is_uncommitted_selected() {
            match app.detail_focus {
                DetailSection::Staged => {
                    entries_data.push(("Unstage File", "↵"));
                    entries_data.push(("Unstage All", "a"));
                    entries_data.push(("Discard", "x"));
                    entries_data.push(("Discard All", "X"));
                }
                DetailSection::Unstaged => {
                    entries_data.push(("Stage File", "↵"));
                    entries_data.push(("Stage All", "a"));
                    entries_data.push(("Discard", "x"));
                    entries_data.push(("Discard All", "X"));
                }
                DetailSection::StagingDetails => {
                    if app.diff.diff_line_mode {
                        entries_data.push(("Hunk Mode", "l"));
                        if app.last_staging_focus == DetailSection::Staged {
                            entries_data.push(("Unstage Line", "↵/u"));
                        } else if app.last_staging_focus == DetailSection::Unstaged {
                            entries_data.push(("Stage Line", "↵/s"));
                            entries_data.push(("Discard Line", "x/Del"));
                        }
                    } else {
                        entries_data.push(("Line Mode", "l"));
                        if app.last_staging_focus == DetailSection::Staged {
                            entries_data.push(("Unstage Hunk", "↵/u"));
                        } else if app.last_staging_focus == DetailSection::Unstaged {
                            entries_data.push(("Stage Hunk", "↵/s"));
                            entries_data.push(("Discard Hunk", "x/Del"));
                        }
                    }
                }
                _ => {}
            }
            entries_data.push(("Commit/Amend", "c/C"));
        }

        entries_data.push(("Select File", "↑↓"));
        if app.detail_focus == DetailSection::StagingDetails {
            entries_data.push(("Full Screen Diff", "→"));
            entries_data.push(("Scroll Diff", "↑↓"));
        } else {
            entries_data.push(("Scroll Diff", "↑↓ (focused)"));
        }
        entries_data.push(("Help", "?"));
    }

    let exit_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::CloseDetail, compat);
    let cycle_focus_key = format!(
        "{}/{}",
        app.keybindings.format_action_keys(crate::keybindings::Action::CycleFocusForward, compat),
        app.keybindings.format_action_keys(crate::keybindings::Action::CycleFocusBackward, compat)
    );
    let help_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::DetailHelp, compat);

    let exit_key_ref = exit_key.as_str();
    let cycle_focus_key_ref = cycle_focus_key.as_str();
    let help_key_ref = help_key.as_str();

    let mut final_entries = Vec::new();
    for (label, key) in entries_data {
        if label == "Workspace" || label == "Logs UI" {
            final_entries.push((label, exit_key_ref));
        } else if label == "Cycle Focus" {
            final_entries.push(("Cycle Focus", cycle_focus_key_ref));
        } else if label == "Help" {
            final_entries.push(("Help", help_key_ref));
        } else {
            final_entries.push((label, key));
        }
    }

    let entries = super::build_status_entries(&final_entries);
    (message_spans, entries)
}

pub(crate) fn detail_help_entries(app: &App) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let compat = app.config.compatibility_mode;
    let help_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::DetailHelp, compat);
    let esc_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::CloseDetail, compat);
    let close_keys = format!("{}/{}", help_key, esc_key);
    let entries = vec![StatusEntry::new(vec![
        Span::raw("Close Help"),
        Span::raw(" "),
        Span::styled("[", muted_style()),
        Span::styled(close_keys, accent_style()),
        Span::styled("]", muted_style()),
    ])];
    (None, entries)
}
