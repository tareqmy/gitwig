use crate::app::{App, DetailSection};
use crossterm::event::{KeyCode, KeyEvent};

pub mod branches;
pub mod files;
pub mod home;
pub mod logs;
pub mod remotes;
pub mod stashes;
pub mod tags;
pub mod workspace;

pub use branches::BranchesTab;
pub use files::FilesTab;
pub use home::HomeTab;
pub use logs::LogsTab;
pub use remotes::RemotesTab;
pub use stashes::StashesTab;
pub use tags::TagsTab;
pub use workspace::WorkspaceTab;

pub fn route_detail_event(app: &mut App, key: KeyEvent) -> bool {
    let code = key.code;
    match code {
        KeyCode::Esc => {
            if app.inspect_full_diff {
                app.inspect_full_diff = false;
            } else if app.commit_list.search_query.is_some() {
                app.cancel_commit_search();
            } else {
                app.close_detail();
            }
        }
        KeyCode::Char('q') | KeyCode::Char('Q') => app.close_detail(),
        KeyCode::Char('?') => app.open_detail_help(),
        KeyCode::Char('w') => {
            app.cycle_detail_focus(false);
            return true;
        }
        KeyCode::Char('W') => {
            app.cycle_detail_focus(true);
            return true;
        }
        KeyCode::Char('R') => {
            app.resync_detail();
            app.status_message = Some("Refreshed".to_string());
        }
        KeyCode::Tab => {
            app.inspect_full_diff = false;
            app.detail_tab = (app.detail_tab + 1) % 8;
            app.set_default_focus_for_tab();
            if app.config.resync_on_tab_change {
                app.resync_detail();
            }
        }
        KeyCode::BackTab => {
            app.inspect_full_diff = false;
            app.detail_tab = if app.detail_tab == 0 { 7 } else { app.detail_tab - 1 };
            app.set_default_focus_for_tab();
            if app.config.resync_on_tab_change {
                app.resync_detail();
            }
        }
        KeyCode::Char('1') => {
            app.inspect_full_diff = false;
            app.detail_tab = 0;
            app.detail_focus = DetailSection::Commits;
            if app.config.resync_on_tab_change {
                app.resync_detail();
            }
        }
        KeyCode::Char('2') => {
            app.inspect_full_diff = false;
            app.detail_tab = 1;
            app.detail_focus = DetailSection::Files;
            if app.config.resync_on_tab_change {
                app.resync_detail();
            }
        }
        KeyCode::Char('3') => {
            app.inspect_full_diff = false;
            app.detail_tab = 2;
            if app.config.resync_on_tab_change {
                app.resync_detail();
            }
        }
        KeyCode::Char('4') => {
            app.inspect_full_diff = false;
            app.detail_tab = 3;
            app.detail_focus = DetailSection::LocalBranches;
            if app.config.resync_on_tab_change {
                app.resync_detail();
            }
        }
        KeyCode::Char('5') => {
            app.inspect_full_diff = false;
            app.detail_tab = 4;
            app.detail_focus = DetailSection::LocalTags;
            let attempted =
                if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
                    info.remote_tags_attempted
                } else {
                    false
                };
            if !attempted {
                app.fetch_remote_tags(true);
            }
            if app.config.resync_on_tab_change {
                app.resync_detail();
            }
        }
        KeyCode::Char('6') => {
            app.inspect_full_diff = false;
            app.detail_tab = 5;
            app.detail_focus = DetailSection::Remotes;
            let remote_name =
                if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
                    info.remotes
                        .get(app.branch_list.remote_selection)
                        .or_else(|| info.remotes.first())
                        .map(|r| r.name.clone())
                } else {
                    None
                };
            if let Some(name) = remote_name {
                app.fetch_remote(&name);
            }
            if app.config.resync_on_tab_change {
                app.resync_detail();
            }
        }
        KeyCode::Char('7') => {
            app.inspect_full_diff = false;
            app.detail_tab = 6;
            app.detail_focus = DetailSection::Stashes;
            if app.config.resync_on_tab_change {
                app.resync_detail();
            }
        }
        KeyCode::Char('8') => {
            app.inspect_full_diff = false;
            app.detail_tab = 7;
            app.detail_focus = DetailSection::Commits;
            if app.config.resync_on_tab_change {
                app.resync_detail();
            }
        }
        _ => match app.detail_tab {
            0 => return WorkspaceTab::handle_event(app, key),
            1 => return FilesTab::handle_event(app, key),
            3 => return BranchesTab::handle_event(app, key),
            4 => return TagsTab::handle_event(app, key),
            5 => return RemotesTab::handle_event(app, key),
            6 => return StashesTab::handle_event(app, key),
            _ => {}
        },
    }
    false
}
