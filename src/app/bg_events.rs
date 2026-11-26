//! Typed background worker events and `App::drain_bg_events`.
//!
//! Background threads report completion by pushing a [`BgEvent`] onto `App.tx`
//! instead of encoding payloads and error classification in `"PREFIX:..."` /
//! `"|||"` strings. This prevents branch names containing `"failed"` from being
//! misclassified as errors and ensures failures from a repo that was closed
//! mid-flight still surface cleanly.

use std::path::{Path, PathBuf};

use super::{App, DetailSection, Mode, is_newer_version};
use crate::repo;

/// Which statistic counter a successful [`BgEvent::RepoOpResult`] updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoOpKind {
    Push,
    Fetch,
    Pull,
    Other,
}

/// Typed events emitted by background threads onto `App::tx`.
#[derive(Debug, Clone)]
pub enum BgEvent {
    RepoScanFound { name: String, path: String },
    RepoScanCount(usize),
    RepoScanComplete,
    BulkFetchSuccess { path: String },
    BulkFetchError { path: String, error: String },
    RefreshRepo { repo_root: String },
    UpdateCheckSuccess { latest_version: String },
    UpdateCheckFailed,
    SelfUpdateResult(Result<String, String>),
    CloneResult(Result<String, String>),
    ForgeCheckoutResult { repo_path: PathBuf, result: Result<String, String> },
    ForgeCommentResult { repo_path: PathBuf, result: Result<(), String> },
    RemoteTagsResult { repo_path: PathBuf, result: Result<Vec<repo::BranchInfo>, String> },
    RepoOpResult { repo_path: PathBuf, kind: RepoOpKind, result: Result<String, String> },
}

fn repo_display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

impl App {
    /// Whether `repo_path` matches the repository currently open in Detail view.
    fn is_open_repo(&self, repo_path: &Path) -> bool {
        matches!(
            &self.current_detail,
            Some(repo::ItemDetail::Repo { resolved, .. }) if resolved.as_path() == repo_path
        )
    }

    /// Drains all pending [`BgEvent`]s from `self.rx`, returning whether any
    /// event was processed (so the event loop knows to redraw).
    pub fn drain_bg_events(&mut self) -> bool {
        let mut received = false;
        while let Ok(event) = self.rx.try_recv() {
            received = true;
            self.handle_bg_event(event);
        }
        received
    }

    fn handle_bg_event(&mut self, event: BgEvent) {
        match event {
            BgEvent::RepoScanFound { name, path } => {
                if !self.scanned_repos.iter().any(|(_, p)| p == &path) {
                    self.scanned_repos.push((name, path));
                }
            }
            BgEvent::RepoScanCount(count) => {
                self.repo_scan_count = count;
            }
            BgEvent::RepoScanComplete => {
                self.repo_scan_active = false;
            }
            BgEvent::BulkFetchSuccess { path } => {
                self.bulk_fetching.remove(&path);
                if let Some(idx) = self.config.items.iter().position(|item| item == &path) {
                    let moved = self.store_status(idx, &path, repo::inspect_summary(&path));
                    self.resort_after_status_refresh(moved);
                }
                self.bulk_fetch_results
                    .insert(path.clone(), Ok("Fetched successfully".to_string()));
                self.bulk_fetch_failures.remove(&path);
                if self.bulk_fetching.is_empty() {
                    self.status_message = Some(self.bulk_fetch_summary());
                    self.bulk_fetch_completed_at = Some(std::time::Instant::now());
                }
                self.decrement_implicit_network();
            }
            BgEvent::BulkFetchError { path, error } => {
                self.bulk_fetching.remove(&path);
                // Classify once, here, rather than on every render pass.
                let failure = crate::fetch_error::classify(&error);
                self.bulk_fetch_failures.insert(path.clone(), failure);
                self.bulk_fetch_results.insert(path.clone(), Err(error.clone()));
                crate::debug_log::warn(format!(
                    "Network Action: Fetch failed for '{}' ({}): {}",
                    path,
                    failure.label(),
                    error.replace('\n', " / ")
                ));
                if self.bulk_fetching.is_empty() {
                    self.status_message = Some(self.bulk_fetch_summary());
                    self.bulk_fetch_completed_at = Some(std::time::Instant::now());
                }
                self.decrement_implicit_network();
            }
            BgEvent::RefreshRepo { repo_root } => self.handle_refresh_repo(&repo_root),
            BgEvent::UpdateCheckSuccess { latest_version } => {
                let current_version = env!("CARGO_PKG_VERSION");
                if is_newer_version(current_version, &latest_version) {
                    self.update_available = Some(latest_version.clone());
                    if self.update_check_manual {
                        self.status_message =
                            Some(format!("Update available: v{}", latest_version));
                    }
                } else if self.update_check_manual {
                    self.status_message = Some("Gitwig is up to date".to_string());
                }
                if !self.update_check_manual {
                    self.decrement_implicit_network();
                }
                self.update_check_manual = false;
            }
            BgEvent::UpdateCheckFailed => {
                if self.update_check_manual {
                    self.status_message = Some("Failed to check for updates".to_string());
                }
                if !self.update_check_manual {
                    self.decrement_implicit_network();
                }
                self.update_check_manual = false;
            }
            BgEvent::SelfUpdateResult(result) => {
                self.fetching = false;
                match result {
                    Ok(msg) => self.status_message = Some(msg),
                    Err(err) => self.set_error(err),
                }
            }
            BgEvent::CloneResult(result) => {
                self.fetching = false;
                match result {
                    Ok(dest_path) => {
                        crate::debug_log::info(format!(
                            "Network Action: Cloning succeeded to {}",
                            dest_path
                        ));
                        self.status_message = Some("Cloning completed successfully".to_string());
                        self.pending_add_repo = Some(dest_path);
                        self.input_buffer.clear();
                        self.mode = Mode::CloneRepoLabelInput;
                    }
                    Err(err) => {
                        crate::debug_log::error(format!("Network Action: Cloning failed: {}", err));
                        self.set_error(err);
                    }
                }
            }
            BgEvent::ForgeCheckoutResult { repo_path, result } => {
                self.fetching = false;
                let relevant = self.is_open_repo(&repo_path);
                match result {
                    Ok(msg) if relevant => {
                        self.status_message = Some(msg);
                        self.resync_detail();
                    }
                    Ok(_) => {}
                    Err(err) if relevant => {
                        self.set_error(err);
                    }
                    Err(err) => {
                        self.set_error(format!("[{}] {}", repo_display_name(&repo_path), err));
                    }
                }
            }
            BgEvent::ForgeCommentResult { repo_path, result } => {
                self.fetching = false;
                let relevant = self.is_open_repo(&repo_path);
                match result {
                    Ok(()) => {
                        self.stats.forge_comments_made += 1;
                        self.stats.forge_prs_reviewed += 1;
                        if relevant {
                            self.status_message = Some("Comment posted successfully".to_string());
                            self.load_comments_for_selected_pr();
                        }
                    }
                    Err(err) if relevant => {
                        self.set_error(err);
                    }
                    Err(err) => {
                        self.set_error(format!("[{}] {}", repo_display_name(&repo_path), err));
                    }
                }
            }
            BgEvent::RemoteTagsResult { repo_path, result } => {
                let was_explicit = self.fetching;
                if was_explicit {
                    self.fetching = false;
                } else {
                    self.decrement_implicit_network();
                }
                let relevant = self.is_open_repo(&repo_path);
                match result {
                    Ok(tags) => {
                        if relevant {
                            crate::debug_log::info(
                                "Network Action: Fetching remote tags succeeded",
                            );
                            if let Some(repo::ItemDetail::Repo { info, .. }) =
                                &mut self.current_detail
                            {
                                info.remote_tags = repo::TabData::Loaded(tags);
                                info.remote_tags_loaded = true;
                            }
                        }
                    }
                    Err(err) => {
                        crate::debug_log::warn(format!(
                            "Network Action: Fetching remote tags failed: {}",
                            err
                        ));
                        if was_explicit {
                            if relevant {
                                self.set_error(err);
                            } else {
                                self.set_error(format!(
                                    "[{}] {}",
                                    repo_display_name(&repo_path),
                                    err
                                ));
                            }
                        }
                    }
                }
            }
            BgEvent::RepoOpResult { repo_path, kind, result } => {
                self.fetching = false;
                let relevant = self.is_open_repo(&repo_path);
                if relevant {
                    match result {
                        Ok(msg) => {
                            crate::debug_log::info(format!(
                                "Network Action: Operation succeeded: {}",
                                msg
                            ));
                            match kind {
                                RepoOpKind::Push => self.stats.pushes += 1,
                                RepoOpKind::Fetch => self.stats.fetches += 1,
                                RepoOpKind::Pull => self.stats.pulls += 1,
                                RepoOpKind::Other => {}
                            }
                            self.status_message = Some(msg);
                        }
                        Err(err) => {
                            let has_conflict = err.contains("conflict") || err.contains("CONFLICT");
                            crate::debug_log::error(format!(
                                "Network Action: Operation failed: {}",
                                err
                            ));
                            self.set_error(err);
                            if has_conflict {
                                self.detail_focus = DetailSection::Conflicts;
                            }
                        }
                    }
                    self.resync_detail();
                } else if let Err(err) = result {
                    // Surface failures from a repository the user already navigated away from.
                    self.set_error(format!("[{}] {}", repo_display_name(&repo_path), err));
                }
            }
        }
    }

    fn handle_refresh_repo(&mut self, repo_path: &str) {
        let canon_target =
            std::fs::canonicalize(repo_path).unwrap_or_else(|_| PathBuf::from(repo_path));
        let already_tracked = self.config.items.iter().position(|item| {
            let expanded = repo::expand_tilde(item);
            let canon_item = std::fs::canonicalize(&expanded).unwrap_or(expanded);
            canon_item == canon_target
        });
        if let Some(idx) = already_tracked {
            let item = self.config.items[idx].clone();
            let moved = self.store_status(idx, &item, repo::inspect_summary(&item));
            self.resort_after_status_refresh(moved);
            if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
                if resolved == &canon_target {
                    self.resync_detail();
                }
            }
            return;
        }

        let mut inside_watch_dir = false;
        if self.config.enable_watch_dirs {
            for watch_dir in &self.config.watch_dirs {
                let expanded = repo::expand_tilde(watch_dir);
                if let Ok(canon_watch) = std::fs::canonicalize(&expanded) {
                    if canon_target.starts_with(&canon_watch) {
                        inside_watch_dir = true;
                        break;
                    }
                }
            }
        }
        if !inside_watch_dir {
            return;
        }
        if self
            .config
            .scan
            .excludes
            .iter()
            .any(|ex| canon_target.components().any(|c| c.as_os_str() == ex.as_str()))
        {
            return;
        }
        let git_dir = canon_target.join(".git");
        if git_dir.exists() && git_dir.is_dir() {
            self.auto_discover_add(repo_path.to_string());
            self.status_message = Some("Auto-discovered new repository".to_string());
        }
    }
}
