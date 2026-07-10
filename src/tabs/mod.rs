use crate::app::{App, Mode};
use crate::keybindings::Action;
use crate::repo;
use crossterm::event::KeyEvent;

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
    if app.is_bound(Action::Overview, key) {
        app.mode = Mode::Overview;
        app.trigger_overview_load_if_needed();
        return true;
    }

    if app.is_bound(Action::CloseDetail, key) {
        if app.inspect_full_diff {
            app.inspect_full_diff = false;
            return true;
        } else if app.commit_list.search_query.is_some() {
            app.cancel_commit_search();
            return true;
        }
        if app.advanced_tabs {
            app.advanced_tabs = false;
            app.detail_tab = 0;
            app.set_default_focus_for_tab();
            if app.get_current_resync_on_tab_change() {
                app.resync_detail();
            }
            return true;
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
        if app.advanced_tabs {
            app.detail_tab = 7 + (app.detail_tab - 7 + 1) % 5;
        } else {
            app.detail_tab = (app.detail_tab + 1) % 7;
        }
        app.commit_list.selection = 0;
        app.set_default_focus_for_tab();
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::CycleTabBackward, key) {
        app.inspect_full_diff = false;
        if app.advanced_tabs {
            app.detail_tab = 7 + if app.detail_tab == 7 { 4 } else { app.detail_tab - 7 - 1 };
        } else {
            app.detail_tab = if app.detail_tab == 0 { 6 } else { app.detail_tab - 1 };
        }
        app.commit_list.selection = 0;
        app.set_default_focus_for_tab();
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::ToggleAdvancedTabs, key) {
        app.inspect_full_diff = false;
        if app.advanced_tabs {
            app.advanced_tabs = false;
            app.detail_tab = 0;
        } else {
            app.advanced_tabs = true;
            app.detail_tab = 7;
        }
        app.commit_list.selection = 0;
        app.set_default_focus_for_tab();
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab1, key) {
        app.inspect_full_diff = false;
        app.detail_tab = if app.advanced_tabs { 7 } else { 0 };
        app.commit_list.selection = 0;
        app.set_default_focus_for_tab();
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab2, key) {
        app.inspect_full_diff = false;
        app.detail_tab = if app.advanced_tabs { 8 } else { 1 };
        app.commit_list.selection = 0;
        app.set_default_focus_for_tab();
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab3, key) {
        app.inspect_full_diff = false;
        app.detail_tab = if app.advanced_tabs { 9 } else { 2 };
        app.commit_list.selection = 0;
        app.set_default_focus_for_tab();
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab4, key) {
        app.inspect_full_diff = false;
        app.detail_tab = if app.advanced_tabs { 10 } else { 3 };
        app.commit_list.selection = 0;
        app.set_default_focus_for_tab();
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab5, key) {
        app.inspect_full_diff = false;
        app.detail_tab = if app.advanced_tabs { 11 } else { 4 };
        app.commit_list.selection = 0;
        app.set_default_focus_for_tab();
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab6, key) {
        if !app.advanced_tabs {
            app.inspect_full_diff = false;
            app.detail_tab = 5;
            app.commit_list.selection = 0;
            app.set_default_focus_for_tab();
            if app.get_current_resync_on_tab_change() {
                app.resync_detail();
            }
            return true;
        }
    }

    if app.is_bound(Action::GoToTab7, key) {
        if !app.advanced_tabs {
            app.inspect_full_diff = false;
            app.detail_tab = 6;
            app.commit_list.selection = 0;
            app.set_default_focus_for_tab();
            if app.get_current_resync_on_tab_change() {
                app.resync_detail();
            }
            return true;
        }
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
        9 => return handle_reflog_events(app, key),
        10 => return handle_forge_events(app, key),
        11 => return handle_forge_pr_events(app, key),
        _ => {}
    }
    false
}

fn handle_forge_events(app: &mut App, key: KeyEvent) -> bool {
    let issues_count = if let Some(repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
        if let repo::TabData::Loaded(issues) = &info.forge_issues { issues.len() } else { 0 }
    } else {
        0
    };

    if app.is_bound(Action::DetailMoveDown, key) {
        if issues_count > 0 {
            app.forge_issue_selection = (app.forge_issue_selection + 1).min(issues_count - 1);
        }
        return true;
    }
    if app.is_bound(Action::DetailMoveUp, key) {
        app.forge_issue_selection = app.forge_issue_selection.saturating_sub(1);
        return true;
    }
    if app.is_bound(Action::DetailPageDown, key) {
        if issues_count > 0 {
            app.forge_issue_selection =
                (app.forge_issue_selection + app.config.page_size).min(issues_count - 1);
        }
        return true;
    }
    if app.is_bound(Action::DetailPageUp, key) {
        app.forge_issue_selection = app.forge_issue_selection.saturating_sub(app.config.page_size);
        return true;
    }
    if app.is_bound(Action::DetailHome, key) {
        app.forge_issue_selection = 0;
        return true;
    }
    if app.is_bound(Action::DetailEnd, key) {
        if issues_count > 0 {
            app.forge_issue_selection = issues_count - 1;
        }
        return true;
    }
    if app.is_bound(Action::ForgeCheckout, key) {
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &app.current_detail {
            if let repo::TabData::Loaded(issues) = &info.forge_issues {
                if let Some(issue) = issues.get(app.forge_issue_selection) {
                    let num = issue.number;
                    let path = resolved.clone();
                    app.fetching = true;
                    app.status_message =
                        Some(format!("Resolving and switching branch for issue #{}...", num));
                    let tx = app.tx.clone();
                    std::thread::spawn(move || {
                        match repo::resolve_and_checkout_issue_branch(&path, num) {
                            Ok(msg) => {
                                let _ = tx.send(format!("CHECKOUT_SUCCESS:{}", msg));
                            }
                            Err(e) => {
                                let _ = tx
                                    .send(format!("CHECKOUT_ERROR:Failed to switch branch: {}", e));
                            }
                        }
                    });
                }
            }
        }
        return true;
    }
    if app.is_bound(Action::ForgeOpenBrowser, key) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
            if let repo::TabData::Loaded(issues) = &info.forge_issues {
                if let Some(issue) = issues.get(app.forge_issue_selection) {
                    repo::open_browser(&issue.url);
                    app.status_message = Some(format!("Opened issue #{} in browser", issue.number));
                }
            }
        }
        return true;
    }
    false
}

fn handle_forge_pr_events(app: &mut App, key: KeyEvent) -> bool {
    let prs_count = if let Some(repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
        if let repo::TabData::Loaded(prs) = &info.forge_prs { prs.len() } else { 0 }
    } else {
        0
    };

    if app.is_bound(Action::DetailMoveDown, key) {
        if prs_count > 0 {
            app.forge_pr_selection = (app.forge_pr_selection + 1).min(prs_count - 1);
        }
        return true;
    }
    if app.is_bound(Action::DetailMoveUp, key) {
        app.forge_pr_selection = app.forge_pr_selection.saturating_sub(1);
        return true;
    }
    if app.is_bound(Action::DetailPageDown, key) {
        if prs_count > 0 {
            app.forge_pr_selection =
                (app.forge_pr_selection + app.config.page_size).min(prs_count - 1);
        }
        return true;
    }
    if app.is_bound(Action::DetailPageUp, key) {
        app.forge_pr_selection = app.forge_pr_selection.saturating_sub(app.config.page_size);
        return true;
    }
    if app.is_bound(Action::DetailHome, key) {
        app.forge_pr_selection = 0;
        return true;
    }
    if app.is_bound(Action::DetailEnd, key) {
        if prs_count > 0 {
            app.forge_pr_selection = prs_count - 1;
        }
        return true;
    }
    if app.is_bound(Action::ForgeCheckout, key) {
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &app.current_detail {
            if let repo::TabData::Loaded(prs) = &info.forge_prs {
                if let Some(pr) = prs.get(app.forge_pr_selection) {
                    let num = pr.number;
                    let path = resolved.clone();
                    app.fetching = true;
                    app.status_message = Some(format!("Checking out branch for PR #{}...", num));
                    let tx = app.tx.clone();
                    std::thread::spawn(move || match repo::checkout_pr_branch(&path, num) {
                        Ok(msg) => {
                            let _ = tx.send(format!("CHECKOUT_SUCCESS:{}", msg));
                        }
                        Err(e) => {
                            let _ =
                                tx.send(format!("CHECKOUT_ERROR:Failed to switch branch: {}", e));
                        }
                    });
                }
            }
        }
        return true;
    }
    if app.is_bound(Action::ForgeOpenBrowser, key) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
            if let repo::TabData::Loaded(prs) = &info.forge_prs {
                if let Some(pr) = prs.get(app.forge_pr_selection) {
                    repo::open_browser(&pr.url);
                    app.status_message = Some(format!("Opened PR #{} in browser", pr.number));
                }
            }
        }
        return true;
    }
    false
}

fn handle_reflog_events(app: &mut App, key: KeyEvent) -> bool {
    let entries_count = if let Some(repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
        if let repo::TabData::Loaded(reflog) = &info.reflog { reflog.len() } else { 0 }
    } else {
        0
    };

    if app.is_bound(Action::DetailMoveDown, key) {
        if entries_count > 0 {
            app.reflog_selection = (app.reflog_selection + 1).min(entries_count - 1);
        }
        return true;
    }
    if app.is_bound(Action::DetailMoveUp, key) {
        app.reflog_selection = app.reflog_selection.saturating_sub(1);
        return true;
    }
    if app.is_bound(Action::DetailPageDown, key) {
        if entries_count > 0 {
            app.reflog_selection =
                (app.reflog_selection + app.config.page_size).min(entries_count - 1);
        }
        return true;
    }
    if app.is_bound(Action::DetailPageUp, key) {
        app.reflog_selection = app.reflog_selection.saturating_sub(app.config.page_size);
        return true;
    }
    if app.is_bound(Action::DetailHome, key) {
        app.reflog_selection = 0;
        return true;
    }
    if app.is_bound(Action::DetailEnd, key) {
        if entries_count > 0 {
            app.reflog_selection = entries_count - 1;
        }
        return true;
    }
    if app.is_bound(Action::ReflogCheckout, key) {
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &app.current_detail {
            if let repo::TabData::Loaded(reflog) = &info.reflog {
                if let Some(entry) = reflog.get(app.reflog_selection) {
                    let target_oid = entry.target_oid.clone();
                    let path = resolved.clone();
                    app.fetching = true;
                    app.status_message = Some(format!("Checking out OID {}...", target_oid));
                    let tx = app.tx.clone();
                    std::thread::spawn(move || match repo::checkout_commit(&path, &target_oid) {
                        Ok(_) => {
                            let _ = tx.send(format!(
                                "CHECKOUT_SUCCESS:Checked out commit {}",
                                target_oid
                            ));
                        }
                        Err(e) => {
                            let _ = tx.send(format!("CHECKOUT_ERROR:Failed to checkout: {}", e));
                        }
                    });
                }
            }
        }
        return true;
    }
    false
}

fn handle_submodule_events(app: &mut App, key: KeyEvent) -> bool {
    let subs_count = if let Some(repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
        if let repo::TabData::Loaded(subs) = &info.submodules { subs.len() } else { 0 }
    } else {
        0
    };

    if app.is_bound(Action::DetailMoveDown, key) {
        if subs_count > 0 {
            app.submodule_selection = (app.submodule_selection + 1).min(subs_count - 1);
        }
        return true;
    }
    if app.is_bound(Action::DetailMoveUp, key) {
        app.submodule_selection = app.submodule_selection.saturating_sub(1);
        return true;
    }
    if app.is_bound(Action::DetailPageDown, key) {
        if subs_count > 0 {
            app.submodule_selection =
                (app.submodule_selection + app.config.page_size).min(subs_count - 1);
        }
        return true;
    }
    if app.is_bound(Action::DetailPageUp, key) {
        app.submodule_selection = app.submodule_selection.saturating_sub(app.config.page_size);
        return true;
    }
    if app.is_bound(Action::DetailHome, key) {
        app.submodule_selection = 0;
        return true;
    }
    if app.is_bound(Action::DetailEnd, key) {
        if subs_count > 0 {
            app.submodule_selection = subs_count - 1;
        }
        return true;
    }
    if app.is_bound(Action::SubmodulesAdd, key) {
        app.submodule_add_url.clear();
        app.submodule_add_path.clear();
        app.input_buffer.clear();
        app.mode = Mode::SubmoduleAddUrlInput;
        return true;
    }
    if app.is_bound(Action::SubmodulesDelete, key) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
            if let repo::TabData::Loaded(subs) = &info.submodules {
                if let Some(sub) = subs.get(app.submodule_selection) {
                    app.submodule_delete_target = Some(sub.name.clone());
                    app.mode = Mode::SubmoduleDeleteConfirm;
                }
            }
        }
        return true;
    }
    false
}

fn handle_worktree_events(app: &mut App, key: KeyEvent) -> bool {
    let wts_count = if let Some(repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
        if let repo::TabData::Loaded(wts) = &info.worktrees { wts.len() } else { 0 }
    } else {
        0
    };

    if app.is_bound(Action::DetailMoveDown, key) {
        if wts_count > 0 {
            app.worktree_selection = (app.worktree_selection + 1).min(wts_count - 1);
        }
        return true;
    }
    if app.is_bound(Action::DetailMoveUp, key) {
        app.worktree_selection = app.worktree_selection.saturating_sub(1);
        return true;
    }
    if app.is_bound(Action::DetailPageDown, key) {
        if wts_count > 0 {
            app.worktree_selection =
                (app.worktree_selection + app.config.page_size).min(wts_count - 1);
        }
        return true;
    }
    if app.is_bound(Action::DetailPageUp, key) {
        app.worktree_selection = app.worktree_selection.saturating_sub(app.config.page_size);
        return true;
    }
    if app.is_bound(Action::DetailHome, key) {
        app.worktree_selection = 0;
        return true;
    }
    if app.is_bound(Action::DetailEnd, key) {
        if wts_count > 0 {
            app.worktree_selection = wts_count - 1;
        }
        return true;
    }
    if app.is_bound(Action::WorktreesAdd, key) {
        app.worktree_add_branch.clear();
        app.worktree_add_path.clear();
        app.input_buffer.clear();
        app.mode = Mode::WorktreeAddBranchInput;
        return true;
    }
    if app.is_bound(Action::WorktreesLock, key) {
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
        return true;
    }
    if app.is_bound(Action::WorktreesDelete, key) {
        app.worktree_remove_delete_folder = false;
        app.worktree_remove_force = false;
        app.input_buffer.clear();
        app.mode = Mode::WorktreeRemoveConfirm;
        return true;
    }
    if app.is_bound(Action::WorktreesPrune, key) {
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
        return true;
    }
    if app.is_bound(Action::WorktreesOpen, key) {
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
                        if let Some(pos) = app.config.items.iter().position(|x| x == &wt_path_str) {
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
        return true;
    }
    false
}
