//! App cursor, focus selection, active panel, tab switching, and navigation logic.

use super::*;

mod detail;
mod home;
mod scrolling;
mod settings_input;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HomeRow {
    GroupHeader { name: String, count: usize, collapsed: bool },
    Repo { actual_index: usize, path: String, primary_label: String },
}

impl App {
    /// Records an error for the error popup. The text is sanitized because it
    /// often carries raw `git`/`ssh` stderr — ANSI escapes or control
    /// characters rendered into the buffer would tear the TUI frame.
    pub fn set_error(&mut self, msg: String) {
        crate::debug_log::error(&msg);
        // Carriage returns (progress-frame rewrites in git/ssh output) survive
        // sanitize_text; flatten them so the popup shows clean lines.
        let clean = crate::repo::sanitize_text(&msg).replace("\r\n", "\n").replace('\r', "\n");
        self.error_message = Some(clean);
    }

    /// Number of the PR selected in the PRs tab, if its list is loaded.
    pub fn selected_pr_number(&self) -> Option<u32> {
        match &self.current_detail {
            Some(repo::ItemDetail::Repo { info, .. }) => match &info.forge_prs {
                repo::TabData::Loaded(prs) => prs.get(self.forge_pr_selection).map(|pr| pr.number),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn load_comments_for_selected_pr(&mut self) {
        self.forge_pr_comments = None;
        self.forge_pr_comments_pr = self.selected_pr_number();
        let Some(pr_number) = self.forge_pr_comments_pr else {
            return;
        };
        if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
            self.forge_pr_comments_loading = true;
            let path = resolved.clone();
            let timeout = std::time::Duration::from_secs(self.config.fetch_timeout_secs);
            let tx = self.tab_tx.clone();
            std::thread::spawn(move || {
                let result = repo::load_pr_comments(&path, pr_number, timeout);
                let _ = tx.send((
                    path.to_string_lossy().to_string(),
                    11, // tab_idx (PRs tab)
                    repo::TabPayload::PRComments { pr_number, result },
                ));
            });
        }
    }

    pub fn status_height(&self) -> u16 {
        if self.status_expanded {
            let width = if let Ok(size) = crossterm::terminal::size() { size.0 } else { 80 };
            crate::components::cmd_bar::calculate_status_rows(self, width)
        } else {
            1
        }
    }

    pub fn toggle_status_expanded(&mut self) {
        self.status_expanded = !self.status_expanded;
    }
}
