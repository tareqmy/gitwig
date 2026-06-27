use crate::app::{App, Mode};
use crossterm::event::{KeyCode, KeyEvent};

pub struct LogSearchPopup;

impl LogSearchPopup {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        let code = key.code;
        match app.mode {
            Mode::LogsSearchInput => match code {
                KeyCode::Esc => {
                    app.commit_list.search_query = None;
                    app.mode = Mode::Logs;
                }
                KeyCode::Enter => {
                    let query = app.input_buffer.clone();
                    if query.trim().is_empty() {
                        app.commit_list.search_query = None;
                    } else {
                        app.commit_list.search_query = Some(query);
                    }
                    app.mode = Mode::Logs;
                }
                KeyCode::Backspace => {
                    app.input_backspace();
                }
                KeyCode::Char(c) => {
                    app.input_char(c);
                }
                _ => {}
            },
            Mode::CommitSearchInput => match code {
                KeyCode::Esc => app.cancel_commit_search(),
                KeyCode::Enter => app.mode = Mode::Detail,
                KeyCode::Backspace => {
                    app.input_backspace();
                    app.commit_search_input_change();
                }
                KeyCode::Char(c) => {
                    app.input_char(c);
                    app.commit_search_input_change();
                }
                _ => {}
            },
            _ => {}
        }
        true
    }
}
