use crate::app::{App, DetailSection, Mode};
use crate::config::SortOrder;
use crate::ui::style::{
    ACCENT, CARD_BORDER, DANGER, SUCCESS, accent_style, muted_style, primary_style,
};
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
            draw_input_status(f, area, "Add", &app.input_buffer, app.config.compatibility_mode);
        }
        Mode::BulkAddInput => {
            draw_input_status(
                f,
                area,
                "Bulk Add (Tab for FZF)",
                &app.input_buffer,
                app.config.compatibility_mode,
            );
        }
        Mode::Editing => {
            draw_input_status(f, area, "Edit", &app.input_buffer, app.config.compatibility_mode);
        }
        Mode::LabelInput => {
            draw_input_status(f, area, "Labels", &app.input_buffer, app.config.compatibility_mode);
        }
        Mode::RepoSearchInput => {
            draw_input_status(f, area, "Find", &app.input_buffer, app.config.compatibility_mode);
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
        Mode::Legend => {
            let (msg_spans, entries) = legend_dismiss_entries();
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::RepoSettings => {
            let msg_spans = vec![
                Span::styled(
                    "Repository Settings  ",
                    Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Use arrow keys / j / k to select, Enter / Space / Left / Right to edit",
                    muted_style(),
                ),
            ];
            let entries_data = if app.repo_settings_editing {
                vec![("Confirm", "Enter"), ("Cancel", "Esc")]
            } else {
                vec![("Close", "Esc/q")]
            };
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
        Mode::Detail | Mode::RemoteAddNameInput | Mode::RemoteAddUrlInput => {
            let (msg_spans, entries) = detail_dismiss_entries(app);
            draw_status_layout(f, area, msg_spans, entries, app);
        }
        Mode::Overview => {
            let mut entries = Vec::new();
            let entries_data = [("Close Overview", "Esc/q/v"), ("Repo Settings", "s")];
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
            draw_status_layout(f, area, None, entries, app);
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
            draw_input_status(
                f,
                area,
                "Create Branch",
                &app.input_buffer,
                app.config.compatibility_mode,
            );
        }
        Mode::TagCreateInput => {
            draw_input_status(
                f,
                area,
                "Create Tag",
                &app.input_buffer,
                app.config.compatibility_mode,
            );
        }
        Mode::StashCreateInput => {
            draw_input_status(
                f,
                area,
                "Stash Changes",
                &app.input_buffer,
                app.config.compatibility_mode,
            );
        }
        Mode::WorktreeAddBranchInput => {
            draw_input_status(
                f,
                area,
                "Add Worktree (Branch/Commit)",
                &app.input_buffer,
                app.config.compatibility_mode,
            );
        }
        Mode::WorktreeAddPathInput => {
            draw_input_status(
                f,
                area,
                "Add Worktree (Path)",
                &app.input_buffer,
                app.config.compatibility_mode,
            );
        }
        Mode::WorktreeLockReasonInput => {
            draw_input_status(
                f,
                area,
                "Lock Worktree (Reason)",
                &app.input_buffer,
                app.config.compatibility_mode,
            );
        }
        Mode::WorktreeRemoveConfirm => {
            draw_input_status(
                f,
                area,
                "Remove Worktree (1: Metadata only, 2: Delete folder & metadata)",
                &app.input_buffer,
                app.config.compatibility_mode,
            );
        }
        Mode::SubmoduleAddUrlInput => {
            draw_input_status(
                f,
                area,
                "Add Submodule (URL)",
                &app.input_buffer,
                app.config.compatibility_mode,
            );
        }
        Mode::SubmoduleAddPathInput => {
            draw_input_status(
                f,
                area,
                "Add Submodule (Path)",
                &app.input_buffer,
                app.config.compatibility_mode,
            );
        }
        Mode::StashingUI => {
            let mut entries = Vec::new();
            let entries_data = [
                ("Cancel", "⎋/q"),
                ("Save Stash", "s"),
                ("Toggle Untracked", "u"),
                ("Toggle Keep Index", "i"),
                ("Navigate", "↑↓"),
            ];
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
            draw_status_layout(f, area, None, entries, app);
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
        Mode::SubmoduleDeleteConfirm => {
            let target = app.submodule_delete_target.as_deref().unwrap_or("");
            let (msg_spans, entries) = confirm_submodule_delete_entries(target);
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
            draw_input_status(
                f,
                area,
                "Search Logs",
                &app.input_buffer,
                app.config.compatibility_mode,
            );
        }
        Mode::Logs => {
            let msg_spans = vec![
                Span::styled(
                    "Logs UI  ",
                    Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Use arrow keys / j / k / PgUp / PgDn / Home / End to navigate commits  ",
                    muted_style(),
                ),
            ];
            let entries_data = [
                ("Inspect", "Enter"),
                ("Search / Columns", "f"),
                ("Load More", "G"),
                ("Back to Workspace", "Esc/q"),
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
            draw_status_layout(f, area, Some(msg_spans), entries, app);
        }
        Mode::CommitSearchInput => {
            draw_input_status(
                f,
                area,
                "Search Commits",
                &app.input_buffer,
                app.config.compatibility_mode,
            );
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
        Mode::FileHistory => {
            let msg_spans = vec![Span::styled(
                "File History  ",
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            )];
            let entries_data = if app.file_history_focus == 0 {
                vec![("Back", "Esc/q"), ("Navigate Revisions", "↑↓"), ("Focus Diff", "Tab/w/→")]
            } else {
                vec![
                    ("Back", "Esc/q"),
                    ("Scroll Diff", "↑↓/PgUp/PgDn"),
                    ("Focus Revisions", "Tab/w/←"),
                ]
            };
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
        Mode::UpdateConfirm => {
            let msg_spans = vec![Span::styled(
                "Update Available  ",
                Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
            )];
            let entries_data = [("Confirm Update", "y"), ("Cancel", "n/Esc")];
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
        Mode::RepoJump => {
            let msg_spans = vec![
                Span::raw("Jump to repository: type query to search, select, then press "),
                Span::styled("Enter", accent_style()),
            ];
            let entries_data = [("Select Match", "↑/↓"), ("Confirm", "Enter"), ("Cancel", "Esc")];
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
                v.push(("Tag", "t"));
                v.push(("Interactive Rebase", "i"));
                v.push(("Cherry-pick", "p"));
                v.push(("Revert", "v"));
                v.push(("Search/Columns", "f"));
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
                v.push(("Fuzzy Find", "f"));
                v.push(("History", "⇧H"));
            } else if app.detail_focus == DetailSection::FileContent {
                if app.inspect_full_diff {
                    v.push(("Exit Full Screen", "←/⎋/q"));
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
                v.push(("Fetch", "f/F"));
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
            ("Fetch", "f/F"),
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
            ("Tabs", "Tab/1-9"),
            ("Navigate", "↑↓"),
            ("Add", "a"),
            ("Delete", "D"),
            ("Resync", "R"),
            ("Help", "?"),
        ],
        _ => vec![("Home", "⎋/q"), ("Tabs", "Tab/1-9"), ("Resync", "R"), ("Help", "?")],
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

fn get_mode_badge(mode: &Mode) -> Span<'static> {
    let (label, color) = match mode {
        Mode::RepoJump => ("JUMP", Color::Red),
        Mode::Normal => ("NORMAL", Color::Blue),
        Mode::Detail => ("DETAIL", Color::Magenta),
        Mode::Overview => ("OVERVIEW", Color::Magenta),
        Mode::Inspect => ("INSPECT", Color::Rgb(175, 95, 0)),
        Mode::FileHistory => ("HISTORY", Color::Rgb(0, 135, 175)),
        Mode::StashingUI => ("STASH", ACCENT()),
        Mode::Settings => ("SETTINGS", Color::Green),
        Mode::Help | Mode::DetailHelp => ("HELP", Color::Rgb(150, 150, 150)),
        Mode::About => ("ABOUT", Color::Rgb(150, 150, 150)),
        Mode::Legend => ("LEGEND", Color::Rgb(150, 150, 150)),
        Mode::RepoSettings => ("REPO SETTINGS", Color::Rgb(135, 0, 135)),
        Mode::Adding
        | Mode::BulkAddInput
        | Mode::Editing
        | Mode::RepoSearchInput
        | Mode::ImportUrlInput
        | Mode::ImportDestInput
        | Mode::ImportNameInput
        | Mode::RemoteAddNameInput
        | Mode::RemoteAddUrlInput
        | Mode::BranchCreateInput
        | Mode::TagCreateInput
        | Mode::StashCreateInput
        | Mode::LogsSearchInput => ("INPUT", Color::Red),
        Mode::ConfirmDelete
        | Mode::BranchDeleteConfirm
        | Mode::BranchCheckoutConfirm
        | Mode::TagCheckoutConfirm
        | Mode::BranchPushConfirm
        | Mode::BranchMergeConfirm
        | Mode::BranchRebaseConfirm
        | Mode::BranchInteractiveRebaseConfirm
        | Mode::TagDeleteConfirm
        | Mode::TagPushConfirm
        | Mode::TagPushAllConfirm
        | Mode::StashDeleteConfirm
        | Mode::StashApplyConfirm
        | Mode::CherryPickConfirm
        | Mode::RevertConfirm
        | Mode::MergeAbortConfirm
        | Mode::MergeContinueConfirm => ("CONFIRM", Color::Rgb(135, 0, 135)),
        _ => ("NORMAL", Color::Blue),
    };

    Span::styled(label, Style::default().fg(color).add_modifier(Modifier::BOLD))
}

fn extend_spans_with_separator(
    spans: &mut Vec<Span<'static>>,
    entries: Vec<StatusEntry>,
    is_compat: bool,
) {
    let separator = if is_compat { " > " } else { " ⟩ " };
    let mut first = true;
    for entry in entries {
        if !first {
            spans.push(Span::styled(separator, muted_style()));
        }
        first = false;
        let mut start = 0;
        if !entry.spans.is_empty() && entry.spans[0].content.trim().is_empty() {
            start = 1;
        }
        for span in &entry.spans[start..] {
            spans.push(span.clone());
        }
    }
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
        .constraints([Constraint::Min(1), Constraint::Length(28)])
        .split(area);

    let left_area = status_chunks[0];
    let right_area = status_chunks[1];

    let mut spans = Vec::new();

    // Add Mode Badge
    let badge = get_mode_badge(&app.mode);
    let badge_len = badge.content.chars().count();
    let mode_sep = if app.config.compatibility_mode { " > " } else { " ⟩ " };

    spans.push(badge);
    spans.push(Span::styled(mode_sep, muted_style()));

    let mut initial_width = badge_len + 3;
    if is_merging {
        spans.push(Span::styled(
            "[ ⚡ MERGING ] ",
            Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD),
        ));
        initial_width += 14;
    }

    if let Some(ref msg) = message_spans {
        spans.extend(msg.clone());
        initial_width += msg.iter().map(|s| s.content.chars().count()).sum::<usize>();
    }

    let max_width = left_area.width as usize;

    if app.status_expanded {
        extend_spans_with_separator(&mut spans, entries, app.config.compatibility_mode);
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
        let sep_width = 3;
        let mut first = true;

        for entry in entries {
            let mut w = entry.width();
            if !entry.spans.is_empty() && entry.spans[0].content.trim().is_empty() {
                w = w.saturating_sub(entry.spans[0].content.chars().count());
            }
            let increment = if first { w } else { w + sep_width };
            if current_width + increment <= limit {
                current_width += increment;
                fitted_entries.push(entry);
                first = false;
            } else {
                truncated = true;
                break;
            }
        }

        extend_spans_with_separator(&mut spans, fitted_entries, app.config.compatibility_mode);

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
        format!(" mem: {:.1}mb │ cpu: {:.1}% ", rss_mb, cpu_pct)
    } else {
        "".to_string()
    };
    if !stats_text.is_empty() {
        if right_area.height >= 3 {
            let block = ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .border_style(muted_style())
                .border_type(CARD_BORDER());
            let inner = block.inner(right_area);
            f.render_widget(block, right_area);

            let stats_line = Line::from(vec![Span::styled(stats_text.trim(), muted_style())])
                .alignment(Alignment::Center);
            f.render_widget(Paragraph::new(stats_line), inner);
        } else {
            let stats_line = Line::from(vec![
                Span::styled("│", muted_style()),
                Span::styled(stats_text, muted_style()),
                Span::styled("│", muted_style()),
            ])
            .alignment(Alignment::Right);
            f.render_widget(Paragraph::new(stats_line), right_area);
        }
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
        ("Delete", "D"),
        ("Labels", "l"),
        ("Refresh", "R"),
        ("Pin", "p"),
        ("Star", "*"),
        ("Check Update", "u"),
        ("Debug Logs", "d"),
        ("About", "V"),
        ("Compact", "v"),
        ("Legend", "h"),
        ("Help", "?"),
        ("Quit", "ctrl+q"),
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
            Span::styled("n/⎋/↵", accent_style()),
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

fn legend_dismiss_entries() -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let entries = vec![StatusEntry::new(vec![
        Span::raw("Close Legend"),
        Span::raw(" "),
        Span::styled("[", muted_style()),
        Span::styled("h/⎋/q", accent_style()),
        Span::styled("]", muted_style()),
    ])];
    (None, entries)
}

fn confirm_submodule_delete_entries(
    target: &str,
) -> (Option<Vec<Span<'static>>>, Vec<StatusEntry>) {
    let message_spans = Some(vec![
        Span::raw("Delete submodule "),
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
