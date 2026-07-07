//! Keystroke dispatch.
//!
//! `handle_key` reads `app.mode` and routes the keystroke to the
//! appropriate `App` method. Returns `false` when the user has asked to
//! quit, `true` otherwise.

use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, DetailSection, Mode};
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

    if app.is_bound(crate::keybindings::Action::Close, key) {
        return false;
    }

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
            | Mode::AddRepoLabelInput
            | Mode::BulkAddRepoLabelInput
            | Mode::CloneRepoLabelInput
            | Mode::BranchCreateInput
            | Mode::TagCreateInput
            | Mode::StashCreateInput
            | Mode::RepoSearchInput
            | Mode::RepoJump
            | Mode::RepoScanPicker
            | Mode::BulkAddScanPicker
            | Mode::BranchSearchInput
            | Mode::FileSearchInput
            | Mode::CommitFuzzySearch
            | Mode::TagSearchInput
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
            | Mode::GlobalSearch
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
        | Mode::AddRepoLabelInput
        | Mode::BulkAddRepoLabelInput
        | Mode::CloneRepoLabelInput
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
        Mode::Overview => {
            if app.is_bound(crate::keybindings::Action::Overview, key) {
                app.mode = Mode::Detail;
                return true;
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
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
                KeyCode::Tab | KeyCode::Char('w') | KeyCode::Char('W') => {
                    app.overview_focus = match app.overview_focus {
                        crate::app::OverviewFocus::Overview => crate::app::OverviewFocus::Stats,
                        crate::app::OverviewFocus::Stats => crate::app::OverviewFocus::Overview,
                    };
                    return true;
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    app.overview_scroll_up();
                    return true;
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    app.overview_scroll_down();
                    return true;
                }
                KeyCode::PageUp => {
                    let p = app.get_current_page_size();
                    app.overview_scroll_page_up(p);
                    return true;
                }
                KeyCode::PageDown => {
                    let p = app.get_current_page_size();
                    app.overview_scroll_page_down(p);
                    return true;
                }
                KeyCode::Home => {
                    app.overview_scroll_to_top();
                    return true;
                }
                KeyCode::End => {
                    app.overview_scroll_to_bottom();
                    return true;
                }
                _ => {}
            }
        }
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
        Mode::GlobalSearch => {
            match code {
                KeyCode::Esc => {
                    app.input_buffer.clear();
                    app.mode = Mode::Normal;
                }
                KeyCode::Tab => {
                    app.global_search_focus_input = !app.global_search_focus_input;
                }
                KeyCode::Up => {
                    if !app.global_search_results.is_empty() {
                        app.global_search_selection = app.global_search_selection.saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if !app.global_search_results.is_empty()
                        && app.global_search_selection + 1 < app.global_search_results.len()
                    {
                        app.global_search_selection += 1;
                    }
                }
                KeyCode::PageUp => {
                    if !app.global_search_results.is_empty() {
                        app.global_search_selection =
                            app.global_search_selection.saturating_sub(app.config.page_size);
                    }
                }
                KeyCode::PageDown => {
                    if !app.global_search_results.is_empty() {
                        app.global_search_selection = (app.global_search_selection
                            + app.config.page_size)
                            .min(app.global_search_results.len().saturating_sub(1));
                    }
                }
                KeyCode::Home => {
                    if !app.global_search_results.is_empty() {
                        app.global_search_selection = 0;
                    }
                }
                KeyCode::End => {
                    if !app.global_search_results.is_empty() {
                        app.global_search_selection =
                            app.global_search_results.len().saturating_sub(1);
                    }
                }
                KeyCode::Enter => {
                    if app.global_search_focus_input {
                        app.trigger_global_search();
                    } else {
                        app.select_global_search_result();
                    }
                }
                KeyCode::Backspace => {
                    if app.global_search_focus_input {
                        app.input_buffer.pop();
                    }
                }
                KeyCode::Char(c) if app.global_search_focus_input => {
                    app.input_buffer.push(c);
                }
                _ => {}
            }
            return true;
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
        Mode::RepoScanPicker => {
            let matches = app.get_scan_matches();
            match code {
                KeyCode::Esc => {
                    app.input_buffer.clear();
                    app.mode = Mode::Normal;
                }
                KeyCode::Up => {
                    if !matches.is_empty() {
                        app.repo_scan_selection = app.repo_scan_selection.saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if !matches.is_empty() && app.repo_scan_selection + 1 < matches.len() {
                        app.repo_scan_selection += 1;
                    }
                }
                KeyCode::Enter => {
                    if !matches.is_empty() && app.repo_scan_selection < matches.len() {
                        let path = matches[app.repo_scan_selection].1.clone();
                        app.pending_add_repo = Some(path);
                        app.input_buffer.clear();
                        app.mode = Mode::AddRepoLabelInput;
                    } else {
                        app.input_buffer.clear();
                        app.mode = Mode::Normal;
                    }
                }
                KeyCode::Backspace => {
                    app.input_buffer.pop();
                    app.repo_scan_selection = 0;
                }
                KeyCode::Char(c) => {
                    app.input_buffer.push(c);
                    app.repo_scan_selection = 0;
                }
                _ => {}
            }
            return true;
        }
        Mode::BulkAddScanPicker => {
            let matches = app.get_scan_matches();
            match code {
                KeyCode::Esc => {
                    app.input_buffer.clear();
                    app.mode = Mode::Normal;
                }
                KeyCode::Up => {
                    if !matches.is_empty() {
                        app.repo_scan_selection = app.repo_scan_selection.saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if !matches.is_empty() && app.repo_scan_selection + 1 < matches.len() {
                        app.repo_scan_selection += 1;
                    }
                }
                KeyCode::Enter => {
                    if !matches.is_empty() && app.repo_scan_selection < matches.len() {
                        let path = matches[app.repo_scan_selection].1.clone();
                        app.pending_bulk_add_repo = Some(path);
                        app.input_buffer.clear();
                        app.mode = Mode::BulkAddRepoLabelInput;
                    } else {
                        app.input_buffer.clear();
                        app.mode = Mode::Normal;
                    }
                }
                KeyCode::Backspace => {
                    app.input_buffer.pop();
                    app.repo_scan_selection = 0;
                }
                KeyCode::Char(c) => {
                    app.input_buffer.push(c);
                    app.repo_scan_selection = 0;
                }
                _ => {}
            }
            return true;
        }
        Mode::BranchSearchInput => {
            let matches = app.get_branch_search_matches();
            match code {
                KeyCode::Esc => {
                    app.input_buffer.clear();
                    app.mode = app.previous_mode.unwrap_or(Mode::Detail);
                }
                KeyCode::Up => {
                    if !matches.is_empty() {
                        app.branch_search_selection = app.branch_search_selection.saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if !matches.is_empty() && app.branch_search_selection + 1 < matches.len() {
                        app.branch_search_selection += 1;
                    }
                }
                KeyCode::Enter => {
                    if !matches.is_empty() && app.branch_search_selection < matches.len() {
                        let (branch_name, is_remote) = matches[app.branch_search_selection].clone();
                        app.branch_action_target = Some((branch_name, is_remote));
                        app.mode = Mode::BranchCheckoutConfirm;
                    } else {
                        app.input_buffer.clear();
                        app.mode = app.previous_mode.unwrap_or(Mode::Detail);
                    }
                }
                KeyCode::Backspace => {
                    app.input_buffer.pop();
                    app.branch_search_selection = 0;
                }
                KeyCode::Char(c) => {
                    app.input_buffer.push(c);
                    app.branch_search_selection = 0;
                }
                _ => {}
            }
            return true;
        }
        Mode::FileSearchInput => {
            let matches = app.get_file_search_matches();
            match code {
                KeyCode::Esc => {
                    app.input_buffer.clear();
                    app.mode = app.previous_mode.unwrap_or(Mode::Detail);
                }
                KeyCode::Up => {
                    if !matches.is_empty() {
                        app.file_search_selection = app.file_search_selection.saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if !matches.is_empty() && app.file_search_selection + 1 < matches.len() {
                        app.file_search_selection += 1;
                    }
                }
                KeyCode::Enter => {
                    if !matches.is_empty() && app.file_search_selection < matches.len() {
                        let selected = matches[app.file_search_selection].clone();
                        let parts: Vec<&str> = selected.split('/').collect();
                        let mut accumulated = String::new();
                        for part in parts.iter().take(parts.len().saturating_sub(1)) {
                            if !accumulated.is_empty() {
                                accumulated.push('/');
                            }
                            accumulated.push_str(part);
                            app.file_tree.expanded_folders.insert(accumulated.clone());
                        }
                        app.rebuild_visible_files();
                        if let Some(pos) = app
                            .file_tree
                            .visible_files
                            .iter()
                            .position(|item| item.full_path == selected)
                        {
                            app.file_tree.file_list_selection = pos;
                            app.file_tree.file_content_scroll = 0;
                            app.detail_focus = DetailSection::Files;
                        }
                        app.input_buffer.clear();
                        app.mode = app.previous_mode.unwrap_or(Mode::Detail);
                    } else {
                        app.input_buffer.clear();
                        app.mode = app.previous_mode.unwrap_or(Mode::Detail);
                    }
                }
                KeyCode::Backspace => {
                    app.input_buffer.pop();
                    app.file_search_selection = 0;
                }
                KeyCode::Char(c) => {
                    app.input_buffer.push(c);
                    app.file_search_selection = 0;
                }
                _ => {}
            }
            return true;
        }
        Mode::CommitFuzzySearch => {
            let matches = app.get_commit_fuzzy_matches();
            match code {
                KeyCode::Esc => {
                    app.input_buffer.clear();
                    app.mode = app.previous_mode.unwrap_or(Mode::Logs);
                }
                KeyCode::Up => {
                    if !matches.is_empty() {
                        app.commit_search_selection = app.commit_search_selection.saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if !matches.is_empty() && app.commit_search_selection + 1 < matches.len() {
                        app.commit_search_selection += 1;
                    }
                }
                KeyCode::Enter => {
                    if !matches.is_empty() && app.commit_search_selection < matches.len() {
                        let original_index = matches[app.commit_search_selection].0;
                        app.commit_list.selection = original_index;
                        app.input_buffer.clear();
                        app.mode = app.previous_mode.unwrap_or(Mode::Logs);
                    } else {
                        app.input_buffer.clear();
                        app.mode = app.previous_mode.unwrap_or(Mode::Logs);
                    }
                }
                KeyCode::Backspace => {
                    app.input_buffer.pop();
                    app.commit_search_selection = 0;
                }
                KeyCode::Char(c) => {
                    app.input_buffer.push(c);
                    app.commit_search_selection = 0;
                }
                _ => {}
            }
            return true;
        }
        Mode::TagSearchInput => {
            let matches = app.get_tag_search_matches();
            match code {
                KeyCode::Esc => {
                    app.input_buffer.clear();
                    app.mode = app.previous_mode.unwrap_or(Mode::Detail);
                }
                KeyCode::Up => {
                    if !matches.is_empty() {
                        app.tag_search_selection = app.tag_search_selection.saturating_sub(1);
                    }
                }
                KeyCode::Down => {
                    if !matches.is_empty() && app.tag_search_selection + 1 < matches.len() {
                        app.tag_search_selection += 1;
                    }
                }
                KeyCode::Enter => {
                    if !matches.is_empty() && app.tag_search_selection < matches.len() {
                        let tag_name = matches[app.tag_search_selection].clone();
                        app.tag_checkout_target = Some(tag_name);
                        app.mode = Mode::TagCheckoutConfirm;
                    } else {
                        app.input_buffer.clear();
                        app.mode = app.previous_mode.unwrap_or(Mode::Detail);
                    }
                }
                KeyCode::Backspace => {
                    app.input_buffer.pop();
                    app.tag_search_selection = 0;
                }
                KeyCode::Char(c) => {
                    app.input_buffer.push(c);
                    app.tag_search_selection = 0;
                }
                _ => {}
            }
            return true;
        }
    }
    true
}
