use crate::app::{App, DetailSection, Mode};
use crossterm::event::{KeyCode, KeyEvent};

pub struct WorkspaceTab;

impl WorkspaceTab {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        let code = key.code;
        let detail_focus = app.detail_focus;
        match detail_focus {
            // ── Commits panel ──────────────────────────────────────────────────
            DetailSection::Commits => match code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    app.detail_commit_up();
                    return true;
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    app.detail_commit_down();
                    return true;
                }
                KeyCode::PageUp => {
                    app.detail_commit_page_up(app.config.page_size);
                    return true;
                }
                KeyCode::PageDown => {
                    app.detail_commit_page_down(app.config.page_size);
                    return true;
                }
                KeyCode::Home => {
                    app.detail_commit_to_top();
                    return true;
                }
                KeyCode::End => {
                    app.detail_commit_to_bottom();
                    return true;
                }
                KeyCode::Char('G') => {
                    if app.commit_list.limit > 0 {
                        let add_amount =
                            if app.config.max_commits > 0 { app.config.max_commits } else { 200 };
                        app.commit_list.limit = app.commit_list.limit.saturating_add(add_amount);
                        app.resync_detail();
                        app.status_message = Some("Loading more commits...".to_string());
                    }
                    return true;
                }
                KeyCode::Char('f') => {
                    app.search_column_selection = 0;
                    app.mode = Mode::SearchColumnPicker;
                    return true;
                }
                KeyCode::Char('l') | KeyCode::Char('L') => {
                    app.in_logs_ui = true;
                    app.mode = Mode::Logs;
                    return true;
                }
                KeyCode::Char('c') if !app.is_uncommitted_selected() => {
                    app.start_commit();
                    return true;
                }
                KeyCode::Char('C') if !app.is_uncommitted_selected() => {
                    app.start_commit_amend();
                    return true;
                }
                KeyCode::Char('c') if app.is_uncommitted_selected() => {
                    app.start_commit();
                    return true;
                }
                KeyCode::Char('C') if app.is_uncommitted_selected() => {
                    app.start_commit_amend();
                    return true;
                }
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    app.start_tag_create();
                    return true;
                }
                KeyCode::Char('b') | KeyCode::Char('B') => {
                    app.start_branch_create();
                    return true;
                }
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    app.run_interactive_rebase();
                    return true;
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    app.request_cherry_pick();
                    return true;
                }
                KeyCode::Char('v') | KeyCode::Char('V') => {
                    app.request_revert();
                    return true;
                }
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    app.yank_selected_commit_hash();
                    return true;
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    app.stashing_ui_selection = 0;
                    app.stash_untracked = true;
                    app.stash_keep_index = false;
                    app.mode = Mode::StashingUI;
                    return true;
                }
                KeyCode::Enter | KeyCode::Right => {
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
                _ => {}
            },
            // ── Staged / Unstaged / Conflicts panels ───────────────────────────
            DetailSection::Staged | DetailSection::Unstaged | DetailSection::Conflicts => {
                match code {
                    KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                        if detail_focus == DetailSection::Conflicts {
                            app.conflict_file_up();
                        } else if app.is_uncommitted_selected() {
                            app.staging_file_up();
                        } else {
                            app.detail_file_up();
                        }
                        return true;
                    }
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                        if detail_focus == DetailSection::Conflicts {
                            app.conflict_file_down();
                        } else if app.is_uncommitted_selected() {
                            app.staging_file_down();
                        } else {
                            app.detail_file_down();
                        }
                        return true;
                    }
                    KeyCode::PageUp => {
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
                    KeyCode::PageDown => {
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
                    KeyCode::Home => {
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
                    KeyCode::End => {
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
                    KeyCode::Enter
                        if detail_focus == DetailSection::Staged
                            && app.is_uncommitted_selected() =>
                    {
                        app.unstage_selected_file();
                        return true;
                    }
                    KeyCode::Enter
                        if detail_focus == DetailSection::Unstaged
                            && app.is_uncommitted_selected() =>
                    {
                        app.stage_selected_file();
                        return true;
                    }
                    KeyCode::Enter | KeyCode::Right if detail_focus == DetailSection::Conflicts => {
                        app.mode = Mode::Inspect;
                        app.last_staging_focus = DetailSection::Conflicts;
                        app.detail_focus = DetailSection::ConflictDiff;
                        app.diff.diff_scroll = 0;
                        app.refresh_staging_diff();
                        return true;
                    }
                    KeyCode::Right
                        if detail_focus == DetailSection::Staged
                            || detail_focus == DetailSection::Unstaged =>
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
                    KeyCode::Char('a') | KeyCode::Char('A') if app.is_uncommitted_selected() => {
                        if detail_focus == DetailSection::Unstaged {
                            app.stage_all_changes();
                        } else if detail_focus == DetailSection::Staged {
                            app.unstage_all_changes();
                        }
                        return true;
                    }
                    KeyCode::Char('x')
                        if app.is_uncommitted_selected()
                            && (detail_focus == DetailSection::Staged
                                || detail_focus == DetailSection::Unstaged) =>
                    {
                        app.request_discard_changes();
                        return true;
                    }
                    KeyCode::Char('X') if app.is_uncommitted_selected() => {
                        app.request_discard_all_changes();
                        return true;
                    }
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        app.stashing_ui_selection = 0;
                        app.stash_untracked = true;
                        app.stash_keep_index = false;
                        app.mode = Mode::StashingUI;
                        return true;
                    }
                    KeyCode::Char('c') if app.is_uncommitted_selected() => {
                        app.start_commit();
                        return true;
                    }
                    KeyCode::Char('C') if app.is_uncommitted_selected() => {
                        app.start_commit_amend();
                        return true;
                    }
                    KeyCode::Char('o')
                        if detail_focus == DetailSection::Conflicts
                            && app.is_uncommitted_selected() =>
                    {
                        app.resolve_conflict_ours();
                        return true;
                    }
                    KeyCode::Char('t')
                        if detail_focus == DetailSection::Conflicts
                            && app.is_uncommitted_selected() =>
                    {
                        app.resolve_conflict_theirs();
                        return true;
                    }
                    KeyCode::Char('r')
                        if detail_focus == DetailSection::Conflicts
                            && app.is_uncommitted_selected() =>
                    {
                        app.mark_conflict_resolved();
                        return true;
                    }
                    KeyCode::Char('A')
                        if detail_focus == DetailSection::Conflicts
                            && app.is_uncommitted_selected() =>
                    {
                        app.mode = Mode::MergeAbortConfirm;
                        return true;
                    }
                    KeyCode::Char('C')
                        if detail_focus == DetailSection::Conflicts
                            && app.is_uncommitted_selected() =>
                    {
                        app.mode = Mode::MergeContinueConfirm;
                        return true;
                    }
                    _ => {}
                }
            }
            // ── StagingDetails / ConflictDiff panels ───────────────────────────
            DetailSection::StagingDetails | DetailSection::ConflictDiff => match code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
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
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
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
                KeyCode::PageUp => {
                    app.diff.diff_scroll_page_up(app.config.page_size);
                    return true;
                }
                KeyCode::PageDown => {
                    app.diff.diff_scroll_page_down(app.config.page_size);
                    return true;
                }
                KeyCode::Home => {
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
                KeyCode::End => {
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
                KeyCode::Enter
                    if app.is_uncommitted_selected()
                        && detail_focus == DetailSection::StagingDetails =>
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
                KeyCode::Delete
                    if app.is_uncommitted_selected()
                        && detail_focus == DetailSection::StagingDetails
                        && app.last_staging_focus == DetailSection::Unstaged =>
                {
                    if app.diff.diff_line_mode {
                        app.discard_selected_line();
                    } else {
                        app.discard_selected_hunk();
                    }
                    return true;
                }
                KeyCode::Char('x')
                    if app.is_uncommitted_selected()
                        && detail_focus == DetailSection::StagingDetails
                        && app.last_staging_focus == DetailSection::Unstaged =>
                {
                    if app.diff.diff_line_mode {
                        app.discard_selected_line();
                    } else {
                        app.discard_selected_hunk();
                    }
                    return true;
                }
                KeyCode::Char('X') if app.is_uncommitted_selected() => {
                    app.request_discard_all_changes();
                    return true;
                }
                KeyCode::Char('l') | KeyCode::Char('L')
                    if app.is_uncommitted_selected()
                        && detail_focus == DetailSection::StagingDetails =>
                {
                    app.toggle_diff_line_mode();
                    return true;
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    app.stashing_ui_selection = 0;
                    app.stash_untracked = true;
                    app.stash_keep_index = false;
                    app.mode = Mode::StashingUI;
                    return true;
                }
                KeyCode::Char('c') if app.is_uncommitted_selected() => {
                    app.start_commit();
                    return true;
                }
                KeyCode::Char('C') if app.is_uncommitted_selected() => {
                    app.start_commit_amend();
                    return true;
                }
                KeyCode::Char('o') if app.is_uncommitted_selected() => {
                    app.resolve_conflict_ours();
                    return true;
                }
                KeyCode::Char('t') if app.is_uncommitted_selected() => {
                    app.resolve_conflict_theirs();
                    return true;
                }
                KeyCode::Char('r') if app.is_uncommitted_selected() => {
                    app.mark_conflict_resolved();
                    return true;
                }
                KeyCode::Char('A') if app.is_uncommitted_selected() => {
                    app.mode = Mode::MergeAbortConfirm;
                    return true;
                }
                KeyCode::Right => {
                    app.mode = Mode::Inspect;
                    return true;
                }
                _ => {}
            },
            // ── CommitDetails panel ────────────────────────────────────────────
            DetailSection::CommitDetails => match code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    app.commit_details_scroll_up();
                    return true;
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    app.commit_details_scroll_down();
                    return true;
                }
                KeyCode::PageUp => {
                    for _ in 0..app.config.page_size {
                        app.commit_details_scroll_up();
                    }
                    return true;
                }
                KeyCode::PageDown => {
                    for _ in 0..app.config.page_size {
                        app.commit_details_scroll_down();
                    }
                    return true;
                }
                KeyCode::Home => {
                    app.commit_list.details_scroll = 0;
                    return true;
                }
                KeyCode::End => {
                    app.commit_list.details_scroll = usize::MAX;
                    return true;
                }
                KeyCode::Right => {
                    app.detail_focus = DetailSection::StagingDetails;
                    app.diff.diff_scroll = 0;
                    app.mode = Mode::Inspect;
                    app.refresh_file_diff();
                    return true;
                }
                _ => {}
            },
            _ => {}
        }
        false
    }
}
