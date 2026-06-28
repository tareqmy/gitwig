use crate::app::{App, Mode};
use crossterm::event::{KeyCode, KeyEvent};

pub struct RemotesTab;

impl RemotesTab {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        let code = key.code;
        match code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => app.remote_up(),
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => app.remote_down(),
            KeyCode::PageUp => app.remote_page_up(app.config.page_size),
            KeyCode::PageDown => app.remote_page_down(app.config.page_size),
            KeyCode::Home => app.remote_to_top(),
            KeyCode::End => app.remote_to_bottom(),
            KeyCode::Char('f') | KeyCode::Char('F') => {
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
                        app.remote_picker_action =
                            Some(crate::app::RemotePickerAction::FetchRemote);
                        app.remote_picker_selection = app.branch_list.remote_selection;
                        app.mode = Mode::RemotePicker;
                    }
                    None => {}
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => app.start_remote_add(),
            KeyCode::Char('d') | KeyCode::Char('D') => app.request_remote_delete(),
            _ => {}
        }
        false
    }
}
