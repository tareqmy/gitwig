use crate::app::{App, Mode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct HomeTab;

impl HomeTab {
    pub fn handle_event(app: &mut App, key: KeyEvent, visible_count: usize) -> bool {
        let code = key.code;
        match &app.mode {
            Mode::Normal => match code {
                KeyCode::Esc if app.repo_search_query.is_some() => {
                    app.repo_search_query = None;
                    app.selected_index = 0;
                    app.scroll_top = 0;
                }
                KeyCode::Char('q') | KeyCode::Esc => return false,
                KeyCode::Down => app.move_down(visible_count),
                KeyCode::Up => app.move_up(),
                KeyCode::PageDown => app.page_down(app.config.page_size),
                KeyCode::PageUp => app.page_up(app.config.page_size),
                KeyCode::Home => app.move_to_top(),
                KeyCode::End => app.move_to_bottom(visible_count),
                KeyCode::Char('a') => app.start_add(),
                KeyCode::Char('A') => {
                    app.start_bulk_add();
                }
                KeyCode::Char('e') => app.start_edit(),
                KeyCode::Char('d') => app.request_delete(),
                KeyCode::Char('?') => app.open_help(),
                KeyCode::Char('v') | KeyCode::Char('V') => app.open_about(),
                KeyCode::Char('R') => app.refresh_selected_status(),
                KeyCode::Char('o') => app.cycle_sort_order(),
                KeyCode::Char('O') => app.toggle_sort_reverse(),
                KeyCode::Char('p') => app.toggle_pin_selected(),
                KeyCode::Char('s') => {
                    app.mode = Mode::Settings;
                    app.settings_selected_index = 0;
                    app.settings_editing = false;
                    app.settings_focus_sidebar = true;
                }
                KeyCode::Char('D') | KeyCode::Char('l') | KeyCode::Char('L') => {
                    crate::debug_log::info("Opening debug logs");
                    app.mode = Mode::DebugLogs;
                    app.debug_log_scroll = 0;
                }
                KeyCode::Char('i') => {
                    crate::debug_log::info("Starting repository import");
                    app.mode = Mode::ImportUrlInput;
                    app.input_buffer.clear();
                    app.import_url.clear();
                    app.import_dest.clear();
                    app.import_name.clear();
                }
                KeyCode::Char('g') => {
                    app.pending_git_app = true;
                }
                KeyCode::Char('f') => {
                    app.input_buffer.clear();
                    if let Some(ref q) = app.repo_search_query {
                        app.input_buffer.push_str(q);
                    }
                    app.mode = Mode::RepoSearchInput;
                }
                KeyCode::Enter | KeyCode::Right => app.open_detail(),
                _ => {}
            },
            Mode::RepoSearchInput => match code {
                KeyCode::Esc => {
                    app.repo_search_query = None;
                    app.selected_index = 0;
                    app.scroll_top = 0;
                    app.mode = Mode::Normal;
                }
                KeyCode::Enter => {
                    app.mode = Mode::Normal;
                }
                KeyCode::Backspace => {
                    app.input_backspace();
                    let query = app.input_buffer.clone();
                    if query.is_empty() {
                        app.repo_search_query = None;
                    } else {
                        app.repo_search_query = Some(query);
                    }
                    app.clamp_selection();
                    app.clamp_scroll(visible_count);
                }
                KeyCode::Char(c) => {
                    app.input_char(c);
                    let query = app.input_buffer.clone();
                    app.repo_search_query = Some(query);
                    app.clamp_selection();
                    app.clamp_scroll(visible_count);
                }
                _ => {}
            },
            Mode::Adding => match code {
                KeyCode::Esc => app.cancel_input(),
                KeyCode::Enter => app.commit_add(),
                KeyCode::Backspace => app.input_backspace(),
                KeyCode::Char(c) => app.input_char(c),
                _ => {}
            },
            Mode::BulkAddInput => match code {
                KeyCode::Esc => app.cancel_input(),
                KeyCode::Enter => app.commit_bulk_add(),
                KeyCode::Backspace => app.input_backspace(),
                KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if app.config.fzf.enabled {
                        app.start_bulk_add();
                    } else {
                        app.error_message =
                            Some("FZF is disabled in settings. Enable it first.".to_string());
                    }
                }
                KeyCode::Tab => {
                    if app.config.fzf.enabled {
                        app.start_bulk_add();
                    } else {
                        app.error_message =
                            Some("FZF is disabled in settings. Enable it first.".to_string());
                    }
                }
                KeyCode::Char(c) => app.input_char(c),
                _ => {}
            },
            Mode::Editing => match code {
                KeyCode::Esc => app.cancel_input(),
                KeyCode::Enter => app.commit_edit(),
                KeyCode::Backspace => app.input_backspace(),
                KeyCode::Char(c) => app.input_char(c),
                _ => {}
            },
            Mode::ConfirmDelete => match code {
                KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete(),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => app.close_dialog(),
                _ => {}
            },
            _ => {}
        }
        true
    }
}
