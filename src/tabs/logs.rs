use crate::app::{App, DetailSection, Mode};
use crossterm::event::{KeyCode, KeyEvent};

pub struct LogsTab;
impl LogsTab {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        let code = key.code;
        match code {
            KeyCode::Up => app.detail_commit_up(),
            KeyCode::Down => app.detail_commit_down(),
            KeyCode::PageUp => app.detail_commit_page_up(app.config.page_size),
            KeyCode::PageDown => app.detail_commit_page_down(app.config.page_size),
            KeyCode::Home => app.detail_commit_to_top(),
            KeyCode::End => app.detail_commit_to_bottom(),
            KeyCode::Char('f') => {
                app.search_column_selection = 0;
                app.mode = Mode::SearchColumnPicker;
            }
            KeyCode::Enter => {
                app.mode = Mode::Inspect;
                if app.is_uncommitted_selected() {
                    app.detail_focus = DetailSection::Staged;
                    app.last_staging_focus = DetailSection::Staged;
                    app.status_list.staging_file_selection = 0;
                } else {
                    app.detail_focus = DetailSection::Staged;
                    app.last_staging_focus = DetailSection::Staged;
                    app.status_list.file_selection = 0;
                }
                app.diff.diff_scroll = 0;
                app.refresh_file_diff();
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.in_logs_ui = false;
                app.commit_list.search_query = None;
                app.mode = Mode::Detail;
            }
            _ => {}
        }
        false
    }
}
