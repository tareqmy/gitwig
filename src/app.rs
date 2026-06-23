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

/// What operation the remote picker was opened for.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RemotePickerAction {
    PushBranch,
    PushTag,
    PushAllTags,
    DeleteRemoteTag,
    FetchRemote,
}

/// Interaction modes for the item list.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// Browsing the list.
    Normal,
    #[allow(dead_code)]
    Adding,
    /// Typing replacement text for the selected item.
    Editing,
    /// Asking the user to confirm deletion of the selected item.
    ConfirmDelete,
    /// Showing the full shortcut reference as a centered overlay.
    Help,
    /// Showing the full-screen detail view for the selected item.
    Detail,
    /// Showing the shortcut reference overlay inside the detail view.
    DetailHelp,
    /// Typing a commit message.
    CommitInput,
    /// Typing a branch name to create.
    BranchCreateInput,
    /// Typing a tag name to create.
    TagCreateInput,
    /// Confirming deletion of a branch.
    BranchDeleteConfirm,
    /// Confirming push of a branch.
    BranchPushConfirm,
    /// Confirming deletion of a tag.
    TagDeleteConfirm,
    /// Confirming push of a tag.
    TagPushConfirm,
    /// Confirming push of all tags.
    TagPushAllConfirm,
    /// Confirming deletion of a stash.
    StashDeleteConfirm,
    /// Confirming apply of a stash.
    StashApplyConfirm,
    /// Picking a remote when multiple are available.
    RemotePicker,
    /// Typing a search query for commits.
    CommitSearchInput,
    /// Confirming merge of a branch.
    BranchMergeConfirm,
    /// Confirming rebase onto a branch.
    BranchRebaseConfirm,
    /// Confirming interactive rebase onto a branch.
    BranchInteractiveRebaseConfirm,
    /// Confirming discarding changes in a file.
    DiscardChangesConfirm,
    /// Inspecting a selected commit.
    Inspect,
    /// Settings page.
    Settings,
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
    FileContent,
    Remotes,
    Stashes,
    StashedFiles,
}

/// Resizable splitter identifier.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Splitter {
    InspectHorizontal,  // Left panel vs Right (Diff) panel
    InspectVertical,    // Top sub-panel vs Bottom sub-panel in left panel
    WorkspaceMain,      // Commits list vs staging/files details (vertical split, top/bottom)
    FilesHorizontal,    // Files view: left (tree) vs right (preview)
    BranchesHorizontal, // Branches view: left (local) vs right (remote)
    StashesHorizontal,  // Stashes view: left (lists) vs right (diff)
    StashesVertical,    // Stashes view: top list vs bottom files list
    OverviewHorizontal, // Overview view: left (info) vs right (stats)
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
            Self::Files => Self::FileContent,
            Self::FileContent => Self::Files,
            Self::Remotes => Self::Remotes,
            Self::Stashes => Self::Stashes,
            Self::StashedFiles => Self::StashedFiles,
        }
    }

    /// Move back to the previous section in the cycle.
    pub fn prev(self) -> Self {
        match self {
            Self::Commits => Self::StagingDetails,
            Self::Staged => Self::Commits,
            Self::Unstaged => Self::Staged,
            Self::CommitDetails => Self::Unstaged,
            Self::StagingDetails => Self::CommitDetails,
            Self::LocalBranches => Self::RemoteBranches,
            Self::RemoteBranches => Self::LocalBranches,
            Self::LocalTags => Self::RemoteTags,
            Self::RemoteTags => Self::LocalTags,
            Self::Files => Self::FileContent,
            Self::FileContent => Self::Files,
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
    /// Active query for filtering commits in the commits panel
    pub commit_search_query: Option<String>,
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
    /// Vertical scroll offset for the commit input popup.
    pub commit_input_scroll: usize,
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
    /// Vertical scroll offset for the file content preview in Files tab.
    pub file_content_scroll: usize,
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
    /// Whether lazygit launch is pending.
    pub pending_lazygit: bool,
    /// Whether fzf search launch is pending.
    pub pending_fzf: bool,
    /// Whether fzf files search launch is pending.
    pub pending_files_fzf: bool,
    /// Whether interactive rebase is pending.
    pub pending_interactive_rebase: Option<(PathBuf, String)>,
    /// Target branch name and remote flag for deletion/creation actions.
    pub branch_action_target: Option<(String, bool)>,
    /// Target commit OID for tag creation.
    pub tag_action_target_oid: Option<String>,
    /// Target tag name and remote flag for deletion action.
    pub tag_delete_target: Option<(String, bool)>,
    /// Target tag name for push action.
    pub tag_push_target: Option<String>,
    /// Target file path and staged flag for discard/revert action.
    pub discard_target: Option<(String, bool)>,
    /// Simulated fetch progress percentage.
    pub fetch_progress: u16,
    /// Option to delete the stash after applying.
    pub stash_apply_delete_after: bool,
    /// Option to amend the last commit.
    pub commit_amend: bool,
    /// Preserved original order of repository items from the config.
    pub original_items: Vec<String>,
    /// Which action the remote picker was opened for.
    pub remote_picker_action: Option<RemotePickerAction>,
    /// Selected row in the remote picker popup.
    pub remote_picker_selection: usize,
    /// Percentage width of the left panel in the Inspect view (default: 40).
    pub inspect_horizontal_split_pct: u16,
    /// Percentage height of the top left sub-panel in the Inspect view (default: 50).
    pub inspect_vertical_split_pct: u16,
    /// Percentage height of the commits list in the Workspace tab (default: 50).
    pub workspace_main_split_pct: u16,
    /// Percentage width of the files tree in the Files tab (default: 45).
    pub files_horizontal_split_pct: u16,
    /// Percentage width of the local branches list in the Branches tab (default: 50).
    pub branches_horizontal_split_pct: u16,
    /// Percentage width of the left list column in the Stashes tab (default: 35).
    pub stashes_horizontal_split_pct: u16,
    /// Percentage height of the top stashes list in the Stashes tab (default: 50).
    pub stashes_vertical_split_pct: u16,
    /// Percentage width of the left overview panel in the Overview tab (default: 50).
    pub overview_horizontal_split_pct: u16,
    /// Active drag splitter if dragging is in progress.
    pub active_drag_splitter: Option<Splitter>,
    pub settings_selected_index: usize,
    pub settings_editing: bool,
    pub settings_theme_list: Vec<String>,
    pub settings_theme_index: usize,
    pub last_staging_focus: DetailSection,
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
        crate::ui::update_theme(&config.theme);
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
            commit_search_query: None,
            file_selection: 0,
            staging_file_selection: 0,
            file_diff: Vec::new(),
            diff_scroll: 0,
            commit_details_scroll: 0,
            commit_input_scroll: 0,
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
            file_content_scroll: 0,
            expanded_folders: std::collections::HashSet::new(),
            visible_files: Vec::new(),
            graph_scroll: 0,
            commit_editing: false,
            status_expanded: false,
            tx,
            rx,
            fetching: false,
            pending_gitui: false,
            pending_lazygit: false,
            pending_fzf: false,
            pending_files_fzf: false,
            pending_interactive_rebase: None,
            branch_action_target: None,
            tag_action_target_oid: None,
            tag_delete_target: None,
            tag_push_target: None,
            discard_target: None,
            fetch_progress: 0,
            stash_apply_delete_after: true,
            commit_amend: false,
            remote_picker_action: None,
            remote_picker_selection: 0,
            inspect_horizontal_split_pct: 40,
            inspect_vertical_split_pct: 50,
            workspace_main_split_pct: 50,
            files_horizontal_split_pct: 25,
            branches_horizontal_split_pct: 50,
            stashes_horizontal_split_pct: 35,
            stashes_vertical_split_pct: 50,
            overview_horizontal_split_pct: 50,
            active_drag_splitter: None,
            settings_selected_index: 0,
            settings_editing: false,
            settings_theme_list: Vec::new(),
            settings_theme_index: 0,
            last_staging_focus: DetailSection::Staged,
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
        self.pending_fzf = true;
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
        let mut zipped: Vec<(String, ItemStatus)> = match self.config.sort_by {
            SortOrder::Custom => {
                let mut status_map: std::collections::HashMap<String, ItemStatus> = self
                    .config
                    .items
                    .drain(..)
                    .zip(self.statuses.drain(..))
                    .collect();
                let mut z: Vec<(String, ItemStatus)> = self
                    .original_items
                    .iter()
                    .map(|item| {
                        let status = status_map
                            .remove(item)
                            .unwrap_or_else(|| repo::inspect_summary(item));
                        (item.clone(), status)
                    })
                    .collect();
                if self.config.sort_reverse {
                    z.reverse();
                }
                z
            }
            SortOrder::Alphabetical => {
                let mut z: Vec<(String, ItemStatus)> = self
                    .config
                    .items
                    .drain(..)
                    .zip(self.statuses.drain(..))
                    .collect();
                z.sort_by(|a, b| a.0.cmp(&b.0));
                if self.config.sort_reverse {
                    z.reverse();
                }
                z
            }
            SortOrder::RecentVisit => {
                let visits = &self.config.visits;
                let mut z: Vec<(String, ItemStatus)> = self
                    .config
                    .items
                    .drain(..)
                    .zip(self.statuses.drain(..))
                    .collect();
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
                let mut z: Vec<(String, ItemStatus)> = self
                    .config
                    .items
                    .drain(..)
                    .zip(self.statuses.drain(..))
                    .collect();
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

        let selected_item = self.config.items.get(self.selected_index).cloned();

        self.sort_items_in_place();

        if let Some(item) = selected_item {
            if let Some(pos) = self.config.items.iter().position(|x| x == &item) {
                self.selected_index = pos;
            }
        }

        self.persist("Sort mode updated");
    }

    pub fn toggle_sort_reverse(&mut self) {
        self.config.sort_reverse = !self.config.sort_reverse;

        let selected_item = self.config.items.get(self.selected_index).cloned();

        self.sort_items_in_place();

        if let Some(item) = selected_item {
            if let Some(pos) = self.config.items.iter().position(|x| x == &item) {
                self.selected_index = pos;
            }
        }

        self.persist("Sort direction updated");
    }

    pub fn toggle_pin_selected(&mut self) {
        if self.config.items.is_empty() {
            return;
        }
        let selected_item = self.config.items[self.selected_index].clone();
        if self.config.pinned.contains(&selected_item) {
            self.config.pinned.remove(&selected_item);
            self.status_message = Some("Unpinned repository".to_string());
        } else {
            self.config.pinned.insert(selected_item.clone());
            self.status_message = Some("Pinned repository".to_string());
        }

        self.sort_items_in_place();

        if let Some(pos) = self.config.items.iter().position(|x| x == &selected_item) {
            self.selected_index = pos;
        }

        let msg = self
            .status_message
            .as_deref()
            .unwrap_or("Saved")
            .to_string();
        self.persist(&msg);
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
            self.file_content_scroll = 0;
            self.expanded_folders.clear();
            self.rebuild_visible_files();
            self.detail_tab = 0;
            self.graph_scroll = 0;
            self.mode = Mode::Detail;
            self.refresh_file_diff();
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
            let mut next_focus = if reverse {
                self.detail_focus.prev()
            } else {
                self.detail_focus.next()
            };
            for _ in 0..5 {
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
                    DetailSection::CommitDetails => self.is_uncommitted_selected(),
                    DetailSection::StagingDetails => {
                        if self.is_uncommitted_selected() {
                            self.is_staged_empty() && self.is_unstaged_empty()
                        } else {
                            self.is_selected_commit_empty()
                        }
                    }
                    _ => false,
                };
                if skip {
                    next_focus = if reverse {
                        next_focus.prev()
                    } else {
                        next_focus.next()
                    };
                } else {
                    break;
                }
            }
            self.detail_focus = next_focus;
        } else {
            self.detail_focus = if reverse {
                self.detail_focus.prev()
            } else {
                self.detail_focus.next()
            };
        }
        if self.detail_focus == DetailSection::Staged
            || self.detail_focus == DetailSection::Unstaged
        {
            self.last_staging_focus = self.detail_focus;
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
        self.file_content_scroll = 0;
    }

    /// Move file selection down in the Files tab.
    pub fn file_list_down(&mut self) {
        let total = self.visible_files.len();
        if total > 0 && self.file_list_selection + 1 < total {
            self.file_list_selection += 1;
            self.file_content_scroll = 0;
        }
    }

    /// Scroll file selection up by page.
    pub fn file_list_page_up(&mut self, page: usize) {
        self.file_list_selection = self.file_list_selection.saturating_sub(page);
        self.file_content_scroll = 0;
    }

    /// Scroll file selection down by page.
    pub fn file_list_page_down(&mut self, page: usize) {
        let total = self.visible_files.len();
        if total > 0 {
            self.file_list_selection =
                (self.file_list_selection + page).min(total.saturating_sub(1));
            self.file_content_scroll = 0;
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
    /// If multiple remotes exist and no upstream is configured, opens the remote picker first.
    pub fn request_branch_push(&mut self) {
        if self.fetching {
            return;
        }
        if let Some(repo::ItemDetail::Repo { info, resolved }) = &self.current_detail {
            if let Some(branch_info) = info.local_branches.get(self.local_branch_selection) {
                let branch_name = branch_info.name.clone();
                // Check if this branch already has a configured upstream remote.
                let has_upstream = git2::Repository::open(resolved)
                    .ok()
                    .and_then(|repo| {
                        repo.find_branch(&branch_name, git2::BranchType::Local)
                            .ok()
                            .and_then(|b| {
                                b.upstream().ok().and_then(|up| {
                                    up.get()
                                        .name()
                                        .ok()
                                        .and_then(|n| repo.branch_upstream_remote(n).ok())
                                })
                            })
                    })
                    .is_some();

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
            let (repo_path, remotes_len, first_remote) =
                if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
                    (
                        resolved.clone(),
                        info.remotes.len(),
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
            if info.remotes.len() > 1 {
                self.remote_picker_action = Some(RemotePickerAction::PushAllTags);
                self.remote_picker_selection = 0;
                self.mode = Mode::RemotePicker;
            } else {
                self.mode = Mode::TagPushAllConfirm;
            }
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

    pub fn request_branch_merge(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            match self.detail_focus {
                DetailSection::LocalBranches => {
                    if let Some(branch_info) = info.local_branches.get(self.local_branch_selection)
                    {
                        self.branch_action_target = Some((branch_info.name.clone(), false));
                        self.mode = Mode::BranchMergeConfirm;
                    }
                }
                DetailSection::RemoteBranches => {
                    if let Some(branch_info) =
                        info.remote_branches.get(self.remote_branch_selection)
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
                self.status_message = Some(format!(
                    "Merging '{}' into '{}'...",
                    branch_name, current_branch
                ));

                let repo_path = resolved.clone();
                let target_name = branch_name.clone();
                let tx = self.tx.clone();

                std::thread::spawn(move || {
                    let res = (|| -> Result<String, Box<dyn std::error::Error>> {
                        let output = std::process::Command::new("git")
                            .arg("merge")
                            .arg(&target_name)
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
                if let Some(branch_info) = info.local_branches.get(self.local_branch_selection) {
                    if !branch_info.is_head {
                        self.branch_action_target = Some((branch_info.name.clone(), false));
                        self.mode = Mode::BranchRebaseConfirm;
                    }
                }
            } else if self.detail_focus == DetailSection::RemoteBranches {
                if let Some(branch_info) = info.remote_branches.get(self.remote_branch_selection) {
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
                self.status_message = Some(format!(
                    "Rebasing '{}' onto '{}'...",
                    current_branch, branch_name
                ));

                let repo_path = resolved.clone();
                let target_name = branch_name.clone();
                let tx = self.tx.clone();

                std::thread::spawn(move || {
                    let res = (|| -> Result<String, Box<dyn std::error::Error>> {
                        let output = std::process::Command::new("git")
                            .arg("rebase")
                            .arg(&target_name)
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
                if let Some(branch_info) = info.local_branches.get(self.local_branch_selection) {
                    if !branch_info.is_head {
                        self.branch_action_target = Some((branch_info.name.clone(), false));
                        self.mode = Mode::BranchInteractiveRebaseConfirm;
                    }
                }
            } else if self.detail_focus == DetailSection::RemoteBranches {
                if let Some(branch_info) = info.remote_branches.get(self.remote_branch_selection) {
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
                    self.commit_selection.saturating_sub(1)
                } else {
                    self.commit_selection
                };
                info.commits
                    .get(commit_idx)
                    .map(|c| (resolved.clone(), c.oid.clone()))
            }
            _ => None,
        };

        if let Some((repo_path, commit_oid)) = params {
            // Check if the commit is root using git2
            let is_root = if let Ok(repo) = git2::Repository::open(&repo_path) {
                if let Ok(oid) = git2::Oid::from_str(&commit_oid) {
                    if let Ok(commit) = repo.find_commit(oid) {
                        commit.parent_count() == 0
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            let target = if is_root {
                "--root".to_string()
            } else {
                format!("{}~1", commit_oid)
            };
            self.pending_interactive_rebase = Some((repo_path, target));
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

    pub fn get_file_content_line_count(&self) -> usize {
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
            if let Some(selected_item) = self.visible_files.get(self.file_list_selection) {
                if selected_item.is_dir {
                    let prefix = if selected_item.full_path.is_empty() {
                        "".to_string()
                    } else {
                        format!("{}/", selected_item.full_path)
                    };
                    let mut direct_children = std::collections::BTreeSet::new();
                    for f_path in &info.files {
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
                    if direct_children.is_empty() {
                        1
                    } else {
                        direct_children.len()
                    }
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

    /// Scroll the file content panel up by one line.
    pub fn file_content_scroll_up(&mut self) {
        self.file_content_scroll = self.file_content_scroll.saturating_sub(1);
    }

    /// Scroll the file content panel down by one line.
    pub fn file_content_scroll_down(&mut self) {
        let max = self.get_file_content_line_count().saturating_sub(1);
        if self.file_content_scroll < max {
            self.file_content_scroll += 1;
        }
    }

    /// Scroll the file content panel up by `page` lines.
    pub fn file_content_scroll_page_up(&mut self, page: usize) {
        self.file_content_scroll = self.file_content_scroll.saturating_sub(page);
    }

    /// Scroll the file content panel down by `page` lines.
    pub fn file_content_scroll_page_down(&mut self, page: usize) {
        let max = self.get_file_content_line_count().saturating_sub(1);
        self.file_content_scroll = (self.file_content_scroll + page).min(max);
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
    /// Total number of rows in the Commits panel (dirty row + real commits).
    fn commit_total(&self) -> usize {
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => {
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                let show_dirty = if dirty {
                    if let Some(ref query) = self.commit_search_query {
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

    /// Total files in the currently-selected commit's Changed Files panel.
    fn file_total(&self) -> usize {
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => {
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                let show_dirty = if dirty {
                    if let Some(ref query) = self.commit_search_query {
                        "<uncommitted>".contains(&query.to_lowercase())
                    } else {
                        true
                    }
                } else {
                    false
                };
                // Uncommitted row (staging area) has no file list.
                if show_dirty && self.commit_selection == 0 {
                    return 0;
                }
                let idx = if show_dirty {
                    self.commit_selection.saturating_sub(1)
                } else {
                    self.commit_selection
                };
                let filtered = self.get_filtered_commits();
                filtered.get(idx).map(|c| c.files.len()).unwrap_or(0)
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
                let show_dirty = if dirty {
                    if let Some(ref query) = self.commit_search_query {
                        "<uncommitted>".contains(&query.to_lowercase())
                    } else {
                        true
                    }
                } else {
                    false
                };
                show_dirty && self.commit_selection == 0
            }
            _ => false,
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

    pub fn is_selected_commit_empty(&self) -> bool {
        match &self.current_detail {
            Some(ItemDetail::Repo { info, .. }) => {
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                let show_dirty = if dirty {
                    if let Some(ref query) = self.commit_search_query {
                        "<uncommitted>".contains(&query.to_lowercase())
                    } else {
                        true
                    }
                } else {
                    false
                };
                if show_dirty && self.commit_selection == 0 {
                    return true;
                }
                let commit_idx = if show_dirty {
                    self.commit_selection.saturating_sub(1)
                } else {
                    self.commit_selection
                };
                let filtered = self.get_filtered_commits();
                filtered
                    .get(commit_idx)
                    .map(|c| c.files.is_empty())
                    .unwrap_or(true)
            }
            _ => true,
        }
    }

    fn current_diff_params(&self) -> Option<(PathBuf, String, String)> {
        match &self.current_detail {
            Some(ItemDetail::Repo { resolved, info }) => {
                let dirty = !info.changes.staged.is_empty()
                    || !info.changes.unstaged.is_empty()
                    || !info.changes.untracked.is_empty()
                    || !info.changes.conflicted.is_empty();
                let show_dirty = if dirty {
                    if let Some(ref query) = self.commit_search_query {
                        "<uncommitted>".contains(&query.to_lowercase())
                    } else {
                        true
                    }
                } else {
                    false
                };
                if show_dirty && self.commit_selection == 0 {
                    return None;
                }
                let commit_idx = if show_dirty {
                    self.commit_selection.saturating_sub(1)
                } else {
                    self.commit_selection
                };
                let filtered = self.get_filtered_commits();
                let commit = filtered.get(commit_idx)?;
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
                let focus_to_use = match self.detail_focus {
                    DetailSection::Staged => DetailSection::Staged,
                    DetailSection::Unstaged => DetailSection::Unstaged,
                    DetailSection::StagingDetails => self.last_staging_focus,
                    _ => {
                        self.file_diff.clear();
                        return;
                    }
                };
                let (files, staged) = match focus_to_use {
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

    pub fn request_discard_changes(&mut self) {
        let params = match &self.current_detail {
            Some(repo::ItemDetail::Repo { resolved, info }) => match self.detail_focus {
                DetailSection::Staged => info
                    .changes
                    .staged
                    .get(self.staging_file_selection)
                    .map(|f| (resolved.clone(), f.path.clone(), true)),
                DetailSection::Unstaged => info
                    .changes
                    .unstaged
                    .get(self.staging_file_selection)
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
                match repo::discard_file_changes(resolved, &file_path, staged) {
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

    pub fn close_detail(&mut self) {
        self.current_detail = None;
        self.commit_search_query = None;
        self.mode = Mode::Normal;
    }

    pub fn get_filtered_commits(&self) -> Vec<&crate::repo::CommitEntry> {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            if let Some(ref query) = self.commit_search_query {
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

    pub fn clamp_commit_selection(&mut self) {
        let total = self.commit_total();
        if total == 0 {
            self.commit_selection = 0;
        } else if self.commit_selection >= total {
            self.commit_selection = total - 1;
        }
    }

    pub fn start_commit_search(&mut self) {
        self.input_buffer = self.commit_search_query.clone().unwrap_or_default();
        self.mode = Mode::CommitSearchInput;
    }

    pub fn cancel_commit_search(&mut self) {
        self.commit_search_query = None;
        self.clamp_commit_selection();
        self.file_selection = 0;
        self.diff_scroll = 0;
        self.refresh_file_diff();
        self.mode = Mode::Detail;
    }

    pub fn commit_search_input_change(&mut self) {
        self.commit_search_query = if self.input_buffer.is_empty() {
            None
        } else {
            Some(self.input_buffer.clone())
        };
        self.clamp_commit_selection();
        self.file_selection = 0;
        self.diff_scroll = 0;
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
            self.input_buffer.clear();
            self.commit_editing = true;
            self.commit_amend = false;
            self.commit_input_scroll = 0;
            self.mode = Mode::CommitInput;
        } else {
            self.status_message = Some("No staged changes to commit".to_string());
        }
    }

    /// Cancels commit input and returns to the detail view.
    pub fn cancel_commit(&mut self) {
        self.input_buffer.clear();
        self.commit_input_scroll = 0;
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
        self.commit_input_scroll = 0;
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
                self.input_buffer = self.config.fzf.max_depth.to_string();
            }
            5 => {
                self.settings_editing = true;
                self.input_buffer = self.config.fzf.start_dir.clone();
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
                    let themes_dir = self
                        .config_path
                        .parent()
                        .unwrap_or(&self.config_path)
                        .join("themes");
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
                    self.config.fzf.max_depth = val;
                    self.persist("FZF max depth updated");
                    self.settings_editing = false;
                    self.input_buffer.clear();
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            5 => {
                self.config.fzf.start_dir = trimmed.to_string();
                self.persist("FZF start directory updated");
                self.settings_editing = false;
                self.input_buffer.clear();
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
        let themes_dir = self
            .config_path
            .parent()
            .unwrap_or(&self.config_path)
            .join("themes");
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

    fn canonical_path(p: &std::path::Path) -> PathBuf {
        match std::fs::canonicalize(p) {
            Ok(canon) => canon,
            Err(_) => p.to_path_buf(),
        }
    }

    pub fn commit_add(&mut self) {
        let trimmed = self.input_buffer.trim().to_string();
        if !trimmed.is_empty() {
            let new_expanded = repo::expand_tilde(&trimmed);
            let new_canonical = Self::canonical_path(&new_expanded);
            let already_exists = self.config.items.iter().any(|item| {
                let item_expanded = repo::expand_tilde(item);
                item.trim() == trimmed
                    || item_expanded == new_expanded
                    || Self::canonical_path(&item_expanded) == new_canonical
            });
            if already_exists {
                self.status_message = Some("Repository already added".to_string());
                self.input_buffer.clear();
                self.mode = Mode::Normal;
                return;
            }

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

    pub fn add_repo_path(&mut self, path: String) {
        let trimmed = path.trim().to_string();
        if !trimmed.is_empty() {
            let new_expanded = repo::expand_tilde(&trimmed);
            let new_canonical = Self::canonical_path(&new_expanded);
            let already_exists = self.config.items.iter().any(|item| {
                let item_expanded = repo::expand_tilde(item);
                item.trim() == trimmed
                    || item_expanded == new_expanded
                    || Self::canonical_path(&item_expanded) == new_canonical
            });
            if already_exists {
                self.status_message = Some("Repository already added".to_string());
                return;
            }

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
            self.persist("Added repository");
        }
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

            if self.config.pinned.remove(&old_item) {
                self.config.pinned.insert(trimmed.clone());
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
            self.config.pinned.remove(&item);
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
            1 => {
                self.detail_focus = DetailSection::Files;
                self.file_content_scroll = 0;
            }
            3 => self.detail_focus = DetailSection::LocalBranches,
            4 => {
                self.detail_focus = DetailSection::LocalTags;
                self.fetch_remote_tags(false);
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

    pub fn fetch_remote_tags(&mut self, show_progress: bool) {
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
            // Use the currently selected remote in the Remotes tab if available,
            // otherwise fall back to the first remote.
            let remote = info
                .remotes
                .get(self.remote_selection)
                .or_else(|| info.remotes.first());
            if let Some(remote) = remote {
                let repo_path = resolved.clone();
                let remote_name = remote.name.clone();
                let tx = self.tx.clone();
                if show_progress {
                    self.fetching = true;
                    self.status_message = Some(format!("Fetching tags from '{}'...", remote_name));
                }
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
                    let output = std::process::Command::new("git")
                        .arg("fetch")
                        .arg(&remote_name)
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
            info.remotes
                .get(self.remote_picker_selection)
                .map(|r| r.name.clone())
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
                self.execute_tag_push_all_to(&remote_name);
                self.mode = Mode::Detail;
            }
            RemotePickerAction::DeleteRemoteTag => {
                if let Some((tag_name, _)) = self.tag_delete_target.take() {
                    self.execute_delete_remote_tag_on(&tag_name, &remote_name);
                }
                self.mode = Mode::Detail;
            }
            RemotePickerAction::FetchRemote => {
                self.remote_selection = self.remote_picker_selection;
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
                let mut cmd = std::process::Command::new("git");
                cmd.arg("push")
                    .arg("-u")
                    .arg(&remote_name)
                    .arg(&branch_name)
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
                app.fetching = false;
            } else if let Some(err_msg) = msg.strip_prefix("REMOTE_TAGS_ERR:") {
                app.status_message = Some(err_msg.to_string());
                app.fetching = false;
            } else {
                let success_fetch = msg.starts_with("Fetched remote ");
                app.status_message = Some(msg);
                app.fetching = false;
                if let Some(item) = app.config.items.get(app.selected_index) {
                    app.current_detail = Some(repo::inspect_detail(item));
                    app.rebuild_visible_files();
                    if success_fetch {
                        app.fetch_remote_tags(false);
                    }
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

        if app.pending_lazygit {
            app.pending_lazygit = false;
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
                    let status = std::process::Command::new("lazygit")
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
                            app.status_message = Some("Returned from lazygit".to_string());
                            app.refresh_selected_status();
                        }
                        Ok(_) => {
                            app.status_message = Some("lazygit exited with error".to_string());
                            app.refresh_selected_status();
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            app.status_message = Some("lazygit is not installed".to_string());
                        }
                        Err(e) => {
                            app.status_message = Some(format!("Could not run lazygit: {}", e));
                        }
                    }
                }
            }
        }

        if let Some((repo_path, target)) = app.pending_interactive_rebase.take() {
            let raw_res = crossterm::terminal::disable_raw_mode();
            let exec_res = crossterm::execute!(
                std::io::stdout(),
                crossterm::terminal::LeaveAlternateScreen,
                crossterm::event::DisableMouseCapture
            );
            let cursor_res = terminal.show_cursor();

            if raw_res.is_ok() && exec_res.is_ok() && cursor_res.is_ok() {
                let status = std::process::Command::new("git")
                    .arg("rebase")
                    .arg("-i")
                    .arg(&target)
                    .current_dir(&repo_path)
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
                        app.status_message =
                            Some("Interactive rebase completed successfully".to_string());
                    }
                    Ok(s) => {
                        app.status_message = Some(format!(
                            "Rebase exited with status: {}. Check terminal/git status.",
                            s
                        ));
                    }
                    Err(e) => {
                        app.status_message = Some(format!("Failed to run git rebase: {}", e));
                    }
                }
                if let Some(item) = app.config.items.get(app.selected_index) {
                    let new_status = repo::inspect_summary(item);
                    if let Some(slot) = app.statuses.get_mut(app.selected_index) {
                        *slot = new_status;
                    }
                }
                app.refresh_detail();
            }
        }

        if app.pending_fzf {
            app.pending_fzf = false;

            let raw_res = crossterm::terminal::disable_raw_mode();
            let exec_res = crossterm::execute!(
                std::io::stdout(),
                crossterm::terminal::LeaveAlternateScreen,
                crossterm::event::DisableMouseCapture
            );
            let cursor_res = terminal.show_cursor();

            if raw_res.is_ok() && exec_res.is_ok() && cursor_res.is_ok() {
                let max_depth = app.config.fzf.max_depth;
                let fd_excludes = app
                    .config
                    .fzf
                    .excludes
                    .iter()
                    .map(|x| format!("--exclude '{}'", x))
                    .collect::<Vec<String>>()
                    .join(" ");
                let find_prunes = app
                    .config
                    .fzf
                    .excludes
                    .iter()
                    .map(|x| format!("-path '*/{}'", x))
                    .collect::<Vec<String>>()
                    .join(" -o ");
                let find_prune_clause = if find_prunes.is_empty() {
                    "".to_string()
                } else {
                    format!("\\( {} \\) -prune -o ", find_prunes)
                };

                let expanded_start_dir = crate::repo::expand_tilde(&app.config.fzf.start_dir);
                let start_dir_str = expanded_start_dir.to_string_lossy().into_owned();
                let start_dir = start_dir_str.replace('\'', "'\\''");

                let cmd = format!(
                    "if ! command -v fzf >/dev/null 2>&1; then exit 127; fi; (command -v fd >/dev/null 2>&1 && fd . '{}' --type d --max-depth {} {} 2>/dev/null || find '{}' -maxdepth {} {} -type d -print 2>/dev/null) | fzf",
                    start_dir, max_depth, fd_excludes, start_dir, max_depth, find_prune_clause
                );

                let output = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .stdin(std::process::Stdio::inherit())
                    .stderr(std::process::Stdio::inherit())
                    .stdout(std::process::Stdio::piped())
                    .output();

                let _ = crossterm::terminal::enable_raw_mode();
                let _ = crossterm::execute!(
                    std::io::stdout(),
                    crossterm::terminal::EnterAlternateScreen,
                    crossterm::event::EnableMouseCapture
                );
                let _ = terminal.clear();

                match output {
                    Ok(out) => {
                        if out.status.success() {
                            let selected = String::from_utf8_lossy(&out.stdout).trim().to_string();
                            if !selected.is_empty() {
                                app.add_repo_path(selected);
                            }
                        } else if out.status.code() == Some(127) {
                            app.status_message =
                                Some("fzf is not installed. Please install fzf.".to_string());
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        app.status_message = Some("fzf is not installed".to_string());
                    }
                    Err(e) => {
                        app.status_message = Some(format!("Could not run fzf: {}", e));
                    }
                }
            }
        }

        if app.pending_files_fzf {
            app.pending_files_fzf = false;
            if let Some(repo::ItemDetail::Repo { resolved, info }) = &app.current_detail {
                let repo_path = resolved.clone();
                let files = info.files.clone();

                let raw_res = crossterm::terminal::disable_raw_mode();
                let exec_res = crossterm::execute!(
                    std::io::stdout(),
                    crossterm::terminal::LeaveAlternateScreen,
                    crossterm::event::DisableMouseCapture
                );
                let cursor_res = terminal.show_cursor();

                if raw_res.is_ok() && exec_res.is_ok() && cursor_res.is_ok() {
                    let mut child_cmd = std::process::Command::new("fzf");
                    child_cmd.arg("--prompt").arg("Select file> ");
                    let child = child_cmd
                        .current_dir(&repo_path)
                        .stdin(std::process::Stdio::piped())
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::inherit())
                        .spawn();

                    let output = match child {
                        Ok(mut c) => {
                            if let Some(mut stdin) = c.stdin.take() {
                                use std::io::Write;
                                for file in files {
                                    let _ = writeln!(stdin, "{}", file);
                                }
                            }
                            c.wait_with_output()
                        }
                        Err(e) => Err(e),
                    };

                    let _ = crossterm::terminal::enable_raw_mode();
                    let _ = crossterm::execute!(
                        std::io::stdout(),
                        crossterm::terminal::EnterAlternateScreen,
                        crossterm::event::EnableMouseCapture
                    );
                    let _ = terminal.clear();

                    match output {
                        Ok(out) => {
                            if out.status.success() {
                                let selected =
                                    String::from_utf8_lossy(&out.stdout).trim().to_string();
                                if !selected.is_empty() {
                                    // Expand the parent directories of the selected file
                                    let parts: Vec<&str> = selected.split('/').collect();
                                    let mut accumulated = String::new();
                                    for part in parts.iter().take(parts.len().saturating_sub(1)) {
                                        if !accumulated.is_empty() {
                                            accumulated.push('/');
                                        }
                                        accumulated.push_str(part);
                                        app.expanded_folders.insert(accumulated.clone());
                                    }
                                    app.rebuild_visible_files();
                                    if let Some(pos) = app
                                        .visible_files
                                        .iter()
                                        .position(|item| item.full_path == selected)
                                    {
                                        app.file_list_selection = pos;
                                        app.file_content_scroll = 0;
                                        app.detail_focus = DetailSection::Files;
                                    }
                                    app.status_message = Some(format!("Selected {}", selected));
                                }
                            } else if out.status.code() == Some(127) {
                                app.status_message =
                                    Some("fzf is not installed. Please install fzf.".to_string());
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            app.status_message = Some("fzf is not installed".to_string());
                        }
                        Err(e) => {
                            app.status_message = Some(format!("Could not run fzf: {}", e));
                        }
                    }
                }
            } else {
                app.status_message = Some("Not inside a repository".to_string());
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
    use crate::config::{FzfConfig, SortOrder, ThemeConfig};
    use std::collections::HashMap;

    struct TestFileGuard {
        path: PathBuf,
    }

    impl Drop for TestFileGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

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
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
        };
        let temp_path = std::env::temp_dir().join("twig_test_config_sort.toml");
        let _guard = TestFileGuard {
            path: temp_path.clone(),
        };
        let mut app = App::new(config, temp_path);

        // Assert initial custom sort
        assert_eq!(app.config.items[0], "z_repo");
        assert_eq!(app.config.items[1], "a_repo");

        // Cycle to alphabetical
        app.cycle_sort_order();
        assert_eq!(app.config.sort_by, SortOrder::Alphabetical);
        assert_eq!(app.config.items[0], "a_repo");
        assert_eq!(app.config.items[1], "m_repo");
        assert_eq!(app.config.items[2], "z_repo");

        // Toggle reverse sorting
        app.toggle_sort_reverse();
        assert!(app.config.sort_reverse);
        assert_eq!(app.config.items[0], "z_repo");
        assert_eq!(app.config.items[1], "m_repo");
        assert_eq!(app.config.items[2], "a_repo");

        // Toggle back
        app.toggle_sort_reverse();
        assert!(!app.config.sort_reverse);

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

    #[test]
    fn test_duplicate_prevention() {
        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
        };
        let temp_path = std::env::temp_dir().join("twig_test_config_duplicate.toml");
        // Ensure starting with a clean state and clean up upon drop
        let _ = std::fs::remove_file(&temp_path);
        let _guard = TestFileGuard {
            path: temp_path.clone(),
        };
        let mut app = App::new(config, temp_path);

        // 1. Test adding a repository via input buffer (commit_add)
        app.input_buffer = " /path/to/repo ".to_string(); // trimmed to "/path/to/repo"
        app.commit_add();
        assert_eq!(app.config.items.len(), 1);
        assert_eq!(app.config.items[0], "/path/to/repo");
        assert_eq!(app.status_message, Some("Saved".to_string()));
        app.status_message = None; // Reset

        // 2. Test trying to add the exact same repo path again (via commit_add)
        app.input_buffer = "/path/to/repo".to_string();
        app.commit_add();
        assert_eq!(app.config.items.len(), 1);
        assert_eq!(
            app.status_message,
            Some("Repository already added".to_string())
        );
        app.status_message = None; // Reset

        // 3. Test trying to add a tilde version of the same repo when it's resolved
        if let Some(home) = dirs::home_dir() {
            let home_str = home.to_string_lossy().to_string();
            // First add the tilde path
            app.input_buffer = "~/my_cool_repo".to_string();
            app.commit_add();
            assert_eq!(app.config.items.len(), 2);
            assert_eq!(app.config.items[1], "~/my_cool_repo");
            assert_eq!(app.status_message, Some("Saved".to_string()));
            app.status_message = None; // Reset

            // Now try to add the expanded absolute path
            let expanded_path = format!("{}/my_cool_repo", home_str);
            app.input_buffer = expanded_path;
            app.commit_add();
            // Should be rejected
            assert_eq!(app.config.items.len(), 2);
            assert_eq!(
                app.status_message,
                Some("Repository already added".to_string())
            );
            app.status_message = None; // Reset

            // Try the opposite direction: add a new absolute path, then try to add with tilde
            let new_abs = format!("{}/another_cool_repo", home_str);
            app.input_buffer = new_abs;
            app.commit_add();
            assert_eq!(app.config.items.len(), 3);
            assert_eq!(
                app.config.items[2],
                format!("{}/another_cool_repo", home_str)
            );
            assert_eq!(app.status_message, Some("Saved".to_string()));
            app.status_message = None; // Reset

            // Now try to add with tilde
            app.input_buffer = "~/another_cool_repo".to_string();
            app.commit_add();
            // Should be rejected
            assert_eq!(app.config.items.len(), 3);
            assert_eq!(
                app.status_message,
                Some("Repository already added".to_string())
            );
            app.status_message = None; // Reset
        }

        // 4. Test adding via add_repo_path directly
        let len_before = app.config.items.len();
        app.add_repo_path(" /another/path ".to_string());
        assert_eq!(app.config.items.len(), len_before + 1);
        assert_eq!(app.config.items.last().unwrap(), "/another/path");
        assert_eq!(app.status_message, Some("Added repository".to_string()));
        app.status_message = None; // Reset

        // Try duplicate via add_repo_path
        app.add_repo_path("/another/path".to_string());
        assert_eq!(app.config.items.len(), len_before + 1);
        assert_eq!(
            app.status_message,
            Some("Repository already added".to_string())
        );
    }

    #[test]
    fn test_pinning_and_sorting() {
        let config = Config {
            items: vec![
                "z_repo".to_string(),
                "a_repo".to_string(),
                "m_repo".to_string(),
            ],
            poll_interval_ms: 100,
            sort_by: SortOrder::Alphabetical,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
        };
        let temp_path = std::env::temp_dir().join("twig_test_config_pin.toml");
        let _guard = TestFileGuard {
            path: temp_path.clone(),
        };
        let mut app = App::new(config, temp_path);

        // Sorting is Alphabetical: initially items should be sorted as a_repo, m_repo, z_repo
        app.sort_items_in_place();
        assert_eq!(app.config.items[0], "a_repo");
        assert_eq!(app.config.items[1], "m_repo");
        assert_eq!(app.config.items[2], "z_repo");

        // Pin the last one ("z_repo", index 2)
        app.selected_index = 2;
        app.toggle_pin_selected();

        // After pinning, z_repo is pinned.
        // It must move to the top (index 0).
        // The selection cursor must also follow z_repo, meaning selected_index should become 0.
        assert!(app.config.pinned.contains("z_repo"));
        assert_eq!(app.config.items[0], "z_repo");
        assert_eq!(app.config.items[1], "a_repo");
        assert_eq!(app.config.items[2], "m_repo");
        assert_eq!(app.selected_index, 0);

        // Reverse sorting with z_repo pinned:
        // Pinned block is ["z_repo"]. Unpinned block is ["a_repo", "m_repo"] -> reverse alphabetical is ["m_repo", "a_repo"]
        // Pinned is kept on top: ["z_repo", "m_repo", "a_repo"]
        app.toggle_sort_reverse();
        assert_eq!(app.config.items[0], "z_repo");
        assert_eq!(app.config.items[1], "m_repo");
        assert_eq!(app.config.items[2], "a_repo");
        // selected_index should still track "z_repo" (which is at index 0)
        assert_eq!(app.selected_index, 0);

        // Toggle reverse back
        app.toggle_sort_reverse();

        // Pin m_repo too (currently at index 2)
        app.selected_index = 2; // "m_repo"
        app.toggle_pin_selected();

        // Now both z_repo and m_repo are pinned.
        // Alphabetical sort:
        // Pinned: m_repo, z_repo -> sorted alphabetically is ["m_repo", "z_repo"]
        // Unpinned: a_repo -> ["a_repo"]
        // Combined: ["m_repo", "z_repo", "a_repo"]
        assert_eq!(app.config.items[0], "m_repo");
        assert_eq!(app.config.items[1], "z_repo");
        assert_eq!(app.config.items[2], "a_repo");
        // cursor was on m_repo, which ended up at index 0
        assert_eq!(app.selected_index, 0);

        // Unpin m_repo (currently at index 0)
        app.selected_index = 0;
        app.toggle_pin_selected();

        // Now only z_repo is pinned.
        // Items should be ["z_repo", "a_repo", "m_repo"]
        assert_eq!(app.config.items[0], "z_repo");
        assert_eq!(app.config.items[1], "a_repo");
        assert_eq!(app.config.items[2], "m_repo");
    }

    #[test]
    fn test_commit_input_scroll() {
        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
        };
        let temp_path = std::env::temp_dir().join("twig_test_config_commit_scroll.toml");
        let _guard = TestFileGuard {
            path: temp_path.clone(),
        };
        let mut app = App::new(config, temp_path);

        assert_eq!(app.commit_input_scroll, 0);

        app.commit_input_scroll_down();
        assert_eq!(app.commit_input_scroll, 1);

        app.commit_input_scroll_down();
        assert_eq!(app.commit_input_scroll, 2);

        app.commit_input_scroll_up();
        assert_eq!(app.commit_input_scroll, 1);

        // Cancel resets it
        app.cancel_commit();
        assert_eq!(app.commit_input_scroll, 0);
    }

    #[test]
    fn test_splitter_dragging() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::layout::Rect;

        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
        };
        let temp_path = std::env::temp_dir().join("twig_test_config_splitter.toml");
        let _guard = TestFileGuard {
            path: temp_path.clone(),
        };
        let mut app = App::new(config, temp_path);

        // Mock the detail_areas to simulate a drawn UI frame.
        // Left panel is 40 columns wide (from 0 to 40), Right is 60 columns wide (40 to 100).
        // Total width = 100. Horizontal splitter is at column 40.
        // We set the bounding boxes.
        app.detail_areas.bottom_left = Some(Rect::new(0, 0, 40, 50));
        app.detail_areas.bottom_right = Some(Rect::new(40, 0, 60, 50));
        app.detail_areas.inspect_horizontal_splitter = Some(Rect::new(39, 0, 2, 50));

        // Click on the horizontal splitter
        let down_event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 39,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, down_event);
        assert_eq!(app.active_drag_splitter, Some(Splitter::InspectHorizontal));

        // Drag to column 30 (which means 30% of total width 100)
        let drag_event = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 30,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, drag_event);
        assert_eq!(app.inspect_horizontal_split_pct, 30);

        // Release mouse
        let up_event = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 30,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, up_event);
        assert_eq!(app.active_drag_splitter, None);

        // Test WorkspaceMain splitter dragging
        app.detail_areas.commits = Some(Rect::new(0, 0, 100, 20));
        app.detail_areas.bottom_right = Some(Rect::new(0, 20, 100, 30));
        app.detail_areas.workspace_main_splitter = Some(Rect::new(0, 19, 100, 2));

        // Click on the vertical workspace main splitter
        let down_event_main = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 19,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, down_event_main);
        assert_eq!(app.active_drag_splitter, Some(Splitter::WorkspaceMain));

        // Drag to row 25 (which is 50% height since total height is 50)
        let drag_event_main = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 10,
            row: 25,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, drag_event_main);
        assert_eq!(app.workspace_main_split_pct, 50);

        // Drag to row 15 (which is 30% height)
        let drag_event_main_2 = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 10,
            row: 15,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, drag_event_main_2);
        assert_eq!(app.workspace_main_split_pct, 30);

        // Release mouse
        let up_event_main = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 10,
            row: 15,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, up_event_main);
        assert_eq!(app.active_drag_splitter, None);

        // Test Files splitter dragging
        app.detail_areas.files = Some(Rect::new(0, 0, 45, 50));
        app.detail_areas.file_content = Some(Rect::new(45, 0, 55, 50));
        app.detail_areas.files_horizontal_splitter = Some(Rect::new(44, 0, 2, 50));

        let down_event_files = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 44,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, down_event_files);
        assert_eq!(app.active_drag_splitter, Some(Splitter::FilesHorizontal));

        let drag_event_files = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 60,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, drag_event_files);
        assert_eq!(app.files_horizontal_split_pct, 60);

        let up_event_files = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 60,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, up_event_files);
        assert_eq!(app.active_drag_splitter, None);

        // Test Branches splitter dragging
        app.detail_areas = DetailAreas::default();
        app.detail_areas.local_branches = Some(Rect::new(0, 0, 50, 50));
        app.detail_areas.remote_branches = Some(Rect::new(50, 0, 50, 50));
        app.detail_areas.branches_horizontal_splitter = Some(Rect::new(49, 0, 2, 50));

        let down_event_branches = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 49,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, down_event_branches);
        assert_eq!(app.active_drag_splitter, Some(Splitter::BranchesHorizontal));

        let drag_event_branches = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 35,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, drag_event_branches);
        assert_eq!(app.branches_horizontal_split_pct, 35);

        let up_event_branches = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 35,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, up_event_branches);
        assert_eq!(app.active_drag_splitter, None);

        // Test Stashes splitter dragging (horizontal & vertical)
        app.detail_areas = DetailAreas::default();
        app.detail_areas.stashes = Some(Rect::new(0, 0, 35, 25));
        app.detail_areas.stashed_files = Some(Rect::new(0, 25, 35, 25));
        app.detail_areas.bottom_right = Some(Rect::new(35, 0, 65, 50));
        app.detail_areas.stashes_horizontal_splitter = Some(Rect::new(34, 0, 2, 50));
        app.detail_areas.stashes_vertical_splitter = Some(Rect::new(0, 24, 35, 2));

        // Click stashes horizontal splitter
        let down_stashes_h = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 34,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, down_stashes_h);
        assert_eq!(app.active_drag_splitter, Some(Splitter::StashesHorizontal));

        let drag_stashes_h = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 40,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, drag_stashes_h);
        assert_eq!(app.stashes_horizontal_split_pct, 40);

        let up_stashes_h = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 40,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, up_stashes_h);

        // Click stashes vertical splitter
        let down_stashes_v = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 24,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, down_stashes_v);
        assert_eq!(app.active_drag_splitter, Some(Splitter::StashesVertical));

        let drag_stashes_v = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 10,
            row: 30,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, drag_stashes_v);
        assert_eq!(app.stashes_vertical_split_pct, 60);

        let up_stashes_v = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 10,
            row: 30,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, up_stashes_v);

        // Test Overview splitter dragging
        app.detail_areas = DetailAreas::default();
        app.detail_areas.tab_bar = Some(Rect::new(0, 0, 100, 2));
        app.detail_areas.overview_horizontal_splitter = Some(Rect::new(49, 2, 2, 48));

        let down_overview = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 49,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, down_overview);
        assert_eq!(app.active_drag_splitter, Some(Splitter::OverviewHorizontal));

        let drag_overview = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 30,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, drag_overview);
        assert_eq!(app.overview_horizontal_split_pct, 30);

        let up_overview = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 30,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, up_overview);
        assert_eq!(app.active_drag_splitter, None);
    }

    #[test]
    fn test_settings_mode_navigation_and_editing() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
        };
        let temp_path = std::env::temp_dir().join("twig_test_config_settings.toml");
        let _guard = TestFileGuard {
            path: temp_path.clone(),
        };
        let mut app = App::new(config, temp_path);

        assert_eq!(app.mode, Mode::Normal);

        // Press 's' to enter settings
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('s')), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::Settings);
        assert_eq!(app.settings_selected_index, 0);
        assert!(!app.settings_editing);

        // Select poll interval, press enter to edit
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(handled);
        assert!(app.settings_editing);
        assert_eq!(app.input_buffer, "100");

        // Backspace once and append '5' to make it '105'
        crate::input::handle_key(&mut app, key_event(KeyCode::Backspace), 10);
        crate::input::handle_key(&mut app, key_event(KeyCode::Char('5')), 10);
        assert_eq!(app.input_buffer, "105");

        // Commit change
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(!app.settings_editing);
        assert_eq!(app.config.poll_interval_ms, 105);

        // Go down to "Sort By"
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
        assert_eq!(app.settings_selected_index, 1);

        // Toggle Sort By (Custom -> Alphabetical)
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert_eq!(app.config.sort_by, SortOrder::Alphabetical);

        // Go down to "Sort Reverse"
        crate::input::handle_key(&mut app, key_event(KeyCode::Char('j')), 10);
        assert_eq!(app.settings_selected_index, 2);

        // Toggle Sort Reverse (false -> true)
        crate::input::handle_key(&mut app, key_event(KeyCode::Char(' ')), 10);
        assert!(app.config.sort_reverse);

        // Go down to Theme (index 3)
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
        assert_eq!(app.settings_selected_index, 3);

        // Edit Theme Name dropdown
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(app.settings_editing);
        assert!(app.settings_theme_list.contains(&"default".to_string()));

        // Pressing Down increases index (if there are other themes available)
        let prev_idx = app.settings_theme_index;
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
        if app.settings_theme_list.len() > 1 {
            assert_eq!(app.settings_theme_index, prev_idx + 1);
        }

        // Cancel theme edit
        crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
        assert!(!app.settings_editing);

        // Go down to FZF Max Depth (index 4)
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
        assert_eq!(app.settings_selected_index, 4);

        // Edit FZF Max Depth
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(app.settings_editing);
        app.input_buffer = "3".to_string();
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(!app.settings_editing);
        assert_eq!(app.config.fzf.max_depth, 3);

        // Go down to FZF Start Dir (index 5)
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
        assert_eq!(app.settings_selected_index, 5);

        // Edit FZF Start Dir
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(app.settings_editing);
        app.input_buffer = "/some/path".to_string();
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(!app.settings_editing);
        assert_eq!(app.config.fzf.start_dir, "/some/path");

        // Press Esc to exit settings
        crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_workspace_tab_right_arrow_inspect() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
        };
        let temp_path = std::env::temp_dir().join("twig_test_config_inspect.toml");
        let _guard = TestFileGuard {
            path: temp_path.clone(),
        };
        let mut app = App::new(config, temp_path);

        // Open details view
        app.mode = Mode::Detail;
        app.detail_tab = 0;
        app.detail_focus = DetailSection::Staged;

        let mut changes = crate::repo::WorktreeChanges::default();
        changes.staged.push(crate::repo::FileEntry {
            path: "dummy.txt".to_string(),
            label: "M",
        });
        let info = crate::repo::RepoInfo {
            branch: Some("main".to_string()),
            head: None,
            upstream: None,
            summary: crate::repo::RepoSummary::default(),
            changes,
            commits: vec![],
            graph_lines: vec![],
            local_branches: vec![],
            remote_branches: vec![],
            remotes: vec![],
            local_tags: vec![],
            remote_tags: vec![],
            remote_tags_loaded: false,
            files: vec![],
            stashes: vec![],
            committer_stats: vec![],
            committer_stats_limit_reached: false,
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: PathBuf::from("."),
            info: Box::new(info),
        });
        app.commit_selection = 0;

        // Verify we are not in Inspect mode
        assert_ne!(app.mode, Mode::Inspect);

        // Press Right arrow on Staged files list
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Right), 10);
        assert!(handled);

        // Verify we transitioned to Inspect mode and focused StagingDetails
        assert_eq!(app.mode, Mode::Inspect);
        assert_eq!(app.detail_focus, DetailSection::StagingDetails);

        // Press Left arrow in Inspect mode on StagingDetails
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Left), 10);
        assert!(handled);

        // Verify we are still in Inspect mode, but focus returned to Staged files list
        assert_eq!(app.mode, Mode::Inspect);
        assert_eq!(app.detail_focus, DetailSection::Staged);
    }

    #[test]
    fn test_commit_enter_key_inspect() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
        };
        let temp_path = std::env::temp_dir().join("twig_test_config_inspect_enter.toml");
        let _guard = TestFileGuard {
            path: temp_path.clone(),
        };
        let mut app = App::new(config, temp_path);

        // Open details view and focus Commits section
        app.mode = Mode::Detail;
        app.detail_tab = 0;
        app.detail_focus = DetailSection::Commits;

        let mut changes = crate::repo::WorktreeChanges::default();
        changes.staged.push(crate::repo::FileEntry {
            path: "dummy.txt".to_string(),
            label: "M",
        });
        let info = crate::repo::RepoInfo {
            branch: Some("main".to_string()),
            head: None,
            upstream: None,
            summary: crate::repo::RepoSummary::default(),
            changes,
            commits: vec![],
            graph_lines: vec![],
            local_branches: vec![],
            remote_branches: vec![],
            remotes: vec![],
            local_tags: vec![],
            remote_tags: vec![],
            remote_tags_loaded: false,
            files: vec![],
            stashes: vec![],
            committer_stats: vec![],
            committer_stats_limit_reached: false,
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: PathBuf::from("."),
            info: Box::new(info),
        });
        app.commit_selection = 0;

        // Verify we are not in Inspect mode
        assert_ne!(app.mode, Mode::Inspect);

        // Press Enter on Commits section
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(handled);

        // Verify we transitioned to Inspect mode and focused Staged files list
        assert_eq!(app.mode, Mode::Inspect);
        assert_eq!(app.detail_focus, DetailSection::Staged);
    }

    #[test]
    fn test_workspace_tab_focus_cycle_skips_empty_panels() {
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
        };
        let temp_path = std::env::temp_dir().join("twig_test_config_cycle.toml");
        let _guard = TestFileGuard {
            path: temp_path.clone(),
        };
        let mut app = App::new(config, temp_path);

        // 1. Uncommitted selected, Staged is not empty, Unstaged is empty
        app.mode = Mode::Detail;
        app.detail_tab = 0;
        app.detail_focus = DetailSection::Commits;

        let mut changes = crate::repo::WorktreeChanges::default();
        changes.staged.push(crate::repo::FileEntry {
            path: "staged_file.txt".to_string(),
            label: "M",
        });
        // Unstaged is empty
        let info = crate::repo::RepoInfo {
            branch: Some("main".to_string()),
            head: None,
            upstream: None,
            summary: crate::repo::RepoSummary::default(),
            changes,
            commits: vec![],
            graph_lines: vec![],
            local_branches: vec![],
            remote_branches: vec![],
            remotes: vec![],
            local_tags: vec![],
            remote_tags: vec![],
            remote_tags_loaded: false,
            files: vec![],
            stashes: vec![],
            committer_stats: vec![],
            committer_stats_limit_reached: false,
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: PathBuf::from("."),
            info: Box::new(info),
        });
        app.commit_selection = 0; // index 0 is "<uncommitted>"

        // We cycle from Commits -> Staged (since Staged is not empty)
        app.cycle_detail_focus(false);
        assert_eq!(app.detail_focus, DetailSection::Staged);

        // Cycle from Staged -> StagingDetails (skips empty Unstaged, skips CommitDetails because uncommitted is selected)
        app.cycle_detail_focus(false);
        assert_eq!(app.detail_focus, DetailSection::StagingDetails);

        // Cycle from StagingDetails -> Commits
        app.cycle_detail_focus(false);
        assert_eq!(app.detail_focus, DetailSection::Commits);

        // Cycle reverse: Commits -> StagingDetails
        app.cycle_detail_focus(true);
        assert_eq!(app.detail_focus, DetailSection::StagingDetails);

        // Cycle reverse: StagingDetails -> Staged
        app.cycle_detail_focus(true);
        assert_eq!(app.detail_focus, DetailSection::Staged);

        // Cycle reverse: Staged -> Commits
        app.cycle_detail_focus(true);
        assert_eq!(app.detail_focus, DetailSection::Commits);

        // 2. Regular commit selected (is_uncommitted_selected is false)
        // With a regular commit, staged & unstaged are empty.
        app.commit_selection = 1; // Not uncommitted

        let empty_info = crate::repo::RepoInfo {
            branch: Some("main".to_string()),
            head: None,
            upstream: None,
            summary: crate::repo::RepoSummary::default(),
            changes: crate::repo::WorktreeChanges::default(),
            commits: vec![],
            graph_lines: vec![],
            local_branches: vec![],
            remote_branches: vec![],
            remotes: vec![],
            local_tags: vec![],
            remote_tags: vec![],
            remote_tags_loaded: false,
            files: vec![],
            stashes: vec![],
            committer_stats: vec![],
            committer_stats_limit_reached: false,
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: PathBuf::from("."),
            info: Box::new(empty_info),
        });

        // We cycle from Commits -> CommitDetails (skips empty Staged and empty Unstaged)
        app.cycle_detail_focus(false);
        assert_eq!(app.detail_focus, DetailSection::CommitDetails);

        // Cycle from CommitDetails -> Commits (skips empty StagingDetails because staged & unstaged are empty)
        app.cycle_detail_focus(false);
        assert_eq!(app.detail_focus, DetailSection::Commits);

        // Cycle reverse: Commits -> CommitDetails
        app.cycle_detail_focus(true);
        assert_eq!(app.detail_focus, DetailSection::CommitDetails);

        // Cycle reverse: CommitDetails -> Commits
        app.cycle_detail_focus(true);
        assert_eq!(app.detail_focus, DetailSection::Commits);
    }

    #[test]
    fn test_lazygit_shortcut_triggers_pending() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
        };
        let temp_path = std::env::temp_dir().join("twig_test_config_lazygit.toml");
        let _guard = TestFileGuard {
            path: temp_path.clone(),
        };
        let mut app = App::new(config, temp_path);

        assert!(!app.pending_lazygit);

        // Pressing 'l' triggers pending_lazygit
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('l')), 10);
        assert!(handled);
        assert!(app.pending_lazygit);
    }

    #[test]
    fn test_files_fzf_shortcut_triggers_pending() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
        };
        let temp_path = std::env::temp_dir().join("twig_test_config_files_fzf.toml");
        let _guard = TestFileGuard {
            path: temp_path.clone(),
        };
        let mut app = App::new(config, temp_path);
        app.mode = Mode::Detail;
        app.detail_tab = 1; // Files tab

        assert!(!app.pending_files_fzf);

        // Pressing 'f' triggers pending_files_fzf when in files tab
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('f')), 10);
        assert!(handled);
        assert!(app.pending_files_fzf);
    }
}
