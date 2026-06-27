use crate::app::{App, DetailSection};
use crossterm::event::{KeyCode, KeyEvent};

pub struct BranchesTab;

impl BranchesTab {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        let code = key.code;
        let detail_focus = app.detail_focus;
        match code {
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
        }
        false
    }
}
