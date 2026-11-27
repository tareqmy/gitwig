//! List selection, cursor movement, and viewport scrolling helpers for detail tabs and overlays.

use super::super::*;

impl App {
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
}
