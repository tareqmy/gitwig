use crate::app::App;
use crate::components::Component;
use crate::keybindings::Action;
use crate::queue::InternalEvent;
use crossterm::event::KeyEvent;

pub struct TagsTab;

impl TagsTab {
    /// Routes a key on the Tags tab. The tab's actions are resolved through
    /// `App::is_bound` first so the `[tags]` bindings in `keybindings.toml`
    /// and the command palette both reach them; the list component then
    /// handles plain navigation.
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        if app.is_bound(Action::TagsSearch, key) {
            app.start_tag_search();
            return true;
        }
        let actions = [
            (Action::TagsCheckout, InternalEvent::CheckoutTag),
            (Action::TagsDelete, InternalEvent::RequestDeleteTag),
            (Action::TagsPush, InternalEvent::RequestPushTag),
            (Action::TagsPushAll, InternalEvent::RequestPushAllTags),
            (Action::TagsFetch, InternalEvent::FetchRemoteTags),
        ];
        for (action, event) in actions {
            if app.is_bound(action, key) {
                app.queue.push(event);
                return true;
            }
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
