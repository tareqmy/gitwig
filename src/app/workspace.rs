use super::*;

impl App {
    pub fn request_cherry_pick(&mut self) {
        if self.is_uncommitted_selected() {
            self.status_message = Some("Cannot cherry-pick uncommitted changes.".to_string());
            return;
        }
        let commit_data = match &self.current_detail {
            Some(repo::ItemDetail::Repo { info, .. }) => {
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                let commit_idx = if dirty {
                    self.commit_list.selection.saturating_sub(1)
                } else {
                    self.commit_list.selection
                };
                info.commits.get(commit_idx).map(|c| (c.oid.clone(), c.summary.clone()))
            }
            _ => None,
        };

        if let Some((oid, summary)) = commit_data {
            let mut local_branches = Vec::new();
            if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
                let current_branch = info.branch.as_deref().unwrap_or("HEAD");

                let mut branches_list = Vec::new();
                if let Some(branches) = info.local_branches.as_ref() {
                    branches_list = branches.iter().map(|b| b.name.clone()).collect();
                }

                if branches_list.is_empty() {
                    match repo::load_tab_branches(resolved) {
                        (Ok(branches), _) => {
                            branches_list = branches.iter().map(|b| b.name.clone()).collect();
                        }
                        (Err(err), _) => {
                            self.status_message =
                                Some(format!("Failed to load local branches: {}", err));
                        }
                    }
                }

                local_branches =
                    branches_list.into_iter().filter(|name| name != current_branch).collect();
            }

            if local_branches.is_empty() && self.status_message.is_none() {
                self.status_message = Some("No local destination branches found.".to_string());
            }

            self.cherry_pick_dest_branches = local_branches;
            self.cherry_pick_dest_selection = 0;
            self.cherry_pick_target = Some((oid, summary));
            self.mode = Mode::CherryPickConfirm;
        }
    }

    pub fn confirm_cherry_pick(&mut self) {
        let target = self.cherry_pick_target.take();
        let dest_branch =
            self.cherry_pick_dest_branches.get(self.cherry_pick_dest_selection).cloned();
        self.mode = Mode::Detail;

        if let (Some((commit_oid, _summary)), Some(dest_branch)) = (target, dest_branch) {
            if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
                self.fetching = true;
                self.status_message = Some(format!(
                    "Cherry-picking commit {:.7} into {}...",
                    commit_oid, dest_branch
                ));

                let repo_path = resolved.clone();
                let tx = self.tx.clone();

                std::thread::spawn(move || {
                    let res = (|| -> Result<String, Box<dyn std::error::Error>> {
                        // 1. Checkout the destination branch
                        let checkout_output = std::process::Command::new("git")
                            .arg("checkout")
                            .arg(&dest_branch)
                            .current_dir(&repo_path)
                            .output()?;
                        if !checkout_output.status.success() {
                            let stderr =
                                String::from_utf8_lossy(&checkout_output.stderr).trim().to_string();
                            return Err(format!("git checkout failed: {}", stderr).into());
                        }

                        // 2. Perform cherry-pick
                        let output = std::process::Command::new("git")
                            .env("GIT_TERMINAL_PROMPT", "0")
                            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
                            .arg("cherry-pick")
                            .arg(&commit_oid)
                            .current_dir(&repo_path)
                            .output()?;

                        if output.status.success() {
                            Ok(format!(
                                "Cherry-picked commit {:.7} successfully into {}",
                                commit_oid, dest_branch
                            ))
                        } else {
                            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                            let mut err_msg = if !stderr.is_empty() { stderr } else { stdout };
                            if err_msg.contains("CONFLICT") || err_msg.contains("conflict") {
                                err_msg = "Conflicts detected. Please resolve in terminal or abort (git cherry-pick --abort).".to_string();
                            }
                            Err(format!("git cherry-pick failed: {}", err_msg).into())
                        }
                    })();

                    let msg = match res {
                        Ok(success) => success,
                        Err(e) => format!("Cherry-pick failed: {}", e),
                    };
                    let _ = tx.send(msg);
                });
            }
        }
    }

    pub fn cancel_cherry_pick(&mut self) {
        self.cherry_pick_target = None;
        self.cherry_pick_dest_branches.clear();
        self.cherry_pick_dest_selection = 0;
        self.mode = Mode::Detail;
    }

    pub fn request_revert(&mut self) {
        if self.is_uncommitted_selected() {
            self.status_message = Some("Cannot revert uncommitted changes.".to_string());
            return;
        }
        let commit_data = match &self.current_detail {
            Some(repo::ItemDetail::Repo { info, .. }) => {
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                let commit_idx = if dirty {
                    self.commit_list.selection.saturating_sub(1)
                } else {
                    self.commit_list.selection
                };
                info.commits.get(commit_idx).map(|c| (c.oid.clone(), c.summary.clone()))
            }
            _ => None,
        };

        if let Some((oid, summary)) = commit_data {
            self.revert_target = Some((oid, summary));
            self.mode = Mode::RevertConfirm;
        }
    }

    pub fn confirm_revert(&mut self) {
        let target = self.revert_target.take();
        self.mode = Mode::Detail;

        if let Some((commit_oid, _summary)) = target {
            if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
                self.fetching = true;
                self.status_message = Some(format!("Reverting commit {:.7}...", commit_oid));

                let repo_path = resolved.clone();
                let tx = self.tx.clone();

                std::thread::spawn(move || {
                    let res = (|| -> Result<String, Box<dyn std::error::Error>> {
                        let output = std::process::Command::new("git")
                            .env("GIT_TERMINAL_PROMPT", "0")
                            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
                            .arg("revert")
                            .arg("--no-edit")
                            .arg(&commit_oid)
                            .current_dir(&repo_path)
                            .output()?;

                        if output.status.success() {
                            Ok(format!("Reverted commit {:.7} successfully", commit_oid))
                        } else {
                            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                            let mut err_msg = if !stderr.is_empty() { stderr } else { stdout };
                            if err_msg.contains("CONFLICT") || err_msg.contains("conflict") {
                                err_msg = "Conflicts detected. Please resolve in terminal or abort (git revert --abort).".to_string();
                            }
                            Err(format!("git revert failed: {}", err_msg).into())
                        }
                    })();

                    let msg = match res {
                        Ok(success) => success,
                        Err(e) => format!("Revert failed: {}", e),
                    };
                    let _ = tx.send(msg);
                });
            }
        }
    }

    pub fn cancel_revert(&mut self) {
        self.revert_target = None;
        self.mode = Mode::Detail;
    }

    /// Scroll the diff panel up by one line.

    /// Scroll the diff panel up by `page` lines.

    pub fn get_conflict_hunk_ranges(&self) -> Vec<std::ops::Range<usize>> {
        let mut ranges = Vec::new();
        let mut start = None;
        for (i, line) in self.diff.file_diff.iter().enumerate() {
            if line.kind == repo::DiffLineKind::ConflictSeparator {
                if line.content.starts_with("<<<<<<<") {
                    start = Some(i);
                } else if line.content.starts_with(">>>>>>>") {
                    if let Some(s) = start {
                        ranges.push(s..i + 1);
                        start = None;
                    }
                }
            }
        }
        ranges
    }

    pub fn get_diff_hunk_ranges(&self) -> Vec<std::ops::Range<usize>> {
        if self.detail_focus == DetailSection::Conflicts
            || self.detail_focus == DetailSection::ConflictDiff
            || self.last_staging_focus == DetailSection::Conflicts
        {
            return self.get_conflict_hunk_ranges();
        }
        let mut ranges = Vec::new();
        let mut current_start = None;
        for (i, line) in self.diff.file_diff.iter().enumerate() {
            if line.kind == repo::DiffLineKind::Header {
                if let Some(start) = current_start {
                    ranges.push(start..i);
                }
                current_start = Some(i);
            }
        }
        if let Some(start) = current_start {
            ranges.push(start..self.diff.file_diff.len());
        }
        ranges
    }

    pub fn toggle_diff_line_mode(&mut self) {
        if self.diff.file_diff.is_empty() {
            return;
        }
        self.diff.diff_line_mode = !self.diff.diff_line_mode;
        if self.diff.diff_line_mode {
            let ranges = self.get_diff_hunk_ranges();
            if let Some(range) = ranges.get(self.diff.diff_hunk_selection) {
                self.diff.diff_line_selection = range.start;
            } else {
                self.diff.diff_line_selection = 0;
            }
        } else {
            let ranges = self.get_diff_hunk_ranges();
            for (idx, range) in ranges.iter().enumerate() {
                if range.contains(&self.diff.diff_line_selection) {
                    self.diff.diff_hunk_selection = idx;
                    break;
                }
            }
        }
        self.scroll_to_selected_hunk();
    }

    /// Stage the currently-selected hunk in the Unstaged diff (`git apply --cached -`).
    pub fn stage_selected_hunk(&mut self) {
        let params = match &self.current_detail {
            Some(ItemDetail::Repo { resolved, info }) => {
                let focus_to_use = match self.detail_focus {
                    DetailSection::Staged => DetailSection::Staged,
                    DetailSection::Unstaged => DetailSection::Unstaged,
                    DetailSection::StagingDetails => self.last_staging_focus,
                    _ => return,
                };
                if focus_to_use != DetailSection::Unstaged {
                    return;
                }
                info.changes
                    .unstaged
                    .get(self.status_list.staging_file_selection)
                    .map(|f| (resolved.clone(), f.path.clone()))
            }
            _ => None,
        };
        if let Some((repo_path, file_path)) = params {
            let ranges = self.get_diff_hunk_ranges();
            if let Some(range) = ranges.get(self.diff.diff_hunk_selection) {
                let hunk = &self.diff.file_diff[range.clone()];
                match repo::stage_hunk(&repo_path, &file_path, hunk) {
                    Ok(()) => {
                        self.status_message = Some(format!("Staged hunk from: {}", file_path));
                        let prev_hunk_idx = self.diff.diff_hunk_selection;
                        self.refresh_detail();
                        let new_hunk_count = self.get_diff_hunk_ranges().len();
                        self.diff.diff_hunk_selection =
                            prev_hunk_idx.min(new_hunk_count.saturating_sub(1));
                        self.scroll_to_selected_hunk();
                    }
                    Err(e) => self.status_message = Some(format!("Stage hunk failed: {}", e)),
                }
            }
        }
    }

    /// Unstage the currently-selected hunk in the Staged diff (`git apply --cached --reverse -`).
    pub fn unstage_selected_hunk(&mut self) {
        let params = match &self.current_detail {
            Some(ItemDetail::Repo { resolved, info }) => {
                let focus_to_use = match self.detail_focus {
                    DetailSection::Staged => DetailSection::Staged,
                    DetailSection::Unstaged => DetailSection::Unstaged,
                    DetailSection::StagingDetails => self.last_staging_focus,
                    _ => return,
                };
                if focus_to_use != DetailSection::Staged {
                    return;
                }
                info.changes
                    .staged
                    .get(self.status_list.staging_file_selection)
                    .map(|f| (resolved.clone(), f.path.clone()))
            }
            _ => None,
        };
        if let Some((repo_path, file_path)) = params {
            let ranges = self.get_diff_hunk_ranges();
            if let Some(range) = ranges.get(self.diff.diff_hunk_selection) {
                let hunk = &self.diff.file_diff[range.clone()];
                match repo::unstage_hunk(&repo_path, &file_path, hunk) {
                    Ok(()) => {
                        self.status_message = Some(format!("Unstaged hunk from: {}", file_path));
                        let prev_hunk_idx = self.diff.diff_hunk_selection;
                        self.refresh_detail();
                        let new_hunk_count = self.get_diff_hunk_ranges().len();
                        self.diff.diff_hunk_selection =
                            prev_hunk_idx.min(new_hunk_count.saturating_sub(1));
                        self.scroll_to_selected_hunk();
                    }
                    Err(e) => self.status_message = Some(format!("Unstage hunk failed: {}", e)),
                }
            }
        }
    }

    /// Discard the currently-selected hunk in the Unstaged diff (`git apply --reverse -`).
    pub fn discard_selected_hunk(&mut self) {
        let params = match &self.current_detail {
            Some(ItemDetail::Repo { resolved, info }) => {
                let focus_to_use = match self.detail_focus {
                    DetailSection::Staged => DetailSection::Staged,
                    DetailSection::Unstaged => DetailSection::Unstaged,
                    DetailSection::StagingDetails => self.last_staging_focus,
                    _ => return,
                };
                if focus_to_use != DetailSection::Unstaged {
                    return;
                }
                info.changes
                    .unstaged
                    .get(self.status_list.staging_file_selection)
                    .map(|f| (resolved.clone(), f.path.clone()))
            }
            _ => None,
        };
        if let Some((repo_path, file_path)) = params {
            let ranges = self.get_diff_hunk_ranges();
            if let Some(range) = ranges.get(self.diff.diff_hunk_selection) {
                let hunk = &self.diff.file_diff[range.clone()];
                match repo::discard_hunk(&repo_path, &file_path, hunk) {
                    Ok(()) => {
                        self.status_message = Some(format!("Discarded hunk from: {}", file_path));
                        let prev_hunk_idx = self.diff.diff_hunk_selection;
                        self.refresh_detail();
                        let new_hunk_count = self.get_diff_hunk_ranges().len();
                        self.diff.diff_hunk_selection =
                            prev_hunk_idx.min(new_hunk_count.saturating_sub(1));
                        self.scroll_to_selected_hunk();
                    }
                    Err(e) => self.status_message = Some(format!("Discard hunk failed: {}", e)),
                }
            }
        }
    }

    /// Stage the currently-selected line in the Unstaged diff (`git apply --cached -`).
    pub fn stage_selected_line(&mut self) {
        let params = match &self.current_detail {
            Some(ItemDetail::Repo { resolved, info }) => {
                let focus_to_use = match self.detail_focus {
                    DetailSection::Staged => DetailSection::Staged,
                    DetailSection::Unstaged => DetailSection::Unstaged,
                    DetailSection::StagingDetails => self.last_staging_focus,
                    _ => return,
                };
                if focus_to_use != DetailSection::Unstaged {
                    return;
                }
                info.changes
                    .unstaged
                    .get(self.status_list.staging_file_selection)
                    .map(|f| (resolved.clone(), f.path.clone()))
            }
            _ => None,
        };
        if let Some((repo_path, file_path)) = params {
            let ranges = self.get_diff_hunk_ranges();
            if let Some(range) = ranges.get(self.diff.diff_hunk_selection) {
                if range.contains(&self.diff.diff_line_selection) {
                    let hunk = &self.diff.file_diff[range.clone()];
                    let selected_line_idx_in_hunk = self.diff.diff_line_selection - range.start;
                    match repo::stage_line(&repo_path, &file_path, hunk, selected_line_idx_in_hunk)
                    {
                        Ok(()) => {
                            self.status_message = Some(format!("Staged line from: {}", file_path));
                            self.refresh_detail_for_line_action();
                        }
                        Err(e) => self.status_message = Some(format!("Stage line failed: {}", e)),
                    }
                }
            }
        }
    }

    /// Unstage the currently-selected line in the Staged diff (`git apply --cached --reverse -`).
    pub fn unstage_selected_line(&mut self) {
        let params = match &self.current_detail {
            Some(ItemDetail::Repo { resolved, info }) => {
                let focus_to_use = match self.detail_focus {
                    DetailSection::Staged => DetailSection::Staged,
                    DetailSection::Unstaged => DetailSection::Unstaged,
                    DetailSection::StagingDetails => self.last_staging_focus,
                    _ => return,
                };
                if focus_to_use != DetailSection::Staged {
                    return;
                }
                info.changes
                    .staged
                    .get(self.status_list.staging_file_selection)
                    .map(|f| (resolved.clone(), f.path.clone()))
            }
            _ => None,
        };
        if let Some((repo_path, file_path)) = params {
            let ranges = self.get_diff_hunk_ranges();
            if let Some(range) = ranges.get(self.diff.diff_hunk_selection) {
                if range.contains(&self.diff.diff_line_selection) {
                    let hunk = &self.diff.file_diff[range.clone()];
                    let selected_line_idx_in_hunk = self.diff.diff_line_selection - range.start;
                    match repo::unstage_line(
                        &repo_path,
                        &file_path,
                        hunk,
                        selected_line_idx_in_hunk,
                    ) {
                        Ok(()) => {
                            self.status_message =
                                Some(format!("Unstaged line from: {}", file_path));
                            self.refresh_detail_for_line_action();
                        }
                        Err(e) => self.status_message = Some(format!("Unstage line failed: {}", e)),
                    }
                }
            }
        }
    }

    /// Discard the currently-selected line in the Unstaged diff (`git apply --reverse -`).
    pub fn discard_selected_line(&mut self) {
        let params = match &self.current_detail {
            Some(ItemDetail::Repo { resolved, info }) => {
                let focus_to_use = match self.detail_focus {
                    DetailSection::Staged => DetailSection::Staged,
                    DetailSection::Unstaged => DetailSection::Unstaged,
                    DetailSection::StagingDetails => self.last_staging_focus,
                    _ => return,
                };
                if focus_to_use != DetailSection::Unstaged {
                    return;
                }
                info.changes
                    .unstaged
                    .get(self.status_list.staging_file_selection)
                    .map(|f| (resolved.clone(), f.path.clone()))
            }
            _ => None,
        };
        if let Some((repo_path, file_path)) = params {
            let ranges = self.get_diff_hunk_ranges();
            if let Some(range) = ranges.get(self.diff.diff_hunk_selection) {
                if range.contains(&self.diff.diff_line_selection) {
                    let hunk = &self.diff.file_diff[range.clone()];
                    let selected_line_idx_in_hunk = self.diff.diff_line_selection - range.start;
                    match repo::discard_line(
                        &repo_path,
                        &file_path,
                        hunk,
                        selected_line_idx_in_hunk,
                    ) {
                        Ok(()) => {
                            self.status_message =
                                Some(format!("Discarded line from: {}", file_path));
                            self.refresh_detail_for_line_action();
                        }
                        Err(e) => self.status_message = Some(format!("Discard line failed: {}", e)),
                    }
                }
            }
        }
    }

    pub fn is_staged_empty(&self) -> bool {
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => info.changes.staged.is_empty(),
            _ => true,
        }
    }

    pub fn is_unstaged_empty(&self) -> bool {
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => info.changes.unstaged.is_empty(),
            _ => true,
        }
    }

    pub fn is_conflicted_empty(&self) -> bool {
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => info.changes.conflicted.is_empty(),
            _ => true,
        }
    }

    fn current_diff_params(&self) -> Option<(PathBuf, String, String)> {
        match &self.current_detail {
            Some(ItemDetail::Repo { resolved, .. }) => {
                let commit = self.get_selected_commit()?;
                let file = commit.files.get(self.status_list.file_selection)?;
                Some((resolved.clone(), commit.oid.clone(), file.path.clone()))
            }
            _ => None,
        }
    }

    pub fn refresh_file_diff(&mut self) {
        self.ensure_selected_commit_files_loaded();
        if self.detail_tab == 6 {
            let params = match &self.current_detail {
                Some(ItemDetail::Repo { resolved, info }) => {
                    info.stashes.get(self.stash_list.stash_selection).and_then(|stash| {
                        stash.files.get(self.stash_list.stash_file_selection).map(|file| {
                            (resolved.clone(), stash.commit_id.clone(), file.path.clone())
                        })
                    })
                }
                _ => None,
            };
            if let Some((repo_path, commit_oid, file_path)) = params {
                self.diff.file_diff =
                    repo::get_commit_file_diff(&repo_path, &commit_oid, &file_path);
            } else {
                self.diff.file_diff.clear();
            }
            return;
        }
        if self.is_uncommitted_selected() {
            let params = match &self.current_detail {
                Some(ItemDetail::Repo { resolved, info }) => {
                    if !info.changes.conflicted.is_empty() {
                        info.changes
                            .conflicted
                            .first()
                            .map(|f| (resolved.clone(), f.path.clone(), None))
                    } else if !info.changes.staged.is_empty() {
                        info.changes
                            .staged
                            .first()
                            .map(|f| (resolved.clone(), f.path.clone(), Some(true)))
                    } else if !info.changes.unstaged.is_empty() {
                        info.changes
                            .unstaged
                            .first()
                            .map(|f| (resolved.clone(), f.path.clone(), Some(false)))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some((repo_path, file_path, staged_opt)) = params {
                if let Some(staged) = staged_opt {
                    self.diff.file_diff =
                        repo::get_worktree_file_diff(&repo_path, &file_path, staged);
                } else {
                    self.diff.file_diff = repo::get_conflict_markers_diff(&repo_path, &file_path);
                }
            } else {
                self.diff.file_diff.clear();
            }
        } else if let Some((repo_path, commit_oid, file_path)) = self.current_diff_params() {
            self.diff.file_diff = repo::get_commit_file_diff(&repo_path, &commit_oid, &file_path);
        } else {
            self.diff.file_diff.clear();
        }
    }

    /// Reload `file_diff` from the currently-focused Staged/Unstaged file.
    pub fn refresh_staging_diff(&mut self) {
        let params = match &self.current_detail {
            Some(ItemDetail::Repo { resolved, info }) => {
                let focus_to_use = match self.detail_focus {
                    DetailSection::Staged => DetailSection::Staged,
                    DetailSection::Unstaged => DetailSection::Unstaged,
                    DetailSection::Conflicts => DetailSection::Conflicts,
                    DetailSection::StagingDetails => self.last_staging_focus,
                    _ => {
                        self.diff.file_diff.clear();
                        return;
                    }
                };
                match focus_to_use {
                    DetailSection::Staged => info
                        .changes
                        .staged
                        .get(self.status_list.staging_file_selection)
                        .map(|f| (resolved.clone(), f.path.clone(), Some(true))),
                    DetailSection::Unstaged => info
                        .changes
                        .unstaged
                        .get(self.status_list.staging_file_selection)
                        .map(|f| (resolved.clone(), f.path.clone(), Some(false))),
                    DetailSection::Conflicts => info
                        .changes
                        .conflicted
                        .get(self.status_list.conflict_file_selection)
                        .map(|f| (resolved.clone(), f.path.clone(), None)),
                    _ => {
                        self.diff.file_diff.clear();
                        return;
                    }
                }
            }
            _ => None,
        };
        if let Some((repo_path, file_path, staged_opt)) = params {
            if let Some(staged) = staged_opt {
                self.diff.file_diff = repo::get_worktree_file_diff(&repo_path, &file_path, staged);
            } else {
                self.diff.file_diff = repo::get_conflict_markers_diff(&repo_path, &file_path);
            }
        } else {
            self.diff.file_diff.clear();
        }
        self.diff.diff_hunk_selection = 0;
    }

    /// Stage the currently-selected file in the Unstaged panel (`git add`).
    pub fn stage_selected_file(&mut self) {
        let params = match &self.current_detail {
            Some(ItemDetail::Repo { resolved, info }) => info
                .changes
                .unstaged
                .get(self.status_list.staging_file_selection)
                .map(|f| (resolved.clone(), f.path.clone())),
            _ => None,
        };
        if let Some((repo_path, file_path)) = params {
            match repo::stage_file(&repo_path, &file_path) {
                Ok(()) => {
                    self.status_message = Some(format!("Staged: {}", file_path));
                    self.refresh_detail();
                }
                Err(e) => self.status_message = Some(format!("Stage failed: {}", e)),
            }
        }
    }

    /// Unstage the currently-selected file in the Staged panel (`git restore --staged`).
    pub fn unstage_selected_file(&mut self) {
        let params = match &self.current_detail {
            Some(ItemDetail::Repo { resolved, info }) => info
                .changes
                .staged
                .get(self.status_list.staging_file_selection)
                .map(|f| (resolved.clone(), f.path.clone())),
            _ => None,
        };
        if let Some((repo_path, file_path)) = params {
            match repo::unstage_file(&repo_path, &file_path) {
                Ok(()) => {
                    self.status_message = Some(format!("Unstaged: {}", file_path));
                    self.refresh_detail();
                }
                Err(e) => self.status_message = Some(format!("Unstage failed: {}", e)),
            }
        }
    }

    /// Accept the OURS version of the currently-selected conflicted file (whole file or selected hunk).
    pub fn resolve_conflict_ours(&mut self) {
        let params = match &self.current_detail {
            Some(ItemDetail::Repo { resolved, info }) => info
                .changes
                .conflicted
                .get(self.status_list.conflict_file_selection)
                .map(|f| (resolved.clone(), f.path.clone())),
            _ => None,
        };
        if let Some((repo_path, file_path)) = params {
            let result = if self.detail_focus == DetailSection::ConflictDiff {
                repo::resolve_conflict_hunk(
                    &repo_path,
                    &file_path,
                    self.diff.diff_hunk_selection,
                    true,
                )
            } else {
                repo::resolve_ours(&repo_path, &file_path)
            };
            match result {
                Ok(()) => {
                    let scope = if self.detail_focus == DetailSection::ConflictDiff {
                        format!("hunk {}", self.diff.diff_hunk_selection + 1)
                    } else {
                        "whole file".to_string()
                    };
                    self.status_message =
                        Some(format!("Resolved (ours, {}): {}", scope, file_path));
                    self.refresh_detail();
                    self.clamp_conflict_selection();
                    self.refresh_staging_diff();
                }
                Err(e) => self.status_message = Some(format!("Resolve ours failed: {}", e)),
            }
        }
    }

    /// Accept the THEIRS version of the currently-selected conflicted file (whole file or selected hunk).
    pub fn resolve_conflict_theirs(&mut self) {
        let params = match &self.current_detail {
            Some(ItemDetail::Repo { resolved, info }) => info
                .changes
                .conflicted
                .get(self.status_list.conflict_file_selection)
                .map(|f| (resolved.clone(), f.path.clone())),
            _ => None,
        };
        if let Some((repo_path, file_path)) = params {
            let result = if self.detail_focus == DetailSection::ConflictDiff {
                repo::resolve_conflict_hunk(
                    &repo_path,
                    &file_path,
                    self.diff.diff_hunk_selection,
                    false,
                )
            } else {
                repo::resolve_theirs(&repo_path, &file_path)
            };
            match result {
                Ok(()) => {
                    let scope = if self.detail_focus == DetailSection::ConflictDiff {
                        format!("hunk {}", self.diff.diff_hunk_selection + 1)
                    } else {
                        "whole file".to_string()
                    };
                    self.status_message =
                        Some(format!("Resolved (theirs, {}): {}", scope, file_path));
                    self.refresh_detail();
                    self.clamp_conflict_selection();
                    self.refresh_staging_diff();
                }
                Err(e) => self.status_message = Some(format!("Resolve theirs failed: {}", e)),
            }
        }
    }

    /// Mark the currently-selected conflicted file as resolved after manual edits.
    pub fn mark_conflict_resolved(&mut self) {
        let params = match &self.current_detail {
            Some(ItemDetail::Repo { resolved, info }) => info
                .changes
                .conflicted
                .get(self.status_list.conflict_file_selection)
                .map(|f| (resolved.clone(), f.path.clone())),
            _ => None,
        };
        if let Some((repo_path, file_path)) = params {
            match repo::mark_resolved(&repo_path, &file_path) {
                Ok(()) => {
                    self.status_message = Some(format!("Marked resolved: {}", file_path));
                    self.refresh_detail();
                    self.clamp_conflict_selection();
                    self.refresh_staging_diff();
                }
                Err(e) => self.status_message = Some(format!("Mark resolved failed: {}", e)),
            }
        }
    }

    /// Stage all unstaged/untracked changes in the repository (`git add -A`).
    pub fn stage_all_changes(&mut self) {
        if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
            match repo::stage_all_changes(resolved) {
                Ok(()) => {
                    self.status_message = Some("Staged all changes".to_string());
                    self.refresh_detail();
                    if self.detail_focus == DetailSection::Unstaged {
                        self.detail_focus = DetailSection::Staged;
                    }
                }
                Err(e) => self.status_message = Some(format!("Stage all failed: {}", e)),
            }
        }
    }

    /// Unstage all staged changes in the repository (`git reset`).
    pub fn unstage_all_changes(&mut self) {
        if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
            match repo::unstage_all_changes(resolved) {
                Ok(()) => {
                    self.status_message = Some("Unstaged all changes".to_string());
                    self.refresh_detail();
                    if self.detail_focus == DetailSection::Staged {
                        self.detail_focus = DetailSection::Unstaged;
                    }
                }
                Err(e) => self.status_message = Some(format!("Unstage all failed: {}", e)),
            }
        }
    }

    /// Discard all changes in the repository after confirmation.
    pub fn request_discard_all_changes(&mut self) {
        if let Some(repo::ItemDetail::Repo { .. }) = &self.current_detail {
            self.discard_target = Some(("All Changes".to_string(), false));
            self.mode = Mode::DiscardChangesConfirm;
        }
    }

    pub fn request_discard_changes(&mut self) {
        let params = match &self.current_detail {
            Some(repo::ItemDetail::Repo { resolved, info }) => match self.detail_focus {
                DetailSection::Staged => info
                    .changes
                    .staged
                    .get(self.status_list.staging_file_selection)
                    .map(|f| (resolved.clone(), f.path.clone(), true)),
                DetailSection::Unstaged => info
                    .changes
                    .unstaged
                    .get(self.status_list.staging_file_selection)
                    .map(|f| (resolved.clone(), f.path.clone(), false)),
                _ => None,
            },
            _ => None,
        };

        if let Some((_, file_path, staged)) = params {
            self.discard_target = Some((file_path, staged));
            self.mode = Mode::DiscardChangesConfirm;
        }
    }

    pub fn confirm_discard_changes(&mut self) {
        self.mode = Mode::Detail;
        let target = self.discard_target.take();
        if let Some((file_path, staged)) = target {
            if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
                let res = if file_path == "All Changes" {
                    repo::discard_all_changes(resolved)
                } else {
                    repo::discard_file_changes(resolved, &file_path, staged)
                };
                match res {
                    Ok(()) => {
                        self.status_message = Some(format!("Discarded: {}", file_path));
                        self.refresh_detail();
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Discard failed: {}", e));
                    }
                }
            }
        }
    }

    pub fn cancel_discard_changes(&mut self) {
        self.discard_target = None;
        self.mode = Mode::Detail;
    }

    pub fn cancel_commit_search(&mut self) {
        self.commit_list.search_query = None;
        self.clamp_commit_selection();
        self.status_list.file_selection = 0;
        self.diff.diff_scroll = 0;
        self.refresh_file_diff();
        self.mode = Mode::Detail;
    }

    /// Cancels commit input and returns to the detail view.
    pub fn cancel_commit(&mut self) {
        self.commit_popup.input_buffer.clear();
        self.commit_input_scroll = 0;
        self.commit_popup.maximized = false;
        self.mode = Mode::Detail;
    }

    /// Performs the git commit with the message in `input_buffer`.
    pub fn commit_git_changes(&mut self) {
        let msg = self.commit_popup.input_buffer.trim().to_string();
        if msg.is_empty() {
            self.status_message = Some("Commit message cannot be empty".to_string());
            self.mode = Mode::Detail;
            return;
        }

        let repo_path = match &self.current_detail {
            Some(ItemDetail::Repo { resolved, .. }) => Some(resolved.clone()),
            _ => None,
        };

        if let Some(path) = repo_path {
            let res = if self.commit_popup.amend {
                repo::commit_amend(&path, &msg)
            } else {
                repo::commit_changes(&path, &msg)
            };
            match res {
                Ok(()) => {
                    let success_msg = if self.commit_popup.amend {
                        "Amended commit successfully"
                    } else {
                        "Committed successfully"
                    };
                    self.status_message = Some(success_msg.to_string());
                    self.refresh_detail();
                    self.refresh_selected_status();
                }
                Err(e) => {
                    let fail_msg = if self.commit_popup.amend {
                        format!("Amend failed: {}", e)
                    } else {
                        format!("Commit failed: {}", e)
                    };
                    self.status_message = Some(fail_msg);
                }
            }
        }

        self.commit_popup.input_buffer.clear();
        self.commit_input_scroll = 0;
        self.commit_popup.maximized = false;
        self.mode = Mode::Detail;
    }
}
