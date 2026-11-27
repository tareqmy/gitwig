//! Interactive Git Bisect actions on `App`.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BisectMark {
    Good,
    Bad,
    Skip,
}

impl App {
    /// Opens the interactive Git Bisect overlay for the active repository.
    pub fn open_bisect_popup(&mut self) {
        if !matches!(self.current_detail, Some(ItemDetail::Repo { .. })) {
            return;
        }
        self.bisect_log_scroll = 0;
        self.mode = Mode::Bisect;
    }

    fn active_bisect_repo_path(&self) -> Option<PathBuf> {
        match &self.current_detail {
            Some(ItemDetail::Repo { resolved, .. }) => Some(resolved.clone()),
            _ => None,
        }
    }

    fn sync_bisect_snapshot(&mut self, repo_path: &std::path::Path) {
        let status = repo::get_bisect_status(repo_path);
        if let Some(ItemDetail::Repo { info, .. }) = &mut self.current_detail {
            info.summary.state =
                if status.active { repo::RepoState::Bisect } else { repo::RepoState::Clean };
            info.bisect = status;
        }
    }

    /// Starts a `git bisect` session (optionally marking `bad` and `good`).
    pub fn run_bisect_start(&mut self, bad: Option<&str>, good: Option<&str>) {
        let Some(repo_path) = self.active_bisect_repo_path() else {
            return;
        };
        match repo::bisect_start(&repo_path, bad, good) {
            Ok(outcome) => {
                self.bisect_last_message = Some(outcome.summary.clone());
                self.status_message = Some(outcome.summary);
                self.sync_bisect_snapshot(&repo_path);
                self.refresh_detail();
                self.refresh_active_repo_status();
            }
            Err(e) => self.set_error(format!("Bisect start failed: {}", e)),
        }
    }

    /// Marks `rev` (or `HEAD` when `None`) as Good, Bad, or Skip.
    pub fn run_bisect_mark(&mut self, mark: BisectMark, rev: Option<&str>) {
        let Some(repo_path) = self.active_bisect_repo_path() else {
            return;
        };
        let res = match mark {
            BisectMark::Good => repo::bisect_good(&repo_path, rev),
            BisectMark::Bad => repo::bisect_bad(&repo_path, rev),
            BisectMark::Skip => repo::bisect_skip(&repo_path, rev),
        };
        match res {
            Ok(outcome) => {
                self.bisect_last_message = Some(outcome.summary.clone());
                self.status_message = Some(outcome.summary);
                self.sync_bisect_snapshot(&repo_path);
                self.refresh_detail();
                self.refresh_active_repo_status();
            }
            Err(e) => self.set_error(format!("Bisect failed: {}", e)),
        }
    }

    /// Ends the active bisect session and restores the original HEAD.
    pub fn run_bisect_reset(&mut self) {
        let Some(repo_path) = self.active_bisect_repo_path() else {
            return;
        };
        match repo::bisect_reset(&repo_path) {
            Ok(msg) => {
                self.bisect_last_message = None;
                self.bisect_log_scroll = 0;
                self.mode = Mode::Detail;
                self.status_message = Some(msg);
                self.sync_bisect_snapshot(&repo_path);
                self.refresh_detail();
                self.refresh_active_repo_status();
            }
            Err(e) => self.set_error(format!("Bisect reset failed: {}", e)),
        }
    }
}
