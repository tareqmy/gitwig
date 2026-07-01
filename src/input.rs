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

    let is_text_input = matches!(
        app.mode,
        Mode::Adding
            | Mode::Editing
            | Mode::LabelInput
            | Mode::BranchCreateInput
            | Mode::TagCreateInput
            | Mode::StashCreateInput
            | Mode::RepoSearchInput
            | Mode::RepoJump
            | Mode::ImportUrlInput
            | Mode::ImportDestInput
            | Mode::ImportNameInput
            | Mode::BulkAddInput
            | Mode::RemoteAddNameInput
            | Mode::RemoteAddUrlInput
            | Mode::WorktreeAddBranchInput
            | Mode::WorktreeAddPathInput
            | Mode::WorktreeLockReasonInput
            | Mode::WorktreeRemoveConfirm
            | Mode::SubmoduleAddUrlInput
            | Mode::SubmoduleAddPathInput
    ) || (matches!(app.mode, Mode::CommitInput) && app.commit_popup.editing)
        || (matches!(app.mode, Mode::Settings) && app.settings_editing);
    if !is_text_input && app.is_bound(crate::keybindings::Action::ToggleStatusBar, key) {
        app.toggle_status_expanded();
        return true;
    }
    match &app.mode {
        Mode::Normal
        | Mode::RepoSearchInput
        | Mode::Adding
        | Mode::BulkAddInput
        | Mode::Editing
        | Mode::LabelInput
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
        Mode::CherryPickConfirm | Mode::StashApplyConfirm => {
            if crate::popups::confirm::ConfirmPopup::handle_event(app, key) {
                return true;
            }
        }
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
        Mode::Legend => {
            if crate::popups::legend::LegendPopup::handle_event(app, key) {
                return true;
            }
        }
        Mode::Detail => {
            if crate::tabs::route_detail_event(app, key) {
                return true;
            }
        }
        Mode::FileHistory => {
            if crate::tabs::FileHistoryTab::handle_event(app, key) {
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
        Mode::SearchColumnPicker => {
            if crate::popups::search_columns::SearchColumnsPopup::handle_event(app, key) {
                return true;
            }
        }
        Mode::LogsSearchInput | Mode::CommitSearchInput => {
            if crate::popups::log_search::LogSearchPopup::handle_event(app, key) {
                return true;
            }
        }
        Mode::Logs => {
            if crate::tabs::logs::LogsTab::handle_event(app, key) {
                return true;
            }
        }
        Mode::RepoSettings => {
            if crate::popups::repo_settings::RepoSettingsPopup::handle_event(app, key) {
                return true;
            }
        }
        Mode::Overview => match key.code {
            KeyCode::Esc
            | KeyCode::Char('q')
            | KeyCode::Char('Q')
            | KeyCode::Char('v')
            | KeyCode::Char('V') => {
                app.mode = Mode::Detail;
                return true;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                app.repo_settings_selected_index = 0;
                app.repo_settings_editing = false;
                app.repo_settings_input = String::new();
                app.mode = Mode::RepoSettings;
                return true;
            }
            _ => {}
        },
        Mode::RemotePicker => {
            if crate::popups::remote_picker::RemotePickerPopup::handle_event(app, key) {
                return true;
            }
        }

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
        | Mode::RemoteDeleteConfirm
        | Mode::SubmoduleDeleteConfirm
        | Mode::UpdateConfirm => {
            let is_destructive = matches!(
                app.mode,
                Mode::BranchDeleteConfirm
                    | Mode::DiscardChangesConfirm
                    | Mode::TagDeleteConfirm
                    | Mode::StashDeleteConfirm
                    | Mode::RemoteDeleteConfirm
                    | Mode::SubmoduleDeleteConfirm
                    | Mode::MergeAbortConfirm
            );

            if is_destructive && key.code == KeyCode::Enter {
                app.queue.push(crate::queue::InternalEvent::ConfirmNo);
                return true;
            }

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
        Mode::StashingUI => match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.mode = Mode::Detail;
                return true;
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                app.stash_untracked = !app.stash_untracked;
                return true;
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                app.stash_keep_index = !app.stash_keep_index;
                return true;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                app.start_stash_create();
                return true;
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                app.stashing_ui_selection = app.stashing_ui_selection.saturating_sub(1);
                return true;
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
                    let mut count = info.changes.conflicted.len()
                        + info.changes.staged.len()
                        + info.changes.unstaged.len();
                    if app.stash_untracked {
                        count += info.changes.untracked.len();
                    }
                    if count > 0 {
                        app.stashing_ui_selection =
                            (app.stashing_ui_selection + 1).min(count.saturating_sub(1));
                    }
                }
                return true;
            }
            _ => {}
        },
        Mode::StashCreateInput => {
            if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) {
                match key.code {
                    KeyCode::Char('u') | KeyCode::Char('U') => {
                        app.stash_untracked = !app.stash_untracked;
                        return true;
                    }
                    KeyCode::Char('i') | KeyCode::Char('I') => {
                        app.stash_keep_index = !app.stash_keep_index;
                        return true;
                    }
                    _ => {}
                }
            }
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
        Mode::BranchCreateInput
        | Mode::TagCreateInput
        | Mode::RemoteAddNameInput
        | Mode::RemoteAddUrlInput
        | Mode::WorktreeAddBranchInput
        | Mode::WorktreeAddPathInput
        | Mode::WorktreeLockReasonInput
        | Mode::WorktreeRemoveConfirm
        | Mode::SubmoduleAddUrlInput
        | Mode::SubmoduleAddPathInput => {
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
        Mode::RepoJump => {
            let matches = app.get_jump_matches();
            match code {
                KeyCode::Esc => {
                    app.input_buffer.clear();
                    app.mode = Mode::Normal;
                }
                KeyCode::Up => {
                    if !matches.is_empty() {
                        app.repo_jump_selection = app.repo_jump_selection.saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if !matches.is_empty() && app.repo_jump_selection + 1 < matches.len() {
                        app.repo_jump_selection += 1;
                    }
                }
                KeyCode::Enter => {
                    if !matches.is_empty() && app.repo_jump_selection < matches.len() {
                        let original_index = matches[app.repo_jump_selection].0;
                        app.jump_to_repo(original_index);
                    } else {
                        app.input_buffer.clear();
                        app.mode = Mode::Normal;
                    }
                }
                KeyCode::Backspace => {
                    app.input_buffer.pop();
                    app.repo_jump_selection = 0;
                }
                KeyCode::Char(c) => {
                    app.input_buffer.push(c);
                    app.repo_jump_selection = 0;
                }
                _ => {}
            }
            return true;
        }
    }
    true
}
