//! Dynamic status bar rendering active Mode context shortcuts and status notifications.

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

pub(crate) fn build_status_entries(entries_data: &[(&str, &str)]) -> Vec<StatusEntry> {
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
}

/// Compact caption for an action's keys in the status bar: every bound key,
/// with arrow, paging and Enter/Esc/Tab names collapsed to the glyphs the bar
/// already uses for its fixed entries (`→`, `⇞`, `↵`, ...). Keys come from the
/// live keybindings, so a rebinding never leaves the bar lying.
pub(crate) fn compact_action_keys(
    kb: &crate::keybindings::KeybindingsConfig,
    action: crate::keybindings::Action,
    compat: bool,
) -> String {
    let keys = kb.get_action_keys(action);
    if keys.is_empty() {
        return "-".to_string();
    }
    keys.iter()
        .map(|k| {
            let name = match (k.as_str(), compat) {
                ("up", false) => "↑",
                ("up", true) => "Up",
                ("down", false) => "↓",
                ("down", true) => "Down",
                ("left", false) => "←",
                ("left", true) => "Left",
                ("right", false) => "→",
                ("right", true) => "Right",
                ("pageup" | "pgup", false) => "⇞",
                ("pageup" | "pgup", true) => "PgUp",
                ("pagedown" | "pgdn", false) => "⇟",
                ("pagedown" | "pgdn", true) => "PgDn",
                ("home", _) => "Home",
                ("end", _) => "End",
                ("space", _) => "Space",
                ("delete" | "del", _) => "Del",
                ("backspace", false) => "⌫",
                ("backspace", true) => "Backspace",
                ("enter" | "return", false) => "↵",
                ("enter" | "return", true) => "Enter",
                ("esc" | "escape", false) => "⎋",
                ("esc" | "escape", true) => "Esc",
                ("tab", false) => "⇥",
                ("tab", true) => "Tab",
                ("backtab" | "shift-tab", false) => "⇧⇥",
                ("backtab" | "shift-tab", true) => "Shift+Tab",
                _ => k.as_str(),
            };
            name.to_string()
        })
        .collect::<Vec<_>>()
        .join("/")
}

mod detail;
mod main;
mod popups;

pub(crate) use detail::{detail_dismiss_entries, detail_help_entries, inspect_dismiss_entries};
pub(crate) use main::normal_status_entries;
pub(crate) use popups::{
    about_dismiss_entries, commit_input_confirm_entries, commit_input_editing_entries,
    confirm_branch_checkout_entries, confirm_branch_delete_entries,
    confirm_branch_interactive_rebase_entries, confirm_branch_merge_entries,
    confirm_branch_merge_into_entries, confirm_branch_push_entries, confirm_branch_rebase_entries,
    confirm_commit_checkout_entries, confirm_delete_entries, confirm_discard_changes_entries,
    confirm_remote_delete_entries, confirm_stash_apply_entries, confirm_stash_delete_entries,
    confirm_submodule_delete_entries, confirm_tag_checkout_entries, help_dismiss_entries,
    legend_dismiss_entries, remote_picker_status_entries,
};

pub fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    // A focused terminal panel forwards nearly every key to the shell, so
    // show only the ways out instead of the current mode's entries.
    if app.terminal_focused {
        let toggle = app.keybindings.format_action_keys(
            crate::keybindings::Action::ToggleTerminalPanel,
            app.config.compatibility_mode,
        );
        let quit = app
            .keybindings
            .format_action_keys(crate::keybindings::Action::Close, app.config.compatibility_mode);
        let entries_data = [
            ("Hide Terminal", toggle.as_str()),
            ("Scrollback", "Shift+PgUp/PgDn"),
            ("Quit", quit.as_str()),
        ];
        let entries = build_status_entries(&entries_data);
        draw_status_layout(f, area, None, entries, app);
        return;
    }

    if app.command_palette.is_some() {
        let msg_spans = vec![Span::styled(
            "Command palette: type to filter, then run the highlighted action  ",
            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
        )];
        // Esc is literal in the palette handler; the palette key itself also
        // closes it (src/popups/command_palette.rs).
        let palette_key = app.keybindings.format_action_keys(
            crate::keybindings::Action::CommandPalette,
            app.config.compatibility_mode,
        );
        let cancel_key = format!("Esc/{}", palette_key);
        let page_key = if app.config.compatibility_mode { "PgUp/PgDn" } else { "⇞/⇟" };
        let entries_data = [
            ("Select", "↑/↓"),
            ("Run", "Enter"),
            ("Cancel", cancel_key.as_str()),
            ("Page", page_key),
        ];
        let entries = build_status_entries(&entries_data);
        draw_status_layout(f, area, Some(msg_spans), entries, app);
        return;
    }

    if app.loading_repo_path.is_some() {
        let msg_spans = vec![Span::styled(
            "Loading Repository...  ",
            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
        )];
        let entries_data = [("Cancel", "Esc")];
        let entries = build_status_entries(&entries_data);
        draw_status_layout(f, area, Some(msg_spans), entries, app);
        return;
    }

    if let Some((msg_spans, entries)) = get_status_layout_components(app) {
        draw_status_layout(f, area, msg_spans, entries, app);
    } else {
        match &app.mode {
            Mode::Adding => {
                draw_input_status(
                    f,
                    area,
                    "Add",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::BulkAddInput => {
                draw_input_status(
                    f,
                    area,
                    "Bulk Add (Tab for Scan)",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::Editing => {
                draw_input_status(
                    f,
                    area,
                    "Edit",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::LabelInput => {
                draw_input_status(
                    f,
                    area,
                    "Labels",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::AddRepoLabelInput => {
                draw_input_status(
                    f,
                    area,
                    "Add Repository Labels (comma-separated, optional)",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::BulkAddRepoLabelInput => {
                draw_input_status(
                    f,
                    area,
                    "Bulk Add Repository Labels (comma-separated, optional)",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::CloneRepoLabelInput => {
                draw_input_status(
                    f,
                    area,
                    "Add Repository Labels (comma-separated, optional)",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::RepoSearchInput => {
                draw_input_status(
                    f,
                    area,
                    "Find",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::BranchCreateInput => {
                draw_input_status(
                    f,
                    area,
                    "Create Branch",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::TagCreateInput => {
                draw_input_status(
                    f,
                    area,
                    "Create Tag",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::RemoteAddNameInput => {
                draw_input_status(
                    f,
                    area,
                    "Add Remote (Name)",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::RemoteAddUrlInput => {
                draw_input_status(
                    f,
                    area,
                    "Add Remote (URL)",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::StashCreateInput => {
                draw_input_status(
                    f,
                    area,
                    "Stash Changes",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::WorktreeAddBranchInput => {
                draw_input_status(
                    f,
                    area,
                    "Add Worktree (Branch/Commit)",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::WorktreeAddPathInput => {
                draw_input_status(
                    f,
                    area,
                    "Add Worktree (Path)",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::WorktreeLockReasonInput => {
                draw_input_status(
                    f,
                    area,
                    "Lock Worktree (Reason)",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::WorktreeRemoveConfirm => {
                let name = app.worktree_remove_target.as_ref().map_or("", |wt| wt.name.as_str());
                let label = format!(
                    "Remove worktree '{}' and its folder? (1: only if clean, 2: force, discarding uncommitted changes)",
                    name
                );
                draw_input_status(
                    f,
                    area,
                    &label,
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::SubmoduleAddUrlInput => {
                draw_input_status(
                    f,
                    area,
                    "Add Submodule (URL)",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::SubmoduleAddPathInput => {
                draw_input_status(
                    f,
                    area,
                    "Add Submodule (Path)",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::LogsSearchInput => {
                draw_input_status(
                    f,
                    area,
                    "Search Logs",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            Mode::CommitSearchInput => {
                draw_input_status(
                    f,
                    area,
                    "Search Commits",
                    &app.input_buffer,
                    app.config.compatibility_mode,
                    app.input_cursor_clamped(),
                );
            }
            _ => {}
        }
    }
}

pub(crate) fn get_status_layout_components(
    app: &App,
) -> Option<(Option<Vec<Span<'static>>>, Vec<StatusEntry>)> {
    let comps = match &app.mode {
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
                // Esc only leaves the settings page from the sidebar; in the
                // fields panel it returns focus to the sidebar first, while
                // q / Q always go straight back (src/popups/settings.rs).
                let (back_label, back_key, home_entry) = if app.settings_focus_sidebar {
                    ("Back", "Esc/q", None)
                } else {
                    ("Sidebar", "Esc", Some(("Home", "q")))
                };
                let mut entries_data = vec![
                    ("Select", "↑/↓"),
                    ("Page", "⇟/⇞"),
                    ("Jump", "Home/End"),
                    ("Pane", "Tab/←/→"),
                    ("Category", "1-5"),
                    ("Edit/Toggle", "Enter/Space"),
                    (back_label, back_key),
                ];
                if let Some(home) = home_entry {
                    entries_data.push(home);
                }
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
            (Some(msg_spans), entries)
        }
        Mode::Normal => {
            let (msg_spans, entries) = normal_status_entries(app);
            (msg_spans, entries)
        }
        Mode::ImportUrlInput | Mode::ImportDestInput | Mode::ImportNameInput => {
            let msg_spans = vec![Span::styled(
                "Importing Remote Repository  ",
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            )];
            // Esc cancels only on the first (URL) step; on the later steps it
            // goes back to the previous prompt.
            let entries_data = if app.mode == Mode::ImportUrlInput {
                [("Next", "Enter"), ("Cancel", "Esc")]
            } else {
                [("Next", "Enter"), ("Back", "Esc")]
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
            (Some(msg_spans), entries)
        }
        Mode::ConfirmDelete => {
            let target = app.get_selected_item().map(|s| s.as_str()).unwrap_or("");
            let (msg_spans, entries) = confirm_delete_entries(target);
            (msg_spans, entries)
        }
        Mode::Help => {
            let (msg_spans, entries) = help_dismiss_entries(app);
            (msg_spans, entries)
        }
        Mode::About => {
            let (msg_spans, entries) = about_dismiss_entries(app);
            (msg_spans, entries)
        }
        Mode::Legend => {
            let (msg_spans, entries) = legend_dismiss_entries(app);
            (msg_spans, entries)
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
            (Some(msg_spans), entries)
        }
        Mode::LabelSettings => {
            let label = app.label_settings_target.as_deref().unwrap_or("");
            let msg_spans = vec![
                Span::styled(
                    format!("Label Settings: {}  ", label),
                    Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Use arrow keys / j / k to select, Enter / Space / Left / Right to edit",
                    muted_style(),
                ),
            ];
            let entries_data = if app.label_settings_editing {
                vec![("Confirm", "Enter"), ("Cancel", "Esc")]
            } else {
                vec![("Back", "Esc")]
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
            (Some(msg_spans), entries)
        }
        Mode::Detail => {
            let (msg_spans, entries) = detail_dismiss_entries(app);
            (msg_spans, entries)
        }
        Mode::Overview => {
            let mut entries = Vec::new();
            // The Overview handler (src/input.rs) matches Esc / q / Q, s / S
            // and Tab / w / W as literal key codes; only the Overview toggle
            // itself is a configurable action.
            let overview_key = app.keybindings.format_action_keys(
                crate::keybindings::Action::Overview,
                app.config.compatibility_mode,
            );
            let close_key = format!("Esc/q/{}", overview_key);
            let entries_data = [
                ("Close Overview", close_key.as_str()),
                ("Repo Settings", "s/S"),
                ("Cycle Focus", "Tab/w/W"),
                ("Scroll", "↑↓/k/j"),
                ("Page", "⇟/⇞"),
                ("Jump", "Home/End"),
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
            (None, entries)
        }
        Mode::DetailHelp => {
            let (msg_spans, entries) = detail_help_entries(app);
            (msg_spans, entries)
        }
        Mode::CommitInput => {
            let (msg_spans, entries) = if app.commit_popup.editing {
                commit_input_editing_entries()
            } else {
                commit_input_confirm_entries(app.commit_popup.amend)
            };
            (msg_spans, entries)
        }
        Mode::StashingUI => {
            let mut entries = Vec::new();
            // The stashing panel (src/input.rs, Mode::StashingUI) matches
            // these as literal key codes, upper- and lowercase alike.
            let entries_data = [
                ("Cancel", "⎋/q/Q"),
                ("Save Stash", "s/S"),
                ("Toggle Untracked", "u/U"),
                ("Toggle Keep Index", "i/I"),
                ("Navigate", "↑↓/j/k"),
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
            (None, entries)
        }
        Mode::BranchDeleteConfirm => {
            let (target, is_remote) = app
                .branch_action_target
                .as_ref()
                .map(|(name, remote)| (name.as_str(), *remote))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_branch_delete_entries(target, is_remote);
            (msg_spans, entries)
        }
        Mode::BranchCheckoutConfirm => {
            let (target, is_remote) = app
                .branch_action_target
                .as_ref()
                .map(|(name, remote)| (name.as_str(), *remote))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_branch_checkout_entries(target, is_remote);
            (msg_spans, entries)
        }
        Mode::TagCheckoutConfirm => {
            let target = app.tag_checkout_target.as_deref().unwrap_or("");
            let (msg_spans, entries) = confirm_tag_checkout_entries(target);
            (msg_spans, entries)
        }
        Mode::CommitCheckoutConfirm => {
            let target = app.commit_action_target_oid.as_deref().unwrap_or("");
            let (msg_spans, entries) = confirm_commit_checkout_entries(target);
            (msg_spans, entries)
        }
        Mode::BranchPushConfirm => {
            let target =
                app.branch_action_target.as_ref().map(|(name, _)| name.as_str()).unwrap_or("");
            let msg_spans = vec![
                Span::styled("Push branch '", primary_style()),
                Span::styled(target.to_string(), accent_style()),
                Span::styled("' to remote?", primary_style()),
            ];
            let entries_data = [("Push", "y"), ("Push + Tags", "t"), ("Cancel", "n/Esc")];
            (Some(msg_spans), build_status_entries(&entries_data))
        }
        Mode::BranchMergeConfirm => {
            let (target, is_remote) = app
                .branch_action_target
                .as_ref()
                .map(|(name, remote)| (name.as_str(), *remote))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_branch_merge_entries(target, is_remote);
            (msg_spans, entries)
        }
        Mode::BranchMergeIntoConfirm => {
            let target =
                app.branch_action_target.as_ref().map(|(name, _)| name.as_str()).unwrap_or("");
            let (msg_spans, entries) = confirm_branch_merge_into_entries(target);
            (msg_spans, entries)
        }
        Mode::BranchRebaseConfirm => {
            let (target, is_remote) = app
                .branch_action_target
                .as_ref()
                .map(|(name, remote)| (name.as_str(), *remote))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_branch_rebase_entries(target, is_remote);
            (msg_spans, entries)
        }
        Mode::BranchInteractiveRebaseConfirm => {
            let (target, is_remote) = app
                .branch_action_target
                .as_ref()
                .map(|(name, remote)| (name.as_str(), *remote))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_branch_interactive_rebase_entries(target, is_remote);
            (msg_spans, entries)
        }
        Mode::TagDeleteConfirm => {
            let (target, is_on_remote) = app
                .tag_delete_target
                .as_ref()
                .map(|(name, is_on_remote)| (name.as_str(), *is_on_remote))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_tag_delete_entries(target, is_on_remote);
            (msg_spans, entries)
        }
        Mode::TagOverwriteConfirm => {
            let target =
                app.tag_overwrite_target.as_ref().map(|(n, _, _)| n.as_str()).unwrap_or("");
            let msg_spans = vec![
                Span::styled("Force update tag '", primary_style()),
                Span::styled(target.to_string(), accent_style()),
                Span::styled("'?", primary_style()),
            ];
            let entries_data = [("Confirm", "y"), ("Cancel", "n/Esc")];
            (Some(msg_spans), build_status_entries(&entries_data))
        }
        Mode::SubmoduleDeleteConfirm => {
            let target = app
                .submodule_delete_target
                .as_ref()
                .map(|sub| sub.path.display().to_string())
                .unwrap_or_default();
            let (msg_spans, entries) = confirm_submodule_delete_entries(&target);
            (msg_spans, entries)
        }
        Mode::TagPushConfirm => {
            let target = app.tag_push_target.as_deref().unwrap_or("");
            let msg_spans = vec![
                Span::styled("Push tag '", primary_style()),
                Span::styled(target.to_string(), accent_style()),
                Span::styled("' to remote?", primary_style()),
            ];
            let entries_data = [("Push", "y"), ("Force Push", "f"), ("Cancel", "n/Esc")];
            (Some(msg_spans), build_status_entries(&entries_data))
        }
        Mode::TagPushAllConfirm => {
            let (msg_spans, entries) = confirm_tag_push_all_entries();
            (msg_spans, entries)
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
            (msg_spans, entries)
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
            (msg_spans, entries)
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
            (Some(msg_spans), entries)
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
            (Some(msg_spans), entries)
        }
        Mode::RemotePicker => {
            let (msg_spans, entries) = remote_picker_status_entries();
            (msg_spans, entries)
        }
        Mode::CommitHistoryPicker => {
            // The picker routes through the generic navigation bindings
            // (src/popups/commit_history.rs).
            let compat = app.config.compatibility_mode;
            let kb = &app.keybindings;
            let nav_key = |a| compact_action_keys(kb, a, compat);
            let select_key = format!(
                "{}/{}",
                nav_key(crate::keybindings::Action::NavUp),
                nav_key(crate::keybindings::Action::NavDown)
            );
            let use_key = nav_key(crate::keybindings::Action::NavEnter);
            let cancel_key = nav_key(crate::keybindings::Action::NavEsc);
            let msg_spans = vec![Span::styled(
                "Previous commit messages  ",
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            )];
            let entries_data = [
                ("Select", select_key.as_str()),
                ("Use Message", use_key.as_str()),
                ("Cancel", cancel_key.as_str()),
            ];
            (Some(msg_spans), build_status_entries(&entries_data))
        }
        Mode::LabelPicker => {
            // Literal key codes in src/input.rs (Mode::LabelPicker).
            let msg_spans = vec![
                Span::raw("Label filter: type to narrow the list, then press "),
                Span::styled("Enter", accent_style()),
            ];
            let page_key = if app.config.compatibility_mode { "PgUp/PgDn" } else { "⇞/⇟" };
            let entries_data = [
                ("Select", "↑/↓"),
                ("Page", page_key),
                ("Jump", "Home/End"),
                ("Apply Filter", "Enter"),
                ("Label Settings", "→"),
                ("Cancel", "Esc"),
            ];
            (Some(msg_spans), build_status_entries(&entries_data))
        }
        Mode::StatsDashboard => {
            // Literal key codes in src/input.rs (Mode::StatsDashboard).
            let msg_spans = vec![Span::styled(
                "App Usage Stats  ",
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            )];
            let entries_data = [("Close", "Esc/Enter/q")];
            (Some(msg_spans), build_status_entries(&entries_data))
        }
        Mode::ForgeCommentPathInput | Mode::ForgeCommentLineInput | Mode::ForgeCommentBodyInput => {
            // Literal key codes in src/input.rs; Esc always drops the whole
            // comment and returns to the PR list.
            let step = match app.mode {
                Mode::ForgeCommentPathInput => "Step 1/3: file path  ",
                Mode::ForgeCommentLineInput => "Step 2/3: line number  ",
                _ => "Step 3/3: comment body  ",
            };
            let msg_spans = vec![
                Span::styled(
                    "Add PR Line Comment  ",
                    Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
                ),
                Span::styled(step, muted_style()),
            ];
            let entries_data = if app.mode == Mode::ForgeCommentBodyInput {
                [("Post Comment", "Enter"), ("Cancel", "Esc")]
            } else {
                [("Next", "Enter"), ("Cancel", "Esc")]
            };
            (Some(msg_spans), build_status_entries(&entries_data))
        }
        Mode::SearchColumnPicker => {
            let msg_spans = vec![
                Span::styled(
                    "Search Columns  ",
                    Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
                ),
                Span::styled("Choose columns to apply search on  ", muted_style()),
            ];
            let entries_data = [
                ("Navigate", "↑↓"),
                ("Toggle", "Space"),
                ("Confirm & Search", "Enter"),
                ("Cancel", "Esc"),
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
            (Some(msg_spans), entries)
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
                ("Fuzzy Search", "/"),
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
            (Some(msg_spans), entries)
        }
        Mode::DiscardChangesConfirm => {
            let (target, staged) = app
                .discard_target
                .as_ref()
                .map(|(name, staged)| (name.as_str(), *staged))
                .unwrap_or(("", false));
            let (msg_spans, entries) = confirm_discard_changes_entries(target, staged);
            (msg_spans, entries)
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
            (Some(msg_spans), entries)
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
            (Some(msg_spans), entries)
        }
        Mode::Inspect => {
            let (msg_spans, entries) = inspect_dismiss_entries(app);
            (msg_spans, entries)
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
            (Some(msg_spans), entries)
        }
        Mode::DebugLogs => {
            let msg_spans = vec![Span::styled(
                "Debug Logs  ",
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            )];
            let entries_data = if app.debug_log_search_editing {
                vec![("Type to filter", ""), ("Focus List", "Enter"), ("Clear/Exit Search", "Esc")]
            } else if app.debug_log_search_query.is_some() {
                vec![
                    ("Edit Query", "/"),
                    ("Clear Logs", "c/x"),
                    ("Clear Filter", "Esc"),
                    ("Back", "q"),
                ]
            } else {
                vec![("Find", "/"), ("Clear", "c/x"), ("Back", "Esc/q")]
            };
            let mut entries = Vec::new();
            for (i, (label, key)) in entries_data.iter().enumerate() {
                let mut spans = Vec::new();
                if i > 0 {
                    spans.push(Span::styled(" ", muted_style()));
                }
                spans.push(Span::raw((*label).to_string()));
                if !key.is_empty() {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled("[", muted_style()));
                    spans.push(Span::styled((*key).to_string(), accent_style()));
                    spans.push(Span::styled("]", muted_style()));
                }
                entries.push(StatusEntry::new(spans));
            }
            (Some(msg_spans), entries)
        }
        Mode::RemoteDeleteConfirm => {
            let target = app.remote_action_target.as_deref().unwrap_or("");
            let (msg_spans, entries) = confirm_remote_delete_entries(target);
            (msg_spans, entries)
        }
        Mode::UpdateConfirm => {
            let can_update = app.can_self_update();
            let title = if can_update { "Update Available  " } else { "New Version Available  " };
            let msg_spans = vec![Span::styled(
                title,
                Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
            )];
            let entries_data = if can_update {
                [("Confirm Update", "y"), ("Cancel", "n/Esc")]
            } else {
                [("Show Info", "y"), ("Cancel", "n/Esc")]
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
            (Some(msg_spans), entries)
        }

        Mode::TagSearchInput => {
            let msg_spans = vec![
                Span::raw("Fuzzy Tag Search: type query to search, select, then press "),
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
            (Some(msg_spans), entries)
        }
        Mode::CommitFuzzySearch => {
            let msg_spans = vec![
                Span::raw("Fuzzy Commit Search: type query to search, select, then press "),
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
            (Some(msg_spans), entries)
        }
        Mode::FileSearchInput => {
            let msg_spans = vec![
                Span::raw("Fuzzy File Search: type query to search, select, then press "),
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
            (Some(msg_spans), entries)
        }
        Mode::BranchSearchInput => {
            let msg_spans = vec![
                Span::raw("Fuzzy Branch Search: type query to search, select, then press "),
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
            (Some(msg_spans), entries)
        }
        Mode::RepoScanPicker => {
            let msg_spans = vec![
                Span::raw(
                    "Scan and Add repository: type to filter/manual path, select, then press ",
                ),
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
            (Some(msg_spans), entries)
        }
        Mode::BulkAddScanPicker => {
            let msg_spans = vec![
                Span::raw("Scan and Bulk Add: type to filter/manual path, select, then press "),
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
            (Some(msg_spans), entries)
        }
        Mode::GlobalSearch => {
            let msg_spans = vec![
                Span::raw("Global Search: "),
                Span::styled(
                    if app.global_search_focus_input { "Input Focused" } else { "Results Focused" },
                    accent_style().add_modifier(Modifier::BOLD),
                ),
            ];
            let entries_data = if app.global_search_focus_input {
                vec![
                    ("Type Query", "Char"),
                    ("Search", "Enter"),
                    ("Focus Results", "Tab"),
                    ("Cancel", "Esc"),
                ]
            } else {
                vec![
                    ("Select Result", "↑/↓"),
                    ("Open Detail", "Enter"),
                    ("Focus Input", "Tab"),
                    ("Cancel", "Esc"),
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
            (Some(msg_spans), entries)
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
            (Some(msg_spans), entries)
        }
        Mode::AddCwdRepoConfirm => {
            let msg_spans = vec![Span::styled(
                "Untracked repository  ",
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD),
            )];
            let entries = vec![
                StatusEntry::new(vec![
                    Span::raw("Add"),
                    Span::raw(" "),
                    Span::styled("[", muted_style()),
                    Span::styled("y", accent_style()),
                    Span::styled("]", muted_style()),
                ]),
                StatusEntry::new(vec![
                    Span::raw("Not now"),
                    Span::raw(" "),
                    Span::styled("[", muted_style()),
                    Span::styled("n/Esc", accent_style()),
                    Span::styled("]", muted_style()),
                ]),
            ];
            (Some(msg_spans), entries)
        }
        Mode::NotGitRepo => {
            let msg_spans = vec![Span::styled(
                "Not a Git Repository  ",
                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
            )];
            let entries = vec![StatusEntry::new(vec![
                Span::raw("Dismiss"),
                Span::raw(" "),
                Span::styled("[", muted_style()),
                Span::styled("Esc/Enter/q", accent_style()),
                Span::styled("]", muted_style()),
            ])];
            (Some(msg_spans), entries)
        }
        _ => return None,
    };
    Some(comps)
}

pub fn calculate_status_rows(app: &App, width: u16) -> u16 {
    if !app.status_expanded {
        return 1;
    }

    let is_merging =
        if let Some(crate::repo::ItemDetail::Repo { resolved, .. }) = &app.current_detail {
            crate::repo::is_merging(resolved)
        } else if let Some(selected_item) = app.get_selected_item() {
            let path = crate::repo::expand_tilde(selected_item);
            crate::repo::is_merging(&path)
        } else {
            false
        };

    let (message_spans, entries) = if app.loading_repo_path.is_some() {
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
        (Some(msg_spans), entries)
    } else if let Some(comps) = get_status_layout_components(app) {
        comps
    } else {
        return 1;
    };

    let left_width = width.saturating_sub(28) as usize;
    if left_width == 0 {
        return 1;
    }

    let mut tokens: Vec<String> = Vec::new();

    let badge = get_mode_badge(&app.mode);
    let mode_sep = if app.config.compatibility_mode { " > " } else { " ⟩ " };
    tokens.push(format!("{}{}", badge.content, mode_sep));

    if is_merging {
        tokens.push("[ ⚡ MERGING ] ".to_string());
    }

    if let Some(msg) = message_spans {
        let msg_text = msg.iter().map(|s| s.content.as_ref()).collect::<Vec<_>>().join("");
        for word in msg_text.split_whitespace() {
            tokens.push(word.to_string());
        }
    }

    let separator = if app.config.compatibility_mode { " > " } else { " ⟩ " };
    let mut first = true;
    for entry in entries {
        if !first {
            tokens.push(separator.to_string());
        }
        first = false;

        let mut entry_str = String::new();
        let mut start = 0;
        if !entry.spans.is_empty() && entry.spans[0].content.trim().is_empty() {
            start = 1;
        }
        for span in &entry.spans[start..] {
            entry_str.push_str(&span.content);
        }
        tokens.push(entry_str);
    }

    tokens.push(" Less [.]".to_string());

    let mut rows = 1;
    let mut current_line_len = 0;

    for (i, token) in tokens.iter().enumerate() {
        let token_len = token.chars().count();
        if i == 0 {
            current_line_len = token_len;
        } else {
            let has_space_prefix = token.starts_with(' ');
            let space_needed = if has_space_prefix { 0 } else { 1 };

            if current_line_len + space_needed + token_len > left_width {
                rows += 1;
                current_line_len = token_len;
            } else {
                current_line_len += space_needed + token_len;
            }
        }
    }

    rows as u16
}

fn get_mode_badge(mode: &Mode) -> Span<'static> {
    let (label, color) = match mode {
        Mode::TagSearchInput => ("TAG SEARCH", Color::Rgb(135, 0, 135)),
        Mode::CommitFuzzySearch => ("COMMIT SEARCH", Color::Rgb(175, 95, 0)),
        Mode::FileSearchInput => ("FILE SEARCH", Color::Rgb(0, 135, 175)),
        Mode::BranchSearchInput => ("BRANCH SEARCH", Color::Rgb(135, 0, 135)),
        Mode::RepoScanPicker => ("SCAN", ACCENT()),
        Mode::BulkAddScanPicker => ("BULK SCAN", ACCENT()),
        Mode::GlobalSearch => ("GLOBAL SEARCH", Color::Rgb(0, 135, 175)),
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
        Mode::LabelSettings => ("LABEL SETTINGS", Color::Rgb(135, 0, 135)),
        Mode::LabelPicker => ("LABELS", Color::Rgb(135, 0, 135)),
        Mode::DebugLogs => ("DEBUG", Color::Rgb(150, 150, 150)),
        Mode::Logs => ("LOGS", Color::Magenta),
        Mode::CommitHistoryPicker => ("HISTORY", Color::Rgb(175, 95, 0)),
        Mode::StatsDashboard => ("STATS", Color::Green),
        Mode::Adding
        | Mode::BulkAddInput
        | Mode::Editing
        | Mode::LabelInput
        | Mode::AddRepoLabelInput
        | Mode::BulkAddRepoLabelInput
        | Mode::CloneRepoLabelInput
        | Mode::RepoSearchInput
        | Mode::ImportUrlInput
        | Mode::ImportDestInput
        | Mode::ImportNameInput
        | Mode::RemoteAddNameInput
        | Mode::RemoteAddUrlInput
        | Mode::BranchCreateInput
        | Mode::TagCreateInput
        | Mode::StashCreateInput
        | Mode::WorktreeAddBranchInput
        | Mode::WorktreeAddPathInput
        | Mode::WorktreeLockReasonInput
        | Mode::SubmoduleAddUrlInput
        | Mode::SubmoduleAddPathInput
        | Mode::ForgeCommentPathInput
        | Mode::ForgeCommentLineInput
        | Mode::ForgeCommentBodyInput
        | Mode::CommitInput
        | Mode::CommitSearchInput
        | Mode::LogsSearchInput => ("INPUT", Color::Red),
        Mode::ConfirmDelete
        | Mode::BranchDeleteConfirm
        | Mode::BranchCheckoutConfirm
        | Mode::TagCheckoutConfirm
        | Mode::CommitCheckoutConfirm
        | Mode::BranchPushConfirm
        | Mode::BranchMergeConfirm
        | Mode::BranchMergeIntoConfirm
        | Mode::BranchRebaseConfirm
        | Mode::BranchInteractiveRebaseConfirm
        | Mode::DiscardChangesConfirm
        | Mode::TagDeleteConfirm
        | Mode::TagOverwriteConfirm
        | Mode::TagPushConfirm
        | Mode::TagPushAllConfirm
        | Mode::StashDeleteConfirm
        | Mode::StashApplyConfirm
        | Mode::RemoteDeleteConfirm
        | Mode::SubmoduleDeleteConfirm
        | Mode::UpdateConfirm
        | Mode::WorktreeRemoveConfirm
        | Mode::AddCwdRepoConfirm
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

    let status_chunks = if app.config.show_system_stats {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(28)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(0)])
            .split(area)
    };

    let left_area = status_chunks[0];
    let right_area = status_chunks[1];

    let mut spans = Vec::new();
    let toggle_key = app.keybindings.format_action_keys(
        crate::keybindings::Action::ToggleStatusBar,
        app.config.compatibility_mode,
    );

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
        spans.push(Span::styled(toggle_key.clone(), accent_style()));
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
            spans.push(Span::styled(toggle_key.clone(), accent_style()));
            spans.push(Span::styled("]", muted_style()));
        }

        let para = Paragraph::new(Line::from(spans));
        f.render_widget(para, left_area);
    }

    // Render CPU & Memory Stats on the right
    let (rss_mb, cpu_pct) = get_process_stats(app);
    let stats_text = if app.config.show_system_stats && rss_mb > 0.0 {
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
