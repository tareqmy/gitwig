use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HomeRow {
    GroupHeader { name: String, count: usize, collapsed: bool },
    Repo { actual_index: usize, path: String, primary_label: String },
}

impl App {
    pub fn set_error(&mut self, msg: String) {
        crate::debug_log::error(&msg);
        self.error_message = Some(msg);
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

    pub fn get_home_rows(&self) -> Vec<HomeRow> {
        let filtered = self.get_filtered_items();
        let mut recent_repos = Vec::new();
        for (actual_index, item) in filtered.iter() {
            if let Some(&time) = self.config.visits.get(*item) {
                if time > 0 {
                    recent_repos.push((time, *actual_index, (*item).clone()));
                }
            }
        }
        recent_repos.sort_by_key(|b| std::cmp::Reverse(b.0));
        recent_repos.truncate(5);

        let mut starred_repos = Vec::new();
        for (actual_index, item) in filtered.iter() {
            if self.config.starred.contains(*item) {
                starred_repos.push((*actual_index, (*item).clone()));
            }
        }
        starred_repos.sort_by(|a, b| {
            let name_a = std::path::Path::new(&a.1)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&a.1)
                .to_lowercase();
            let name_b = std::path::Path::new(&b.1)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&b.1)
                .to_lowercase();
            name_a.cmp(&name_b)
        });

        let has_any_labels = filtered
            .iter()
            .any(|(_, item)| self.config.labels.get(*item).is_some_and(|lbls| !lbls.is_empty()));

        if !self.config.show_grouping
            || (!has_any_labels && recent_repos.is_empty() && starred_repos.is_empty())
        {
            return filtered
                .into_iter()
                .map(|(actual_index, path)| HomeRow::Repo {
                    actual_index,
                    path: path.clone(),
                    primary_label: String::new(),
                })
                .collect();
        }

        let mut rows = Vec::new();

        if !recent_repos.is_empty() {
            let collapsed = self.collapsed_groups.contains("Recent");
            rows.push(HomeRow::GroupHeader {
                name: "Recent".to_string(),
                count: recent_repos.len(),
                collapsed,
            });
            if !collapsed {
                for &(_, actual_index, ref path) in &recent_repos {
                    rows.push(HomeRow::Repo {
                        actual_index,
                        path: path.clone(),
                        primary_label: "Recent".to_string(),
                    });
                }
            }
        }

        if !starred_repos.is_empty() {
            let collapsed = self.collapsed_groups.contains("Starred");
            rows.push(HomeRow::GroupHeader {
                name: "Starred".to_string(),
                count: starred_repos.len(),
                collapsed,
            });
            if !collapsed {
                for &(actual_index, ref path) in &starred_repos {
                    rows.push(HomeRow::Repo {
                        actual_index,
                        path: path.clone(),
                        primary_label: "Starred".to_string(),
                    });
                }
            }
        }

        if has_any_labels {
            let mut groups: std::collections::HashMap<String, Vec<(usize, &String)>> =
                std::collections::HashMap::new();
            for &(actual_index, item) in &filtered {
                if let Some(lbls) = self.config.labels.get(item) {
                    if !lbls.is_empty() {
                        for label in lbls {
                            groups.entry(label.clone()).or_default().push((actual_index, item));
                        }
                    } else {
                        groups
                            .entry("Unlabeled".to_string())
                            .or_default()
                            .push((actual_index, item));
                    }
                } else {
                    groups.entry("Unlabeled".to_string()).or_default().push((actual_index, item));
                }
            }

            let mut group_names: Vec<String> = groups.keys().cloned().collect();
            group_names.sort_by(|a, b| {
                if a == "Unlabeled" {
                    std::cmp::Ordering::Greater
                } else if b == "Unlabeled" {
                    std::cmp::Ordering::Less
                } else {
                    a.to_lowercase().cmp(&b.to_lowercase())
                }
            });

            for name in group_names {
                let repos = &groups[&name];
                let collapsed = self.collapsed_groups.contains(&name);
                rows.push(HomeRow::GroupHeader {
                    name: name.clone(),
                    count: repos.len(),
                    collapsed,
                });
                if !collapsed {
                    for &(actual_index, path) in repos {
                        rows.push(HomeRow::Repo {
                            actual_index,
                            path: path.clone(),
                            primary_label: name.clone(),
                        });
                    }
                }
            }
        } else {
            for &(actual_index, path) in &filtered {
                rows.push(HomeRow::Repo {
                    actual_index,
                    path: path.clone(),
                    primary_label: String::new(),
                });
            }
        }

        rows
    }

    pub fn get_filtered_items(&self) -> Vec<(usize, &String)> {
        let base_items: Vec<(usize, &String)> = if let Some(filter) = self.global_filter {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let stale_threshold = 30 * 24 * 60 * 60; // 30 days

            self.config
                .items
                .iter()
                .enumerate()
                .filter(|&(idx, _)| {
                    let status = self.statuses.get(idx);
                    match filter {
                        GlobalFilter::Dirty => {
                            if let Some(crate::repo::ItemStatus::GitRepo(Some(summary))) = status {
                                !summary.is_clean()
                            } else {
                                false
                            }
                        }
                        GlobalFilter::Ahead => {
                            if let Some(crate::repo::ItemStatus::GitRepo(Some(summary))) = status {
                                summary.ahead > 0
                            } else {
                                false
                            }
                        }
                        GlobalFilter::Stale => {
                            if let Some(crate::repo::ItemStatus::GitRepo(Some(summary))) = status {
                                if let Some(t) = summary.last_commit_time {
                                    now - t > stale_threshold
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }
                    }
                })
                .collect()
        } else {
            self.config.items.iter().enumerate().collect()
        };

        if let Some(ref query) = self.repo_search_query {
            let query_lower = query.to_lowercase();
            base_items
                .into_iter()
                .filter(|&(_, item)| {
                    let file_name = std::path::Path::new(item)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(item.as_str())
                        .to_lowercase();
                    let full_path = item.to_lowercase();
                    let label_match = self.config.labels.get(item).is_some_and(|lbls| {
                        lbls.iter().any(|lbl| lbl.to_lowercase().contains(&query_lower))
                    });
                    file_name.contains(&query_lower)
                        || full_path.contains(&query_lower)
                        || label_match
                })
                .collect()
        } else {
            base_items
        }
    }

    pub fn get_items_len(&self) -> usize {
        self.get_home_rows().len()
    }

    pub fn get_selected_item(&self) -> Option<&String> {
        let rows = self.get_home_rows();
        let row = rows.get(self.selected_index)?;
        match row {
            HomeRow::Repo { actual_index, .. } => self.config.items.get(*actual_index),
            HomeRow::GroupHeader { .. } => None,
        }
    }

    pub fn get_current_page_size(&self) -> usize {
        self.get_selected_item()
            .and_then(|path| self.config.repo_configs.get(path))
            .and_then(|rc| rc.page_size)
            .unwrap_or(self.config.page_size)
    }

    pub fn get_current_max_commits(&self) -> usize {
        self.get_selected_item()
            .and_then(|path| self.config.repo_configs.get(path))
            .and_then(|rc| rc.max_commits)
            .unwrap_or(self.config.max_commits)
    }

    pub fn get_current_resync_on_tab_change(&self) -> bool {
        self.get_selected_item()
            .and_then(|path| self.config.repo_configs.get(path))
            .and_then(|rc| rc.resync_on_tab_change)
            .unwrap_or(self.config.resync_on_tab_change)
    }

    pub fn get_selected_item_index(&self) -> Option<usize> {
        let rows = self.get_home_rows();
        let row = rows.get(self.selected_index)?;
        match row {
            HomeRow::Repo { actual_index, .. } => Some(*actual_index),
            HomeRow::GroupHeader { .. } => None,
        }
    }

    /// Ensure `selected_index` is a valid index into `config.items` (or filtered items).
    pub fn clamp_selection(&mut self) {
        let len = self.get_items_len();
        if len == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= len {
            self.selected_index = len - 1;
        }
    }

    /// Ensure the scroll window doesn't extend past the end of the list.
    pub fn clamp_scroll(&mut self, visible_count: usize) {
        let max_scroll = self.get_items_len().saturating_sub(visible_count);
        if self.scroll_top > max_scroll {
            self.scroll_top = max_scroll;
        }
    }

    /// Clamp the help scroll value so it doesn't go out of bounds.
    pub fn clamp_help_scroll(&mut self, height: usize) {
        let (percent_y, lines_len) = match self.mode {
            Mode::Help => {
                let width = crossterm::terminal::size().map(|s| s.0).unwrap_or(80);
                (70, crate::popups::help::get_help_lines_len(self, width))
            }
            Mode::DetailHelp => {
                let width = crossterm::terminal::size().map(|s| s.0).unwrap_or(80);
                (55, crate::popups::detail_help::get_detail_help_lines_len(self, width))
            }
            _ => return,
        };
        let popup_height = (height * percent_y) / 100;
        let inner_height = popup_height.saturating_sub(2);
        let max_scroll = lines_len.saturating_sub(inner_height);
        if self.help_scroll > max_scroll {
            self.help_scroll = max_scroll;
        }
    }

    /// Clamp the legend scroll value so it doesn't go out of bounds.
    pub fn clamp_legend_scroll(&mut self) {
        let lines_len = crate::popups::legend::get_legend_lines_len(self);
        let inner_height = 14;
        let max_scroll = lines_len.saturating_sub(inner_height);
        if self.legend_scroll > max_scroll {
            self.legend_scroll = max_scroll;
        }
    }

    pub fn move_down(&mut self, visible_count: usize) {
        let len = self.get_items_len();
        if self.selected_index + 1 < len {
            self.selected_index += 1;
            let bottom = self.scroll_top + visible_count;
            if self.selected_index >= bottom {
                self.scroll_top = self.scroll_top.saturating_add(1);
            }
        }
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            if self.selected_index < self.scroll_top {
                self.scroll_top = self.scroll_top.saturating_sub(1);
            }
        }
    }

    /// Jump the selection forward by one page (= `visible_count` items).
    /// The scroll window advances by the same amount so the newly selected
    /// item is always at the top of the visible area.
    pub fn page_down(&mut self, visible_count: usize) {
        let len = self.get_items_len();
        let last = len.saturating_sub(1);
        self.selected_index = (self.selected_index + visible_count).min(last);
        // Align scroll so the selection lands at the top of the viewport,
        // then let clamp_scroll cap it at the list end.
        self.scroll_top = self.selected_index;
    }

    /// Jump the selection backward by one page (= `visible_count` items).
    pub fn page_up(&mut self, visible_count: usize) {
        self.selected_index = self.selected_index.saturating_sub(visible_count);
        self.scroll_top = self.selected_index;
    }

    pub fn move_to_top(&mut self) {
        self.selected_index = 0;
        self.scroll_top = 0;
    }

    pub fn move_to_bottom(&mut self, visible_count: usize) {
        let len = self.get_items_len();
        if len > 0 {
            self.selected_index = len - 1;
            self.scroll_top = self.selected_index.saturating_sub(visible_count - 1);
        }
    }

    pub fn open_help(&mut self) {
        self.help_scroll = 0;
        self.mode = Mode::Help;
    }

    pub fn open_about(&mut self) {
        self.mode = Mode::About;
    }

    /// Re-runs the cheap filesystem inspection for the selected item and
    /// updates its status indicator. Surfaces a transient "Refreshed" /
    /// "Refresh failed" message in the status bar so the user knows the
    /// keystroke landed (the indicator alone may not visibly change).
    pub fn refresh_selected_status(&mut self) {
        crate::debug_log::info("Refreshing selected repository status");
        let Some(orig_idx) = self.get_selected_item_index() else {
            return;
        };
        let Some(item) = self.config.items.get(orig_idx) else {
            return;
        };
        let new_status = repo::inspect_summary(item);
        if let Some(slot) = self.statuses.get_mut(orig_idx) {
            *slot = new_status;
        }
        self.status_message = Some("Refreshed".to_string());
    }

    pub fn sort_items_in_place(&mut self) {
        let mut zipped: Vec<(String, ItemStatus)> = match self.config.sort_by {
            SortOrder::Custom => {
                let mut status_map: std::collections::HashMap<String, ItemStatus> =
                    self.config.items.drain(..).zip(self.statuses.drain(..)).collect();
                let mut z: Vec<(String, ItemStatus)> = self
                    .original_items
                    .iter()
                    .map(|item| {
                        let status =
                            status_map.remove(item).unwrap_or_else(|| repo::inspect_summary(item));
                        (item.clone(), status)
                    })
                    .collect();
                if self.config.sort_reverse {
                    z.reverse();
                }
                z
            }
            SortOrder::Alphabetical => {
                let mut z: Vec<(String, ItemStatus)> =
                    self.config.items.drain(..).zip(self.statuses.drain(..)).collect();
                z.sort_by(|a, b| {
                    let name_a = std::path::Path::new(&a.0)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&a.0)
                        .to_lowercase();
                    let name_b = std::path::Path::new(&b.0)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&b.0)
                        .to_lowercase();
                    name_a.cmp(&name_b)
                });
                if self.config.sort_reverse {
                    z.reverse();
                }
                z
            }
            SortOrder::RecentVisit => {
                let visits = &self.config.visits;
                let mut z: Vec<(String, ItemStatus)> =
                    self.config.items.drain(..).zip(self.statuses.drain(..)).collect();
                z.sort_by(|a, b| {
                    let time_a = visits.get(&a.0).copied().unwrap_or(0);
                    let time_b = visits.get(&b.0).copied().unwrap_or(0);
                    time_b.cmp(&time_a) // Descending
                });
                if self.config.sort_reverse {
                    z.reverse();
                }
                z
            }
            SortOrder::LatestChanges => {
                let mut z: Vec<(String, ItemStatus)> =
                    self.config.items.drain(..).zip(self.statuses.drain(..)).collect();
                z.sort_by(|a, b| {
                    let time_a = repo::get_latest_change_time(&a.0);
                    let time_b = repo::get_latest_change_time(&b.0);
                    time_b.cmp(&time_a) // Descending
                });
                if self.config.sort_reverse {
                    z.reverse();
                }
                z
            }
        };

        // Stable-partition pinned items to the top
        zipped.sort_by_key(|(item, _)| !self.config.pinned.contains(item));

        let (items, statuses): (Vec<String>, Vec<ItemStatus>) = zipped.into_iter().unzip();
        self.config.items = items;
        self.statuses = statuses;
    }

    pub fn cycle_sort_order(&mut self) {
        self.config.sort_by = match self.config.sort_by {
            SortOrder::Custom => SortOrder::Alphabetical,
            SortOrder::Alphabetical => SortOrder::RecentVisit,
            SortOrder::RecentVisit => SortOrder::LatestChanges,
            SortOrder::LatestChanges => SortOrder::Custom,
        };

        let selected_item = self.get_selected_item().cloned();

        self.sort_items_in_place();

        if let Some(item) = selected_item {
            let filtered = self.get_filtered_items();
            if let Some(pos) = filtered.iter().position(|(_, x)| *x == &item) {
                self.selected_index = pos;
            }
        }

        self.persist("Sort mode updated");
    }

    pub fn toggle_sort_reverse(&mut self) {
        self.config.sort_reverse = !self.config.sort_reverse;

        let selected_item = self.get_selected_item().cloned();

        self.sort_items_in_place();

        if let Some(item) = selected_item {
            let filtered = self.get_filtered_items();
            if let Some(pos) = filtered.iter().position(|(_, x)| *x == &item) {
                self.selected_index = pos;
            }
        }

        self.persist("Sort direction updated");
    }

    pub fn toggle_pin_selected(&mut self) {
        let Some(selected_item) = self.get_selected_item().cloned() else {
            return;
        };
        if self.config.pinned.contains(&selected_item) {
            self.config.pinned.remove(&selected_item);
            self.status_message = Some("Unpinned repository".to_string());
        } else {
            self.config.pinned.insert(selected_item.clone());
            self.status_message = Some("Pinned repository".to_string());
        }

        self.sort_items_in_place();

        let filtered = self.get_filtered_items();
        if let Some(pos) = filtered.iter().position(|(_, x)| *x == &selected_item) {
            self.selected_index = pos;
        }

        let msg = self.status_message.as_deref().unwrap_or("Saved").to_string();
        self.persist(&msg);
    }

    pub fn toggle_star_selected(&mut self) {
        let Some(selected_item) = self.get_selected_item().cloned() else {
            return;
        };
        if self.config.starred.contains(&selected_item) {
            self.config.starred.remove(&selected_item);
            self.status_message = Some("Removed star from repository".to_string());
        } else {
            self.config.starred.insert(selected_item.clone());
            self.status_message = Some("Starred repository".to_string());
        }

        self.sort_items_in_place();

        // Keep the selection on the starred repo
        let rows = self.get_home_rows();
        if let Some(pos) = rows.iter().position(|r| match r {
            HomeRow::Repo { path, .. } => path == &selected_item,
            _ => false,
        }) {
            self.selected_index = pos;
        }

        let msg = self.status_message.as_deref().unwrap_or("Saved").to_string();
        self.persist(&msg);
    }

    /// Snapshot the selected item's filesystem/git state and enter the
    /// Detail view. The snapshot is held in `current_detail` for as long
    /// as the view is open; closing clears it.
    pub fn open_detail(&mut self) {
        if let Some(item) = self.get_selected_item().cloned() {
            crate::debug_log::info(format!("Opening detail view for repository: {}", item));
            // Update visit time
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            self.config.visits.insert(item.clone(), now);
            let _ = save_config(&self.config, &self.config_path);

            if self.config.sort_by == SortOrder::RecentVisit {
                self.sort_items_in_place();
                let filtered = self.get_filtered_items();
                if let Some(pos) = filtered.iter().position(|(_, x)| *x == &item) {
                    self.selected_index = pos;
                }
            }

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
                if let Some(cached) = self.detail_cache.get(&item).cloned() {
                    let cached_commits_count = match &cached.detail {
                        repo::ItemDetail::Repo { info, .. } => info.commits.len(),
                        _ => 200,
                    };
                    self.commit_list.limit = if self.get_current_max_commits() > 0 {
                        cached_commits_count.max(self.get_current_max_commits())
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
                self.commit_list.limit = self.get_current_max_commits();
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
            self.commit_list.selection = 0;
            self.detail_tab = 0;
            self.graph_scroll = 0;
            self.graph_selection = 0;
            self.inspect_full_diff = false;
            self.commit_popup.maximized = false;
            self.mode = Mode::Detail;
        }
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
            let path = std::path::PathBuf::from(&item);
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
                info.committer_stats = repo::TabData::NotLoaded;
                info.remote_tags_loaded = false;
                info.remote_tags_attempted = false;
                info.tab_loaded_at = [None; 10];
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
                            path_str.clone(),
                            tab_idx,
                            tx_clone,
                        );
                        let _ = tx.send((path_str, tab_idx, repo::TabPayload::Graph(res)));
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
    pub fn local_branch_up(&mut self) {
        self.branch_list.local_branch_selection =
            self.branch_list.local_branch_selection.saturating_sub(1);
    }

    /// Move local branch selection down.
    pub fn local_branch_down(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.local_branches.len();
            if total > 0 && self.branch_list.local_branch_selection + 1 < total {
                self.branch_list.local_branch_selection += 1;
            }
        }
    }

    /// Scroll local branch selection up by page.
    pub fn local_branch_page_up(&mut self, page: usize) {
        self.branch_list.local_branch_selection =
            self.branch_list.local_branch_selection.saturating_sub(page);
    }

    /// Scroll local branch selection down by page.
    pub fn local_branch_page_down(&mut self, page: usize) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.local_branches.len();
            if total > 0 {
                self.branch_list.local_branch_selection =
                    (self.branch_list.local_branch_selection + page).min(total.saturating_sub(1));
            }
        }
    }

    /// Move remote branch selection up.
    pub fn remote_branch_up(&mut self) {
        self.branch_list.remote_branch_selection =
            self.branch_list.remote_branch_selection.saturating_sub(1);
    }

    /// Move remote branch selection down.
    pub fn remote_branch_down(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.remote_branches.len();
            if total > 0 && self.branch_list.remote_branch_selection + 1 < total {
                self.branch_list.remote_branch_selection += 1;
            }
        }
    }

    /// Scroll remote branch selection up by page.
    pub fn remote_branch_page_up(&mut self, page: usize) {
        self.branch_list.remote_branch_selection =
            self.branch_list.remote_branch_selection.saturating_sub(page);
    }

    /// Scroll remote branch selection down by page.
    pub fn remote_branch_page_down(&mut self, page: usize) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.remote_branches.len();
            if total > 0 {
                self.branch_list.remote_branch_selection =
                    (self.branch_list.remote_branch_selection + page).min(total.saturating_sub(1));
            }
        }
    }

    /// Move file selection up in the Files tab.
    pub fn file_list_up(&mut self) {
        self.file_tree.file_list_selection = self.file_tree.file_list_selection.saturating_sub(1);
        self.file_tree.file_content_scroll = 0;
    }

    /// Move file selection down in the Files tab.
    pub fn file_list_down(&mut self) {
        let total = self.file_tree.visible_files.len();
        if total > 0 && self.file_tree.file_list_selection + 1 < total {
            self.file_tree.file_list_selection += 1;
            self.file_tree.file_content_scroll = 0;
        }
    }

    /// Scroll file selection up by page.
    pub fn file_list_page_up(&mut self, page: usize) {
        self.file_tree.file_list_selection =
            self.file_tree.file_list_selection.saturating_sub(page);
        self.file_tree.file_content_scroll = 0;
    }

    /// Scroll file selection down by page.
    pub fn file_list_page_down(&mut self, page: usize) {
        let total = self.file_tree.visible_files.len();
        if total > 0 {
            self.file_tree.file_list_selection =
                (self.file_tree.file_list_selection + page).min(total.saturating_sub(1));
            self.file_tree.file_content_scroll = 0;
        }
    }

    pub fn local_branch_to_top(&mut self) {
        self.branch_list.local_branch_selection = 0;
    }

    pub fn local_branch_to_bottom(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.local_branches.len();
            if total > 0 {
                self.branch_list.local_branch_selection = total - 1;
            }
        }
    }

    pub fn remote_branch_to_top(&mut self) {
        self.branch_list.remote_branch_selection = 0;
    }

    pub fn remote_branch_to_bottom(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.remote_branches.len();
            if total > 0 {
                self.branch_list.remote_branch_selection = total - 1;
            }
        }
    }

    pub fn file_list_to_top(&mut self) {
        self.file_tree.file_list_selection = 0;
        self.file_tree.file_content_scroll = 0;
    }

    pub fn file_list_to_bottom(&mut self) {
        let total = self.file_tree.visible_files.len();
        if total > 0 {
            self.file_tree.file_list_selection = total - 1;
            self.file_tree.file_content_scroll = 0;
        }
    }

    fn get_logs_matching_indices(&self) -> Vec<usize> {
        if !self.in_logs_ui || self.commit_list.search_query.is_none() {
            return Vec::new();
        }
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => info
                .commits
                .iter()
                .enumerate()
                .filter(|(_, c)| self.commit_matches_query(c))
                .map(|(i, _)| i)
                .collect(),
            _ => Vec::new(),
        }
    }

    fn get_logs_nav_index(&self, direction: LogsNavDirection) -> Option<usize> {
        let matching_indices = self.get_logs_matching_indices();
        if matching_indices.is_empty() {
            return None;
        }

        let pos_opt = matching_indices.iter().position(|&idx| idx >= self.commit_list.selection);

        match direction {
            LogsNavDirection::Down => {
                if let Some(pos) = pos_opt {
                    if matching_indices[pos] == self.commit_list.selection {
                        if pos + 1 < matching_indices.len() {
                            Some(matching_indices[pos + 1])
                        } else {
                            Some(matching_indices[pos])
                        }
                    } else {
                        Some(matching_indices[pos])
                    }
                } else {
                    matching_indices.last().copied()
                }
            }
            LogsNavDirection::Up => {
                if let Some(pos) = pos_opt {
                    if pos > 0 {
                        Some(matching_indices[pos - 1])
                    } else {
                        Some(matching_indices[0])
                    }
                } else {
                    matching_indices.last().copied()
                }
            }
            LogsNavDirection::PageDown(page) => {
                if let Some(pos) = pos_opt {
                    let target_pos = if matching_indices[pos] == self.commit_list.selection {
                        pos + page
                    } else {
                        pos + page - 1
                    };
                    let final_pos = target_pos.min(matching_indices.len() - 1);
                    Some(matching_indices[final_pos])
                } else {
                    matching_indices.last().copied()
                }
            }
            LogsNavDirection::PageUp(page) => {
                if let Some(pos) = pos_opt {
                    let target_pos = pos.saturating_sub(page);
                    Some(matching_indices[target_pos])
                } else {
                    let last_pos = matching_indices.len() - 1;
                    let target_pos = last_pos.saturating_sub(page);
                    Some(matching_indices[target_pos])
                }
            }
        }
    }

    /// Move commit selection up one row.
    pub fn detail_commit_up(&mut self) {
        if let Some(next_idx) = self.get_logs_nav_index(LogsNavDirection::Up) {
            self.commit_list.selection = next_idx;
        } else {
            self.commit_list.selection = self.commit_list.selection.saturating_sub(1);
        }
        self.status_list.file_selection = 0;
        self.diff.diff_scroll = 0;
        self.refresh_file_diff();
    }

    /// Move commit selection down one row, clamped to the last visible row.
    pub fn detail_commit_down(&mut self) {
        if let Some(next_idx) = self.get_logs_nav_index(LogsNavDirection::Down) {
            self.commit_list.selection = next_idx;
        } else {
            let total = self.commit_total();
            if total > 0 && self.commit_list.selection + 1 < total {
                self.commit_list.selection += 1;
            }
        }
        self.status_list.file_selection = 0;
        self.diff.diff_scroll = 0;
        self.refresh_file_diff();
    }

    /// Jump commit selection up by `page` rows.
    pub fn detail_commit_page_up(&mut self, page: usize) {
        if let Some(next_idx) = self.get_logs_nav_index(LogsNavDirection::PageUp(page)) {
            self.commit_list.selection = next_idx;
        } else {
            self.commit_list.selection = self.commit_list.selection.saturating_sub(page);
        }
        self.status_list.file_selection = 0;
        self.diff.diff_scroll = 0;
        self.refresh_file_diff();
    }

    /// Jump commit selection down by `page` rows, clamped to the last row.
    pub fn detail_commit_page_down(&mut self, page: usize) {
        if let Some(next_idx) = self.get_logs_nav_index(LogsNavDirection::PageDown(page)) {
            self.commit_list.selection = next_idx;
        } else {
            let total = self.commit_total();
            if total > 0 {
                self.commit_list.selection = (self.commit_list.selection + page).min(total - 1);
            }
        }
        self.status_list.file_selection = 0;
        self.diff.diff_scroll = 0;
        self.refresh_file_diff();
    }

    /// Move file selection up one row in the Changed Files panel.
    pub fn detail_file_up(&mut self) {
        self.status_list.file_selection = self.status_list.file_selection.saturating_sub(1);
        self.diff.diff_scroll = 0;
        self.refresh_file_diff();
    }

    /// Move file selection down one row in the Changed Files panel.
    pub fn detail_file_down(&mut self) {
        let total = self.file_total();
        if total > 0 && self.status_list.file_selection + 1 < total {
            self.status_list.file_selection += 1;
        }
        self.diff.diff_scroll = 0;
        self.refresh_file_diff();
    }

    /// Move staging-area file selection up one row (Staged or Unstaged panel).
    pub fn staging_file_up(&mut self) {
        self.status_list.staging_file_selection =
            self.status_list.staging_file_selection.saturating_sub(1);
        self.diff.diff_scroll = 0;
        self.refresh_staging_diff();
    }

    /// Move staging-area file selection down one row (Staged or Unstaged panel).
    pub fn staging_file_down(&mut self) {
        let total = self.staging_file_total();
        if total > 0 && self.status_list.staging_file_selection + 1 < total {
            self.status_list.staging_file_selection += 1;
        }
        self.diff.diff_scroll = 0;
        self.refresh_staging_diff();
    }

    /// Move conflict-area file selection up one row.
    pub fn conflict_file_up(&mut self) {
        self.status_list.conflict_file_selection =
            self.status_list.conflict_file_selection.saturating_sub(1);
        self.diff.diff_scroll = 0;
        self.refresh_staging_diff();
    }

    /// Move conflict-area file selection down one row.
    pub fn conflict_file_down(&mut self) {
        let total = match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => info.changes.conflicted.len(),
            _ => 0,
        };
        if total > 0 && self.status_list.conflict_file_selection + 1 < total {
            self.status_list.conflict_file_selection += 1;
        }
        self.diff.diff_scroll = 0;
        self.refresh_staging_diff();
    }

    pub fn diff_hunk_up(&mut self) {
        if self.diff.diff_hunk_selection > 0 {
            self.diff.diff_hunk_selection -= 1;
            self.scroll_to_selected_hunk();
        }
    }

    pub fn diff_hunk_down(&mut self) {
        let hunk_count = self.get_diff_hunk_ranges().len();
        if self.diff.diff_hunk_selection + 1 < hunk_count {
            self.diff.diff_hunk_selection += 1;
            self.scroll_to_selected_hunk();
        }
    }

    pub fn scroll_to_selected_hunk(&mut self) {
        let ranges = self.get_diff_hunk_ranges();
        if let Some(range) = ranges.get(self.diff.diff_hunk_selection) {
            self.diff.diff_scroll = range.start;
        }
    }

    pub fn diff_line_up(&mut self) {
        if self.diff.diff_line_selection > 0 {
            self.diff.diff_line_selection -= 1;
            let ranges = self.get_diff_hunk_ranges();
            for (idx, range) in ranges.iter().enumerate() {
                if range.contains(&self.diff.diff_line_selection) {
                    self.diff.diff_hunk_selection = idx;
                    break;
                }
            }
            if self.diff.diff_line_selection < self.diff.diff_scroll {
                self.diff.diff_scroll = self.diff.diff_line_selection;
            }
        }
    }

    pub fn diff_line_down(&mut self) {
        if self.diff.diff_line_selection + 1 < self.diff.file_diff.len() {
            self.diff.diff_line_selection += 1;
            let ranges = self.get_diff_hunk_ranges();
            for (idx, range) in ranges.iter().enumerate() {
                if range.contains(&self.diff.diff_line_selection) {
                    self.diff.diff_hunk_selection = idx;
                    break;
                }
            }
            if self.diff.diff_line_selection >= self.diff.diff_scroll + 18 {
                self.diff.diff_scroll = self.diff.diff_line_selection.saturating_sub(17);
            }
        }
    }

    pub fn refresh_detail_for_line_action(&mut self) {
        let prev_line_idx = self.diff.diff_line_selection;
        self.refresh_detail();

        let new_len = self.diff.file_diff.len();
        if new_len == 0 {
            self.diff.diff_line_selection = 0;
            self.diff.diff_hunk_selection = 0;
            self.diff.diff_scroll = 0;
            return;
        }

        self.diff.diff_line_selection = prev_line_idx.min(new_len - 1);
        let ranges = self.get_diff_hunk_ranges();
        for (idx, range) in ranges.iter().enumerate() {
            if range.contains(&self.diff.diff_line_selection) {
                self.diff.diff_hunk_selection = idx;
                break;
            }
        }

        if self.diff.diff_line_selection < self.diff.diff_scroll {
            self.diff.diff_scroll = self.diff.diff_line_selection;
        } else if self.diff.diff_line_selection >= self.diff.diff_scroll + 18 {
            self.diff.diff_scroll = self.diff.diff_line_selection.saturating_sub(17);
        }
    }

    pub fn get_file_content_line_count(&self) -> usize {
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
            if let Some(selected_item) =
                self.file_tree.visible_files.get(self.file_tree.file_list_selection)
            {
                if selected_item.is_dir {
                    let prefix = if selected_item.full_path.is_empty() {
                        "".to_string()
                    } else {
                        format!("{}/", selected_item.full_path)
                    };
                    let mut direct_children = std::collections::BTreeSet::new();
                    for f_path in info.files.iter() {
                        if f_path.starts_with(&prefix) {
                            let relative = &f_path[prefix.len()..];
                            if let Some(idx) = relative.find('/') {
                                let subdir = &relative[..idx];
                                direct_children.insert((subdir.to_string(), true));
                            } else {
                                direct_children.insert((relative.to_string(), false));
                            }
                        }
                    }
                    if direct_children.is_empty() { 1 } else { direct_children.len() }
                } else {
                    let file_path = resolved.join(&selected_item.full_path);
                    match std::fs::File::open(&file_path) {
                        Ok(file) => {
                            use std::io::Read;
                            let mut buffer = Vec::new();
                            if file.take(100_000).read_to_end(&mut buffer).is_ok() {
                                if let Ok(s) = String::from_utf8(buffer) {
                                    s.lines().count()
                                } else {
                                    1
                                }
                            } else {
                                1
                            }
                        }
                        Err(_) => 1,
                    }
                }
            } else {
                1
            }
        } else {
            1
        }
    }

    pub fn overview_scroll_up(&mut self) {
        match self.overview_focus {
            OverviewFocus::Overview => {
                self.overview_scroll = self.overview_scroll.saturating_sub(1);
            }
            OverviewFocus::Stats => {
                self.stats_scroll = self.stats_scroll.saturating_sub(1);
            }
        }
    }

    pub fn overview_scroll_down(&mut self) {
        match self.overview_focus {
            OverviewFocus::Overview => {
                self.overview_scroll = self.overview_scroll.saturating_add(1);
            }
            OverviewFocus::Stats => {
                self.stats_scroll = self.stats_scroll.saturating_add(1);
            }
        }
    }

    pub fn overview_scroll_page_up(&mut self, page: usize) {
        match self.overview_focus {
            OverviewFocus::Overview => {
                self.overview_scroll = self.overview_scroll.saturating_sub(page);
            }
            OverviewFocus::Stats => {
                self.stats_scroll = self.stats_scroll.saturating_sub(page);
            }
        }
    }

    pub fn overview_scroll_page_down(&mut self, page: usize) {
        match self.overview_focus {
            OverviewFocus::Overview => {
                self.overview_scroll = self.overview_scroll.saturating_add(page);
            }
            OverviewFocus::Stats => {
                self.stats_scroll = self.stats_scroll.saturating_add(page);
            }
        }
    }

    pub fn overview_scroll_to_top(&mut self) {
        match self.overview_focus {
            OverviewFocus::Overview => {
                self.overview_scroll = 0;
            }
            OverviewFocus::Stats => {
                self.stats_scroll = 0;
            }
        }
    }

    pub fn overview_scroll_to_bottom(&mut self) {
        match self.overview_focus {
            OverviewFocus::Overview => {
                self.overview_scroll = 99999;
            }
            OverviewFocus::Stats => {
                self.stats_scroll = 99999;
            }
        }
    }

    /// Scroll the file content panel up by one line.
    pub fn file_content_scroll_up(&mut self) {
        self.file_tree.file_content_scroll = self.file_tree.file_content_scroll.saturating_sub(1);
    }

    /// Scroll the file content panel down by one line.
    pub fn file_content_scroll_down(&mut self) {
        let max = self.get_file_content_line_count().saturating_sub(1);
        if self.file_tree.file_content_scroll < max {
            self.file_tree.file_content_scroll += 1;
        }
    }

    /// Scroll the file content panel up by `page` lines.
    pub fn file_content_scroll_page_up(&mut self, page: usize) {
        self.file_tree.file_content_scroll =
            self.file_tree.file_content_scroll.saturating_sub(page);
    }

    /// Scroll the file content panel down by `page` lines.
    pub fn file_content_scroll_page_down(&mut self, page: usize) {
        let max = self.get_file_content_line_count().saturating_sub(1);
        self.file_tree.file_content_scroll = (self.file_tree.file_content_scroll + page).min(max);
    }

    /// Move graph selection up by one line.
    pub fn graph_select_up(&mut self) {
        if self.graph_selection > 0 {
            self.graph_selection -= 1;
            if self.graph_selection < self.graph_scroll {
                self.graph_scroll = self.graph_selection;
            }
        }
    }

    /// Move graph selection down by one line.
    pub fn graph_select_down(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.graph_lines.len();
            let visible_height = self.graph_visible_height.get();
            let visible_height =
                if visible_height > 0 { visible_height } else { self.get_current_page_size() };
            if total > 0 && self.graph_selection + 1 < total {
                self.graph_selection += 1;
                let bottom = self.graph_scroll + visible_height;
                if self.graph_selection >= bottom {
                    self.graph_scroll = self.graph_selection.saturating_sub(visible_height - 1);
                }
            }
        }
    }

    /// Move graph selection up by a page.
    pub fn graph_select_page_up(&mut self, page: usize) {
        self.graph_selection = self.graph_selection.saturating_sub(page);
        if self.graph_selection < self.graph_scroll {
            self.graph_scroll = self.graph_selection;
        }
    }

    /// Move graph selection down by a page.
    pub fn graph_select_page_down(&mut self, page: usize) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.graph_lines.len();
            let visible_height = self.graph_visible_height.get();
            let visible_height =
                if visible_height > 0 { visible_height } else { self.get_current_page_size() };
            if total > 0 {
                self.graph_selection = (self.graph_selection + page).min(total.saturating_sub(1));
                let bottom = self.graph_scroll + visible_height;
                if self.graph_selection >= bottom {
                    self.graph_scroll = self.graph_selection.saturating_sub(visible_height - 1);
                }
            }
        }
    }

    /// Scroll the commit details panel up by one line.
    pub fn commit_details_scroll_up(&mut self) {
        self.commit_list.details_scroll = self.commit_list.details_scroll.saturating_sub(1);
    }

    /// Scroll the commit details panel down by one line.
    pub fn commit_details_scroll_down(&mut self) {
        self.commit_list.details_scroll = self.commit_list.details_scroll.saturating_add(1);
    }

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

    pub(super) fn clamp_conflict_selection(&mut self) {
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
        self.input_buffer = self.commit_list.search_query.clone().unwrap_or_default();
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
            self.commit_popup.input_buffer.clear();
            self.commit_popup.editing = true;
            self.commit_popup.amend = false;
            self.commit_input_scroll = 0;
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
                    self.input_buffer = last_msg;
                }
            }
        }
    }

    pub fn toggle_commit_popup_maximized(&mut self) {
        self.commit_popup.maximized = !self.commit_popup.maximized;
    }

    pub fn commit_input_scroll_up(&mut self) {
        self.commit_input_scroll = self.commit_input_scroll.saturating_sub(1);
    }

    pub fn commit_input_scroll_down(&mut self) {
        self.commit_input_scroll = self.commit_input_scroll.saturating_add(1);
    }

    pub fn toggle_or_edit_setting(&mut self) {
        match self.settings_selected_index {
            0 => {
                self.settings_editing = true;
                self.input_buffer = self.config.poll_interval_ms.to_string();
            }
            1 => {
                self.config.sort_by = match self.config.sort_by {
                    SortOrder::Custom => SortOrder::Alphabetical,
                    SortOrder::Alphabetical => SortOrder::RecentVisit,
                    SortOrder::RecentVisit => SortOrder::LatestChanges,
                    SortOrder::LatestChanges => SortOrder::Custom,
                };
                if self.config.sort_by != SortOrder::Custom {
                    self.sort_items_in_place();
                }
                self.persist("Sort mode updated");
            }
            2 => {
                self.config.sort_reverse = !self.config.sort_reverse;
                if self.config.sort_by != SortOrder::Custom {
                    self.sort_items_in_place();
                }
                self.persist("Sort direction updated");
            }
            3 => {
                self.settings_theme_list = self.get_available_themes();
                self.settings_theme_index = self
                    .settings_theme_list
                    .iter()
                    .position(|t| t == &self.config.theme_name)
                    .unwrap_or(0);
                self.settings_editing = true;
            }
            4 => {
                self.settings_editing = true;
                self.input_buffer = self.config.scan.max_depth.to_string();
            }
            5 => {
                self.settings_editing = true;
                self.input_buffer = self.config.scan.start_dir.clone();
            }
            6 => {
                self.settings_editing = true;
                self.input_buffer = self.config.max_commits.to_string();
            }
            7 => {
                self.settings_editing = true;
                self.input_buffer = self.config.page_size.to_string();
            }
            8 => {
                self.settings_editing = true;
                self.input_buffer = self.config.scan.excludes.join(",");
            }
            9 => {
                self.settings_editing = true;
                self.input_buffer = self.config.git_app.clone();
            }
            10 => {
                self.config.scan.git_only = !self.config.scan.git_only;
                self.persist("Scan Git Only updated");
            }
            12 => {
                self.config.compatibility_mode = !self.config.compatibility_mode;
                self.persist("Compatibility Mode updated");
            }
            13 => {
                self.config.resync_on_tab_change = !self.config.resync_on_tab_change;
                self.persist("Resync on Tab Change updated");
            }
            58 => {
                self.config.show_grouping = !self.config.show_grouping;
                self.persist("Show Grouping updated");
            }
            55 => {
                self.config.ssh_strict_host_checking = !self.config.ssh_strict_host_checking;
                unsafe {
                    if self.config.ssh_strict_host_checking {
                        std::env::set_var("GITWIG_SSH_STRICT", "1");
                    } else {
                        std::env::set_var("GITWIG_SSH_STRICT", "0");
                    }
                }
                self.persist("SSH Strict Host Checking updated");
            }
            56 => {
                self.settings_editing = true;
                self.input_buffer = self.config.editor.clone();
            }
            60 => {
                self.settings_editing = true;
                self.input_buffer = self.config.auto_fetch_interval_mins.to_string();
            }
            61 => {
                self.settings_editing = true;
                self.input_buffer = self.config.watch_dirs.join(",");
            }
            62 => {
                self.config.show_system_stats = !self.config.show_system_stats;
                self.persist("Show CPU/MEM updated");
            }
            63 => {
                self.config.enable_commit_signatures = !self.config.enable_commit_signatures;
                self.persist("Commit signatures updated");
            }
            64 => {
                self.settings_editing = true;
                self.input_buffer = self.config.graph_max_commits.to_string();
            }
            65 => {
                self.settings_editing = true;
                self.input_buffer = self.config.detail_cache_ttl_secs.to_string();
            }
            66 => {
                self.settings_editing = true;
                self.input_buffer = self.config.tab_ttl_secs.to_string();
            }
            67 => {
                self.config.compact_view = !self.config.compact_view;
                self.persist("Compact view toggled");
            }
            idx if idx >= 14 => {
                if let Some(action) = crate::keybindings::Action::from_index(idx) {
                    self.settings_editing = true;
                    self.input_buffer = self.keybindings.get_action_keys(action).join(", ");
                }
            }
            _ => {}
        }
    }

    pub fn commit_settings_edit(&mut self) {
        let trimmed = self.input_buffer.trim();
        match self.settings_selected_index {
            0 => {
                if let Ok(val) = trimmed.parse::<u64>() {
                    if val >= 10 {
                        self.config.poll_interval_ms = val;
                        self.persist("Poll interval updated");
                        self.settings_editing = false;
                        self.input_buffer.clear();
                    } else {
                        self.status_message =
                            Some("Poll interval must be at least 10ms".to_string());
                    }
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            3 => {
                if self.settings_theme_index < self.settings_theme_list.len() {
                    let selected_theme =
                        self.settings_theme_list[self.settings_theme_index].clone();
                    self.config.theme_name = selected_theme.clone();
                    let themes_dir =
                        self.config_path.parent().unwrap_or(&self.config_path).join("themes");
                    let theme_path = themes_dir.join(format!("{}.theme", selected_theme));
                    if theme_path.exists() {
                        if let Ok(theme_contents) = std::fs::read_to_string(&theme_path) {
                            if let Ok(theme) =
                                toml::from_str::<crate::config::ThemeConfig>(&theme_contents)
                            {
                                self.config.theme = theme;
                                crate::ui::update_theme(&self.config.theme);
                                self.persist("Theme updated");
                                self.settings_editing = false;
                                return;
                            }
                        }
                    }
                    crate::ui::update_theme(&self.config.theme);
                    self.settings_editing = false;
                    self.persist("Theme updated");
                }
            }
            4 => {
                if let Ok(val) = trimmed.parse::<usize>() {
                    self.config.scan.max_depth = val;
                    self.persist("Scan max depth updated");
                    self.settings_editing = false;
                    self.input_buffer.clear();
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            5 => {
                self.config.scan.start_dir = trimmed.to_string();
                self.persist("Scan start directory updated");
                self.settings_editing = false;
                self.input_buffer.clear();
            }
            6 => {
                if let Ok(val) = trimmed.parse::<usize>() {
                    self.config.max_commits = val;
                    self.persist("Max commits updated");
                    self.settings_editing = false;
                    self.input_buffer.clear();
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            7 => {
                if let Ok(val) = trimmed.parse::<usize>() {
                    if val >= 1 {
                        self.config.page_size = val;
                        self.persist("Page size updated");
                        self.settings_editing = false;
                        self.input_buffer.clear();
                    } else {
                        self.status_message = Some("Page size must be at least 1".to_string());
                    }
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            8 => {
                self.config.scan.excludes = trimmed
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                self.persist("Scan exclude folders updated");
                self.settings_editing = false;
                self.input_buffer.clear();
            }
            9 => {
                let trimmed_app = trimmed.to_string();
                if !trimmed_app.is_empty() {
                    let allowed = ["git", "gitui", "lazygit"];
                    if allowed.contains(&trimmed_app.as_str()) {
                        self.config.git_app = trimmed_app;
                        self.persist("Preferred Git Client updated");
                        self.settings_editing = false;
                        self.input_buffer.clear();
                    } else {
                        self.status_message =
                            Some("Invalid client! Allowed values: git, gitui, lazygit".to_string());
                    }
                } else {
                    self.status_message = Some("Preferred Git Client cannot be empty".to_string());
                }
            }
            56 => {
                let trimmed_editor = trimmed.to_string();
                if !trimmed_editor.is_empty() {
                    self.config.editor = trimmed_editor;
                    self.persist("Editor Command updated");
                    self.settings_editing = false;
                    self.input_buffer.clear();
                } else {
                    self.status_message = Some("Editor Command cannot be empty".to_string());
                }
            }
            60 => {
                if let Ok(val) = trimmed.parse::<u64>() {
                    self.config.auto_fetch_interval_mins = val;
                    self.persist("Auto-fetch interval updated");
                    self.settings_editing = false;
                    self.input_buffer.clear();
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            61 => {
                self.config.watch_dirs = trimmed
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                self.persist("Watch directories updated");
                self.settings_editing = false;
                self.input_buffer.clear();
                self.setup_watcher();
            }
            64 => {
                if let Ok(val) = trimmed.parse::<usize>() {
                    self.config.graph_max_commits = val;
                    self.persist("Graph max commits updated");
                    self.settings_editing = false;
                    self.input_buffer.clear();
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            65 => {
                if let Ok(val) = trimmed.parse::<u64>() {
                    self.config.detail_cache_ttl_secs = val;
                    self.persist("Detail cache TTL updated");
                    self.settings_editing = false;
                    self.input_buffer.clear();
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            66 => {
                if let Ok(val) = trimmed.parse::<u64>() {
                    self.config.tab_ttl_secs = val;
                    self.persist("Tab cache TTL updated");
                    self.settings_editing = false;
                    self.input_buffer.clear();
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            idx if idx >= 14 => {
                if let Some(action) = crate::keybindings::Action::from_index(idx) {
                    let keys: Vec<String> = trimmed
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();

                    let mut all_valid = true;
                    for k in &keys {
                        if crate::keybindings::parse_key(k).is_none() {
                            all_valid = false;
                            break;
                        }
                    }

                    if all_valid {
                        self.keybindings.update_action_keys(action, keys);
                        let config_dir = self.config_path.parent().unwrap_or(&self.config_path);
                        if let Err(e) = self.keybindings.save(config_dir) {
                            self.status_message =
                                Some(format!("Failed to save keybindings: {}", e));
                        } else {
                            self.status_message = Some("Keybindings updated and saved".to_string());
                            self.settings_editing = false;
                            self.input_buffer.clear();
                        }
                    } else {
                        self.status_message =
                            Some("One or more key mappings are invalid".to_string());
                    }
                }
            }
            _ => {}
        }
    }

    pub fn cancel_settings_edit(&mut self) {
        self.settings_editing = false;
        self.input_buffer.clear();
    }

    pub fn get_available_themes(&self) -> Vec<String> {
        let mut themes = vec!["default".to_string()];
        let themes_dir = self.config_path.parent().unwrap_or(&self.config_path).join("themes");
        if themes_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(themes_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().is_some_and(|ext| ext == "theme") {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let theme_name = stem.to_string();
                            if theme_name != "default" && !themes.contains(&theme_name) {
                                themes.push(theme_name);
                            }
                        }
                    }
                }
            }
        }
        themes.sort();
        themes
    }

    pub fn cancel_input(&mut self) {
        self.input_buffer.clear();
        self.mode = Mode::Normal;
    }

    pub fn input_char(&mut self, c: char) {
        self.input_buffer.push(c);
    }

    pub fn input_backspace(&mut self) {
        self.input_buffer.pop();
    }

    pub(super) fn canonical_path(p: &std::path::Path) -> PathBuf {
        match std::fs::canonicalize(p) {
            Ok(canon) => canon,
            Err(_) => p.to_path_buf(),
        }
    }

    pub fn start_bulk_add(&mut self) {
        crate::debug_log::info("Initiating bulk repository add");
        self.start_bulk_repo_scan();
    }

    pub fn commit_bulk_add(&mut self) {
        let trimmed = self.input_buffer.trim().to_string();
        self.input_buffer.clear();
        self.mode = Mode::Normal;
        if !trimmed.is_empty() {
            self.bulk_add_path_with_labels(trimmed, vec![]);
        }
    }

    pub fn commit_bulk_add_label_input(&mut self) {
        let labels_str = self.input_buffer.trim().to_string();
        let labels: Vec<String> = if labels_str.is_empty() {
            Vec::new()
        } else {
            labels_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        };
        let path = self.pending_bulk_add_repo.take().unwrap_or_default();
        self.bulk_add_path_with_labels(path, labels);
    }

    pub fn bulk_add_path(&mut self, path: String) {
        self.bulk_add_path_with_labels(path, vec![]);
    }

    pub fn bulk_add_path_with_labels(&mut self, path: String, labels: Vec<String>) {
        let trimmed = path.trim().to_string();
        if trimmed.is_empty() {
            return;
        }

        let base_path = repo::expand_tilde(&trimmed);
        if !base_path.exists() {
            self.set_error(format!("Directory does not exist: {}", trimmed));
            return;
        }
        if !base_path.is_dir() {
            self.set_error(format!("Path is not a directory: {}", trimmed));
            return;
        }

        let entries = match std::fs::read_dir(&base_path) {
            Ok(read) => read,
            Err(e) => {
                self.set_error(format!("Failed to read directory: {}", e));
                return;
            }
        };

        let mut added_paths = Vec::new();
        let git_only = self.config.scan.git_only;

        for entry_opt in entries {
            let entry = match entry_opt {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if path.is_dir() {
                let show_dir = if git_only { path.join(".git").exists() } else { true };
                if show_dir {
                    if let Some(sub_name) = path.file_name().and_then(|n| n.to_str()) {
                        let mut base_str = trimmed.clone();
                        if !base_str.ends_with(std::path::MAIN_SEPARATOR) {
                            base_str.push(std::path::MAIN_SEPARATOR);
                        }
                        let path_to_add = format!("{}{}", base_str, sub_name);
                        added_paths.push(path_to_add);
                    }
                }
            }
        }

        added_paths.sort();

        if added_paths.is_empty() {
            self.status_message = Some("No matching directories found to add".to_string());
            return;
        }

        let mut newly_added_count = 0;
        let mut first_new_path = None;
        for path_str in added_paths {
            let trimmed_path = path_str.trim().to_string();
            let new_expanded = repo::expand_tilde(&trimmed_path);
            let new_canonical = Self::canonical_path(&new_expanded);

            let already_exists = self.config.items.iter().any(|item| {
                let item_expanded = repo::expand_tilde(item);
                item.trim() == trimmed_path
                    || item_expanded == new_expanded
                    || Self::canonical_path(&item_expanded) == new_canonical
            });

            if !already_exists {
                let status = repo::inspect_summary(&trimmed_path);
                self.statuses.push(status);
                self.config.items.push(trimmed_path.clone());
                self.original_items.push(trimmed_path.clone());
                if !labels.is_empty() {
                    self.config.labels.insert(trimmed_path.clone(), labels.clone());
                }
                if first_new_path.is_none() {
                    first_new_path = Some(trimmed_path);
                }
                newly_added_count += 1;
            }
        }

        if newly_added_count > 0 {
            self.sort_items_in_place();
            self.repo_search_query = None;
            if let Some(ref target) = first_new_path {
                if let Some(pos) = self.config.items.iter().position(|x| x == target) {
                    self.selected_index = pos;
                }
            }
            self.persist(&format!("Added {} directories", newly_added_count));
        } else {
            self.status_message = Some("All discovered directories were already added".to_string());
        }
    }

    pub fn start_import_clone(&mut self) {
        let url = self.import_url.trim().to_string();
        let dest = self.import_dest.trim().to_string();
        let name = self.import_name.trim().to_string();

        if url.is_empty() || dest.is_empty() {
            self.set_error("Source URL and Destination path cannot be empty".to_string());
            self.mode = Mode::Normal;
            return;
        }

        let mut dest_path = repo::expand_tilde(&dest);
        if !name.is_empty() {
            dest_path.push(&name);
        }
        let dest_str = dest_path.to_string_lossy().to_string();

        crate::debug_log::info(format!(
            "Network Action: Cloning remote repository {} to {}",
            url, dest_str
        ));
        self.fetching = true;
        self.status_message = Some(format!("Cloning {}...", url));
        self.mode = Mode::Normal;

        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let res = (|| -> Result<String, String> {
                let dest_expanded = repo::expand_tilde(&dest_str);
                let _ = std::fs::create_dir_all(&dest_expanded);

                let trimmed_url = url.trim().to_lowercase();
                if trimmed_url.contains("ext:") || trimmed_url.contains("fd:") {
                    return Err("Malicious URL protocol rejected".to_string());
                }

                let mut cmd = git_command();
                cmd.arg("clone").arg("--").arg(&url).arg(&dest_expanded);

                let output = cmd.output().map_err(|e| e.to_string())?;
                if output.status.success() {
                    Ok(format!("CLONE_SUCCESS:{}", dest_str))
                } else {
                    let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    Err(format!("Clone failed: {}", err))
                }
            })();

            match res {
                Ok(msg) => {
                    let _ = tx.send(msg);
                }
                Err(e) => {
                    let _ = tx.send(format!("Failed to clone: {}", e));
                }
            }
        });
    }

    pub fn close_dialog(&mut self) {
        self.mode = Mode::Normal;
    }

    /// Persists `self.config` and records a status message (success or
    /// the save error) for the next render.
    pub fn persist(&mut self, success_msg: &str) {
        self.resolve_repo_themes();
        self.status_message = match save_config(&self.config, &self.config_path) {
            Ok(()) => Some(success_msg.to_string()),
            Err(e) => Some(format!("Save failed: {}", e)),
        };
        self.setup_watcher();
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
                match repo::get_file_history(&repo_path, &file_path) {
                    Ok(revisions) => {
                        self.file_history_revisions = revisions;
                        self.file_history_selection = 0;
                        self.file_history_path = file_path;
                        self.file_history_diff = Vec::new();
                        self.file_history_diff_scroll = 0;
                        self.file_history_focus = 0;
                        self.mode = Mode::FileHistory;
                        self.refresh_file_history_diff();
                    }
                    Err(e) => {
                        self.set_error(format!("Failed to load file history: {}", e));
                    }
                }
            }
        }
    }

    pub fn refresh_file_history_diff(&mut self) {
        if self.file_history_revisions.is_empty() {
            self.file_history_diff = Vec::new();
            return;
        }
        let revision = &self.file_history_revisions[self.file_history_selection].clone();
        let repo_path = if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail
        {
            Some(resolved.clone())
        } else {
            None
        };
        if let Some(repo_path) = repo_path {
            self.file_history_diff = repo::get_commit_file_diff(
                &repo_path,
                &revision.commit_oid,
                &self.file_history_path,
            );
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
            _ => {}
        }
    }

    pub fn remote_picker_up(&mut self) {
        self.remote_picker_selection = self.remote_picker_selection.saturating_sub(1);
    }

    pub fn remote_picker_down(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.remotes.len();
            if total > 0 && self.remote_picker_selection + 1 < total {
                self.remote_picker_selection += 1;
            }
        }
    }

    pub fn local_tag_up(&mut self) {
        self.tag_list.local_tag_selection = self.tag_list.local_tag_selection.saturating_sub(1);
    }

    pub fn local_tag_down(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.local_tags.len();
            if total > 0 && self.tag_list.local_tag_selection + 1 < total {
                self.tag_list.local_tag_selection += 1;
            }
        }
    }

    pub fn local_tag_page_up(&mut self, page: usize) {
        self.tag_list.local_tag_selection = self.tag_list.local_tag_selection.saturating_sub(page);
    }

    pub fn local_tag_page_down(&mut self, page: usize) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.local_tags.len();
            if total > 0 {
                self.tag_list.local_tag_selection =
                    (self.tag_list.local_tag_selection + page).min(total.saturating_sub(1));
            }
        }
    }

    pub fn remote_up(&mut self) {
        self.branch_list.remote_selection = self.branch_list.remote_selection.saturating_sub(1);
    }

    pub fn remote_down(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.remotes.len();
            if total > 0 && self.branch_list.remote_selection + 1 < total {
                self.branch_list.remote_selection += 1;
            }
        }
    }

    pub fn remote_page_up(&mut self, page: usize) {
        self.branch_list.remote_selection = self.branch_list.remote_selection.saturating_sub(page);
    }

    pub fn remote_page_down(&mut self, page: usize) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.remotes.len();
            if total > 0 {
                self.branch_list.remote_selection =
                    (self.branch_list.remote_selection + page).min(total.saturating_sub(1));
            }
        }
    }

    pub fn stash_up(&mut self) {
        self.stash_list.stash_selection = self.stash_list.stash_selection.saturating_sub(1);
        self.stash_list.stash_file_selection = 0;
        self.refresh_file_diff();
    }

    pub fn stash_down(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.stashes.len();
            if total > 0 && self.stash_list.stash_selection + 1 < total {
                self.stash_list.stash_selection += 1;
                self.stash_list.stash_file_selection = 0;
                self.refresh_file_diff();
            }
        }
    }

    pub fn stash_page_up(&mut self, page: usize) {
        self.stash_list.stash_selection = self.stash_list.stash_selection.saturating_sub(page);
        self.stash_list.stash_file_selection = 0;
        self.refresh_file_diff();
    }

    pub fn stash_page_down(&mut self, page: usize) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.stashes.len();
            if total > 0 {
                self.stash_list.stash_selection =
                    (self.stash_list.stash_selection + page).min(total.saturating_sub(1));
                self.stash_list.stash_file_selection = 0;
                self.refresh_file_diff();
            }
        }
    }

    pub fn stash_file_up(&mut self) {
        self.stash_list.stash_file_selection =
            self.stash_list.stash_file_selection.saturating_sub(1);
        self.refresh_file_diff();
    }

    pub fn stash_file_down(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if let Some(stash) = info.stashes.get(self.stash_list.stash_selection) {
                let total = stash.files.len();
                if total > 0 && self.stash_list.stash_file_selection + 1 < total {
                    self.stash_list.stash_file_selection += 1;
                    self.refresh_file_diff();
                }
            }
        }
    }

    pub fn stash_file_page_up(&mut self, page: usize) {
        self.stash_list.stash_file_selection =
            self.stash_list.stash_file_selection.saturating_sub(page);
        self.refresh_file_diff();
    }

    pub fn stash_file_page_down(&mut self, page: usize) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if let Some(stash) = info.stashes.get(self.stash_list.stash_selection) {
                let total = stash.files.len();
                if total > 0 {
                    self.stash_list.stash_file_selection =
                        (self.stash_list.stash_file_selection + page).min(total.saturating_sub(1));
                    self.refresh_file_diff();
                }
            }
        }
    }

    pub fn local_tag_to_top(&mut self) {
        self.tag_list.local_tag_selection = 0;
    }

    pub fn local_tag_to_bottom(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.local_tags.len();
            if total > 0 {
                self.tag_list.local_tag_selection = total - 1;
            }
        }
    }

    pub fn remote_to_top(&mut self) {
        self.branch_list.remote_selection = 0;
    }

    pub fn remote_to_bottom(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.remotes.len();
            if total > 0 {
                self.branch_list.remote_selection = total - 1;
            }
        }
    }

    pub fn stash_to_top(&mut self) {
        self.stash_list.stash_selection = 0;
        self.stash_list.stash_file_selection = 0;
        self.refresh_file_diff();
    }

    pub fn stash_to_bottom(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.stashes.len();
            if total > 0 {
                self.stash_list.stash_selection = total - 1;
                self.stash_list.stash_file_selection = 0;
                self.refresh_file_diff();
            }
        }
    }

    pub fn stash_file_to_top(&mut self) {
        self.stash_list.stash_file_selection = 0;
        self.refresh_file_diff();
    }

    pub fn stash_file_to_bottom(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if let Some(stash) = info.stashes.get(self.stash_list.stash_selection) {
                let total = stash.files.len();
                if total > 0 {
                    self.stash_list.stash_file_selection = total - 1;
                    self.refresh_file_diff();
                }
            }
        }
    }

    pub fn graph_select_to_top(&mut self) {
        self.graph_selection = 0;
        self.graph_scroll = 0;
    }

    pub fn graph_select_to_bottom(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.graph_lines.len();
            let visible_height = self.graph_visible_height.get();
            let visible_height =
                if visible_height > 0 { visible_height } else { self.get_current_page_size() };
            if total > 0 {
                self.graph_selection = total.saturating_sub(1);
                self.graph_scroll = self.graph_selection.saturating_sub(visible_height - 1);
            }
        }
    }

    pub fn file_content_scroll_to_top(&mut self) {
        self.file_tree.file_content_scroll = 0;
    }

    pub fn file_content_scroll_to_bottom(&mut self) {
        let max = self.get_file_content_line_count().saturating_sub(1);
        self.file_tree.file_content_scroll = max;
    }

    pub fn detail_commit_to_top(&mut self) {
        if self.in_logs_ui && self.commit_list.search_query.is_some() {
            let matching_indices = self.get_logs_matching_indices();
            if let Some(&first) = matching_indices.first() {
                self.commit_list.selection = first;
            }
        } else {
            self.commit_list.selection = 0;
        }
        self.status_list.file_selection = 0;
        self.diff.diff_scroll = 0;
        self.refresh_file_diff();
    }

    pub fn detail_commit_to_bottom(&mut self) {
        if self.in_logs_ui && self.commit_list.search_query.is_some() {
            let matching_indices = self.get_logs_matching_indices();
            if let Some(&last) = matching_indices.last() {
                self.commit_list.selection = last;
            }
        } else {
            let total = self.commit_total();
            if total > 0 {
                self.commit_list.selection = total - 1;
            }
        }
        self.status_list.file_selection = 0;
        self.diff.diff_scroll = 0;
        self.refresh_file_diff();
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
                    self.status_message = Some(format!("Failed to copy to clipboard: {}", e));
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
                    self.status_message = Some(format!("Failed to copy to clipboard: {}", e));
                }
            }
        }
    }

    pub fn help_scroll_up(&mut self) {
        self.help_scroll = self.help_scroll.saturating_sub(1);
    }

    pub fn help_scroll_down(&mut self) {
        self.help_scroll = self.help_scroll.saturating_add(1);
    }

    pub fn help_scroll_page_up(&mut self, amount: usize) {
        self.help_scroll = self.help_scroll.saturating_sub(amount);
    }

    pub fn help_scroll_page_down(&mut self, amount: usize) {
        self.help_scroll = self.help_scroll.saturating_add(amount);
    }

    pub fn help_scroll_to_top(&mut self) {
        self.help_scroll = 0;
    }

    pub fn help_scroll_to_bottom(&mut self) {
        self.help_scroll = usize::MAX;
    }

    pub fn legend_scroll_up(&mut self) {
        self.legend_scroll = self.legend_scroll.saturating_sub(1);
    }

    pub fn legend_scroll_down(&mut self) {
        self.legend_scroll = self.legend_scroll.saturating_add(1);
    }

    pub fn legend_scroll_page_up(&mut self, amount: usize) {
        self.legend_scroll = self.legend_scroll.saturating_sub(amount);
    }

    pub fn legend_scroll_page_down(&mut self, amount: usize) {
        self.legend_scroll = self.legend_scroll.saturating_add(amount);
    }

    pub fn legend_scroll_to_top(&mut self) {
        self.legend_scroll = 0;
    }

    pub fn legend_scroll_to_bottom(&mut self) {
        self.legend_scroll = usize::MAX;
    }

    pub fn get_jump_matches(&self) -> Vec<(usize, String, String)> {
        let query = self.input_buffer.to_lowercase();
        if query.is_empty() {
            return self
                .config
                .items
                .iter()
                .enumerate()
                .map(|(idx, path)| {
                    let name = std::path::Path::new(path)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(path)
                        .to_string();
                    (idx, path.clone(), name)
                })
                .collect();
        }

        let mut matches = Vec::new();
        for (idx, path) in self.config.items.iter().enumerate() {
            let name =
                std::path::Path::new(path).file_name().and_then(|s| s.to_str()).unwrap_or(path);
            let name_lower = name.to_lowercase();
            let path_lower = path.to_lowercase();

            if name_lower.contains(&query) {
                let score = 1000 - (name_lower.len() - query.len());
                matches.push((score, idx, path.clone(), name.to_string()));
            } else if path_lower.contains(&query) {
                let score = 500 - (path_lower.len() - query.len());
                matches.push((score, idx, path.clone(), name.to_string()));
            } else {
                let mut name_chars = name_lower.chars();
                let mut matched = true;
                for qc in query.chars() {
                    if !name_chars.any(|nc| nc == qc) {
                        matched = false;
                        break;
                    }
                }
                if matched {
                    matches.push((100, idx, path.clone(), name.to_string()));
                }
            }
        }

        matches.sort_by(|a, b| b.0.cmp(&a.0).then(a.3.cmp(&b.3)));

        matches.into_iter().map(|(_, idx, path, name)| (idx, path, name)).collect()
    }

    pub fn jump_to_repo(&mut self, original_index: usize) {
        if let Some(path) = self.config.items.get(original_index) {
            if let Some(lbls) = self.config.labels.get(path) {
                for label in lbls {
                    self.collapsed_groups.remove(label);
                }
            } else {
                self.collapsed_groups.remove("Unlabeled");
            }
            // Starred and Recent groups should be expanded too
            self.collapsed_groups.remove("Starred");
            self.collapsed_groups.remove("Recent");

            let rows = self.get_home_rows();
            if let Some(pos) = rows.iter().position(|r| match r {
                HomeRow::Repo { actual_index, .. } => *actual_index == original_index,
                _ => false,
            }) {
                self.selected_index = pos;

                // Calculate list_height dynamically from terminal size
                let list_height = if let Ok(size) = crossterm::terminal::size() {
                    let inner_vertical_margin = 2;
                    let inner_height = (size.1 as usize).saturating_sub(inner_vertical_margin);
                    let available_height =
                        inner_height.saturating_sub(self.status_height() as usize);
                    let mut lh = if self.config.compact_view {
                        available_height.saturating_sub(1)
                    } else {
                        available_height
                    };
                    if !self.config.items.is_empty() {
                        lh = lh.saturating_sub(2);
                    }
                    lh
                } else {
                    20
                };

                // Check if the selected row is already visible starting from scroll_top
                let mut accumulated = 0;
                let mut is_visible = false;
                if pos >= self.scroll_top {
                    for idx in self.scroll_top..=pos {
                        let h = match &rows[idx] {
                            HomeRow::GroupHeader { .. } => {
                                if self.config.compact_view {
                                    1
                                } else {
                                    2
                                }
                            }
                            HomeRow::Repo { .. } => {
                                if self.config.compact_view {
                                    1
                                } else {
                                    4
                                }
                            }
                        };
                        accumulated += h;
                    }
                    if accumulated <= list_height {
                        is_visible = true;
                    }
                }

                if !is_visible {
                    if pos < self.scroll_top {
                        self.scroll_top = pos;
                    } else {
                        // Scroll down to make it visible at the bottom of the viewport
                        let mut acc_height = 0;
                        let mut temp_scroll = pos;
                        while temp_scroll > 0 {
                            let h = match &rows[temp_scroll] {
                                HomeRow::GroupHeader { .. } => {
                                    if self.config.compact_view {
                                        1
                                    } else {
                                        2
                                    }
                                }
                                HomeRow::Repo { .. } => {
                                    if self.config.compact_view {
                                        1
                                    } else {
                                        4
                                    }
                                }
                            };
                            if acc_height + h <= list_height {
                                acc_height += h;
                                temp_scroll -= 1;
                            } else {
                                break;
                            }
                        }
                        if temp_scroll == 0 {
                            let h = match &rows[0] {
                                HomeRow::GroupHeader { .. } => {
                                    if self.config.compact_view {
                                        1
                                    } else {
                                        2
                                    }
                                }
                                HomeRow::Repo { .. } => {
                                    if self.config.compact_view {
                                        1
                                    } else {
                                        4
                                    }
                                }
                            };
                            if acc_height + h <= list_height {
                                self.scroll_top = 0;
                            } else {
                                self.scroll_top = 1;
                            }
                        } else {
                            self.scroll_top = temp_scroll + 1;
                        }
                    }
                }
            }
        }
        self.input_buffer.clear();
        self.mode = Mode::Normal;
    }
}

fn git_command() -> std::process::Command {
    let mut cmd = std::process::Command::new("git");
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd.env("GIT_SSH_COMMAND", crate::config::ssh_command_val());
    cmd.env("GIT_ALLOW_PROTOCOL", "https:ssh:git:file");
    cmd.env("GIT_PROTOCOL_FROM_USER", "0");
    cmd
}

fn _safe_ref(r: &str) -> Result<&str, String> {
    let trimmed = r.trim();
    if trimmed.starts_with('-') {
        return Err(format!("Invalid ref name: '{}' (ref names cannot start with '-')", r));
    }
    if trimmed.is_empty() {
        return Err("Ref name cannot be empty".to_string());
    }
    Ok(trimmed)
}
