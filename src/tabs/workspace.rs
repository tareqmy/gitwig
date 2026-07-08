use crate::app::{App, DetailSection, Mode};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub struct WorkspaceTab;

impl WorkspaceTab {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        let detail_focus = app.detail_focus;
        match detail_focus {
            // ── Commits panel ──────────────────────────────────────────────────
            DetailSection::Commits => {
                if app.is_bound(Action::DetailMoveUp, key) {
                    app.detail_commit_up();
                    return true;
                }
                if app.is_bound(Action::DetailMoveDown, key) {
                    app.detail_commit_down();
                    return true;
                }
                if app.is_bound(Action::DetailPageUp, key) {
                    app.detail_commit_page_up(app.config.page_size);
                    return true;
                }
                if app.is_bound(Action::DetailPageDown, key) {
                    app.detail_commit_page_down(app.config.page_size);
                    return true;
                }
                if app.is_bound(Action::DetailHome, key) {
                    app.detail_commit_to_top();
                    return true;
                }
                if app.is_bound(Action::DetailEnd, key) {
                    app.detail_commit_to_bottom();
                    return true;
                }
                if app.is_bound(Action::WorkspaceLoadMore, key) {
                    if app.commit_list.limit > 0 {
                        let add_amount =
                            if app.config.max_commits > 0 { app.config.max_commits } else { 200 };
                        app.commit_list.limit = app.commit_list.limit.saturating_add(add_amount);
                        app.resync_detail();
                        app.status_message = Some("Loading more commits...".to_string());
                    }
                    return true;
                }
                if app.is_bound(Action::WorkspaceFuzzySearch, key) {
                    app.start_commit_fuzzy_search();
                    return true;
                }
                if app.is_bound(Action::WorkspaceColumnPicker, key) {
                    app.search_column_selection = 0;
                    app.mode = Mode::SearchColumnPicker;
                    return true;
                }
                if app.is_bound(Action::WorkspaceLogsView, key) {
                    app.in_logs_ui = true;
                    app.mode = Mode::Logs;
                    return true;
                }
                if app.is_bound(Action::WorkspaceCommit, key) {
                    app.start_commit();
                    return true;
                }
                if app.is_bound(Action::WorkspaceCommitAmend, key) {
                    app.start_commit_amend();
                    return true;
                }
                if app.is_bound(Action::WorkspaceCreateTag, key) {
                    app.start_tag_create();
                    return true;
                }
                if app.is_bound(Action::WorkspaceCreateBranch, key) {
                    app.start_branch_create();
                    return true;
                }
                if app.is_bound(Action::WorkspaceInteractiveRebase, key) {
                    app.run_interactive_rebase();
                    return true;
                }
                if app.is_bound(Action::WorkspaceCherryPick, key) {
                    app.request_cherry_pick();
                    return true;
                }
                if app.is_bound(Action::WorkspaceRevert, key) {
                    app.request_revert();
                    return true;
                }
                if app.is_bound(Action::WorkspaceYankHash, key) {
                    app.yank_selected_commit_hash();
                    return true;
                }
                if app.is_bound(Action::WorkspaceStashUI, key) {
                    app.stashing_ui_selection = 0;
                    app.stash_untracked = true;
                    app.stash_keep_index = false;
                    app.mode = Mode::StashingUI;
                    return true;
                }
                if app.is_bound(Action::HomeOpenDetail, key) {
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
                    return true;
                }
            }
            // ── Staged / Unstaged / Conflicts panels ───────────────────────────
            DetailSection::Staged | DetailSection::Unstaged | DetailSection::Conflicts => {
                if app.is_bound(Action::DetailMoveUp, key) {
                    if detail_focus == DetailSection::Conflicts {
                        app.conflict_file_up();
                    } else if app.is_uncommitted_selected() {
                        app.staging_file_up();
                    } else {
                        app.detail_file_up();
                    }
                    return true;
                }
                if app.is_bound(Action::DetailMoveDown, key) {
                    if detail_focus == DetailSection::Conflicts {
                        app.conflict_file_down();
                    } else if app.is_uncommitted_selected() {
                        app.staging_file_down();
                    } else {
                        app.detail_file_down();
                    }
                    return true;
                }
                if app.is_bound(Action::DetailPageUp, key) {
                    let n = app.config.page_size;
                    if detail_focus == DetailSection::Conflicts {
                        for _ in 0..n {
                            app.conflict_file_up();
                        }
                    } else if app.is_uncommitted_selected() {
                        for _ in 0..n {
                            app.staging_file_up();
                        }
                    } else {
                        for _ in 0..n {
                            app.detail_file_up();
                        }
                    }
                    return true;
                }
                if app.is_bound(Action::DetailPageDown, key) {
                    let n = app.config.page_size;
                    if detail_focus == DetailSection::Conflicts {
                        for _ in 0..n {
                            app.conflict_file_down();
                        }
                    } else if app.is_uncommitted_selected() {
                        for _ in 0..n {
                            app.staging_file_down();
                        }
                    } else {
                        for _ in 0..n {
                            app.detail_file_down();
                        }
                    }
                    return true;
                }
                if app.is_bound(Action::DetailHome, key) {
                    if detail_focus == DetailSection::Conflicts {
                        app.status_list.conflict_file_selection = 0;
                        app.refresh_staging_diff();
                    } else if app.is_uncommitted_selected() {
                        app.status_list.staging_file_selection = 0;
                        app.refresh_staging_diff();
                    } else {
                        app.status_list.file_selection = 0;
                        app.refresh_file_diff();
                    }
                    return true;
                }
                if app.is_bound(Action::DetailEnd, key) {
                    if detail_focus == DetailSection::Conflicts {
                        let len = if let Some(crate::repo::ItemDetail::Repo { info, .. }) =
                            &app.current_detail
                        {
                            info.changes.conflicted.len()
                        } else {
                            0
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
                    return true;
                }
                if app.is_bound(Action::WorkspaceStage, key) {
                    if detail_focus == DetailSection::Staged && app.is_uncommitted_selected() {
                        app.unstage_selected_file();
                        return true;
                    }
                    if detail_focus == DetailSection::Unstaged && app.is_uncommitted_selected() {
                        app.stage_selected_file();
                        return true;
                    }
                    if detail_focus == DetailSection::Conflicts {
                        app.mode = Mode::Inspect;
                        app.last_staging_focus = DetailSection::Conflicts;
                        app.detail_focus = DetailSection::ConflictDiff;
                        app.diff.diff_scroll = 0;
                        app.refresh_staging_diff();
                        return true;
                    }
                }
                if app.is_bound(Action::FilesFullScreen, key) {
                    if detail_focus == DetailSection::Staged
                        || detail_focus == DetailSection::Unstaged
                    {
                        app.last_staging_focus = detail_focus;
                        app.detail_focus = DetailSection::StagingDetails;
                        app.diff.diff_scroll = 0;
                        app.mode = Mode::Inspect;
                        if app.is_uncommitted_selected() {
                            app.refresh_staging_diff();
                        } else {
                            app.refresh_file_diff();
                        }
                        return true;
                    }
                }
                if app.is_bound(Action::WorkspaceStageAll, key) {
                    if app.is_uncommitted_selected() && detail_focus != DetailSection::Conflicts {
                        if detail_focus == DetailSection::Unstaged {
                            app.stage_all_changes();
                        } else if detail_focus == DetailSection::Staged {
                            app.unstage_all_changes();
                        }
                        return true;
                    }
                }
                if app.is_bound(Action::WorkspaceDiscard, key) {
                    if app.is_uncommitted_selected()
                        && (detail_focus == DetailSection::Staged
                            || detail_focus == DetailSection::Unstaged)
                    {
                        app.request_discard_changes();
                        return true;
                    }
                }
                if app.is_bound(Action::WorkspaceDiscardAll, key) {
                    if app.is_uncommitted_selected() {
                        app.request_discard_all_changes();
                        return true;
                    }
                }
                if app.is_bound(Action::WorkspaceStashUI, key) {
                    app.stashing_ui_selection = 0;
                    app.stash_untracked = true;
                    app.stash_keep_index = false;
                    app.mode = Mode::StashingUI;
                    return true;
                }
                if app.is_bound(Action::WorkspaceCommit, key) {
                    if app.is_uncommitted_selected() {
                        app.start_commit();
                        return true;
                    }
                }
                if app.is_bound(Action::WorkspaceCommitAmend, key) {
                    if app.is_uncommitted_selected() && detail_focus != DetailSection::Conflicts {
                        app.start_commit_amend();
                        return true;
                    }
                }
                if app.is_bound(Action::ConflictOurs, key) {
                    if detail_focus == DetailSection::Conflicts && app.is_uncommitted_selected() {
                        app.resolve_conflict_ours();
                        return true;
                    }
                }
                if app.is_bound(Action::ConflictTheirs, key) {
                    if detail_focus == DetailSection::Conflicts && app.is_uncommitted_selected() {
                        app.resolve_conflict_theirs();
                        return true;
                    }
                }
                if app.is_bound(Action::ConflictResolve, key) {
                    if detail_focus == DetailSection::Conflicts && app.is_uncommitted_selected() {
                        app.mark_conflict_resolved();
                        return true;
                    }
                }
                if app.is_bound(Action::ConflictAbort, key) {
                    if detail_focus == DetailSection::Conflicts && app.is_uncommitted_selected() {
                        app.mode = Mode::MergeAbortConfirm;
                        return true;
                    }
                }
                if app.is_bound(Action::ConflictContinue, key) {
                    if detail_focus == DetailSection::Conflicts && app.is_uncommitted_selected() {
                        app.mode = Mode::MergeContinueConfirm;
                        return true;
                    }
                }
            }
            // ── StagingDetails / ConflictDiff panels ───────────────────────────
            DetailSection::StagingDetails | DetailSection::ConflictDiff => {
                if app.is_bound(Action::DetailMoveUp, key) {
                    if app.is_uncommitted_selected() {
                        if app.diff.diff_line_mode {
                            app.diff_line_up();
                        } else {
                            app.diff_hunk_up();
                        }
                    } else {
                        app.diff.diff_scroll_up();
                    }
                    return true;
                }
                if app.is_bound(Action::DetailMoveDown, key) {
                    if app.is_uncommitted_selected() {
                        if app.diff.diff_line_mode {
                            app.diff_line_down();
                        } else {
                            app.diff_hunk_down();
                        }
                    } else {
                        app.diff.diff_scroll_down();
                    }
                    return true;
                }
                if app.is_bound(Action::DetailPageUp, key) {
                    app.diff.diff_scroll_page_up(app.config.page_size);
                    return true;
                }
                if app.is_bound(Action::DetailPageDown, key) {
                    app.diff.diff_scroll_page_down(app.config.page_size);
                    return true;
                }
                if app.is_bound(Action::DetailHome, key) {
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
                    return true;
                }
                if app.is_bound(Action::DetailEnd, key) {
                    if app.is_uncommitted_selected() {
                        if app.diff.diff_line_mode {
                            app.diff.diff_line_selection =
                                app.diff.file_diff.len().saturating_sub(1);
                            app.diff.diff_scroll = app.diff.diff_line_selection.saturating_sub(17);
                        } else {
                            let hc = app.get_diff_hunk_ranges().len();
                            app.diff.diff_hunk_selection = hc.saturating_sub(1);
                            app.scroll_to_selected_hunk();
                        }
                    } else {
                        app.diff.diff_scroll_to_bottom();
                    }
                    return true;
                }
                if app.is_bound(Action::WorkspaceStage, key) {
                    if app.is_uncommitted_selected()
                        && detail_focus == DetailSection::StagingDetails
                    {
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
                        return true;
                    }
                }
                if app.is_bound(Action::DiffDiscard, key) {
                    if app.is_uncommitted_selected()
                        && detail_focus == DetailSection::StagingDetails
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
                if app.is_bound(Action::WorkspaceDiscardAll, key) {
                    if app.is_uncommitted_selected() {
                        app.request_discard_all_changes();
                        return true;
                    }
                }
                if app.is_bound(Action::DiffLineMode, key) {
                    if app.is_uncommitted_selected()
                        && detail_focus == DetailSection::StagingDetails
                    {
                        app.toggle_diff_line_mode();
                        return true;
                    }
                }
                if app.is_bound(Action::DiffStage, key) {
                    if app.is_uncommitted_selected()
                        && detail_focus == DetailSection::StagingDetails
                    {
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
                if app.is_bound(Action::DiffUnstage, key) {
                    if app.is_uncommitted_selected()
                        && detail_focus == DetailSection::StagingDetails
                    {
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
                if app.is_bound(Action::WorkspaceStashUI, key) {
                    app.stashing_ui_selection = 0;
                    app.stash_untracked = true;
                    app.stash_keep_index = false;
                    app.mode = Mode::StashingUI;
                    return true;
                }
                if app.is_bound(Action::WorkspaceCommit, key) {
                    if app.is_uncommitted_selected() {
                        app.start_commit();
                        return true;
                    }
                }
                if app.is_bound(Action::WorkspaceCommitAmend, key) {
                    if app.is_uncommitted_selected() {
                        app.start_commit_amend();
                        return true;
                    }
                }
                if app.is_bound(Action::ConflictOurs, key) {
                    if app.is_uncommitted_selected() {
                        app.resolve_conflict_ours();
                        return true;
                    }
                }
                if app.is_bound(Action::ConflictTheirs, key) {
                    if app.is_uncommitted_selected() {
                        app.resolve_conflict_theirs();
                        return true;
                    }
                }
                if app.is_bound(Action::ConflictResolve, key) {
                    if app.is_uncommitted_selected() {
                        app.mark_conflict_resolved();
                        return true;
                    }
                }
                if app.is_bound(Action::ConflictAbort, key) {
                    if app.is_uncommitted_selected() {
                        app.mode = Mode::MergeAbortConfirm;
                        return true;
                    }
                }
                if app.is_bound(Action::FilesFullScreen, key) {
                    app.mode = Mode::Inspect;
                    return true;
                }
            }
            // ── CommitDetails panel ────────────────────────────────────────────
            DetailSection::CommitDetails => {
                if app.is_bound(Action::DetailMoveUp, key) {
                    app.commit_details_scroll_up();
                    return true;
                }
                if app.is_bound(Action::DetailMoveDown, key) {
                    app.commit_details_scroll_down();
                    return true;
                }
                if app.is_bound(Action::DetailPageUp, key) {
                    for _ in 0..app.config.page_size {
                        app.commit_details_scroll_up();
                    }
                    return true;
                }
                if app.is_bound(Action::DetailPageDown, key) {
                    for _ in 0..app.config.page_size {
                        app.commit_details_scroll_down();
                    }
                    return true;
                }
                if app.is_bound(Action::DetailHome, key) {
                    app.commit_list.details_scroll = 0;
                    return true;
                }
                if app.is_bound(Action::DetailEnd, key) {
                    app.commit_list.details_scroll = usize::MAX;
                    return true;
                }
                if app.is_bound(Action::FilesFullScreen, key) {
                    app.detail_focus = DetailSection::StagingDetails;
                    app.diff.diff_scroll = 0;
                    app.mode = Mode::Inspect;
                    app.refresh_file_diff();
                    return true;
                }
            }
            _ => {}
        }
        false
    }
}
