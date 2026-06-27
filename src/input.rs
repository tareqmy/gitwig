//! Keystroke dispatch.
//!
//! `handle_key` reads `app.mode` and routes the keystroke to the
//! appropriate `App` method. Returns `false` when the user has asked to
//! quit, `true` otherwise.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
        Mode::Normal => match code {
            KeyCode::Esc if app.repo_search_query.is_some() => {
                app.repo_search_query = None;
                app.selected_index = 0;
                app.scroll_top = 0;
            }
            KeyCode::Char('q') | KeyCode::Esc => return false,
            KeyCode::Down => app.move_down(visible_count),
            KeyCode::Up => app.move_up(),
            KeyCode::PageDown => app.page_down(app.config.page_size),
            KeyCode::PageUp => app.page_up(app.config.page_size),
            KeyCode::Home => app.move_to_top(),
            KeyCode::End => app.move_to_bottom(visible_count),
            KeyCode::Char('a') => app.start_add(),
            KeyCode::Char('A') => {
                app.start_bulk_add();
            }
            KeyCode::Char('e') => app.start_edit(),
            KeyCode::Char('d') => app.request_delete(),
            KeyCode::Char('?') => app.open_help(),
            KeyCode::Char('v') | KeyCode::Char('V') => app.open_about(),
            KeyCode::Char('R') => app.refresh_selected_status(),
            KeyCode::Char('o') => app.cycle_sort_order(),
            KeyCode::Char('O') => app.toggle_sort_reverse(),
            KeyCode::Char('p') => app.toggle_pin_selected(),
            KeyCode::Char('s') => {
                app.mode = Mode::Settings;
                app.settings_selected_index = 0;
                app.settings_editing = false;
            }
            KeyCode::Char('D') | KeyCode::Char('l') | KeyCode::Char('L') => {
                crate::debug_log::info("Opening debug logs");
                app.mode = Mode::DebugLogs;
                app.debug_log_scroll = 0;
            }
            KeyCode::Char('i') => {
                crate::debug_log::info("Starting repository import");
                app.mode = Mode::ImportUrlInput;
                app.input_buffer.clear();
                app.import_url.clear();
                app.import_dest.clear();
                app.import_name.clear();
            }
            KeyCode::Char('g') => {
                app.pending_git_app = true;
            }
            KeyCode::Char('f') => {
                app.input_buffer.clear();
                if let Some(ref q) = app.repo_search_query {
                    app.input_buffer.push_str(q);
                }
                app.mode = Mode::RepoSearchInput;
            }
            KeyCode::Enter | KeyCode::Right => app.open_detail(),
            _ => {}
        },
        Mode::Settings => {
            if app.settings_editing && app.settings_selected_index == 3 {
                match code {
                    KeyCode::Esc => app.cancel_settings_edit(),
                    KeyCode::Enter => app.commit_settings_edit(),
                    KeyCode::Down
                        if app.settings_theme_index + 1 < app.settings_theme_list.len() =>
                    {
                        app.settings_theme_index += 1;
                    }
                    KeyCode::Up if app.settings_theme_index > 0 => {
                        app.settings_theme_index -= 1;
                    }
                    KeyCode::PageUp if app.settings_theme_index > 0 => {
                        app.settings_theme_index =
                            app.settings_theme_index.saturating_sub(app.config.page_size);
                    }
                    KeyCode::PageDown
                        if app.settings_theme_index + 1 < app.settings_theme_list.len() =>
                    {
                        app.settings_theme_index = (app.settings_theme_index
                            + app.config.page_size)
                            .min(app.settings_theme_list.len().saturating_sub(1));
                    }
                    KeyCode::Home => {
                        app.settings_theme_index = 0;
                    }
                    KeyCode::End if !app.settings_theme_list.is_empty() => {
                        app.settings_theme_index = app.settings_theme_list.len() - 1;
                    }
                    _ => {}
                }
            } else {
                match code {
                    KeyCode::Esc if app.settings_editing => app.cancel_settings_edit(),
                    KeyCode::Esc => app.mode = Mode::Normal,
                    KeyCode::Char('q') if !app.settings_editing => app.mode = Mode::Normal,
                    KeyCode::Down if !app.settings_editing => {
                        if app.settings_selected_index + 1 < 14 {
                            app.settings_selected_index += 1;
                        }
                    }
                    KeyCode::Up if !app.settings_editing => {
                        if app.settings_selected_index > 0 {
                            app.settings_selected_index -= 1;
                        }
                    }
                    KeyCode::PageUp if !app.settings_editing => {
                        app.settings_selected_index =
                            app.settings_selected_index.saturating_sub(app.config.page_size);
                    }
                    KeyCode::PageDown if !app.settings_editing => {
                        app.settings_selected_index =
                            (app.settings_selected_index + app.config.page_size).min(13);
                    }
                    KeyCode::Home if !app.settings_editing => {
                        app.settings_selected_index = 0;
                    }
                    KeyCode::End if !app.settings_editing => {
                        app.settings_selected_index = 13;
                    }
                    KeyCode::Enter if app.settings_editing => app.commit_settings_edit(),
                    KeyCode::Enter => app.toggle_or_edit_setting(),
                    KeyCode::Char(' ') if !app.settings_editing => app.toggle_or_edit_setting(),
                    KeyCode::Backspace if app.settings_editing => app.input_backspace(),
                    KeyCode::Char(c) if app.settings_editing => app.input_char(c),
                    _ => {}
                }
            }
        }
        Mode::DebugLogs => match code {
            KeyCode::Esc
            | KeyCode::Char('q')
            | KeyCode::Char('D')
            | KeyCode::Char('l')
            | KeyCode::Char('L') => {
                crate::debug_log::info("Exiting debug logs");
                app.mode = Mode::Normal;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let log_count = crate::debug_log::get_logs().len();
                let max_scroll = log_count.saturating_sub(1);
                if app.debug_log_scroll < max_scroll {
                    app.debug_log_scroll += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') if app.debug_log_scroll > 0 => {
                app.debug_log_scroll -= 1;
            }
            KeyCode::PageUp => {
                app.debug_log_scroll = app.debug_log_scroll.saturating_sub(app.config.page_size);
            }
            KeyCode::PageDown => {
                let log_count = crate::debug_log::get_logs().len();
                let max_scroll = log_count.saturating_sub(1);
                app.debug_log_scroll =
                    (app.debug_log_scroll + app.config.page_size).min(max_scroll);
            }
            KeyCode::Home => {
                app.debug_log_scroll = 0;
            }
            KeyCode::End => {
                let log_count = crate::debug_log::get_logs().len();
                app.debug_log_scroll = log_count.saturating_sub(1);
            }
            _ => {}
        },
        Mode::ImportUrlInput => match code {
            KeyCode::Esc => {
                crate::debug_log::info("Cancelled repository import");
                app.mode = Mode::Normal;
                app.input_buffer.clear();
            }
            KeyCode::Enter => {
                app.import_url = app.input_buffer.clone();
                app.input_buffer.clear();

                let repo_name = if let Some(last) = app.import_url.split('/').next_back() {
                    let name = last.trim_end_matches(".git");
                    if name.is_empty() { "repo".to_string() } else { name.to_string() }
                } else {
                    "repo".to_string()
                };

                if let Some(home) = dirs::home_dir() {
                    app.input_buffer = home.join(&repo_name).to_string_lossy().to_string();
                } else {
                    app.input_buffer = format!("./{}", repo_name);
                }

                app.mode = Mode::ImportDestInput;
            }
            KeyCode::Backspace => app.input_backspace(),
            KeyCode::Char(c) => app.input_char(c),
            _ => {}
        },
        Mode::ImportDestInput => match code {
            KeyCode::Esc => {
                app.mode = Mode::ImportUrlInput;
                app.input_buffer = app.import_url.clone();
            }
            KeyCode::Enter => {
                app.import_dest = app.input_buffer.clone();
                app.input_buffer.clear();

                let repo_name = if let Some(last) = app.import_url.split('/').next_back() {
                    let name = last.trim_end_matches(".git");
                    if name.is_empty() { "repo".to_string() } else { name.to_string() }
                } else {
                    "repo".to_string()
                };
                app.input_buffer = repo_name;
                app.mode = Mode::ImportNameInput;
            }
            KeyCode::Backspace => app.input_backspace(),
            KeyCode::Char(c) => app.input_char(c),
            _ => {}
        },
        Mode::ImportNameInput => match code {
            KeyCode::Esc => {
                app.mode = Mode::ImportDestInput;
                app.input_buffer = app.import_dest.clone();
            }
            KeyCode::Enter => {
                app.import_name = app.input_buffer.clone();
                app.input_buffer.clear();
                app.start_import_clone();
            }
            KeyCode::Backspace => app.input_backspace(),
            KeyCode::Char(c) => app.input_char(c),
            _ => {}
        },
        Mode::RepoSearchInput => match code {
            KeyCode::Esc => {
                app.repo_search_query = None;
                app.selected_index = 0;
                app.scroll_top = 0;
                app.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                app.mode = Mode::Normal;
            }
            KeyCode::Backspace => {
                app.input_backspace();
                let query = app.input_buffer.clone();
                if query.is_empty() {
                    app.repo_search_query = None;
                } else {
                    app.repo_search_query = Some(query);
                }
                app.clamp_selection();
                app.clamp_scroll(visible_count);
            }
            KeyCode::Char(c) => {
                app.input_char(c);
                let query = app.input_buffer.clone();
                app.repo_search_query = Some(query);
                app.clamp_selection();
                app.clamp_scroll(visible_count);
            }
            _ => {}
        },
        Mode::Adding => match code {
            KeyCode::Esc => app.cancel_input(),
            KeyCode::Enter => app.commit_add(),
            KeyCode::Backspace => app.input_backspace(),
            KeyCode::Char(c) => app.input_char(c),
            _ => {}
        },
        Mode::BulkAddInput => match code {
            KeyCode::Esc => app.cancel_input(),
            KeyCode::Enter => app.commit_bulk_add(),
            KeyCode::Backspace => app.input_backspace(),
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if app.config.fzf.enabled {
                    app.start_bulk_add();
                } else {
                    app.error_message =
                        Some("FZF is disabled in settings. Enable it first.".to_string());
                }
            }
            KeyCode::Tab => {
                if app.config.fzf.enabled {
                    app.start_bulk_add();
                } else {
                    app.error_message =
                        Some("FZF is disabled in settings. Enable it first.".to_string());
                }
            }
            KeyCode::Char(c) => app.input_char(c),
            _ => {}
        },
        Mode::Editing => match code {
            KeyCode::Esc => app.cancel_input(),
            KeyCode::Enter => app.commit_edit(),
            KeyCode::Backspace => app.input_backspace(),
            KeyCode::Char(c) => app.input_char(c),
            _ => {}
        },
        Mode::ConfirmDelete => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.close_dialog(),
            _ => {}
        },
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
        Mode::Help => { if crate::popups::help::HelpPopup::handle_event(app, key) { return true; } }
        Mode::About => { if crate::popups::about::AboutPopup::handle_event(app, key) { return true; } }
        Mode::Detail => { if crate::tabs::route_detail_event(app, key) { return true; } }

        Mode::Inspect => { if crate::popups::inspect::InspectPopup::handle_event(app, key) { return true; } }

        Mode::DetailHelp => { if crate::popups::help::DetailHelpPopup::handle_event(app, key) { return true; } }
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
        Mode::Logs => { if crate::tabs::logs::LogsTab::handle_event(app, key) { return true; } }
        Mode::RemotePicker => match code {
            KeyCode::Up => app.remote_picker_up(),
            KeyCode::Down => app.remote_picker_down(),
            KeyCode::Enter => app.confirm_remote_picker(),
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => app.cancel_remote_picker(),
            _ => {}
        },
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
        
        Mode::BranchDeleteConfirm | Mode::BranchPushConfirm | Mode::BranchMergeConfirm | Mode::MergeAbortConfirm | Mode::MergeContinueConfirm | Mode::BranchRebaseConfirm | Mode::BranchInteractiveRebaseConfirm | Mode::DiscardChangesConfirm | Mode::RevertConfirm | Mode::TagDeleteConfirm | Mode::TagPushConfirm | Mode::TagPushAllConfirm | Mode::StashDeleteConfirm | Mode::BranchCheckoutConfirm | Mode::TagCheckoutConfirm | Mode::RemoteDeleteConfirm => {
            let ev = crossterm::event::Event::Key(key);
            if app.confirm_popup.event(&ev).unwrap_or(crate::components::EventState::NotConsumed).is_consumed() { return true; }
        }
        Mode::BranchCreateInput | Mode::TagCreateInput | Mode::StashCreateInput | Mode::RemoteAddNameInput | Mode::RemoteAddUrlInput => {
            let ev = crossterm::event::Event::Key(key);
            if app.generic_input_popup.event(&ev).unwrap_or(crate::components::EventState::NotConsumed).is_consumed() { return true; }
        }

        Mode::CommitInput => {
            let ev = crossterm::event::Event::Key(key);
            if app.commit_popup.event(&ev).unwrap_or(crate::components::EventState::NotConsumed).is_consumed() { return true; }
        }
    }
    true
}

