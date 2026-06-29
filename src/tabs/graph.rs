use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent};

pub struct GraphTab;

impl GraphTab {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                app.graph_scroll_up();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.graph_scroll_down();
                true
            }
            KeyCode::PageUp => {
                let page = app.get_current_page_size();
                app.graph_scroll_page_up(page);
                true
            }
            KeyCode::PageDown => {
                let page = app.get_current_page_size();
                app.graph_scroll_page_down(page);
                true
            }
            KeyCode::Home => {
                app.graph_scroll_to_top();
                true
            }
            KeyCode::End => {
                app.graph_scroll_to_bottom();
                true
            }
            _ => false,
        }
    }
}
