//! Keystroke dispatch.
//!
//! `handle_key` reads `app.mode` and routes the keystroke to the
//! appropriate `App` method. Returns `false` when the user has asked to
//! quit, `true` otherwise.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Position;

use crate::app::{App, DetailSection, Mode, RemotePickerAction, Splitter};

/// Dispatch a key press. Returns `false` if the user requested quit.
pub fn handle_key(app: &mut App, key: KeyEvent, visible_count: usize) -> bool {
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

    // Toggle status bar expanded mode with '.' (except in text input fields)
    let is_text_input = matches!(
        app.mode,
        Mode::Adding
            | Mode::Editing
            | Mode::BranchCreateInput
            | Mode::TagCreateInput
            | Mode::StashCreateInput
            | Mode::RepoSearchInput
    ) || (matches!(app.mode, Mode::CommitInput) && app.commit_editing)
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
            KeyCode::Char('e') => app.start_edit(),
            KeyCode::Char('d') => app.request_delete(),
            KeyCode::Char('?') => app.open_help(),
            KeyCode::Char('R') => app.refresh_selected_status(),
            KeyCode::Char('o') => app.cycle_sort_order(),
            KeyCode::Char('O') => app.toggle_sort_reverse(),
            KeyCode::Char('p') => app.toggle_pin_selected(),
            KeyCode::Char('s') => {
                app.mode = Mode::Settings;
                app.settings_selected_index = 0;
                app.settings_editing = false;
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
                        app.settings_theme_index = app
                            .settings_theme_index
                            .saturating_sub(app.config.page_size);
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
                        if app.settings_selected_index + 1 < 10 {
                            app.settings_selected_index += 1;
                        }
                    }
                    KeyCode::Up if !app.settings_editing => {
                        if app.settings_selected_index > 0 {
                            app.settings_selected_index -= 1;
                        }
                    }
                    KeyCode::PageUp if !app.settings_editing => {
                        app.settings_selected_index = app
                            .settings_selected_index
                            .saturating_sub(app.config.page_size);
                    }
                    KeyCode::PageDown if !app.settings_editing => {
                        app.settings_selected_index =
                            (app.settings_selected_index + app.config.page_size).min(9);
                    }
                    KeyCode::Home if !app.settings_editing => {
                        app.settings_selected_index = 0;
                    }
                    KeyCode::End if !app.settings_editing => {
                        app.settings_selected_index = 9;
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
        Mode::BranchCreateInput => match code {
            KeyCode::Esc => app.cancel_branch_create(),
            KeyCode::Enter => app.commit_branch_create(),
            KeyCode::Backspace => app.input_backspace(),
            KeyCode::Char(c) => app.input_char(c),
            _ => {}
        },
        Mode::TagCreateInput => match code {
            KeyCode::Esc => {
                app.tag_action_target_oid = None;
                app.mode = Mode::Detail;
            }
            KeyCode::Enter => app.commit_tag_create(),
            KeyCode::Backspace => app.input_backspace(),
            KeyCode::Char(c) => app.input_char(c),
            _ => {}
        },
        Mode::StashCreateInput => match code {
            KeyCode::Esc => {
                app.mode = Mode::Detail;
            }
            KeyCode::Enter => app.commit_stash_create(),
            KeyCode::Backspace => app.input_backspace(),
            KeyCode::Char(c) => app.input_char(c),
            _ => {}
        },
        Mode::BranchDeleteConfirm => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_branch_delete(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_branch_delete(),
            _ => {}
        },
        Mode::BranchPushConfirm => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_branch_push(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_branch_push(),
            _ => {}
        },
        Mode::BranchMergeConfirm => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_branch_merge(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_branch_merge(),
            _ => {}
        },
        Mode::MergeAbortConfirm => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_abort_merge(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.mode = Mode::Detail,
            _ => {}
        },
        Mode::MergeContinueConfirm => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_continue_merge(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.mode = Mode::Detail,
            _ => {}
        },
        Mode::BranchRebaseConfirm => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_branch_rebase(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_branch_rebase(),
            _ => {}
        },
        Mode::BranchInteractiveRebaseConfirm => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_branch_interactive_rebase(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.cancel_branch_interactive_rebase()
            }
            _ => {}
        },
        Mode::DiscardChangesConfirm => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_discard_changes(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_discard_changes(),
            _ => {}
        },
        Mode::TagDeleteConfirm => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_tag_delete(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_tag_delete(),
            _ => {}
        },
        Mode::TagPushConfirm => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_tag_push(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_tag_push(),
            _ => {}
        },
        Mode::TagPushAllConfirm => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_tag_push_all(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_tag_push_all(),
            _ => {}
        },
        Mode::StashDeleteConfirm => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_stash_delete(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_stash_delete(),
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
        Mode::BranchCheckoutConfirm => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_branch_checkout(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_branch_checkout(),
            _ => {}
        },
        Mode::TagCheckoutConfirm => match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_tag_checkout(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.cancel_tag_checkout(),
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
        Mode::Detail => match code {
            KeyCode::Esc => {
                if app.inspect_full_diff {
                    app.inspect_full_diff = false;
                } else if app.commit_search_query.is_some() {
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
                app.resync_detail();
            }
            KeyCode::BackTab => {
                app.inspect_full_diff = false;
                app.detail_tab = if app.detail_tab == 0 {
                    7
                } else {
                    app.detail_tab - 1
                };
                app.set_default_focus_for_tab();
                app.resync_detail();
            }
            KeyCode::Char('1') => {
                app.inspect_full_diff = false;
                app.detail_tab = 0;
                app.detail_focus = DetailSection::Commits;
                app.resync_detail();
            }
            KeyCode::Char('2') => {
                app.inspect_full_diff = false;
                app.detail_tab = 1;
                app.detail_focus = DetailSection::Files;
                app.resync_detail();
            }
            KeyCode::Char('3') => {
                app.inspect_full_diff = false;
                app.detail_tab = 2;
                app.resync_detail();
            }
            KeyCode::Char('4') => {
                app.inspect_full_diff = false;
                app.detail_tab = 3;
                app.detail_focus = DetailSection::LocalBranches;
                app.resync_detail();
            }
            KeyCode::Char('5') => {
                app.inspect_full_diff = false;
                app.detail_tab = 4;
                app.detail_focus = DetailSection::LocalTags;
                app.fetch_remote_tags(false);
                app.resync_detail();
            }
            KeyCode::Char('6') => {
                app.inspect_full_diff = false;
                app.detail_tab = 5;
                app.detail_focus = DetailSection::Remotes;
                app.resync_detail();
            }
            KeyCode::Char('7') => {
                app.inspect_full_diff = false;
                app.detail_tab = 6;
                app.detail_focus = DetailSection::Stashes;
                app.resync_detail();
            }
            KeyCode::Char('8') => {
                app.inspect_full_diff = false;
                app.detail_tab = 7;
                app.detail_focus = DetailSection::Commits;
                app.resync_detail();
            }
            _ if app.detail_tab == 0 => match code {
                KeyCode::Char('f') if detail_focus == DetailSection::Commits => {
                    app.search_column_selection = 0;
                    app.mode = Mode::SearchColumnPicker;
                }
                KeyCode::Char('c') | KeyCode::Char('C')
                    if detail_focus != DetailSection::Conflicts
                        && detail_focus != DetailSection::ConflictDiff =>
                {
                    app.start_commit()
                }
                KeyCode::Char('t') | KeyCode::Char('T')
                    if detail_focus == DetailSection::Commits =>
                {
                    app.start_tag_create();
                }
                KeyCode::Char('i') | KeyCode::Char('I')
                    if detail_focus == DetailSection::Commits =>
                {
                    app.run_interactive_rebase();
                }
                KeyCode::Char('w') => app.cycle_detail_focus(false),
                KeyCode::Char('W') => app.cycle_detail_focus(true),
                KeyCode::Right | KeyCode::Enter if detail_focus == DetailSection::Commits => {
                    app.mode = Mode::Inspect;
                    if app.is_uncommitted_selected() {
                        app.detail_focus = DetailSection::Staged;
                        app.last_staging_focus = DetailSection::Staged;
                        app.staging_file_selection = 0;
                    } else {
                        app.detail_focus = DetailSection::Staged;
                        app.last_staging_focus = DetailSection::Staged;
                        app.file_selection = 0;
                    }
                    app.diff_scroll = 0;
                    app.refresh_file_diff();
                }
                KeyCode::Right
                    if detail_focus == DetailSection::Staged
                        || detail_focus == DetailSection::Unstaged
                        || detail_focus == DetailSection::CommitDetails
                        || detail_focus == DetailSection::StagingDetails =>
                {
                    app.mode = Mode::Inspect;
                    if detail_focus == DetailSection::Staged
                        || detail_focus == DetailSection::Unstaged
                    {
                        app.last_staging_focus = detail_focus;
                    }
                    app.detail_focus = DetailSection::StagingDetails;
                    app.diff_scroll = 0;
                    if app.is_uncommitted_selected() {
                        app.refresh_staging_diff();
                    } else {
                        app.refresh_file_diff();
                    }
                }
                KeyCode::Up if detail_focus == DetailSection::Commits => app.detail_commit_up(),
                KeyCode::Down if detail_focus == DetailSection::Commits => app.detail_commit_down(),
                KeyCode::PageUp if detail_focus == DetailSection::Commits => {
                    app.detail_commit_page_up(app.config.page_size)
                }
                KeyCode::PageDown if detail_focus == DetailSection::Commits => {
                    app.detail_commit_page_down(app.config.page_size)
                }
                KeyCode::Home if detail_focus == DetailSection::Commits => {
                    app.detail_commit_to_top()
                }
                KeyCode::End if detail_focus == DetailSection::Commits => {
                    app.detail_commit_to_bottom()
                }
                KeyCode::Up
                    if detail_focus == DetailSection::Staged
                        || detail_focus == DetailSection::Unstaged
                        || detail_focus == DetailSection::Conflicts =>
                {
                    if detail_focus == DetailSection::Conflicts {
                        app.conflict_file_up();
                    } else if app.is_uncommitted_selected() {
                        app.staging_file_up();
                    } else {
                        app.detail_file_up();
                    }
                }
                KeyCode::Down
                    if detail_focus == DetailSection::Staged
                        || detail_focus == DetailSection::Unstaged
                        || detail_focus == DetailSection::Conflicts =>
                {
                    if detail_focus == DetailSection::Conflicts {
                        app.conflict_file_down();
                    } else if app.is_uncommitted_selected() {
                        app.staging_file_down();
                    } else {
                        app.detail_file_down();
                    }
                }
                KeyCode::Enter
                    if detail_focus == DetailSection::Staged && app.is_uncommitted_selected() =>
                {
                    app.unstage_selected_file()
                }
                KeyCode::Enter
                    if detail_focus == DetailSection::Unstaged && app.is_uncommitted_selected() =>
                {
                    app.stage_selected_file()
                }
                KeyCode::Enter | KeyCode::Right if detail_focus == DetailSection::Conflicts => {
                    app.mode = Mode::Inspect;
                    app.last_staging_focus = DetailSection::Conflicts;
                    app.detail_focus = DetailSection::ConflictDiff;
                    app.diff_scroll = 0;
                    app.refresh_staging_diff();
                }
                KeyCode::Char('o')
                    if (detail_focus == DetailSection::Conflicts
                        || detail_focus == DetailSection::ConflictDiff)
                        && app.is_uncommitted_selected() =>
                {
                    app.resolve_conflict_ours()
                }
                KeyCode::Char('t')
                    if (detail_focus == DetailSection::Conflicts
                        || detail_focus == DetailSection::ConflictDiff)
                        && app.is_uncommitted_selected() =>
                {
                    app.resolve_conflict_theirs()
                }
                KeyCode::Char('r')
                    if (detail_focus == DetailSection::Conflicts
                        || detail_focus == DetailSection::ConflictDiff)
                        && app.is_uncommitted_selected() =>
                {
                    app.mark_conflict_resolved()
                }
                KeyCode::Char('A')
                    if (detail_focus == DetailSection::Conflicts
                        || detail_focus == DetailSection::ConflictDiff)
                        && app.is_uncommitted_selected() =>
                {
                    app.mode = Mode::MergeAbortConfirm;
                }
                KeyCode::Char('C')
                    if (detail_focus == DetailSection::Conflicts
                        || detail_focus == DetailSection::ConflictDiff)
                        && app.is_uncommitted_selected() =>
                {
                    app.mode = Mode::MergeContinueConfirm;
                }
                KeyCode::Enter
                    if detail_focus == DetailSection::StagingDetails
                        && app.is_uncommitted_selected() =>
                {
                    if app.last_staging_focus == DetailSection::Staged {
                        app.unstage_selected_hunk();
                    } else if app.last_staging_focus == DetailSection::Unstaged {
                        app.stage_selected_hunk();
                    }
                }
                KeyCode::Char('a') | KeyCode::Char('A')
                    if detail_focus == DetailSection::Unstaged && app.is_uncommitted_selected() =>
                {
                    app.stage_all_changes()
                }
                KeyCode::Char('a') | KeyCode::Char('A')
                    if detail_focus == DetailSection::Staged && app.is_uncommitted_selected() =>
                {
                    app.unstage_all_changes()
                }
                KeyCode::Char('x')
                    if (detail_focus == DetailSection::Staged
                        || detail_focus == DetailSection::Unstaged)
                        && app.is_uncommitted_selected() =>
                {
                    app.request_discard_changes()
                }
                KeyCode::Char('X')
                    if (detail_focus == DetailSection::Staged
                        || detail_focus == DetailSection::Unstaged
                        || detail_focus == DetailSection::StagingDetails)
                        && app.is_uncommitted_selected() =>
                {
                    app.request_discard_all_changes()
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    let can_stash = ((detail_focus == DetailSection::Staged
                        || detail_focus == DetailSection::Unstaged
                        || detail_focus == DetailSection::StagingDetails)
                        && app.is_uncommitted_selected())
                        || (detail_focus == DetailSection::Commits
                            && app.has_uncommitted_changes());
                    if can_stash {
                        app.start_stash_create();
                    }
                }
                KeyCode::Up
                    if detail_focus == DetailSection::StagingDetails
                        || detail_focus == DetailSection::ConflictDiff =>
                {
                    app.diff_scroll_up()
                }
                KeyCode::Down
                    if detail_focus == DetailSection::StagingDetails
                        || detail_focus == DetailSection::ConflictDiff =>
                {
                    app.diff_scroll_down()
                }
                KeyCode::PageUp
                    if detail_focus == DetailSection::StagingDetails
                        || detail_focus == DetailSection::ConflictDiff =>
                {
                    app.diff_scroll_page_up(app.config.page_size)
                }
                KeyCode::PageDown
                    if detail_focus == DetailSection::StagingDetails
                        || detail_focus == DetailSection::ConflictDiff =>
                {
                    app.diff_scroll_page_down(app.config.page_size)
                }
                KeyCode::Home
                    if detail_focus == DetailSection::StagingDetails
                        || detail_focus == DetailSection::ConflictDiff =>
                {
                    app.diff_scroll_to_top()
                }
                KeyCode::End
                    if detail_focus == DetailSection::StagingDetails
                        || detail_focus == DetailSection::ConflictDiff =>
                {
                    app.diff_scroll_to_bottom()
                }
                KeyCode::Up if detail_focus == DetailSection::CommitDetails => {
                    app.commit_details_scroll_up()
                }
                KeyCode::Down if detail_focus == DetailSection::CommitDetails => {
                    app.commit_details_scroll_down()
                }
                _ => {}
            },
            _ if app.detail_tab == 1 => match code {
                KeyCode::Char('w') => app.cycle_detail_focus(false),
                KeyCode::Char('W') => app.cycle_detail_focus(true),
                KeyCode::Char('f') if app.detail_focus == DetailSection::Files => {
                    app.pending_files_fzf = true;
                }
                KeyCode::Up => {
                    if app.detail_focus == DetailSection::FileContent {
                        app.file_content_scroll_up();
                    } else {
                        app.file_list_up();
                    }
                }
                KeyCode::Down => {
                    if app.detail_focus == DetailSection::FileContent {
                        app.file_content_scroll_down();
                    } else {
                        app.file_list_down();
                    }
                }
                KeyCode::PageUp => {
                    if app.detail_focus == DetailSection::FileContent {
                        app.file_content_scroll_page_up(app.config.page_size);
                    } else {
                        app.file_list_page_up(app.config.page_size);
                    }
                }
                KeyCode::PageDown => {
                    if app.detail_focus == DetailSection::FileContent {
                        app.file_content_scroll_page_down(app.config.page_size);
                    } else {
                        app.file_list_page_down(app.config.page_size);
                    }
                }
                KeyCode::Home => {
                    if app.detail_focus == DetailSection::FileContent {
                        app.file_content_scroll_to_top();
                    } else {
                        app.file_list_to_top();
                    }
                }
                KeyCode::End => {
                    if app.detail_focus == DetailSection::FileContent {
                        app.file_content_scroll_to_bottom();
                    } else {
                        app.file_list_to_bottom();
                    }
                }
                KeyCode::Char('>') | KeyCode::Char('.') | KeyCode::Right
                    if app.detail_focus == DetailSection::Files =>
                {
                    app.expand_selected_folder();
                }
                KeyCode::Char('<') | KeyCode::Char(',') | KeyCode::Left
                    if app.detail_focus == DetailSection::Files =>
                {
                    app.collapse_selected_folder();
                }
                KeyCode::Right if app.detail_focus == DetailSection::FileContent => {
                    app.inspect_full_diff = true;
                }
                KeyCode::Left
                    if app.detail_focus == DetailSection::FileContent && app.inspect_full_diff =>
                {
                    app.inspect_full_diff = false;
                }
                _ => {}
            },
            _ if app.detail_tab == 2 => match code {
                KeyCode::Up => app.graph_scroll_up(),
                KeyCode::Down => app.graph_scroll_down(),
                KeyCode::PageUp => app.graph_scroll_page_up(app.config.page_size),
                KeyCode::PageDown => app.graph_scroll_page_down(app.config.page_size),
                KeyCode::Home => app.graph_scroll_to_top(),
                KeyCode::End => app.graph_scroll_to_bottom(),
                _ => {}
            },
            _ if app.detail_tab == 3 => match code {
                KeyCode::Char('w') => app.cycle_detail_focus(false),
                KeyCode::Char('W') => app.cycle_detail_focus(true),
                KeyCode::Char('c') | KeyCode::Char('C') => app.start_branch_create(),
                KeyCode::Char('d') | KeyCode::Char('D') => app.request_branch_delete(),
                KeyCode::Char('m') | KeyCode::Char('M') => app.request_branch_merge(),
                KeyCode::Char('r') | KeyCode::Char('R') => app.request_branch_rebase(),
                KeyCode::Char('i') | KeyCode::Char('I') => app.request_branch_interactive_rebase(),
                KeyCode::Char('F') => {
                    if app.detail_focus == DetailSection::LocalBranches {
                        app.fetch_selected_branch();
                    }
                }
                KeyCode::Char('P') => {
                    if app.detail_focus == DetailSection::LocalBranches {
                        app.request_branch_push();
                    }
                }
                KeyCode::Char('p') => {
                    if app.detail_focus == DetailSection::LocalBranches {
                        app.pull_selected_branch();
                    }
                }
                KeyCode::Enter => {
                    app.request_branch_checkout();
                }
                KeyCode::Up => {
                    if app.detail_focus == DetailSection::LocalBranches {
                        app.local_branch_up();
                    } else {
                        app.remote_branch_up();
                    }
                }
                KeyCode::Down => {
                    if app.detail_focus == DetailSection::LocalBranches {
                        app.local_branch_down();
                    } else {
                        app.remote_branch_down();
                    }
                }
                KeyCode::PageUp => {
                    if app.detail_focus == DetailSection::LocalBranches {
                        app.local_branch_page_up(app.config.page_size);
                    } else {
                        app.remote_branch_page_up(app.config.page_size);
                    }
                }
                KeyCode::PageDown => {
                    if app.detail_focus == DetailSection::LocalBranches {
                        app.local_branch_page_down(app.config.page_size);
                    } else {
                        app.remote_branch_page_down(app.config.page_size);
                    }
                }
                KeyCode::Home => {
                    if app.detail_focus == DetailSection::LocalBranches {
                        app.local_branch_to_top();
                    } else {
                        app.remote_branch_to_top();
                    }
                }
                KeyCode::End => {
                    if app.detail_focus == DetailSection::LocalBranches {
                        app.local_branch_to_bottom();
                    } else {
                        app.remote_branch_to_bottom();
                    }
                }
                KeyCode::Left => app.move_focus_left(),
                KeyCode::Right => app.move_focus_right(),
                _ => {}
            },
            _ if app.detail_tab == 4 => match code {
                KeyCode::Enter => {
                    app.request_tag_checkout();
                }
                KeyCode::Up => {
                    app.local_tag_up();
                }
                KeyCode::Down => {
                    app.local_tag_down();
                }
                KeyCode::PageUp => {
                    app.local_tag_page_up(app.config.page_size);
                }
                KeyCode::PageDown => {
                    app.local_tag_page_down(app.config.page_size);
                }
                KeyCode::Home => {
                    app.local_tag_to_top();
                }
                KeyCode::End => {
                    app.local_tag_to_bottom();
                }
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    app.request_tag_delete();
                }
                KeyCode::Char('p') => {
                    app.request_tag_push();
                }
                KeyCode::Char('P') => {
                    app.request_tag_push_all();
                }
                _ => {}
            },
            _ if app.detail_tab == 5 => match code {
                KeyCode::Up => {
                    app.remote_up();
                }
                KeyCode::Down => {
                    app.remote_down();
                }
                KeyCode::PageUp => {
                    app.remote_page_up(app.config.page_size);
                }
                KeyCode::PageDown => {
                    app.remote_page_down(app.config.page_size);
                }
                KeyCode::Home => {
                    app.remote_to_top();
                }
                KeyCode::End => {
                    app.remote_to_bottom();
                }
                // f / F — fetch remote tags from the selected remote
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    let remote_action =
                        if let Some(crate::repo::ItemDetail::Repo { info, .. }) =
                            &app.current_detail
                        {
                            if info.remotes.len() > 1 {
                                Some(None) // Open picker
                            } else {
                                info.remotes.first().map(|r| Some(r.name.clone()))
                            }
                        } else {
                            None
                        };

                    match remote_action {
                        Some(Some(remote_name)) => {
                            app.fetch_remote(&remote_name);
                        }
                        Some(None) => {
                            app.remote_picker_action = Some(RemotePickerAction::FetchRemote);
                            app.remote_picker_selection = app.remote_selection;
                            app.mode = Mode::RemotePicker;
                        }
                        None => {}
                    }
                }
                _ => {}
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
                    DetailSection::StagingDetails => app.diff_scroll_up(),
                    _ => {}
                },
                KeyCode::Down => match detail_focus {
                    DetailSection::Stashes => app.stash_down(),
                    DetailSection::StashedFiles => app.stash_file_down(),
                    DetailSection::StagingDetails => app.diff_scroll_down(),
                    _ => {}
                },
                KeyCode::PageUp => match detail_focus {
                    DetailSection::Stashes => app.stash_page_up(app.config.page_size),
                    DetailSection::StashedFiles => app.stash_file_page_up(app.config.page_size),
                    DetailSection::StagingDetails => app.diff_scroll_page_up(app.config.page_size),
                    _ => {}
                },
                KeyCode::PageDown => match detail_focus {
                    DetailSection::Stashes => app.stash_page_down(app.config.page_size),
                    DetailSection::StashedFiles => app.stash_file_page_down(app.config.page_size),
                    DetailSection::StagingDetails => {
                        app.diff_scroll_page_down(app.config.page_size)
                    }
                    _ => {}
                },
                KeyCode::Home => match detail_focus {
                    DetailSection::Stashes => app.stash_to_top(),
                    DetailSection::StashedFiles => app.stash_file_to_top(),
                    DetailSection::StagingDetails => app.diff_scroll_to_top(),
                    _ => {}
                },
                KeyCode::End => match detail_focus {
                    DetailSection::Stashes => app.stash_to_bottom(),
                    DetailSection::StashedFiles => app.stash_file_to_bottom(),
                    DetailSection::StagingDetails => app.diff_scroll_to_bottom(),
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
                    app.diff_scroll = 0;
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
                    app.diff_scroll = 0;
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
                    app.commit_details_scroll_up();
                } else {
                    if app.is_uncommitted_selected() {
                        if app.diff_line_mode {
                            app.diff_line_up();
                        } else {
                            app.diff_hunk_up();
                        }
                    } else {
                        app.diff_scroll_up();
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
                    app.commit_details_scroll_down();
                } else {
                    if app.is_uncommitted_selected() {
                        if app.diff_line_mode {
                            app.diff_line_down();
                        } else {
                            app.diff_hunk_down();
                        }
                    } else {
                        app.diff_scroll_down();
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
                        app.commit_details_scroll_up();
                    }
                } else {
                    app.diff_scroll_page_up(app.config.page_size);
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
                        app.commit_details_scroll_down();
                    }
                } else {
                    app.diff_scroll_page_down(app.config.page_size);
                }
            }
            KeyCode::Home => {
                if app.detail_focus == DetailSection::Staged
                    || app.detail_focus == DetailSection::Unstaged
                    || app.detail_focus == DetailSection::Conflicts
                {
                    if app.detail_focus == DetailSection::Conflicts {
                        app.conflict_file_selection = 0;
                        app.refresh_staging_diff();
                    } else if app.is_uncommitted_selected() {
                        app.staging_file_selection = 0;
                        app.refresh_staging_diff();
                    } else {
                        app.file_selection = 0;
                        app.refresh_file_diff();
                    }
                } else if app.detail_focus == DetailSection::CommitDetails {
                    app.commit_details_scroll = 0;
                } else {
                    if app.is_uncommitted_selected() {
                        if app.diff_line_mode {
                            app.diff_line_selection = 0;
                            app.diff_scroll = 0;
                        } else {
                            app.diff_hunk_selection = 0;
                            app.scroll_to_selected_hunk();
                        }
                    } else {
                        app.diff_scroll_to_top();
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
                        app.conflict_file_selection = len.saturating_sub(1);
                        app.refresh_staging_diff();
                    } else if app.is_uncommitted_selected() {
                        app.staging_file_selection = app.staging_file_total().saturating_sub(1);
                        app.refresh_staging_diff();
                    } else {
                        app.file_selection = app.file_total().saturating_sub(1);
                        app.refresh_file_diff();
                    }
                } else if app.detail_focus == DetailSection::CommitDetails {
                    app.commit_details_scroll = usize::MAX;
                } else {
                    if app.is_uncommitted_selected() {
                        if app.diff_line_mode {
                            app.diff_line_selection = app.file_diff.len().saturating_sub(1);
                            app.diff_scroll = app.diff_line_selection.saturating_sub(17);
                        } else {
                            let hunk_count = app.get_diff_hunk_ranges().len();
                            app.diff_hunk_selection = hunk_count.saturating_sub(1);
                            app.scroll_to_selected_hunk();
                        }
                    } else {
                        app.diff_scroll_to_bottom();
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
                    app.diff_scroll = 0;
                    app.refresh_staging_diff();
                } else if app.detail_focus == DetailSection::StagingDetails {
                    if app.diff_line_mode {
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
                if app.diff_line_mode {
                    app.discard_selected_line();
                } else {
                    app.discard_selected_hunk();
                }
            }
            KeyCode::Char('x') | KeyCode::Char('X') if app.is_uncommitted_selected() => {
                if app.detail_focus == DetailSection::Staged
                    || app.detail_focus == DetailSection::Unstaged
                {
                    app.request_discard_changes();
                } else if app.detail_focus == DetailSection::StagingDetails
                    && app.last_staging_focus == DetailSection::Unstaged
                {
                    if app.diff_line_mode {
                        app.discard_selected_line();
                    } else {
                        app.discard_selected_hunk();
                    }
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
            KeyCode::Char('c') | KeyCode::Char('C') if app.is_uncommitted_selected() => {
                app.start_commit();
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
                app.input_buffer = app.commit_search_query.clone().unwrap_or_default();
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
                app.commit_search_query = None;
                app.mode = Mode::Logs;
            }
            KeyCode::Enter => {
                let query = app.input_buffer.clone();
                if query.trim().is_empty() {
                    app.commit_search_query = None;
                } else {
                    app.commit_search_query = Some(query);
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
                    app.staging_file_selection = 0;
                } else {
                    app.detail_focus = DetailSection::Staged;
                    app.last_staging_focus = DetailSection::Staged;
                    app.file_selection = 0;
                }
                app.diff_scroll = 0;
                app.refresh_file_diff();
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.in_logs_ui = false;
                app.commit_search_query = None;
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
        Mode::CommitInput => {
            if app.commit_editing {
                match code {
                    KeyCode::Esc => app.cancel_commit(),
                    KeyCode::Enter => app.input_char('\n'),
                    KeyCode::Char('c') | KeyCode::Char('C')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        app.commit_done_editing()
                    }
                    KeyCode::Backspace => app.input_backspace(),
                    KeyCode::Up => app.commit_input_scroll_up(),
                    KeyCode::Down => app.commit_input_scroll_down(),
                    KeyCode::Char(c) => app.input_char(c),
                    _ => {}
                }
            } else {
                match code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => app.cancel_commit(),
                    KeyCode::Enter => app.commit_git_changes(),
                    KeyCode::Char('e') | KeyCode::Char('E') => app.commit_start_editing(),
                    KeyCode::Char('a') | KeyCode::Char('A') | KeyCode::Char(' ') => {
                        app.toggle_commit_amend()
                    }
                    KeyCode::Up => app.commit_input_scroll_up(),
                    KeyCode::Down => app.commit_input_scroll_down(),
                    _ => {}
                }
            }
        }
    }
    true
}

/// Dispatch a mouse event.
pub fn handle_mouse(app: &mut App, mouse: MouseEvent) {
    let is_click = mouse.kind == MouseEventKind::Down(MouseButton::Left);
    let is_drag = mouse.kind == MouseEventKind::Drag(MouseButton::Left);
    let is_release = mouse.kind == MouseEventKind::Up(MouseButton::Left);
    let is_scroll_up = mouse.kind == MouseEventKind::ScrollUp;
    let is_scroll_down = mouse.kind == MouseEventKind::ScrollDown;

    if !is_click && !is_drag && !is_release && !is_scroll_up && !is_scroll_down {
        return;
    }

    let pos = Position {
        x: mouse.column,
        y: mouse.row,
    };

    let areas = app.detail_areas;

    // Handle splitter dragging
    if let Some(splitter) = app.active_drag_splitter {
        if is_release {
            app.active_drag_splitter = None;
        } else if is_drag {
            match splitter {
                Splitter::InspectHorizontal => {
                    if let (Some(left), Some(right)) = (areas.bottom_left, areas.bottom_right) {
                        let start_x = areas.commit_details.map(|r| r.x).unwrap_or(left.x);
                        let total_width = (right.x + right.width).saturating_sub(start_x);
                        if total_width > 0 {
                            let relative_x = pos.x.saturating_sub(start_x);
                            let pct = ((relative_x as f32 / total_width as f32) * 100.0) as u16;
                            app.inspect_horizontal_split_pct = pct.clamp(15, 85);
                        }
                    }
                }
                Splitter::InspectVertical => {
                    let mut start_y = None;
                    let mut total_height = None;

                    if let (Some(top), Some(bottom)) = (areas.staged_sub, areas.unstaged_sub) {
                        start_y = Some(top.y);
                        total_height = Some((bottom.y + bottom.height).saturating_sub(top.y));
                    } else if let (Some(top), Some(bottom)) =
                        (areas.commit_details, areas.bottom_left)
                    {
                        start_y = Some(top.y);
                        total_height = Some((bottom.y + bottom.height).saturating_sub(top.y));
                    }

                    if let (Some(sy), Some(th)) = (start_y, total_height) {
                        if th > 0 {
                            let relative_y = pos.y.saturating_sub(sy);
                            let pct = ((relative_y as f32 / th as f32) * 100.0) as u16;
                            app.inspect_vertical_split_pct = pct.clamp(15, 85);
                        }
                    }
                }
                Splitter::WorkspaceMain => {
                    if let (Some(top), Some(bottom)) = (areas.commits, areas.bottom_right) {
                        let start_y = top.y;
                        let total_height = (bottom.y + bottom.height).saturating_sub(start_y);
                        if total_height > 0 {
                            let relative_y = pos.y.saturating_sub(start_y);
                            let pct = ((relative_y as f32 / total_height as f32) * 100.0) as u16;
                            app.workspace_main_split_pct = pct.clamp(15, 85);
                        }
                    }
                }
                Splitter::FilesHorizontal => {
                    if let (Some(left), Some(right)) = (areas.files, areas.file_content) {
                        let start_x = left.x;
                        let total_width = (right.x + right.width).saturating_sub(start_x);
                        if total_width > 0 {
                            let relative_x = pos.x.saturating_sub(start_x);
                            let pct = ((relative_x as f32 / total_width as f32) * 100.0) as u16;
                            app.files_horizontal_split_pct = pct.clamp(15, 85);
                        }
                    }
                }
                Splitter::BranchesHorizontal => {
                    if let (Some(left), Some(right)) = (areas.local_branches, areas.remote_branches)
                    {
                        let start_x = left.x;
                        let total_width = (right.x + right.width).saturating_sub(start_x);
                        if total_width > 0 {
                            let relative_x = pos.x.saturating_sub(start_x);
                            let pct = ((relative_x as f32 / total_width as f32) * 100.0) as u16;
                            app.branches_horizontal_split_pct = pct.clamp(15, 85);
                        }
                    }
                }
                Splitter::StashesHorizontal => {
                    if let (Some(stashes), Some(right)) = (areas.stashes, areas.bottom_right) {
                        let start_x = stashes.x;
                        let total_width = (right.x + right.width).saturating_sub(start_x);
                        if total_width > 0 {
                            let relative_x = pos.x.saturating_sub(start_x);
                            let pct = ((relative_x as f32 / total_width as f32) * 100.0) as u16;
                            app.stashes_horizontal_split_pct = pct.clamp(15, 85);
                        }
                    }
                }
                Splitter::StashesVertical => {
                    if let (Some(top), Some(bottom)) = (areas.stashes, areas.stashed_files) {
                        let start_y = top.y;
                        let total_height = (bottom.y + bottom.height).saturating_sub(start_y);
                        if total_height > 0 {
                            let relative_y = pos.y.saturating_sub(start_y);
                            let pct = ((relative_y as f32 / total_height as f32) * 100.0) as u16;
                            app.stashes_vertical_split_pct = pct.clamp(15, 85);
                        }
                    }
                }
                Splitter::OverviewHorizontal => {
                    if let Some(tab_bar) = areas.tab_bar {
                        let start_x = tab_bar.x;
                        let total_width = tab_bar.width;
                        if total_width > 0 {
                            let relative_x = pos.x.saturating_sub(start_x);
                            let pct = ((relative_x as f32 / total_width as f32) * 100.0) as u16;
                            app.overview_horizontal_split_pct = pct.clamp(15, 85);
                        }
                    }
                }
            }
        }
        return;
    }

    if is_click {
        if let Some(rect) = areas.inspect_horizontal_splitter {
            if rect.contains(pos) {
                app.active_drag_splitter = Some(Splitter::InspectHorizontal);
                return;
            }
        }
        if let Some(rect) = areas.inspect_vertical_splitter {
            if rect.contains(pos) {
                app.active_drag_splitter = Some(Splitter::InspectVertical);
                return;
            }
        }
        if let Some(rect) = areas.workspace_main_splitter {
            if rect.contains(pos) {
                app.active_drag_splitter = Some(Splitter::WorkspaceMain);
                return;
            }
        }
        if let Some(rect) = areas.files_horizontal_splitter {
            if rect.contains(pos) {
                app.active_drag_splitter = Some(Splitter::FilesHorizontal);
                return;
            }
        }
        if let Some(rect) = areas.branches_horizontal_splitter {
            if rect.contains(pos) {
                app.active_drag_splitter = Some(Splitter::BranchesHorizontal);
                return;
            }
        }
        if let Some(rect) = areas.stashes_horizontal_splitter {
            if rect.contains(pos) {
                app.active_drag_splitter = Some(Splitter::StashesHorizontal);
                return;
            }
        }
        if let Some(rect) = areas.stashes_vertical_splitter {
            if rect.contains(pos) {
                app.active_drag_splitter = Some(Splitter::StashesVertical);
                return;
            }
        }
        if let Some(rect) = areas.overview_horizontal_splitter {
            if rect.contains(pos) {
                app.active_drag_splitter = Some(Splitter::OverviewHorizontal);
                return;
            }
        }
    }

    if is_drag || is_release {
        return;
    }

    if app.mode == Mode::Settings {
        return;
    }

    if app.mode == Mode::Help || app.mode == Mode::DetailHelp {
        if is_scroll_up {
            app.help_scroll_up();
        } else if is_scroll_down {
            app.help_scroll_down();
        }
        return;
    }

    if app.mode == Mode::CommitInput {
        if is_scroll_up {
            app.commit_input_scroll_up();
        } else if is_scroll_down {
            app.commit_input_scroll_down();
        }
        return;
    }

    if app.mode == Mode::Normal {
        if is_click {
            for (i, rect) in app.main_areas.iter().enumerate() {
                if rect.contains(pos) {
                    let actual_index = i + app.scroll_top;
                    if actual_index < app.get_items_len() {
                        let now = std::time::Instant::now();
                        let is_double_click = if let Some((last_time, last_idx)) = app.last_click {
                            last_idx == actual_index
                                && now.duration_since(last_time).as_millis() < 400
                        } else {
                            false
                        };

                        if is_double_click {
                            app.selected_index = actual_index;
                            app.open_detail();
                            app.last_click = None;
                        } else {
                            app.selected_index = actual_index;
                            app.last_click = Some((now, actual_index));
                        }
                    }
                    return;
                }
            }
        } else {
            let visible_count = app.main_areas.len();
            if is_scroll_up {
                app.move_up();
            } else if is_scroll_down {
                app.move_down(visible_count);
            }
        }
        return;
    }

    // Only handle detail modes beyond this point.
    if !matches!(
        app.mode,
        Mode::Detail | Mode::DetailHelp | Mode::Inspect | Mode::Logs
    ) {
        return;
    }

    let areas = app.detail_areas;

    // Handle tab switching if the user clicks on the tab bar.
    if app.mode == Mode::Detail {
        if let Some(rect) = areas.tab_bar {
            if rect.contains(pos) {
                if is_click {
                    let click_x = pos.x - rect.x;
                    let use_short = rect.width < 124;
                    let tabs_data = [
                        ("Details", "D", 0),
                        ("Files", "F", 1),
                        ("Graph", "G", 2),
                        ("Branches", "B", 3),
                        ("Tags", "T", 4),
                        ("Remotes", "R", 5),
                        ("Stashes", "S", 6),
                        ("Overview", "O", 7),
                    ];
                    let mut current_offset = 2;
                    for &(long_name, short_name, tab_index) in &tabs_data {
                        let name = if use_short { short_name } else { long_name };
                        let tab_width = name.len() + 8;
                        if click_x >= current_offset && click_x < current_offset + tab_width as u16
                        {
                            app.detail_tab = tab_index;
                            match tab_index {
                                0 => app.detail_focus = DetailSection::Commits,
                                1 => app.detail_focus = DetailSection::Files,
                                2 => {}
                                3 => app.detail_focus = DetailSection::LocalBranches,
                                4 => {
                                    app.detail_focus = DetailSection::LocalTags;
                                    app.fetch_remote_tags(false);
                                }
                                5 => app.detail_focus = DetailSection::Remotes,
                                6 => app.detail_focus = DetailSection::Stashes,
                                7 => app.detail_focus = DetailSection::Commits,
                                _ => {}
                            }
                            app.resync_detail();
                            break;
                        }
                        current_offset += tab_width as u16 + 1;
                    }
                }
                return;
            }
        }
    }

    // Graph view scroll (tab 3, index 2)
    if app.detail_tab == 2 {
        if let Some(rect) = areas.tab_bar {
            if pos.y >= rect.y + rect.height {
                if is_scroll_up {
                    app.graph_scroll_up();
                } else if is_scroll_down {
                    app.graph_scroll_down();
                }
                return;
            }
        }
    }

    // Staged sub-panel (inside Staging Area left block) — check before bottom_left
    // so the more-specific sub-panels win.
    if let Some(rect) = areas.staged_sub {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::Staged;
                app.last_staging_focus = DetailSection::Staged;

                let mut clicked_file = false;
                if let Some(inner) = areas.staged_sub_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = if app.detail_focus == DetailSection::Staged {
                            app.staged_list_state.borrow().offset()
                        } else {
                            0
                        };
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                                info.changes.staged.len()
                            }
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.staging_file_selection = actual_idx;
                            clicked_file = true;
                        }
                    }
                }

                if !clicked_file {
                    let total = match &app.current_detail {
                        Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                            info.changes.staged.len()
                        }
                        _ => 0,
                    };
                    if total > 0 {
                        app.staging_file_selection = app.staging_file_selection.min(total - 1);
                    } else {
                        app.staging_file_selection = 0;
                    }
                }
                app.diff_scroll = 0;
                app.refresh_staging_diff();
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Staged;
                app.last_staging_focus = DetailSection::Staged;
                if app.is_uncommitted_selected() {
                    app.staging_file_up();
                } else {
                    app.detail_file_up();
                }
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Staged;
                app.last_staging_focus = DetailSection::Staged;
                if app.is_uncommitted_selected() {
                    app.staging_file_down();
                } else {
                    app.detail_file_down();
                }
            }
            return;
        }
    }
    // Unstaged sub-panel.
    if let Some(rect) = areas.unstaged_sub {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::Unstaged;
                app.last_staging_focus = DetailSection::Unstaged;

                let mut clicked_file = false;
                if let Some(inner) = areas.unstaged_sub_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = if app.detail_focus == DetailSection::Unstaged {
                            app.unstaged_list_state.borrow().offset()
                        } else {
                            0
                        };
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                                info.changes.unstaged.len()
                            }
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.staging_file_selection = actual_idx;
                            clicked_file = true;
                        }
                    }
                }

                if !clicked_file {
                    let total = match &app.current_detail {
                        Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                            info.changes.unstaged.len()
                        }
                        _ => 0,
                    };
                    if total > 0 {
                        app.staging_file_selection = app.staging_file_selection.min(total - 1);
                    } else {
                        app.staging_file_selection = 0;
                    }
                }
                app.diff_scroll = 0;
                app.refresh_staging_diff();
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Unstaged;
                app.last_staging_focus = DetailSection::Unstaged;
                if app.is_uncommitted_selected() {
                    app.staging_file_up();
                } else {
                    app.detail_file_up();
                }
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Unstaged;
                app.last_staging_focus = DetailSection::Unstaged;
                if app.is_uncommitted_selected() {
                    app.staging_file_down();
                } else {
                    app.detail_file_down();
                }
            }
            return;
        }
    }
    // Conflicts sub-panel.
    if let Some(rect) = areas.conflicts_sub {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::Conflicts;
                app.last_staging_focus = DetailSection::Conflicts;

                let mut clicked_file = false;
                if let Some(inner) = areas.conflicts_sub_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = if app.detail_focus == DetailSection::Conflicts {
                            app.conflicts_list_state.borrow().offset()
                        } else {
                            0
                        };
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                                info.changes.conflicted.len()
                            }
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.conflict_file_selection = actual_idx;
                            clicked_file = true;
                        }
                    }
                }

                if !clicked_file {
                    let total = match &app.current_detail {
                        Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                            info.changes.conflicted.len()
                        }
                        _ => 0,
                    };
                    if total > 0 {
                        app.conflict_file_selection = app.conflict_file_selection.min(total - 1);
                    } else {
                        app.conflict_file_selection = 0;
                    }
                }
                app.diff_scroll = 0;
                app.refresh_staging_diff();
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Conflicts;
                app.last_staging_focus = DetailSection::Conflicts;
                let total = match &app.current_detail {
                    Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                        info.changes.conflicted.len()
                    }
                    _ => 0,
                };
                if total > 0 {
                    app.conflict_file_selection = app.conflict_file_selection.saturating_sub(1);
                    app.refresh_staging_diff();
                }
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Conflicts;
                app.last_staging_focus = DetailSection::Conflicts;
                let total = match &app.current_detail {
                    Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                        info.changes.conflicted.len()
                    }
                    _ => 0,
                };
                if total > 0 {
                    app.conflict_file_selection = (app.conflict_file_selection + 1).min(total - 1);
                    app.refresh_staging_diff();
                }
            }
            return;
        }
    }
    // Commit details sub-panel (inside Changed Files / Commit Details left block).
    if let Some(rect) = areas.commit_details {
        if rect.contains(pos) {
            if is_click {
                if app.detail_focus != DetailSection::CommitDetails {
                    app.detail_focus = DetailSection::CommitDetails;
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::CommitDetails;
                app.commit_details_scroll_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::CommitDetails;
                app.commit_details_scroll_down();
            }
            return;
        }
    }

    // Bottom-left panel (Staging Area outer block or Changed Files).
    if let Some(rect) = areas.bottom_left {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::Staged;
                if app.is_uncommitted_selected() {
                    let total = app.staging_file_total();
                    if total > 0 {
                        app.staging_file_selection = app.staging_file_selection.min(total - 1);
                    } else {
                        app.staging_file_selection = 0;
                    }
                    app.diff_scroll = 0;
                    app.refresh_staging_diff();
                } else {
                    let mut clicked_file = false;
                    if let Some(inner) = areas.changed_files_inner {
                        if inner.contains(pos) {
                            let clicked_row = (pos.y - inner.y) as usize;
                            let offset = app.changed_files_list_state.borrow().offset();
                            let actual_idx = offset + clicked_row;
                            let total = app.file_total();
                            if actual_idx < total {
                                app.file_selection = actual_idx;
                                clicked_file = true;
                            }
                        }
                    }
                    if !clicked_file {
                        let total = app.file_total();
                        if total > 0 {
                            app.file_selection = app.file_selection.min(total - 1);
                        } else {
                            app.file_selection = 0;
                        }
                    }
                    app.diff_scroll = 0;
                    app.refresh_file_diff();
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Staged;
                if app.is_uncommitted_selected() {
                    app.staging_file_up();
                } else {
                    app.detail_file_up();
                }
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Staged;
                if app.is_uncommitted_selected() {
                    app.staging_file_down();
                } else {
                    app.detail_file_down();
                }
            }
            return;
        }
    }
    // Right panel (Diff / Staging Details).
    if let Some(rect) = areas.bottom_right {
        if rect.contains(pos) {
            if is_click {
                if app.detail_focus != DetailSection::StagingDetails {
                    if app.detail_focus == DetailSection::Staged
                        || app.detail_focus == DetailSection::Unstaged
                    {
                        app.last_staging_focus = app.detail_focus;
                    }
                    app.detail_focus = DetailSection::StagingDetails;
                    app.diff_scroll = 0;
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::StagingDetails;
                app.diff_scroll_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::StagingDetails;
                app.diff_scroll_down();
            }
            return;
        }
    }
    // Commits panel (top).
    if let Some(rect) = areas.commits {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::Commits;
                if let Some(inner) = areas.commits_inner {
                    if pos.y > inner.y {
                        let row_clicked = (pos.y - inner.y - 1) as usize;
                        let offset = app.commits_table_state.borrow().offset();
                        let actual_idx = offset + row_clicked;
                        let total = app.commit_total();
                        if actual_idx < total {
                            app.commit_selection = actual_idx;
                            app.file_selection = 0;
                            app.staging_file_selection = 0;
                            app.diff_scroll = 0;
                            app.refresh_file_diff();
                        }
                    }
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Commits;
                app.detail_commit_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Commits;
                app.detail_commit_down();
            }
        }
    }
    // Local branches panel (inside Branches view).
    if let Some(rect) = areas.local_branches {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::LocalBranches;
                if let Some(inner) = areas.local_branches_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = app.local_branch_list_state.borrow().offset();
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                                info.local_branches.len()
                            }
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.local_branch_selection = actual_idx;
                        }
                    }
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::LocalBranches;
                app.local_branch_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::LocalBranches;
                app.local_branch_down();
            }
        }
    }
    // Remote branches panel (inside Branches view).
    if let Some(rect) = areas.remote_branches {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::RemoteBranches;
                if let Some(inner) = areas.remote_branches_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = app.remote_branch_list_state.borrow().offset();
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                                info.remote_branches.len()
                            }
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.remote_branch_selection = actual_idx;
                        }
                    }
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::RemoteBranches;
                app.remote_branch_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::RemoteBranches;
                app.remote_branch_down();
            }
        }
    }
    // Local tags panel (inside Tags view).
    if let Some(rect) = areas.local_tags {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::LocalTags;
                if let Some(inner) = areas.local_tags_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = app.local_tag_list_state.borrow().offset();
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => {
                                info.local_tags.len()
                            }
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.local_tag_selection = actual_idx;
                        }
                    }
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::LocalTags;
                app.local_tag_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::LocalTags;
                app.local_tag_down();
            }
        }
    }
    // Files list panel (inside Files view).
    if let Some(rect) = areas.files {
        if rect.contains(pos) {
            if is_click {
                if app.detail_focus != DetailSection::Files {
                    app.detail_focus = DetailSection::Files;
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Files;
                app.file_list_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Files;
                app.file_list_down();
            }
        }
    }
    // File content preview panel (inside Files view).
    if let Some(rect) = areas.file_content {
        if rect.contains(pos) {
            if is_click {
                if app.detail_focus != DetailSection::FileContent {
                    app.detail_focus = DetailSection::FileContent;
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::FileContent;
                app.file_content_scroll_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::FileContent;
                app.file_content_scroll_down();
            }
        }
    }
    // Remotes list panel.
    if let Some(rect) = areas.remotes {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::Remotes;
                if let Some(inner) = areas.remotes_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = app.remote_list_state.borrow().offset();
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => info.remotes.len(),
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.remote_selection = actual_idx;
                        }
                    }
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Remotes;
                app.remote_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Remotes;
                app.remote_down();
            }
        }
    }
    // Stashes list panel.
    if let Some(rect) = areas.stashes {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::Stashes;
                if let Some(inner) = areas.stashes_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = app.stash_list_state.borrow().offset();
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => info.stashes.len(),
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.stash_selection = actual_idx;
                            app.stash_file_selection = 0;
                            app.diff_scroll = 0;
                            app.refresh_file_diff();
                        }
                    }
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Stashes;
                app.stash_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Stashes;
                app.stash_down();
            }
        }
    }
    // Stashed files panel.
    if let Some(rect) = areas.stashed_files {
        if rect.contains(pos) {
            if is_click {
                app.detail_focus = DetailSection::StashedFiles;
                if let Some(inner) = areas.stashed_files_inner {
                    if inner.contains(pos) {
                        let clicked_row = (pos.y - inner.y) as usize;
                        let offset = app.stash_file_list_state.borrow().offset();
                        let actual_idx = offset + clicked_row;
                        let total = match &app.current_detail {
                            Some(crate::repo::ItemDetail::Repo { info, .. }) => info
                                .stashes
                                .get(app.stash_selection)
                                .map(|s| s.files.len())
                                .unwrap_or(0),
                            _ => 0,
                        };
                        if actual_idx < total {
                            app.stash_file_selection = actual_idx;
                            app.diff_scroll = 0;
                            app.refresh_file_diff();
                        }
                    }
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::StashedFiles;
                app.stash_file_up();
            } else if is_scroll_down {
                app.detail_focus = DetailSection::StashedFiles;
                app.stash_file_down();
            }
        }
    }
}
