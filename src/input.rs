//! Keystroke dispatch.
//!
//! `handle_key` reads `app.mode` and routes the keystroke to the
//! appropriate `App` method. Returns `false` when the user has asked to
//! quit, `true` otherwise.

use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, Mode};
use crate::components::Component;

/// Dispatch a key press. Returns `false` if the user requested quit.
/// The queue is always drained after every keypress, regardless of exit path.
pub fn handle_key(app: &mut App, key: KeyEvent, visible_count: usize) -> bool {
    app.drain_queue();
    let result = dispatch_key(app, key, visible_count);
    app.drain_queue();
    result
}

fn dispatch_key(app: &mut App, key: KeyEvent, visible_count: usize) -> bool {
    crate::debug_log::info(format!("Key pressed: {:?}", key.code));
    let code = key.code;

    if app.error_message.is_some() {
        if matches!(
            code,
            KeyCode::Esc
                | KeyCode::Enter
                | KeyCode::Char('q')
                | KeyCode::Char('Q')
                | KeyCode::Char(' ')
        ) {
            app.error_message = None;
        }
        return true;
    }

    if app.fetching {
        // Allow Esc / q to dismiss a stuck progress popup.
        if matches!(code, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q')) {
            app.dismiss_fetch();
        }
        return true;
    }

    if app.loading_repo_path.is_some() {
        // Allow Esc / q / Q to cancel repository loading and go back.
        if matches!(code, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q')) {
            app.close_detail();
        }
        return true;
    }

    // Toggle status bar expanded mode with '.' (except in text input fields)
    let is_text_input = matches!(
        app.mode,
        Mode::Adding
            | Mode::Editing
            | Mode::BranchCreateInput
            | Mode::TagCreateInput
            | Mode::StashCreateInput
            | Mode::RepoSearchInput
            | Mode::ImportUrlInput
            | Mode::ImportDestInput
            | Mode::ImportNameInput
            | Mode::BulkAddInput
            | Mode::RemoteAddNameInput
            | Mode::RemoteAddUrlInput
    ) || (matches!(app.mode, Mode::CommitInput) && app.commit_popup.editing)
        || (matches!(app.mode, Mode::Settings) && app.settings_editing);
    if !is_text_input && code == KeyCode::Char('.') {
        app.toggle_status_expanded();
        return true;
    }
    match &app.mode {
        Mode::Normal
        | Mode::RepoSearchInput
        | Mode::Adding
        | Mode::BulkAddInput
        | Mode::Editing
        | Mode::ConfirmDelete => {
            if !crate::tabs::HomeTab::handle_event(app, key, visible_count) {
                return false;
            }
        }
        Mode::Settings => {
            if crate::popups::settings::SettingsPopup::handle_event(app, key) {
                return true;
            }
        }
        Mode::DebugLogs => {
            if crate::popups::debug::DebugLogsPopup::handle_event(app, key) {
                return true;
            }
        }
        Mode::ImportUrlInput | Mode::ImportDestInput | Mode::ImportNameInput => {
            if crate::popups::import::ImportPopup::handle_event(app, key) {
                return true;
            }
        }
        Mode::CherryPickConfirm => match code {
            KeyCode::Up | KeyCode::Char('k') => {
                app.cherry_pick_dest_selection = app.cherry_pick_dest_selection.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !app.cherry_pick_dest_branches.is_empty() {
                    app.cherry_pick_dest_selection = (app.cherry_pick_dest_selection + 1)
                        .min(app.cherry_pick_dest_branches.len().saturating_sub(1));
                }
            }
            KeyCode::Enter => app.confirm_cherry_pick(),
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => app.cancel_cherry_pick(),
            _ => {}
        },
        Mode::StashApplyConfirm => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => app.confirm_stash_apply(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_stash_apply(),
            KeyCode::Char('d')
            | KeyCode::Char('D')
            | KeyCode::Char(' ')
            | KeyCode::Char('a')
            | KeyCode::Char('A') => {
                app.toggle_stash_apply_delete();
            }
            _ => {}
        },
        Mode::Help => {
            if crate::popups::help::HelpPopup::handle_event(app, key) {
                return true;
            }
        }
        Mode::About => {
            if crate::popups::about::AboutPopup::handle_event(app, key) {
                return true;
            }
        }
        Mode::Detail => {
            if crate::tabs::route_detail_event(app, key) {
                return true;
            }
        }

        Mode::Inspect => {
            if crate::popups::inspect::InspectPopup::handle_event(app, key) {
                return true;
            }
        }

        Mode::DetailHelp => {
            if crate::popups::help::DetailHelpPopup::handle_event(app, key) {
                return true;
            }
        }
        Mode::SearchColumnPicker => match code {
            KeyCode::Up => {
                app.search_column_selection = app.search_column_selection.saturating_sub(1);
            }
            KeyCode::Down => {
                if app.search_column_selection < 3 {
                    app.search_column_selection += 1;
                }
            }
            KeyCode::Char(' ') => match app.search_column_selection {
                0 => app.search_columns_sha = !app.search_columns_sha,
                1 => app.search_columns_message = !app.search_columns_message,
                2 => app.search_columns_author = !app.search_columns_author,
                3 => app.search_columns_date = !app.search_columns_date,
                _ => {}
            },
            KeyCode::Enter => {
                app.input_buffer = app.commit_list.search_query.clone().unwrap_or_default();
                app.in_logs_ui = true;
                app.mode = Mode::LogsSearchInput;
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                if app.in_logs_ui {
                    app.mode = Mode::Logs;
                } else {
                    app.mode = Mode::Detail;
                }
            }
            _ => {}
        },
        Mode::LogsSearchInput => match code {
            KeyCode::Esc => {
                app.commit_list.search_query = None;
                app.mode = Mode::Logs;
            }
            KeyCode::Enter => {
                let query = app.input_buffer.clone();
                if query.trim().is_empty() {
                    app.commit_list.search_query = None;
                } else {
                    app.commit_list.search_query = Some(query);
                }
                app.mode = Mode::Logs;
            }
            KeyCode::Backspace => {
                app.input_backspace();
            }
            KeyCode::Char(c) => {
                app.input_char(c);
            }
            _ => {}
        },
        Mode::Logs => {
            if crate::tabs::logs::LogsTab::handle_event(app, key) {
                return true;
            }
        }
        Mode::RemotePicker => {
            if crate::popups::remote_picker::RemotePickerPopup::handle_event(app, key) {
                return true;
            }
        }
        Mode::CommitSearchInput => match code {
            KeyCode::Esc => app.cancel_commit_search(),
            KeyCode::Enter => app.mode = Mode::Detail,
            KeyCode::Backspace => {
                app.input_backspace();
                app.commit_search_input_change();
            }
            KeyCode::Char(c) => {
                app.input_char(c);
                app.commit_search_input_change();
            }
            _ => {}
        },

        Mode::BranchDeleteConfirm
        | Mode::BranchPushConfirm
        | Mode::BranchMergeConfirm
        | Mode::MergeAbortConfirm
        | Mode::MergeContinueConfirm
        | Mode::BranchRebaseConfirm
        | Mode::BranchInteractiveRebaseConfirm
        | Mode::DiscardChangesConfirm
        | Mode::RevertConfirm
        | Mode::TagDeleteConfirm
        | Mode::TagPushConfirm
        | Mode::TagPushAllConfirm
        | Mode::StashDeleteConfirm
        | Mode::BranchCheckoutConfirm
        | Mode::TagCheckoutConfirm
        | Mode::RemoteDeleteConfirm => {
            let ev = crossterm::event::Event::Key(key);
            if app
                .confirm_popup
                .event(&ev)
                .unwrap_or(crate::components::EventState::NotConsumed)
                .is_consumed()
            {
                return true;
            }
        }
        Mode::BranchCreateInput
        | Mode::TagCreateInput
        | Mode::StashCreateInput
        | Mode::RemoteAddNameInput
        | Mode::RemoteAddUrlInput => {
            let ev = crossterm::event::Event::Key(key);
            if app
                .generic_input_popup
                .event(&ev)
                .unwrap_or(crate::components::EventState::NotConsumed)
                .is_consumed()
            {
                return true;
            }
        }

        Mode::CommitInput => {
            let ev = crossterm::event::Event::Key(key);
            if app
                .commit_popup
                .event(&ev)
                .unwrap_or(crate::components::EventState::NotConsumed)
                .is_consumed()
            {
                return true;
            }
        }
    }
    true
}
