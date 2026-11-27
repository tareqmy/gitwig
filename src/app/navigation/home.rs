//! Home list rows, grouping, summary/label filtering, sorting, pinning, starring, archiving, and quick jump.

use super::super::*;

impl App {
    pub fn get_home_rows(&self) -> Vec<HomeRow> {
        let filtered = self.get_filtered_items();
        let mut recent_repos = Vec::new();
        for (actual_index, item) in filtered.iter() {
            if let Some(&time) = self.state.visits.get(*item) {
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

    /// For each home row, whether a thin rule is drawn under it to set the
    /// pinned repositories apart from the rest: the sort keeps pinned rows at
    /// the top of the list (and of each label group), so the rule marks where
    /// the pinned block ends. Only a block that opens its group gets one; the
    /// Recent and Starred groups, ordered by visit time and by name, can
    /// interleave pinned and unpinned rows and are left alone. None at all
    /// when `show_separators` is off.
    pub fn home_pin_separators(&self, rows: &[HomeRow]) -> Vec<bool> {
        let mut separators = vec![false; rows.len()];
        if !self.config.show_separators {
            return separators;
        }
        // Whether every repository row since the current group started is pinned.
        let mut pinned_block = true;
        for (i, row) in rows.iter().enumerate() {
            match row {
                HomeRow::GroupHeader { .. } => pinned_block = true,
                HomeRow::Repo { path, primary_label, .. } => {
                    if primary_label == "Recent" || primary_label == "Starred" {
                        pinned_block = false;
                        continue;
                    }
                    if self.config.pinned.contains(path) {
                        continue;
                    }
                    if pinned_block && i > 0 && matches!(rows[i - 1], HomeRow::Repo { .. }) {
                        separators[i - 1] = true;
                    }
                    pinned_block = false;
                }
            }
        }
        separators
    }

    pub fn get_active_items(&self) -> Vec<(usize, &String)> {
        let label_ok = |path: &String| match &self.state.active_label_filter {
            Some(label) => self.config.labels.get(path).is_some_and(|lbls| lbls.contains(label)),
            None => true,
        };
        if let Some(filter) = self.global_filter {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let stale_threshold = (self.config.stale_threshold_months as i64) * 30 * 24 * 60 * 60;

            self.config
                .items
                .iter()
                .enumerate()
                .filter(|&(idx, item)| {
                    if !label_ok(item) {
                        return false;
                    }
                    let is_archived = self.config.archived.contains(item);
                    let status = self.statuses.get(idx);
                    match filter {
                        GlobalFilter::Dirty => {
                            if is_archived {
                                return false;
                            }
                            if let Some(crate::repo::ItemStatus::GitRepo(Some(summary))) = status {
                                if !summary.is_clean() {
                                    if !self.config.show_stale_projects {
                                        if let Some(t) = summary.last_commit_time {
                                            return now - t <= stale_threshold;
                                        }
                                    }
                                    true
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }
                        GlobalFilter::Ahead => {
                            if is_archived {
                                return false;
                            }
                            if let Some(crate::repo::ItemStatus::GitRepo(Some(summary))) = status {
                                if summary.ahead > 0 {
                                    if !self.config.show_stale_projects {
                                        if let Some(t) = summary.last_commit_time {
                                            return now - t <= stale_threshold;
                                        }
                                    }
                                    true
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }
                        GlobalFilter::Stale => {
                            if is_archived {
                                return false;
                            }
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
                        GlobalFilter::Archived => is_archived,
                    }
                })
                .collect()
        } else {
            let mut items: Vec<(usize, &String)> = self.config.items.iter().enumerate().collect();
            items.retain(|&(_, path)| label_ok(path) && !self.config.archived.contains(path));
            if !self.config.show_stale_projects {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                let stale_threshold =
                    (self.config.stale_threshold_months as i64) * 30 * 24 * 60 * 60;
                items.retain(|&(idx, _)| {
                    if let Some(crate::repo::ItemStatus::GitRepo(Some(summary))) =
                        self.statuses.get(idx)
                    {
                        if let Some(t) = summary.last_commit_time {
                            now - t <= stale_threshold
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                });
            }
            items
        }
    }

    /// Steps the summary-bar filter through repos (no filter) → dirty → ahead →
    /// stale → archived, wrapping around; `backward` walks the same ring in reverse.
    pub fn cycle_global_filter(&mut self, backward: bool) {
        self.global_filter = if backward {
            match self.global_filter {
                None => Some(GlobalFilter::Archived),
                Some(GlobalFilter::Archived) => Some(GlobalFilter::Stale),
                Some(GlobalFilter::Stale) => Some(GlobalFilter::Ahead),
                Some(GlobalFilter::Ahead) => Some(GlobalFilter::Dirty),
                Some(GlobalFilter::Dirty) => None,
            }
        } else {
            match self.global_filter {
                None => Some(GlobalFilter::Dirty),
                Some(GlobalFilter::Dirty) => Some(GlobalFilter::Ahead),
                Some(GlobalFilter::Ahead) => Some(GlobalFilter::Stale),
                Some(GlobalFilter::Stale) => Some(GlobalFilter::Archived),
                Some(GlobalFilter::Archived) => None,
            }
        };
        self.selected_index = 0;
        self.scroll_top = 0;
    }

    /// Rows for the label picker: a leading `None` ("All repositories") entry,
    /// then every distinct label carried by a tracked repository with its repo
    /// count, alphabetical, narrowed by the fuzzy query in `input_buffer`.
    pub fn get_label_matches(&self) -> Vec<(Option<String>, usize)> {
        let query = self.input_buffer.to_lowercase();
        let mut counts: std::collections::BTreeMap<&String, usize> =
            std::collections::BTreeMap::new();
        for path in &self.config.items {
            if let Some(labels) = self.config.labels.get(path) {
                for label in labels {
                    *counts.entry(label).or_insert(0) += 1;
                }
            }
        }

        let mut rows = vec![(None, self.config.items.len())];
        rows.extend(
            counts
                .into_iter()
                .filter(|(label, _)| query.is_empty() || label.to_lowercase().contains(&query))
                .map(|(label, count)| (Some(label.clone()), count)),
        );
        rows
    }

    /// Applies the sticky label filter, toggling it off when the active label
    /// is selected again (`None` always clears), and persists the choice.
    /// Viewing a label for the first time also gives it a quick-label slot.
    pub fn select_label_filter(&mut self, label: Option<String>) {
        let toggled_off = label.is_some() && self.state.active_label_filter == label;
        let before = self.effective_sort();
        self.state.active_label_filter = if toggled_off { None } else { label };
        self.selected_index = 0;
        self.scroll_top = 0;
        self.resort_if_sort_changed(before);
        match self.state.active_label_filter.clone() {
            Some(l) => {
                self.record_label_slot(&l);
                self.persist_state(&format!("Label filter: {}", l));
            }
            None => self.persist_state("Label filter cleared"),
        }
    }

    /// Grows or shrinks the focused detail-view panel by nudging the split
    /// percentage that directly borders it (the same splits the mouse drags),
    /// clamped to the drag limits. Focuses without an adjustable split
    /// (single-pane tabs, fixed-ratio stacks) are a no-op.
    pub fn resize_focused_panel(&mut self, grow: bool) {
        const STEP: u16 = 5;
        let in_stashes_tab = self.detail_tab == 6;

        // (split percentage, whether the focused panel owns the `pct` side).
        let target: Option<(&mut u16, bool)> = match self.detail_focus {
            DetailSection::Commits => Some((&mut self.workspace_main_split_pct, true)),
            DetailSection::Staged | DetailSection::Unstaged | DetailSection::Conflicts => {
                Some((&mut self.inspect_horizontal_split_pct, true))
            }
            DetailSection::CommitDetails => Some((&mut self.inspect_vertical_split_pct, false)),
            DetailSection::ConflictDiff => Some((&mut self.inspect_horizontal_split_pct, false)),
            DetailSection::StagingDetails if in_stashes_tab => {
                Some((&mut self.stashes_horizontal_split_pct, false))
            }
            DetailSection::StagingDetails => Some((&mut self.inspect_horizontal_split_pct, false)),
            DetailSection::Files => Some((&mut self.files_horizontal_split_pct, true)),
            DetailSection::FileContent => Some((&mut self.files_horizontal_split_pct, false)),
            DetailSection::LocalBranches => Some((&mut self.branches_horizontal_split_pct, true)),
            DetailSection::RemoteBranches => Some((&mut self.branches_horizontal_split_pct, false)),
            DetailSection::Stashes => Some((&mut self.stashes_vertical_split_pct, true)),
            DetailSection::StashedFiles => Some((&mut self.stashes_vertical_split_pct, false)),
            DetailSection::ForgeIssues => Some((&mut self.forge_vertical_split_pct, true)),
            DetailSection::ForgeIssueDetails => Some((&mut self.forge_vertical_split_pct, false)),
            DetailSection::ForgePRs => Some((&mut self.forge_pr_vertical_split_pct, true)),
            DetailSection::ForgePRDetails => Some((&mut self.forge_pr_vertical_split_pct, false)),
            _ => None,
        };

        if let Some((pct, owns_pct_side)) = target {
            // Same clamp range the mouse drag uses.
            if grow == owns_pct_side {
                *pct = (*pct + STEP).min(85);
            } else {
                *pct = pct.saturating_sub(STEP).max(15);
            }
        }
    }

    /// The quick-label slots in FIFO order: slot 1 is the label that was
    /// viewed first, and each label keeps its slot until evicted (see
    /// `record_label_slot`). Each entry carries its repository count; labels
    /// no longer carried by any tracked repository are skipped. Empty until
    /// the first label view, so the strip starts clean.
    pub fn quick_labels(&self) -> Vec<(String, usize)> {
        let count_of = |label: &str| {
            self.config
                .items
                .iter()
                .filter_map(|path| self.config.labels.get(path))
                .filter(|labels| labels.iter().any(|l| l == label))
                .count()
        };
        self.state
            .label_slots
            .iter()
            .map(|label| (label.clone(), count_of(label)))
            .filter(|(_, count)| *count > 0)
            .take(crate::keybindings::HOME_LABEL_SLOTS)
            .collect()
    }

    /// Zero-based quick-label slot index of `label`, if it holds one.
    pub fn quick_label_slot(&self, label: &str) -> Option<usize> {
        self.quick_labels().iter().position(|(l, _)| l == label)
    }

    /// Gives `label` the next free quick-label slot on its first view. A label
    /// that already holds a slot keeps it (viewing never reorders the strip);
    /// when all slots are taken, the oldest entry is evicted and the rest
    /// shift down one so the new label lands in the last slot.
    fn record_label_slot(&mut self, label: &str) {
        if self.state.label_slots.iter().any(|l| l == label) {
            return;
        }
        self.state.label_slots.push(label.to_string());
        while self.state.label_slots.len() > crate::keybindings::HOME_LABEL_SLOTS {
            self.state.label_slots.remove(0);
        }
    }

    /// Drops quick-label slots whose label no repository carries any more, so
    /// a deleted label cannot linger in the strip.
    pub fn prune_label_slots(&mut self) {
        let used: std::collections::HashSet<&String> = self
            .config
            .items
            .iter()
            .filter_map(|path| self.config.labels.get(path))
            .flatten()
            .collect();
        let keep: Vec<String> =
            self.state.label_slots.iter().filter(|l| used.contains(l)).cloned().collect();
        self.state.label_slots = keep;
    }

    /// Applies the label held by zero-based `slot` as the sticky filter
    /// (toggling it off when it is already active, like the picker).
    /// An empty slot only reports a status message so the list never changes
    /// under a mistyped key.
    pub fn select_label_slot(&mut self, slot: usize) {
        match self.quick_labels().into_iter().nth(slot) {
            Some((label, _)) => self.select_label_filter(Some(label)),
            None => {
                self.status_message = Some(format!("No label in quick slot {}", slot + 1));
            }
        }
    }

    /// Removes `label` from the quick-label strip and persists `state.toml`.
    /// Leaves `active_label_filter` untouched so forgetting a chip never alters
    /// the current repository list view.
    pub fn forget_label_slot(&mut self, label: &str) -> bool {
        if !self.state.label_slots.iter().any(|l| l == label) {
            self.status_message = Some(format!("'{}' is not in quick labels", label));
            return false;
        }
        self.state.label_slots.retain(|l| l != label);
        self.persist_state(&format!("Removed '{}' from quick labels", label));
        true
    }

    /// Removes the quick-label chip at zero-based rendered `slot`.
    pub fn forget_label_slot_at(&mut self, slot: usize) -> bool {
        let Some((label, _)) = self.quick_labels().into_iter().nth(slot) else {
            return false;
        };
        self.forget_label_slot(&label)
    }

    /// Removes the currently active label filter from the quick-label strip.
    pub fn forget_active_quick_label(&mut self) -> bool {
        let Some(active) = self.state.active_label_filter.clone() else {
            self.status_message = Some("No active label filter to forget".to_string());
            return false;
        };
        self.forget_label_slot(&active)
    }

    /// Clears every quick-label chip from the strip and persists `state.toml`.
    pub fn clear_quick_labels(&mut self) -> bool {
        if self.state.label_slots.is_empty() {
            self.status_message = Some("No quick labels to clear".to_string());
            return false;
        }
        self.state.label_slots.clear();
        self.persist_state("Cleared all quick labels");
        true
    }

    /// Drops the sticky label filter when its label no longer exists on any
    /// tracked repository, so the home list cannot get stuck permanently empty.
    /// Returns whether the filter was cleared.
    pub fn clear_label_filter_if_orphaned(&mut self) -> bool {
        let Some(active) = self.state.active_label_filter.clone() else {
            return false;
        };
        let still_used = self
            .config
            .items
            .iter()
            .filter_map(|path| self.config.labels.get(path))
            .any(|labels| labels.contains(&active));
        if !still_used {
            let before = self.effective_sort();
            self.state.active_label_filter = None;
            self.selected_index = 0;
            self.scroll_top = 0;
            self.resort_if_sort_changed(before);
        }
        !still_used
    }

    pub fn get_filtered_items(&self) -> Vec<(usize, &String)> {
        let base_items = self.get_active_items();

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

    /// The `config.items` entry of the repository the user is working in:
    /// the one being loaded, else the one the Detail view has open, else the
    /// home selection. Anything that means "this repository" inside the
    /// Detail view must use this, not [`App::get_selected_item`]: the home
    /// cursor is an index into the grouped, sorted home rows, which can
    /// reorder while a repository is open and leave the cursor on a neighbour.
    /// A load wins over the open snapshot because opening a worktree from the
    /// Worktrees tab keeps the parent's snapshot until the worktree arrives.
    pub fn active_repo_item(&self) -> Option<&String> {
        let open = match (&self.loading_repo_path, &self.current_detail) {
            (Some(loading), _) => Some(repo::expand_tilde(loading)),
            (
                None,
                Some(
                    repo::ItemDetail::Repo { resolved, .. }
                    | repo::ItemDetail::Missing { resolved, .. }
                    | repo::ItemDetail::Directory { resolved, .. }
                    | repo::ItemDetail::Error { resolved, .. },
                ),
            ) => Some(resolved.clone()),
            (None, None) => None,
        };
        match open {
            Some(resolved) => {
                self.config.items.iter().find(|item| repo::expand_tilde(item) == resolved)
            }
            None => self.get_selected_item(),
        }
    }

    /// The repository under the home cursor and the group its row is listed
    /// under (`""` outside any group), so the row can be found again with
    /// [`App::select_home_row`] after the home rows reorder. `None` on a
    /// group header or an empty list.
    pub(crate) fn home_cursor(&self) -> Option<(String, String)> {
        match self.get_home_rows().get(self.selected_index)? {
            HomeRow::Repo { path, primary_label, .. } => {
                Some((path.clone(), primary_label.clone()))
            }
            HomeRow::GroupHeader { .. } => None,
        }
    }

    /// Put the home cursor on `item`'s row, preferring the one listed under
    /// `group`, since a repository can appear several times (Recent, Starred,
    /// each of its labels). Leaves the cursor alone when no visible row shows
    /// `item`.
    pub(crate) fn select_home_row(&mut self, item: &str, group: Option<&str>) {
        let rows = self.get_home_rows();
        let is_item = |row: &HomeRow| matches!(row, HomeRow::Repo { path, .. } if path == item);
        let pos = rows
            .iter()
            .position(|row| {
                is_item(row)
                    && matches!(row, HomeRow::Repo { primary_label, .. }
                        if Some(primary_label.as_str()) == group)
            })
            .or_else(|| rows.iter().position(is_item));
        if let Some(pos) = pos {
            self.selected_index = pos;
        }
    }

    /// Put the home cursor on a newly added `item`, first expanding its label
    /// groups (or Unlabeled) when all of them are collapsed, so the new
    /// repository is on the list under the cursor. Leaves the cursor alone
    /// when a filter hides `item`.
    pub(crate) fn reveal_home_row(&mut self, item: &str) {
        let visible = self
            .get_home_rows()
            .iter()
            .any(|row| matches!(row, HomeRow::Repo { path, .. } if path == item));
        if !visible {
            self.expand_label_groups_of(item);
        }
        self.select_home_row(item, None);
    }

    /// Expand the label groups that list `item`: each of its labels, or
    /// Unlabeled when it has none.
    fn expand_label_groups_of(&mut self, item: &str) {
        match self.config.labels.get(item) {
            Some(labels) if !labels.is_empty() => {
                for label in labels {
                    self.collapsed_groups.remove(label);
                }
            }
            _ => {
                self.collapsed_groups.remove("Unlabeled");
            }
        }
    }

    /// The label config of the sticky label filter, if a filter is active and
    /// that label has any settings stored.
    fn active_label_config(&self) -> Option<&crate::config::LabelConfig> {
        self.state.active_label_filter.as_ref().and_then(|l| self.config.label_configs.get(l))
    }

    /// The sort mode the home list actually uses: the active label's own
    /// `sort_by` while its filter ("project view") is on, else the global one.
    pub fn effective_sort_by(&self) -> SortOrder {
        self.active_label_config().and_then(|lc| lc.sort_by).unwrap_or(self.config.sort_by)
    }

    /// The sort direction the home list actually uses: the active label's own
    /// `sort_reverse` while its filter is on, else the global one.
    pub fn effective_sort_reverse(&self) -> bool {
        self.active_label_config()
            .and_then(|lc| lc.sort_reverse)
            .unwrap_or(self.config.sort_reverse)
    }

    /// `(sort_by, sort_reverse)` as the list currently applies them.
    pub fn effective_sort(&self) -> (SortOrder, bool) {
        (self.effective_sort_by(), self.effective_sort_reverse())
    }

    /// The layout the home list actually uses: the active label's own
    /// `view_mode` while its filter is on, else the global one. Every layout
    /// decision (row heights, tile columns, the compact header) reads this.
    pub fn effective_view_mode(&self) -> crate::config::HomeViewMode {
        self.active_label_config().and_then(|lc| lc.view_mode).unwrap_or(self.config.view_mode)
    }

    /// Like [`App::label_owning_sort_by`], for `view_mode` and the `v` key.
    fn label_owning_view_mode(&self) -> Option<String> {
        let label = self.state.active_label_filter.as_deref()?;
        self.config.label_configs.get(label)?.view_mode.map(|_| label.to_string())
    }

    /// The `v` key: steps the layout cards → compact → tile, acting on the
    /// active label's own `view_mode` when its Label Settings set one (so the
    /// key changes the layout on screen), else on the global one.
    pub fn cycle_view_mode(&mut self) {
        let msg = match self.label_owning_view_mode() {
            Some(label) => {
                if let Some(lc) = self.config.label_configs.get_mut(&label) {
                    lc.view_mode = lc.view_mode.map(crate::config::HomeViewMode::next);
                }
                format!("View mode cycled for label '{}'", label)
            }
            None => {
                self.config.view_mode = self.config.view_mode.next();
                "View mode cycled".to_string()
            }
        };
        self.persist(&msg);
    }

    /// The active label filter's name when that label overrides `sort_by` or
    /// `sort_reverse`, i.e. when the sort shown is the label's, not the global.
    pub fn sort_override_label(&self) -> Option<&str> {
        let label = self.state.active_label_filter.as_deref()?;
        let lc = self.config.label_configs.get(label)?;
        (lc.sort_by.is_some() || lc.sort_reverse.is_some()).then_some(label)
    }

    /// Re-sorts the list when the effective sort differs from `before` (taken
    /// with [`App::effective_sort`] prior to a label-filter change), keeping
    /// the selection on the same repository.
    pub fn resort_if_sort_changed(&mut self, before: (SortOrder, bool)) {
        if self.effective_sort() == before {
            return;
        }
        let cursor = self.home_cursor();
        self.sort_items_in_place();
        if let Some((item, group)) = cursor {
            self.select_home_row(&item, Some(&group));
        }
    }

    /// Resolves one setting for `path` through the three tiers: the per-repo
    /// override wins first, then the first of the repo's labels (in stored
    /// order) that defines the setting, then the global default. Keeping the
    /// precedence in a single place means every caller stays consistent.
    pub fn resolve_setting<T>(
        &self,
        path: &str,
        from_repo: impl Fn(&crate::config::RepoConfig) -> Option<T>,
        from_label: impl Fn(&crate::config::LabelConfig) -> Option<T>,
        global: T,
    ) -> T {
        if let Some(v) = self.config.repo_configs.get(path).and_then(&from_repo) {
            return v;
        }
        if let Some(labels) = self.config.labels.get(path) {
            for label in labels {
                if let Some(v) = self.config.label_configs.get(label).and_then(&from_label) {
                    return v;
                }
            }
        }
        global
    }

    /// The effective theme name for `path`: the per-repo override, else the
    /// first label (in stored order) that sets a theme, else `None` (global).
    pub fn effective_theme_name(&self, path: &str) -> Option<String> {
        if let Some(theme) = self.config.repo_configs.get(path).and_then(|rc| rc.theme.clone()) {
            return Some(theme);
        }
        if let Some(labels) = self.config.labels.get(path) {
            for label in labels {
                if let Some(theme) =
                    self.config.label_configs.get(label).and_then(|lc| lc.theme.clone())
                {
                    return Some(theme);
                }
            }
        }
        None
    }

    /// The editor command for `path`, resolved through repo → label → global.
    pub fn effective_editor(&self, path: &str) -> String {
        self.resolve_setting(
            path,
            |rc| rc.editor.clone(),
            |lc| lc.editor.clone(),
            self.config.editor.clone(),
        )
    }

    pub fn get_current_page_size(&self) -> usize {
        match self.active_repo_item() {
            Some(path) => self.resolve_setting(
                path,
                |rc| rc.page_size,
                |lc| lc.page_size,
                self.config.page_size,
            ),
            None => self.config.page_size,
        }
    }

    pub fn get_current_max_commits(&self) -> usize {
        match self.active_repo_item() {
            Some(path) => self.resolve_setting(
                path,
                |rc| rc.max_commits,
                |lc| lc.max_commits,
                self.config.max_commits,
            ),
            None => self.config.max_commits,
        }
    }

    /// Auto-fetch cadence in minutes for one repository: the per-repo override
    /// when set, otherwise the first label that sets it, otherwise the global
    /// interval. `0` means disabled.
    pub fn effective_auto_fetch_interval_mins(&self, path: &str) -> u64 {
        self.resolve_setting(
            path,
            |rc| rc.auto_fetch_interval_mins,
            |lc| lc.auto_fetch_interval_mins,
            self.config.auto_fetch_interval_mins,
        )
    }

    pub fn get_current_resync_on_tab_change(&self) -> bool {
        match self.active_repo_item() {
            Some(path) => self.resolve_setting(
                path,
                |rc| rc.resync_on_tab_change,
                |lc| lc.resync_on_tab_change,
                self.config.resync_on_tab_change,
            ),
            None => self.config.resync_on_tab_change,
        }
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

    pub fn get_tile_cols(&self) -> usize {
        if self.effective_view_mode() != crate::config::HomeViewMode::Tile {
            return 1;
        }
        if self.config.tile_columns > 0 {
            return self.config.tile_columns;
        }
        if let Ok(size) = crossterm::terminal::size() {
            let width = size.0 as usize;
            (width / 60).max(1)
        } else {
            3
        }
    }

    pub fn move_down(&mut self, visible_count: usize) {
        let len = self.get_items_len();
        let cols = self.get_tile_cols();
        if self.selected_index + cols < len {
            self.selected_index += cols;
        } else if self.selected_index + 1 < len {
            self.selected_index = len - 1;
        } else {
            return;
        }
        let bottom = self.scroll_top + visible_count;
        if self.selected_index >= bottom {
            let diff = self.selected_index + 1 - bottom;
            self.scroll_top = self.scroll_top.saturating_add(diff);
        }
    }

    pub fn move_up(&mut self) {
        let cols = self.get_tile_cols();
        if self.selected_index >= cols {
            self.selected_index -= cols;
        } else if self.selected_index > 0 {
            self.selected_index = 0;
        }
        if self.selected_index < self.scroll_top {
            self.scroll_top = self.selected_index;
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

    /// Re-runs the cheap filesystem inspection for the open repository after
    /// an external tool (`git rebase -i`, `git mergetool`) changed it, so its
    /// home card is current on return. `statuses` runs parallel to
    /// `config.items`, so the slot is the item's position there, never the
    /// home-row cursor.
    pub(crate) fn refresh_active_repo_status(&mut self) {
        let Some(item) = self.active_repo_item() else {
            return;
        };
        let Some(idx) = self.config.items.iter().position(|i| i == item) else {
            return;
        };
        let item = item.clone();
        let moved = self.store_status(idx, &item, repo::inspect_summary(&item));
        self.resort_after_status_refresh(moved);
    }

    /// The directory the external git app (`git_app`) starts in: the
    /// repository under the home cursor, since only the home screen launches it.
    pub(crate) fn git_app_dir(&self) -> Option<std::path::PathBuf> {
        self.get_selected_item().map(|item| repo::expand_tilde(item))
    }

    /// The repositories to open an external shell in: every multi-selected
    /// one (the selection is consumed), else the one under the home cursor.
    pub(crate) fn take_shell_targets(&mut self) -> Vec<String> {
        if !self.multi_selected.is_empty() {
            return std::mem::take(&mut self.multi_selected).into_iter().collect();
        }
        self.get_selected_item().cloned().into_iter().collect()
    }

    /// Stores a refreshed `status` for the repository at `path`. `idx` is where
    /// it was when the refresh started; if the list has been re-sorted since,
    /// the status goes wherever `path` is now (it used to be dropped, leaving
    /// the card stale until the next refresh). Returns whether the
    /// repository's last-commit time, the Latest Changes sort key, changed.
    pub(crate) fn store_status(&mut self, idx: usize, path: &str, status: ItemStatus) -> bool {
        let idx = if self.config.items.get(idx).map(String::as_str) == Some(path) {
            Some(idx)
        } else {
            self.config.items.iter().position(|item| item == path)
        };
        let Some(slot) = idx.and_then(|idx| self.statuses.get_mut(idx)) else {
            return false;
        };
        let commit_time = |status: &ItemStatus| match status {
            ItemStatus::GitRepo(Some(summary)) => summary.last_commit_time,
            _ => None,
        };
        let moved = commit_time(slot) != commit_time(&status);
        *slot = status;
        moved
    }

    /// Re-sorts the home list after refreshed statuses moved a repository's
    /// last commit, when the list is sorted by Latest Changes: the order was
    /// only computed when something else re-sorted (changing the sort, adding a
    /// repository), so a new commit left its repository where it was. The
    /// cursor stays on the repository it was on.
    pub(crate) fn resort_after_status_refresh(&mut self, commit_moved: bool) {
        if !commit_moved || self.effective_sort_by() != SortOrder::LatestChanges {
            return;
        }
        let cursor = self.home_cursor();
        self.sort_items_in_place();
        if let Some((item, group)) = cursor {
            self.select_home_row(&item, Some(&group));
        }
    }

    pub fn sort_items_in_place(&mut self) {
        let sort_reverse = self.effective_sort_reverse();
        let mut zipped: Vec<(String, ItemStatus)> = match self.effective_sort_by() {
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
                if sort_reverse {
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
                if sort_reverse {
                    z.reverse();
                }
                z
            }
            SortOrder::RecentVisit => {
                let visits = &self.state.visits;
                let mut z: Vec<(String, ItemStatus)> =
                    self.config.items.drain(..).zip(self.statuses.drain(..)).collect();
                z.sort_by(|a, b| {
                    let time_a = visits.get(&a.0).copied().unwrap_or(0);
                    let time_b = visits.get(&b.0).copied().unwrap_or(0);
                    time_b.cmp(&time_a) // Descending
                });
                if sort_reverse {
                    z.reverse();
                }
                z
            }
            SortOrder::LatestChanges => {
                let mut z: Vec<(String, ItemStatus)> =
                    self.config.items.drain(..).zip(self.statuses.drain(..)).collect();
                let mut times = std::collections::HashMap::with_capacity(z.len());
                for (path, status) in &z {
                    let t = match status {
                        crate::repo::ItemStatus::GitRepo(Some(s)) => {
                            s.last_commit_time.unwrap_or(0) as u64
                        }
                        _ => repo::get_latest_change_time(path),
                    };
                    times.insert(path.clone(), t);
                }
                z.sort_by(|a, b| {
                    let time_a = times.get(&a.0).copied().unwrap_or(0);
                    let time_b = times.get(&b.0).copied().unwrap_or(0);
                    time_b.cmp(&time_a) // Descending
                });
                if sort_reverse {
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

    /// The label whose `sort_by` the `o` key should cycle: the active filter's
    /// label when it overrides the mode, so the key changes the sort that is
    /// actually on screen rather than a hidden global.
    fn label_owning_sort_by(&self) -> Option<String> {
        let label = self.state.active_label_filter.as_deref()?;
        self.config.label_configs.get(label)?.sort_by.map(|_| label.to_string())
    }

    /// Like [`App::label_owning_sort_by`], for `sort_reverse` and the `O` key.
    fn label_owning_sort_reverse(&self) -> Option<String> {
        let label = self.state.active_label_filter.as_deref()?;
        self.config.label_configs.get(label)?.sort_reverse.map(|_| label.to_string())
    }

    pub fn cycle_sort_order(&mut self) {
        let msg = match self.label_owning_sort_by() {
            Some(label) => {
                if let Some(lc) = self.config.label_configs.get_mut(&label) {
                    lc.sort_by = lc.sort_by.map(SortOrder::next);
                }
                format!("Sort mode updated for label '{}'", label)
            }
            None => {
                self.config.sort_by = self.config.sort_by.next();
                "Sort mode updated".to_string()
            }
        };

        let cursor = self.home_cursor();

        self.sort_items_in_place();

        if let Some((item, group)) = cursor {
            self.select_home_row(&item, Some(&group));
        }

        self.persist(&msg);
    }

    pub fn toggle_sort_reverse(&mut self) {
        let msg = match self.label_owning_sort_reverse() {
            Some(label) => {
                if let Some(lc) = self.config.label_configs.get_mut(&label) {
                    lc.sort_reverse = lc.sort_reverse.map(|r| !r);
                }
                format!("Sort direction updated for label '{}'", label)
            }
            None => {
                self.config.sort_reverse = !self.config.sort_reverse;
                "Sort direction updated".to_string()
            }
        };

        let cursor = self.home_cursor();

        self.sort_items_in_place();

        if let Some((item, group)) = cursor {
            self.select_home_row(&item, Some(&group));
        }

        self.persist(&msg);
    }

    pub fn toggle_pin_selected(&mut self) {
        let Some((selected_item, group)) = self.home_cursor() else {
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
        self.select_home_row(&selected_item, Some(&group));

        let msg = self.status_message.as_deref().unwrap_or("Saved").to_string();
        self.persist(&msg);
    }

    pub fn toggle_star_selected(&mut self) {
        let Some((selected_item, group)) = self.home_cursor() else {
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

        // Keep the selection on the starred repo, in the group it was starred
        // from (the Starred group it joins, or leaves, sits above it).
        self.select_home_row(&selected_item, Some(&group));

        let msg = self.status_message.as_deref().unwrap_or("Saved").to_string();
        self.persist(&msg);
    }

    pub fn toggle_archive_selected(&mut self) {
        let Some((selected_item, _group)) = self.home_cursor() else {
            return;
        };
        if self.config.archived.contains(&selected_item) {
            self.config.archived.remove(&selected_item);
            self.status_message = Some("Unarchived repository".to_string());
        } else {
            self.config.archived.insert(selected_item.clone());
            self.status_message = Some("Archived repository".to_string());
        }

        self.clamp_selection();

        let msg = self.status_message.as_deref().unwrap_or("Saved").to_string();
        self.persist(&msg);
    }

    /// Resolves the target label and all currently shown repositories for that
    /// label: either the active sticky label filter (using all currently
    /// filtered items) or, when no label filter is active, the currently
    /// highlighted real label group header.
    pub fn resolve_archive_label_target(&self) -> Option<LabelArchiveTarget> {
        let unarchive = self.global_filter == Some(GlobalFilter::Archived);
        if let Some(label) = self.state.active_label_filter.clone() {
            let repos =
                self.get_filtered_items().into_iter().map(|(_, path)| path.clone()).collect();
            return Some(LabelArchiveTarget { label, repos, unarchive });
        }

        match self.get_home_rows().get(self.selected_index) {
            Some(HomeRow::GroupHeader { name, .. })
                if name != "Recent" && name != "Starred" && name != "Unlabeled" =>
            {
                let repos = self
                    .get_filtered_items()
                    .into_iter()
                    .filter(|&(_, path)| {
                        self.config
                            .labels
                            .get(path)
                            .is_some_and(|lbls| lbls.iter().any(|l| l == name))
                    })
                    .map(|(_, path)| path.clone())
                    .collect();
                Some(LabelArchiveTarget { label: name.clone(), repos, unarchive })
            }
            _ => None,
        }
    }

    /// Prompts the user to confirm archiving (or unarchiving on the Archived
    /// tab) all currently shown repositories for the active label filter or
    /// selected label group header.
    pub fn request_archive_label_repos(&mut self) {
        let Some(target) = self.resolve_archive_label_target() else {
            self.status_message =
                Some("Filter by a label (L) or select a label group first".to_string());
            return;
        };
        if target.repos.is_empty() {
            let verb = if target.unarchive { "unarchive" } else { "archive" };
            self.status_message =
                Some(format!("No shown repositories to {} for label '{}'", verb, target.label));
            return;
        }
        self.label_archive_target = Some(target);
        self.mode = Mode::ConfirmArchiveLabel;
    }

    /// Confirms [`Mode::ConfirmArchiveLabel`], updating `config.archived` for
    /// all target repositories and persisting the change.
    pub fn confirm_archive_label_repos(&mut self) {
        let Some(target) = self.label_archive_target.take() else {
            self.mode = Mode::Normal;
            return;
        };
        let count = target.repos.len();
        if target.unarchive {
            for repo in &target.repos {
                self.config.archived.remove(repo);
            }
        } else {
            for repo in &target.repos {
                self.config.archived.insert(repo.clone());
                self.multi_selected.remove(repo);
            }
        }
        self.mode = Mode::Normal;
        self.clamp_selection();
        if self.get_items_len() == 0 {
            self.scroll_top = 0;
        }
        let verb = if target.unarchive { "Unarchived" } else { "Archived" };
        let noun = if count == 1 { "repository" } else { "repositories" };
        let msg = format!("{} {} {} for label '{}'", verb, count, noun, target.label);
        self.persist(&msg);
    }

    /// Cancels [`Mode::ConfirmArchiveLabel`] without modifying archive state.
    pub fn cancel_archive_label_repos(&mut self) {
        self.label_archive_target = None;
        self.mode = Mode::Normal;
    }

    /// Open the repository under the home cursor in the Detail view.

    pub fn get_jump_matches(&self) -> Vec<(usize, String, String)> {
        let query = self.input_buffer.to_lowercase();
        let active_items = self.get_active_items();
        if query.is_empty() {
            return active_items
                .into_iter()
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
        for (idx, path) in active_items {
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
        if let Some(path) = self.config.items.get(original_index).cloned() {
            self.expand_label_groups_of(&path);
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
                        inner_height.saturating_sub(self.status_height() as usize).saturating_sub(
                            self.terminal_panel_outer_height(inner_height as u16) as usize,
                        );
                    let mut lh =
                        if self.effective_view_mode() == crate::config::HomeViewMode::Compact {
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
                    let cols = if self.effective_view_mode() == crate::config::HomeViewMode::Tile {
                        self.get_tile_cols()
                    } else {
                        1
                    };
                    let mut current_col = 0;

                    for idx in self.scroll_top..=pos {
                        match &rows[idx] {
                            HomeRow::GroupHeader { .. } => {
                                if current_col > 0 {
                                    accumulated += 4;
                                    current_col = 0;
                                }
                                let h = if self.effective_view_mode()
                                    == crate::config::HomeViewMode::Compact
                                {
                                    1
                                } else {
                                    2
                                };
                                accumulated += h;
                            }
                            HomeRow::Repo { .. } => {
                                if self.effective_view_mode() == crate::config::HomeViewMode::Tile {
                                    if current_col == 0 {
                                        accumulated += 4;
                                    }
                                    current_col += 1;
                                    if current_col == cols {
                                        current_col = 0;
                                    }
                                } else {
                                    let h = if self.effective_view_mode()
                                        == crate::config::HomeViewMode::Compact
                                    {
                                        1
                                    } else {
                                        4
                                    };
                                    accumulated += h;
                                }
                            }
                        }
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
                        let cols =
                            if self.effective_view_mode() == crate::config::HomeViewMode::Tile {
                                self.get_tile_cols()
                            } else {
                                1
                            };

                        // When scrolling up from the bottom, it's a bit tricky with tiles because tiles are grouped from top to bottom.
                        // To be perfectly accurate, we should find the exact layout from `0..=pos`.
                        // But for simplicity, we can layout from 0 to pos, and then advance scroll_top until it just fits.
                        let mut layout_heights = Vec::new();
                        let mut current_col = 0;
                        for idx in 0..=pos {
                            match &rows[idx] {
                                HomeRow::GroupHeader { .. } => {
                                    if current_col > 0 {
                                        layout_heights.push(4);
                                        current_col = 0;
                                    }
                                    let h = if self.effective_view_mode()
                                        == crate::config::HomeViewMode::Compact
                                    {
                                        1
                                    } else {
                                        2
                                    };
                                    layout_heights.push(h);
                                }
                                HomeRow::Repo { .. } => {
                                    if self.effective_view_mode()
                                        == crate::config::HomeViewMode::Tile
                                    {
                                        if current_col == 0 {
                                            layout_heights.push(4);
                                        } else {
                                            layout_heights.push(0); // share row
                                        }
                                        current_col += 1;
                                        if current_col == cols {
                                            current_col = 0;
                                        }
                                    } else {
                                        let h = if self.effective_view_mode()
                                            == crate::config::HomeViewMode::Compact
                                        {
                                            1
                                        } else {
                                            4
                                        };
                                        layout_heights.push(h);
                                    }
                                }
                            }
                        }

                        let mut new_scroll = 0;
                        let mut total_height: usize = layout_heights.iter().sum();

                        while total_height > list_height && new_scroll < pos {
                            total_height -= layout_heights[new_scroll];
                            new_scroll += 1;
                        }
                        self.scroll_top = new_scroll;
                    }
                }
            }
        }
        self.input_buffer.clear();
        self.mode = Mode::Normal;
    }
}
