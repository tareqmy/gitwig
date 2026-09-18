use crate::app::App;
use crate::components::Component;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub struct TagsTab;

impl TagsTab {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        if app.is_bound(Action::TagsSearch, key) {
            app.start_tag_search();
            return true;
        }
        if app.is_bound(Action::TagsCheckout, key) {
            app.request_tag_checkout();
            return true;
        }
        if app.is_bound(Action::TagsDelete, key) {
            app.request_tag_delete();
            return true;
        }
        if app.is_bound(Action::TagsPushAll, key) {
            app.request_tag_push_all();
            return true;
        }
        if app.is_bound(Action::TagsPush, key) {
            app.request_tag_push();
            return true;
        }
        if app.is_bound(Action::TagsFetch, key) {
            app.fetch_remote_tags(true);
            return true;
        }

        let ev = crossterm::event::Event::Key(key);
        if app
            .tag_list
            .event(&ev, &app.keybindings)
            .unwrap_or(crate::components::EventState::NotConsumed)
            .is_consumed()
        {
            return true;
        }
        false
    }
}
