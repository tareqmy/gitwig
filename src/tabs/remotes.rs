use crate::app::{App, Mode};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub struct RemotesTab;

impl RemotesTab {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        if app.is_bound(Action::RemotesAdd, key) {
            app.start_remote_add();
            return true;
        }
        if app.is_bound(Action::RemotesDelete, key) {
            app.request_remote_delete();
            return true;
        }
        if app.is_bound(Action::RemotesFetch, key) {
            // Open picker if >1 remote, else fetch directly
            let remote_action =
                if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
                    if info.remotes.len() > 1 {
                        Some(None)
                    } else {
                        info.remotes.first().map(|r| Some(r.name.clone()))
                    }
                } else {
                    None
                };
            match remote_action {
                Some(Some(name)) => app.fetch_remote(&name),
                Some(None) => {
                    app.remote_picker_action = Some(crate::app::RemotePickerAction::FetchRemote);
                    app.remote_picker_selection = app.branch_list.remote_selection;
                    app.mode = Mode::RemotePicker;
                }
                None => {}
            }
            return true;
        }

        if app.is_bound(Action::DetailMoveUp, key) {
            app.remote_up();
            return true;
        }
        if app.is_bound(Action::DetailMoveDown, key) {
            app.remote_down();
            return true;
        }
        if app.is_bound(Action::DetailPageUp, key) {
            app.remote_page_up(app.config.page_size);
            return true;
        }
        if app.is_bound(Action::DetailPageDown, key) {
            app.remote_page_down(app.config.page_size);
            return true;
        }
        if app.is_bound(Action::DetailHome, key) {
            app.remote_to_top();
            return true;
        }
        if app.is_bound(Action::DetailEnd, key) {
            app.remote_to_bottom();
            return true;
        }

        false
    }
}
