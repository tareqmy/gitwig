//! Application state and the main run loop.
//!
//! `App` owns everything mutable about a session: the current config, where
//! to persist it back to, where the cursor is, what mode we're in, and any
//! transient status message. The drawing layer (`ui`) reads `App` but never
//! mutates it. Key handling (`input`) calls back into `App` methods so the
//! state-mutation logic stays in one place.

use std::error::Error;
use std::path::PathBuf;

use crossterm::event::{self, Event};
use ratatui::Terminal;
use ratatui::layout::{Margin, Rect};

use crate::config::{Config, save_config};
use crate::input;
use crate::repo::{self, DiffLine, ItemDetail, ItemStatus};
use crate::ui;
use crate::ui_detail::DetailAreas;

/// Height of each item row inside the bordered list area.
/// Borders (top + bottom) take 2 rows; the remaining 2 inner rows hold
/// the item path and the branch name respectively.
pub const ITEM_HEIGHT: u16 = 4;

/// Interaction modes for the item list. The mode dictates how keystrokes
/// are interpreted and what guidance the status bar shows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Browsing the list. Navigation + add/edit/delete shortcuts are active.
    Normal,
    /// Typing a new item to append. Enter commits, Esc cancels.
    Adding,
    /// Typing replacement text for the selected item. Enter commits, Esc cancels.
    Editing,
    /// Asking the user to confirm deletion of the selected item.
    ConfirmDelete,
    /// Showing the full shortcut reference as a centered overlay.
    Help,
    /// Showing the full-screen detail view for the selected item.
    Detail,
    /// Showing the repo overview popup inside the detail view (triggered by 'o').
    DetailOverview,
    /// Showing the shortcut reference overlay inside the detail view (triggered by '?').
    DetailHelp,
    /// Typing a commit message. Enter commits, Esc cancels.
    CommitInput,
}

/// Which panel in the detail view currently has keyboard focus.
/// Tab cycles through them in order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DetailSection {
    Commits,
    Staged,
    Unstaged,
    CommitDetails,
    StagingDetails,
    LocalBranches,
    RemoteBranches,
    Files,
}

impl DetailSection {
    /// Advance to the next section in the cycle.
    pub fn next(self) -> Self {
        match self {
            Self::Commits => Self::Staged,
            Self::Staged => Self::Unstaged,
            Self::Unstaged => Self::CommitDetails,
            Self::CommitDetails => Self::StagingDetails,
            Self::StagingDetails => Self::Commits,
            Self::LocalBranches => Self::RemoteBranches,
            Self::RemoteBranches => Self::LocalBranches,
            Self::Files => Self::Files,
        }
    }
}

/// All mutable session state.
pub struct App {
    pub config: Config,
    pub config_path: PathBuf,
    /// Filesystem classification per item, parallel to `config.items`.
    /// Recomputed on add/edit/delete so it never drifts from the list.
    pub statuses: Vec<ItemStatus>,
    pub selected_index: usize,
    pub scroll_top: usize,
    pub mode: Mode,
    pub input_buffer: String,
    pub status_message: Option<String>,
    /// Populated when entering `Mode::Detail`, cleared when leaving. The
    /// detail snapshot is taken once on open (not re-fetched per frame)
    /// so opening a slow repo only costs one git2 call.
    pub current_detail: Option<ItemDetail>,
    /// Which panel is focused inside the detail view.
    pub detail_focus: DetailSection,
    /// Selected row index inside the Commits panel (0 = top row).
    pub commit_selection: usize,
    /// Selected file index inside the Changed Files panel (real commits).
    pub file_selection: usize,
    /// Selected file index inside the Staged/Unstaged sub-panels (uncommitted view).
    pub staging_file_selection: usize,
    /// Cached unified-diff lines for the currently selected file.
    pub file_diff: Vec<DiffLine>,
    /// Vertical scroll offset for the diff panel (StagingDetails focus).
    pub diff_scroll: usize,
    /// Vertical scroll offset for the commit details panel (CommitDetails focus).
    pub commit_details_scroll: usize,
    /// Selected local branch index in Branches tab.
    pub local_branch_selection: usize,
    /// Selected remote branch index in Branches tab.
    pub remote_branch_selection: usize,
    /// Panel bounding boxes recorded after each draw, used for mouse hit-testing.
    pub detail_areas: DetailAreas,
    /// Main panel item bounding boxes recorded after each draw, used for mouse hit-testing.
    pub main_areas: Vec<Rect>,
    /// Timestamp and selected index of the last mouse click for double-click detection.
    pub last_click: Option<(std::time::Instant, usize)>,
    /// Active tab in the detail view (0 = Details, 1 = Graph, 2 = Branches, 3 = Files).
    pub detail_tab: usize,
    /// Selected file index in the Files tab.
    pub file_list_selection: usize,
    /// Vertical scroll offset for the git history graph view (Graph tab).
    pub graph_scroll: usize,
    /// Whether we are currently editing the commit message in the popup.
    pub commit_editing: bool,
    /// Whether the status bar is expanded.
    pub status_expanded: bool,
    /// Sender for background task events.
    pub tx: std::sync::mpsc::Sender<String>,
    /// Receiver for background task events.
    pub rx: std::sync::mpsc::Receiver<String>,
    /// Whether a background fetch is active.
    pub fetching: bool,
    /// Whether gitui launch is pending.
    pub pending_gitui: bool,
}

impl App {
    pub fn new(config: Config, config_path: PathBuf) -> Self {
        let statuses = config
            .items
            .iter()
            .map(|s| repo::inspect_summary(s))
            .collect();
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            config,
            config_path,
            statuses,
            selected_index: 0,
            scroll_top: 0,
            mode: Mode::Normal,
            input_buffer: String::new(),
            status_message: None,
            current_detail: None,
            detail_focus: DetailSection::Commits,
            commit_selection: 0,
            file_selection: 0,
            staging_file_selection: 0,
            file_diff: Vec::new(),
            diff_scroll: 0,
            commit_details_scroll: 0,
            local_branch_selection: 0,
            remote_branch_selection: 0,
            detail_areas: DetailAreas::default(),
            main_areas: Vec::new(),
            last_click: None,
            detail_tab: 0,
            file_list_selection: 0,
            graph_scroll: 0,
            commit_editing: false,
            status_expanded: false,
            tx,
            rx,
            fetching: false,
            pending_gitui: false,
        }
    }

    pub fn status_height(&self) -> u16 {
        if self.status_expanded { 3 } else { 1 }
    }

    pub fn toggle_status_expanded(&mut self) {
        self.status_expanded = !self.status_expanded;
    }

    /// Ensure `selected_index` is a valid index into `config.items`.
    pub fn clamp_selection(&mut self) {
        if self.config.items.is_empty() {
            self.selected_index = 0;
        } else if self.selected_index >= self.config.items.len() {
            self.selected_index = self.config.items.len() - 1;
        }
    }

    /// Ensure the scroll window doesn't extend past the end of the list.
    pub fn clamp_scroll(&mut self, visible_count: usize) {
        let max_scroll = self.config.items.len().saturating_sub(visible_count);
        if self.scroll_top > max_scroll {
            self.scroll_top = max_scroll;
        }
    }

    pub fn move_down(&mut self, visible_count: usize) {
        if self.selected_index + 1 < self.config.items.len() {
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
        let last = self.config.items.len().saturating_sub(1);
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

    pub fn start_add(&mut self) {
        self.input_buffer.clear();
        self.mode = Mode::Adding;
    }

    pub fn start_edit(&mut self) {
        if let Some(current) = self.config.items.get(self.selected_index) {
            self.input_buffer = current.clone();
            self.mode = Mode::Editing;
        }
    }

    pub fn request_delete(&mut self) {
        if !self.config.items.is_empty() {
            self.mode = Mode::ConfirmDelete;
        }
    }

    pub fn open_help(&mut self) {
        self.mode = Mode::Help;
    }

    /// Re-runs the cheap filesystem inspection for the selected item and
    /// updates its status indicator. Surfaces a transient "Refreshed" /
    /// "Refresh failed" message in the status bar so the user knows the
    /// keystroke landed (the indicator alone may not visibly change).
    pub fn refresh_selected_status(&mut self) {
        let Some(item) = self.config.items.get(self.selected_index) else {
            return;
        };
        let new_status = repo::inspect_summary(item);
        if let Some(slot) = self.statuses.get_mut(self.selected_index) {
            *slot = new_status;
        }
        self.status_message = Some("Refreshed".to_string());
    }

    /// Snapshot the selected item's filesystem/git state and enter the
    /// Detail view. The snapshot is held in `current_detail` for as long
    /// as the view is open; closing clears it.
    pub fn open_detail(&mut self) {
        if let Some(item) = self.config.items.get(self.selected_index) {
            self.current_detail = Some(repo::inspect_detail(item));
            self.detail_focus = DetailSection::Commits;
            self.commit_selection = 0;
            self.file_selection = 0;
            self.staging_file_selection = 0;
            self.file_diff.clear();
            self.diff_scroll = 0;
            self.commit_details_scroll = 0;
            self.local_branch_selection = 0;
            self.remote_branch_selection = 0;
            self.file_list_selection = 0;
            self.detail_tab = 0;
            self.graph_scroll = 0;
            self.mode = Mode::Detail;
            self.refresh_file_diff();
        }
    }

    /// Advance focus to the next detail panel (Tab key).
    pub fn cycle_detail_focus(&mut self) {
        if self.detail_tab == 2 {
            self.detail_focus = match self.detail_focus {
                DetailSection::LocalBranches => DetailSection::RemoteBranches,
                _ => DetailSection::LocalBranches,
            };
            return;
        }
        self.detail_focus = self.detail_focus.next();
        if !self.is_uncommitted_selected() && self.detail_focus == DetailSection::Unstaged {
            self.detail_focus = self.detail_focus.next();
        }
        if self.is_uncommitted_selected() && self.detail_focus == DetailSection::CommitDetails {
            self.detail_focus = self.detail_focus.next();
        }
        // Reset staging selection and pre-load diff when landing on Staged/Unstaged.
        match self.detail_focus {
            DetailSection::Staged | DetailSection::Unstaged => {
                self.staging_file_selection = 0;
                self.diff_scroll = 0;
                self.refresh_staging_diff();
            }
            DetailSection::CommitDetails => {
                self.commit_details_scroll = 0;
            }
            DetailSection::StagingDetails => {
                self.diff_scroll = 0;
            }
            // Pre-load the diff when landing on the Changed Files panel (real commit).
            _ => {
                if matches!(
                    self.detail_focus,
                    DetailSection::Staged | DetailSection::Unstaged
                ) {
                    self.refresh_file_diff();
                }
            }
        }
    }

    /// Move local branch selection up.
    pub fn local_branch_up(&mut self) {
        self.local_branch_selection = self.local_branch_selection.saturating_sub(1);
    }

    /// Move local branch selection down.
    pub fn local_branch_down(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.local_branches.len();
            if total > 0 && self.local_branch_selection + 1 < total {
                self.local_branch_selection += 1;
            }
        }
    }

    /// Scroll local branch selection up by page.
    pub fn local_branch_page_up(&mut self, page: usize) {
        self.local_branch_selection = self.local_branch_selection.saturating_sub(page);
    }

    /// Scroll local branch selection down by page.
    pub fn local_branch_page_down(&mut self, page: usize) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.local_branches.len();
            if total > 0 {
                self.local_branch_selection =
                    (self.local_branch_selection + page).min(total.saturating_sub(1));
            }
        }
    }

    /// Move remote branch selection up.
    pub fn remote_branch_up(&mut self) {
        self.remote_branch_selection = self.remote_branch_selection.saturating_sub(1);
    }

    /// Move remote branch selection down.
    pub fn remote_branch_down(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.remote_branches.len();
            if total > 0 && self.remote_branch_selection + 1 < total {
                self.remote_branch_selection += 1;
            }
        }
    }

    /// Scroll remote branch selection up by page.
    pub fn remote_branch_page_up(&mut self, page: usize) {
        self.remote_branch_selection = self.remote_branch_selection.saturating_sub(page);
    }

    /// Scroll remote branch selection down by page.
    pub fn remote_branch_page_down(&mut self, page: usize) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.remote_branches.len();
            if total > 0 {
                self.remote_branch_selection =
                    (self.remote_branch_selection + page).min(total.saturating_sub(1));
            }
        }
    }

    /// Move file selection up in the Files tab.
    pub fn file_list_up(&mut self) {
        self.file_list_selection = self.file_list_selection.saturating_sub(1);
    }

    /// Move file selection down in the Files tab.
    pub fn file_list_down(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.files.len();
            if total > 0 && self.file_list_selection + 1 < total {
                self.file_list_selection += 1;
            }
        }
    }

    /// Scroll file selection up by page.
    pub fn file_list_page_up(&mut self, page: usize) {
        self.file_list_selection = self.file_list_selection.saturating_sub(page);
    }

    /// Scroll file selection down by page.
    pub fn file_list_page_down(&mut self, page: usize) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.files.len();
            if total > 0 {
                self.file_list_selection =
                    (self.file_list_selection + page).min(total.saturating_sub(1));
            }
        }
    }

    /// Spawns a background thread to fetch the remote of the selected local branch.
    pub fn fetch_selected_branch(&mut self) {
        if self.fetching {
            return;
        }
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
            if let Some(branch_info) = info.local_branches.get(self.local_branch_selection) {
                self.fetching = true;
                self.status_message = Some("Fetching...".to_string());

                let repo_path = resolved.clone();
                let branch_name = branch_info.name.clone();
                let tx = self.tx.clone();

                std::thread::spawn(move || {
                    let res = (|| -> Result<String, Box<dyn std::error::Error>> {
                        let repo = git2::Repository::open(&repo_path)?;
                        let branch = repo.find_branch(&branch_name, git2::BranchType::Local)?;

                        let upstream = match branch.upstream() {
                            Ok(u) => u,
                            Err(_) => {
                                return Ok(
                                    "No upstream tracking branch configured for this branch"
                                        .to_string(),
                                );
                            }
                        };
                        let upstream_ref = upstream.get().name()?;
                        let remote_buf = repo.branch_upstream_remote(upstream_ref)?;
                        let remote_name = remote_buf.as_str()?;

                        let output = std::process::Command::new("git")
                            .arg("fetch")
                            .arg(remote_name)
                            .current_dir(&repo_path)
                            .output()?;

                        if output.status.success() {
                            Ok(format!("Fetched remote '{}' successfully", remote_name))
                        } else {
                            let err_msg =
                                String::from_utf8_lossy(&output.stderr).trim().to_string();
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
    }

    /// Checks out the selected local branch (safety checks apply).
    pub fn checkout_selected_local_branch(&mut self) {
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
            if let Some(branch_info) = info.local_branches.get(self.local_branch_selection) {
                if !branch_info.is_head {
                    match repo::checkout_local_branch(resolved, &branch_info.name) {
                        Ok(()) => {
                            self.status_message =
                                Some(format!("Switched to branch '{}'", branch_info.name));
                            // Refresh detail snapshot
                            let item = &self.config.items[self.selected_index];
                            self.current_detail = Some(repo::inspect_detail(item));
                            self.local_branch_selection = 0;
                        }
                        Err(e) => {
                            self.status_message = Some(format!("Checkout failed: {}", e));
                        }
                    }
                }
            }
        }
    }

    /// Checks out the selected remote branch, creating a local tracking branch if needed.
    pub fn checkout_selected_remote_branch(&mut self) {
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
            if let Some(branch_info) = info.remote_branches.get(self.remote_branch_selection) {
                match repo::checkout_remote_branch(resolved, &branch_info.name) {
                    Ok(msg) => {
                        self.status_message = Some(msg);
                        // Refresh detail snapshot
                        let item = &self.config.items[self.selected_index];
                        self.current_detail = Some(repo::inspect_detail(item));
                        self.local_branch_selection = 0;
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Checkout failed: {}", e));
                    }
                }
            }
        }
    }

    /// Move commit selection up one row.
    pub fn detail_commit_up(&mut self) {
        self.commit_selection = self.commit_selection.saturating_sub(1);
        self.file_selection = 0;
        self.diff_scroll = 0;
        self.refresh_file_diff();
    }

    /// Move commit selection down one row, clamped to the last visible row.
    pub fn detail_commit_down(&mut self) {
        let total = self.commit_total();
        if total > 0 && self.commit_selection + 1 < total {
            self.commit_selection += 1;
        }
        self.file_selection = 0;
        self.diff_scroll = 0;
        self.refresh_file_diff();
    }

    /// Jump commit selection up by `page` rows.
    pub fn detail_commit_page_up(&mut self, page: usize) {
        self.commit_selection = self.commit_selection.saturating_sub(page);
        self.file_selection = 0;
        self.diff_scroll = 0;
        self.refresh_file_diff();
    }

    /// Jump commit selection down by `page` rows, clamped to the last row.
    pub fn detail_commit_page_down(&mut self, page: usize) {
        let total = self.commit_total();
        if total > 0 {
            self.commit_selection = (self.commit_selection + page).min(total - 1);
        }
        self.file_selection = 0;
        self.diff_scroll = 0;
        self.refresh_file_diff();
    }

    /// Move file selection up one row in the Changed Files panel.
    pub fn detail_file_up(&mut self) {
        self.file_selection = self.file_selection.saturating_sub(1);
        self.diff_scroll = 0;
        self.refresh_file_diff();
    }

    /// Move file selection down one row in the Changed Files panel.
    pub fn detail_file_down(&mut self) {
        let total = self.file_total();
        if total > 0 && self.file_selection + 1 < total {
            self.file_selection += 1;
        }
        self.diff_scroll = 0;
        self.refresh_file_diff();
    }

    /// Move staging-area file selection up one row (Staged or Unstaged panel).
    pub fn staging_file_up(&mut self) {
        self.staging_file_selection = self.staging_file_selection.saturating_sub(1);
        self.diff_scroll = 0;
        self.refresh_staging_diff();
    }

    /// Move staging-area file selection down one row (Staged or Unstaged panel).
    pub fn staging_file_down(&mut self) {
        let total = self.staging_file_total();
        if total > 0 && self.staging_file_selection + 1 < total {
            self.staging_file_selection += 1;
        }
        self.diff_scroll = 0;
        self.refresh_staging_diff();
    }

    /// Scroll the diff panel up by one line.
    pub fn diff_scroll_up(&mut self) {
        self.diff_scroll = self.diff_scroll.saturating_sub(1);
    }

    /// Scroll the diff panel down by one line, clamped so the last line stays visible.
    pub fn diff_scroll_down(&mut self) {
        let max = self.file_diff.len().saturating_sub(1);
        if self.diff_scroll < max {
            self.diff_scroll += 1;
        }
    }

    /// Scroll the diff panel up by `page` lines.
    pub fn diff_scroll_page_up(&mut self, page: usize) {
        self.diff_scroll = self.diff_scroll.saturating_sub(page);
    }

    /// Scroll the diff panel down by `page` lines.
    pub fn diff_scroll_page_down(&mut self, page: usize) {
        let max = self.file_diff.len().saturating_sub(1);
        self.diff_scroll = (self.diff_scroll + page).min(max);
    }

    /// Scroll the graph view up by one line.
    pub fn graph_scroll_up(&mut self) {
        self.graph_scroll = self.graph_scroll.saturating_sub(1);
    }

    /// Scroll the graph view down by one line.
    pub fn graph_scroll_down(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let max = info.graph_lines.len().saturating_sub(1);
            if self.graph_scroll < max {
                self.graph_scroll += 1;
            }
        }
    }

    /// Scroll the graph view up by a page.
    pub fn graph_scroll_page_up(&mut self, page: usize) {
        self.graph_scroll = self.graph_scroll.saturating_sub(page);
    }

    /// Scroll the graph view down by a page.
    pub fn graph_scroll_page_down(&mut self, page: usize) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let max = info.graph_lines.len().saturating_sub(1);
            self.graph_scroll = (self.graph_scroll + page).min(max);
        }
    }

    /// Scroll the commit details panel up by one line.
    pub fn commit_details_scroll_up(&mut self) {
        self.commit_details_scroll = self.commit_details_scroll.saturating_sub(1);
    }

    /// Scroll the commit details panel down by one line.
    pub fn commit_details_scroll_down(&mut self) {
        self.commit_details_scroll = self.commit_details_scroll.saturating_add(1);
    }

    /// Total number of rows in the Commits panel (dirty row + real commits).
    fn commit_total(&self) -> usize {
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => {
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                info.commits.len() + usize::from(dirty)
            }
            _ => 0,
        }
    }

    /// Total files in the currently-selected commit's Changed Files panel.
    fn file_total(&self) -> usize {
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => {
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                // Uncommitted row (staging area) has no file list.
                if dirty && self.commit_selection == 0 {
                    return 0;
                }
                let idx = if dirty {
                    self.commit_selection.saturating_sub(1)
                } else {
                    self.commit_selection
                };
                info.commits.get(idx).map(|c| c.files.len()).unwrap_or(0)
            }
            _ => 0,
        }
    }

    pub fn is_uncommitted_selected(&self) -> bool {
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => {
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                dirty && self.commit_selection == 0
            }
            _ => false,
        }
    }

    fn current_diff_params(&self) -> Option<(PathBuf, String, String)> {
        match &self.current_detail {
            Some(ItemDetail::Repo { resolved, info }) => {
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                if dirty && self.commit_selection == 0 {
                    return None;
                }
                let commit_idx = if dirty {
                    self.commit_selection.saturating_sub(1)
                } else {
                    self.commit_selection
                };
                let commit = info.commits.get(commit_idx)?;
                let file = commit.files.get(self.file_selection)?;
                Some((resolved.clone(), commit.oid.clone(), file.path.clone()))
            }
            _ => None,
        }
    }

    pub fn refresh_file_diff(&mut self) {
        if let Some((repo_path, commit_oid, file_path)) = self.current_diff_params() {
            self.file_diff = repo::get_commit_file_diff(&repo_path, &commit_oid, &file_path);
        } else {
            self.file_diff.clear();
        }
    }

    /// Total files in the currently-focused Staged or Unstaged sub-panel.
    fn staging_file_total(&self) -> usize {
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => match self.detail_focus {
                DetailSection::Staged => info.changes.staged.len(),
                DetailSection::Unstaged => info.changes.unstaged.len(),
                _ => 0,
            },
            _ => 0,
        }
    }

    /// Reload `file_diff` from the currently-focused Staged/Unstaged file.
    pub fn refresh_staging_diff(&mut self) {
        let params = match &self.current_detail {
            Some(ItemDetail::Repo { resolved, info }) => {
                let (files, staged) = match self.detail_focus {
                    DetailSection::Staged => (&info.changes.staged, true),
                    DetailSection::Unstaged => (&info.changes.unstaged, false),
                    _ => {
                        self.file_diff.clear();
                        return;
                    }
                };
                files
                    .get(self.staging_file_selection)
                    .map(|f| (resolved.clone(), f.path.clone(), staged))
            }
            _ => None,
        };
        if let Some((repo_path, file_path, staged)) = params {
            self.file_diff = repo::get_worktree_file_diff(&repo_path, &file_path, staged);
        } else {
            self.file_diff.clear();
        }
    }

    /// Re-snapshot the repo for the selected item, preserving focus and clamping selection.
    /// Call this after any index-mutating operation (stage / unstage).
    pub fn refresh_detail(&mut self) {
        if let Some(item) = self.config.items.get(self.selected_index) {
            self.current_detail = Some(repo::inspect_detail(item));
            // Clamp staging_file_selection to the new list length.
            let new_total = self.staging_file_total();
            if new_total == 0 {
                self.staging_file_selection = 0;
            } else if self.staging_file_selection >= new_total {
                self.staging_file_selection = new_total - 1;
            }
            self.diff_scroll = 0;
            self.refresh_staging_diff();
        }
    }

    /// Stage the currently-selected file in the Unstaged panel (`git add`).
    pub fn stage_selected_file(&mut self) {
        let params = match &self.current_detail {
            Some(ItemDetail::Repo { resolved, info }) => info
                .changes
                .unstaged
                .get(self.staging_file_selection)
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
                .get(self.staging_file_selection)
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

    pub fn close_detail(&mut self) {
        self.current_detail = None;
        self.mode = Mode::Normal;
    }

    /// Opens the repo overview popup while staying in the detail view.
    pub fn open_overview_popup(&mut self) {
        self.mode = Mode::DetailOverview;
    }

    /// Closes the overview popup and returns to the normal detail view.
    pub fn close_overview_popup(&mut self) {
        self.mode = Mode::Detail;
    }

    /// Opens the shortcut help overlay inside the detail view.
    pub fn open_detail_help(&mut self) {
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
        if has_staged {
            self.input_buffer.clear();
            self.commit_editing = true;
            self.mode = Mode::CommitInput;
        } else {
            self.status_message = Some("No staged changes to commit".to_string());
        }
    }

    /// Cancels commit input and returns to the detail view.
    pub fn cancel_commit(&mut self) {
        self.input_buffer.clear();
        self.mode = Mode::Detail;
    }

    /// Transitions from editing the message to confirming the commit.
    pub fn commit_done_editing(&mut self) {
        self.commit_editing = false;
    }

    /// Transitions back to editing the message from confirm state.
    pub fn commit_start_editing(&mut self) {
        self.commit_editing = true;
    }

    /// Performs the git commit with the message in `input_buffer`.
    pub fn commit_git_changes(&mut self) {
        let msg = self.input_buffer.trim().to_string();
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
            match repo::commit_changes(&path, &msg) {
                Ok(()) => {
                    self.status_message = Some("Committed successfully".to_string());
                    self.refresh_detail();
                    self.refresh_selected_status();
                }
                Err(e) => {
                    self.status_message = Some(format!("Commit failed: {}", e));
                }
            }
        }

        self.input_buffer.clear();
        self.mode = Mode::Detail;
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

    pub fn commit_add(&mut self) {
        let trimmed = self.input_buffer.trim().to_string();
        if !trimmed.is_empty() {
            self.statuses.push(repo::inspect_summary(&trimmed));
            self.config.items.push(trimmed);
            self.selected_index = self.config.items.len() - 1;
            self.persist("Saved");
        }
        self.input_buffer.clear();
        self.mode = Mode::Normal;
    }

    pub fn commit_edit(&mut self) {
        let trimmed = self.input_buffer.trim().to_string();
        if !trimmed.is_empty()
            && let Some(slot) = self.config.items.get_mut(self.selected_index)
        {
            *slot = trimmed.clone();
            if let Some(slot) = self.statuses.get_mut(self.selected_index) {
                *slot = repo::inspect_summary(&trimmed);
            }
            self.persist("Saved");
        }
        self.input_buffer.clear();
        self.mode = Mode::Normal;
    }

    pub fn confirm_delete(&mut self) {
        if self.selected_index < self.config.items.len() {
            self.config.items.remove(self.selected_index);
            if self.selected_index < self.statuses.len() {
                self.statuses.remove(self.selected_index);
            }
            self.persist("Deleted");
        }
        self.mode = Mode::Normal;
    }

    pub fn close_dialog(&mut self) {
        self.mode = Mode::Normal;
    }

    /// Persists `self.config` and records a status message (success or
    /// the save error) for the next render.
    fn persist(&mut self, success_msg: &str) {
        self.status_message = match save_config(&self.config, &self.config_path) {
            Ok(()) => Some(success_msg.to_string()),
            Err(e) => Some(format!("Save failed: {}", e)),
        };
    }
}

/// Main event loop: compute layout, draw, poll input, repeat.
pub fn run<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> Result<(), Box<dyn Error>>
where
    <B as ratatui::backend::Backend>::Error: 'static,
{
    loop {
        while let Ok(msg) = app.rx.try_recv() {
            app.status_message = Some(msg);
            app.fetching = false;
            if let Some(item) = app.config.items.get(app.selected_index) {
                app.current_detail = Some(repo::inspect_detail(item));
            }
        }

        if app.pending_gitui {
            app.pending_gitui = false;
            if let Some(item) = app.config.items.get(app.selected_index) {
                let path = repo::expand_tilde(item);

                let raw_res = crossterm::terminal::disable_raw_mode();
                let exec_res = crossterm::execute!(
                    std::io::stdout(),
                    crossterm::terminal::LeaveAlternateScreen,
                    crossterm::event::DisableMouseCapture
                );
                let cursor_res = terminal.show_cursor();

                if raw_res.is_ok() && exec_res.is_ok() && cursor_res.is_ok() {
                    let status = std::process::Command::new("gitui")
                        .current_dir(&path)
                        .status();

                    let _ = crossterm::terminal::enable_raw_mode();
                    let _ = crossterm::execute!(
                        std::io::stdout(),
                        crossterm::terminal::EnterAlternateScreen,
                        crossterm::event::EnableMouseCapture
                    );
                    let _ = terminal.clear();

                    match status {
                        Ok(s) if s.success() => {
                            app.status_message = Some("Returned from gitui".to_string());
                            app.refresh_selected_status();
                        }
                        Ok(_) => {
                            app.status_message = Some("gitui exited with error".to_string());
                            app.refresh_selected_status();
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            app.status_message = Some("gitui is not installed".to_string());
                        }
                        Err(e) => {
                            app.status_message = Some(format!("Could not run gitui: {}", e));
                        }
                    }
                }
            }
        }

        app.clamp_selection();

        let size = terminal.size()?;
        let area = Rect::new(0, 0, size.width, size.height);
        let inner_area = area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        let available_height = inner_area.height.saturating_sub(app.status_height());
        let visible_count =
            (available_height / ITEM_HEIGHT).min(app.config.items.len() as u16) as usize;
        app.clamp_scroll(visible_count);

        // Capture panel rects from the draw pass for mouse hit-testing.
        let mut detail_areas = DetailAreas::default();
        let mut main_areas = Vec::new();
        terminal.draw(|f| {
            ui::draw(
                f,
                &app,
                area,
                inner_area,
                visible_count,
                &mut detail_areas,
                &mut main_areas,
            )
        })?;
        app.detail_areas = detail_areas;
        app.main_areas = main_areas;

        // Transient feedback disappears after one frame, unless we are fetching.
        if app.fetching {
            app.status_message = Some("Fetching...".to_string());
        } else {
            app.status_message = None;
        }

        if event::poll(std::time::Duration::from_millis(
            app.config.poll_interval_ms,
        ))? {
            match event::read()? {
                Event::Key(key) => {
                    if !input::handle_key(&mut app, key, visible_count) {
                        return Ok(());
                    }
                }
                Event::Mouse(mouse) => {
                    input::handle_mouse(&mut app, mouse);
                }
                _ => {}
            }
        }
    }
}
