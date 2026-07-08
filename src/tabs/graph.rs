use crate::app::{App, DetailSection, Mode};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub struct GraphTab;

impl GraphTab {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        if app.is_bound(Action::DetailMoveUp, key) {
            app.graph_select_up();
            return true;
        }
        if app.is_bound(Action::DetailMoveDown, key) {
            app.graph_select_down();
            return true;
        }
        if app.is_bound(Action::DetailPageUp, key) {
            let page = app.get_current_page_size();
            app.graph_select_page_up(page);
            return true;
        }
        if app.is_bound(Action::DetailPageDown, key) {
            let page = app.get_current_page_size();
            app.graph_select_page_down(page);
            return true;
        }
        if app.is_bound(Action::DetailHome, key) {
            app.graph_select_to_top();
            return true;
        }
        if app.is_bound(Action::DetailEnd, key) {
            app.graph_select_to_bottom();
            return true;
        }
        if app.is_bound(Action::HomeOpenDetail, key) {
            if let Some(commit) = app.get_selected_commit() {
                let _oid = commit.oid.clone();
                app.mode = Mode::Inspect;
                app.detail_focus = DetailSection::Files;
                app.status_list.file_selection = 0;
                app.diff.diff_scroll = 0;
                app.refresh_file_diff();
                return true;
            }
        }
        if app.is_bound(Action::WorkspaceYankHash, key) {
            app.yank_selected_commit_hash();
            return true;
        }
        false
    }
}
