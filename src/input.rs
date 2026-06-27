//! Keystroke dispatch.
//!
//! `handle_key` reads `app.mode` and routes the keystroke to the
//! appropriate `App` method. Returns `false` when the user has asked to
//! quit, `true` otherwise.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, DetailSection, Mode};
use crate::components::Component;

/// Dispatch a key press. Returns `false` if the user requested quit.
pub fn handle_key(app: &mut App, key: KeyEvent, visible_count: usize) -> bool {
    app.drain_queue();
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

    let detail_focus = app.detail_focus; // Copy before borrow in match
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
        Mode::Help => match code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.close_dialog();
            }
            KeyCode::Up => {
                app.help_scroll_up();
            }
            KeyCode::Down => {
                app.help_scroll_down();
            }
            KeyCode::PageUp => {
                app.help_scroll_page_up(app.config.page_size);
            }
            KeyCode::PageDown => {
                app.help_scroll_page_down(app.config.page_size);
            }
            KeyCode::Home => {
                app.help_scroll_to_top();
            }
            KeyCode::End => {
                app.help_scroll_to_bottom();
            }
            _ => {}
        },
        Mode::About => match code {
            KeyCode::Char('v')
            | KeyCode::Char('V')
            | KeyCode::Esc
            | KeyCode::Char('q')
            | KeyCode::Char('Q') => {
                app.close_dialog();
            }
            _ => {}
        },
        Mode::Detail => match code {
            KeyCode::Esc => {
                if app.inspect_full_diff {
                    app.inspect_full_diff = false;
                } else if app.commit_list.search_query.is_some() {
                    app.cancel_commit_search();
                } else {
                    app.close_detail();
                }
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => app.close_detail(),
            KeyCode::Char('?') => app.open_detail_help(),
            KeyCode::Char('R') => {
                app.resync_detail();
                app.status_message = Some("Refreshed".to_string());
            }
            KeyCode::Tab => {
                app.inspect_full_diff = false;
                app.detail_tab = (app.detail_tab + 1) % 8;
                app.set_default_focus_for_tab();
                if app.config.resync_on_tab_change {
                    app.resync_detail();
                }
            }
            KeyCode::BackTab => {
                app.inspect_full_diff = false;
                app.detail_tab = if app.detail_tab == 0 { 7 } else { app.detail_tab - 1 };
                app.set_default_focus_for_tab();
                if app.config.resync_on_tab_change {
                    app.resync_detail();
                }
            }
            KeyCode::Char('1') => {
                app.inspect_full_diff = false;
                app.detail_tab = 0;
                app.detail_focus = DetailSection::Commits;
                if app.config.resync_on_tab_change {
                    app.resync_detail();
                }
            }
            KeyCode::Char('2') => {
                app.inspect_full_diff = false;
                app.detail_tab = 1;
                app.detail_focus = DetailSection::Files;
                if app.config.resync_on_tab_change {
                    app.resync_detail();
                }
            }
            KeyCode::Char('3') => {
                app.inspect_full_diff = false;
                app.detail_tab = 2;
                if app.config.resync_on_tab_change {
                    app.resync_detail();
                }
            }
            KeyCode::Char('4') => {
                app.inspect_full_diff = false;
                app.detail_tab = 3;
                app.detail_focus = DetailSection::LocalBranches;
                if app.config.resync_on_tab_change {
                    app.resync_detail();
                }
            }
            KeyCode::Char('5') => {
                app.inspect_full_diff = false;
                app.detail_tab = 4;
                app.detail_focus = DetailSection::LocalTags;
                let attempted =
                    if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
                        info.remote_tags_attempted
                    } else {
                        false
                    };
                if !attempted {
                    app.fetch_remote_tags(true);
                }
                if app.config.resync_on_tab_change {
                    app.resync_detail();
                }
            }
            KeyCode::Char('6') => {
                app.inspect_full_diff = false;
                app.detail_tab = 5;
                app.detail_focus = DetailSection::Remotes;
                let remote_name =
                    if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
                        info.remotes
                            .get(app.branch_list.remote_selection)
                            .or_else(|| info.remotes.first())
                            .map(|r| r.name.clone())
                    } else {
                        None
                    };
                if let Some(name) = remote_name {
                    app.fetch_remote(&name);
                }
                if app.config.resync_on_tab_change {
                    app.resync_detail();
                }
            }
            KeyCode::Char('7') => {
                app.inspect_full_diff = false;
                app.detail_tab = 6;
                app.detail_focus = DetailSection::Stashes;
                if app.config.resync_on_tab_change {
                    app.resync_detail();
                }
            }
            KeyCode::Char('8') => {
                app.inspect_full_diff = false;
                app.detail_tab = 7;
                app.detail_focus = DetailSection::Commits;
                if app.config.resync_on_tab_change {
                    app.resync_detail();
                }
            }
            _ if app.detail_tab == 0 => {
                let ev = crossterm::event::Event::Key(key);
                if detail_focus == DetailSection::Commits || detail_focus == DetailSection::CommitDetails {
                    if app.commit_list.event(&ev).unwrap_or(crate::components::EventState::NotConsumed).is_consumed() { return true; }
                } else if detail_focus == DetailSection::StagingDetails || detail_focus == DetailSection::ConflictDiff {
                    if app.diff.event(&ev).unwrap_or(crate::components::EventState::NotConsumed).is_consumed() { return true; }
                    
                    // Staging details specific actions
                    match code {
                        KeyCode::Enter => {
                            if app.is_uncommitted_selected() {
                                if app.last_staging_focus == DetailSection::Staged {
                                    app.unstage_selected_hunk();
                                } else if app.last_staging_focus == DetailSection::Unstaged {
                                    app.stage_selected_hunk();
                                }
                            }
                        }
                        KeyCode::Char('X') if app.is_uncommitted_selected() => app.request_discard_all_changes(),
                        KeyCode::Char('s') | KeyCode::Char('S') if app.is_uncommitted_selected() => app.start_stash_create(),
                        _ => {}
                    }
                } else if detail_focus == DetailSection::Staged || detail_focus == DetailSection::Unstaged || detail_focus == DetailSection::Conflicts {
                    if app.status_list.event(&ev).unwrap_or(crate::components::EventState::NotConsumed).is_consumed() { return true; }
                    
                    // Status list specific enter/conflicts logic
                    match code {
                        KeyCode::Enter if detail_focus == DetailSection::Staged && app.is_uncommitted_selected() => app.unstage_selected_file(),
                        KeyCode::Enter if detail_focus == DetailSection::Unstaged && app.is_uncommitted_selected() => app.stage_selected_file(),
                        KeyCode::Enter | KeyCode::Right if detail_focus == DetailSection::Conflicts => {
                            app.mode = Mode::Inspect;
                            app.last_staging_focus = DetailSection::Conflicts;
                            app.detail_focus = DetailSection::ConflictDiff;
                            app.diff.diff_scroll = 0;
                            app.refresh_staging_diff();
                        }
                        KeyCode::Char('o') if detail_focus == DetailSection::Conflicts && app.is_uncommitted_selected() => app.resolve_conflict_ours(),
                        KeyCode::Char('t') if detail_focus == DetailSection::Conflicts && app.is_uncommitted_selected() => app.resolve_conflict_theirs(),
                        KeyCode::Char('r') if detail_focus == DetailSection::Conflicts && app.is_uncommitted_selected() => app.mark_conflict_resolved(),
                        KeyCode::Char('A') if detail_focus == DetailSection::Conflicts && app.is_uncommitted_selected() => app.mode = Mode::MergeAbortConfirm,
                        KeyCode::Char('C') if detail_focus == DetailSection::Conflicts && app.is_uncommitted_selected() => app.mode = Mode::MergeContinueConfirm,
                        _ => {}
                    }
                }
            },
            _ if app.detail_tab == 1 => {
                let ev = crossterm::event::Event::Key(key);
                if detail_focus == DetailSection::Files {
                    if app.file_tree.event(&ev).unwrap_or(crate::components::EventState::NotConsumed).is_consumed() { return true; }
                } else if detail_focus == DetailSection::FileContent {
                    // map file content scrolling to the tree's internal event handler
                    match code {
                        KeyCode::Up => app.file_tree.queue.push(crate::queue::InternalEvent::FileContentUp),
                        KeyCode::Down => app.file_tree.queue.push(crate::queue::InternalEvent::FileContentDown),
                        KeyCode::PageUp => app.file_tree.queue.push(crate::queue::InternalEvent::FileContentPageUp),
                        KeyCode::PageDown => app.file_tree.queue.push(crate::queue::InternalEvent::FileContentPageDown),
                        KeyCode::Home => app.file_tree.queue.push(crate::queue::InternalEvent::FileContentTop),
                        KeyCode::End => app.file_tree.queue.push(crate::queue::InternalEvent::FileContentBottom),
                        _ => {}
                    }
                }
            },
            _ if app.detail_tab == 3 => {
                let ev = crossterm::event::Event::Key(key);
                if detail_focus == DetailSection::LocalBranches {
                    if app.branch_list.event(&ev).unwrap_or(crate::components::EventState::NotConsumed).is_consumed() { return true; }
                } else if detail_focus == DetailSection::Remotes {
                    // Also routed through branch list for now, we map remote keys explicitly here since branch list event() is mapped to local currently.
                    match code {
                        KeyCode::Up => app.branch_list.queue.push(crate::queue::InternalEvent::RemoteBranchUp),
                        KeyCode::Down => app.branch_list.queue.push(crate::queue::InternalEvent::RemoteBranchDown),
                        KeyCode::PageUp => app.branch_list.queue.push(crate::queue::InternalEvent::RemoteBranchPageUp),
                        KeyCode::PageDown => app.branch_list.queue.push(crate::queue::InternalEvent::RemoteBranchPageDown),
                        KeyCode::Home => app.branch_list.queue.push(crate::queue::InternalEvent::RemoteBranchTop),
                        KeyCode::End => app.branch_list.queue.push(crate::queue::InternalEvent::RemoteBranchBottom),
                        KeyCode::Char('f') | KeyCode::Char('F') => app.branch_list.queue.push(crate::queue::InternalEvent::FetchRemote),
                        KeyCode::Char('d') | KeyCode::Char('D') => app.branch_list.queue.push(crate::queue::InternalEvent::RequestDeleteRemote),
                        _ => {}
                    }
                }
            },
            _ if app.detail_tab == 4 => {
                let ev = crossterm::event::Event::Key(key);
                if app.tag_list.event(&ev).unwrap_or(crate::components::EventState::NotConsumed).is_consumed() { return true; }
            },
            _ if app.detail_tab == 5 => {
                match code {
                    KeyCode::Up => app.branch_list.queue.push(crate::queue::InternalEvent::RemoteBranchUp),
                    KeyCode::Down => app.branch_list.queue.push(crate::queue::InternalEvent::RemoteBranchDown),
                    KeyCode::PageUp => app.branch_list.queue.push(crate::queue::InternalEvent::RemoteBranchPageUp),
                    KeyCode::PageDown => app.branch_list.queue.push(crate::queue::InternalEvent::RemoteBranchPageDown),
                    KeyCode::Home => app.branch_list.queue.push(crate::queue::InternalEvent::RemoteBranchTop),
                    KeyCode::End => app.branch_list.queue.push(crate::queue::InternalEvent::RemoteBranchBottom),
                    KeyCode::Char('f') | KeyCode::Char('F') => app.branch_list.queue.push(crate::queue::InternalEvent::FetchRemote),
                    KeyCode::Char('a') | KeyCode::Char('A') => app.branch_list.queue.push(crate::queue::InternalEvent::StartRemoteAdd),
                    KeyCode::Char('d') | KeyCode::Char('D') => app.branch_list.queue.push(crate::queue::InternalEvent::RequestDeleteRemote),
                    _ => {}
                }
            },
            _ if app.detail_tab == 6 => match code {
                KeyCode::Char('w') => app.cycle_detail_focus(false),
                KeyCode::Char('W') => app.cycle_detail_focus(true),
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    if detail_focus == DetailSection::Stashes {
                        app.request_stash_delete();
                    }
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    if detail_focus == DetailSection::Stashes {
                        app.request_stash_apply();
                    }
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    if detail_focus == DetailSection::Stashes {
                        app.start_stash_create();
                    }
                }
                KeyCode::Up => match detail_focus {
                    DetailSection::Stashes => app.stash_up(),
                    DetailSection::StashedFiles => app.stash_file_up(),
                    DetailSection::StagingDetails => app.diff.diff_scroll_up(),
                    _ => {}
                },
                KeyCode::Down => match detail_focus {
                    DetailSection::Stashes => app.stash_down(),
                    DetailSection::StashedFiles => app.stash_file_down(),
                    DetailSection::StagingDetails => app.diff.diff_scroll_down(),
                    _ => {}
                },
                KeyCode::PageUp => match detail_focus {
                    DetailSection::Stashes => app.stash_page_up(app.config.page_size),
                    DetailSection::StashedFiles => app.stash_file_page_up(app.config.page_size),
                    DetailSection::StagingDetails => app.diff.diff_scroll_page_up(app.config.page_size),
                    _ => {}
                },
                KeyCode::PageDown => match detail_focus {
                    DetailSection::Stashes => app.stash_page_down(app.config.page_size),
                    DetailSection::StashedFiles => app.stash_file_page_down(app.config.page_size),
                    DetailSection::StagingDetails => {
                        app.diff.diff_scroll_page_down(app.config.page_size)
                    }
                    _ => {}
                },
                KeyCode::Home => match detail_focus {
                    DetailSection::Stashes => app.stash_to_top(),
                    DetailSection::StashedFiles => app.stash_file_to_top(),
                    DetailSection::StagingDetails => app.diff.diff_scroll_to_top(),
                    _ => {}
                },
                KeyCode::End => match detail_focus {
                    DetailSection::Stashes => app.stash_to_bottom(),
                    DetailSection::StashedFiles => app.stash_file_to_bottom(),
                    DetailSection::StagingDetails => app.diff.diff_scroll_to_bottom(),
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        },

        Mode::Inspect => match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                if app.inspect_full_diff {
                    app.inspect_full_diff = false;
                } else if app.in_logs_ui {
                    app.mode = Mode::Logs;
                } else {
                    app.mode = Mode::Detail;
                    app.detail_focus = DetailSection::Commits;
                }
            }
            KeyCode::Char('?') => {
                app.open_detail_help();
            }
            KeyCode::Char('w') | KeyCode::Tab => {
                if app.is_uncommitted_selected() {
                    let mut next_focus = match app.detail_focus {
                        DetailSection::Staged => DetailSection::Unstaged,
                        DetailSection::Unstaged => DetailSection::Conflicts,
                        DetailSection::Conflicts => DetailSection::StagingDetails,
                        DetailSection::StagingDetails => DetailSection::ConflictDiff,
                        _ => DetailSection::Staged,
                    };
                    for _ in 0..6 {
                        let skip = match next_focus {
                            DetailSection::Staged => app.is_staged_empty(),
                            DetailSection::Unstaged => app.is_unstaged_empty(),
                            DetailSection::Conflicts => app.is_conflicted_empty(),
                            DetailSection::StagingDetails => {
                                app.is_staged_empty() && app.is_unstaged_empty()
                            }
                            DetailSection::ConflictDiff => app.is_conflicted_empty(),
                            _ => false,
                        };
                        if skip {
                            next_focus = match next_focus {
                                DetailSection::Staged => DetailSection::Unstaged,
                                DetailSection::Unstaged => DetailSection::Conflicts,
                                DetailSection::Conflicts => DetailSection::StagingDetails,
                                DetailSection::StagingDetails => DetailSection::ConflictDiff,
                                _ => DetailSection::Staged,
                            };
                        } else {
                            break;
                        }
                    }
                    if app.detail_focus == DetailSection::Staged
                        || app.detail_focus == DetailSection::Unstaged
                        || app.detail_focus == DetailSection::Conflicts
                    {
                        app.last_staging_focus = app.detail_focus;
                    }
                    app.detail_focus = next_focus;
                    app.diff.diff_scroll = 0;
                    app.refresh_staging_diff();
                } else {
                    let mut next_focus = match app.detail_focus {
                        DetailSection::Staged => DetailSection::CommitDetails,
                        DetailSection::CommitDetails => DetailSection::StagingDetails,
                        _ => DetailSection::Staged,
                    };
                    for _ in 0..3 {
                        let skip = match next_focus {
                            DetailSection::Staged => app.is_selected_commit_empty(),
                            DetailSection::CommitDetails => false,
                            DetailSection::StagingDetails => app.is_selected_commit_empty(),
                            _ => false,
                        };
                        if skip {
                            next_focus = match next_focus {
                                DetailSection::Staged => DetailSection::CommitDetails,
                                DetailSection::CommitDetails => DetailSection::StagingDetails,
                                _ => DetailSection::Staged,
                            };
                        } else {
                            break;
                        }
                    }
                    app.detail_focus = next_focus;
                }
            }
            KeyCode::Char('W') => {
                if app.is_uncommitted_selected() {
                    let mut next_focus = match app.detail_focus {
                        DetailSection::Staged => DetailSection::ConflictDiff,
                        DetailSection::ConflictDiff => DetailSection::StagingDetails,
                        DetailSection::StagingDetails => DetailSection::Conflicts,
                        DetailSection::Conflicts => DetailSection::Unstaged,
                        _ => DetailSection::Staged,
                    };
                    for _ in 0..6 {
                        let skip = match next_focus {
                            DetailSection::Staged => app.is_staged_empty(),
                            DetailSection::Unstaged => app.is_unstaged_empty(),
                            DetailSection::Conflicts => app.is_conflicted_empty(),
                            DetailSection::StagingDetails => {
                                app.is_staged_empty() && app.is_unstaged_empty()
                            }
                            DetailSection::ConflictDiff => app.is_conflicted_empty(),
                            _ => false,
                        };
                        if skip {
                            next_focus = match next_focus {
                                DetailSection::Staged => DetailSection::ConflictDiff,
                                DetailSection::ConflictDiff => DetailSection::StagingDetails,
                                DetailSection::StagingDetails => DetailSection::Conflicts,
                                DetailSection::Conflicts => DetailSection::Unstaged,
                                _ => DetailSection::Staged,
                            };
                        } else {
                            break;
                        }
                    }
                    if app.detail_focus == DetailSection::Staged
                        || app.detail_focus == DetailSection::Unstaged
                        || app.detail_focus == DetailSection::Conflicts
                    {
                        app.last_staging_focus = app.detail_focus;
                    }
                    app.detail_focus = next_focus;
                    app.diff.diff_scroll = 0;
                    app.refresh_staging_diff();
                } else {
                    let mut next_focus = match app.detail_focus {
                        DetailSection::Staged => DetailSection::StagingDetails,
                        DetailSection::StagingDetails => DetailSection::CommitDetails,
                        _ => DetailSection::Staged,
                    };
                    for _ in 0..3 {
                        let skip = match next_focus {
                            DetailSection::Staged => app.is_selected_commit_empty(),
                            DetailSection::CommitDetails => false,
                            DetailSection::StagingDetails => app.is_selected_commit_empty(),
                            _ => false,
                        };
                        if skip {
                            next_focus = match next_focus {
                                DetailSection::Staged => DetailSection::StagingDetails,
                                DetailSection::StagingDetails => DetailSection::CommitDetails,
                                _ => DetailSection::Staged,
                            };
                        } else {
                            break;
                        }
                    }
                    app.detail_focus = next_focus;
                }
            }
            KeyCode::Right => {
                if app.detail_focus == DetailSection::StagingDetails
                    || app.detail_focus == DetailSection::ConflictDiff
                {
                    app.inspect_full_diff = true;
                } else if app.is_uncommitted_selected() {
                    if app.detail_focus == DetailSection::Staged
                        || app.detail_focus == DetailSection::Unstaged
                        || app.detail_focus == DetailSection::Conflicts
                    {
                        app.last_staging_focus = app.detail_focus;
                        if app.detail_focus == DetailSection::Conflicts {
                            app.detail_focus = DetailSection::ConflictDiff;
                        } else {
                            app.detail_focus = DetailSection::StagingDetails;
                        }
                    }
                } else {
                    if app.detail_focus == DetailSection::Staged
                        || app.detail_focus == DetailSection::CommitDetails
                    {
                        app.detail_focus = DetailSection::StagingDetails;
                    }
                }
            }
            KeyCode::Left => {
                if app.inspect_full_diff {
                    app.inspect_full_diff = false;
                } else if app.detail_focus == DetailSection::StagingDetails
                    || app.detail_focus == DetailSection::ConflictDiff
                {
                    if app.is_uncommitted_selected() {
                        app.detail_focus = app.last_staging_focus;
                    } else {
                        app.detail_focus = DetailSection::CommitDetails;
                    }
                }
            }
            KeyCode::Up => {
                if app.detail_focus == DetailSection::Staged
                    || app.detail_focus == DetailSection::Unstaged
                    || app.detail_focus == DetailSection::Conflicts
                {
                    if app.detail_focus == DetailSection::Conflicts {
                        app.conflict_file_up();
                    } else if app.is_uncommitted_selected() {
                        app.staging_file_up();
                    } else {
                        app.detail_file_up();
                    }
                } else if app.detail_focus == DetailSection::CommitDetails {
                    app.commit_list.details_scroll_up();
                } else {
                    if app.is_uncommitted_selected() {
                        if app.diff.diff_line_mode {
                            app.diff_line_up();
                        } else {
                            app.diff_hunk_up();
                        }
                    } else {
                        app.diff.diff_scroll_up();
                    }
                }
            }
            KeyCode::Down => {
                if app.detail_focus == DetailSection::Staged
                    || app.detail_focus == DetailSection::Unstaged
                    || app.detail_focus == DetailSection::Conflicts
                {
                    if app.detail_focus == DetailSection::Conflicts {
                        app.conflict_file_down();
                    } else if app.is_uncommitted_selected() {
                        app.staging_file_down();
                    } else {
                        app.detail_file_down();
                    }
                } else if app.detail_focus == DetailSection::CommitDetails {
                    app.commit_list.details_scroll_down();
                } else {
                    if app.is_uncommitted_selected() {
                        if app.diff.diff_line_mode {
                            app.diff_line_down();
                        } else {
                            app.diff_hunk_down();
                        }
                    } else {
                        app.diff.diff_scroll_down();
                    }
                }
            }
            KeyCode::PageUp => {
                if app.detail_focus == DetailSection::Staged
                    || app.detail_focus == DetailSection::Unstaged
                    || app.detail_focus == DetailSection::Conflicts
                {
                    if app.detail_focus == DetailSection::Conflicts {
                        for _ in 0..app.config.page_size {
                            app.conflict_file_up();
                        }
                    } else if app.is_uncommitted_selected() {
                        for _ in 0..app.config.page_size {
                            app.staging_file_up();
                        }
                    } else {
                        for _ in 0..app.config.page_size {
                            app.detail_file_up();
                        }
                    }
                } else if app.detail_focus == DetailSection::CommitDetails {
                    for _ in 0..app.config.page_size {
                        app.commit_list.details_scroll_up();
                    }
                } else {
                    app.diff.diff_scroll_page_up(app.config.page_size);
                }
            }
            KeyCode::PageDown => {
                if app.detail_focus == DetailSection::Staged
                    || app.detail_focus == DetailSection::Unstaged
                    || app.detail_focus == DetailSection::Conflicts
                {
                    if app.detail_focus == DetailSection::Conflicts {
                        for _ in 0..app.config.page_size {
                            app.conflict_file_down();
                        }
                    } else if app.is_uncommitted_selected() {
                        for _ in 0..app.config.page_size {
                            app.staging_file_down();
                        }
                    } else {
                        for _ in 0..app.config.page_size {
                            app.detail_file_down();
                        }
                    }
                } else if app.detail_focus == DetailSection::CommitDetails {
                    for _ in 0..app.config.page_size {
                        app.commit_list.details_scroll_down();
                    }
                } else {
                    app.diff.diff_scroll_page_down(app.config.page_size);
                }
            }
            KeyCode::Home => {
                if app.detail_focus == DetailSection::Staged
                    || app.detail_focus == DetailSection::Unstaged
                    || app.detail_focus == DetailSection::Conflicts
                {
                    if app.detail_focus == DetailSection::Conflicts {
                        app.status_list.conflict_file_selection = 0;
                        app.refresh_staging_diff();
                    } else if app.is_uncommitted_selected() {
                        app.status_list.staging_file_selection = 0;
                        app.refresh_staging_diff();
                    } else {
                        app.status_list.file_selection = 0;
                        app.refresh_file_diff();
                    }
                } else if app.detail_focus == DetailSection::CommitDetails {
                    app.commit_list.details_scroll = 0;
                } else {
                    if app.is_uncommitted_selected() {
                        if app.diff.diff_line_mode {
                            app.diff.diff_line_selection = 0;
                            app.diff.diff_scroll = 0;
                        } else {
                            app.diff.diff_hunk_selection = 0;
                            app.scroll_to_selected_hunk();
                        }
                    } else {
                        app.diff.diff_scroll_to_top();
                    }
                }
            }
            KeyCode::End => {
                if app.detail_focus == DetailSection::Staged
                    || app.detail_focus == DetailSection::Unstaged
                    || app.detail_focus == DetailSection::Conflicts
                {
                    if app.detail_focus == DetailSection::Conflicts {
                        let len = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                                info.changes.conflicted.len()
                            }
                            _ => 0,
                        };
                        app.status_list.conflict_file_selection = len.saturating_sub(1);
                        app.refresh_staging_diff();
                    } else if app.is_uncommitted_selected() {
                        app.status_list.staging_file_selection = app.staging_file_total().saturating_sub(1);
                        app.refresh_staging_diff();
                    } else {
                        app.status_list.file_selection = app.file_total().saturating_sub(1);
                        app.refresh_file_diff();
                    }
                } else if app.detail_focus == DetailSection::CommitDetails {
                    app.commit_list.details_scroll = usize::MAX;
                } else {
                    if app.is_uncommitted_selected() {
                        if app.diff.diff_line_mode {
                            app.diff.diff_line_selection = app.diff.file_diff.len().saturating_sub(1);
                            app.diff.diff_scroll = app.diff.diff_line_selection.saturating_sub(17);
                        } else {
                            let hunk_count = app.get_diff_hunk_ranges().len();
                            app.diff.diff_hunk_selection = hunk_count.saturating_sub(1);
                            app.scroll_to_selected_hunk();
                        }
                    } else {
                        app.diff.diff_scroll_to_bottom();
                    }
                }
            }
            KeyCode::Enter if app.is_uncommitted_selected() => {
                if app.detail_focus == DetailSection::Staged {
                    app.unstage_selected_file();
                } else if app.detail_focus == DetailSection::Unstaged {
                    app.stage_selected_file();
                } else if app.detail_focus == DetailSection::Conflicts {
                    app.detail_focus = DetailSection::ConflictDiff;
                    app.diff.diff_scroll = 0;
                    app.refresh_staging_diff();
                } else if app.detail_focus == DetailSection::StagingDetails {
                    if app.diff.diff_line_mode {
                        if app.last_staging_focus == DetailSection::Staged {
                            app.unstage_selected_line();
                        } else if app.last_staging_focus == DetailSection::Unstaged {
                            app.stage_selected_line();
                        }
                    } else {
                        if app.last_staging_focus == DetailSection::Staged {
                            app.unstage_selected_hunk();
                        } else if app.last_staging_focus == DetailSection::Unstaged {
                            app.stage_selected_hunk();
                        }
                    }
                }
            }
            KeyCode::Delete
                if app.is_uncommitted_selected()
                    && app.detail_focus == DetailSection::StagingDetails
                    && app.last_staging_focus == DetailSection::Unstaged =>
            {
                if app.diff.diff_line_mode {
                    app.discard_selected_line();
                } else {
                    app.discard_selected_hunk();
                }
            }
            KeyCode::Char('x') if app.is_uncommitted_selected() => {
                if app.detail_focus == DetailSection::Staged
                    || app.detail_focus == DetailSection::Unstaged
                {
                    app.request_discard_changes();
                } else if app.detail_focus == DetailSection::StagingDetails
                    && app.last_staging_focus == DetailSection::Unstaged
                {
                    if app.diff.diff_line_mode {
                        app.discard_selected_line();
                    } else {
                        app.discard_selected_hunk();
                    }
                }
            }
            KeyCode::Char('X') if app.is_uncommitted_selected() => {
                if app.detail_focus == DetailSection::Staged
                    || app.detail_focus == DetailSection::Unstaged
                {
                    app.request_discard_all_changes();
                } else if app.detail_focus == DetailSection::StagingDetails
                    && app.last_staging_focus == DetailSection::Unstaged
                {
                    if app.diff.diff_line_mode {
                        app.discard_selected_line();
                    } else {
                        app.discard_selected_hunk();
                    }
                }
            }
            KeyCode::Char('a') if app.is_uncommitted_selected() => {
                if app.detail_focus == DetailSection::Unstaged {
                    app.stage_all_changes();
                } else if app.detail_focus == DetailSection::Staged {
                    app.unstage_all_changes();
                }
            }
            KeyCode::Char('l') | KeyCode::Char('L')
                if app.is_uncommitted_selected()
                    && app.detail_focus == DetailSection::StagingDetails =>
            {
                app.toggle_diff_line_mode();
            }
            KeyCode::Char('o')
                if (app.detail_focus == DetailSection::Conflicts
                    || app.detail_focus == DetailSection::ConflictDiff)
                    && app.is_uncommitted_selected() =>
            {
                app.resolve_conflict_ours();
            }
            KeyCode::Char('t')
                if (app.detail_focus == DetailSection::Conflicts
                    || app.detail_focus == DetailSection::ConflictDiff)
                    && app.is_uncommitted_selected() =>
            {
                app.resolve_conflict_theirs();
            }
            KeyCode::Char('r')
                if (app.detail_focus == DetailSection::Conflicts
                    || app.detail_focus == DetailSection::ConflictDiff)
                    && app.is_uncommitted_selected() =>
            {
                app.mark_conflict_resolved();
            }
            KeyCode::Char('A')
                if (app.detail_focus == DetailSection::Conflicts
                    || app.detail_focus == DetailSection::ConflictDiff)
                    && app.is_uncommitted_selected() =>
            {
                app.mode = Mode::MergeAbortConfirm;
            }
            KeyCode::Char('C')
                if (app.detail_focus == DetailSection::Conflicts
                    || app.detail_focus == DetailSection::ConflictDiff)
                    && app.is_uncommitted_selected() =>
            {
                app.mode = Mode::MergeContinueConfirm;
            }
            KeyCode::Char('c') if app.is_uncommitted_selected() => {
                app.start_commit();
            }
            KeyCode::Char('C') if app.is_uncommitted_selected() => {
                app.start_commit_amend();
            }
            _ => {}
        },

        Mode::DetailHelp => match code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.close_detail_help();
            }
            KeyCode::Up => {
                app.help_scroll_up();
            }
            KeyCode::Down => {
                app.help_scroll_down();
            }
            KeyCode::PageUp => {
                app.help_scroll_page_up(app.config.page_size);
            }
            KeyCode::PageDown => {
                app.help_scroll_page_down(app.config.page_size);
            }
            KeyCode::Home => {
                app.help_scroll_to_top();
            }
            KeyCode::End => {
                app.help_scroll_to_bottom();
            }
            _ => {}
        },
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
        Mode::Logs => match code {
            KeyCode::Up => app.detail_commit_up(),
            KeyCode::Down => app.detail_commit_down(),
            KeyCode::PageUp => app.detail_commit_page_up(app.config.page_size),
            KeyCode::PageDown => app.detail_commit_page_down(app.config.page_size),
            KeyCode::Home => app.detail_commit_to_top(),
            KeyCode::End => app.detail_commit_to_bottom(),
            KeyCode::Char('f') => {
                app.search_column_selection = 0;
                app.mode = Mode::SearchColumnPicker;
            }
            KeyCode::Enter => {
                app.mode = Mode::Inspect;
                if app.is_uncommitted_selected() {
                    app.detail_focus = DetailSection::Staged;
                    app.last_staging_focus = DetailSection::Staged;
                    app.status_list.staging_file_selection = 0;
                } else {
                    app.detail_focus = DetailSection::Staged;
                    app.last_staging_focus = DetailSection::Staged;
                    app.status_list.file_selection = 0;
                }
                app.diff.diff_scroll = 0;
                app.refresh_file_diff();
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.in_logs_ui = false;
                app.commit_list.search_query = None;
                app.mode = Mode::Detail;
            }
            _ => {}
        },
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

