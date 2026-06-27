
use crate::app::{App, DetailSection, Mode};
use crossterm::event::{KeyCode, KeyEvent};
use crate::components::Component;

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
        }
    false
}

pub mod logs;
