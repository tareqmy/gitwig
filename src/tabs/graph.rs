use crate::app::{App, DetailSection, Mode};
use crossterm::event::{KeyCode, KeyEvent};

pub struct GraphTab;

impl GraphTab {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                app.graph_select_up();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.graph_select_down();
                true
            }
            KeyCode::PageUp => {
                let page = app.get_current_page_size();
                app.graph_select_page_up(page);
                true
            }
            KeyCode::PageDown => {
                let page = app.get_current_page_size();
                app.graph_select_page_down(page);
                true
            }
            KeyCode::Home => {
                app.graph_select_to_top();
                true
            }
            KeyCode::End => {
                app.graph_select_to_bottom();
                true
            }
            KeyCode::Enter | KeyCode::Right => {
                if let Some(commit) = app.get_selected_commit() {
                    let _oid = commit.oid.clone();
                    app.mode = Mode::Inspect;
                    app.detail_focus = DetailSection::Files;
                    app.status_list.file_selection = 0;
                    app.diff.diff_scroll = 0;
                    app.refresh_file_diff();
                    true
                } else {
                    false
                }
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.yank_selected_commit_hash();
                true
            }
            _ => false,
        }
    }
}
