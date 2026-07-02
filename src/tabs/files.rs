use crate::app::{App, DetailSection};
use crate::components::Component;
use crossterm::event::{KeyCode, KeyEvent};

pub struct FilesTab;

impl FilesTab {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        let code = key.code;
        let detail_focus = app.detail_focus;
        let ev = crossterm::event::Event::Key(key);

        if code == KeyCode::Char('e') || code == KeyCode::Char('o') {
            if let Some(item) = app.file_tree.visible_files.get(app.file_tree.file_list_selection) {
                if !item.is_dir {
                    app.pending_editor_file = Some(item.full_path.clone());
                    return true;
                }
            }
        }

        if detail_focus == DetailSection::Files {
            if code == KeyCode::Char('H') {
                app.open_file_history();
                return true;
            }
            // 'f' launches FZF file picker or native fallback
            if code == KeyCode::Char('f') {
                if app.config.fzf.enabled && app.is_fzf_installed() {
                    app.pending_files_fzf = true;
                } else {
                    app.start_file_search();
                }
                return true;
            }
            // '>'/'.' expand folder, '<'/',' collapse folder
            match code {
                KeyCode::Char('>') | KeyCode::Char('.') => {
                    app.expand_selected_folder();
                    return true;
                }
                KeyCode::Char('<') | KeyCode::Char(',') => {
                    app.collapse_selected_folder();
                    return true;
                }
                _ => {}
            }
            if app
                .file_tree
                .event(&ev)
                .unwrap_or(crate::components::EventState::NotConsumed)
                .is_consumed()
            {
                return true;
            }
        } else if detail_focus == DetailSection::FileContent {
            match code {
                KeyCode::Right => {
                    app.inspect_full_diff = true;
                    return true;
                }
                KeyCode::Left if app.inspect_full_diff => {
                    app.inspect_full_diff = false;
                    return true;
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    app.file_tree.queue.push(crate::queue::InternalEvent::FileContentUp)
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    app.file_tree.queue.push(crate::queue::InternalEvent::FileContentDown)
                }
                KeyCode::PageUp => {
                    app.file_tree.queue.push(crate::queue::InternalEvent::FileContentPageUp)
                }
                KeyCode::PageDown => {
                    app.file_tree.queue.push(crate::queue::InternalEvent::FileContentPageDown)
                }
                KeyCode::Home => {
                    app.file_tree.queue.push(crate::queue::InternalEvent::FileContentTop)
                }
                KeyCode::End => {
                    app.file_tree.queue.push(crate::queue::InternalEvent::FileContentBottom)
                }
                _ => {}
            }
        }
        false
    }
}
