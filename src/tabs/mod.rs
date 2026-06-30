use crate::app::{App, DetailSection, Mode};
use crate::repo;
use crossterm::event::{KeyCode, KeyEvent};

pub mod branches;
pub mod file_history;
pub mod files;
pub mod graph;
pub mod home;
pub mod logs;
pub mod remotes;
pub mod stashes;
pub mod tags;
pub mod workspace;

pub use branches::BranchesTab;
pub use file_history::FileHistoryTab;
pub use files::FilesTab;
pub use graph::GraphTab;
pub use home::HomeTab;
pub use logs::LogsTab;
pub use remotes::RemotesTab;
pub use stashes::StashesTab;
pub use tags::TagsTab;
pub use workspace::WorkspaceTab;

pub fn route_detail_event(app: &mut App, key: KeyEvent) -> bool {
    let code = key.code;
    use crate::keybindings::Action;

    if code == KeyCode::Char('v') || code == KeyCode::Char('V') {
        app.mode = Mode::Overview;
        app.trigger_overview_load_if_needed();
        return true;
    }

    if app.is_bound(Action::CloseDetail, key) {
        if code == KeyCode::Esc {
            if app.inspect_full_diff {
                app.inspect_full_diff = false;
                return true;
            } else if app.commit_list.search_query.is_some() {
                app.cancel_commit_search();
                return true;
            }
        }
        app.close_detail();
        return true;
    }

    if app.is_bound(Action::DetailHelp, key) {
        app.open_detail_help();
        return true;
    }

    if app.is_bound(Action::CycleFocusForward, key) {
        app.cycle_detail_focus(false);
        return true;
    }

    if app.is_bound(Action::CycleFocusBackward, key) {
        app.cycle_detail_focus(true);
        return true;
    }

    if app.is_bound(Action::RefreshDetail, key) {
        app.resync_detail();
        app.status_message = Some("Refreshed".to_string());
        return true;
    }

    if app.is_bound(Action::CycleTabForward, key) {
        app.inspect_full_diff = false;
        app.detail_tab = (app.detail_tab + 1) % 9;
        app.commit_list.selection = 0;
        app.set_default_focus_for_tab();
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::CycleTabBackward, key) {
        app.inspect_full_diff = false;
        app.detail_tab = if app.detail_tab == 0 { 8 } else { app.detail_tab - 1 };
        app.commit_list.selection = 0;
        app.set_default_focus_for_tab();
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab1, key) {
        app.inspect_full_diff = false;
        app.detail_tab = 0;
        app.commit_list.selection = 0;
        app.detail_focus = DetailSection::Commits;
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab2, key) {
        app.inspect_full_diff = false;
        app.detail_tab = 1;
        app.commit_list.selection = 0;
        app.detail_focus = DetailSection::Files;
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab3, key) {
        app.inspect_full_diff = false;
        app.detail_tab = 2;
        app.commit_list.selection = 0;
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab4, key) {
        app.inspect_full_diff = false;
        app.detail_tab = 3;
        app.commit_list.selection = 0;
        app.detail_focus = DetailSection::LocalBranches;
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab5, key) {
        app.inspect_full_diff = false;
        app.detail_tab = 4;
        app.commit_list.selection = 0;
        app.detail_focus = DetailSection::LocalTags;
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab6, key) {
        app.inspect_full_diff = false;
        app.detail_tab = 5;
        app.commit_list.selection = 0;
        app.detail_focus = DetailSection::Remotes;
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab7, key) {
        app.inspect_full_diff = false;
        app.detail_tab = 6;
        app.commit_list.selection = 0;
        app.detail_focus = DetailSection::Stashes;
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab8, key) {
        app.inspect_full_diff = false;
        app.detail_tab = 7;
        app.commit_list.selection = 0;
        app.detail_focus = DetailSection::Worktrees;
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab9, key) {
        app.inspect_full_diff = false;
        app.detail_tab = 8;
        app.commit_list.selection = 0;
        app.detail_focus = DetailSection::Submodules;
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    match app.detail_tab {
        0 => return WorkspaceTab::handle_event(app, key),
        1 => return FilesTab::handle_event(app, key),
        2 => return GraphTab::handle_event(app, key),
        3 => return BranchesTab::handle_event(app, key),
        4 => return TagsTab::handle_event(app, key),
        5 => return RemotesTab::handle_event(app, key),
        6 => return StashesTab::handle_event(app, key),
        7 => return handle_worktree_events(app, key),
        8 => return handle_submodule_events(app, key),
        _ => {}
    }
    false
}

fn handle_submodule_events(app: &mut App, key: KeyEvent) -> bool {
    let subs_count = if let Some(repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
        if let repo::TabData::Loaded(subs) = &info.submodules { subs.len() } else { 0 }
    } else {
        0
    };

    let code = key.code;
    match code {
        KeyCode::Down | KeyCode::Char('j') => {
            if subs_count > 0 {
                app.submodule_selection = (app.submodule_selection + 1).min(subs_count - 1);
            }
            true
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.submodule_selection = app.submodule_selection.saturating_sub(1);
            true
        }
        KeyCode::Char('a') => {
            app.submodule_add_url.clear();
            app.submodule_add_path.clear();
            app.input_buffer.clear();
            app.mode = Mode::SubmoduleAddUrlInput;
            true
        }
        KeyCode::Char('D') => {
            if let Some(repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
                if let repo::TabData::Loaded(subs) = &info.submodules {
                    if let Some(sub) = subs.get(app.submodule_selection) {
                        app.submodule_delete_target = Some(sub.name.clone());
                        app.mode = Mode::SubmoduleDeleteConfirm;
                    }
                }
            }
            true
        }
        _ => false,
    }
}

fn handle_worktree_events(app: &mut App, key: KeyEvent) -> bool {
    let wts_count = if let Some(repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
        if let repo::TabData::Loaded(wts) = &info.worktrees { wts.len() } else { 0 }
    } else {
        0
    };

    let code = key.code;
    match code {
        KeyCode::Down | KeyCode::Char('j') => {
            if wts_count > 0 {
                app.worktree_selection = (app.worktree_selection + 1).min(wts_count - 1);
            }
            true
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.worktree_selection = app.worktree_selection.saturating_sub(1);
            true
        }
        KeyCode::Char('a') => {
            app.worktree_add_branch.clear();
            app.worktree_add_path.clear();
            app.input_buffer.clear();
            app.mode = Mode::WorktreeAddBranchInput;
            true
        }
        KeyCode::Char('l') => {
            if let Some(repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
                if let repo::TabData::Loaded(wts) = &info.worktrees {
                    if let Some(wt) = wts.get(app.worktree_selection) {
                        if wt.is_locked {
                            let path_str = match app.config.items.get(app.selected_index) {
                                Some(p) => p,
                                None => return false,
                            };
                            let path = repo::expand_tilde(path_str);
                            match repo::worktree_unlock(&path, &wt.name) {
                                Ok(_) => {
                                    app.status_message =
                                        Some("Worktree unlocked successfully".to_string());
                                    app.resync_detail();
                                }
                                Err(e) => {
                                    app.status_message = Some(format!("Failed to unlock: {}", e));
                                }
                            }
                        } else {
                            app.worktree_lock_reason.clear();
                            app.input_buffer.clear();
                            app.mode = Mode::WorktreeLockReasonInput;
                        }
                    }
                }
            }
            true
        }
        KeyCode::Char('D') => {
            app.worktree_remove_delete_folder = false;
            app.worktree_remove_force = false;
            app.input_buffer.clear();
            app.mode = Mode::WorktreeRemoveConfirm;
            true
        }
        KeyCode::Char('p') => {
            let path_str = match app.config.items.get(app.selected_index) {
                Some(p) => p,
                None => return false,
            };
            let path = repo::expand_tilde(path_str);
            match repo::worktree_prune(&path) {
                Ok(_) => {
                    app.status_message = Some("Pruned stale worktree metadata".to_string());
                    app.resync_detail();
                }
                Err(e) => {
                    app.status_message = Some(format!("Prune failed: {}", e));
                }
            }
            true
        }
        KeyCode::Enter => {
            if let Some(repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
                if let repo::TabData::Loaded(wts) = &info.worktrees {
                    if let Some(wt) = wts.get(app.worktree_selection) {
                        let wt_path = wt.path.clone();
                        if wt_path.exists() {
                            let wt_path_str = wt_path.to_string_lossy().to_string();
                            if !app.config.items.contains(&wt_path_str) {
                                app.config.items.push(wt_path_str.clone());
                                let _ = crate::config::save_config(&app.config, &app.config_path);
                                app.original_items = app.config.items.clone();
                                if app.config.sort_by != crate::config::SortOrder::Custom {
                                    app.sort_items_in_place();
                                }
                            }
                            if let Some(pos) =
                                app.config.items.iter().position(|x| x == &wt_path_str)
                            {
                                app.selected_index = pos;
                                app.open_detail();
                            }
                        } else {
                            app.status_message =
                                Some("Worktree path does not exist on disk".to_string());
                        }
                    }
                }
            }
            true
        }
        _ => false,
    }
}
