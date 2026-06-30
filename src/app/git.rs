use super::*;

impl App {
    /// Spawns a background thread to pull the upstream remote branch into the selected local branch.
    /// Can only pull if the selected local branch is the currently checked-out branch.
    pub fn pull_selected_branch(&mut self) {
        if self.fetching {
            return;
        }
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
            if let Some(branch_info) =
                info.local_branches.get(self.branch_list.local_branch_selection)
            {
                if !branch_info.is_head {
                    self.status_message = Some(format!(
                        "Can only pull into the currently checked-out branch. Checkout '{}' first.",
                        branch_info.name
                    ));
                    return;
                }

                self.fetching = true;
                self.status_message = Some("Pulling...".to_string());

                let repo_path = resolved.clone();
                let branch_name = branch_info.name.clone();
                let tx = self.tx.clone();

                std::thread::spawn(move || {
                    let res = (|| -> Result<String, Box<dyn std::error::Error>> {
                        if !repo::has_upstream_remote(&repo_path, &branch_name) {
                            return Ok("No upstream tracking branch configured for this branch"
                                .to_string());
                        }

                        let _safe_branch = safe_ref(&branch_name)?;
                        let output = git_command()
                            .arg("pull")
                            .current_dir(&repo_path)
                            .output()?;

                        if output.status.success() {
                            Ok(format!("Pulled successfully for '{}'", branch_name))
                        } else {
                            let err_msg =
                                String::from_utf8_lossy(&output.stderr).trim().to_string();
                            Err(format!("git pull failed: {}", err_msg).into())
                        }
                    })();

                    let msg = match res {
                        Ok(success) => success,
                        Err(e) => format!("Pull failed: {}", e),
                    };
                    let _ = tx.send(msg);
                });
            }
        }
    }

    /// Spawns a background thread to push the selected local branch to its upstream remote.
    /// If no upstream is configured, it falls back to the first configured remote (typically origin)
    /// and sets upstream tracking (-u).
    /// Requests confirmation to push the selected local branch.
    /// If multiple remotes exist and no upstream is configured, opens the remote picker first.
    pub fn request_branch_push(&mut self) {
        if self.fetching {
            return;
        }
        if let Some(repo::ItemDetail::Repo { info, resolved }) = &self.current_detail {
            if let Some(branch_info) =
                info.local_branches.get(self.branch_list.local_branch_selection)
            {
                let branch_name = branch_info.name.clone();
                // Check if this branch already has a configured upstream remote.
                let has_upstream = repo::has_upstream_remote(resolved, &branch_name);

                if !has_upstream && info.remotes.len() > 1 {
                    // Multiple remotes, no upstream — ask user to pick.
                    self.branch_action_target = Some((branch_name, false));
                    self.remote_picker_action = Some(RemotePickerAction::PushBranch);
                    self.remote_picker_selection = 0;
                    self.mode = Mode::RemotePicker;
                } else {
                    self.branch_action_target = Some((branch_name, false));
                    self.mode = Mode::BranchPushConfirm;
                }
            }
        }
    }

    /// Confirms the push operation.
    pub fn confirm_branch_push(&mut self) {
        if let Some((branch_name, _)) = &self.branch_action_target {
            let branch_name = branch_name.clone();
            self.execute_branch_push(&branch_name);
        }
        self.branch_action_target = None;
        self.mode = Mode::Detail;
    }

    /// Cancels the push operation.
    pub fn cancel_branch_push(&mut self) {
        self.branch_action_target = None;
        self.mode = Mode::Detail;
    }

    /// Spawns a background thread to push the chosen local branch to its upstream remote.
    /// If no upstream is configured, it falls back to the first configured remote (typically origin)
    /// and sets upstream tracking (-u).
    pub fn execute_branch_push(&mut self, branch_name: &str) {
        if self.fetching {
            return;
        }
        if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
            let repo_path = resolved.clone();
            let branch_name = branch_name.to_string();

            let (remote_name, set_upstream) =
                match repo::get_branch_push_target(&repo_path, &branch_name) {
                    Some((name, set_up)) => (Some(name), set_up),
                    None => (None, false),
                };

            let remote_name = match remote_name {
                Some(name) => name,
                None => {
                    self.status_message =
                        Some("No remotes configured for this repository".to_string());
                    return;
                }
            };

            self.fetching = true;
            self.status_message =
                Some(format!("Pushing '{}' to '{}'...", branch_name, remote_name));

            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let res = (|| -> Result<String, Box<dyn std::error::Error>> {
                    let safe_remote = safe_ref(&remote_name)?;
                    let safe_branch = safe_ref(&branch_name)?;
                    let mut cmd = git_command();
                    cmd.arg("push");
                    if set_upstream {
                        cmd.arg("-u");
                    }
                    cmd.arg(safe_remote).arg(safe_branch).current_dir(&repo_path);

                    let output = cmd.output()?;

                    if output.status.success() {
                        Ok(format!("Pushed '{}' to '{}' successfully", branch_name, remote_name))
                    } else {
                        let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
                        Err(format!("git push failed: {}", err_msg).into())
                    }
                })();

                let msg = match res {
                    Ok(success) => success,
                    Err(e) => format!("Push failed: {}", e),
                };
                let _ = tx.send(msg);
            });
        }
    }

    pub fn request_branch_checkout(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            match self.detail_focus {
                DetailSection::LocalBranches => {
                    if let Some(branch_info) =
                        info.local_branches.get(self.branch_list.local_branch_selection)
                    {
                        if !branch_info.is_head {
                            self.branch_action_target = Some((branch_info.name.clone(), false));
                            self.mode = Mode::BranchCheckoutConfirm;
                        }
                    }
                }
                DetailSection::RemoteBranches => {
                    if let Some(branch_info) =
                        info.remote_branches.get(self.branch_list.remote_branch_selection)
                    {
                        self.branch_action_target = Some((branch_info.name.clone(), true));
                        self.mode = Mode::BranchCheckoutConfirm;
                    }
                }
                _ => {}
            }
        }
    }

    pub fn confirm_branch_checkout(&mut self) {
        if let Some((branch_name, is_remote)) = self.branch_action_target.take() {
            if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
                let res = if is_remote {
                    repo::checkout_remote_branch(resolved, &branch_name)
                } else {
                    repo::checkout_local_branch(resolved, &branch_name)
                        .map(|_| format!("Switched to branch '{}'", branch_name))
                };

                match res {
                    Ok(msg) => {
                        self.status_message = Some(msg);
                        self.branch_list.local_branch_selection = 0;
                        self.resync_detail();
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Checkout failed: {}", e));
                    }
                }
            }
        }
        self.mode = Mode::Detail;
    }

    pub fn cancel_branch_checkout(&mut self) {
        self.branch_action_target = None;
        self.mode = Mode::Detail;
    }

    pub fn start_tag_create(&mut self) {
        if self.detail_tab != 0 {
            return;
        }
        if self.detail_focus != DetailSection::Commits {
            return;
        }
        if self.is_uncommitted_selected() {
            self.status_message = Some("Cannot tag uncommitted changes".to_string());
            return;
        }
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let dirty = !info.changes.staged.is_empty()
                || !info.changes.unstaged.is_empty()
                || !info.changes.untracked.is_empty()
                || !info.changes.conflicted.is_empty();
            let commit_idx = if dirty {
                self.commit_list.selection.saturating_sub(1)
            } else {
                self.commit_list.selection
            };
            if let Some(commit) = info.commits.get(commit_idx) {
                self.tag_action_target_oid = Some(commit.oid.clone());
                self.commit_popup.input_buffer.clear();
                self.mode = Mode::TagCreateInput;
            }
        }
    }

    pub fn commit_tag_create(&mut self) {
        let tag_name = self.input_buffer.trim().to_string();
        if tag_name.is_empty() {
            self.status_message = Some("Tag name cannot be empty".to_string());
            self.mode = Mode::Detail;
            return;
        }
        if let Some(oid) = self.tag_action_target_oid.take() {
            if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
                match repo::create_tag(resolved, &tag_name, &oid) {
                    Ok(()) => {
                        self.status_message = Some(format!("Created tag '{}'", tag_name));
                        self.resync_detail();
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Failed to create tag: {}", e));
                    }
                }
            }
        }
        self.mode = Mode::Detail;
    }

    pub fn start_stash_create(&mut self) {
        self.commit_popup.input_buffer.clear();
        self.mode = Mode::StashCreateInput;
    }

    pub fn commit_stash_create(&mut self) {
        let stash_name = self.input_buffer.trim().to_string();
        if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
            match repo::save_stash(
                resolved,
                &stash_name,
                self.stash_untracked,
                self.stash_keep_index,
            ) {
                Ok(()) => {
                    let msg = if stash_name.is_empty() {
                        "Created stash".to_string()
                    } else {
                        format!("Created stash '{}'", stash_name)
                    };
                    self.status_message = Some(msg);
                    self.stash_list.stash_selection = 0;
                    self.stash_list.stash_file_selection = 0;
                    self.resync_detail();
                }
                Err(e) => {
                    self.set_error(format!("Failed to stash changes: {}", e));
                }
            }
        }
        self.mode = Mode::Detail;
    }

    pub fn start_remote_add(&mut self) {
        self.mode = Mode::RemoteAddNameInput;
        self.commit_popup.input_buffer.clear();
        self.remote_add_name.clear();
        self.remote_add_url.clear();
    }

    pub fn commit_remote_add_name(&mut self) {
        let trimmed = self.input_buffer.trim().to_string();
        self.commit_popup.input_buffer.clear();
        if trimmed.is_empty() {
            self.mode = Mode::Detail;
            return;
        }
        self.remote_add_name = trimmed;
        self.mode = Mode::RemoteAddUrlInput;
    }

    pub fn commit_remote_add_url(&mut self) {
        let trimmed = self.input_buffer.trim().to_string();
        self.commit_popup.input_buffer.clear();
        self.mode = Mode::Detail;
        if trimmed.is_empty() {
            return;
        }
        self.remote_add_url = trimmed;

        if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
            match repo::remote_add(resolved, &self.remote_add_name, &self.remote_add_url) {
                Ok(_) => {
                    self.status_message =
                        Some(format!("Remote '{}' added successfully", self.remote_add_name));
                    self.resync_detail();
                }
                Err(e) => {
                    self.set_error(format!("Failed to add remote: {}", e));
                }
            }
        }
    }

    pub fn request_remote_delete(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if let Some(remote_info) = info.remotes.get(self.branch_list.remote_selection) {
                self.remote_action_target = Some(remote_info.name.clone());
                self.mode = Mode::RemoteDeleteConfirm;
            }
        }
    }

    pub fn confirm_remote_delete(&mut self) {
        if let Some(remote_name) = self.remote_action_target.take() {
            if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
                match repo::remote_delete(resolved, &remote_name) {
                    Ok(_) => {
                        self.status_message = Some(format!("Remote '{}' removed", remote_name));
                        self.branch_list.remote_selection = 0;
                        self.resync_detail();
                    }
                    Err(e) => {
                        self.set_error(format!("Failed to remove remote: {}", e));
                    }
                }
            }
        }
        self.mode = Mode::Detail;
    }

    pub fn request_tag_delete(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if let Some(tag_info) = info.local_tags.get(self.tag_list.local_tag_selection) {
                let is_on_remote = !info.remotes.is_empty();
                self.tag_delete_target = Some((tag_info.name.clone(), is_on_remote));
                self.mode = Mode::TagDeleteConfirm;
            }
        }
    }

    pub fn confirm_tag_delete(&mut self) {
        if let Some((tag_name, is_on_remote)) = self.tag_delete_target.take() {
            let (repo_path, remotes_len, first_remote) = if let Some(repo::ItemDetail::Repo {
                resolved,
                info,
            }) = &self.current_detail
            {
                (resolved.clone(), info.remotes.len(), info.remotes.first().map(|r| r.name.clone()))
            } else {
                return;
            };

            match repo::delete_tag(&repo_path, &tag_name) {
                Ok(()) => {
                    self.status_message = Some(format!("Deleted local tag '{}'", tag_name));
                    self.tag_list.local_tag_selection = 0;
                    self.resync_detail();

                    if is_on_remote {
                        if remotes_len > 1 {
                            // Ask which remote to delete from.
                            self.tag_delete_target = Some((tag_name, true));
                            self.remote_picker_action = Some(RemotePickerAction::DeleteRemoteTag);
                            self.remote_picker_selection = 0;
                            self.mode = Mode::RemotePicker;
                            return;
                        } else if let Some(remote_name) = first_remote {
                            self.execute_delete_remote_tag_on(&tag_name, &remote_name);
                        }
                    }
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to delete tag: {}", e));
                }
            }
        }
        self.mode = Mode::Detail;
    }

    pub fn cancel_tag_delete(&mut self) {
        self.tag_delete_target = None;
        self.mode = Mode::Detail;
    }

    pub fn request_tag_push(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if let Some(tag_info) = info.local_tags.get(self.tag_list.local_tag_selection) {
                let is_on_remote = if info.remotes.is_empty() {
                    false
                } else if info.remote_tags_loaded {
                    info.remote_tags.iter().any(|rt| rt.name == tag_info.name)
                } else {
                    false
                };
                if is_on_remote {
                    self.status_message =
                        Some(format!("Tag '{}' is already on the remote", tag_info.name));
                    return;
                }
                if info.remotes.len() > 1 {
                    self.tag_push_target = Some(tag_info.name.clone());
                    self.remote_picker_action = Some(RemotePickerAction::PushTag);
                    self.remote_picker_selection = 0;
                    self.mode = Mode::RemotePicker;
                } else {
                    self.tag_push_target = Some(tag_info.name.clone());
                    self.mode = Mode::TagPushConfirm;
                }
            }
        }
    }

    pub fn confirm_tag_push(&mut self) {
        if let Some(tag_name) = self.tag_push_target.take() {
            if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
                let repo_path = resolved.clone();
                let remote_name = match info.remotes.first().map(|r| r.name.clone()) {
                    Some(name) => name,
                    None => {
                        self.status_message =
                            Some("No remotes configured for this repository".to_string());
                        self.mode = Mode::Detail;
                        return;
                    }
                };

                self.fetching = true;
                self.status_message =
                    Some(format!("Pushing tag '{}' to '{}'...", tag_name, remote_name));
                let tx = self.tx.clone();
                std::thread::spawn(move || {
                    let safe_remote = match safe_ref(&remote_name) {
                        Ok(r) => r,
                        Err(e) => {
                            let _ = tx.send(format!("Invalid remote: {}", e));
                            return;
                        }
                    };
                    let safe_tag = match safe_ref(&tag_name) {
                        Ok(t) => t,
                        Err(e) => {
                            let _ = tx.send(format!("Invalid tag: {}", e));
                            return;
                        }
                    };
                    let mut cmd = git_command();
                    cmd.arg("push").arg(safe_remote).arg(safe_tag).current_dir(&repo_path);

                    let output = match cmd.output() {
                        Ok(o) => o,
                        Err(e) => {
                            let _ = tx.send(format!("Failed to run git push: {}", e));
                            return;
                        }
                    };

                    if output.status.success() {
                        let _ = tx.send(format!(
                            "Pushed tag '{}' to '{}' successfully",
                            tag_name, remote_name
                        ));
                    } else {
                        let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
                        let _ = tx.send(format!("Failed to push tag: {}", err_msg));
                    }
                });
            }
        }
        self.mode = Mode::Detail;
    }

    pub fn cancel_tag_push(&mut self) {
        self.tag_push_target = None;
        self.mode = Mode::Detail;
    }

    pub fn request_tag_push_all(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if info.remotes.is_empty() {
                self.status_message = Some("No remotes configured for this repository".to_string());
                return;
            }
            if info.remotes.len() > 1 {
                self.remote_picker_action = Some(RemotePickerAction::PushAllTags);
                self.remote_picker_selection = 0;
                self.mode = Mode::RemotePicker;
            } else {
                let remote_name = info
                    .remotes
                    .first()
                    .map(|r| r.name.clone())
                    .unwrap_or_else(|| "origin".to_string());
                self.remote_action_target = Some(remote_name);
                self.mode = Mode::TagPushAllConfirm;
            }
        }
    }

    pub fn confirm_tag_push_all(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let remote_name = match self
                .remote_action_target
                .take()
                .or_else(|| info.remotes.first().map(|r| r.name.clone()))
            {
                Some(name) => name,
                None => {
                    self.status_message =
                        Some("No remotes configured for this repository".to_string());
                    self.mode = Mode::Detail;
                    return;
                }
            };
            self.execute_tag_push_all_to(&remote_name);
        }
        self.mode = Mode::Detail;
    }

    pub fn cancel_tag_push_all(&mut self) {
        self.remote_action_target = None;
        self.mode = Mode::Detail;
    }

    pub fn start_branch_create(&mut self) {
        if let Some(repo::ItemDetail::Repo { .. }) = &self.current_detail {
            self.commit_popup.input_buffer.clear();
            self.mode = Mode::BranchCreateInput;
        }
    }

    pub fn commit_branch_create(&mut self) {
        let branch_name = self.input_buffer.trim().to_string();
        if branch_name.is_empty() {
            self.status_message = Some("Branch name cannot be empty".to_string());
            self.mode = Mode::Detail;
            return;
        }

        if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
            match repo::create_branch(resolved, &branch_name) {
                Ok(()) => {
                    match repo::checkout_local_branch(resolved, &branch_name) {
                        Ok(()) => {
                            self.status_message =
                                Some(format!("Created and switched to branch '{}'", branch_name));
                        }
                        Err(e) => {
                            self.status_message = Some(format!(
                                "Created branch '{}', but checkout failed: {}",
                                branch_name, e
                            ));
                        }
                    }
                    self.branch_list.local_branch_selection = 0;
                    self.resync_detail();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to create branch: {}", e));
                }
            }
        }
        self.commit_popup.input_buffer.clear();
        self.mode = Mode::Detail;
    }

    pub fn cancel_branch_create(&mut self) {
        self.commit_popup.input_buffer.clear();
        self.mode = Mode::Detail;
    }

    pub fn request_branch_delete(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            match self.detail_focus {
                DetailSection::LocalBranches => {
                    if let Some(branch_info) =
                        info.local_branches.get(self.branch_list.local_branch_selection)
                    {
                        if branch_info.is_head {
                            self.status_message =
                                Some("Cannot delete the currently checked out branch".to_string());
                            return;
                        }
                        self.branch_action_target = Some((branch_info.name.clone(), false));
                        self.mode = Mode::BranchDeleteConfirm;
                    }
                }
                DetailSection::RemoteBranches => {
                    if let Some(branch_info) =
                        info.remote_branches.get(self.branch_list.remote_branch_selection)
                    {
                        self.branch_action_target = Some((branch_info.name.clone(), true));
                        self.mode = Mode::BranchDeleteConfirm;
                    }
                }
                _ => {}
            }
        }
    }

    pub fn confirm_branch_delete(&mut self) {
        if let Some((branch_name, is_remote)) = self.branch_action_target.take() {
            if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
                let res = if is_remote {
                    repo::delete_remote_branch(resolved, &branch_name)
                } else {
                    repo::delete_local_branch(resolved, &branch_name)
                };

                match res {
                    Ok(()) => {
                        self.status_message = Some(format!("Deleted branch '{}'", branch_name));
                        self.branch_list.local_branch_selection = 0;
                        self.branch_list.remote_branch_selection = 0;
                        self.resync_detail();
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Failed to delete branch: {}", e));
                    }
                }
            }
        }
        self.mode = Mode::Detail;
    }

    pub fn cancel_branch_delete(&mut self) {
        self.branch_action_target = None;
        self.mode = Mode::Detail;
    }

    pub fn request_branch_merge(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            match self.detail_focus {
                DetailSection::LocalBranches => {
                    if let Some(branch_info) =
                        info.local_branches.get(self.branch_list.local_branch_selection)
                    {
                        self.branch_action_target = Some((branch_info.name.clone(), false));
                        self.mode = Mode::BranchMergeConfirm;
                    }
                }
                DetailSection::RemoteBranches => {
                    if let Some(branch_info) =
                        info.remote_branches.get(self.branch_list.remote_branch_selection)
                    {
                        self.branch_action_target = Some((branch_info.name.clone(), true));
                        self.mode = Mode::BranchMergeConfirm;
                    }
                }
                _ => {}
            }
        }
    }

    pub fn confirm_branch_merge(&mut self) {
        let target = self.branch_action_target.take();
        self.mode = Mode::Detail;

        if let Some((branch_name, is_remote)) = target {
            if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
                // Find current checked out branch name (HEAD)
                let current_branch = match info.local_branches.iter().find(|b| b.is_head) {
                    Some(b) => b.name.clone(),
                    None => {
                        self.status_message = Some(
                            "No checked-out branch (detached HEAD). Cannot merge.".to_string(),
                        );
                        return;
                    }
                };

                // Can't merge a branch into itself
                if !is_remote && branch_name == current_branch {
                    self.status_message = Some("Cannot merge a branch into itself.".to_string());
                    return;
                }

                self.fetching = true;
                self.status_message =
                    Some(format!("Merging '{}' into '{}'...", branch_name, current_branch));

                let repo_path = resolved.clone();
                let target_name = branch_name.clone();
                let tx = self.tx.clone();

                std::thread::spawn(move || {
                    let res = (|| -> Result<String, Box<dyn std::error::Error>> {
                        let safe_target = safe_ref(&target_name)?;
                        let output = git_command()
                            .arg("merge")
                            .arg(safe_target)
                            .current_dir(&repo_path)
                            .output()?;

                        if output.status.success() {
                            Ok(format!(
                                "Merged '{}' into '{}' successfully",
                                target_name, current_branch
                            ))
                        } else {
                            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                            let mut err_msg = if !stderr.is_empty() { stderr } else { stdout };
                            if err_msg.contains("CONFLICT") {
                                err_msg = "Merge conflicts detected. Please resolve conflicts."
                                    .to_string();
                            }
                            Err(format!("git merge failed: {}", err_msg).into())
                        }
                    })();

                    let msg = match res {
                        Ok(success) => success,
                        Err(e) => format!("Merge failed: {}", e),
                    };
                    let _ = tx.send(msg);
                });
            }
        }
    }

    pub fn cancel_branch_merge(&mut self) {
        self.branch_action_target = None;
        self.mode = Mode::Detail;
    }

    pub fn request_branch_rebase(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if self.detail_focus == DetailSection::LocalBranches {
                if let Some(branch_info) =
                    info.local_branches.get(self.branch_list.local_branch_selection)
                {
                    if !branch_info.is_head {
                        self.branch_action_target = Some((branch_info.name.clone(), false));
                        self.mode = Mode::BranchRebaseConfirm;
                    }
                }
            } else if self.detail_focus == DetailSection::RemoteBranches {
                if let Some(branch_info) =
                    info.remote_branches.get(self.branch_list.remote_branch_selection)
                {
                    self.branch_action_target = Some((branch_info.name.clone(), true));
                    self.mode = Mode::BranchRebaseConfirm;
                }
            }
        }
    }

    pub fn confirm_branch_rebase(&mut self) {
        let target = self.branch_action_target.take();
        self.mode = Mode::Detail;

        if let Some((branch_name, is_remote)) = target {
            if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
                // Find current checked out branch name (HEAD)
                let current_branch = match info.local_branches.iter().find(|b| b.is_head) {
                    Some(b) => b.name.clone(),
                    None => {
                        self.status_message = Some(
                            "No checked-out branch (detached HEAD). Cannot rebase.".to_string(),
                        );
                        return;
                    }
                };

                // Can't rebase a branch onto itself
                if !is_remote && branch_name == current_branch {
                    self.status_message = Some("Cannot rebase a branch onto itself.".to_string());
                    return;
                }

                self.fetching = true;
                self.status_message =
                    Some(format!("Rebasing '{}' onto '{}'...", current_branch, branch_name));

                let repo_path = resolved.clone();
                let target_name = branch_name.clone();
                let tx = self.tx.clone();

                std::thread::spawn(move || {
                    let res = (|| -> Result<String, Box<dyn std::error::Error>> {
                        let safe_target = safe_ref(&target_name)?;
                        let output = git_command()
                            .arg("rebase")
                            .arg(safe_target)
                            .current_dir(&repo_path)
                            .output()?;

                        if output.status.success() {
                            Ok(format!(
                                "Rebased '{}' onto '{}' successfully",
                                current_branch, target_name
                            ))
                        } else {
                            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                            let mut err_msg = if !stderr.is_empty() { stderr } else { stdout };
                            if err_msg.contains("CONFLICT") || err_msg.contains("conflict") {
                                err_msg = "Rebase conflicts detected. Please resolve in terminal (git rebase --continue/--abort).".to_string();
                            }
                            Err(format!("git rebase failed: {}", err_msg).into())
                        }
                    })();

                    let msg = match res {
                        Ok(success) => success,
                        Err(e) => format!("Rebase failed: {}", e),
                    };
                    let _ = tx.send(msg);
                });
            }
        }
    }

    pub fn cancel_branch_rebase(&mut self) {
        self.branch_action_target = None;
        self.mode = Mode::Detail;
    }

    pub fn request_branch_interactive_rebase(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if self.detail_focus == DetailSection::LocalBranches {
                if let Some(branch_info) =
                    info.local_branches.get(self.branch_list.local_branch_selection)
                {
                    if !branch_info.is_head {
                        self.branch_action_target = Some((branch_info.name.clone(), false));
                        self.mode = Mode::BranchInteractiveRebaseConfirm;
                    }
                }
            } else if self.detail_focus == DetailSection::RemoteBranches {
                if let Some(branch_info) =
                    info.remote_branches.get(self.branch_list.remote_branch_selection)
                {
                    self.branch_action_target = Some((branch_info.name.clone(), true));
                    self.mode = Mode::BranchInteractiveRebaseConfirm;
                }
            }
        }
    }

    pub fn confirm_branch_interactive_rebase(&mut self) {
        let target = self.branch_action_target.take();
        self.mode = Mode::Detail;

        if let Some((branch_name, _is_remote)) = target {
            if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
                self.pending_interactive_rebase = Some((resolved.clone(), branch_name));
            }
        }
    }

    pub fn cancel_branch_interactive_rebase(&mut self) {
        self.branch_action_target = None;
        self.mode = Mode::Detail;
    }

    pub fn run_interactive_rebase(&mut self) {
        if self.is_uncommitted_selected() {
            self.status_message =
                Some("Cannot run interactive rebase on <uncommitted> row.".to_string());
            return;
        }
        let params = match &self.current_detail {
            Some(repo::ItemDetail::Repo { resolved, info }) => {
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                let commit_idx = if dirty {
                    self.commit_list.selection.saturating_sub(1)
                } else {
                    self.commit_list.selection
                };
                info.commits.get(commit_idx).map(|c| (resolved.clone(), c.oid.clone()))
            }
            _ => None,
        };

        if let Some((repo_path, commit_oid)) = params {
            // Check if the commit is root using git2
            let is_root = repo::is_root_commit(&repo_path, &commit_oid);

            let target = if is_root { "--root".to_string() } else { format!("{}~1", commit_oid) };
            self.pending_interactive_rebase = Some((repo_path, target));
        }
    }

    /// Total files in the currently-focused Staged or Unstaged sub-panel.
    pub fn staging_file_total(&self) -> usize {
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => match self.detail_focus {
                DetailSection::Staged => info.changes.staged.len(),
                DetailSection::Unstaged => info.changes.unstaged.len(),
                DetailSection::Conflicts => info.changes.conflicted.len(),
                _ => 0,
            },
            _ => 0,
        }
    }

    /// Confirm and abort the in-progress merge.
    pub fn confirm_abort_merge(&mut self) {
        self.mode = Mode::Detail;
        let repo_path = match &self.current_detail {
            Some(ItemDetail::Repo { resolved, .. }) => Some(resolved.clone()),
            _ => None,
        };
        if let Some(repo_path) = repo_path {
            match repo::abort_merge(&repo_path) {
                Ok(()) => {
                    self.status_message = Some("Merge aborted successfully".to_string());
                    self.refresh_detail();
                    if self.is_conflicted_empty() {
                        self.detail_focus = DetailSection::Unstaged;
                    }
                    self.refresh_staging_diff();
                }
                Err(e) => self.status_message = Some(format!("Abort merge failed: {}", e)),
            }
        }
    }

    /// Confirm and continue the in-progress merge.
    pub fn confirm_continue_merge(&mut self) {
        self.mode = Mode::Detail;
        let repo_path = match &self.current_detail {
            Some(ItemDetail::Repo { resolved, .. }) => Some(resolved.clone()),
            _ => None,
        };
        if let Some(repo_path) = repo_path {
            match repo::continue_merge(&repo_path) {
                Ok(()) => {
                    self.status_message = Some("Merge continued successfully".to_string());
                    self.refresh_detail();
                    if self.is_conflicted_empty() {
                        self.detail_focus = DetailSection::Unstaged;
                    }
                    self.refresh_staging_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Merge continue failed: {}", e));
                    self.refresh_detail();
                    self.refresh_staging_diff();
                }
            }
        }
    }

    pub fn fetch_remote_tags(&mut self, show_progress: bool) {
        if self.fetching {
            return;
        }
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &mut self.current_detail {
            info.remote_tags_attempted = true;
            // Use the currently selected remote in the Remotes tab if available,
            // otherwise fall back to the first remote.
            let remote = info
                .remotes
                .get(self.branch_list.remote_selection)
                .or_else(|| info.remotes.first());
            if let Some(remote) = remote {
                let repo_path = resolved.clone();
                let remote_name = remote.name.clone();
                let tx = self.tx.clone();
                if show_progress {
                    self.fetching = true;
                    self.status_message = Some(format!("Fetching tags from '{}'...", remote_name));
                }
                std::thread::spawn(move || match repo::get_remote_tags(&repo_path, &remote_name) {
                    Ok(tags) => {
                        let serialized = repo::serialize_tags(&tags);
                        let _ = tx.send(format!("REMOTE_TAGS:{}", serialized));
                    }
                    Err(e) => {
                        let _ =
                            tx.send(format!("REMOTE_TAGS_ERR:Failed to get remote tags: {}", e));
                    }
                });
            }
        }
    }

    /// Spawns a background thread to fetch all updates (git fetch) from the specified remote.
    pub fn fetch_remote(&mut self, remote_name: &str) {
        if self.fetching {
            return;
        }
        if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
            self.fetching = true;
            self.status_message = Some(format!("Fetching remote '{}'...", remote_name));

            let repo_path = resolved.clone();
            let remote_name = remote_name.to_string();
            let tx = self.tx.clone();

            std::thread::spawn(move || {
                let res = (|| -> Result<String, Box<dyn std::error::Error>> {
                    let safe_remote = safe_ref(&remote_name)?;
                    let output = git_command()
                        .arg("fetch")
                        .arg(safe_remote)
                        .current_dir(&repo_path)
                        .output()?;

                    if output.status.success() {
                        Ok(format!("Fetched remote '{}' successfully", remote_name))
                    } else {
                        let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
                        Err(format!("git fetch failed: {}", err_msg).into())
                    }
                })();

                let msg = match res {
                    Ok(success) => success,
                    Err(e) => format!("Fetch failed: {}", e),
                };
                let _ = tx.send(msg);
            });
        }
    }

    /// Confirm the remote picker selection and proceed with the queued action.
    pub fn confirm_remote_picker(&mut self) {
        let action = match self.remote_picker_action.take() {
            Some(a) => a,
            None => {
                self.mode = Mode::Detail;
                return;
            }
        };
        let remote_name = if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            info.remotes.get(self.remote_picker_selection).map(|r| r.name.clone())
        } else {
            None
        };
        let remote_name = match remote_name {
            Some(n) => n,
            None => {
                self.mode = Mode::Detail;
                return;
            }
        };

        match action {
            RemotePickerAction::PushBranch => {
                // Override the execute path by injecting the chosen remote directly.
                self.mode = Mode::BranchPushConfirm;
                // Store picked remote in branch_action_target second field (reused as remote override).
                if let Some((ref name, _)) = self.branch_action_target.clone() {
                    self.execute_branch_push_to(name, &remote_name);
                }
                self.branch_action_target = None;
                self.mode = Mode::Detail;
            }
            RemotePickerAction::PushTag => {
                if let Some(tag_name) = self.tag_push_target.take() {
                    self.execute_tag_push_to(&tag_name, &remote_name);
                }
                self.mode = Mode::Detail;
            }
            RemotePickerAction::PushAllTags => {
                self.remote_action_target = Some(remote_name);
                self.mode = Mode::TagPushAllConfirm;
            }
            RemotePickerAction::DeleteRemoteTag => {
                if let Some((tag_name, _)) = self.tag_delete_target.take() {
                    self.execute_delete_remote_tag_on(&tag_name, &remote_name);
                }
                self.mode = Mode::Detail;
            }
            RemotePickerAction::FetchRemote => {
                self.branch_list.remote_selection = self.remote_picker_selection;
                self.fetch_remote(&remote_name);
                self.mode = Mode::Detail;
            }
        }
    }

    pub fn cancel_remote_picker(&mut self) {
        self.remote_picker_action = None;
        self.branch_action_target = None;
        self.tag_push_target = None;
        self.tag_delete_target = None;
        self.mode = Mode::Detail;
    }

    /// Force-dismiss a stuck progress popup.
    /// The background thread may still be running; any result it eventually
    /// sends will be received and silently dropped or displayed as a status
    /// message. The UI is unblocked immediately.
    pub fn dismiss_fetch(&mut self) {
        self.fetching = false;
        self.status_message =
            Some("Operation dismissed (may still be running in background)".to_string());
    }

    /// Push a branch to a specific remote by name (bypasses upstream detection).
    fn execute_branch_push_to(&mut self, branch_name: &str, remote_name: &str) {
        if self.fetching {
            return;
        }
        if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
            let repo_path = resolved.clone();
            let branch_name = branch_name.to_string();
            let remote_name = remote_name.to_string();
            self.fetching = true;
            self.status_message =
                Some(format!("Pushing '{}' to '{}'...", branch_name, remote_name));
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let safe_remote = match safe_ref(&remote_name) {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = tx.send(format!("Invalid remote: {}", e));
                        return;
                    }
                };
                let safe_branch = match safe_ref(&branch_name) {
                    Ok(b) => b,
                    Err(e) => {
                        let _ = tx.send(format!("Invalid branch: {}", e));
                        return;
                    }
                };
                let mut cmd = git_command();
                cmd.arg("push")
                    .arg("-u")
                    .arg(safe_remote)
                    .arg(safe_branch)
                    .current_dir(&repo_path);
                let output = match cmd.output() {
                    Ok(o) => o,
                    Err(e) => {
                        let _ = tx.send(format!("Failed to run git push: {}", e));
                        return;
                    }
                };
                if output.status.success() {
                    let _ = tx.send(format!(
                        "Pushed '{}' to '{}' successfully",
                        branch_name, remote_name
                    ));
                } else {
                    let _ = tx.send(format!(
                        "Failed to push: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    ));
                }
            });
        }
    }

    /// Push a single tag to a specific remote.
    fn execute_tag_push_to(&mut self, tag_name: &str, remote_name: &str) {
        if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
            let repo_path = resolved.clone();
            let tag_name = tag_name.to_string();
            let remote_name = remote_name.to_string();
            self.fetching = true;
            self.status_message =
                Some(format!("Pushing tag '{}' to '{}'...", tag_name, remote_name));
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let safe_remote = match safe_ref(&remote_name) {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = tx.send(format!("Invalid remote: {}", e));
                        return;
                    }
                };
                let safe_tag = match safe_ref(&tag_name) {
                    Ok(t) => t,
                    Err(e) => {
                        let _ = tx.send(format!("Invalid tag: {}", e));
                        return;
                    }
                };
                let mut cmd = git_command();
                cmd.arg("push").arg(safe_remote).arg(safe_tag).current_dir(&repo_path);
                let output = match cmd.output() {
                    Ok(o) => o,
                    Err(e) => {
                        let _ = tx.send(format!("Failed to run git push: {}", e));
                        return;
                    }
                };
                if output.status.success() {
                    let _ = tx.send(format!(
                        "Pushed tag '{}' to '{}' successfully",
                        tag_name, remote_name
                    ));
                } else {
                    let _ = tx.send(format!(
                        "Failed to push tag: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    ));
                }
            });
        }
    }

    /// Push all tags to a specific remote.
    fn execute_tag_push_all_to(&mut self, remote_name: &str) {
        if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
            let repo_path = resolved.clone();
            let remote_name = remote_name.to_string();
            self.fetching = true;
            self.status_message = Some(format!("Pushing all tags to '{}'...", remote_name));
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let safe_remote = match safe_ref(&remote_name) {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = tx.send(format!("Invalid remote: {}", e));
                        return;
                    }
                };
                let mut cmd = git_command();
                cmd.arg("push").arg(safe_remote).arg("--tags").current_dir(&repo_path);
                let output = match cmd.output() {
                    Ok(o) => o,
                    Err(e) => {
                        let _ = tx.send(format!("Failed to run git push: {}", e));
                        return;
                    }
                };
                if output.status.success() {
                    let _ = tx.send(format!("Pushed all tags to '{}' successfully", remote_name));
                } else {
                    let _ = tx.send(format!(
                        "Failed to push tags: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    ));
                }
            });
        }
    }

    /// Delete a remote tag on a specific remote.
    fn execute_delete_remote_tag_on(&mut self, tag_name: &str, remote_name: &str) {
        let repo_path = if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail
        {
            resolved.clone()
        } else {
            return;
        };
        let tag_name = tag_name.to_string();
        let remote_name = remote_name.to_string();
        let tx = self.tx.clone();
        self.fetching = true;
        self.status_message = Some(format!("Deleting remote tag '{}'...", tag_name));
        std::thread::spawn(move || {
            match repo::delete_remote_tag(&repo_path, &remote_name, &tag_name) {
                Ok(()) => {
                    let _ = tx.send(format!("Deleted remote tag '{}'", tag_name));
                }
                Err(e) => {
                    let _ = tx.send(format!("Failed to delete remote tag: {}", e));
                }
            }
        });
    }

    pub fn request_stash_delete(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if info.stashes.get(self.stash_list.stash_selection).is_some() {
                self.mode = Mode::StashDeleteConfirm;
            }
        }
    }

    pub fn confirm_stash_delete(&mut self) {
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
            if let Some(stash) = info.stashes.get(self.stash_list.stash_selection) {
                let index_to_delete = stash.index;
                match repo::delete_stash(resolved, index_to_delete) {
                    Ok(()) => {
                        self.status_message =
                            Some(format!("Deleted stash@{{{}}}", index_to_delete));
                        self.stash_list.stash_selection = 0;
                        self.stash_list.stash_file_selection = 0;
                        self.resync_detail();
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Failed to delete stash: {}", e));
                    }
                }
            }
        }
        self.mode = Mode::Detail;
    }

    pub fn cancel_stash_delete(&mut self) {
        self.mode = Mode::Detail;
    }

    pub fn request_stash_apply(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if info.stashes.get(self.stash_list.stash_selection).is_some() {
                self.stash_apply_delete_after = true;
                self.mode = Mode::StashApplyConfirm;
            }
        }
    }

    pub fn confirm_stash_apply(&mut self) {
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
            if let Some(stash) = info.stashes.get(self.stash_list.stash_selection) {
                let index_to_apply = stash.index;
                match repo::apply_stash(resolved, index_to_apply) {
                    Ok(()) => {
                        let mut success_msg = format!("Applied stash@{{{}}}", index_to_apply);
                        if self.stash_apply_delete_after {
                            match repo::delete_stash(resolved, index_to_apply) {
                                Ok(()) => {
                                    success_msg.push_str(" and deleted it");
                                }
                                Err(e) => {
                                    success_msg
                                        .push_str(&format!(", but failed to delete it: {}", e));
                                }
                            }
                        }
                        self.status_message = Some(success_msg);
                        self.stash_list.stash_selection = 0;
                        self.stash_list.stash_file_selection = 0;
                        self.resync_detail();
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Failed to apply stash: {}", e));
                    }
                }
            }
        }
        self.mode = Mode::Detail;
    }

    pub fn toggle_stash_apply_delete(&mut self) {
        self.stash_apply_delete_after = !self.stash_apply_delete_after;
    }

    pub fn cancel_stash_apply(&mut self) {
        self.mode = Mode::Detail;
    }

    pub fn request_tag_checkout(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if let Some(tag_info) = info.local_tags.get(self.tag_list.local_tag_selection) {
                self.tag_checkout_target = Some(tag_info.name.clone());
                self.mode = Mode::TagCheckoutConfirm;
            }
        }
    }

    pub fn confirm_tag_checkout(&mut self) {
        if let Some(tag_name) = self.tag_checkout_target.take() {
            if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
                match repo::checkout_tag(resolved, &tag_name) {
                    Ok(()) => {
                        self.status_message =
                            Some(format!("Checked out tag '{}' (detached HEAD)", tag_name));
                        self.resync_detail();
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Failed to checkout tag: {}", e));
                    }
                }
            }
        }
        self.mode = Mode::Detail;
    }

    pub fn cancel_tag_checkout(&mut self) {
        self.tag_checkout_target = None;
        self.mode = Mode::Detail;
    }

    pub fn commit_worktree_add_branch(&mut self) {
        let branch = self.input_buffer.trim().to_string();
        if branch.is_empty() {
            self.status_message = Some("Branch name cannot be empty".to_string());
            self.mode = Mode::Detail;
            return;
        }
        self.worktree_add_branch = branch;
        self.input_buffer.clear();
        self.mode = Mode::WorktreeAddPathInput;
    }

    pub fn commit_worktree_add_path(&mut self) {
        let path_str = self.input_buffer.trim().to_string();
        if path_str.is_empty() {
            self.status_message = Some("Path cannot be empty".to_string());
            self.mode = Mode::Detail;
            return;
        }
        let wt_path = repo::expand_tilde(&path_str);
        let resolved_path = match &self.current_detail {
            Some(repo::ItemDetail::Repo { resolved, .. }) => resolved.clone(),
            _ => {
                self.mode = Mode::Detail;
                return;
            }
        };
        match repo::worktree_add(&resolved_path, &self.worktree_add_branch, &wt_path) {
            Ok(_) => {
                self.status_message = Some("Worktree added successfully".to_string());
                self.resync_detail();
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to add worktree: {}", e));
            }
        }
        self.input_buffer.clear();
        self.mode = Mode::Detail;
    }

    pub fn commit_worktree_lock_reason(&mut self) {
        let reason = self.input_buffer.trim().to_string();
        let resolved_path = match &self.current_detail {
            Some(repo::ItemDetail::Repo { resolved, .. }) => resolved.clone(),
            _ => {
                self.mode = Mode::Detail;
                return;
            }
        };
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if let repo::TabData::Loaded(wts) = &info.worktrees {
                if let Some(wt) = wts.get(self.worktree_selection) {
                    match repo::worktree_lock(&resolved_path, &wt.name, &reason) {
                        Ok(_) => {
                            self.status_message = Some("Worktree locked successfully".to_string());
                            self.resync_detail();
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Failed to lock: {}", e));
                        }
                    }
                }
            }
        }
        self.input_buffer.clear();
        self.mode = Mode::Detail;
    }

    pub fn remove_worktree(&mut self, delete_folder: bool) {
        let resolved_path = match &self.current_detail {
            Some(repo::ItemDetail::Repo { resolved, .. }) => resolved.clone(),
            _ => {
                self.mode = Mode::Detail;
                return;
            }
        };
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if let repo::TabData::Loaded(wts) = &info.worktrees {
                if let Some(wt) = wts.get(self.worktree_selection) {
                    if wt.is_locked {
                        let _ = repo::worktree_unlock(&resolved_path, &wt.name);
                    }

                    let wt_path = wt.path.clone();
                    match repo::worktree_remove(&resolved_path, &wt.name, true) {
                        Ok(_) => {
                            if delete_folder && wt_path.exists() {
                                if let Err(e) = std::fs::remove_dir_all(&wt_path) {
                                    self.status_message = Some(format!(
                                        "Worktree removed, but failed to delete directory: {}",
                                        e
                                    ));
                                } else {
                                    self.status_message =
                                        Some("Worktree metadata and directory removed".to_string());
                                }
                            } else {
                                self.status_message = Some("Worktree metadata removed".to_string());
                            }
                            self.resync_detail();
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Failed to remove worktree: {}", e));
                        }
                    }
                }
            }
        }
        self.mode = Mode::Detail;
    }

    pub fn commit_worktree_remove(&mut self) {
        let choice = self.input_buffer.trim().to_string();
        self.input_buffer.clear();
        self.mode = Mode::Detail;

        match choice.as_str() {
            "1" => {
                self.remove_worktree(false);
            }
            "2" => {
                self.remove_worktree(true);
            }
            _ => {
                self.status_message =
                    Some("Invalid selection. Type 1 or 2 to remove worktree.".to_string());
            }
        }
    }

    pub fn commit_submodule_add_url(&mut self) {
        self.submodule_add_url = self.input_buffer.trim().to_string();
        self.input_buffer.clear();
        self.mode = Mode::SubmoduleAddPathInput;
    }

    pub fn commit_submodule_add_path(&mut self) {
        self.submodule_add_path = self.input_buffer.trim().to_string();
        self.input_buffer.clear();
        self.start_submodule_add();
    }

    pub fn start_submodule_add(&mut self) {
        let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail else {
            self.mode = Mode::Detail;
            return;
        };
        let repo_path = resolved.clone();
        let url = self.submodule_add_url.clone();
        let path = self.submodule_add_path.clone();

        self.fetching = true;
        self.status_message = Some(format!("Adding submodule '{}'...", path));
        self.mode = Mode::Detail;

        let tx = self.tx.clone();

        std::thread::spawn(move || {
            let trimmed_url = url.trim().to_lowercase();
            if trimmed_url.contains("ext:") || trimmed_url.contains("fd:") {
                let _ = tx.send("Error: Malicious URL protocol rejected".to_string());
                return;
            }
            let mut cmd = git_command();
            cmd.arg("submodule").arg("add").arg("--").arg(&url).arg(&path).current_dir(&repo_path);

            match cmd.output() {
                Ok(out) if out.status.success() => {
                    let _ = git_command()
                        .arg("submodule")
                        .arg("update")
                        .arg("--init")
                        .arg("--recursive")
                        .current_dir(&repo_path)
                        .output();

                    let _ = tx.send(format!("Submodule '{}' added successfully", path));
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr).to_string();
                    let clean_err = err.trim().replace('\n', " ");
                    let _ = tx.send(format!("Failed to add submodule: {}", clean_err));
                }
                Err(e) => {
                    let _ = tx.send(format!("Failed to run git submodule add: {}", e));
                }
            }
        });
    }

    pub fn confirm_submodule_delete(&mut self) {
        let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail else {
            self.mode = Mode::Detail;
            return;
        };
        let Some(sub_name) = self.submodule_delete_target.take() else {
            self.mode = Mode::Detail;
            return;
        };

        let repo_path = resolved.clone();
        self.fetching = true;
        self.status_message = Some(format!("Removing submodule '{}'...", sub_name));
        self.mode = Mode::Detail;

        let tx = self.tx.clone();

        std::thread::spawn(move || {
            let safe_sub = match safe_ref(&sub_name) {
                Ok(s) => s,
                Err(e) => {
                    let _ = tx.send(format!("Invalid submodule name: {}", e));
                    return;
                }
            };

            let deinit_res = git_command()
                .arg("submodule")
                .arg("deinit")
                .arg("-f")
                .arg("--")
                .arg(safe_sub)
                .current_dir(&repo_path)
                .output();

            if let Err(e) = deinit_res {
                let _ = tx.send(format!("Failed to deinit submodule: {}", e));
                return;
            }

            let rm_res = git_command()
                .arg("rm")
                .arg("-f")
                .arg("--")
                .arg(safe_sub)
                .current_dir(&repo_path)
                .output();

            match rm_res {
                Ok(out) if out.status.success() => {
                    let dotgit_modules = repo_path.join(".git").join("modules").join(&sub_name);
                    if dotgit_modules.exists() {
                        let _ = std::fs::remove_dir_all(dotgit_modules);
                    }

                    let _ = tx.send(format!("Submodule '{}' removed successfully", sub_name));
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr).to_string();
                    let clean_err = err.trim().replace('\n', " ");
                    let _ = tx.send(format!(
                        "Failed to remove submodule directory from git: {}",
                        clean_err
                    ));
                }
                Err(e) => {
                    let _ = tx.send(format!("Failed to run git rm for submodule: {}", e));
                }
            }
        });
    }

    pub fn cancel_submodule_delete(&mut self) {
        self.submodule_delete_target = None;
        self.mode = Mode::Detail;
    }
}

fn git_command() -> std::process::Command {
    let mut cmd = std::process::Command::new("git");
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd.env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new");
    cmd.env("GIT_ALLOW_PROTOCOL", "https:ssh:git:file");
    cmd.env("GIT_PROTOCOL_FROM_USER", "0");
    cmd
}

fn safe_ref(r: &str) -> Result<&str, String> {
    let trimmed = r.trim();
    if trimmed.starts_with('-') {
        return Err(format!("Invalid ref name: '{}' (ref names cannot start with '-')", r));
    }
    if trimmed.is_empty() {
        return Err("Ref name cannot be empty".to_string());
    }
    Ok(trimmed)
}
