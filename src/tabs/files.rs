use crate::app::{App, DetailSection};
use crate::components::Component;
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub struct FilesTab;

impl FilesTab {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        let detail_focus = app.detail_focus;
        let ev = crossterm::event::Event::Key(key);

        if app.is_bound(Action::FilesBlame, key) {
            app.file_tree.show_blame = !app.file_tree.show_blame;
            app.refresh_blame_if_shown();
            return true;
        }
        if app.is_bound(Action::FilesLineNumbers, key) {
            app.file_tree.show_line_numbers = !app.file_tree.show_line_numbers;
            return true;
        }
        if app.is_bound(Action::FilesEditor, key) {
            if let Some(item) = app.file_tree.visible_files.get(app.file_tree.file_list_selection) {
                if !item.is_dir {
                    app.pending_editor_file = Some(item.full_path.clone());
                    return true;
                }
            }
        }

        if detail_focus == DetailSection::Files {
            if app.is_bound(Action::FilesHistory, key) {
                app.open_file_history();
                return true;
            }
            if app.is_bound(Action::FilesSearch, key) {
                app.start_file_search();
                return true;
            }
            if app.is_bound(Action::FilesExpand, key) {
                app.expand_selected_folder();
                return true;
            }
            if app.is_bound(Action::FilesCollapse, key) {
                app.collapse_selected_folder();
                return true;
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
            if app.is_bound(Action::FilesFullScreen, key) {
                if !app.inspect_full_diff {
                    app.inspect_full_diff = true;
                    return true;
                }
            }
            if (key.code == crossterm::event::KeyCode::Left
                || app.is_bound(Action::CloseDetail, key))
                && app.inspect_full_diff
            {
                app.inspect_full_diff = false;
                return true;
            }

            if app.is_bound(Action::DetailMoveUp, key) {
                app.file_tree.queue.push(crate::queue::InternalEvent::FileContentUp);
                return true;
            }
            if app.is_bound(Action::DetailMoveDown, key) {
                app.file_tree.queue.push(crate::queue::InternalEvent::FileContentDown);
                return true;
            }
            if app.is_bound(Action::DetailPageUp, key) {
                app.file_tree.queue.push(crate::queue::InternalEvent::FileContentPageUp);
                return true;
            }
            if app.is_bound(Action::DetailPageDown, key) {
                app.file_tree.queue.push(crate::queue::InternalEvent::FileContentPageDown);
                return true;
            }
            if app.is_bound(Action::DetailHome, key) {
                app.file_tree.queue.push(crate::queue::InternalEvent::FileContentTop);
                return true;
            }
            if app.is_bound(Action::DetailEnd, key) {
                app.file_tree.queue.push(crate::queue::InternalEvent::FileContentBottom);
                return true;
            }
        }
        false
    }
}
