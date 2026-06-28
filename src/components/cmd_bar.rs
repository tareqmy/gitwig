use crate::app::{App, DetailSection, Mode};
use crate::config::SortOrder;
use crate::ui::style::{ACCENT, DANGER, SUCCESS, accent_style, muted_style, primary_style};
use crate::ui::{
    confirm_tag_delete_entries, confirm_tag_push_all_entries, confirm_tag_push_entries,
    draw_input_status, get_process_stats,
};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

pub struct StatusEntry {
    pub(crate) spans: Vec<Span<'static>>,
}

impl StatusEntry {
    pub fn new(spans: Vec<Span<'static>>) -> Self {
        Self { spans }
    }

    pub fn width(&self) -> usize {
        self.spans.iter().map(|s| s.content.chars().count()).sum()
    }
}

pub fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    if app.loading_repo_path.is_some() {
        let msg_spans = vec![Span::styled(
            "Loading Repository...  ",
            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
        )];
        let entries_data = [("Cancel", "Esc")];
        let mut entries = Vec::new();
        for (i, (label, key)) in entries_data.iter().enumerate() {
            let mut spans = Vec::new();
            if i > 0 {
                spans.push(Span::styled(" ", muted_style()));
            }
            spans.push(Span::raw((*label).to_string()));
            spans.push(Span::raw(" "));
            spans.push(Span::styled("[", muted_style()));
            spans.push(Span::styled((*key).to_string(), accent_style()));
            spans.push(Span::styled("]", muted_style()));
            entries.push(StatusEntry::new(spans));
        }
        draw_status_layout(f, area, Some(msg_spans), entries, app);
        return;
    }

    match &app.mode {
        Mode::Settings => {
            let msg_spans = if let Some(msg) = &app.status_message {
                vec![Span::styled(format!("{} ", msg), accent_style())]
            } else if app.settings_editing {
                if app.settings_selected_index == 3 {
                    vec![Span::raw("Selecting theme... (Press Up/Down to choose)")]
                } else {
                    vec![Span::raw("Editing setting...")]
                }
            } else {
                vec![
                    Span::raw("Settings (Esc to exit) | Use "),
                    Span::styled("Enter", accent_style()),
                    Span::raw(" / "),
                    Span::styled("Space", accent_style()),
                    Span::raw(" to toggle/edit"),
                ]
            };
            let entries = if app.settings_editing {
                let entries_data = [("Save", "Enter"), ("Cancel", "Esc")];
                let mut entries = Vec::new();
                for (i, (label, key)) in entries_data.iter().enumerate() {
                    let mut spans = Vec::new();
                    if i > 0 {
                        spans.push(Span::styled(" ", muted_style()));
                    }
                    spans.push(Span::raw((*label).to_string()));
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled("[", muted_style()));
                    spans.push(Span::styled((*key).to_string(), accent_style()));
                    spans.push(Span::styled("]", muted_style()));
                    entries.push(StatusEntry::new(spans));
                }
                entries
            } else {
                let entries_data = [
                    ("Select", "↑/↓"),
                    ("Page", "⇟/⇞"),
                    ("Jump", "Home/End"),
                    ("Edit/Toggle", "Enter/Space"),
                    ("Back", "Esc/q"),
                ];
                let mut entries = Vec::new();
                for (i, (label, key)) in entries_data.iter().enumerate() {
                    let mut spans = Vec::new();
                    if i > 0 {
                        spans.push(Span::styled(" ", muted_style()));
                    }
                    spans.push(Span::raw((*label).to_string()));
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled("[", muted_style()));
                    spans.push(Span::styled((*key).to_string(), accent_style()));
                    spans.push(Span::styled("]", muted_style()));
                    entries.push(StatusEntry::new(spans));
                }
                entries
            };
            draw_status_layout(f, area, Some(msg_spans), entries, app);
        }
        Mode::Normal => {
            let (msg_spans, entries) = normal_status_entries(app);
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::Adding => {
            draw_input_status(f, area, "Add", &app.input_buffer);
        }
        Mode::BulkAddInput => {
            draw_input_status(f, area, "Bulk Add (Tab for FZF)", &app.input_buffer);
        }
        Mode::Editing => {
            draw_input_status(f, area, "Edit", &app.input_buffer);
        }
        Mode::RepoSearchInput => {
            draw_input_status(f, area, "Find", &app.input_buffer);
        }
        Mode::ImportUrlInput | Mode::ImportDestInput | Mode::ImportNameInput => {
            let msg_spans = vec![Span::styled(
                "Importing Remote Repository  ",
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            )];
            let entries_data = [("Cancel", "Esc")];
            let mut entries = Vec::new();
            for (i, (label, key)) in entries_data.iter().enumerate() {
                let mut spans = Vec::new();
                if i > 0 {
                    spans.push(Span::styled(" ", muted_style()));
                }
                spans.push(Span::raw((*label).to_string()));
                spans.push(Span::raw(" "));
                spans.push(Span::styled("[", muted_style()));
                spans.push(Span::styled((*key).to_string(), accent_style()));
                spans.push(Span::styled("]", muted_style()));
                entries.push(StatusEntry::new(spans));
            }
            draw_status_layout(f, area, Some(msg_spans), entries, app);
        }
        Mode::ConfirmDelete => {
            let target = app.get_selected_item().map(|s| s.as_str()).unwrap_or("");
            let (msg_spans, entries) = confirm_delete_entries(target);
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::Help => {
            let (msg_spans, entries) = help_dismiss_entries();
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::About => {
            let (msg_spans, entries) = about_dismiss_entries();
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::Detail | Mode::RemoteAddNameInput | Mode::RemoteAddUrlInput => {
            let (msg_spans, entries) = detail_dismiss_entries(app);
            draw_status_layout(f, area, msg_spans, entries, app);
        }

        Mode::DetailHelp => {
            let (msg_spans, entries) = detail_help_entries();
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::CommitInput => {
            let (msg_spans, entries) = if app.commit_popup.editing {
                commit_input_editing_entries()
            } else {
                commit_input_confirm_entries(app.commit_popup.amend)
            };
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::BranchCreateInput => {
            draw_input_status(f, area, "Create Branch", &app.input_buffer);
        }
        Mode::TagCreateInput => {
            draw_input_status(f, area, "Create Tag", &app.input_buffer);
        }
        Mode::StashCreateInput => {
            draw_input_status(f, area, "Stash Changes", &app.input_buffer);
        }
        Mode::BranchDeleteConfirm => {
            let (target, is_remote) = app
                .branch_action_target
                .as_ref()
                .map(|(name, remote)| (name.as_str(), *remote))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_branch_delete_entries(target, is_remote);
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::BranchCheckoutConfirm => {
            let (target, is_remote) = app
                .branch_action_target
                .as_ref()
                .map(|(name, remote)| (name.as_str(), *remote))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_branch_checkout_entries(target, is_remote);
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::TagCheckoutConfirm => {
            let target = app.tag_checkout_target.as_deref().unwrap_or("");
            let (msg_spans, entries) = confirm_tag_checkout_entries(target);
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::BranchPushConfirm => {
            let target =
                app.branch_action_target.as_ref().map(|(name, _)| name.as_str()).unwrap_or("");
            let (msg_spans, entries) = confirm_branch_push_entries(target);
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::BranchMergeConfirm => {
            let (target, is_remote) = app
                .branch_action_target
                .as_ref()
                .map(|(name, remote)| (name.as_str(), *remote))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_branch_merge_entries(target, is_remote);
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::BranchRebaseConfirm => {
            let (target, is_remote) = app
                .branch_action_target
                .as_ref()
                .map(|(name, remote)| (name.as_str(), *remote))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_branch_rebase_entries(target, is_remote);
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::BranchInteractiveRebaseConfirm => {
            let (target, is_remote) = app
                .branch_action_target
                .as_ref()
                .map(|(name, remote)| (name.as_str(), *remote))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_branch_interactive_rebase_entries(target, is_remote);
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::TagDeleteConfirm => {
            let (target, is_on_remote) = app
                .tag_delete_target
                .as_ref()
                .map(|(name, is_on_remote)| (name.as_str(), *is_on_remote))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_tag_delete_entries(target, is_on_remote);
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::TagPushConfirm => {
            let target = app.tag_push_target.as_deref().unwrap_or("");
            let (msg_spans, entries) = confirm_tag_push_entries(target);
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::TagPushAllConfirm => {
            let (msg_spans, entries) = confirm_tag_push_all_entries();
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::StashDeleteConfirm => {
            let target = match &app.current_detail {
                Some(crate::repo::ItemDetail::Repo { info, .. }) => info
                    .stashes
                    .get(app.stash_list.stash_selection)
                    .map(|s| format!("stash@{{{}}}", s.index))
                    .unwrap_or_else(|| "".to_string()),
                _ => "".to_string(),
            };
            let (msg_spans, entries) = confirm_stash_delete_entries(&target);
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::StashApplyConfirm => {
            let target = match &app.current_detail {
                Some(crate::repo::ItemDetail::Repo { info, .. }) => info
                    .stashes
                    .get(app.stash_list.stash_selection)
                    .map(|s| format!("stash@{{{}}}", s.index))
                    .unwrap_or_else(|| "".to_string()),
                _ => "".to_string(),
            };
            let (msg_spans, entries) =
                confirm_stash_apply_entries(&target, app.stash_apply_delete_after);
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::CherryPickConfirm => {
            let (target, summary) = app
                .cherry_pick_target
                .as_ref()
                .map(|(oid, sum)| (oid.clone(), sum.clone()))
                .unwrap_or_default();
            let msg_spans = vec![
                Span::raw("Cherry-pick commit "),
                Span::styled(format!("{:.7}", target), accent_style().add_modifier(Modifier::BOLD)),
                Span::raw(" ("),
                Span::styled(summary, primary_style()),
                Span::raw(")?"),
            ];
            let entries = vec![
                StatusEntry::new(vec![
                    Span::raw("Navigate"),
                    Span::raw(" "),
                    Span::styled("[", muted_style()),
                    Span::styled("↑↓/jk", accent_style()),
                    Span::styled("]", muted_style()),
                ]),
                StatusEntry::new(vec![
                    Span::styled(" ", muted_style()),
                    Span::raw("Confirm"),
                    Span::raw(" "),
                    Span::styled("[", muted_style()),
                    Span::styled("↵", accent_style()),
                    Span::styled("]", muted_style()),
                ]),
                StatusEntry::new(vec![
                    Span::styled(" ", muted_style()),
                    Span::raw("Cancel"),
                    Span::raw(" "),
                    Span::styled("[", muted_style()),
                    Span::styled("⎋/q", accent_style()),
                    Span::styled("]", muted_style()),
                ]),
            ];
            draw_status_layout(f, area, Some(msg_spans), entries, app);
        }
        Mode::RevertConfirm => {
            let (target, summary) = app
                .revert_target
                .as_ref()
                .map(|(oid, sum)| (oid.clone(), sum.clone()))
                .unwrap_or_default();
            let msg_spans = vec![
                Span::raw("Revert commit "),
                Span::styled(format!("{:.7}", target), accent_style().add_modifier(Modifier::BOLD)),
                Span::raw(" ("),
                Span::styled(summary, primary_style()),
                Span::raw(")?"),
            ];
            let entries = vec![
                StatusEntry::new(vec![
                    Span::raw("Confirm Revert"),
                    Span::raw(" "),
                    Span::styled("[", muted_style()),
                    Span::styled("y", accent_style()),
                    Span::styled("]", muted_style()),
                ]),
                StatusEntry::new(vec![
                    Span::styled(" ", muted_style()),
                    Span::raw("Cancel"),
                    Span::raw(" "),
                    Span::styled("[", muted_style()),
                    Span::styled("n/⎋", accent_style()),
                    Span::styled("]", muted_style()),
                ]),
            ];
            draw_status_layout(f, area, Some(msg_spans), entries, app);
        }
        Mode::RemotePicker => {
            let (msg_spans, entries) = remote_picker_status_entries();
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::SearchColumnPicker => {
            let msg_spans = vec![
                Span::styled(
                    "Search Columns  ",
                    Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
                ),
                Span::styled("Choose columns to apply search on  ", muted_style()),
            ];
            let entries_data =
                [("Toggle", "Space"), ("Confirm & Search", "Enter"), ("Cancel", "Esc")];
            let mut entries = Vec::new();
            for (i, (label, key)) in entries_data.iter().enumerate() {
                let mut spans = Vec::new();
                if i > 0 {
                    spans.push(Span::styled(" ", muted_style()));
                }
                spans.push(Span::raw((*label).to_string()));
                spans.push(Span::raw(" "));
                spans.push(Span::styled("[", muted_style()));
                spans.push(Span::styled((*key).to_string(), accent_style()));
                spans.push(Span::styled("]", muted_style()));
                entries.push(StatusEntry::new(spans));
            }
            draw_status_layout(f, area, Some(msg_spans), entries, app);
        }
        Mode::LogsSearchInput => {
            draw_input_status(f, area, "Search Logs", &app.input_buffer);
        }
        Mode::Logs => {
            let msg_spans = vec![
                Span::styled(
                    "Logs UI  ",
                    Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Use arrow keys / PgUp / PgDn / Home / End to navigate commits  ",
                    muted_style(),
                ),
            ];
            let entries_data =
                [("Inspect", "Enter"), ("Search / Columns", "f"), ("Back to Workspace", "Esc/q")];
            let mut entries = Vec::new();
            for (i, (label, key)) in entries_data.iter().enumerate() {
                let mut spans = Vec::new();
                if i > 0 {
                    spans.push(Span::styled(" ", muted_style()));
                }
                spans.push(Span::raw((*label).to_string()));
                spans.push(Span::raw(" "));
                spans.push(Span::styled("[", muted_style()));
                spans.push(Span::styled((*key).to_string(), accent_style()));
                spans.push(Span::styled("]", muted_style()));
                entries.push(StatusEntry::new(spans));
            }
            draw_status_layout(f, area, Some(msg_spans), entries, app);
        }
        Mode::CommitSearchInput => {
            draw_input_status(f, area, "Search Commits", &app.input_buffer);
        }
        Mode::DiscardChangesConfirm => {
            let (target, staged) = app
                .discard_target
                .as_ref()
                .map(|(name, staged)| (name.as_str(), *staged))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_discard_changes_entries(target, staged);
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::MergeAbortConfirm => {
            let msg_spans = vec![Span::styled(
                "Abort Merge?  ",
                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
            )];
            let entries = vec![
                StatusEntry::new(vec![
                    Span::raw("Confirm"),
                    Span::raw(" "),
                    Span::styled("[", muted_style()),
                    Span::styled("y", accent_style()),
                    Span::styled("]", muted_style()),
                ]),
                StatusEntry::new(vec![
                    Span::styled(" ", muted_style()),
                    Span::raw("Cancel"),
                    Span::raw(" "),
                    Span::styled("[", muted_style()),
                    Span::styled("n/⎋", accent_style()),
                    Span::styled("]", muted_style()),
                ]),
            ];
            draw_status_layout(f, area, Some(msg_spans), entries, app);
        }
        Mode::MergeContinueConfirm => {
            let msg_spans = vec![
                Span::styled(
                    "Continue Merge  ",
                    Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
                ),
                Span::styled("Are you sure you want to continue the merge?  ", primary_style()),
            ];
            let entries_data = [("Confirm Continue", "y"), ("Cancel", "n/Esc")];
            let mut entries = Vec::new();
            for (i, (label, key)) in entries_data.iter().enumerate() {
                let mut spans = Vec::new();
                if i > 0 {
                    spans.push(Span::styled(" ", muted_style()));
                }
                spans.push(Span::raw((*label).to_string()));
                spans.push(Span::raw(" "));
                spans.push(Span::styled("[", muted_style()));
                spans.push(Span::styled((*key).to_string(), accent_style()));
                spans.push(Span::styled("]", muted_style()));
                entries.push(StatusEntry::new(spans));
            }
            draw_status_layout(f, area, Some(msg_spans), entries, app);
        }
        Mode::Inspect => {
            let (msg_spans, entries) = inspect_dismiss_entries(app);
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::DebugLogs => {
            let msg_spans = vec![Span::styled(
                "Debug Logs  ",
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            )];
            let entries_data = [("Back", "Esc/q/l")];
            let mut entries = Vec::new();
            for (i, (label, key)) in entries_data.iter().enumerate() {
                let mut spans = Vec::new();
                if i > 0 {
                    spans.push(Span::styled(" ", muted_style()));
                }
                spans.push(Span::raw((*label).to_string()));
                spans.push(Span::raw(" "));
                spans.push(Span::styled("[", muted_style()));
                spans.push(Span::styled((*key).to_string(), accent_style()));
                spans.push(Span::styled("]", muted_style()));
                entries.push(StatusEntry::new(spans));
            }
            draw_status_layout(f, area, Some(msg_spans), entries, app);
        }
        Mode::RemoteDeleteConfirm => {
            let target = app.remote_action_target.as_deref().unwrap_or("");
            let (msg_spans, entries) = confirm_remote_delete_entries(target);
            draw_status_layout(f, area, msg_spans, entries, app);
        }
    }
}

pub(crate) fn detail_dismiss_entries(app: &App) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let mut message_spans = None;
    if let Some(msg) = &app.status_message {
        message_spans = Some(vec![Span::styled(format!("{} ", msg), accent_style())]);
    }

    let mut entries = Vec::new();
    let entries_data = match app.detail_tab {
        0 => {
            let mut v = vec![("Home", "⎋/q"), ("Tabs", "Tab/1-8"), ("Cycle Focus", "w/W")];
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
                v.push(("Tag", "t"));
                v.push(("Interactive Rebase", "i"));
                v.push(("Cherry-pick", "p"));
                v.push(("Revert", "v"));
                v.push(("Search/Columns", "f"));
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
                ("Tabs", "Tab/1-8"),
                ("Cycle Focus", "w/W"),
                ("Navigate/Scroll", "↑↓"),
                ("Page", "⇟/⇞"),
                ("Jump", "Home/End"),
            ];
            if app.detail_focus == DetailSection::Files {
                v.push(("Expand/Collapse", "←/→"));
                v.push(("Fuzzy Find", "f"));
            } else if app.detail_focus == DetailSection::FileContent {
                if app.inspect_full_diff {
                    v.push(("Exit Full Screen", "←/⎋/q"));
                } else {
                    v.push(("Full Screen", "→"));
                }
            }
            v.push(("Resync", "R"));
            v.push(("Help", "?"));
            v
        }
        2 => vec![
            ("Home", "⎋/q"),
            ("Tabs", "Tab/1-8"),
            ("Scroll", "↑↓"),
            ("Page", "⇟/⇞"),
            ("Jump", "Home/End"),
            ("Resync", "R"),
            ("Help", "?"),
        ],
        3 => {
            let mut v = vec![
                ("Home", "⎋/q"),
                ("Tabs", "Tab/1-8"),
                ("Cycle Focus", "w/W"),
                ("Checkout", "↵"),
                ("Create", "c"),
                ("Delete", "d"),
                ("Merge", "m"),
                ("Rebase", "r"),
                ("Interactive Rebase", "i"),
            ];
            if app.detail_focus == DetailSection::LocalBranches {
                v.push(("Fetch", "⇧F"));
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
            ("Tabs", "Tab/1-8"),
            ("Checkout", "↵"),
            ("Navigate", "↑↓"),
            ("Page", "⇟/⇞"),
            ("Jump", "Home/End"),
            ("Fetch", "f"),
            ("Push", "p"),
            ("Push All", "⇧P"),
            ("Delete", "d"),
            ("Resync", "R"),
            ("Help", "?"),
        ],
        5 => vec![
            ("Home", "⎋/q"),
            ("Tabs", "Tab/1-8"),
            ("Navigate", "↑↓"),
            ("Page", "⇟/⇞"),
            ("Jump", "Home/End"),
            ("Fetch", "f/F"),
            ("Add", "a/A"),
            ("Delete", "d/D"),
            ("Resync", "R"),
            ("Help", "?"),
        ],
        6 => {
            let mut v = vec![
                ("Home", "⎋/q"),
                ("Tabs", "Tab/1-8"),
                ("Cycle Focus", "w/W"),
                ("Navigate", "↑↓"),
                ("Page", "⇟/⇞"),
                ("Jump", "Home/End"),
            ];
            if app.detail_focus == DetailSection::Stashes {
                v.push(("Apply", "a"));
                v.push(("Delete", "d"));
                v.push(("Stash New", "s"));
            }
            v.push(("Resync", "R"));
            v.push(("Help", "?"));
            v
        }
        7 => vec![("Home", "⎋/q"), ("Tabs", "Tab/1-8"), ("Resync", "R"), ("Help", "?")],
        _ => vec![("Home", "⎋/q"), ("Tabs", "Tab/1-8"), ("Resync", "R"), ("Help", "?")],
    };
    for (i, (label, key)) in entries_data.iter().enumerate() {
        let mut spans = Vec::new();
        if i > 0 {
            spans.push(Span::styled(" ", muted_style()));
        }
        spans.push(Span::raw((*label).to_string()));
        spans.push(Span::raw(" "));
        spans.push(Span::styled("[", muted_style()));
        spans.push(Span::styled((*key).to_string(), accent_style()));
        spans.push(Span::styled("]", muted_style()));
        entries.push(StatusEntry::new(spans));
    }
    (message_spans, entries)
}

pub(crate) fn inspect_dismiss_entries(app: &App) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let mut message_spans = None;
    if let Some(msg) = &app.status_message {
        message_spans = Some(vec![Span::styled(format!("{} ", msg), accent_style())]);
    }

    let mut entries = Vec::new();
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

    for (i, (label, key)) in entries_data.iter().enumerate() {
        let mut spans = Vec::new();
        if i > 0 {
            spans.push(Span::styled(" ", muted_style()));
        }
        spans.push(Span::raw((*label).to_string()));
        spans.push(Span::raw(" "));
        spans.push(Span::styled("[", muted_style()));
        spans.push(Span::styled((*key).to_string(), accent_style()));
        spans.push(Span::styled("]", muted_style()));
        entries.push(StatusEntry::new(spans));
    }
    (message_spans, entries)
}

fn detail_help_entries() -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let entries = vec![StatusEntry::new(vec![
        Span::raw("Close Help"),
        Span::raw(" "),
        Span::styled("[", muted_style()),
        Span::styled("?/⎋/q", accent_style()),
        Span::styled("]", muted_style()),
    ])];
    (None, entries)
}

fn commit_input_editing_entries() -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Done Editing"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("⌃C", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Toggle Amend"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("⌃A", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Newline"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("↵", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel Commit"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Max Size"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("⌃D", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Scroll"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("↑/↓", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (None, entries)
}

fn commit_input_confirm_entries(
    commit_amend: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let amend_toggle_label = if commit_amend { "Amend: [Yes]" } else { "Amend: [No]" };
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Submit Commit"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("↵", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw(amend_toggle_label),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("a/space", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Edit Message"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("e", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("⎋/q", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Max Size"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("d", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Scroll"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("↑/↓", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (None, entries)
}

fn draw_status_layout(
    f: &mut Frame,
    area: Rect,
    message_spans: Option<Vec<Span<'static>>>,
    entries: Vec<StatusEntry>,
    app: &App,
) {
    let is_merging =
        if let Some(crate::repo::ItemDetail::Repo { resolved, .. }) = &app.current_detail {
            crate::repo::is_merging(resolved)
        } else if let Some(selected_item) = app.get_selected_item() {
            let path = crate::repo::expand_tilde(selected_item);
            crate::repo::is_merging(&path)
        } else {
            false
        };

    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(25)])
        .split(area);

    let left_area = status_chunks[0];
    let right_area = status_chunks[1];

    let mut spans = Vec::new();
    if is_merging {
        spans.push(Span::styled(
            "[ ⚡ MERGING ] ",
            Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD),
        ));
    } else {
        spans.push(Span::raw(" "));
    }

    let mut initial_width = if is_merging { 14 } else { 1 };
    if let Some(ref msg) = message_spans {
        spans.extend(msg.clone());
        initial_width += msg.iter().map(|s| s.content.chars().count()).sum::<usize>();
    }

    let max_width = left_area.width as usize;

    if app.status_expanded {
        for entry in entries {
            spans.extend(entry.spans);
        }
        spans.push(Span::styled(" ", muted_style()));
        spans.push(Span::raw("Less"));
        spans.push(Span::raw(" "));
        spans.push(Span::styled("[", muted_style()));
        spans.push(Span::styled(".", accent_style()));
        spans.push(Span::styled("]", muted_style()));

        let para = Paragraph::new(Line::from(spans)).wrap(Wrap { trim: true });
        f.render_widget(para, left_area);
    } else {
        // Need to truncate whole entries. Leave space for " More [.]" which is 9 chars plus 2 safe buffer.
        let limit = max_width.saturating_sub(11);

        let mut fitted_entries = Vec::new();
        let mut current_width = initial_width;
        let mut truncated = false;

        for entry in entries {
            let w = entry.width();
            if current_width + w <= limit {
                current_width += w;
                fitted_entries.push(entry);
            } else {
                truncated = true;
                break;
            }
        }

        for entry in fitted_entries {
            spans.extend(entry.spans);
        }

        if truncated {
            spans.push(Span::styled(" ", muted_style()));
            spans.push(Span::raw("More"));
            spans.push(Span::raw(" "));
            spans.push(Span::styled("[", muted_style()));
            spans.push(Span::styled(".", accent_style()));
            spans.push(Span::styled("]", muted_style()));
        }

        let para = Paragraph::new(Line::from(spans));
        f.render_widget(para, left_area);
    }

    // Render CPU & Memory Stats on the right
    let (rss_mb, cpu_pct) = get_process_stats(app);
    let stats_text = if rss_mb > 0.0 {
        format!(" MEM: {:.1}MB | CPU: {:.1}% ", rss_mb, cpu_pct)
    } else {
        "".to_string()
    };
    if !stats_text.is_empty() {
        let stats_line =
            Line::from(vec![Span::styled(stats_text, muted_style())]).alignment(Alignment::Right);
        f.render_widget(Paragraph::new(stats_line), right_area);
    }
}

fn normal_status_entries(app: &App) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let mut message_spans = None;
    if let Some(msg) = &app.status_message {
        message_spans = Some(vec![Span::styled(format!("{} ", msg), accent_style())]);
    } else if let Some(query) = &app.repo_search_query {
        message_spans = Some(vec![
            Span::styled("Filtered by: ", muted_style()),
            Span::styled(format!("\"{}\" ", query), accent_style()),
            Span::styled("(Esc to clear) ", muted_style()),
        ]);
    }
    let sort_label = match app.config.sort_by {
        SortOrder::Custom => "Custom",
        SortOrder::Alphabetical => "Alphabetical",
        SortOrder::RecentVisit => "Recent",
        SortOrder::LatestChanges => "Changes",
    };
    let sort_dir = if app.config.sort_reverse { " (Rev)" } else { "" };
    let sort_key_label = format!("Sort: {}{}", sort_label, sort_dir);

    let entries_data = vec![
        ("Navigate", "↑↓"),
        ("Page", "⇟/⇞"),
        ("Jump", "Home/End"),
        ("Detail", "↵/→"),
        (&app.config.git_app, "g"),
        (&sort_key_label, "o/O"),
        ("Find", "f"),
        ("Add", "a"),
        ("Bulk Add", "A"),
        ("Import", "i"),
        ("Edit", "e"),
        ("Delete", "d"),
        ("Refresh", "R"),
        ("Pin", "p"),
        ("Debug Logs", "l"),
        ("About", "v"),
        ("Help", "?"),
        ("Quit", "⎋/q"),
    ];
    let mut entries = Vec::new();
    for (i, (label, key)) in entries_data.iter().enumerate() {
        let mut spans = Vec::new();
        if i > 0 {
            spans.push(Span::styled(" ", muted_style()));
        }
        spans.push(Span::raw((*label).to_string()));
        spans.push(Span::raw(" "));
        spans.push(Span::styled("[", muted_style()));
        spans.push(Span::styled((*key).to_string(), accent_style()));
        spans.push(Span::styled("]", muted_style()));
        entries.push(StatusEntry::new(spans));
    }
    (message_spans, entries)
}

fn confirm_delete_entries(target: &str) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![
        Span::raw("Delete "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("y", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_branch_delete_entries(
    target: &str,
    is_remote: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let type_label = if is_remote { "remote-tracking branch" } else { "branch" };
    let message_spans = Some(vec![
        Span::raw("Delete "),
        Span::raw(type_label),
        Span::raw(" "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("y", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_remote_delete_entries(target: &str) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![
        Span::raw("Remove remote "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("y", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_branch_checkout_entries(
    target: &str,
    is_remote: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let type_label = if is_remote { "remote-tracking branch" } else { "branch" };
    let message_spans = Some(vec![
        Span::raw("Checkout "),
        Span::raw(type_label),
        Span::raw(" "),
        Span::styled(format!("\"{}\"", target), accent_style().add_modifier(Modifier::BOLD)),
        Span::raw("? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_tag_checkout_entries(target: &str) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![
        Span::raw("Checkout tag "),
        Span::styled(format!("\"{}\"", target), accent_style().add_modifier(Modifier::BOLD)),
        Span::raw(" (detached HEAD)? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("y", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_discard_changes_entries(
    target: &str,
    staged: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = if target == "All Changes" {
        Some(vec![
            Span::raw("Discard "),
            Span::styled("ALL", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
            Span::raw(" changes in the repository? "),
        ])
    } else {
        let area_label = if staged { "staged" } else { "unstaged" };
        Some(vec![
            Span::raw("Discard "),
            Span::raw(area_label),
            Span::raw(" changes in "),
            Span::styled(
                format!("\"{}\"", target),
                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
            ),
            Span::raw("? "),
        ])
    };
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("y", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_branch_merge_entries(
    target: &str,
    is_remote: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let type_label = if is_remote { "remote-tracking branch" } else { "branch" };
    let message_spans = Some(vec![
        Span::raw("Merge "),
        Span::raw(type_label),
        Span::raw(" "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" into current branch? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("y", Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_branch_rebase_entries(
    target: &str,
    is_remote: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let type_label = if is_remote { "remote-tracking branch" } else { "branch" };
    let message_spans = Some(vec![
        Span::raw("Rebase current branch onto "),
        Span::raw(type_label),
        Span::raw(" "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("y", Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_branch_interactive_rebase_entries(
    target: &str,
    is_remote: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let type_label = if is_remote { "remote-tracking branch" } else { "branch" };
    let message_spans = Some(vec![
        Span::raw("Interactively rebase current branch onto "),
        Span::raw(type_label),
        Span::raw(" "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("y", Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_stash_delete_entries(target: &str) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![
        Span::raw("Delete stash "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("y", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_stash_apply_entries(
    target: &str,
    delete_after: bool,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![
        Span::raw("Apply stash "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let delete_toggle_label =
        if delete_after { "Delete after apply: [Yes]" } else { "Delete after apply: [No]" };
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("y", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw(delete_toggle_label.to_string()),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("d/space/a", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn remote_picker_status_entries() -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![Span::raw("Select a remote to use for this operation")]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Navigate"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("↑↓", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::raw(" Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("⏎", accent_style()),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::raw(" Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn confirm_branch_push_entries(target: &str) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![
        Span::raw("Push branch "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("? "),
    ]);
    let entries = vec![
        StatusEntry::new(vec![
            Span::raw("Confirm"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("y", Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
            Span::styled("]", muted_style()),
        ]),
        StatusEntry::new(vec![
            Span::styled(" ", muted_style()),
            Span::raw("Cancel"),
            Span::raw(" "),
            Span::styled("[", muted_style()),
            Span::styled("n/⎋", accent_style()),
            Span::styled("]", muted_style()),
        ]),
    ];
    (message_spans, entries)
}

fn help_dismiss_entries() -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let entries = vec![StatusEntry::new(vec![
        Span::raw("Close Help"),
        Span::raw(" "),
        Span::styled("[", muted_style()),
        Span::styled("?/⎋/q", accent_style()),
        Span::styled("]", muted_style()),
    ])];
    (None, entries)
}

fn about_dismiss_entries() -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let entries = vec![StatusEntry::new(vec![
        Span::raw("Close About"),
        Span::raw(" "),
        Span::styled("[", muted_style()),
        Span::styled("v/⎋/q", accent_style()),
        Span::styled("]", muted_style()),
    ])];
    (None, entries)
}
