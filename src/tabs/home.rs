use crate::app::{App, Mode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct HomeTab;

impl HomeTab {
    pub fn handle_event(app: &mut App, key: KeyEvent, visible_count: usize) -> bool {
        let code = key.code;
        match &app.mode {
            Mode::Normal => {
                use crate::keybindings::Action;
                let rows = app.get_home_rows();
                if let Some(crate::app::HomeRow::GroupHeader { name, collapsed, .. }) =
                    rows.get(app.selected_index)
                {
                    let name = name.clone();
                    let collapsed = *collapsed;
                    match code {
                        KeyCode::Left => {
                            if !collapsed {
                                app.collapsed_groups.insert(name);
                                app.clamp_selection();
                            }
                            return true;
                        }
                        KeyCode::Right => {
                            if collapsed {
                                app.collapsed_groups.remove(&name);
                                app.clamp_selection();
                            }
                            return true;
                        }
                        KeyCode::Char(' ') | KeyCode::Enter => {
                            if collapsed {
                                app.collapsed_groups.remove(&name);
                            } else {
                                app.collapsed_groups.insert(name);
                            }
                            app.clamp_selection();
                            return true;
                        }
                        _ => {}
                    }
                }

                if app.repo_search_query.is_some() && code == KeyCode::Esc {
                    app.repo_search_query = None;
                    app.selected_index = 0;
                    app.scroll_top = 0;
                } else if app.is_bound(Action::Close, key) {
                    return false;
                } else if app.is_bound(Action::HomeMoveDown, key) {
                    app.move_down(visible_count);
                } else if app.is_bound(Action::HomeMoveUp, key) {
                    app.move_up();
                } else if app.is_bound(Action::HomePageDown, key) {
                    app.page_down(app.get_current_page_size());
                } else if app.is_bound(Action::HomePageUp, key) {
                    app.page_up(app.get_current_page_size());
                } else if app.is_bound(Action::HomeHome, key) {
                    app.move_to_top();
                } else if app.is_bound(Action::HomeEnd, key) {
                    app.move_to_bottom(visible_count);
                } else if app.is_bound(Action::HomeAddRepo, key) {
                    app.start_add();
                } else if app.is_bound(Action::HomeBulkAdd, key) {
                    app.start_bulk_add();
                } else if app.is_bound(Action::HomeEditRepo, key) {
                    app.start_edit();
                } else if app.is_bound(Action::HomeDeleteRepo, key) {
                    app.request_delete();
                } else if app.is_bound(Action::HomeOpenDebugLogs, key) {
                    crate::debug_log::info("Opening debug logs");
                    app.mode = Mode::DebugLogs;
                    app.debug_log_scroll = 0;
                } else if app.is_bound(Action::HomeEditLabels, key) {
                    app.start_edit_labels();
                } else if app.is_bound(Action::Help, key) {
                    app.open_help();
                } else if app.is_bound(Action::HomeAbout, key) {
                    app.open_about();
                } else if app.is_bound(Action::HomeSymbolsHelp, key) {
                    app.mode = Mode::Legend;
                } else if app.is_bound(Action::HomeToggleCompactView, key) {
                    app.config.compact_view = !app.config.compact_view;
                    let _ = crate::config::save_config(&app.config, &app.config_path);
                } else if app.is_bound(Action::HomeRefresh, key) {
                    app.refresh_selected_status();
                } else if app.is_bound(Action::HomeCycleSort, key) {
                    app.cycle_sort_order();
                } else if app.is_bound(Action::HomeToggleSortReverse, key) {
                    app.toggle_sort_reverse();
                } else if app.is_bound(Action::HomeTogglePin, key) {
                    app.toggle_pin_selected();
                } else if app.is_bound(Action::HomeOpenSettings, key) {
                    app.mode = Mode::Settings;
                    app.settings_selected_index = 0;
                    app.settings_editing = false;
                    app.settings_focus_sidebar = true;
                } else if app.is_bound(Action::HomeCheckUpdate, key) {
                    app.trigger_update_check();
                } else if app.is_bound(Action::HomeImportRepo, key) {
                    crate::debug_log::info("Starting repository import");
                    app.mode = Mode::ImportUrlInput;
                    app.input_buffer.clear();
                    app.import_url.clear();
                    app.import_dest.clear();
                    app.import_name.clear();
                } else if app.is_bound(Action::HomeOpenGitApp, key) {
                    app.pending_git_app = true;
                } else if code == KeyCode::Char('*') {
                    app.toggle_star_selected();
                } else if code == KeyCode::Char('/') {
                    app.input_buffer.clear();
                    app.repo_jump_selection = 0;
                    app.mode = Mode::RepoJump;
                } else if app.is_bound(Action::HomeSearchRepo, key) {
                    app.input_buffer.clear();
                    if let Some(ref q) = app.repo_search_query {
                        app.input_buffer.push_str(q);
                    }
                    app.mode = Mode::RepoSearchInput;
                } else if app.is_bound(Action::HomeOpenDetail, key) {
                    app.open_detail();
                } else if code == KeyCode::Char('y') || code == KeyCode::Char('Y') {
                    app.yank_selected_repo_path();
                } else if code == KeyCode::Char('t') || code == KeyCode::Char('T') {
                    app.pending_terminal = true;
                } else if code == KeyCode::Char('F') {
                    app.bulk_fetch_all();
                } else if code == KeyCode::Char(' ') {
                    if let Some(path) = app.get_selected_item() {
                        let path_str = path.clone();
                        if app.multi_selected.contains(&path_str) {
                            app.multi_selected.remove(&path_str);
                        } else {
                            app.multi_selected.insert(path_str);
                        }
                    }
                }
            }
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
            Mode::LabelInput => match code {
                KeyCode::Esc => app.cancel_input(),
                KeyCode::Enter => app.commit_edit_labels(),
                KeyCode::Backspace => app.input_backspace(),
                KeyCode::Char(c) => app.input_char(c),
                _ => {}
            },
            Mode::ConfirmDelete => match code {
                KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete(),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Enter => {
                    app.close_dialog()
                }
                _ => {}
            },
            _ => {}
        }
        true
    }
}
