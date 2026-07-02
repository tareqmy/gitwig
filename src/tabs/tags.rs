use crate::app::App;
use crate::components::Component;
use crossterm::event::{KeyCode, KeyEvent};

pub struct TagsTab;

impl TagsTab {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        let code = key.code;
        match code {
            KeyCode::Char('/') | KeyCode::Char('f') => {
                app.start_tag_search();
                return true;
            }
            _ => {}
        }
        let ev = crossterm::event::Event::Key(key);
        if app
            .tag_list
            .event(&ev)
            .unwrap_or(crate::components::EventState::NotConsumed)
            .is_consumed()
        {
            return true;
        }
        false
    }
}
