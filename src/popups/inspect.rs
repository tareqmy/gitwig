use crate::app::{App, DetailSection, Mode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct InspectPopup;

impl InspectPopup {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        let code = key.code;

        match code {
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
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
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
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
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
                        for _ in 0..app.get_current_page_size() {
                            app.conflict_file_up();
                        }
                    } else if app.is_uncommitted_selected() {
                        for _ in 0..app.get_current_page_size() {
                            app.staging_file_up();
                        }
                    } else {
                        for _ in 0..app.get_current_page_size() {
                            app.detail_file_up();
                        }
                    }
                } else if app.detail_focus == DetailSection::CommitDetails {
                    for _ in 0..app.get_current_page_size() {
                        app.commit_list.details_scroll_up();
                    }
                } else {
                    app.diff.diff_scroll_page_up(app.get_current_page_size());
                }
            }
            KeyCode::PageDown => {
                if app.detail_focus == DetailSection::Staged
                    || app.detail_focus == DetailSection::Unstaged
                    || app.detail_focus == DetailSection::Conflicts
                {
                    if app.detail_focus == DetailSection::Conflicts {
                        for _ in 0..app.get_current_page_size() {
                            app.conflict_file_down();
                        }
                    } else if app.is_uncommitted_selected() {
                        for _ in 0..app.get_current_page_size() {
                            app.staging_file_down();
                        }
                    } else {
                        for _ in 0..app.get_current_page_size() {
                            app.detail_file_down();
                        }
                    }
                } else if app.detail_focus == DetailSection::CommitDetails {
                    for _ in 0..app.get_current_page_size() {
                        app.commit_list.details_scroll_down();
                    }
                } else {
                    app.diff.diff_scroll_page_down(app.get_current_page_size());
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
                        app.status_list.staging_file_selection =
                            app.staging_file_total().saturating_sub(1);
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
                            app.diff.diff_line_selection =
                                app.diff.file_diff.len().saturating_sub(1);
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
        }
        false
    }
}
