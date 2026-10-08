//! Partial stashing: picking the files the stashing panel stashes, and
//! stashing the selected hunk or line of an Unstaged diff (`z`).

use std::collections::HashSet;

use super::*;
use crate::repo::{FileEntry, WorktreeChanges};

/// The status buckets a stashing-panel file sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StashFileKind {
    Conflicted,
    Staged,
    Unstaged,
    StagedAndUnstaged,
    Untracked,
}

impl StashFileKind {
    /// The caption shown after the file's path.
    pub fn caption(self) -> &'static str {
        match self {
            Self::Conflicted => "conflict",
            Self::Staged => "staged",
            Self::Unstaged => "unstaged",
            Self::StagedAndUnstaged => "staged + unstaged",
            Self::Untracked => "untracked",
        }
    }
}

/// One file the stash can take, as a row of the stashing panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashFileRow {
    pub path: String,
    pub label: &'static str,
    pub kind: StashFileKind,
}

/// The stashing panel's rows: conflicted, staged, then unstaged-only files,
/// then untracked ones while they are being stashed.
///
/// Each file is listed once. The status lists repeat a file that is both
/// staged and modified, and list an untracked file as unstaged and untracked;
/// `git stash push -- <path>` takes all of a file's changes either way.
pub fn stash_file_rows(changes: &WorktreeChanges, include_untracked: bool) -> Vec<StashFileRow> {
    let untracked: HashSet<&str> = changes.untracked.iter().map(|e| e.path.as_str()).collect();
    let staged: HashSet<&str> = changes.staged.iter().map(|e| e.path.as_str()).collect();
    let modified: HashSet<&str> = changes
        .unstaged
        .iter()
        .map(|e| e.path.as_str())
        .filter(|path| !untracked.contains(path))
        .collect();
    let row = |e: &FileEntry, kind| StashFileRow { path: e.path.clone(), label: e.label, kind };

    let mut rows: Vec<StashFileRow> =
        changes.conflicted.iter().map(|e| row(e, StashFileKind::Conflicted)).collect();
    rows.extend(changes.staged.iter().map(|e| {
        let kind = if modified.contains(e.path.as_str()) {
            StashFileKind::StagedAndUnstaged
        } else {
            StashFileKind::Staged
        };
        row(e, kind)
    }));
    rows.extend(
        changes
            .unstaged
            .iter()
            .filter(|e| modified.contains(e.path.as_str()) && !staged.contains(e.path.as_str()))
            .map(|e| row(e, StashFileKind::Unstaged)),
    );
    if include_untracked {
        rows.extend(changes.untracked.iter().map(|e| row(e, StashFileKind::Untracked)));
    }
    rows
}

impl App {
    /// The stashing panel's rows for the open repository.
    pub fn stash_panel_rows(&self) -> Vec<StashFileRow> {
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => {
                stash_file_rows(&info.changes, self.stash_untracked)
            }
            _ => Vec::new(),
        }
    }

    /// The picked files still listed, in row order. Empty when nothing is
    /// picked, which stashes everything.
    pub fn stash_panel_paths(&self) -> Vec<String> {
        self.stash_panel_rows()
            .into_iter()
            .map(|row| row.path)
            .filter(|path| self.stash_marked.contains(path))
            .collect()
    }

    /// Opens the stashing panel with its defaults and nothing picked.
    pub fn open_stashing_ui(&mut self) {
        self.stashing_ui_selection = 0;
        self.stash_untracked = true;
        self.stash_keep_index = false;
        self.stash_marked.clear();
        self.mode = Mode::StashingUI;
    }

    /// Moves the stashing panel's cursor one row up or down.
    pub fn stash_panel_move(&mut self, down: bool) {
        self.stashing_ui_selection = if down {
            let last = self.stash_panel_rows().len().saturating_sub(1);
            (self.stashing_ui_selection + 1).min(last)
        } else {
            self.stashing_ui_selection.saturating_sub(1)
        };
    }

    /// `Space`: picks or unpicks the file under the cursor.
    pub fn stash_panel_toggle_file(&mut self) {
        if let Some(row) = self.stash_panel_rows().into_iter().nth(self.stashing_ui_selection) {
            if !self.stash_marked.remove(&row.path) {
                self.stash_marked.insert(row.path);
            }
        }
    }

    /// `a`: picks every listed file, or clears the picks when all are picked.
    pub fn stash_panel_toggle_all(&mut self) {
        let rows = self.stash_panel_rows();
        if rows.iter().all(|row| self.stash_marked.contains(&row.path)) {
            self.stash_marked.clear();
        } else {
            self.stash_marked.extend(rows.into_iter().map(|row| row.path));
        }
    }

    /// `u`: toggles stashing untracked files. Turning it off drops them from
    /// the list, so their picks go too and the cursor stays on a row.
    pub fn stash_panel_toggle_untracked(&mut self) {
        self.stash_untracked = !self.stash_untracked;
        if !self.stash_untracked {
            if let Some(ItemDetail::Repo { info, .. }) = &self.current_detail {
                for entry in &info.changes.untracked {
                    self.stash_marked.remove(&entry.path);
                }
            }
        }
        let last = self.stash_panel_rows().len().saturating_sub(1);
        self.stashing_ui_selection = self.stashing_ui_selection.min(last);
    }

    /// `s` in the stashing panel: asks for the stash's name. The stash takes
    /// the picked files, and cancelling returns to the panel.
    pub fn start_panel_stash_create(&mut self) {
        self.start_stash_create();
        self.stash_from_panel = true;
    }

    /// The repository and the file (path, status label) the diff pane shows,
    /// when it shows an Unstaged file.
    fn unstaged_diff_file(&self) -> Option<(PathBuf, String, &'static str)> {
        let Some(ItemDetail::Repo { resolved, info }) = &self.current_detail else {
            return None;
        };
        let on_unstaged = match self.detail_focus {
            DetailSection::Unstaged => true,
            DetailSection::StagingDetails => self.last_staging_focus == DetailSection::Unstaged,
            _ => false,
        };
        if !on_unstaged {
            return None;
        }
        info.changes
            .unstaged
            .get(self.status_list.staging_file_selection)
            .map(|f| (resolved.clone(), f.path.clone(), f.label))
    }

    /// `z` in an Unstaged diff: stashes the selected hunk, or the selected
    /// line in line mode, and takes it out of the working tree.
    pub fn stash_selected_change(&mut self) {
        let Some((repo_path, file_path, label)) = self.unstaged_diff_file() else {
            return;
        };
        if label != "M" {
            let what = match label {
                "N" => "a new file",
                "D" => "deleted",
                "R" => "renamed",
                "T" => "a type change",
                _ => "not a modified file",
            };
            self.set_error(format!(
                "Only a modified file can be stashed by hunk or line, and '{}' is {}. \
                 Stash the whole file from the stashing panel instead.",
                file_path, what
            ));
            return;
        }

        let ranges = self.get_diff_hunk_ranges();
        let Some(range) = ranges.get(self.diff.diff_hunk_selection).cloned() else {
            return;
        };
        let line_mode = self.diff.diff_line_mode;
        if line_mode && !range.contains(&self.diff.diff_line_selection) {
            return;
        }
        let hunk = &self.diff.file_diff[range.clone()];
        let result = if line_mode {
            repo::stash_line(
                &repo_path,
                &file_path,
                hunk,
                self.diff.diff_line_selection - range.start,
            )
        } else {
            repo::stash_hunk(&repo_path, &file_path, hunk)
        };

        match result {
            Ok(what) => {
                self.status_message = Some(format!("Stashed {}", what));
                self.stats.stashes += 1;
                self.stash_list.stash_selection = 0;
                self.stash_list.stash_file_selection = 0;
                if line_mode {
                    self.refresh_detail_for_line_action();
                } else {
                    let prev_hunk_idx = self.diff.diff_hunk_selection;
                    self.refresh_detail();
                    let hunk_count = self.get_diff_hunk_ranges().len();
                    self.diff.diff_hunk_selection = prev_hunk_idx.min(hunk_count.saturating_sub(1));
                    self.scroll_to_selected_hunk();
                }
            }
            Err(e) => self.set_error(format!("Stash failed: {}", e)),
        }
    }
}
