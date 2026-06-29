use crate::app::{App, DetailSection, Mode};
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
        app.detail_tab = (app.detail_tab + 1) % 8;
        app.set_default_focus_for_tab();
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::CycleTabBackward, key) {
        app.inspect_full_diff = false;
        app.detail_tab = if app.detail_tab == 0 { 7 } else { app.detail_tab - 1 };
        app.set_default_focus_for_tab();
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab1, key) {
        app.inspect_full_diff = false;
        app.detail_tab = 0;
        app.detail_focus = DetailSection::Commits;
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab2, key) {
        app.inspect_full_diff = false;
        app.detail_tab = 1;
        app.detail_focus = DetailSection::Files;
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab3, key) {
        app.inspect_full_diff = false;
        app.detail_tab = 2;
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab4, key) {
        app.inspect_full_diff = false;
        app.detail_tab = 3;
        app.detail_focus = DetailSection::LocalBranches;
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab5, key) {
        app.inspect_full_diff = false;
        app.detail_tab = 4;
        app.detail_focus = DetailSection::LocalTags;
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab6, key) {
        app.inspect_full_diff = false;
        app.detail_tab = 5;
        app.detail_focus = DetailSection::Remotes;
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab7, key) {
        app.inspect_full_diff = false;
        app.detail_tab = 6;
        app.detail_focus = DetailSection::Stashes;
        if app.get_current_resync_on_tab_change() {
            app.resync_detail();
        }
        return true;
    }

    if app.is_bound(Action::GoToTab8, key) {
        app.inspect_full_diff = false;
        app.detail_tab = 7;
        app.detail_focus = DetailSection::Commits;
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
        7 => {
            if code == KeyCode::Char('s') || code == KeyCode::Char('S') {
                app.repo_settings_selected_index = 0;
                app.repo_settings_editing = false;
                app.repo_settings_input = String::new();
                app.mode = Mode::RepoSettings;
                return true;
            }
        }
        _ => {}
    }
    false
}
