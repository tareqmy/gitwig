//! Initialising and updating submodules from the Submodules tab (`u` / `U`).

use super::git::RepoSender;
use super::*;

/// A submodule update, held while `Mode::SubmoduleUpdateConfirm` asks first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmoduleUpdateTarget {
    /// The submodule to update; `None` updates every submodule.
    pub path: Option<PathBuf>,
    /// The targeted submodules that show `Modified`. The update moves each one
    /// back to the commit the repository records, so any of them means asking.
    pub modified: Vec<PathBuf>,
}

impl SubmoduleUpdateTarget {
    /// How messages name what is being updated.
    pub fn label(&self) -> String {
        match &self.path {
            Some(path) => format!("submodule '{}'", path.display()),
            None => "all submodules".to_string(),
        }
    }
}

impl App {
    fn loaded_submodules(&self) -> Option<&[repo::SubmoduleInfo]> {
        match &self.current_detail {
            Some(repo::ItemDetail::Repo { info, .. }) => match &info.submodules {
                repo::TabData::Loaded(subs) => Some(subs),
                _ => None,
            },
            _ => None,
        }
    }

    /// `u`: initialises or updates the selected submodule.
    pub fn request_submodule_update(&mut self) {
        let Some(sub) = self.loaded_submodules().and_then(|s| s.get(self.submodule_selection))
        else {
            return;
        };
        if sub.is_removal_staged() {
            self.status_message = Some(format!(
                "Submodule '{}' is staged for removal; nothing to update",
                sub.path.display()
            ));
            return;
        }
        let modified = if sub.is_modified() { vec![sub.path.clone()] } else { Vec::new() };
        let target = SubmoduleUpdateTarget { path: Some(sub.path.clone()), modified };
        self.request_update(target);
    }

    /// `U`: initialises or updates every submodule.
    pub fn request_submodule_update_all(&mut self) {
        let Some(subs) = self.loaded_submodules() else {
            return;
        };
        if subs.iter().all(|s| s.is_removal_staged()) {
            self.status_message = Some("No submodules to update".to_string());
            return;
        }
        let modified = subs.iter().filter(|s| s.is_modified()).map(|s| s.path.clone()).collect();
        self.request_update(SubmoduleUpdateTarget { path: None, modified });
    }

    fn request_update(&mut self, target: SubmoduleUpdateTarget) {
        if target.modified.is_empty() {
            self.start_submodule_update(target);
        } else {
            self.submodule_update_target = Some(target);
            self.mode = Mode::SubmoduleUpdateConfirm;
        }
    }

    pub fn confirm_submodule_update(&mut self) {
        match self.submodule_update_target.take() {
            Some(target) => self.start_submodule_update(target),
            None => self.mode = Mode::Detail,
        }
    }

    pub fn cancel_submodule_update(&mut self) {
        self.submodule_update_target = None;
        self.mode = Mode::Detail;
    }

    fn start_submodule_update(&mut self, target: SubmoduleUpdateTarget) {
        self.mode = Mode::Detail;
        let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail else {
            return;
        };
        let repo_path = resolved.clone();
        let label = target.label();
        let r_name = repo_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        crate::debug_log::info(format!(
            "Network Action: Updating {} in repository '{}' (user triggered)",
            label, r_name
        ));
        self.fetching = true;
        self.status_message = Some(format!("Updating {}...", label));

        let tx = RepoSender { tx: self.tx.clone(), path: repo_path.clone() };
        let timeout = std::time::Duration::from_secs(self.config.fetch_timeout_secs);
        std::thread::spawn(move || {
            let msg = match repo::submodule_update(&repo_path, target.path.as_deref(), timeout) {
                Ok(()) => format!("Updated {}", label),
                Err(e) => format!("Failed to update {}: {}", label, e),
            };
            let _ = tx.send(msg);
        });
    }
}
