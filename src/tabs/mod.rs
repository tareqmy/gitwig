use crate::app::{App, DetailSection, Mode};
use crate::components::Component;
use crossterm::event::{KeyCode, KeyEvent};

pub fn route_detail_event(app: &mut App, key: KeyEvent) -> bool {
    let code = key.code;
    let detail_focus = app.detail_focus;
    match code {
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
        KeyCode::Char('w') => {
            app.cycle_detail_focus(false);
            return true;
        }
        KeyCode::Char('W') => {
            app.cycle_detail_focus(true);
            return true;
        }
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
            // ── Commits panel ──────────────────────────────────────────────────
            if detail_focus == DetailSection::Commits {
                match code {
                    KeyCode::Up => {
                        app.detail_commit_up();
                        return true;
                    }
                    KeyCode::Down => {
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
                        app.commit_list.limit = app.commit_list.limit.saturating_add(200);
                        app.resync_detail();
                        app.status_message = Some("Loading more commits...".to_string());
                        return true;
                    }
                    KeyCode::Char('f') => {
                        app.search_column_selection = 0;
                        app.mode = Mode::SearchColumnPicker;
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
                    KeyCode::Char('s') | KeyCode::Char('S') if app.has_uncommitted_changes() => {
                        app.start_stash_create();
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
                }
            // ── Staged / Unstaged / Conflicts panels ───────────────────────────
            } else if detail_focus == DetailSection::Staged
                || detail_focus == DetailSection::Unstaged
                || detail_focus == DetailSection::Conflicts
            {
                match code {
                    KeyCode::Up => {
                        if detail_focus == DetailSection::Conflicts {
                            app.conflict_file_up();
                        } else if app.is_uncommitted_selected() {
                            app.staging_file_up();
                        } else {
                            app.detail_file_up();
                        }
                        return true;
                    }
                    KeyCode::Down => {
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
                    KeyCode::Char('s') | KeyCode::Char('S') if app.is_uncommitted_selected() => {
                        app.start_stash_create();
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
            // ── StagingDetails / ConflictDiff panels ───────────────────────────
            } else if detail_focus == DetailSection::StagingDetails
                || detail_focus == DetailSection::ConflictDiff
            {
                match code {
                    KeyCode::Up => {
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
                    KeyCode::Down => {
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
                                app.diff.diff_scroll =
                                    app.diff.diff_line_selection.saturating_sub(17);
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
                    KeyCode::Char('s') | KeyCode::Char('S') if app.is_uncommitted_selected() => {
                        app.start_stash_create();
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
                    _ => {}
                }
            // ── CommitDetails panel ────────────────────────────────────────────
            } else if detail_focus == DetailSection::CommitDetails {
                match code {
                    KeyCode::Up => {
                        app.commit_details_scroll_up();
                        return true;
                    }
                    KeyCode::Down => {
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
                }
            }
        }
        _ if app.detail_tab == 1 => {
            let ev = crossterm::event::Event::Key(key);
            if detail_focus == DetailSection::Files {
                // 'f' launches FZF file picker
                if code == KeyCode::Char('f') {
                    app.pending_files_fzf = true;
                    return true;
                }
                // '>'/'.' expand folder, '<'/',' collapse folder
                match code {
                    KeyCode::Char('>') | KeyCode::Char('.') => {
                        app.expand_selected_folder();
                        return true;
                    }
                    KeyCode::Char('<') | KeyCode::Char(',') => {
                        app.collapse_selected_folder();
                        return true;
                    }
                    _ => {}
                }
                if app
                    .file_tree
                    .event(&ev)
                    .unwrap_or(crate::components::EventState::NotConsumed)
                    .is_consumed()
                {
                    return true;
                }
            } else if detail_focus == DetailSection::FileContent {
                match code {
                    KeyCode::Right => {
                        app.inspect_full_diff = true;
                        return true;
                    }
                    KeyCode::Left if app.inspect_full_diff => {
                        app.inspect_full_diff = false;
                        return true;
                    }
                    KeyCode::Up => {
                        app.file_tree.queue.push(crate::queue::InternalEvent::FileContentUp)
                    }
                    KeyCode::Down => {
                        app.file_tree.queue.push(crate::queue::InternalEvent::FileContentDown)
                    }
                    KeyCode::PageUp => {
                        app.file_tree.queue.push(crate::queue::InternalEvent::FileContentPageUp)
                    }
                    KeyCode::PageDown => {
                        app.file_tree.queue.push(crate::queue::InternalEvent::FileContentPageDown)
                    }
                    KeyCode::Home => {
                        app.file_tree.queue.push(crate::queue::InternalEvent::FileContentTop)
                    }
                    KeyCode::End => {
                        app.file_tree.queue.push(crate::queue::InternalEvent::FileContentBottom)
                    }
                    _ => {}
                }
            }
        }
        _ if app.detail_tab == 3 => match code {
            KeyCode::Char('c') | KeyCode::Char('C') => {
                app.start_branch_create();
                return true;
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                app.request_branch_delete();
                return true;
            }
            KeyCode::Char('m') | KeyCode::Char('M') => {
                app.request_branch_merge();
                return true;
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                app.request_branch_rebase();
                return true;
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                app.request_branch_interactive_rebase();
                return true;
            }
            KeyCode::Char('F') if detail_focus == DetailSection::LocalBranches => {
                app.fetch_selected_branch();
                return true;
            }
            KeyCode::Char('P') if detail_focus == DetailSection::LocalBranches => {
                app.request_branch_push();
                return true;
            }
            KeyCode::Char('p') if detail_focus == DetailSection::LocalBranches => {
                app.pull_selected_branch();
                return true;
            }
            KeyCode::Enter => {
                app.request_branch_checkout();
                return true;
            }
            KeyCode::Left => {
                app.move_focus_left();
                return true;
            }
            KeyCode::Right => {
                app.move_focus_right();
                return true;
            }
            KeyCode::Up => {
                if detail_focus == DetailSection::LocalBranches {
                    app.local_branch_up();
                } else {
                    app.remote_branch_up();
                }
            }
            KeyCode::Down => {
                if detail_focus == DetailSection::LocalBranches {
                    app.local_branch_down();
                } else {
                    app.remote_branch_down();
                }
            }
            KeyCode::PageUp => {
                if detail_focus == DetailSection::LocalBranches {
                    app.local_branch_page_up(app.config.page_size);
                } else {
                    app.remote_branch_page_up(app.config.page_size);
                }
            }
            KeyCode::PageDown => {
                if detail_focus == DetailSection::LocalBranches {
                    app.local_branch_page_down(app.config.page_size);
                } else {
                    app.remote_branch_page_down(app.config.page_size);
                }
            }
            KeyCode::Home => {
                if detail_focus == DetailSection::LocalBranches {
                    app.local_branch_to_top();
                } else {
                    app.remote_branch_to_top();
                }
            }
            KeyCode::End => {
                if detail_focus == DetailSection::LocalBranches {
                    app.local_branch_to_bottom();
                } else {
                    app.remote_branch_to_bottom();
                }
            }
            _ => {}
        },
        _ if app.detail_tab == 4 => {
            let ev = crossterm::event::Event::Key(key);
            if app
                .tag_list
                .event(&ev)
                .unwrap_or(crate::components::EventState::NotConsumed)
                .is_consumed()
            {
                return true;
            }
        }
        _ if app.detail_tab == 5 => {
            match code {
                KeyCode::Up => app.remote_up(),
                KeyCode::Down => app.remote_down(),
                KeyCode::PageUp => app.remote_page_up(app.config.page_size),
                KeyCode::PageDown => app.remote_page_down(app.config.page_size),
                KeyCode::Home => app.remote_to_top(),
                KeyCode::End => app.remote_to_bottom(),
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    // Open picker if >1 remote, else fetch directly
                    let remote_action =
                        if let Some(crate::repo::ItemDetail::Repo { info, .. }) =
                            &app.current_detail
                        {
                            if info.remotes.len() > 1 {
                                Some(None)
                            } else {
                                info.remotes.first().map(|r| Some(r.name.clone()))
                            }
                        } else {
                            None
                        };
                    match remote_action {
                        Some(Some(name)) => app.fetch_remote(&name),
                        Some(None) => {
                            app.remote_picker_action =
                                Some(crate::app::RemotePickerAction::FetchRemote);
                            app.remote_picker_selection = app.branch_list.remote_selection;
                            app.mode = Mode::RemotePicker;
                        }
                        None => {}
                    }
                }
                KeyCode::Char('a') | KeyCode::Char('A') => app.start_remote_add(),
                KeyCode::Char('d') | KeyCode::Char('D') => app.request_remote_delete(),
                _ => {}
            }
        }
        _ if app.detail_tab == 6 => match code {
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
    }
    false
}

pub mod logs;
