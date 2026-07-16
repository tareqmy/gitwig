//! Detail pane view status bar entry generation.

use super::StatusEntry;
use crate::app::{App, DetailSection, Mode};
use crate::ui::style::{ACCENT, DANGER, WARNING, accent_style, muted_style, primary_style};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

pub(crate) fn detail_dismiss_entries(app: &App) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let mut message_spans = None;
    if let Some(msg) = &app.status_message {
        message_spans = Some(vec![Span::styled(format!("{} ", msg), accent_style())]);
    }

    let entries_data = match app.detail_tab {
        0 => {
            let mut v = vec![("Home", "⎋/q"), ("Tabs", "Tab/1-9"), ("Cycle Focus", "w/W")];
            if app.detail_focus == DetailSection::CommitDetails {
                v.push(("Scroll Info", "↑↓"));
                v.push(("Inspect", "→"));
            } else if app.detail_focus == DetailSection::Staged
                || app.detail_focus == DetailSection::Unstaged
                || app.detail_focus == DetailSection::StagingDetails
            {
                v.push(("Navigate/Scroll", "↑↓"));
                v.push(("Page", "⇟/⇞"));
                v.push(("Jump", "Home/End"));
                if app.is_uncommitted_selected() {
                    v.push(("Stage/Unstage", "↵"));
                    if app.detail_focus == DetailSection::Unstaged {
                        v.push(("Stage All", "a"));
                    } else if app.detail_focus == DetailSection::Staged {
                        v.push(("Unstage All", "a"));
                    }
                    v.push(("Discard", "x"));
                    v.push(("Discard All", "X"));
                    v.push(("Stash", "s"));
                }
                v.push(("Inspect", "→"));
            } else if app.detail_focus == DetailSection::Conflicts {
                v.push(("Navigate/Scroll", "↑↓"));
                v.push(("Page", "⇟/⇞"));
                v.push(("Jump", "Home/End"));
                if app.is_uncommitted_selected() {
                    v.push(("Accept Ours", "o"));
                    v.push(("Accept Theirs", "t"));
                    v.push(("Mark Resolved", "r"));
                    v.push(("Abort Merge", "A"));
                    v.push(("Continue Merge", "C"));
                }
                v.push(("Inspect", "↵/→"));
            } else if app.detail_focus == DetailSection::ConflictDiff {
                v.push(("Scroll Diff", "↑↓/⇟⇞"));
                if app.is_uncommitted_selected() {
                    v.push(("Accept Ours", "o"));
                    v.push(("Accept Theirs", "t"));
                    v.push(("Mark Resolved", "r"));
                    v.push(("Abort Merge", "A"));
                    v.push(("Continue Merge", "C"));
                }
                v.push(("Back to List", "←/Esc"));
            } else {
                v.push(("Navigate/Scroll", "↑↓"));
                v.push(("Page", "⇟/⇞"));
                v.push(("Jump", "Home/End"));
                v.push(("Inspect", "↵/→"));
                v.push(("Checkout", "o"));
                v.push(("Tag", "t"));
                v.push(("Interactive Rebase", "i"));
                v.push(("Cherry-pick", "p"));
                v.push(("Revert", "v"));
                v.push(("Fuzzy Search", "/"));
                v.push(("Search Columns", "f"));
                v.push(("Logs UI", "l"));
                v.push(("Load More", "G"));
                v.push(("Yank Hash", "y"));
                if app.has_uncommitted_changes() {
                    v.push(("Stash", "s"));
                }
            }
            if app.detail_focus != DetailSection::Conflicts
                && app.detail_focus != DetailSection::ConflictDiff
            {
                v.push(("Commit/Amend", "c/C"));
            } else {
                v.push(("Commit", "c"));
            }
            v.push(("Resync", "R"));
            v.push(("Help", "?"));
            v
        }
        1 => {
            let mut v = vec![
                ("Home", "⎋/q"),
                ("Tabs", "Tab/1-9"),
                ("Cycle Focus", "w/W"),
                ("Navigate/Scroll", "↑↓"),
                ("Page", "⇟/⇞"),
                ("Jump", "Home/End"),
            ];
            if app.detail_focus == DetailSection::Files {
                v.push(("Expand/Collapse", "←/→"));
                v.push(("Fuzzy Find", "/"));
                v.push(("History", "⇧H"));
            } else if app.detail_focus == DetailSection::FileContent {
                if app.inspect_full_diff {
                    v.push(("Exit Full Screen", "←/⎋/q"));
                    let line_no_label =
                        if app.file_tree.show_line_numbers { "Hide Lines" } else { "Show Lines" };
                    v.push((line_no_label, "n"));
                    let blame_label =
                        if app.file_tree.show_blame { "Hide Blame" } else { "Show Blame" };
                    v.push((blame_label, "b"));
                } else {
                    v.push(("Full Screen", "→"));
                }
            }
            if let Some(item) = app.file_tree.visible_files.get(app.file_tree.file_list_selection) {
                if !item.is_dir {
                    v.push(("Open in Editor", "e/o"));
                }
            }
            v.push(("Resync", "R"));
            v.push(("Help", "?"));
            v
        }
        2 => vec![
            ("Home", "⎋/q"),
            ("Tabs", "Tab/1-9"),
            ("Scroll", "↑↓"),
            ("Page", "⇟/⇞"),
            ("Jump", "Home/End"),
            ("Resync", "R"),
            ("Help", "?"),
        ],
        3 => {
            let mut v = vec![
                ("Home", "⎋/q"),
                ("Tabs", "Tab/1-9"),
                ("Cycle Focus", "w/W"),
                ("Checkout", "↵"),
                ("Create", "c"),
                ("Delete", "D"),
                ("Merge", "m"),
                ("Rebase", "r"),
                ("Interactive Rebase", "i"),
            ];
            if app.detail_focus == DetailSection::LocalBranches {
                v.push(("Fuzzy Search", "/"));
                v.push(("Fetch", "F"));
                v.push(("Pull", "p"));
                v.push(("Push", "⇧P"));
            }
            v.push(("Navigate", "↑↓"));
            v.push(("Page", "⇟/⇞"));
            v.push(("Jump", "Home/End"));
            v.push(("Focus L/R", "←/→"));
            v.push(("Resync", "R"));
            v.push(("Help", "?"));
            v
        }
        4 => vec![
            ("Home", "⎋/q"),
            ("Tabs", "Tab/1-9"),
            ("Checkout", "↵"),
            ("Navigate", "↑↓"),
            ("Page", "⇟/⇞"),
            ("Jump", "Home/End"),
            ("Fuzzy Search", "/"),
            ("Fetch", "F"),
            ("Push", "p"),
            ("Push All", "⇧P"),
            ("Delete", "D"),
            ("Resync", "R"),
            ("Help", "?"),
        ],
        5 => vec![
            ("Home", "⎋/q"),
            ("Tabs", "Tab/1-9"),
            ("Navigate", "↑↓"),
            ("Page", "⇟/⇞"),
            ("Jump", "Home/End"),
            ("Fetch", "f/F"),
            ("Add", "a/A"),
            ("Delete", "D"),
            ("Resync", "R"),
            ("Help", "?"),
        ],
        6 => {
            let mut v = vec![
                ("Home", "⎋/q"),
                ("Tabs", "Tab/1-9"),
                ("Cycle Focus", "w/W"),
                ("Navigate", "↑↓"),
                ("Page", "⇟/⇞"),
                ("Jump", "Home/End"),
            ];
            if app.detail_focus == DetailSection::Stashes {
                v.push(("Apply", "a"));
                v.push(("Delete", "D"));
                v.push(("Stash New", "s"));
            }
            v.push(("Resync", "R"));
            v.push(("Help", "?"));
            v
        }
        7 => vec![
            ("Home", "⎋/q"),
            ("Tabs", "Tab/1-9"),
            ("Navigate", "↑↓"),
            ("Add", "a"),
            ("Delete", "D"),
            ("Lock/Unlock", "l"),
            ("Prune", "p"),
            ("Open", "↵"),
            ("Resync", "R"),
            ("Help", "?"),
        ],
        8 => vec![
            ("Home", "⎋/q"),
            ("Tabs", "Tab/0-9"),
            ("Navigate", "↑↓"),
            ("Add", "a"),
            ("Delete", "D"),
            ("Resync", "R"),
            ("Help", "?"),
        ],
        9 => vec![
            ("Home", "⎋/q"),
            ("Tabs", "Tab/0-9"),
            ("Navigate", "↑↓"),
            ("Page", "⇟/⇞"),
            ("Jump", "Home/End"),
            ("Checkout Commit", "↵/Space"),
            ("Resync", "R"),
            ("Help", "?"),
        ],
        10 => vec![
            ("Home", "⎋/q"),
            ("Tabs", "Tab/0-9"),
            ("Cycle Focus", "w/W"),
            ("Navigate", "↑↓"),
            ("Page", "⇟/⇞"),
            ("Jump", "Home/End"),
            ("Checkout Branch", "↵"),
            ("Open Browser", "o"),
            ("Toggle Assigned", "a"),
            ("Resync", "R"),
            ("Help", "?"),
        ],
        11 => vec![
            ("Home", "⎋/q"),
            ("Tabs", "Tab/0-9"),
            ("Cycle Focus", "w/W"),
            ("Navigate", "↑↓"),
            ("Page", "⇟/⇞"),
            ("Jump", "Home/End"),
            ("Checkout PR Branch", "↵"),
            ("Open Browser", "o"),
            ("Add Comment", "n"),
            ("Resync", "R"),
            ("Help", "?"),
        ],
        _ => vec![("Home", "⎋/q"), ("Tabs", "Tab/0-9"), ("Resync", "R"), ("Help", "?")],
    };
    let compat = app.config.compatibility_mode;
    let home_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::CloseDetail, compat);
    let cycle_focus_key = format!(
        "{}/{}",
        app.keybindings.format_action_keys(crate::keybindings::Action::CycleFocusForward, compat),
        app.keybindings.format_action_keys(crate::keybindings::Action::CycleFocusBackward, compat)
    );
    let resync_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::RefreshDetail, compat);
    let help_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::DetailHelp, compat);
    let toggle_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::ToggleAdvancedTabs, compat);

    let home_key_ref = home_key.as_str();
    let cycle_focus_key_ref = cycle_focus_key.as_str();
    let resync_key_ref = resync_key.as_str();
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

    if app.detail_focus == DetailSection::ConflictDiff {
        let exit_label = if app.inspect_full_diff { "Exit Full Screen" } else { "Workspace" };
        let exit_key = if app.inspect_full_diff { "←/⎋/q" } else { "⎋/q" };
        entries_data.push((exit_label, exit_key));
        if app.is_uncommitted_selected() {
            entries_data.push(("Accept Ours", "o"));
            entries_data.push(("Accept Theirs", "t"));
            entries_data.push(("Mark Resolved", "r"));
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
            entries_data.push(("Abort Merge", "A"));
            entries_data.push(("Continue Merge", "C"));
        }
        entries_data.push(("Inspect", "↵/→"));
        entries_data.push(("Select File", "↑↓"));
        entries_data.push(("Help", "?"));
    } else if app.inspect_full_diff {
        entries_data.push(("Exit Full Screen", "←/⎋/q"));

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

    let compat = app.config.compatibility_mode;
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
