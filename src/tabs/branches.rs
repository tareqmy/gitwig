use crate::app::{App, DetailSection};
use crate::components::Component;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub struct BranchesTab;

impl BranchesTab {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        let detail_focus = app.detail_focus;

        if app.is_bound(Action::BranchesSearch, key) {
            app.start_branch_search();
            return true;
        }
        if app.is_bound(Action::BranchesCreate, key) {
            app.start_branch_create();
            return true;
        }
        if app.is_bound(Action::BranchesDelete, key) {
            app.request_branch_delete();
            return true;
        }
        if app.is_bound(Action::BranchesMerge, key) {
            app.request_branch_merge();
            return true;
        }
        if app.is_bound(Action::BranchesRebase, key) {
            app.request_branch_rebase();
            return true;
        }
        if app.is_bound(Action::BranchesInteractiveRebase, key) {
            app.request_branch_interactive_rebase();
            return true;
        }
        if app.is_bound(Action::BranchesPush, key) && detail_focus == DetailSection::LocalBranches {
            app.request_branch_push();
            return true;
        }
        if app.is_bound(Action::BranchesPull, key) && detail_focus == DetailSection::LocalBranches {
            app.pull_selected_branch();
            return true;
        }
        if app.is_bound(Action::BranchesCheckout, key) {
            app.request_branch_checkout();
            return true;
        }

        if app.is_bound(Action::CycleFocusBackward, key)
            || key.code == crossterm::event::KeyCode::Left
        {
            app.move_focus_left();
            return true;
        }
        if app.is_bound(Action::CycleFocusForward, key)
            || key.code == crossterm::event::KeyCode::Right
        {
            app.move_focus_right();
            return true;
        }

        if app.is_bound(Action::DetailMoveUp, key) {
            if detail_focus == DetailSection::LocalBranches {
                app.local_branch_up();
            } else {
                app.remote_branch_up();
            }
            return true;
        }
        if app.is_bound(Action::DetailMoveDown, key) {
            if detail_focus == DetailSection::LocalBranches {
                app.local_branch_down();
            } else {
                app.remote_branch_down();
            }
            return true;
        }
        if app.is_bound(Action::DetailPageUp, key) {
            if detail_focus == DetailSection::LocalBranches {
                app.local_branch_page_up(app.get_current_page_size());
            } else {
                app.remote_branch_page_up(app.get_current_page_size());
            }
            return true;
        }
        if app.is_bound(Action::DetailPageDown, key) {
            if detail_focus == DetailSection::LocalBranches {
                app.local_branch_page_down(app.get_current_page_size());
            } else {
                app.remote_branch_page_down(app.get_current_page_size());
            }
            return true;
        }
        if app.is_bound(Action::DetailHome, key) {
            if detail_focus == DetailSection::LocalBranches {
                app.local_branch_to_top();
            } else {
                app.remote_branch_to_top();
            }
            return true;
        }
        if app.is_bound(Action::DetailEnd, key) {
            if detail_focus == DetailSection::LocalBranches {
                app.local_branch_to_bottom();
            } else {
                app.remote_branch_to_bottom();
            }
            return true;
        }

        let ev = crossterm::event::Event::Key(key);
        if app
            .branch_list
            .event(&ev)
            .unwrap_or(crate::components::EventState::NotConsumed)
            .is_consumed()
        {
            return true;
        }
        false
    }
}
