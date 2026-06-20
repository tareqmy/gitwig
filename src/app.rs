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

use crate::config::{Config, SortOrder, save_config};
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
    /// Showing the shortcut reference overlay inside the detail view (triggered by '?').
    DetailHelp,
    /// Typing a commit message. Enter commits, Esc cancels.
    CommitInput,
    /// Typing a branch name to create. Enter commits, Esc cancels.
    BranchCreateInput,
    /// Typing a tag name to create. Enter commits, Esc cancels.
    TagCreateInput,
    /// Confirming deletion of a branch. y deletes, n/Esc cancels.
    BranchDeleteConfirm,
    /// Confirming push of a branch. y pushes, n/Esc cancels.
    BranchPushConfirm,
    /// Confirming deletion of a tag. y deletes, n/Esc cancels.
    TagDeleteConfirm,
    /// Confirming push of a tag. y pushes, n/Esc cancels.
    TagPushConfirm,
    /// Confirming push of all tags. y pushes, n/Esc cancels.
    TagPushAllConfirm,
    /// Confirming deletion of a stash. y deletes, n/Esc cancels.
    StashDeleteConfirm,
    /// Confirming apply of a stash.
    StashApplyConfirm,
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
    LocalTags,
    RemoteTags,
    Files,
    Remotes,
    Stashes,
    StashedFiles,
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
            Self::LocalTags => Self::RemoteTags,
            Self::RemoteTags => Self::LocalTags,
            Self::Files => Self::Files,
            Self::Remotes => Self::Remotes,
            Self::Stashes => Self::Stashes,
            Self::StashedFiles => Self::StashedFiles,
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
    /// Selected local tag index in Tags/Branches tabs.
    pub local_tag_selection: usize,
    /// Selected remote tag index in Tags/Branches tabs.
    pub remote_tag_selection: usize,
    /// Selected remote index in Remotes tab.
    pub remote_selection: usize,
    /// Selected stash index in Stashes tab.
    pub stash_selection: usize,
    /// Selected file index in the Stashes tab stashed files list.
    pub stash_file_selection: usize,
    /// Scroll offset for the help overlays.
    pub help_scroll: usize,
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
    /// Set of expanded folder paths.
    pub expanded_folders: std::collections::HashSet<String>,
    /// Flattened visible files inside the Files tab.
    pub visible_files: Vec<FileTreeItem>,
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
    /// Target branch name and remote flag for deletion/creation actions.
    pub branch_action_target: Option<(String, bool)>,
    /// Target commit OID for tag creation.
    pub tag_action_target_oid: Option<String>,
    /// Target tag name and remote flag for deletion action.
    pub tag_delete_target: Option<(String, bool)>,
    /// Target tag name for push action.
    pub tag_push_target: Option<String>,
    /// Simulated fetch progress percentage.
    pub fetch_progress: u16,
    /// Option to delete the stash after applying.
    pub stash_apply_delete_after: bool,
    /// Option to amend the last commit.
    pub commit_amend: bool,
    /// Preserved original order of repository items from the config.
    pub original_items: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct FileTreeItem {
    pub name: String,
    pub full_path: String,
    pub is_dir: bool,
    pub depth: usize,
    pub is_expanded: bool,
}

struct TempNode {
    name: String,
    full_path: String,
    is_dir: bool,
    children: std::collections::BTreeMap<String, TempNode>,
}

impl App {
    pub fn new(config: Config, config_path: PathBuf) -> Self {
        let original_items = config.items.clone();
        let statuses = config
            .items
            .iter()
            .map(|s| repo::inspect_summary(s))
            .collect();
        let (tx, rx) = std::sync::mpsc::channel();
        let mut app = Self {
            original_items,
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
            local_tag_selection: 0,
            remote_tag_selection: 0,
            remote_selection: 0,
            stash_selection: 0,
            stash_file_selection: 0,
            help_scroll: 0,
            detail_areas: DetailAreas::default(),
            main_areas: Vec::new(),
            last_click: None,
            detail_tab: 0,
            file_list_selection: 0,
            expanded_folders: std::collections::HashSet::new(),
            visible_files: Vec::new(),
            graph_scroll: 0,
            commit_editing: false,
            status_expanded: false,
            tx,
            rx,
            fetching: false,
            pending_gitui: false,
            branch_action_target: None,
            tag_action_target_oid: None,
            tag_delete_target: None,
            tag_push_target: None,
            fetch_progress: 0,
            stash_apply_delete_after: true,
            commit_amend: false,
        };

        if app.config.sort_by != SortOrder::Custom {
            app.sort_items_in_place();
        }

        app
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

    /// Clamp the help scroll value so it doesn't go out of bounds.
    pub fn clamp_help_scroll(&mut self, height: usize) {
        let (percent_y, lines_len) = match self.mode {
            Mode::Help => (70, crate::ui::HELP_LINES.len() + 14),
            Mode::DetailHelp => (55, crate::ui_detail::DETAIL_HELP_LINES.len() + 2),
            _ => return,
        };
        let popup_height = (height * percent_y) / 100;
        let inner_height = popup_height.saturating_sub(2);
        let max_scroll = lines_len.saturating_sub(inner_height);
        if self.help_scroll > max_scroll {
            self.help_scroll = max_scroll;
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
        self.help_scroll = 0;
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

    pub fn sort_items_in_place(&mut self) {
        match self.config.sort_by {
            SortOrder::Custom => {
                self.config.items = self.original_items.clone();
                self.statuses = self
                    .config
                    .items
                    .iter()
                    .map(|s| repo::inspect_summary(s))
                    .collect();
            }
            SortOrder::Alphabetical => {
                let mut zipped: Vec<(String, ItemStatus)> = self
                    .config
                    .items
                    .drain(..)
                    .zip(self.statuses.drain(..))
                    .collect();
                zipped.sort_by(|a, b| a.0.cmp(&b.0));
                let (items, statuses): (Vec<String>, Vec<ItemStatus>) = zipped.into_iter().unzip();
                self.config.items = items;
                self.statuses = statuses;
            }
            SortOrder::RecentVisit => {
                let visits = &self.config.visits;
                let mut zipped: Vec<(String, ItemStatus)> = self
                    .config
                    .items
                    .drain(..)
                    .zip(self.statuses.drain(..))
                    .collect();
                zipped.sort_by(|a, b| {
                    let time_a = visits.get(&a.0).copied().unwrap_or(0);
                    let time_b = visits.get(&b.0).copied().unwrap_or(0);
                    time_b.cmp(&time_a) // Descending
                });
                let (items, statuses): (Vec<String>, Vec<ItemStatus>) = zipped.into_iter().unzip();
                self.config.items = items;
                self.statuses = statuses;
            }
            SortOrder::LatestChanges => {
                let mut zipped: Vec<(String, ItemStatus)> = self
                    .config
                    .items
                    .drain(..)
                    .zip(self.statuses.drain(..))
                    .collect();
                zipped.sort_by(|a, b| {
                    let time_a = repo::get_latest_change_time(&a.0);
                    let time_b = repo::get_latest_change_time(&b.0);
                    time_b.cmp(&time_a) // Descending
                });
                let (items, statuses): (Vec<String>, Vec<ItemStatus>) = zipped.into_iter().unzip();
                self.config.items = items;
                self.statuses = statuses;
            }
        }
    }

    pub fn cycle_sort_order(&mut self) {
        self.config.sort_by = match self.config.sort_by {
            SortOrder::Custom => SortOrder::Alphabetical,
            SortOrder::Alphabetical => SortOrder::RecentVisit,
            SortOrder::RecentVisit => SortOrder::LatestChanges,
            SortOrder::LatestChanges => SortOrder::Custom,
        };

        let selected_item = self.config.items.get(self.selected_index).cloned();

        self.sort_items_in_place();

        if let Some(item) = selected_item {
            if let Some(pos) = self.config.items.iter().position(|x| x == &item) {
                self.selected_index = pos;
            }
        }

        self.persist("Sort mode updated");
    }

    /// Snapshot the selected item's filesystem/git state and enter the
    /// Detail view. The snapshot is held in `current_detail` for as long
    /// as the view is open; closing clears it.
    pub fn open_detail(&mut self) {
        if let Some(item) = self.config.items.get(self.selected_index).cloned() {
            // Update visit time
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            self.config.visits.insert(item.clone(), now);
            let _ = save_config(&self.config, &self.config_path);

            if self.config.sort_by == SortOrder::RecentVisit {
                self.sort_items_in_place();
                if let Some(pos) = self.config.items.iter().position(|x| x == &item) {
                    self.selected_index = pos;
                }
            }

            self.current_detail = Some(repo::inspect_detail(&item));
            self.detail_focus = DetailSection::Commits;
            self.commit_selection = 0;
            self.file_selection = 0;
            self.staging_file_selection = 0;
            self.file_diff.clear();
            self.diff_scroll = 0;
            self.commit_details_scroll = 0;
            self.local_branch_selection = 0;
            self.remote_branch_selection = 0;
            self.local_tag_selection = 0;
            self.remote_tag_selection = 0;
            self.remote_selection = 0;
            self.stash_selection = 0;
            self.stash_file_selection = 0;
            self.file_list_selection = 0;
            self.expanded_folders.clear();
            self.rebuild_visible_files();
            self.detail_tab = 0;
            self.graph_scroll = 0;
            self.mode = Mode::Detail;
            self.refresh_file_diff();
        }
    }

    /// Advance focus to the next detail panel (Tab key).
    pub fn cycle_detail_focus(&mut self) {
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
        if self.detail_tab == 6 {
            self.detail_focus = match self.detail_focus {
                DetailSection::Stashes => DetailSection::StashedFiles,
                DetailSection::StashedFiles => DetailSection::StagingDetails,
                _ => DetailSection::Stashes,
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
                self.diff_scroll = 0;
                if self.is_uncommitted_selected() {
                    self.staging_file_selection = 0;
                    self.refresh_staging_diff();
                } else {
                    self.file_selection = 0;
                    self.refresh_file_diff();
                }
            }
            DetailSection::CommitDetails => {
                self.commit_details_scroll = 0;
            }
            DetailSection::StagingDetails => {
                self.diff_scroll = 0;
            }
            _ => {}
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
        let total = self.visible_files.len();
        if total > 0 && self.file_list_selection + 1 < total {
            self.file_list_selection += 1;
        }
    }

    /// Scroll file selection up by page.
    pub fn file_list_page_up(&mut self, page: usize) {
        self.file_list_selection = self.file_list_selection.saturating_sub(page);
    }

    /// Scroll file selection down by page.
    pub fn file_list_page_down(&mut self, page: usize) {
        let total = self.visible_files.len();
        if total > 0 {
            self.file_list_selection =
                (self.file_list_selection + page).min(total.saturating_sub(1));
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

    /// Spawns a background thread to pull the upstream remote branch into the selected local branch.
    /// Can only pull if the selected local branch is the currently checked-out branch.
    pub fn pull_selected_branch(&mut self) {
        if self.fetching {
            return;
        }
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
            if let Some(branch_info) = info.local_branches.get(self.local_branch_selection) {
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
                        let repo = git2::Repository::open(&repo_path)?;
                        let branch = repo.find_branch(&branch_name, git2::BranchType::Local)?;

                        let _upstream = match branch.upstream() {
                            Ok(u) => u,
                            Err(_) => {
                                return Ok(
                                    "No upstream tracking branch configured for this branch"
                                        .to_string(),
                                );
                            }
                        };

                        let output = std::process::Command::new("git")
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
    pub fn request_branch_push(&mut self) {
        if self.fetching {
            return;
        }
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if let Some(branch_info) = info.local_branches.get(self.local_branch_selection) {
                self.branch_action_target = Some((branch_info.name.clone(), false));
                self.mode = Mode::BranchPushConfirm;
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

            let mut remote_name = None;
            let mut set_upstream = false;

            if let Ok(repo) = git2::Repository::open(&repo_path) {
                if let Ok(branch) = repo.find_branch(&branch_name, git2::BranchType::Local) {
                    if let Ok(upstream) = branch.upstream() {
                        if let Ok(upstream_ref) = upstream.get().name() {
                            if let Ok(remote_buf) = repo.branch_upstream_remote(upstream_ref) {
                                if let Ok(name) = remote_buf.as_str() {
                                    remote_name = Some(name.to_string());
                                }
                            }
                        }
                    }
                }

                if remote_name.is_none() {
                    if let Ok(remotes) = repo.remotes() {
                        if let Some(Ok(Some(first_remote))) = remotes.iter().next() {
                            remote_name = Some(first_remote.to_string());
                            set_upstream = true;
                        }
                    }
                }
            }

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
                    let mut cmd = std::process::Command::new("git");
                    cmd.arg("push");
                    if set_upstream {
                        cmd.arg("-u");
                    }
                    cmd.arg(&remote_name)
                        .arg(&branch_name)
                        .current_dir(&repo_path);

                    let output = cmd.output()?;

                    if output.status.success() {
                        Ok(format!(
                            "Pushed '{}' to '{}' successfully",
                            branch_name, remote_name
                        ))
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
                            self.rebuild_visible_files();
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
                        self.rebuild_visible_files();
                        self.local_branch_selection = 0;
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Checkout failed: {}", e));
                    }
                }
            }
        }
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
                self.commit_selection.saturating_sub(1)
            } else {
                self.commit_selection
            };
            if let Some(commit) = info.commits.get(commit_idx) {
                self.tag_action_target_oid = Some(commit.oid.clone());
                self.input_buffer.clear();
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
                        // Refresh detail view to display the new tag refs
                        let item = &self.config.items[self.selected_index];
                        self.current_detail = Some(repo::inspect_detail(item));
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Failed to create tag: {}", e));
                    }
                }
            }
        }
        self.mode = Mode::Detail;
    }

    pub fn request_tag_delete(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if let Some(tag_info) = info.local_tags.get(self.local_tag_selection) {
                let is_on_remote = if info.remotes.is_empty() {
                    false
                } else if info.remote_tags_loaded {
                    info.remote_tags.iter().any(|rt| rt.name == tag_info.name)
                } else {
                    false
                };
                self.tag_delete_target = Some((tag_info.name.clone(), is_on_remote));
                self.mode = Mode::TagDeleteConfirm;
            }
        }
    }

    pub fn confirm_tag_delete(&mut self) {
        if let Some((tag_name, is_on_remote)) = self.tag_delete_target.take() {
            let (repo_path, remote_name) =
                if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
                    (
                        resolved.clone(),
                        info.remotes.first().map(|r| r.name.clone()),
                    )
                } else {
                    return;
                };

            match repo::delete_tag(&repo_path, &tag_name) {
                Ok(()) => {
                    self.status_message = Some(format!("Deleted local tag '{}'", tag_name));
                    let item = &self.config.items[self.selected_index];
                    self.current_detail = Some(repo::inspect_detail(item));
                    self.rebuild_visible_files();
                    self.local_tag_selection = 0;

                    if is_on_remote {
                        if let Some(remote_name) = remote_name {
                            let repo_path = repo_path.clone();
                            let tag_to_delete = tag_name.clone();
                            let tx = self.tx.clone();
                            self.fetching = true;
                            self.status_message =
                                Some(format!("Deleting remote tag '{}'...", tag_name));
                            std::thread::spawn(move || {
                                match repo::delete_remote_tag(
                                    &repo_path,
                                    &remote_name,
                                    &tag_to_delete,
                                ) {
                                    Ok(()) => {
                                        let _ = tx.send(format!(
                                            "Deleted remote tag '{}'",
                                            tag_to_delete
                                        ));
                                    }
                                    Err(e) => {
                                        let _ =
                                            tx.send(format!("Failed to delete remote tag: {}", e));
                                    }
                                }
                            });
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
            if let Some(tag_info) = info.local_tags.get(self.local_tag_selection) {
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
                self.tag_push_target = Some(tag_info.name.clone());
                self.mode = Mode::TagPushConfirm;
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
                self.status_message = Some(format!(
                    "Pushing tag '{}' to '{}'...",
                    tag_name, remote_name
                ));
                let tx = self.tx.clone();
                std::thread::spawn(move || {
                    let mut cmd = std::process::Command::new("git");
                    cmd.arg("push")
                        .arg(&remote_name)
                        .arg(&tag_name)
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
            self.mode = Mode::TagPushAllConfirm;
        }
    }

    pub fn confirm_tag_push_all(&mut self) {
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
            self.status_message = Some(format!("Pushing all tags to '{}'...", remote_name));
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let mut cmd = std::process::Command::new("git");
                cmd.arg("push")
                    .arg(&remote_name)
                    .arg("--tags")
                    .current_dir(&repo_path);

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
                    let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    let _ = tx.send(format!("Failed to push tags: {}", err_msg));
                }
            });
        }
        self.mode = Mode::Detail;
    }

    pub fn cancel_tag_push_all(&mut self) {
        self.mode = Mode::Detail;
    }

    pub fn start_branch_create(&mut self) {
        if let Some(repo::ItemDetail::Repo { .. }) = &self.current_detail {
            self.input_buffer.clear();
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
                    let item = &self.config.items[self.selected_index];
                    self.current_detail = Some(repo::inspect_detail(item));
                    self.rebuild_visible_files();
                    self.local_branch_selection = 0;
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to create branch: {}", e));
                }
            }
        }
        self.input_buffer.clear();
        self.mode = Mode::Detail;
    }

    pub fn cancel_branch_create(&mut self) {
        self.input_buffer.clear();
        self.mode = Mode::Detail;
    }

    pub fn request_branch_delete(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            match self.detail_focus {
                DetailSection::LocalBranches => {
                    if let Some(branch_info) = info.local_branches.get(self.local_branch_selection)
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
                        info.remote_branches.get(self.remote_branch_selection)
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
                        let item = &self.config.items[self.selected_index];
                        self.current_detail = Some(repo::inspect_detail(item));
                        self.rebuild_visible_files();
                        self.local_branch_selection = 0;
                        self.remote_branch_selection = 0;
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
        if self.detail_tab == 6 {
            let params = match &self.current_detail {
                Some(ItemDetail::Repo { resolved, info }) => {
                    info.stashes.get(self.stash_selection).and_then(|stash| {
                        stash.files.get(self.stash_file_selection).map(|file| {
                            (resolved.clone(), stash.commit_id.clone(), file.path.clone())
                        })
                    })
                }
                _ => None,
            };
            if let Some((repo_path, commit_oid, file_path)) = params {
                self.file_diff = repo::get_commit_file_diff(&repo_path, &commit_oid, &file_path);
            } else {
                self.file_diff.clear();
            }
            return;
        }

        if self.is_uncommitted_selected() {
            let params = match &self.current_detail {
                Some(ItemDetail::Repo { resolved, info }) => {
                    if !info.changes.staged.is_empty() {
                        info.changes
                            .staged
                            .first()
                            .map(|f| (resolved.clone(), f.path.clone(), true))
                    } else if !info.changes.unstaged.is_empty() {
                        info.changes
                            .unstaged
                            .first()
                            .map(|f| (resolved.clone(), f.path.clone(), false))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some((repo_path, file_path, staged)) = params {
                self.file_diff = repo::get_worktree_file_diff(&repo_path, &file_path, staged);
            } else {
                self.file_diff.clear();
            }
        } else if let Some((repo_path, commit_oid, file_path)) = self.current_diff_params() {
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
            self.rebuild_visible_files();
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
            self.input_buffer.clear();
            self.commit_editing = true;
            self.commit_amend = false;
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
            let res = if self.commit_amend {
                repo::commit_amend(&path, &msg)
            } else {
                repo::commit_changes(&path, &msg)
            };
            match res {
                Ok(()) => {
                    let success_msg = if self.commit_amend {
                        "Amended commit successfully"
                    } else {
                        "Committed successfully"
                    };
                    self.status_message = Some(success_msg.to_string());
                    self.refresh_detail();
                    self.refresh_selected_status();
                }
                Err(e) => {
                    let fail_msg = if self.commit_amend {
                        format!("Amend failed: {}", e)
                    } else {
                        format!("Commit failed: {}", e)
                    };
                    self.status_message = Some(fail_msg);
                }
            }
        }

        self.input_buffer.clear();
        self.mode = Mode::Detail;
    }

    pub fn toggle_commit_amend(&mut self) {
        self.commit_amend = !self.commit_amend;
        if self.commit_amend && self.input_buffer.trim().is_empty() {
            if let Some(ItemDetail::Repo { resolved, .. }) = &self.current_detail {
                if let Some(last_msg) = repo::get_last_commit_message(resolved) {
                    self.input_buffer = last_msg;
                }
            }
        }
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
            let status = repo::inspect_summary(&trimmed);
            self.statuses.push(status);
            self.config.items.push(trimmed.clone());
            self.original_items.push(trimmed.clone());

            self.sort_items_in_place();

            if let Some(pos) = self.config.items.iter().position(|x| x == &trimmed) {
                self.selected_index = pos;
            } else {
                self.selected_index = self.config.items.len() - 1;
            }
            self.persist("Saved");
        }
        self.input_buffer.clear();
        self.mode = Mode::Normal;
    }

    pub fn commit_edit(&mut self) {
        let trimmed = self.input_buffer.trim().to_string();
        if !trimmed.is_empty() && self.selected_index < self.config.items.len() {
            let old_item = self.config.items[self.selected_index].clone();

            if let Some(pos) = self.original_items.iter().position(|x| x == &old_item) {
                self.original_items[pos] = trimmed.clone();
            }

            if let Some(time) = self.config.visits.remove(&old_item) {
                self.config.visits.insert(trimmed.clone(), time);
            }

            self.config.items[self.selected_index] = trimmed.clone();
            self.statuses[self.selected_index] = repo::inspect_summary(&trimmed);

            self.sort_items_in_place();

            if let Some(pos) = self.config.items.iter().position(|x| x == &trimmed) {
                self.selected_index = pos;
            }
            self.persist("Saved");
        }
        self.input_buffer.clear();
        self.mode = Mode::Normal;
    }

    pub fn confirm_delete(&mut self) {
        if self.selected_index < self.config.items.len() {
            let item = self.config.items.remove(self.selected_index);
            if self.selected_index < self.statuses.len() {
                self.statuses.remove(self.selected_index);
            }
            if let Some(pos) = self.original_items.iter().position(|x| x == &item) {
                self.original_items.remove(pos);
            }
            self.config.visits.remove(&item);
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

            for file_path in &info.files {
                let parts: Vec<&str> = file_path.split('/').collect();
                let mut current = &mut root;
                let mut accumulated = String::new();
                for (i, part) in parts.iter().enumerate() {
                    if !accumulated.is_empty() {
                        accumulated.push('/');
                    }
                    accumulated.push_str(part);

                    let is_last = i == parts.len() - 1;
                    let entry = current
                        .children
                        .entry((*part).to_string())
                        .or_insert_with(|| TempNode {
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

            flatten_tree(&root, 0, &self.expanded_folders, &mut visible_files);
        }
        self.visible_files = visible_files;
    }

    /// Expand the selected folder in the Files tab.
    pub fn expand_selected_folder(&mut self) {
        if let Some(item) = self.visible_files.get(self.file_list_selection) {
            if item.is_dir {
                self.expanded_folders.insert(item.full_path.clone());
                self.rebuild_visible_files();
            }
        }
    }

    /// Collapse the selected folder in the Files tab.
    pub fn collapse_selected_folder(&mut self) {
        if let Some(item) = self.visible_files.get(self.file_list_selection) {
            if item.is_dir {
                self.expanded_folders.remove(&item.full_path);
                self.rebuild_visible_files();
            }
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
            1 => self.detail_focus = DetailSection::Files,
            3 => self.detail_focus = DetailSection::LocalBranches,
            4 => {
                self.detail_focus = DetailSection::LocalTags;
                self.fetch_remote_tags();
            }
            5 => self.detail_focus = DetailSection::Remotes,
            6 => {
                self.detail_focus = DetailSection::Stashes;
                self.stash_file_selection = 0;
                self.refresh_file_diff();
            }
            _ => {}
        }
    }

    pub fn fetch_remote_tags(&mut self) {
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
            if let Some(remote) = info.remotes.first() {
                let repo_path = resolved.clone();
                let remote_name = remote.name.clone();
                let tx = self.tx.clone();
                std::thread::spawn(
                    move || match repo::get_remote_tags(&repo_path, &remote_name) {
                        Ok(tags) => {
                            let serialized = repo::serialize_tags(&tags);
                            let _ = tx.send(format!("REMOTE_TAGS:{}", serialized));
                        }
                        Err(e) => {
                            let _ = tx
                                .send(format!("REMOTE_TAGS_ERR:Failed to get remote tags: {}", e));
                        }
                    },
                );
            }
        }
    }

    pub fn local_tag_up(&mut self) {
        self.local_tag_selection = self.local_tag_selection.saturating_sub(1);
    }

    pub fn local_tag_down(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.local_tags.len();
            if total > 0 && self.local_tag_selection + 1 < total {
                self.local_tag_selection += 1;
            }
        }
    }

    pub fn local_tag_page_up(&mut self, page: usize) {
        self.local_tag_selection = self.local_tag_selection.saturating_sub(page);
    }

    pub fn local_tag_page_down(&mut self, page: usize) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.local_tags.len();
            if total > 0 {
                self.local_tag_selection =
                    (self.local_tag_selection + page).min(total.saturating_sub(1));
            }
        }
    }

    pub fn remote_up(&mut self) {
        self.remote_selection = self.remote_selection.saturating_sub(1);
    }

    pub fn remote_down(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.remotes.len();
            if total > 0 && self.remote_selection + 1 < total {
                self.remote_selection += 1;
            }
        }
    }

    pub fn remote_page_up(&mut self, page: usize) {
        self.remote_selection = self.remote_selection.saturating_sub(page);
    }

    pub fn remote_page_down(&mut self, page: usize) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.remotes.len();
            if total > 0 {
                self.remote_selection = (self.remote_selection + page).min(total.saturating_sub(1));
            }
        }
    }

    pub fn stash_up(&mut self) {
        self.stash_selection = self.stash_selection.saturating_sub(1);
        self.stash_file_selection = 0;
        self.refresh_file_diff();
    }

    pub fn stash_down(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.stashes.len();
            if total > 0 && self.stash_selection + 1 < total {
                self.stash_selection += 1;
                self.stash_file_selection = 0;
                self.refresh_file_diff();
            }
        }
    }

    pub fn stash_page_up(&mut self, page: usize) {
        self.stash_selection = self.stash_selection.saturating_sub(page);
        self.stash_file_selection = 0;
        self.refresh_file_diff();
    }

    pub fn stash_page_down(&mut self, page: usize) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let total = info.stashes.len();
            if total > 0 {
                self.stash_selection = (self.stash_selection + page).min(total.saturating_sub(1));
                self.stash_file_selection = 0;
                self.refresh_file_diff();
            }
        }
    }

    pub fn stash_file_up(&mut self) {
        self.stash_file_selection = self.stash_file_selection.saturating_sub(1);
        self.refresh_file_diff();
    }

    pub fn stash_file_down(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if let Some(stash) = info.stashes.get(self.stash_selection) {
                let total = stash.files.len();
                if total > 0 && self.stash_file_selection + 1 < total {
                    self.stash_file_selection += 1;
                    self.refresh_file_diff();
                }
            }
        }
    }

    pub fn stash_file_page_up(&mut self, page: usize) {
        self.stash_file_selection = self.stash_file_selection.saturating_sub(page);
        self.refresh_file_diff();
    }

    pub fn stash_file_page_down(&mut self, page: usize) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if let Some(stash) = info.stashes.get(self.stash_selection) {
                let total = stash.files.len();
                if total > 0 {
                    self.stash_file_selection =
                        (self.stash_file_selection + page).min(total.saturating_sub(1));
                    self.refresh_file_diff();
                }
            }
        }
    }

    pub fn request_stash_delete(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if info.stashes.get(self.stash_selection).is_some() {
                self.mode = Mode::StashDeleteConfirm;
            }
        }
    }

    pub fn confirm_stash_delete(&mut self) {
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
            if let Some(stash) = info.stashes.get(self.stash_selection) {
                let index_to_delete = stash.index;
                match repo::delete_stash(resolved, index_to_delete) {
                    Ok(()) => {
                        self.status_message =
                            Some(format!("Deleted stash@{{{}}}", index_to_delete));
                        let item = &self.config.items[self.selected_index];
                        self.current_detail = Some(repo::inspect_detail(item));
                        self.rebuild_visible_files();
                        self.stash_selection = 0;
                        self.stash_file_selection = 0;
                        self.refresh_file_diff();
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
            if info.stashes.get(self.stash_selection).is_some() {
                self.stash_apply_delete_after = true;
                self.mode = Mode::StashApplyConfirm;
            }
        }
    }

    pub fn confirm_stash_apply(&mut self) {
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
            if let Some(stash) = info.stashes.get(self.stash_selection) {
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
                        let item = &self.config.items[self.selected_index];
                        self.current_detail = Some(repo::inspect_detail(item));
                        self.rebuild_visible_files();
                        self.stash_selection = 0;
                        self.stash_file_selection = 0;
                        self.refresh_file_diff();
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

    pub fn checkout_selected_local_tag(&mut self) {
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
            if let Some(tag_info) = info.local_tags.get(self.local_tag_selection) {
                match repo::checkout_tag(resolved, &tag_info.name) {
                    Ok(()) => {
                        self.status_message = Some(format!(
                            "Checked out tag '{}' (detached HEAD)",
                            tag_info.name
                        ));
                        let item = &self.config.items[self.selected_index];
                        self.current_detail = Some(repo::inspect_detail(item));
                        self.rebuild_visible_files();
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Failed to checkout tag: {}", e));
                    }
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
            if let Some(tags_data) = msg.strip_prefix("REMOTE_TAGS:") {
                let tags = repo::deserialize_tags(tags_data);
                if let Some(repo::ItemDetail::Repo { info, .. }) = &mut app.current_detail {
                    info.remote_tags = tags;
                    info.remote_tags_loaded = true;
                }
            } else if let Some(err_msg) = msg.strip_prefix("REMOTE_TAGS_ERR:") {
                app.status_message = Some(err_msg.to_string());
            } else {
                app.status_message = Some(msg);
                app.fetching = false;
                if let Some(item) = app.config.items.get(app.selected_index) {
                    app.current_detail = Some(repo::inspect_detail(item));
                    app.rebuild_visible_files();
                }
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
        app.clamp_help_scroll(area.height as usize);

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
            if app.status_message.is_none() {
                app.status_message = Some("Executing Git operation...".to_string());
            }
            app.fetch_progress = (app.fetch_progress + 5) % 105;
        } else {
            app.status_message = None;
            app.fetch_progress = 0;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SortOrder;
    use std::collections::HashMap;

    #[test]
    fn test_sorting_logic() {
        let config = Config {
            items: vec![
                "z_repo".to_string(),
                "a_repo".to_string(),
                "m_repo".to_string(),
            ],
            poll_interval_ms: 100,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
        };
        let mut app = App::new(config, PathBuf::from("dummy_path"));

        // Assert initial custom sort
        assert_eq!(app.config.items[0], "z_repo");
        assert_eq!(app.config.items[1], "a_repo");

        // Cycle to alphabetical
        app.cycle_sort_order();
        assert_eq!(app.config.sort_by, SortOrder::Alphabetical);
        assert_eq!(app.config.items[0], "a_repo");
        assert_eq!(app.config.items[1], "m_repo");
        assert_eq!(app.config.items[2], "z_repo");

        // Cycle to recent visit
        // Set visit times: a_repo visited at 10, z_repo at 20, m_repo at 5
        app.config.visits.insert("a_repo".to_string(), 10);
        app.config.visits.insert("z_repo".to_string(), 20);
        app.config.visits.insert("m_repo".to_string(), 5);

        app.cycle_sort_order();
        assert_eq!(app.config.sort_by, SortOrder::RecentVisit);
        // Descending order (recent first) -> z_repo (20), a_repo (10), m_repo (5)
        assert_eq!(app.config.items[0], "z_repo");
        assert_eq!(app.config.items[1], "a_repo");
        assert_eq!(app.config.items[2], "m_repo");
    }
}
