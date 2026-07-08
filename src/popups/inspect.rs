use crate::app::{App, DetailSection, Mode};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub struct InspectPopup;

impl InspectPopup {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        if app.is_bound(Action::CloseDetail, key) {
            if app.inspect_full_diff {
                app.inspect_full_diff = false;
            } else if app.in_logs_ui {
                app.mode = Mode::Logs;
            } else {
                app.mode = Mode::Detail;
                app.detail_focus = DetailSection::Commits;
            }
            return true;
        }

        if app.is_bound(Action::DetailHelp, key) {
            app.open_detail_help();
            return true;
        }

        if app.is_bound(Action::CycleFocusForward, key) {
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
            return true;
        }

        if app.is_bound(Action::CycleFocusBackward, key) {
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
            return true;
        }

        if app.is_bound(Action::FilesFullScreen, key) {
            if app.detail_focus == DetailSection::StagingDetails
                || app.detail_focus == DetailSection::ConflictDiff
            {
                if !app.inspect_full_diff {
                    app.inspect_full_diff = true;
                    return true;
                }
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
                    return true;
                }
            } else {
                if app.detail_focus == DetailSection::Staged
                    || app.detail_focus == DetailSection::CommitDetails
                {
                    app.detail_focus = DetailSection::StagingDetails;
                    return true;
                }
            }
        }

        if key.code == crossterm::event::KeyCode::Left {
            if app.inspect_full_diff {
                app.inspect_full_diff = false;
                return true;
            } else if app.detail_focus == DetailSection::StagingDetails
                || app.detail_focus == DetailSection::ConflictDiff
            {
                if app.is_uncommitted_selected() {
                    app.detail_focus = app.last_staging_focus;
                } else {
                    app.detail_focus = DetailSection::CommitDetails;
                }
                return true;
            }
        }

        if app.is_bound(Action::DetailMoveUp, key) {
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
            return true;
        }

        if app.is_bound(Action::DetailMoveDown, key) {
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
            return true;
        }

        if app.is_bound(Action::DetailPageUp, key) {
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
            return true;
        }

        if app.is_bound(Action::DetailPageDown, key) {
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
            return true;
        }

        if app.is_bound(Action::DetailHome, key) {
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
            return true;
        }

        if app.is_bound(Action::DetailEnd, key) {
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
            return true;
        }

        if app.is_bound(Action::WorkspaceStage, key) && app.is_uncommitted_selected() {
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
            return true;
        }

        if app.is_bound(Action::DiffDiscard, key) && app.is_uncommitted_selected() {
            if app.detail_focus == DetailSection::StagingDetails
                && app.last_staging_focus == DetailSection::Unstaged
            {
                if app.diff.diff_line_mode {
                    app.discard_selected_line();
                } else {
                    app.discard_selected_hunk();
                }
                return true;
            }
        }

        if app.is_bound(Action::WorkspaceDiscard, key) && app.is_uncommitted_selected() {
            if app.detail_focus == DetailSection::Staged
                || app.detail_focus == DetailSection::Unstaged
            {
                app.request_discard_changes();
                return true;
            } else if app.detail_focus == DetailSection::StagingDetails
                && app.last_staging_focus == DetailSection::Unstaged
            {
                if app.diff.diff_line_mode {
                    app.discard_selected_line();
                } else {
                    app.discard_selected_hunk();
                }
                return true;
            }
        }

        if app.is_bound(Action::WorkspaceDiscardAll, key) && app.is_uncommitted_selected() {
            if app.detail_focus == DetailSection::Staged
                || app.detail_focus == DetailSection::Unstaged
            {
                app.request_discard_all_changes();
                return true;
            } else if app.detail_focus == DetailSection::StagingDetails
                && app.last_staging_focus == DetailSection::Unstaged
            {
                if app.diff.diff_line_mode {
                    app.discard_selected_line();
                } else {
                    app.discard_selected_hunk();
                }
                return true;
            }
        }

        if app.is_bound(Action::WorkspaceStageAll, key) && app.is_uncommitted_selected() {
            if app.detail_focus == DetailSection::Unstaged {
                app.stage_all_changes();
                return true;
            } else if app.detail_focus == DetailSection::Staged {
                app.unstage_all_changes();
                return true;
            }
        }

        if app.is_bound(Action::DiffLineMode, key) && app.is_uncommitted_selected() {
            if app.detail_focus == DetailSection::StagingDetails {
                app.toggle_diff_line_mode();
                return true;
            }
        }

        if app.is_bound(Action::DiffStage, key) && app.is_uncommitted_selected() {
            if app.detail_focus == DetailSection::StagingDetails {
                if app.last_staging_focus == DetailSection::Unstaged {
                    if app.diff.diff_line_mode {
                        app.stage_selected_line();
                    } else {
                        app.stage_selected_hunk();
                    }
                }
                return true;
            }
        }

        if app.is_bound(Action::DiffUnstage, key) && app.is_uncommitted_selected() {
            if app.detail_focus == DetailSection::StagingDetails {
                if app.last_staging_focus == DetailSection::Staged {
                    if app.diff.diff_line_mode {
                        app.unstage_selected_line();
                    } else {
                        app.unstage_selected_hunk();
                    }
                }
                return true;
            }
        }

        if app.is_bound(Action::ConflictOurs, key) && app.is_uncommitted_selected() {
            if app.detail_focus == DetailSection::Conflicts
                || app.detail_focus == DetailSection::ConflictDiff
            {
                app.resolve_conflict_ours();
                return true;
            }
        }

        if app.is_bound(Action::ConflictTheirs, key) && app.is_uncommitted_selected() {
            if app.detail_focus == DetailSection::Conflicts
                || app.detail_focus == DetailSection::ConflictDiff
            {
                app.resolve_conflict_theirs();
                return true;
            }
        }

        if app.is_bound(Action::ConflictResolve, key) && app.is_uncommitted_selected() {
            if app.detail_focus == DetailSection::Conflicts
                || app.detail_focus == DetailSection::ConflictDiff
            {
                app.mark_conflict_resolved();
                return true;
            }
        }

        if app.is_bound(Action::ConflictAbort, key) && app.is_uncommitted_selected() {
            if app.detail_focus == DetailSection::Conflicts
                || app.detail_focus == DetailSection::ConflictDiff
            {
                app.mode = Mode::MergeAbortConfirm;
                return true;
            }
        }

        if app.is_bound(Action::ConflictContinue, key) && app.is_uncommitted_selected() {
            if app.detail_focus == DetailSection::Conflicts
                || app.detail_focus == DetailSection::ConflictDiff
            {
                app.mode = Mode::MergeContinueConfirm;
                return true;
            }
        }

        if app.is_bound(Action::WorkspaceCommit, key) && app.is_uncommitted_selected() {
            app.start_commit();
            return true;
        }

        if app.is_bound(Action::WorkspaceCommitAmend, key) && app.is_uncommitted_selected() {
            app.start_commit_amend();
            return true;
        }

        false
    }
}
