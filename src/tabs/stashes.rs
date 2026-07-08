use crate::app::{App, DetailSection};
use crate::keybindings::Action;
use crossterm::event::KeyEvent;

pub struct StashesTab;

impl StashesTab {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        let detail_focus = app.detail_focus;

        if app.is_bound(Action::StashesDelete, key) {
            if detail_focus == DetailSection::Stashes {
                app.request_stash_delete();
                return true;
            }
        }
        if app.is_bound(Action::StashesApply, key) {
            if detail_focus == DetailSection::Stashes {
                app.request_stash_apply();
                return true;
            }
        }
        if app.is_bound(Action::StashesCreate, key) {
            if detail_focus == DetailSection::Stashes {
                app.start_stash_create();
                return true;
            }
        }

        if app.is_bound(Action::DetailMoveUp, key) {
            match detail_focus {
                DetailSection::Stashes => app.stash_up(),
                DetailSection::StashedFiles => app.stash_file_up(),
                DetailSection::StagingDetails => app.diff.diff_scroll_up(),
                _ => {}
            }
            return true;
        }
        if app.is_bound(Action::DetailMoveDown, key) {
            match detail_focus {
                DetailSection::Stashes => app.stash_down(),
                DetailSection::StashedFiles => app.stash_file_down(),
                DetailSection::StagingDetails => app.diff.diff_scroll_down(),
                _ => {}
            }
            return true;
        }
        if app.is_bound(Action::DetailPageUp, key) {
            match detail_focus {
                DetailSection::Stashes => app.stash_page_up(app.config.page_size),
                DetailSection::StashedFiles => app.stash_file_page_up(app.config.page_size),
                DetailSection::StagingDetails => app.diff.diff_scroll_page_up(app.config.page_size),
                _ => {}
            }
            return true;
        }
        if app.is_bound(Action::DetailPageDown, key) {
            match detail_focus {
                DetailSection::Stashes => app.stash_page_down(app.config.page_size),
                DetailSection::StashedFiles => app.stash_file_page_down(app.config.page_size),
                DetailSection::StagingDetails => {
                    app.diff.diff_scroll_page_down(app.config.page_size)
                }
                _ => {}
            }
            return true;
        }
        if app.is_bound(Action::DetailHome, key) {
            match detail_focus {
                DetailSection::Stashes => app.stash_to_top(),
                DetailSection::StashedFiles => app.stash_file_to_top(),
                DetailSection::StagingDetails => app.diff.diff_scroll_to_top(),
                _ => {}
            }
            return true;
        }
        if app.is_bound(Action::DetailEnd, key) {
            match detail_focus {
                DetailSection::Stashes => app.stash_to_bottom(),
                DetailSection::StashedFiles => app.stash_file_to_bottom(),
                DetailSection::StagingDetails => app.diff.diff_scroll_to_bottom(),
                _ => {}
            }
            return true;
        }

        false
    }
}
