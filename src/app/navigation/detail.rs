//! Repository detail lifecycle, background tab loads, pane focus, commit search, file tree, and yank helpers.

use super::super::*;

impl App {
    pub fn open_detail(&mut self) {
        if let Some(item) = self.get_selected_item().cloned() {
            self.open_repo(item);
        }
    }

    /// Snapshot `item`'s filesystem/git state and enter the Detail view.
    /// `item` is a `config.items` entry; it need not be the home selection
    /// (a worktree opened from the Worktrees tab, a global-search hit). The
    /// snapshot is held in `current_detail` for as long as the view is open;
    /// closing clears it.
    pub fn open_repo(&mut self, item: String) {
        // Guard: only allow opening a git repository. `item` is a raw
        // `config.items` entry and may start with `~`, so expand it first.
        if !repo::expand_tilde(&item).join(".git").exists() {
            self.mode = Mode::NotGitRepo;
            return;
        }

        crate::debug_log::info(format!("Opening detail view for repository: {}", item));

        // Update visit time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // The visit reorders the home list: it lifts this repository to the
        // top of the Recent group (and of the whole list under Recent Visit
        // sort), shifting the rows around it. Put the cursor back on it,
        // under the group it was opened from, so the cursor has not slid onto
        // a neighbour when the user returns home.
        let group = self.home_cursor().map(|(_, group)| group);
        self.state.visits.insert(item.clone(), now);
        self.persist_state("Opened repository");

        if self.effective_sort_by() == SortOrder::RecentVisit {
            self.sort_items_in_place();
        }
        self.select_home_row(&item, group.as_deref());

        // Resolved for `item` itself: when a worktree is opened from inside
        // another repository, `current_detail` still holds that repository.
        let max_commits_setting = self.resolve_setting(
            &item,
            |rc| rc.max_commits,
            |lc| lc.max_commits,
            self.config.max_commits,
        );

        let cached_valid = if let Some(cached) = self.detail_cache.get(&item) {
            cached.loaded_at.elapsed().as_secs() < self.config.detail_cache_ttl_secs
        } else {
            false
        };

        let tx = self.detail_tx.clone();
        let item_clone = item.clone();
        let graph_max_commits = self.config.graph_max_commits;
        let enable_commit_signatures = self.config.enable_commit_signatures;

        if cached_valid {
            if let Some(mut cached) = self.detail_cache.get(&item).cloned() {
                if let repo::ItemDetail::Repo { info, .. } = &mut cached.detail {
                    info.reset_in_flight_loads();
                }
                let cached_commits_count = match &cached.detail {
                    repo::ItemDetail::Repo { info, .. } => info.commits.len(),
                    _ => 200,
                };
                self.commit_list.limit = if max_commits_setting > 0 {
                    cached_commits_count.max(max_commits_setting)
                } else {
                    0
                };
                self.current_detail = Some(cached.detail);
                self.rebuild_visible_files();
            }

            let max_commits = self.commit_list.limit;
            // Silent background refresh
            std::thread::spawn(move || {
                let detail = repo::inspect_detail(
                    &item_clone,
                    max_commits,
                    graph_max_commits,
                    enable_commit_signatures,
                );
                let _ = tx.send((item_clone, detail));
            });
        } else {
            self.commit_list.limit = max_commits_setting;
            self.loading_repo_path = Some(item.clone());
            let max_commits = self.commit_list.limit;
            std::thread::spawn(move || {
                let detail = repo::inspect_detail(
                    &item_clone,
                    max_commits,
                    graph_max_commits,
                    enable_commit_signatures,
                );
                let _ = tx.send((item_clone, detail));
            });
        }

        self.detail_focus = DetailSection::Commits;
        self.commit_list.selection = 0;
        self.status_list.file_selection = 0;
        self.status_list.staging_file_selection = 0;
        self.diff.file_diff.clear();
        self.diff.diff_scroll = 0;
        self.commit_list.details_scroll = 0;
        self.commit_input_scroll = 0;
        self.branch_list.local_branch_selection = 0;
        self.branch_list.remote_branch_selection = 0;
        self.tag_list.local_tag_selection = 0;
        self.tag_list.remote_tag_selection = 0;
        self.branch_list.remote_selection = 0;
        self.stash_list.stash_selection = 0;
        self.stash_list.stash_file_selection = 0;
        self.file_tree.file_list_selection = 0;
        self.file_tree.file_content_scroll = 0;
        self.file_tree.expanded_folders.clear();
        self.file_tree.file_blame = None;
        self.file_tree.blamed_file_path = None;
        self.file_tree.blame_loading = false;
        *self.file_tree.preview_cache.borrow_mut() = None;
        self.commit_list.selection = 0;
        self.detail_tab = 0;
        // Tab 0 is in the Primary group. Opening a worktree from the Worktrees
        // tab used to keep `advanced_tabs` set: no tab was highlighted, Tab /
        // Shift+Tab computed `0 - 7` (a panic in debug builds, a bogus tab in
        // release), and leaving took two presses of `q`.
        self.advanced_tabs = false;
        // The Advanced tabs' selections and PR comments belong to the last
        // repository; its PR #N comments must not show under this one's #N.
        self.worktree_selection = 0;
        self.submodule_selection = 0;
        self.reflog_selection = 0;
        self.forge_issue_selection = 0;
        self.forge_pr_selection = 0;
        self.forge_pr_comments = None;
        self.forge_pr_comments_pr = None;
        self.forge_pr_comments_loading = false;
        self.graph_scroll = 0;
        self.graph_selection = 0;
        self.inspect_full_diff = false;
        self.commit_popup.maximized = false;
        self.mode = Mode::Detail;
    }

    /// Resync the selected item's filesystem/git state inside the Detail view,
    /// clamping selection indices to their new totals.
    /// Resync the selected item's filesystem/git state inside the Detail view asynchronously.
    pub fn resync_detail(&mut self) {
        let path_opt = if let Some(detail) = &self.current_detail {
            match detail {
                repo::ItemDetail::Repo { resolved, .. }
                | repo::ItemDetail::Missing { resolved, .. }
                | repo::ItemDetail::Directory { resolved, .. }
                | repo::ItemDetail::Error { resolved, .. } => {
                    Some(resolved.to_string_lossy().to_string())
                }
            }
        } else {
            self.get_selected_item().cloned()
        };

        if let Some(item) = path_opt {
            crate::debug_log::info("Resyncing repository details");
            let path = repo::expand_tilde(&item);
            repo::invalidate_ref_map_cache(&path);

            if let Some(repo::ItemDetail::Repo { info, .. }) = &mut self.current_detail {
                info.local_branches = repo::TabData::NotLoaded;
                info.remote_branches = repo::TabData::NotLoaded;
                info.local_tags = repo::TabData::NotLoaded;
                info.remote_tags = repo::TabData::NotLoaded;
                info.files = repo::TabData::NotLoaded;
                info.stashes = repo::TabData::NotLoaded;
                info.worktrees = repo::TabData::NotLoaded;
                info.graph_lines = repo::TabData::NotLoaded;
                info.submodules = repo::TabData::NotLoaded;
                info.reflog = repo::TabData::NotLoaded;
                info.forge_issues = repo::TabData::NotLoaded;
                info.forge_prs = repo::TabData::NotLoaded;
                info.committer_stats = repo::TabData::NotLoaded;
                info.remote_tags_loaded = false;
                info.remote_tags_attempted = false;
                info.tab_loaded_at = [None; 12];
            }

            self.loading_repo_path = Some(item.clone());
            let tx = self.detail_tx.clone();
            let max_commits = self.commit_list.limit;
            let graph_max_commits = self.config.graph_max_commits;
            let enable_commit_signatures = self.config.enable_commit_signatures;
            std::thread::spawn(move || {
                let detail = repo::inspect_detail(
                    &item,
                    max_commits,
                    graph_max_commits,
                    enable_commit_signatures,
                );
                let _ = tx.send((item, detail));
            });
        }
    }

    pub fn update_cache_from_current_detail(&mut self) {
        if let Some(detail) = &self.current_detail {
            let path_str = match detail {
                repo::ItemDetail::Repo { resolved, .. }
                | repo::ItemDetail::Missing { resolved, .. }
                | repo::ItemDetail::Directory { resolved, .. }
                | repo::ItemDetail::Error { resolved, .. } => {
                    resolved.to_string_lossy().to_string()
                }
            };
            self.detail_cache.insert(
                path_str,
                DetailCache { detail: detail.clone(), loaded_at: std::time::Instant::now() },
            );
        }
    }

    /// Apply a loaded detail snapshot, clamping selection indices to their new totals.
    pub fn apply_detail_snapshot(&mut self, detail: repo::ItemDetail) {
        let mut merged_detail = detail;
        if let Some(repo::ItemDetail::Repo { resolved: old_resolved, info: old_info }) =
            &self.current_detail
        {
            if let repo::ItemDetail::Repo { resolved: new_resolved, info: new_info } =
                &mut merged_detail
            {
                if old_resolved == new_resolved {
                    if new_info.remotes.is_not_loaded() {
                        new_info.remotes = old_info.remotes.clone();
                    }
                    if new_info.graph_lines.is_not_loaded() {
                        new_info.graph_lines = old_info.graph_lines.clone();
                    }
                    if new_info.local_branches.is_not_loaded() {
                        new_info.local_branches = old_info.local_branches.clone();
                    }
                    if new_info.remote_branches.is_not_loaded() {
                        new_info.remote_branches = old_info.remote_branches.clone();
                    }
                    if new_info.local_tags.is_not_loaded() {
                        new_info.local_tags = old_info.local_tags.clone();
                    }
                    if new_info.remote_tags.is_not_loaded() {
                        new_info.remote_tags = old_info.remote_tags.clone();
                    }
                    new_info.remote_tags_loaded = old_info.remote_tags_loaded;
                    new_info.remote_tags_attempted = old_info.remote_tags_attempted;
                    if new_info.files.is_not_loaded() {
                        new_info.files = old_info.files.clone();
                    }
                    if new_info.stashes.is_not_loaded() {
                        new_info.stashes = old_info.stashes.clone();
                    }
                    if new_info.submodules.is_not_loaded() {
                        new_info.submodules = old_info.submodules.clone();
                    }
                    if new_info.committer_stats.is_not_loaded() {
                        new_info.committer_stats = old_info.committer_stats.clone();
                        new_info.committer_stats_limit_reached =
                            old_info.committer_stats_limit_reached;
                    }
                    if new_info.reflog.is_not_loaded() {
                        new_info.reflog = old_info.reflog.clone();
                    }
                    if new_info.forge_issues.is_not_loaded() {
                        new_info.forge_issues = old_info.forge_issues.clone();
                    }
                    if new_info.forge_prs.is_not_loaded() {
                        new_info.forge_prs = old_info.forge_prs.clone();
                    }
                    new_info.tab_loaded_at = old_info.tab_loaded_at;
                    new_info.tab_loading = old_info.tab_loading;
                }
            }
        }

        self.current_detail = Some(merged_detail);
        self.ensure_selected_commit_files_loaded();
        self.update_cache_from_current_detail();
        self.rebuild_visible_files();

        // Extract all lengths first to avoid borrow-checker conflicts
        let mut info_lengths = None;
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let commits_len = info.commits.len();
            let local_branches_len = info.local_branches.len();
            let remote_branches_len = info.remote_branches.len();
            let local_tags_len = info.local_tags.len();
            let remote_tags_len = info.remote_tags.len();
            let remotes_len = info.remotes.len();
            let stashes_len = info.stashes.len();
            let staged_len = info.changes.staged.len();
            let unstaged_len = info.changes.unstaged.len();
            let worktrees_len =
                if let repo::TabData::Loaded(wts) = &info.worktrees { wts.len() } else { 0 };
            let submodules_len =
                if let repo::TabData::Loaded(subs) = &info.submodules { subs.len() } else { 0 };

            let commit_files_len = self.get_selected_commit().map(|c| c.files.len()).unwrap_or(0);

            info_lengths = Some((
                commits_len,
                local_branches_len,
                remote_branches_len,
                local_tags_len,
                remote_tags_len,
                remotes_len,
                stashes_len,
                staged_len,
                unstaged_len,
                commit_files_len,
                worktrees_len,
                submodules_len,
            ));
        }

        if let Some((
            commits_len,
            local_branches_len,
            remote_branches_len,
            local_tags_len,
            remote_tags_len,
            remotes_len,
            stashes_len,
            staged_len,
            unstaged_len,
            commit_files_len,
            worktrees_len,
            submodules_len,
        )) = info_lengths
        {
            // 1. Commit selection
            if commits_len == 0 {
                self.commit_list.selection = 0;
            } else if self.commit_list.selection >= commits_len {
                self.commit_list.selection = commits_len - 1;
            }

            // 2. File list selection (Files tab)
            let visible_files_len = self.file_tree.visible_files.len();
            if visible_files_len == 0 {
                self.file_tree.file_list_selection = 0;
            } else if self.file_tree.file_list_selection >= visible_files_len {
                self.file_tree.file_list_selection = visible_files_len - 1;
            }

            // Clamp worktree selection
            if worktrees_len == 0 {
                self.worktree_selection = 0;
            } else if self.worktree_selection >= worktrees_len {
                self.worktree_selection = worktrees_len - 1;
            }

            // Clamp submodule selection
            if submodules_len == 0 {
                self.submodule_selection = 0;
            } else if self.submodule_selection >= submodules_len {
                self.submodule_selection = submodules_len - 1;
            }

            // 3. Local branches selection
            if local_branches_len == 0 {
                self.branch_list.local_branch_selection = 0;
            } else if self.branch_list.local_branch_selection >= local_branches_len {
                self.branch_list.local_branch_selection = local_branches_len - 1;
            }

            // 4. Remote branches selection
            if remote_branches_len == 0 {
                self.branch_list.remote_branch_selection = 0;
            } else if self.branch_list.remote_branch_selection >= remote_branches_len {
                self.branch_list.remote_branch_selection = remote_branches_len - 1;
            }

            // 5. Local tags selection
            if local_tags_len == 0 {
                self.tag_list.local_tag_selection = 0;
            } else if self.tag_list.local_tag_selection >= local_tags_len {
                self.tag_list.local_tag_selection = local_tags_len - 1;
            }

            // 6. Remote tags selection
            if remote_tags_len == 0 {
                self.tag_list.remote_tag_selection = 0;
            } else if self.tag_list.remote_tag_selection >= remote_tags_len {
                self.tag_list.remote_tag_selection = remote_tags_len - 1;
            }

            // 7. Remotes selection
            if remotes_len == 0 {
                self.branch_list.remote_selection = 0;
            } else if self.branch_list.remote_selection >= remotes_len {
                self.branch_list.remote_selection = remotes_len - 1;
            }

            // 8. Stashes selection
            if stashes_len == 0 {
                self.stash_list.stash_selection = 0;
            } else if self.stash_list.stash_selection >= stashes_len {
                self.stash_list.stash_selection = stashes_len - 1;
            }

            // 9. Files/Diff selection in Workspace/Commits details
            // Workspace stage/unstage file lists
            if self.is_uncommitted_selected() {
                // Staged files vs Unstaged files selection
                let active_len = if self.detail_focus == DetailSection::Staged {
                    staged_len
                } else if self.detail_focus == DetailSection::Unstaged {
                    unstaged_len
                } else {
                    0
                };
                if active_len == 0 {
                    self.status_list.staging_file_selection = 0;
                } else if self.status_list.staging_file_selection >= active_len {
                    self.status_list.staging_file_selection = active_len - 1;
                }
            } else {
                // Commits file selection
                if commit_files_len == 0 {
                    self.status_list.file_selection = 0;
                } else if self.status_list.file_selection >= commit_files_len {
                    self.status_list.file_selection = commit_files_len - 1;
                }
            }
        }

        self.diff.diff_scroll = 0;
        if self.is_uncommitted_selected() {
            self.refresh_staging_diff();
        } else {
            self.refresh_file_diff();
        }
    }

    /// Trigger asynchronous loading of a tab's lazy data if it is not yet loaded or stale.
    #[allow(clippy::collapsible_match)]
    pub fn trigger_tab_load_if_needed(&mut self, tab_idx: usize) {
        let Some(repo::ItemDetail::Repo { resolved, info }) = &mut self.current_detail else {
            return;
        };
        let path = resolved.clone();
        let tx = self.tab_tx.clone();
        let graph_max_commits = self.config.graph_max_commits;
        let enable_commit_signatures = self.config.enable_commit_signatures;
        let tab_ttl = self.config.tab_ttl_secs;

        let should_trigger = |info: &repo::RepoInfo, tab_idx: usize, is_not_loaded: bool| -> bool {
            if info.tab_loading[tab_idx] {
                return false;
            }
            if is_not_loaded {
                return true;
            }
            if let Some(loaded_at) = info.tab_loaded_at[tab_idx] {
                loaded_at.elapsed().as_secs() >= tab_ttl
            } else {
                true
            }
        };

        match tab_idx {
            1 => {
                let is_not_loaded = info.files.is_not_loaded();
                crate::debug_log::info(format!(
                    "trigger_tab_load_if_needed(1): is_not_loaded={}, tab_loading={}",
                    is_not_loaded, info.tab_loading[tab_idx]
                ));
                if should_trigger(info, tab_idx, is_not_loaded) {
                    crate::debug_log::info(
                        "trigger_tab_load_if_needed(1): spawning load_tab_files thread",
                    );
                    info.tab_loading[tab_idx] = true;
                    if is_not_loaded {
                        info.files = repo::TabData::Loading;
                    }
                    std::thread::spawn(move || {
                        let res = repo::load_tab_files(&path);
                        let _ = tx.send((
                            path.to_string_lossy().to_string(),
                            tab_idx,
                            repo::TabPayload::Files(res),
                        ));
                    });
                }
            }
            2 => {
                let is_not_loaded = info.graph_lines.is_not_loaded();
                if should_trigger(info, tab_idx, is_not_loaded) {
                    info.tab_loading[tab_idx] = true;
                    if is_not_loaded {
                        info.graph_lines = repo::TabData::Loading;
                    }
                    let tx_clone = tx.clone();
                    let path_str = path.to_string_lossy().to_string();
                    std::thread::spawn(move || {
                        let res = repo::load_tab_graph_stream(
                            &path,
                            graph_max_commits,
                            enable_commit_signatures,
                            path_str.clone(),
                            tab_idx,
                            tx_clone,
                        );
                        let (graph, notice) = match res {
                            Ok(load) => (Ok(load.lines), load.notice),
                            Err(e) => (Err(e), None),
                        };
                        let _ =
                            tx.send((path_str.clone(), tab_idx, repo::TabPayload::Graph(graph)));
                        if let Some(notice) = notice {
                            let _ = tx.send((path_str, tab_idx, repo::TabPayload::Notice(notice)));
                        }
                    });
                }
            }
            3 => {
                let is_not_loaded = info.local_branches.is_not_loaded();
                if should_trigger(info, tab_idx, is_not_loaded) {
                    info.tab_loading[tab_idx] = true;
                    if is_not_loaded {
                        info.local_branches = repo::TabData::Loading;
                        info.remote_branches = repo::TabData::Loading;
                    }
                    std::thread::spawn(move || {
                        let (local_res, remote_res) = repo::load_tab_branches(&path);
                        let _ = tx.send((
                            path.to_string_lossy().to_string(),
                            tab_idx,
                            repo::TabPayload::Branches { local: local_res, remote: remote_res },
                        ));
                    });
                }
            }
            4 => {
                let is_not_loaded = info.local_tags.is_not_loaded();
                if should_trigger(info, tab_idx, is_not_loaded) {
                    info.tab_loading[tab_idx] = true;
                    if is_not_loaded {
                        info.local_tags = repo::TabData::Loading;
                        info.remote_tags = repo::TabData::Loading;
                    }
                    std::thread::spawn(move || {
                        let (local_res, remote_res) = repo::load_tab_tags(&path);
                        let _ = tx.send((
                            path.to_string_lossy().to_string(),
                            tab_idx,
                            repo::TabPayload::Tags { local: local_res, remote: remote_res },
                        ));
                    });
                }
            }
            5 => {
                let is_not_loaded = info.remotes.is_not_loaded();
                if should_trigger(info, tab_idx, is_not_loaded) {
                    info.tab_loading[tab_idx] = true;
                    if is_not_loaded {
                        info.remotes = repo::TabData::Loading;
                    }
                    std::thread::spawn(move || {
                        let res = repo::load_tab_remotes(&path);
                        let _ = tx.send((
                            path.to_string_lossy().to_string(),
                            tab_idx,
                            repo::TabPayload::Remotes(res),
                        ));
                    });
                }
            }
            6 => {
                let is_not_loaded = info.stashes.is_not_loaded();
                if should_trigger(info, tab_idx, is_not_loaded) {
                    info.tab_loading[tab_idx] = true;
                    if is_not_loaded {
                        info.stashes = repo::TabData::Loading;
                    }
                    std::thread::spawn(move || {
                        let res = repo::load_tab_stashes(&path);
                        let _ = tx.send((
                            path.to_string_lossy().to_string(),
                            tab_idx,
                            repo::TabPayload::Stashes(res),
                        ));
                    });
                }
            }
            7 => {
                let is_not_loaded = info.worktrees.is_not_loaded();
                if should_trigger(info, tab_idx, is_not_loaded) {
                    info.tab_loading[tab_idx] = true;
                    if is_not_loaded {
                        info.worktrees = repo::TabData::Loading;
                    }
                    std::thread::spawn(move || {
                        let res = repo::load_tab_worktrees(&path);
                        let _ = tx.send((
                            path.to_string_lossy().to_string(),
                            tab_idx,
                            repo::TabPayload::Worktrees(res),
                        ));
                    });
                }
            }
            8 => {
                let is_not_loaded = info.submodules.is_not_loaded();
                if should_trigger(info, tab_idx, is_not_loaded) {
                    info.tab_loading[tab_idx] = true;
                    if is_not_loaded {
                        info.submodules = repo::TabData::Loading;
                    }
                    std::thread::spawn(move || {
                        let res = repo::load_tab_submodules(&path);
                        let _ = tx.send((
                            path.to_string_lossy().to_string(),
                            tab_idx,
                            repo::TabPayload::Submodules(res),
                        ));
                    });
                }
            }
            9 => {
                let is_not_loaded = info.reflog.is_not_loaded();
                if should_trigger(info, tab_idx, is_not_loaded) {
                    info.tab_loading[tab_idx] = true;
                    if is_not_loaded {
                        info.reflog = repo::TabData::Loading;
                    }
                    std::thread::spawn(move || {
                        let res = repo::load_tab_reflog(&path);
                        let _ = tx.send((
                            path.to_string_lossy().to_string(),
                            tab_idx,
                            repo::TabPayload::Reflog(res),
                        ));
                    });
                }
            }
            10 => {
                let is_not_loaded = info.forge_issues.is_not_loaded();
                if should_trigger(info, tab_idx, is_not_loaded) {
                    info.tab_loading[tab_idx] = true;
                    if is_not_loaded {
                        info.forge_issues = repo::TabData::Loading;
                    }
                    let assigned_only = self.forge_issues_assigned_only;
                    let timeout = std::time::Duration::from_secs(self.config.fetch_timeout_secs);
                    std::thread::spawn(move || {
                        let res = repo::load_tab_forge_issues(&path, assigned_only, timeout);
                        let _ = tx.send((
                            path.to_string_lossy().to_string(),
                            tab_idx,
                            repo::TabPayload::ForgeIssues(res),
                        ));
                    });
                }
            }
            11 => {
                let is_not_loaded = info.forge_prs.is_not_loaded();
                if should_trigger(info, tab_idx, is_not_loaded) {
                    info.tab_loading[tab_idx] = true;
                    if is_not_loaded {
                        info.forge_prs = repo::TabData::Loading;
                    }
                    let timeout = std::time::Duration::from_secs(self.config.fetch_timeout_secs);
                    std::thread::spawn(move || {
                        let res = repo::load_tab_forge_prs(&path, timeout);
                        let _ = tx.send((
                            path.to_string_lossy().to_string(),
                            tab_idx,
                            repo::TabPayload::ForgePRs(res),
                        ));
                    });
                } else if !self.forge_pr_comments_loading
                    && self.forge_pr_comments_pr != self.selected_pr_number()
                {
                    // The comments held are for another PR (or none yet): the
                    // selection moved, by key, mouse or a list reload. A
                    // failed load for the selected PR is kept, not retried.
                    self.load_comments_for_selected_pr();
                }
            }
            _ => {}
        }
    }

    pub fn trigger_overview_load_if_needed(&mut self) {
        let commit_limit = self.get_current_max_commits();
        let Some(repo::ItemDetail::Repo { resolved, info }) = &mut self.current_detail else {
            return;
        };
        let path = resolved.clone();
        let tx = self.tab_tx.clone();

        let should_trigger = info.committer_stats.is_not_loaded();
        if should_trigger {
            info.committer_stats = repo::TabData::Loading;
            std::thread::spawn(move || {
                let res = repo::load_tab_overview(&path, commit_limit);
                let _ = tx.send((
                    path.to_string_lossy().to_string(),
                    99,
                    repo::TabPayload::Overview(res),
                ));
            });
        }
    }

    /// Advance focus to the next detail panel (Tab key).
    pub fn cycle_detail_focus(&mut self, reverse: bool) {
        if self.detail_tab == 3 {
            self.detail_focus = match self.detail_focus {
                DetailSection::LocalBranches => DetailSection::RemoteBranches,
                _ => DetailSection::LocalBranches,
            };
            return;
        }
        if self.detail_tab == 4 {
            self.detail_focus = match self.detail_focus {
                DetailSection::LocalTags => DetailSection::RemoteTags,
                _ => DetailSection::LocalTags,
            };
            return;
        }
        if self.detail_tab == 1 {
            self.detail_focus = match self.detail_focus {
                DetailSection::Files => DetailSection::FileContent,
                _ => DetailSection::Files,
            };
            return;
        }
        if self.detail_tab == 6 {
            self.detail_focus = if reverse {
                match self.detail_focus {
                    DetailSection::Stashes => DetailSection::StagingDetails,
                    DetailSection::StagingDetails => DetailSection::StashedFiles,
                    _ => DetailSection::Stashes,
                }
            } else {
                match self.detail_focus {
                    DetailSection::Stashes => DetailSection::StashedFiles,
                    DetailSection::StashedFiles => DetailSection::StagingDetails,
                    _ => DetailSection::Stashes,
                }
            };
            return;
        }
        if self.detail_tab == 10 {
            self.detail_focus = match self.detail_focus {
                DetailSection::ForgeIssues => DetailSection::ForgeIssueDetails,
                _ => DetailSection::ForgeIssues,
            };
            return;
        }
        if self.detail_tab == 11 {
            self.detail_focus = match self.detail_focus {
                DetailSection::ForgePRs => DetailSection::ForgePRDetails,
                _ => DetailSection::ForgePRs,
            };
            return;
        }
        if self.detail_tab == 0 {
            let mut next_focus =
                if reverse { self.detail_focus.prev() } else { self.detail_focus.next() };
            for _ in 0..10 {
                let skip = match next_focus {
                    DetailSection::Staged => {
                        if self.is_uncommitted_selected() {
                            self.is_staged_empty()
                        } else {
                            self.is_selected_commit_empty()
                        }
                    }
                    DetailSection::Unstaged => {
                        self.is_unstaged_empty() || !self.is_uncommitted_selected()
                    }
                    DetailSection::Conflicts => {
                        self.is_conflicted_empty() || !self.is_uncommitted_selected()
                    }
                    DetailSection::CommitDetails => self.is_uncommitted_selected(),
                    DetailSection::StagingDetails => {
                        if self.is_uncommitted_selected() {
                            self.is_staged_empty() && self.is_unstaged_empty()
                        } else {
                            self.is_selected_commit_empty()
                        }
                    }
                    DetailSection::ConflictDiff => {
                        self.is_conflicted_empty() || !self.is_uncommitted_selected()
                    }
                    _ => false,
                };
                if skip {
                    next_focus = if reverse { next_focus.prev() } else { next_focus.next() };
                } else {
                    break;
                }
            }
            self.detail_focus = next_focus;
        } else {
            self.detail_focus =
                if reverse { self.detail_focus.prev() } else { self.detail_focus.next() };
        }
        if self.detail_focus == DetailSection::Staged
            || self.detail_focus == DetailSection::Unstaged
            || self.detail_focus == DetailSection::Conflicts
        {
            self.last_staging_focus = self.detail_focus;
        }
        // Reset staging selection and pre-load diff when landing on Staged/Unstaged/Conflicts.
        match self.detail_focus {
            DetailSection::Staged | DetailSection::Unstaged | DetailSection::Conflicts => {
                self.diff.diff_scroll = 0;
                if self.is_uncommitted_selected() {
                    if self.detail_focus == DetailSection::Conflicts {
                        self.status_list.conflict_file_selection = 0;
                    } else {
                        self.status_list.staging_file_selection = 0;
                    }
                    self.refresh_staging_diff();
                } else {
                    self.status_list.file_selection = 0;
                    self.refresh_file_diff();
                }
            }
            DetailSection::CommitDetails => {
                self.commit_list.details_scroll = 0;
            }
            DetailSection::StagingDetails | DetailSection::ConflictDiff => {
                self.diff.diff_scroll = 0;
            }
            _ => {}
        }
    }

    /// Move local branch selection up.

    /// Total number of rows in the Commits panel (dirty row + real commits).
    /// Total number of rows in the Commits panel (dirty row + real commits).
    pub fn commit_total(&self) -> usize {
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => {
                if self.in_logs_ui {
                    return info.commits.len();
                }
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                let show_dirty = if dirty {
                    if let Some(ref query) = self.commit_list.search_query {
                        "<uncommitted>".contains(&query.to_lowercase())
                    } else {
                        true
                    }
                } else {
                    false
                };
                let filtered_len = self.get_filtered_commits().len();
                filtered_len + usize::from(show_dirty)
            }
            _ => 0,
        }
    }

    pub fn get_selected_commit(&self) -> Option<&crate::repo::CommitEntry> {
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => {
                if self.detail_tab == 2 {
                    if let repo::TabData::Loaded(ref lines) = info.graph_lines {
                        if let Some(line) = lines.get(self.graph_selection) {
                            if let Some(ref c) = line.commit {
                                return info.commits.iter().find(|entry| entry.oid == c.oid);
                            }
                        }
                    }
                    return None;
                }
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                let show_dirty = if dirty {
                    if let Some(ref query) = self.commit_list.search_query {
                        "<uncommitted>".contains(&query.to_lowercase())
                    } else {
                        true
                    }
                } else {
                    false
                };
                if show_dirty && self.commit_list.selection == 0 {
                    return None;
                }
                let idx = if show_dirty {
                    self.commit_list.selection.saturating_sub(1)
                } else {
                    self.commit_list.selection
                };
                if self.in_logs_ui {
                    info.commits.get(idx)
                } else {
                    self.get_filtered_commits().get(idx).copied()
                }
            }
            _ => None,
        }
    }

    /// Total files in the currently-selected commit's Changed Files panel.
    pub fn file_total(&self) -> usize {
        self.get_selected_commit().map(|c| c.files.len()).unwrap_or(0)
    }

    pub fn is_uncommitted_selected(&self) -> bool {
        if self.in_logs_ui {
            return false;
        }
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => {
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                let show_dirty = if dirty {
                    if let Some(ref query) = self.commit_list.search_query {
                        "<uncommitted>".contains(&query.to_lowercase())
                    } else {
                        true
                    }
                } else {
                    false
                };
                show_dirty && self.commit_list.selection == 0
            }
            _ => false,
        }
    }

    pub fn has_uncommitted_changes(&self) -> bool {
        !self.is_staged_empty() || !self.is_unstaged_empty() || !self.is_conflicted_empty()
    }

    pub fn is_selected_commit_empty(&self) -> bool {
        self.get_selected_commit().map(|c| c.files.is_empty()).unwrap_or(true)
    }

    pub fn ensure_selected_commit_files_loaded(&mut self) {
        let target_oid = self.get_selected_commit().map(|c| c.oid.clone());
        if let Some(oid) = target_oid {
            if let Some(repo::ItemDetail::Repo { resolved, info }) = &mut self.current_detail {
                if let Some(c) = info.commits.iter_mut().find(|c| c.oid == oid) {
                    if c.files.is_empty() {
                        if let Ok(files) = repo::get_commit_files(resolved, &oid) {
                            c.files = files;
                        }
                    }
                }
            }
        }
    }

    pub fn refresh_detail(&mut self) {
        self.resync_detail();
    }

    pub(in crate::app) fn clamp_conflict_selection(&mut self) {
        if let Some(ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.changes.conflicted.len();
            if total == 0 {
                self.status_list.conflict_file_selection = 0;
                if self.detail_focus == DetailSection::Conflicts
                    || self.detail_focus == DetailSection::ConflictDiff
                {
                    self.detail_focus = DetailSection::Unstaged;
                }
            } else if self.status_list.conflict_file_selection >= total {
                self.status_list.conflict_file_selection = total.saturating_sub(1);
            }
        }
    }

    pub fn close_detail(&mut self) {
        self.current_detail = None;
        self.commit_list.search_query = None;
        self.loading_repo_path = None;
        self.mode = Mode::Normal;
    }

    pub fn get_filtered_commits(&self) -> Vec<&crate::repo::CommitEntry> {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if let Some(ref query) = self.commit_list.search_query {
                let q = query.to_lowercase();
                info.commits
                    .iter()
                    .filter(|c| {
                        c.id.to_lowercase().contains(&q)
                            || c.author.to_lowercase().contains(&q)
                            || c.when.to_lowercase().contains(&q)
                            || c.summary.to_lowercase().contains(&q)
                    })
                    .collect()
            } else {
                info.commits.iter().collect()
            }
        } else {
            Vec::new()
        }
    }

    pub fn commit_matches_query(&self, commit: &crate::repo::CommitEntry) -> bool {
        if let Some(ref query) = self.commit_list.search_query {
            if query.is_empty() {
                return false;
            }
            let q = query.to_lowercase();
            let mut matches = false;
            if self.search_columns_sha && commit.id.to_lowercase().contains(&q) {
                matches = true;
            }
            if self.search_columns_message && commit.summary.to_lowercase().contains(&q) {
                matches = true;
            }
            if self.search_columns_author && commit.author.to_lowercase().contains(&q) {
                matches = true;
            }
            if self.search_columns_date && commit.when.to_lowercase().contains(&q) {
                matches = true;
            }
            matches
        } else {
            false
        }
    }

    pub fn clamp_commit_selection(&mut self) {
        let total = self.commit_total();
        if total == 0 {
            self.commit_list.selection = 0;
        } else if self.commit_list.selection >= total {
            self.commit_list.selection = total - 1;
        }
    }

    #[allow(dead_code)]
    pub fn start_commit_search(&mut self) {
        self.set_input_buffer(self.commit_list.search_query.clone().unwrap_or_default());
        self.mode = Mode::CommitSearchInput;
    }

    pub fn commit_search_input_change(&mut self) {
        self.commit_list.search_query =
            if self.input_buffer.is_empty() { None } else { Some(self.input_buffer.clone()) };
        self.clamp_commit_selection();
        self.status_list.file_selection = 0;
        self.diff.diff_scroll = 0;
        self.refresh_file_diff();
    }

    /// Opens the shortcut help overlay inside the detail view.
    pub fn open_detail_help(&mut self) {
        self.help_scroll = 0;
        self.mode = Mode::DetailHelp;
    }

    /// Closes the detail help overlay and returns to the normal detail view.
    pub fn close_detail_help(&mut self) {
        self.mode = Mode::Detail;
    }

    /// Enters the commit input mode if there are staged changes to commit.
    pub fn start_commit(&mut self) {
        let has_staged = match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => info.summary.staged > 0,
            _ => false,
        };
        let has_head = match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => info.head.is_some(),
            _ => false,
        };
        if has_staged || has_head {
            // Retain the current buffer as draft, do not clear it
            self.commit_popup.editing = true;
            self.commit_popup.amend = false;
            self.commit_input_scroll = 0;
            self.commit_popup.scroll.set(0);
            self.commit_popup.cursor_idx = self.commit_popup.input_buffer.chars().count();
            self.commit_popup.adjust_scroll();
            self.commit_popup.maximized = false;
            self.mode = Mode::CommitInput;
        } else {
            self.status_message = Some("No staged changes to commit".to_string());
        }
    }

    /// Enters the commit input mode for amending the last commit, pre-populating its message.
    pub fn start_commit_amend(&mut self) {
        let has_head = match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => info.head.is_some(),
            _ => false,
        };
        if has_head {
            self.commit_popup.input_buffer.clear();
            if let Some(ItemDetail::Repo { resolved, .. }) = &self.current_detail {
                if let Some(last_msg) = repo::get_last_commit_message(resolved) {
                    self.commit_popup.input_buffer = last_msg;
                }
            }
            self.commit_popup.editing = true;
            self.commit_popup.amend = true;
            self.commit_input_scroll = 0;
            self.commit_popup.scroll.set(0);
            self.commit_popup.cursor_idx = self.commit_popup.input_buffer.chars().count();
            self.commit_popup.adjust_scroll();
            self.commit_popup.maximized = false;
            self.mode = Mode::CommitInput;
        } else {
            self.status_message = Some("No commit to amend".to_string());
        }
    }

    /// Transitions from editing the message to confirming the commit.
    pub fn commit_done_editing(&mut self) {
        self.commit_popup.editing = false;
    }

    /// Transitions back to editing the message from confirm state.
    pub fn commit_start_editing(&mut self) {
        self.commit_popup.editing = true;
    }

    pub fn toggle_commit_amend(&mut self) {
        self.commit_popup.amend = !self.commit_popup.amend;
        if self.commit_popup.amend && self.commit_popup.input_buffer.trim().is_empty() {
            if let Some(ItemDetail::Repo { resolved, .. }) = &self.current_detail {
                if let Some(last_msg) = repo::get_last_commit_message(resolved) {
                    self.set_input_buffer(last_msg);
                }
            }
        }
    }

    pub fn toggle_commit_popup_maximized(&mut self) {
        self.commit_popup.maximized = !self.commit_popup.maximized;
    }

    pub fn commit_input_scroll_up(&mut self) {
        self.commit_input_scroll = self.commit_input_scroll.saturating_sub(1);
        self.commit_popup.scroll.set(self.commit_input_scroll);
    }

    pub fn commit_input_scroll_down(&mut self) {
        self.commit_input_scroll = self.commit_input_scroll.saturating_add(1);
        self.commit_popup.scroll.set(self.commit_input_scroll);
    }

    /// Rebuilds the flattened list of visible tree nodes in the Files tab.
    pub fn rebuild_visible_files(&mut self) {
        let mut visible_files = Vec::new();
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let mut root = TempNode {
                name: "".to_string(),
                full_path: "".to_string(),
                is_dir: true,
                children: std::collections::BTreeMap::new(),
            };

            for file_path in info.files.iter() {
                let parts: Vec<&str> = file_path.split('/').collect();
                let mut current = &mut root;
                let mut accumulated = String::new();
                for (i, part) in parts.iter().enumerate() {
                    if !accumulated.is_empty() {
                        accumulated.push('/');
                    }
                    accumulated.push_str(part);

                    let is_last = i == parts.len() - 1;
                    let entry =
                        current.children.entry((*part).to_string()).or_insert_with(|| TempNode {
                            name: (*part).to_string(),
                            full_path: accumulated.clone(),
                            is_dir: !is_last,
                            children: std::collections::BTreeMap::new(),
                        });
                    current = entry;
                }
            }

            fn flatten_tree(
                node: &TempNode,
                depth: usize,
                expanded_folders: &std::collections::HashSet<String>,
                out: &mut Vec<FileTreeItem>,
            ) {
                let mut child_nodes: Vec<&TempNode> = node.children.values().collect();
                child_nodes.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                });

                for child in child_nodes {
                    let is_expanded = child.is_dir && expanded_folders.contains(&child.full_path);
                    out.push(FileTreeItem {
                        name: child.name.clone(),
                        full_path: child.full_path.clone(),
                        is_dir: child.is_dir,
                        depth,
                        is_expanded,
                    });
                    if is_expanded {
                        flatten_tree(child, depth + 1, expanded_folders, out);
                    }
                }
            }

            flatten_tree(&root, 0, &self.file_tree.expanded_folders, &mut visible_files);
        }
        self.file_tree.visible_files = visible_files;
    }

    /// Expand the selected folder in the Files tab.

    pub fn toggle_folder_expanded(&mut self) {
        if let Some(item) =
            self.file_tree.visible_files.get(self.file_tree.file_list_selection).cloned()
        {
            if item.is_dir {
                if self.file_tree.expanded_folders.contains(&item.full_path) {
                    self.collapse_selected_folder();
                } else {
                    self.expand_selected_folder();
                }
            } else {
                self.detail_focus = DetailSection::FileContent;
            }
        }
    }

    pub fn collapse_all_folders(&mut self) {
        self.file_tree.expanded_folders.clear();
        self.rebuild_visible_files();
    }

    pub fn expand_selected_folder(&mut self) {
        if let Some(item) = self.file_tree.visible_files.get(self.file_tree.file_list_selection) {
            if item.is_dir {
                self.file_tree.expanded_folders.insert(item.full_path.clone());
                self.rebuild_visible_files();
            }
        }
    }

    /// Collapse the selected folder in the Files tab.
    pub fn collapse_selected_folder(&mut self) {
        if let Some(item) = self.file_tree.visible_files.get(self.file_tree.file_list_selection) {
            if item.is_dir {
                self.file_tree.expanded_folders.remove(&item.full_path);
                self.rebuild_visible_files();
            }
        }
    }

    pub fn open_file_history(&mut self) {
        let selected_file = if let Some(item) =
            self.file_tree.visible_files.get(self.file_tree.file_list_selection)
        {
            if !item.is_dir { Some(item.full_path.clone()) } else { None }
        } else {
            None
        };

        if let Some(file_path) = selected_file {
            let repo_path =
                if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
                    Some(resolved.clone())
                } else {
                    None
                };
            if let Some(repo_path) = repo_path {
                self.file_history_revisions.clear();
                self.file_history_selection = 0;
                self.file_history_path = file_path.clone();
                self.file_history_diff.clear();
                self.file_history_diff_scroll = 0;
                self.file_history_focus = 0;
                self.file_history_loading = true;
                self.mode = Mode::FileHistory;
                let tx = self.tx.clone();
                std::thread::spawn(move || {
                    let result = repo::get_file_history(&repo_path, &file_path);
                    let _ = tx.send(crate::app::BgEvent::FileHistoryLoaded {
                        repo_path,
                        file_path,
                        result,
                    });
                });
            }
        }
    }

    pub fn refresh_file_history_diff(&mut self) {
        if self.file_history_revisions.is_empty() {
            self.file_history_diff = Vec::new();
            return;
        }
        let revision = self.file_history_revisions[self.file_history_selection].clone();
        let repo_path = if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail
        {
            Some(resolved.clone())
        } else {
            None
        };
        if let Some(repo_path) = repo_path {
            let diff_path = revision.path.as_deref().unwrap_or(&self.file_history_path);
            self.file_history_diff =
                repo::get_commit_file_diff(&repo_path, &revision.commit_oid, diff_path);
            self.file_history_diff_scroll = 0;
        }
    }

    /// Shift panel focus to the left.
    pub fn move_focus_left(&mut self) {
        if self.detail_tab == 3 {
            self.detail_focus = DetailSection::LocalBranches;
        }
    }

    /// Shift panel focus to the right.
    pub fn move_focus_right(&mut self) {
        if self.detail_tab == 3 {
            self.detail_focus = DetailSection::RemoteBranches;
        }
    }

    /// Sets the default active panel focus when switching tabs.
    pub fn set_default_focus_for_tab(&mut self) {
        match self.detail_tab {
            0 => self.detail_focus = DetailSection::Commits,
            1 => {
                self.detail_focus = DetailSection::Files;
                self.file_tree.file_content_scroll = 0;
            }
            3 => self.detail_focus = DetailSection::LocalBranches,
            4 => {
                self.detail_focus = DetailSection::LocalTags;
            }
            5 => {
                self.detail_focus = DetailSection::Remotes;
            }
            6 => {
                self.detail_focus = DetailSection::Stashes;
                self.stash_list.stash_file_selection = 0;
                self.refresh_file_diff();
            }
            7 => {
                self.detail_focus = DetailSection::Worktrees;
            }
            8 => {
                self.detail_focus = DetailSection::Submodules;
            }
            9 => {
                self.detail_focus = DetailSection::Reflog;
            }
            10 => {
                self.detail_focus = DetailSection::ForgeIssues;
            }
            11 => {
                self.detail_focus = DetailSection::ForgePRs;
            }
            _ => {}
        }
    }

    pub fn yank_selected_commit_hash(&mut self) {
        if self.is_uncommitted_selected() {
            self.status_message = Some("Cannot yank uncommitted changes".to_string());
            return;
        }
        let hash_to_copy = self.get_selected_commit().map(|commit| commit.oid.clone());

        if let Some(hash) = hash_to_copy {
            match copy_to_clipboard(&hash) {
                Ok(()) => {
                    self.status_message = Some(format!("Copied hash {:.7} to clipboard", hash));
                }
                Err(e) => {
                    self.set_error(format!("Failed to copy to clipboard: {}", e));
                }
            }
        }
    }

    pub fn yank_selected_repo_path(&mut self) {
        if let Some(path) = self.get_selected_item() {
            let expanded = crate::repo::expand_tilde(path);
            let abs_path = match std::fs::canonicalize(&expanded) {
                Ok(p) => p.to_string_lossy().to_string(),
                Err(_) => expanded.to_string_lossy().to_string(),
            };
            match crate::app::copy_to_clipboard(&abs_path) {
                Ok(()) => {
                    self.status_message = Some(format!("Copied path '{}' to clipboard", abs_path));
                }
                Err(e) => {
                    self.set_error(format!("Failed to copy to clipboard: {}", e));
                }
            }
        }
    }
}
