use crate::app::{App, DetailSection};
use crossterm::event::{KeyCode, KeyEvent};

pub struct StashesTab;

impl StashesTab {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        let code = key.code;
        let detail_focus = app.detail_focus;
        match code {
            KeyCode::Char('D') => {
                if detail_focus == DetailSection::Stashes {
                    app.request_stash_delete();
                    return true;
                }
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                if detail_focus == DetailSection::Stashes {
                    app.request_stash_apply();
                    return true;
                }
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                if detail_focus == DetailSection::Stashes {
                    app.start_stash_create();
                    return true;
                }
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => match detail_focus {
                DetailSection::Stashes => app.stash_up(),
                DetailSection::StashedFiles => app.stash_file_up(),
                DetailSection::StagingDetails => app.diff.diff_scroll_up(),
                _ => {}
            },
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => match detail_focus {
                DetailSection::Stashes => app.stash_down(),
                DetailSection::StashedFiles => app.stash_file_down(),
                DetailSection::StagingDetails => app.diff.diff_scroll_down(),
                _ => {}
            },
            KeyCode::PageUp => match detail_focus {
                DetailSection::Stashes => app.stash_page_up(app.config.page_size),
                DetailSection::StashedFiles => app.stash_file_page_up(app.config.page_size),
                DetailSection::StagingDetails => app.diff.diff_scroll_page_up(app.config.page_size),
                _ => {}
            },
            KeyCode::PageDown => match detail_focus {
                DetailSection::Stashes => app.stash_page_down(app.config.page_size),
                DetailSection::StashedFiles => app.stash_file_page_down(app.config.page_size),
                DetailSection::StagingDetails => {
                    app.diff.diff_scroll_page_down(app.config.page_size)
                }
                _ => {}
            },
            KeyCode::Home => match detail_focus {
                DetailSection::Stashes => app.stash_to_top(),
                DetailSection::StashedFiles => app.stash_file_to_top(),
                DetailSection::StagingDetails => app.diff.diff_scroll_to_top(),
                _ => {}
            },
            KeyCode::End => match detail_focus {
                DetailSection::Stashes => app.stash_to_bottom(),
                DetailSection::StashedFiles => app.stash_file_to_bottom(),
                DetailSection::StagingDetails => app.diff.diff_scroll_to_bottom(),
                _ => {}
            },
            _ => {}
        }
        false
    }
}
