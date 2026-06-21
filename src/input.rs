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
        Mode::Adding | Mode::Editing | Mode::BranchCreateInput | Mode::TagCreateInput
    ) || (matches!(app.mode, Mode::CommitInput) && app.commit_editing);
    if !is_text_input && code == KeyCode::Char('.') {
        app.toggle_status_expanded();
        return true;
    }

    let detail_focus = app.detail_focus; // Copy before borrow in match
    match &app.mode {
        Mode::Normal => match code {
            KeyCode::Char('q') | KeyCode::Esc => return false,
            KeyCode::Down | KeyCode::Char('j') => app.move_down(visible_count),
            KeyCode::Up | KeyCode::Char('k') => app.move_up(),
            KeyCode::PageDown => app.page_down(visible_count),
            KeyCode::PageUp => app.page_up(visible_count),
            KeyCode::Char('a') => app.start_add(),
            KeyCode::Char('e') => app.start_edit(),
            KeyCode::Char('d') => app.request_delete(),
            KeyCode::Char('?') => app.open_help(),
            KeyCode::Char('r') => app.refresh_selected_status(),
            KeyCode::Char('o') => app.cycle_sort_order(),
            KeyCode::Char('O') => app.toggle_sort_reverse(),
            KeyCode::Char('p') => app.toggle_pin_selected(),
            KeyCode::Char('g') => {
                app.pending_gitui = true;
            }
            KeyCode::Enter => app.open_detail(),
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
        Mode::Help => match code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.close_dialog();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.help_scroll_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.help_scroll_down();
            }
            KeyCode::PageUp => {
                app.help_scroll_page_up(10);
            }
            KeyCode::PageDown => {
                app.help_scroll_page_down(10);
            }
            _ => {}
        },
        Mode::Detail => match code {
            KeyCode::Esc => {
                if app.commit_search_query.is_some() {
                    app.cancel_commit_search();
                } else {
                    app.close_detail();
                }
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => app.close_detail(),
            KeyCode::Char('?') => app.open_detail_help(),
            KeyCode::Tab => {
                app.detail_tab = (app.detail_tab + 1) % 8;
                app.set_default_focus_for_tab();
            }
            KeyCode::BackTab => {
                app.detail_tab = if app.detail_tab == 0 {
                    7
                } else {
                    app.detail_tab - 1
                };
                app.set_default_focus_for_tab();
            }
            KeyCode::Char('1') => {
                app.detail_tab = 0;
                app.detail_focus = DetailSection::Commits;
            }
            KeyCode::Char('2') => {
                app.detail_tab = 1;
                app.detail_focus = DetailSection::Files;
            }
            KeyCode::Char('3') => {
                app.detail_tab = 2;
            }
            KeyCode::Char('4') => {
                app.detail_tab = 3;
                app.detail_focus = DetailSection::LocalBranches;
            }
            KeyCode::Char('5') => {
                app.detail_tab = 4;
                app.detail_focus = DetailSection::LocalTags;
                app.fetch_remote_tags(false);
            }
            KeyCode::Char('6') => {
                app.detail_tab = 5;
                app.detail_focus = DetailSection::Remotes;
            }
            KeyCode::Char('7') => {
                app.detail_tab = 6;
                app.detail_focus = DetailSection::Stashes;
            }
            KeyCode::Char('8') => {
                app.detail_tab = 7;
                app.detail_focus = DetailSection::Commits;
            }
            _ if app.detail_tab == 0 => match code {
                KeyCode::Char('/') if detail_focus == DetailSection::Commits => {
                    app.start_commit_search();
                }
                KeyCode::Char('c') | KeyCode::Char('C') => app.start_commit(),
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    if detail_focus == DetailSection::Commits {
                        app.start_tag_create();
                    }
                }
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    if detail_focus == DetailSection::Commits {
                        app.run_interactive_rebase();
                    }
                }
                KeyCode::Char('w') | KeyCode::Char('W') => app.cycle_detail_focus(),
                KeyCode::Right if detail_focus == DetailSection::Commits => {
                    app.mode = Mode::Inspect;
                    if app.is_uncommitted_selected() {
                        app.detail_focus = DetailSection::Staged;
                        app.staging_file_selection = 0;
                    } else {
                        app.detail_focus = DetailSection::Staged;
                        app.file_selection = 0;
                    }
                    app.diff_scroll = 0;
                    app.refresh_file_diff();
                }
                KeyCode::Up | KeyCode::Char('k') if detail_focus == DetailSection::Commits => {
                    app.detail_commit_up()
                }
                KeyCode::Down | KeyCode::Char('j') if detail_focus == DetailSection::Commits => {
                    app.detail_commit_down()
                }
                KeyCode::PageUp if detail_focus == DetailSection::Commits => {
                    app.detail_commit_page_up(10)
                }
                KeyCode::PageDown if detail_focus == DetailSection::Commits => {
                    app.detail_commit_page_down(10)
                }
                KeyCode::Up | KeyCode::Char('k')
                    if detail_focus == DetailSection::Staged
                        || detail_focus == DetailSection::Unstaged =>
                {
                    if app.is_uncommitted_selected() {
                        app.staging_file_up()
                    } else {
                        app.detail_file_up()
                    }
                }
                KeyCode::Down | KeyCode::Char('j')
                    if detail_focus == DetailSection::Staged
                        || detail_focus == DetailSection::Unstaged =>
                {
                    if app.is_uncommitted_selected() {
                        app.staging_file_down()
                    } else {
                        app.detail_file_down()
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
                KeyCode::Char('x') | KeyCode::Char('X')
                    if (detail_focus == DetailSection::Staged
                        || detail_focus == DetailSection::Unstaged)
                        && app.is_uncommitted_selected() =>
                {
                    app.request_discard_changes()
                }
                KeyCode::Up | KeyCode::Char('k')
                    if detail_focus == DetailSection::StagingDetails =>
                {
                    app.diff_scroll_up()
                }
                KeyCode::Down | KeyCode::Char('j')
                    if detail_focus == DetailSection::StagingDetails =>
                {
                    app.diff_scroll_down()
                }
                KeyCode::PageUp if detail_focus == DetailSection::StagingDetails => {
                    app.diff_scroll_page_up(10)
                }
                KeyCode::PageDown if detail_focus == DetailSection::StagingDetails => {
                    app.diff_scroll_page_down(10)
                }
                KeyCode::Up | KeyCode::Char('k')
                    if detail_focus == DetailSection::CommitDetails =>
                {
                    app.commit_details_scroll_up()
                }
                KeyCode::Down | KeyCode::Char('j')
                    if detail_focus == DetailSection::CommitDetails =>
                {
                    app.commit_details_scroll_down()
                }
                _ => {}
            },
            _ if app.detail_tab == 1 => match code {
                KeyCode::Up | KeyCode::Char('k') => {
                    app.file_list_up();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.file_list_down();
                }
                KeyCode::PageUp => {
                    app.file_list_page_up(10);
                }
                KeyCode::PageDown => {
                    app.file_list_page_down(10);
                }
                KeyCode::Char('>') | KeyCode::Char('.') | KeyCode::Right => {
                    app.expand_selected_folder();
                }
                KeyCode::Char('<') | KeyCode::Char(',') | KeyCode::Left => {
                    app.collapse_selected_folder();
                }
                _ => {}
            },
            _ if app.detail_tab == 2 => match code {
                KeyCode::Up | KeyCode::Char('k') => app.graph_scroll_up(),
                KeyCode::Down | KeyCode::Char('j') => app.graph_scroll_down(),
                KeyCode::PageUp => app.graph_scroll_page_up(10),
                KeyCode::PageDown => app.graph_scroll_page_down(10),
                _ => {}
            },
            _ if app.detail_tab == 3 => match code {
                KeyCode::Char('w') | KeyCode::Char('W') => app.cycle_detail_focus(),
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
                    if app.detail_focus == DetailSection::LocalBranches {
                        app.checkout_selected_local_branch();
                    } else {
                        app.checkout_selected_remote_branch();
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if app.detail_focus == DetailSection::LocalBranches {
                        app.local_branch_up();
                    } else {
                        app.remote_branch_up();
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if app.detail_focus == DetailSection::LocalBranches {
                        app.local_branch_down();
                    } else {
                        app.remote_branch_down();
                    }
                }
                KeyCode::PageUp => {
                    if app.detail_focus == DetailSection::LocalBranches {
                        app.local_branch_page_up(10);
                    } else {
                        app.remote_branch_page_up(10);
                    }
                }
                KeyCode::PageDown => {
                    if app.detail_focus == DetailSection::LocalBranches {
                        app.local_branch_page_down(10);
                    } else {
                        app.remote_branch_page_down(10);
                    }
                }
                KeyCode::Left => app.move_focus_left(),
                KeyCode::Right => app.move_focus_right(),
                _ => {}
            },
            _ if app.detail_tab == 4 => match code {
                KeyCode::Enter => {
                    app.checkout_selected_local_tag();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    app.local_tag_up();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.local_tag_down();
                }
                KeyCode::PageUp => {
                    app.local_tag_page_up(10);
                }
                KeyCode::PageDown => {
                    app.local_tag_page_down(10);
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
                KeyCode::Up | KeyCode::Char('k') => {
                    app.remote_up();
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.remote_down();
                }
                KeyCode::PageUp => {
                    app.remote_page_up(10);
                }
                KeyCode::PageDown => {
                    app.remote_page_down(10);
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
                KeyCode::Char('w') | KeyCode::Char('W') => app.cycle_detail_focus(),
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
                KeyCode::Up | KeyCode::Char('k') => match detail_focus {
                    DetailSection::Stashes => app.stash_up(),
                    DetailSection::StashedFiles => app.stash_file_up(),
                    DetailSection::StagingDetails => app.diff_scroll_up(),
                    _ => {}
                },
                KeyCode::Down | KeyCode::Char('j') => match detail_focus {
                    DetailSection::Stashes => app.stash_down(),
                    DetailSection::StashedFiles => app.stash_file_down(),
                    DetailSection::StagingDetails => app.diff_scroll_down(),
                    _ => {}
                },
                KeyCode::PageUp => match detail_focus {
                    DetailSection::Stashes => app.stash_page_up(10),
                    DetailSection::StashedFiles => app.stash_file_page_up(10),
                    DetailSection::StagingDetails => app.diff_scroll_page_up(10),
                    _ => {}
                },
                KeyCode::PageDown => match detail_focus {
                    DetailSection::Stashes => app.stash_page_down(10),
                    DetailSection::StashedFiles => app.stash_file_page_down(10),
                    DetailSection::StagingDetails => app.diff_scroll_page_down(10),
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        },

        Mode::Inspect => match code {
            KeyCode::Esc => {
                app.mode = Mode::Detail;
                app.detail_focus = DetailSection::Commits;
            }
            KeyCode::Char('?') => {
                app.open_detail_help();
            }
            KeyCode::Char('w') | KeyCode::Char('W') | KeyCode::Tab => {
                if app.is_uncommitted_selected() {
                    app.detail_focus = match app.detail_focus {
                        DetailSection::Staged => DetailSection::Unstaged,
                        DetailSection::Unstaged => DetailSection::StagingDetails,
                        _ => DetailSection::Staged,
                    };
                    app.diff_scroll = 0;
                    app.refresh_staging_diff();
                } else {
                    app.detail_focus = match app.detail_focus {
                        DetailSection::Staged => DetailSection::CommitDetails,
                        DetailSection::CommitDetails => DetailSection::StagingDetails,
                        _ => DetailSection::Staged,
                    };
                }
            }
            KeyCode::Right => {
                if app.is_uncommitted_selected() {
                    if app.detail_focus == DetailSection::Staged
                        || app.detail_focus == DetailSection::Unstaged
                    {
                        app.detail_focus = DetailSection::StagingDetails;
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
                if app.detail_focus == DetailSection::StagingDetails {
                    app.detail_focus = DetailSection::Staged;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if app.detail_focus == DetailSection::Staged
                    || app.detail_focus == DetailSection::Unstaged
                {
                    if app.is_uncommitted_selected() {
                        app.staging_file_up();
                    } else {
                        app.detail_file_up();
                    }
                } else if app.detail_focus == DetailSection::CommitDetails {
                    app.commit_details_scroll_up();
                } else {
                    app.diff_scroll_up();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.detail_focus == DetailSection::Staged
                    || app.detail_focus == DetailSection::Unstaged
                {
                    if app.is_uncommitted_selected() {
                        app.staging_file_down();
                    } else {
                        app.detail_file_down();
                    }
                } else if app.detail_focus == DetailSection::CommitDetails {
                    app.commit_details_scroll_down();
                } else {
                    app.diff_scroll_down();
                }
            }
            KeyCode::PageUp => {
                if app.detail_focus == DetailSection::Staged
                    || app.detail_focus == DetailSection::Unstaged
                {
                    if app.is_uncommitted_selected() {
                        for _ in 0..10 {
                            app.staging_file_up();
                        }
                    } else {
                        for _ in 0..10 {
                            app.detail_file_up();
                        }
                    }
                } else if app.detail_focus == DetailSection::CommitDetails {
                    for _ in 0..10 {
                        app.commit_details_scroll_up();
                    }
                } else {
                    app.diff_scroll_page_up(10);
                }
            }
            KeyCode::PageDown => {
                if app.detail_focus == DetailSection::Staged
                    || app.detail_focus == DetailSection::Unstaged
                {
                    if app.is_uncommitted_selected() {
                        for _ in 0..10 {
                            app.staging_file_down();
                        }
                    } else {
                        for _ in 0..10 {
                            app.detail_file_down();
                        }
                    }
                } else if app.detail_focus == DetailSection::CommitDetails {
                    for _ in 0..10 {
                        app.commit_details_scroll_down();
                    }
                } else {
                    app.diff_scroll_page_down(10);
                }
            }
            KeyCode::Enter if app.is_uncommitted_selected() => {
                if app.detail_focus == DetailSection::Staged {
                    app.unstage_selected_file();
                } else if app.detail_focus == DetailSection::Unstaged {
                    app.stage_selected_file();
                }
            }
            KeyCode::Char('x') | KeyCode::Char('X')
                if app.is_uncommitted_selected()
                    && (app.detail_focus == DetailSection::Staged
                        || app.detail_focus == DetailSection::Unstaged) =>
            {
                app.request_discard_changes();
            }
            _ => {}
        },

        Mode::DetailHelp => match code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.close_detail_help();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.help_scroll_up();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.help_scroll_down();
            }
            KeyCode::PageUp => {
                app.help_scroll_page_up(10);
            }
            KeyCode::PageDown => {
                app.help_scroll_page_down(10);
            }
            _ => {}
        },
        Mode::RemotePicker => match code {
            KeyCode::Up | KeyCode::Char('k') => app.remote_picker_up(),
            KeyCode::Down | KeyCode::Char('j') => app.remote_picker_down(),
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
    }

    if is_drag || is_release {
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
                    if actual_index < app.config.items.len() {
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
    if !matches!(app.mode, Mode::Detail | Mode::DetailHelp | Mode::Inspect) {
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
                if app.detail_focus != DetailSection::Staged {
                    app.detail_focus = DetailSection::Staged;
                    app.staging_file_selection = 0;
                    app.diff_scroll = 0;
                    app.refresh_staging_diff();
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
    // Unstaged sub-panel.
    if let Some(rect) = areas.unstaged_sub {
        if rect.contains(pos) {
            if is_click {
                if app.detail_focus != DetailSection::Unstaged {
                    app.detail_focus = DetailSection::Unstaged;
                    app.staging_file_selection = 0;
                    app.diff_scroll = 0;
                    app.refresh_staging_diff();
                }
            } else if is_scroll_up {
                app.detail_focus = DetailSection::Unstaged;
                if app.is_uncommitted_selected() {
                    app.staging_file_up();
                } else {
                    app.detail_file_up();
                }
            } else if is_scroll_down {
                app.detail_focus = DetailSection::Unstaged;
                if app.is_uncommitted_selected() {
                    app.staging_file_down();
                } else {
                    app.detail_file_down();
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
                // When sub-panels are not shown (real commit), treat the whole
                // left block as the Staged (Changed Files) focus.
                if app.detail_focus != DetailSection::Staged {
                    app.detail_focus = DetailSection::Staged;
                    app.diff_scroll = 0;
                    if app.is_uncommitted_selected() {
                        app.staging_file_selection = 0;
                        app.refresh_staging_diff();
                    } else {
                        app.file_selection = 0;
                        app.refresh_file_diff();
                    }
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
                if app.detail_focus != DetailSection::Commits {
                    app.detail_focus = DetailSection::Commits;
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
                if app.detail_focus != DetailSection::LocalBranches {
                    app.detail_focus = DetailSection::LocalBranches;
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
                if app.detail_focus != DetailSection::RemoteBranches {
                    app.detail_focus = DetailSection::RemoteBranches;
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
                if app.detail_focus != DetailSection::LocalTags {
                    app.detail_focus = DetailSection::LocalTags;
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
    // Remotes list panel.
    if let Some(rect) = areas.remotes {
        if rect.contains(pos) {
            if is_click {
                if app.detail_focus != DetailSection::Remotes {
                    app.detail_focus = DetailSection::Remotes;
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
                if app.detail_focus != DetailSection::Stashes {
                    app.detail_focus = DetailSection::Stashes;
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
                if app.detail_focus != DetailSection::StashedFiles {
                    app.detail_focus = DetailSection::StashedFiles;
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
