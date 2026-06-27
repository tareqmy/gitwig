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
use crate::repo::{self, ItemDetail, ItemStatus};
use crate::ui;
use crate::ui_detail::DetailAreas;

/// Height of each item row inside the bordered list area.
/// Borders (top + bottom) take 2 rows; the remaining 2 inner rows hold
/// the item path and the branch name respectively.
pub const ITEM_HEIGHT: u16 = 4;

/// What operation the remote picker was opened for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
    /// Typing a stash name/message to create.
    StashCreateInput,
    /// Picking a remote when multiple are available.
    RemotePicker,
    /// Typing a search query for commits.
    #[allow(dead_code)]
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
    /// Debug logs view.
    DebugLogs,
    /// Typing an import URL.
    ImportUrlInput,
    /// Typing an import destination path.
    ImportDestInput,
    /// Typing an import name.
    ImportNameInput,
    /// Typing a directory to bulk add its subdirectories.
    BulkAddInput,
    /// Choosing which columns to filter on.
    SearchColumnPicker,
    /// Typing a remote name to add.
    RemoteAddNameInput,
    /// Typing a remote URL to add.
    RemoteAddUrlInput,
    /// Confirming deletion of a remote.
    RemoteDeleteConfirm,
    /// logs UI with commits only.
    Logs,
    /// Search input in the logs UI.
    LogsSearchInput,
    /// Confirming checkout of a branch.
    BranchCheckoutConfirm,
    /// Confirming checkout of a tag.
    TagCheckoutConfirm,
    /// Search input for repositories on the home page.
    RepoSearchInput,
    /// Confirming aborting of a merge.
    MergeAbortConfirm,
    /// Confirming continuation of a merge.
    MergeContinueConfirm,
    /// Showing the about popup / creator profile.
    About,
    /// Confirming cherry-pick of a commit.
    CherryPickConfirm,
    /// Confirming revert of a commit.
    RevertConfirm,
}

/// Which panel in the detail view currently has keyboard focus.
/// Tab cycles through them in order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DetailSection {
    Commits,
    Staged,
    Unstaged,
    Conflicts,
    CommitDetails,
    StagingDetails,
    ConflictDiff,
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
    CommitPopupWidth,   // Dragging vertical border of commit popup
    CommitPopupHeight,  // Dragging horizontal border of commit popup
    CommitPopupBoth,    // Dragging corner of commit popup
}

impl DetailSection {
    /// Advance to the next section in the cycle.
    pub fn next(self) -> Self {
        match self {
            Self::Commits => Self::Staged,
            Self::Staged => Self::Unstaged,
            Self::Unstaged => Self::Conflicts,
            Self::Conflicts => Self::CommitDetails,
            Self::CommitDetails => Self::StagingDetails,
            Self::StagingDetails => Self::ConflictDiff,
            Self::ConflictDiff => Self::Commits,
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
            Self::Commits => Self::ConflictDiff,
            Self::Staged => Self::Commits,
            Self::Unstaged => Self::Staged,
            Self::Conflicts => Self::Unstaged,
            Self::CommitDetails => Self::Conflicts,
            Self::StagingDetails => Self::CommitDetails,
            Self::ConflictDiff => Self::StagingDetails,
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

#[derive(Debug, Clone)]
pub struct DetailCache {
    pub detail: repo::ItemDetail,
    pub loaded_at: std::time::Instant,
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
    pub error_message: Option<String>,
    pub current_detail: Option<ItemDetail>,
    /// Cache of repository detail views mapped by their path.
    pub detail_cache: std::collections::HashMap<String, DetailCache>,
    /// Which panel is focused inside the detail view.
    pub detail_focus: DetailSection,
    pub queue: crate::queue::Queue,
    pub file_tree: crate::components::file_tree::FileTreeComponent,
    pub branch_list: crate::components::branch_list::BranchListComponent,
    pub tag_list: crate::components::tag_list::TagListComponent,
    pub stash_list: crate::components::stash_list::StashListComponent,
    /// Selected row index inside the Commits panel (0 = top row).
    pub commit_list: crate::components::commit_list::CommitListComponent,
    pub commit_popup: crate::popups::commit::CommitPopup,
    /// Dynamic commit limit for pagination

    /// Active query for filtering commits in the commits panel

    /// Active query for filtering repositories in the home page list
    pub repo_search_query: Option<String>,
    /// Selected file index inside the Changed Files panel (real commits).

    /// Selected file index inside the Staged/Unstaged sub-panels (uncommitted view).

    /// Cached unified-diff lines for the currently selected file.
    pub diff: crate::components::diff::DiffComponent,
    /// Vertical scroll offset for the diff panel (StagingDetails focus).

    /// Selected hunk index for stage/unstage by hunk (StagingDetails focus).

    /// Whether we are selecting lines (true) or hunks (false) in StagingDetails.

    /// Selected line index in the file_diff.

    /// Selected conflict file index in Conflicts panel.

    /// Vertical scroll offset for the commit details panel (CommitDetails focus).

    /// Vertical scroll offset for the commit input popup.
    pub commit_input_scroll: usize,
    /// Selected local branch index in Branches tab.
    /// Selected remote branch index in Branches tab.
    /// Selected local tag index in Tags/Branches tabs.
    /// Selected remote tag index in Tags/Branches tabs.
    /// Selected remote index in Remotes tab.
    /// Selected stash index in Stashes tab.
    /// Selected file index in the Stashes tab stashed files list.
    /// Scroll offset for the help overlays.
    pub help_scroll: usize,
    /// Panel bounding boxes recorded after each draw, used for mouse hit-testing.
    pub detail_areas: DetailAreas,
    /// Main panel item bounding boxes recorded after each draw, used for mouse hit-testing.
    pub main_areas: Vec<Rect>,

    pub status_list: crate::components::status_list::StatusListComponent,



    /// Timestamp and selected index of the last mouse click for double-click detection.
    pub last_click: Option<(std::time::Instant, usize)>,
    /// Active tab in the detail view (0 = Details, 1 = Graph, 2 = Branches, 3 = Files).
    pub detail_tab: usize,
    /// Selected file index in the Files tab.
    /// Vertical scroll offset for the file content preview in Files tab.
    /// Set of expanded folder paths.
    /// Flattened visible files inside the Files tab.
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
    /// Whether external Git application launch is pending.
    pub pending_git_app: bool,
    /// Whether fzf search launch is pending.
    pub pending_fzf: bool,
    /// Whether bulk fzf search launch is pending.
    pub pending_bulk_fzf: bool,
    /// Whether fzf files search launch is pending.
    pub pending_files_fzf: bool,
    /// Whether interactive rebase is pending.
    pub pending_interactive_rebase: Option<(PathBuf, String)>,
    /// Whether we are currently viewing logs UI.
    pub in_logs_ui: bool,
    /// Whether we are in full-screen diff mode under inspect view.
    pub inspect_full_diff: bool,
    /// Whether the commit popup is maximized to leave 20 characters on all sides.
    pub commit_popup_maximized: bool,
    /// Selection in search column picker.
    pub search_column_selection: usize,
    /// Columns to include in search.
    pub search_columns_sha: bool,
    pub search_columns_message: bool,
    pub search_columns_author: bool,
    pub search_columns_date: bool,
    /// Target branch name and remote flag for deletion/creation actions.
    pub branch_action_target: Option<(String, bool)>,
    /// Target commit OID for tag creation.
    pub tag_action_target_oid: Option<String>,
    /// Target tag name and remote flag for deletion action.
    pub tag_delete_target: Option<(String, bool)>,
    /// Target tag name for checkout action.
    pub tag_checkout_target: Option<String>,
    /// Target tag name for push action.
    pub tag_push_target: Option<String>,
    /// Target file path and staged flag for discard/revert action.
    pub discard_target: Option<(String, bool)>,
    /// Target commit (hash, summary) for cherry-pick.
    pub cherry_pick_target: Option<(String, String)>,
    /// Selected destination branch index for the cherry-pick popup.
    pub cherry_pick_dest_selection: usize,
    /// List of local branch names available for cherry-pick destination.
    pub cherry_pick_dest_branches: Vec<String>,
    /// Target commit (hash, summary) for revert.
    pub revert_target: Option<(String, String)>,
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
    /// Percentage width of the commit message popup (default: 80).
    pub commit_popup_width_pct: u16,
    /// Percentage height of the commit message popup (default: 45).
    pub commit_popup_height_pct: u16,
    /// Active drag splitter if dragging is in progress.
    pub active_drag_splitter: Option<Splitter>,
    pub settings_selected_index: usize,
    pub settings_editing: bool,
    pub settings_theme_list: Vec<String>,
    pub settings_theme_index: usize,
    pub debug_log_scroll: usize,
    pub import_url: String,
    pub import_dest: String,
    pub import_name: String,
    pub remote_add_name: String,
    pub remote_add_url: String,
    pub remote_action_target: Option<String>,
    pub last_staging_focus: DetailSection,
    pub force_fzf_missing: Option<bool>,
    pub loading_repo_path: Option<String>,
    pub detail_tx: std::sync::mpsc::Sender<(String, repo::ItemDetail)>,
    pub detail_rx: std::sync::mpsc::Receiver<(String, repo::ItemDetail)>,
    pub tab_tx: std::sync::mpsc::Sender<(String, usize, repo::TabPayload)>,
    pub tab_rx: std::sync::mpsc::Receiver<(String, usize, repo::TabPayload)>,
    pub cpu_tracker: std::sync::Mutex<Option<(f64, std::time::Instant, f64, f64)>>,
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

enum LogsNavDirection {
    Up,
    Down,
    PageUp(usize),
    PageDown(usize),
}

impl App {

    pub fn drain_queue(&mut self) {
        while let Some(ev) = self.queue.pop() {
            match ev {
                
                crate::queue::InternalEvent::ClosePopup => self.mode = Mode::Detail, // simplified
                crate::queue::InternalEvent::Commit => {
                    self.commit_git_changes();
                }
                crate::queue::InternalEvent::SearchColumnPicker => {
                    self.search_column_selection = 0;
                    self.mode = Mode::SearchColumnPicker;
                }
                crate::queue::InternalEvent::StartCommit => self.start_commit(),
                crate::queue::InternalEvent::StartCommitAmend => self.start_commit_amend(),
                crate::queue::InternalEvent::StartTagCreate => self.start_tag_create(),
                crate::queue::InternalEvent::RunInteractiveRebase => self.run_interactive_rebase(),
                crate::queue::InternalEvent::RequestCherryPick => self.request_cherry_pick(),
                crate::queue::InternalEvent::YankSelectedCommitHash => self.yank_selected_commit_hash(),
                crate::queue::InternalEvent::RequestRevert => self.request_revert(),
                crate::queue::InternalEvent::InspectCommit => {
                    self.mode = Mode::Inspect;
                    if self.is_uncommitted_selected() {
                        self.detail_focus = DetailSection::Staged;
                        self.last_staging_focus = DetailSection::Staged;
                        self.status_list.staging_file_selection = 0;
                    } else {
                        self.detail_focus = DetailSection::Staged;
                        self.last_staging_focus = DetailSection::Staged;
                        self.status_list.file_selection = 0;
                    }
                    self.diff.diff_scroll = 0;
                    self.refresh_file_diff();
                }
                crate::queue::InternalEvent::CommitSelectionUp => self.detail_commit_up(),
                crate::queue::InternalEvent::CommitSelectionDown => self.detail_commit_down(),
                crate::queue::InternalEvent::CommitSelectionPageUp => {
                    let page = self.config.page_size;
                    self.detail_commit_page_up(page);
                }
                
                crate::queue::InternalEvent::CommitSelectionTop => self.detail_commit_to_top(),
                crate::queue::InternalEvent::CommitSelectionBottom => self.detail_commit_to_bottom(),
                crate::queue::InternalEvent::LoadMoreCommits => {
                    self.commit_list.limit = self.commit_list.limit.saturating_add(200);
                    self.resync_detail();
                    self.status_message = Some("Loading more commits...".to_string());
                }
                crate::queue::InternalEvent::CommitDetailsUp => self.commit_list.details_scroll_up(),
                crate::queue::InternalEvent::CommitDetailsDown => self.commit_list.details_scroll_down(),
                crate::queue::InternalEvent::StagingFileUp => if self.is_uncommitted_selected() { self.staging_file_up() } else { self.detail_file_up() },
                crate::queue::InternalEvent::StagingFileDown => if self.is_uncommitted_selected() { self.staging_file_down() } else { self.detail_file_down() },
                crate::queue::InternalEvent::ConflictFileUp => self.conflict_file_up(),
                crate::queue::InternalEvent::ConflictFileDown => self.conflict_file_down(),
                crate::queue::InternalEvent::StageSelectedFile => self.stage_selected_file(),
                crate::queue::InternalEvent::UnstageSelectedFile => self.unstage_selected_file(),
                crate::queue::InternalEvent::ResolveConflictOurs => self.resolve_conflict_ours(),
                crate::queue::InternalEvent::ResolveConflictTheirs => self.resolve_conflict_theirs(),
                crate::queue::InternalEvent::MarkConflictResolved => self.mark_conflict_resolved(),
                crate::queue::InternalEvent::MergeAbortConfirm => self.mode = Mode::MergeAbortConfirm,
                crate::queue::InternalEvent::MergeContinueConfirm => self.mode = Mode::MergeContinueConfirm,
                crate::queue::InternalEvent::StageSelectedHunk => self.stage_selected_hunk(),
                crate::queue::InternalEvent::UnstageSelectedHunk => self.unstage_selected_hunk(),
                crate::queue::InternalEvent::StageAllChanges => self.stage_all_changes(),
                crate::queue::InternalEvent::UnstageAllChanges => self.unstage_all_changes(),
                crate::queue::InternalEvent::RequestDiscardChanges => self.request_discard_changes(),
                crate::queue::InternalEvent::RequestDiscardAllChanges => self.request_discard_all_changes(),
                crate::queue::InternalEvent::StartStashCreate => self.start_stash_create(),
                crate::queue::InternalEvent::DiffScrollUp => self.diff.diff_scroll_up(),
                crate::queue::InternalEvent::DiffScrollDown => self.diff.diff_scroll_down(),
                crate::queue::InternalEvent::DiffScrollPageUp => { let page = self.config.page_size; self.diff.diff_scroll_page_up(page); },
                crate::queue::InternalEvent::DiffScrollPageDown => { let page = self.config.page_size; self.diff.diff_scroll_page_down(page); },
                crate::queue::InternalEvent::DiffScrollTop => self.diff.diff_scroll_to_top(),
                crate::queue::InternalEvent::DiffScrollBottom => self.diff.diff_scroll_to_bottom(),

                // FileTree
                crate::queue::InternalEvent::FileTreeUp => self.file_list_up(),
                crate::queue::InternalEvent::FileTreeDown => self.file_list_down(),
                crate::queue::InternalEvent::FileTreePageUp => { let p = self.config.page_size; self.file_list_page_up(p) },
                crate::queue::InternalEvent::FileTreePageDown => { let p = self.config.page_size; self.file_list_page_down(p) },
                crate::queue::InternalEvent::FileTreeTop => self.file_list_to_top(),
                crate::queue::InternalEvent::FileTreeBottom => self.file_list_to_bottom(),
                crate::queue::InternalEvent::FileContentUp => self.file_tree.file_content_scroll_up(),
                crate::queue::InternalEvent::FileContentDown => self.file_tree.file_content_scroll_down(),
                crate::queue::InternalEvent::FileContentPageUp => { let p = self.config.page_size; self.file_tree.file_content_scroll_page_up(p) },
                crate::queue::InternalEvent::FileContentPageDown => { let p = self.config.page_size; self.file_tree.file_content_scroll_page_down(p) },
                crate::queue::InternalEvent::FileContentTop => self.file_tree.file_content_scroll_to_top(),
                crate::queue::InternalEvent::FileContentBottom => self.file_tree.file_content_scroll_to_bottom(),
                crate::queue::InternalEvent::ToggleFolderExpanded => self.toggle_folder_expanded(),
                crate::queue::InternalEvent::CollapseAllFolders => self.collapse_all_folders(),
                crate::queue::InternalEvent::RequestDiscardFile => self.request_discard_changes(),

                // BranchList
                crate::queue::InternalEvent::LocalBranchUp => self.local_branch_up(),
                crate::queue::InternalEvent::LocalBranchDown => self.local_branch_down(),
                crate::queue::InternalEvent::LocalBranchPageUp => { let p = self.config.page_size; self.local_branch_page_up(p) },
                crate::queue::InternalEvent::LocalBranchPageDown => { let p = self.config.page_size; self.local_branch_page_down(p) },
                crate::queue::InternalEvent::LocalBranchTop => self.local_branch_to_top(),
                crate::queue::InternalEvent::LocalBranchBottom => self.local_branch_to_bottom(),
                crate::queue::InternalEvent::RemoteBranchUp => self.remote_branch_up(),
                crate::queue::InternalEvent::RemoteBranchDown => self.remote_branch_down(),
                crate::queue::InternalEvent::RemoteBranchPageUp => { let p = self.config.page_size; self.remote_branch_page_up(p) },
                crate::queue::InternalEvent::RemoteBranchPageDown => { let p = self.config.page_size; self.remote_branch_page_down(p) },
                crate::queue::InternalEvent::RemoteBranchTop => self.remote_branch_to_top(),
                crate::queue::InternalEvent::RemoteBranchBottom => self.remote_branch_to_bottom(),
                crate::queue::InternalEvent::CheckoutBranch => self.request_branch_checkout(),
                crate::queue::InternalEvent::RequestDeleteBranch => self.request_branch_delete(),
                crate::queue::InternalEvent::StartBranchCreate => self.start_branch_create(),
                crate::queue::InternalEvent::StartBranchMerge => self.request_branch_merge(),
                crate::queue::InternalEvent::StartBranchRebase => self.request_branch_rebase(),
                crate::queue::InternalEvent::RequestBranchPush => self.request_branch_push(),
                crate::queue::InternalEvent::FetchRemote => {
                    let remote_name = if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
                        info.remotes.get(self.branch_list.remote_selection).or_else(|| info.remotes.first()).map(|r| r.name.clone())
                    } else { None };
                    if let Some(name) = remote_name { self.fetch_remote(&name); }
                },
                crate::queue::InternalEvent::StartRemoteAdd => self.start_remote_add(),
                crate::queue::InternalEvent::RequestDeleteRemote => self.request_remote_delete(),

                // TagList
                crate::queue::InternalEvent::TagUp => self.local_tag_up(),
                crate::queue::InternalEvent::TagDown => self.local_tag_down(),
                crate::queue::InternalEvent::TagPageUp => { let p = self.config.page_size; self.local_tag_page_up(p) },
                crate::queue::InternalEvent::TagPageDown => { let p = self.config.page_size; self.local_tag_page_down(p) },
                crate::queue::InternalEvent::TagTop => self.local_tag_to_top(),
                crate::queue::InternalEvent::TagBottom => self.local_tag_to_bottom(),
                crate::queue::InternalEvent::CheckoutTag => self.request_tag_checkout(),
                crate::queue::InternalEvent::RequestDeleteTag => self.request_tag_delete(),
                crate::queue::InternalEvent::RequestPushTag => self.request_tag_push(),
                crate::queue::InternalEvent::RequestPushAllTags => self.request_tag_push_all(),
                crate::queue::InternalEvent::FetchRemoteTags => self.fetch_remote_tags(true),

                // StashList
                crate::queue::InternalEvent::StashUp => self.stash_up(),
                crate::queue::InternalEvent::StashDown => self.stash_down(),
                crate::queue::InternalEvent::StashPageUp => { let p = self.config.page_size; self.stash_page_up(p) },
                crate::queue::InternalEvent::StashPageDown => { let p = self.config.page_size; self.stash_page_down(p) },
                crate::queue::InternalEvent::StashTop => self.stash_to_top(),
                crate::queue::InternalEvent::StashBottom => self.stash_to_bottom(),
                crate::queue::InternalEvent::StashFileUp => self.stash_file_up(),
                crate::queue::InternalEvent::StashFileDown => self.stash_file_down(),
                crate::queue::InternalEvent::StashFilePageUp => { let p = self.config.page_size; self.stash_file_page_up(p) },
                crate::queue::InternalEvent::StashFilePageDown => { let p = self.config.page_size; self.stash_file_page_down(p) },
                crate::queue::InternalEvent::StashFileTop => self.stash_file_to_top(),
                crate::queue::InternalEvent::StashFileBottom => self.stash_file_to_bottom(),
                crate::queue::InternalEvent::RequestDeleteStash => self.request_stash_delete(),
                crate::queue::InternalEvent::RequestApplyStash => self.request_stash_apply(),


                crate::queue::InternalEvent::CommitSelectionPageDown => {
                    let page = self.config.page_size;
                    self.detail_commit_page_down(page);
                }
                _ => {}
            }
        }
    }

    pub fn sym(&self, key: &str) -> &'static str {
        self.config.sym(key)
    }

    pub fn new(config: Config, config_path: PathBuf) -> Self {
        crate::debug_log::info("Initializing Gitwig application state");
        crate::ui::update_theme(&config.theme);
        let original_items = config.items.clone();
        let statuses = config.items.iter().map(|s| repo::inspect_summary(s)).collect();
        let (tx, rx) = std::sync::mpsc::channel();
        let (detail_tx, detail_rx) = std::sync::mpsc::channel();
        let (tab_tx, tab_rx) = std::sync::mpsc::channel();
        let queue = crate::queue::Queue::default();
        let mut app = Self {
            queue: queue.clone(),
            original_items,
            config,
            config_path,
            statuses,
            selected_index: 0,
            scroll_top: 0,
            mode: Mode::Normal,
            input_buffer: String::new(),
            status_message: None,
            error_message: None,
            current_detail: None,
            detail_cache: std::collections::HashMap::new(),
            detail_focus: DetailSection::Commits,
            file_tree: crate::components::file_tree::FileTreeComponent::new(queue.clone()),
            branch_list: crate::components::branch_list::BranchListComponent::new(queue.clone()),
            tag_list: crate::components::tag_list::TagListComponent::new(queue.clone()),
            stash_list: crate::components::stash_list::StashListComponent::new(queue.clone()),
            commit_list: crate::components::commit_list::CommitListComponent { limit: 100, queue: queue.clone(), ..Default::default() },
            commit_popup: crate::popups::commit::CommitPopup::new(queue.clone()),
            
            
            repo_search_query: None,
            
            
            diff: crate::components::diff::DiffComponent::new(queue.clone()),
            
            
            
            
            
            
            commit_input_scroll: 0,
            help_scroll: 0,
            detail_areas: DetailAreas::default(),
            main_areas: Vec::new(),
            
            status_list: crate::components::status_list::StatusListComponent::new(queue.clone()),
            
            
            last_click: None,
            detail_tab: 0,
            graph_scroll: 0,
            commit_editing: false,
            status_expanded: false,
            tx,
            rx,
            fetching: false,
            pending_git_app: false,
            pending_fzf: false,
            pending_bulk_fzf: false,
            pending_files_fzf: false,
            pending_interactive_rebase: None,
            in_logs_ui: false,
            inspect_full_diff: false,
            commit_popup_maximized: false,
            search_column_selection: 0,
            search_columns_sha: true,
            search_columns_message: true,
            search_columns_author: true,
            search_columns_date: true,
            branch_action_target: None,
            tag_action_target_oid: None,
            tag_delete_target: None,
            tag_checkout_target: None,
            tag_push_target: None,
            discard_target: None,
            cherry_pick_target: None,
            cherry_pick_dest_selection: 0,
            cherry_pick_dest_branches: Vec::new(),
            revert_target: None,
            fetch_progress: 0,
            stash_apply_delete_after: true,
            commit_amend: false,
            remote_picker_action: None,
            remote_picker_selection: 0,
            inspect_horizontal_split_pct: 38,
            inspect_vertical_split_pct: 38,
            workspace_main_split_pct: 38,
            files_horizontal_split_pct: 38,
            branches_horizontal_split_pct: 50,
            stashes_horizontal_split_pct: 38,
            stashes_vertical_split_pct: 38,
            overview_horizontal_split_pct: 38,
            commit_popup_width_pct: 80,
            commit_popup_height_pct: 45,
            active_drag_splitter: None,
            settings_selected_index: 0,
            settings_editing: false,
            settings_theme_list: Vec::new(),
            settings_theme_index: 0,
            debug_log_scroll: 0,
            import_url: String::new(),
            import_dest: String::new(),
            import_name: String::new(),
            remote_add_name: String::new(),
            remote_add_url: String::new(),
            remote_action_target: None,
            last_staging_focus: DetailSection::Staged,
            force_fzf_missing: None,
            loading_repo_path: None,
            detail_tx,
            detail_rx,
            tab_tx,
            tab_rx,
            cpu_tracker: std::sync::Mutex::new(None),
        };

        if app.config.sort_by != SortOrder::Custom {
            app.sort_items_in_place();
        }

        // Detect update / initial setup
        let current_version = env!("CARGO_PKG_VERSION");
        let version_path = app.config_path.parent().unwrap_or(&app.config_path).join(".version");
        let mut is_first_run = false;

        let last_version = if version_path.exists() {
            std::fs::read_to_string(&version_path).map(|s| s.trim().to_string()).unwrap_or_default()
        } else {
            is_first_run = true;
            String::new()
        };

        if last_version != current_version {
            // 1. Back up config if it is an update and config exists
            if !is_first_run && app.config_path.exists() {
                let backup_path = app.config_path.with_extension("toml.bak");
                let _ = std::fs::copy(&app.config_path, backup_path);
                crate::debug_log::info(format!(
                    "Backed up configuration to {:?}",
                    app.config_path.with_extension("toml.bak")
                ));
            }

            // 2. Perform updates or auto-detections
            let gitui_installed = is_tool_installed("gitui");
            let lazygit_installed = is_tool_installed("lazygit");
            let fzf_installed = is_tool_installed("fzf");

            if app.config.git_app == "gitui" && !gitui_installed && lazygit_installed {
                app.config.git_app = "lazygit".to_string();
                crate::debug_log::info("Auto-configured git_app to lazygit as gitui was not found");
            }

            if app.config.fzf.enabled && !fzf_installed {
                crate::debug_log::warn(
                    "fzf is enabled in configuration but not found in your system PATH.",
                );
            }

            // 3. Write new version file
            let _ = std::fs::write(&version_path, current_version);

            // 4. Save config to persist migration changes
            let _ = crate::config::save_config(&app.config, &app.config_path);

            // 5. Update UI status message to inform user
            if is_first_run {
                app.status_message = Some(format!("Welcome to Gitwig v{}!", current_version));
            } else {
                app.status_message = Some(format!(
                    "Gitwig updated to v{}! Configuration verified and backed up.",
                    current_version
                ));
            }
        }

        app
    }

    pub fn set_error(&mut self, msg: String) {
        crate::debug_log::error(&msg);
        self.error_message = Some(msg);
    }

    pub fn status_height(&self) -> u16 {
        if self.status_expanded { 3 } else { 1 }
    }

    pub fn toggle_status_expanded(&mut self) {
        self.status_expanded = !self.status_expanded;
    }

    pub fn get_filtered_items(&self) -> Vec<(usize, &String)> {
        if let Some(ref query) = self.repo_search_query {
            let query_lower = query.to_lowercase();
            self.config
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    let file_name = std::path::Path::new(item)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(item.as_str())
                        .to_lowercase();
                    let full_path = item.to_lowercase();
                    file_name.contains(&query_lower) || full_path.contains(&query_lower)
                })
                .collect()
        } else {
            self.config.items.iter().enumerate().collect()
        }
    }

    pub fn get_items_len(&self) -> usize {
        if self.repo_search_query.is_some() {
            self.get_filtered_items().len()
        } else {
            self.config.items.len()
        }
    }

    pub fn get_selected_item(&self) -> Option<&String> {
        let orig_idx = self.get_selected_item_index()?;
        self.config.items.get(orig_idx)
    }

    pub fn get_selected_item_index(&self) -> Option<usize> {
        self.get_filtered_items().get(self.selected_index).map(|(orig_idx, _)| *orig_idx)
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

    pub fn is_fzf_installed(&self) -> bool {
        if let Some(forced) = self.force_fzf_missing {
            return !forced;
        }
        std::process::Command::new("fzf")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    }

    pub fn start_add(&mut self) {
        crate::debug_log::info("Initiating repository add");
        if !self.config.fzf.enabled {
            self.mode = Mode::Adding;
            self.commit_popup.input_buffer.clear();
        } else if !self.is_fzf_installed() {
            self.mode = Mode::Adding;
            self.commit_popup.input_buffer.clear();
            self.status_message =
                Some("fzf is not installed. Falling back to manual add.".to_string());
        } else {
            self.pending_fzf = true;
        }
    }

    pub fn start_edit(&mut self) {
        if let Some(current) = self.get_selected_item() {
            crate::debug_log::info(format!("Editing repository entry: {}", current));
            self.input_buffer = current.clone();
            self.mode = Mode::Editing;
        }
    }

    pub fn request_delete(&mut self) {
        if self.get_items_len() > 0 {
            self.mode = Mode::ConfirmDelete;
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
                z.sort_by(|a, b| a.0.cmp(&b.0));
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
                let cached = self.detail_cache.get(&item).unwrap().clone();
                let cached_commits_count = match &cached.detail {
                    repo::ItemDetail::Repo { info, .. } => info.commits.len(),
                    _ => 200,
                };
                self.commit_list.limit = cached_commits_count.max(200);
                self.current_detail = Some(cached.detail);
                self.rebuild_visible_files();

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
                self.commit_list.limit = 200;
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
            self.detail_tab = 0;
            self.graph_scroll = 0;
            self.inspect_full_diff = false;
            self.commit_popup.maximized = false;
            self.mode = Mode::Detail;
        }
    }

    /// Resync the selected item's filesystem/git state inside the Detail view,
    /// clamping selection indices to their new totals.
    /// Resync the selected item's filesystem/git state inside the Detail view asynchronously.
    pub fn resync_detail(&mut self) {
        if let Some(item) = self.get_selected_item().cloned() {
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
                info.graph_lines = repo::TabData::NotLoaded;
                info.committer_stats = repo::TabData::NotLoaded;
                info.tab_loaded_at = [None; 8];
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
                    if new_info.committer_stats.is_not_loaded() {
                        new_info.committer_stats = old_info.committer_stats.clone();
                        new_info.committer_stats_limit_reached =
                            old_info.committer_stats_limit_reached;
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

            let commit_files_len =
                info.commits.get(self.commit_list.selection).map(|c| c.files.len()).unwrap_or(0);

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
        let commit_limit = self.config.max_commits;
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
                let is_not_loaded = info.committer_stats.is_not_loaded();
                if should_trigger(info, tab_idx, is_not_loaded) {
                    info.tab_loading[tab_idx] = true;
                    if is_not_loaded {
                        info.committer_stats = repo::TabData::Loading;
                    }
                    std::thread::spawn(move || {
                        let res = repo::load_tab_overview(&path, commit_limit);
                        let _ = tx.send((
                            path.to_string_lossy().to_string(),
                            tab_idx,
                            repo::TabPayload::Overview(res),
                        ));
                    });
                }
            }
            _ => {}
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
        self.branch_list.local_branch_selection = self.branch_list.local_branch_selection.saturating_sub(1);
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
        self.branch_list.local_branch_selection = self.branch_list.local_branch_selection.saturating_sub(page);
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
        self.branch_list.remote_branch_selection = self.branch_list.remote_branch_selection.saturating_sub(1);
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
        self.branch_list.remote_branch_selection = self.branch_list.remote_branch_selection.saturating_sub(page);
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
        self.file_tree.file_list_selection = self.file_tree.file_list_selection.saturating_sub(page);
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

    /// Spawns a background thread to fetch the remote of the selected local branch.
    pub fn fetch_selected_branch(&mut self) {
        if self.fetching {
            return;
        }
        if let Some(repo::ItemDetail::Repo { resolved, info }) = &self.current_detail {
            if let Some(branch_info) = info.local_branches.get(self.branch_list.local_branch_selection) {
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
                            .env("GIT_TERMINAL_PROMPT", "0")
                            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
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
            if let Some(branch_info) = info.local_branches.get(self.branch_list.local_branch_selection) {
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
                            .env("GIT_TERMINAL_PROMPT", "0")
                            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
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
            if let Some(branch_info) = info.local_branches.get(self.branch_list.local_branch_selection) {
                let branch_name = branch_info.name.clone();
                // Check if this branch already has a configured upstream remote.
                let has_upstream = git2::Repository::open(resolved)
                    .ok()
                    .and_then(|repo| {
                        repo.find_branch(&branch_name, git2::BranchType::Local).ok().and_then(|b| {
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
                    cmd.env("GIT_TERMINAL_PROMPT", "0")
                        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new");
                    cmd.arg("push");
                    if set_upstream {
                        cmd.arg("-u");
                    }
                    cmd.arg(&remote_name).arg(&branch_name).current_dir(&repo_path);

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
                    if let Some(branch_info) = info.local_branches.get(self.branch_list.local_branch_selection)
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
            let commit_idx =
                if dirty { self.commit_list.selection.saturating_sub(1) } else { self.commit_list.selection };
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
        if stash_name.is_empty() {
            self.status_message = Some("Stash name cannot be empty".to_string());
            self.mode = Mode::Detail;
            return;
        }
        if let Some(repo::ItemDetail::Repo { resolved, .. }) = &self.current_detail {
            match repo::save_stash(resolved, &stash_name) {
                Ok(()) => {
                    self.status_message = Some(format!("Created stash '{}'", stash_name));
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
                    let mut cmd = std::process::Command::new("git");
                    cmd.env("GIT_TERMINAL_PROMPT", "0")
                        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new");
                    cmd.arg("push").arg(&remote_name).arg(&tag_name).current_dir(&repo_path);

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
                    if let Some(branch_info) = info.local_branches.get(self.branch_list.local_branch_selection)
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
                    if let Some(branch_info) = info.local_branches.get(self.branch_list.local_branch_selection)
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
                        let output = std::process::Command::new("git")
                            .env("GIT_TERMINAL_PROMPT", "0")
                            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
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
                if let Some(branch_info) = info.local_branches.get(self.branch_list.local_branch_selection) {
                    if !branch_info.is_head {
                        self.branch_action_target = Some((branch_info.name.clone(), false));
                        self.mode = Mode::BranchRebaseConfirm;
                    }
                }
            } else if self.detail_focus == DetailSection::RemoteBranches {
                if let Some(branch_info) = info.remote_branches.get(self.branch_list.remote_branch_selection) {
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
                        let output = std::process::Command::new("git")
                            .env("GIT_TERMINAL_PROMPT", "0")
                            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
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
                if let Some(branch_info) = info.local_branches.get(self.branch_list.local_branch_selection) {
                    if !branch_info.is_head {
                        self.branch_action_target = Some((branch_info.name.clone(), false));
                        self.mode = Mode::BranchInteractiveRebaseConfirm;
                    }
                }
            } else if self.detail_focus == DetailSection::RemoteBranches {
                if let Some(branch_info) = info.remote_branches.get(self.branch_list.remote_branch_selection) {
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

            let target = if is_root { "--root".to_string() } else { format!("{}~1", commit_oid) };
            self.pending_interactive_rebase = Some((repo_path, target));
        }
    }

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
                    Some(*matching_indices.last().unwrap())
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
                    Some(*matching_indices.last().unwrap())
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
                    Some(*matching_indices.last().unwrap())
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
        self.status_list.staging_file_selection = self.status_list.staging_file_selection.saturating_sub(1);
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
        self.status_list.conflict_file_selection = self.status_list.conflict_file_selection.saturating_sub(1);
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
            if let Some(selected_item) = self.file_tree.visible_files.get(self.file_tree.file_list_selection) {
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
        self.file_tree.file_content_scroll = self.file_tree.file_content_scroll.saturating_sub(page);
    }

    /// Scroll the file content panel down by `page` lines.
    pub fn file_content_scroll_page_down(&mut self, page: usize) {
        let max = self.get_file_content_line_count().saturating_sub(1);
        self.file_tree.file_content_scroll = (self.file_tree.file_content_scroll + page).min(max);
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

    pub fn has_uncommitted_changes(&self) -> bool {
        !self.is_staged_empty() || !self.is_unstaged_empty() || !self.is_conflicted_empty()
    }

    pub fn is_selected_commit_empty(&self) -> bool {
        self.get_selected_commit().map(|c| c.files.is_empty()).unwrap_or(true)
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
                self.diff.file_diff = repo::get_commit_file_diff(&repo_path, &commit_oid, &file_path);
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
                    self.diff.file_diff = repo::get_worktree_file_diff(&repo_path, &file_path, staged);
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

    pub fn refresh_detail(&mut self) {
        self.resync_detail();
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
                repo::resolve_conflict_hunk(&repo_path, &file_path, self.diff.diff_hunk_selection, true)
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
                repo::resolve_conflict_hunk(&repo_path, &file_path, self.diff.diff_hunk_selection, false)
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

    fn clamp_conflict_selection(&mut self) {
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

    pub fn cancel_commit_search(&mut self) {
        self.commit_list.search_query = None;
        self.clamp_commit_selection();
        self.status_list.file_selection = 0;
        self.diff.diff_scroll = 0;
        self.refresh_file_diff();
        self.mode = Mode::Detail;
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
                    self.input_buffer = last_msg;
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

    /// Cancels commit input and returns to the detail view.
    pub fn cancel_commit(&mut self) {
        self.commit_popup.input_buffer.clear();
        self.commit_input_scroll = 0;
        self.commit_popup.maximized = false;
        self.mode = Mode::Detail;
    }

    /// Transitions from editing the message to confirming the commit.
    pub fn commit_done_editing(&mut self) {
        self.commit_popup.editing = false;
    }

    /// Transitions back to editing the message from confirm state.
    pub fn commit_start_editing(&mut self) {
        self.commit_popup.editing = true;
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
                self.input_buffer = self.config.fzf.max_depth.to_string();
            }
            5 => {
                self.settings_editing = true;
                self.input_buffer = self.config.fzf.start_dir.clone();
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
                self.input_buffer = self.config.fzf.excludes.join(",");
            }
            9 => {
                self.settings_editing = true;
                self.input_buffer = self.config.git_app.clone();
            }
            10 => {
                self.config.fzf.git_only = !self.config.fzf.git_only;
                self.persist("FZF Git Only updated");
            }
            11 => {
                self.config.fzf.enabled = !self.config.fzf.enabled;
                self.persist("Use FZF updated");
            }
            12 => {
                self.config.compatibility_mode = !self.config.compatibility_mode;
                self.persist("Compatibility Mode updated");
            }
            13 => {
                self.config.resync_on_tab_change = !self.config.resync_on_tab_change;
                self.persist("Resync on Tab Change updated");
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
                        self.commit_popup.input_buffer.clear();
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
                    self.config.fzf.max_depth = val;
                    self.persist("FZF max depth updated");
                    self.settings_editing = false;
                    self.commit_popup.input_buffer.clear();
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            5 => {
                self.config.fzf.start_dir = trimmed.to_string();
                self.persist("FZF start directory updated");
                self.settings_editing = false;
                self.commit_popup.input_buffer.clear();
            }
            6 => {
                if let Ok(val) = trimmed.parse::<usize>() {
                    self.config.max_commits = val;
                    self.persist("Max commits updated");
                    self.settings_editing = false;
                    self.commit_popup.input_buffer.clear();
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
                        self.commit_popup.input_buffer.clear();
                    } else {
                        self.status_message = Some("Page size must be at least 1".to_string());
                    }
                } else {
                    self.status_message = Some("Invalid integer".to_string());
                }
            }
            8 => {
                self.config.fzf.excludes = trimmed
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                self.persist("FZF exclude folders updated");
                self.settings_editing = false;
                self.commit_popup.input_buffer.clear();
            }
            9 => {
                let trimmed_app = trimmed.to_string();
                if !trimmed_app.is_empty() {
                    self.config.git_app = trimmed_app;
                    self.persist("Preferred Git Client updated");
                    self.settings_editing = false;
                    self.commit_popup.input_buffer.clear();
                } else {
                    self.status_message = Some("Preferred Git Client cannot be empty".to_string());
                }
            }
            _ => {}
        }
    }

    pub fn cancel_settings_edit(&mut self) {
        self.settings_editing = false;
        self.commit_popup.input_buffer.clear();
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
        self.commit_popup.input_buffer.clear();
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
                self.commit_popup.input_buffer.clear();
                self.mode = Mode::Normal;
                return;
            }

            let status = repo::inspect_summary(&trimmed);
            self.statuses.push(status);
            self.config.items.push(trimmed.clone());
            self.original_items.push(trimmed.clone());

            self.sort_items_in_place();

            self.repo_search_query = None;
            if let Some(pos) = self.config.items.iter().position(|x| x == &trimmed) {
                self.selected_index = pos;
            } else {
                self.selected_index = self.config.items.len() - 1;
            }
            self.persist("Saved");
        }
        self.commit_popup.input_buffer.clear();
        self.mode = Mode::Normal;
    }

    pub fn start_bulk_add(&mut self) {
        crate::debug_log::info("Initiating bulk repository add");
        if !self.config.fzf.enabled {
            self.mode = Mode::BulkAddInput;
            self.commit_popup.input_buffer.clear();
        } else if !self.is_fzf_installed() {
            self.mode = Mode::BulkAddInput;
            self.commit_popup.input_buffer.clear();
            self.status_message =
                Some("fzf is not installed. Falling back to manual bulk add.".to_string());
        } else {
            self.pending_bulk_fzf = true;
        }
    }

    pub fn commit_bulk_add(&mut self) {
        let trimmed = self.input_buffer.trim().to_string();
        self.commit_popup.input_buffer.clear();
        self.mode = Mode::Normal;
        if !trimmed.is_empty() {
            self.bulk_add_path(trimmed);
        }
    }

    pub fn bulk_add_path(&mut self, path: String) {
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
        let git_only = self.config.fzf.git_only;

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

            self.repo_search_query = None;
            if let Some(pos) = self.config.items.iter().position(|x| x == &trimmed) {
                self.selected_index = pos;
            } else {
                self.selected_index = self.config.items.len() - 1;
            }
            self.persist("Added repository");
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

        self.fetching = true;
        self.status_message = Some(format!("Cloning {}...", url));
        self.mode = Mode::Normal;

        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let res = (|| -> Result<String, String> {
                let dest_expanded = repo::expand_tilde(&dest_str);
                let _ = std::fs::create_dir_all(&dest_expanded);

                let mut cmd = std::process::Command::new("git");
                cmd.env("GIT_TERMINAL_PROMPT", "0")
                    .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new");
                cmd.arg("clone").arg(&url).arg(&dest_expanded);

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

    pub fn commit_edit(&mut self) {
        let trimmed = self.input_buffer.trim().to_string();
        if !trimmed.is_empty() {
            if let Some(orig_idx) = self.get_selected_item_index() {
                if orig_idx < self.config.items.len() {
                    let old_item = self.config.items[orig_idx].clone();

                    if let Some(pos) = self.original_items.iter().position(|x| x == &old_item) {
                        self.original_items[pos] = trimmed.clone();
                    }

                    if let Some(time) = self.config.visits.remove(&old_item) {
                        self.config.visits.insert(trimmed.clone(), time);
                    }

                    if self.config.pinned.remove(&old_item) {
                        self.config.pinned.insert(trimmed.clone());
                    }

                    self.config.items[orig_idx] = trimmed.clone();
                    self.statuses[orig_idx] = repo::inspect_summary(&trimmed);

                    self.sort_items_in_place();

                    self.repo_search_query = None;

                    if let Some(pos) = self.config.items.iter().position(|x| x == &trimmed) {
                        self.selected_index = pos;
                    }
                    self.persist("Saved");
                }
            }
        }
        self.commit_popup.input_buffer.clear();
        self.mode = Mode::Normal;
    }

    pub fn confirm_delete(&mut self) {
        if let Some(orig_idx) = self.get_selected_item_index() {
            if orig_idx < self.config.items.len() {
                let item = self.config.items.remove(orig_idx);
                if orig_idx < self.statuses.len() {
                    self.statuses.remove(orig_idx);
                }
                if let Some(pos) = self.original_items.iter().position(|x| x == &item) {
                    self.original_items.remove(pos);
                }
                self.config.visits.remove(&item);
                self.config.pinned.remove(&item);
                self.persist("Deleted");
            }
        }
        self.repo_search_query = None;
        self.selected_index = 0;
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
        if let Some(item) = self.file_tree.visible_files.get(self.file_tree.file_list_selection).cloned() {
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
                let attempted =
                    if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
                        info.remote_tags_attempted
                    } else {
                        false
                    };
                if !attempted {
                    self.fetch_remote_tags(true);
                }
            }
            5 => {
                self.detail_focus = DetailSection::Remotes;
                let remote_name =
                    if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
                        info.remotes
                            .get(self.branch_list.remote_selection)
                            .or_else(|| info.remotes.first())
                            .map(|r| r.name.clone())
                    } else {
                        None
                    };
                if let Some(name) = remote_name {
                    self.fetch_remote(&name);
                }
            }
            6 => {
                self.detail_focus = DetailSection::Stashes;
                self.stash_list.stash_file_selection = 0;
                self.refresh_file_diff();
            }
            _ => {}
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
            let remote = info.remotes.get(self.branch_list.remote_selection).or_else(|| info.remotes.first());
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
                    let output = std::process::Command::new("git")
                        .env("GIT_TERMINAL_PROMPT", "0")
                        .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
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
                cmd.env("GIT_TERMINAL_PROMPT", "0")
                    .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new");
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
            self.status_message =
                Some(format!("Pushing tag '{}' to '{}'...", tag_name, remote_name));
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let mut cmd = std::process::Command::new("git");
                cmd.env("GIT_TERMINAL_PROMPT", "0")
                    .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new");
                cmd.arg("push").arg(&remote_name).arg(&tag_name).current_dir(&repo_path);
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
                cmd.env("GIT_TERMINAL_PROMPT", "0")
                    .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new");
                cmd.arg("push").arg(&remote_name).arg("--tags").current_dir(&repo_path);
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
                self.branch_list.remote_selection = (self.branch_list.remote_selection + page).min(total.saturating_sub(1));
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
                self.stash_list.stash_selection = (self.stash_list.stash_selection + page).min(total.saturating_sub(1));
                self.stash_list.stash_file_selection = 0;
                self.refresh_file_diff();
            }
        }
    }

    pub fn stash_file_up(&mut self) {
        self.stash_list.stash_file_selection = self.stash_list.stash_file_selection.saturating_sub(1);
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
        self.stash_list.stash_file_selection = self.stash_list.stash_file_selection.saturating_sub(page);
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

    pub fn graph_scroll_to_top(&mut self) {
        self.graph_scroll = 0;
    }

    pub fn graph_scroll_to_bottom(&mut self) {
        if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let max = info.graph_lines.len().saturating_sub(1);
            self.graph_scroll = max;
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

    pub fn yank_selected_commit_hash(&mut self) {
        if self.is_uncommitted_selected() {
            self.status_message = Some("Cannot yank uncommitted changes".to_string());
            return;
        }
        let hash_to_copy = if let Some(repo::ItemDetail::Repo { info, .. }) = &self.current_detail {
            let dirty = !info.changes.staged.is_empty()
                || !info.changes.unstaged.is_empty()
                || !info.changes.untracked.is_empty()
                || !info.changes.conflicted.is_empty();
            let commit_idx =
                if dirty { self.commit_list.selection.saturating_sub(1) } else { self.commit_list.selection };
            info.commits.get(commit_idx).map(|commit| commit.oid.clone())
        } else {
            None
        };

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
            if let Some(dest_path) = msg.strip_prefix("CLONE_SUCCESS:") {
                app.fetching = false;
                app.status_message = Some("Cloning completed successfully".to_string());
                app.add_repo_path(dest_path.to_string());
            } else if let Some(tags_data) = msg.strip_prefix("REMOTE_TAGS:") {
                let tags = repo::deserialize_tags(tags_data);
                if let Some(repo::ItemDetail::Repo { info, .. }) = &mut app.current_detail {
                    info.remote_tags = repo::TabData::Loaded(tags);
                    info.remote_tags_loaded = true;
                }
                app.fetching = false;
            } else if let Some(err_msg) = msg.strip_prefix("REMOTE_TAGS_ERR:") {
                app.set_error(err_msg.to_string());
                app.fetching = false;
            } else {
                let success_fetch = msg.starts_with("Fetched remote ");
                let is_err = msg.starts_with("Fetch failed:")
                    || msg.starts_with("Pull failed:")
                    || msg.starts_with("Push failed:")
                    || msg.starts_with("Failed to")
                    || msg.contains("failed");

                if is_err {
                    let has_conflict = msg.contains("conflict") || msg.contains("CONFLICT");
                    app.set_error(msg);
                    if has_conflict {
                        app.detail_focus = DetailSection::Conflicts;
                    }
                } else {
                    app.status_message = Some(msg);
                }
                app.fetching = false;
                app.resync_detail();
                if success_fetch {
                    app.fetch_remote_tags(false);
                }
            }
        }

        while let Ok((path, detail)) = app.detail_rx.try_recv() {
            app.detail_cache.insert(
                path.clone(),
                DetailCache { detail: detail.clone(), loaded_at: std::time::Instant::now() },
            );

            let is_currently_loading = Some(&path) == app.loading_repo_path.as_ref();
            let is_currently_open = if let Some(current) = &app.current_detail {
                match current {
                    repo::ItemDetail::Repo { resolved, .. }
                    | repo::ItemDetail::Missing { resolved, .. }
                    | repo::ItemDetail::Directory { resolved, .. }
                    | repo::ItemDetail::Error { resolved, .. } => {
                        resolved.to_string_lossy() == path
                    }
                }
            } else {
                false
            };

            if is_currently_loading || is_currently_open {
                app.apply_detail_snapshot(detail);
                if is_currently_loading {
                    app.loading_repo_path = None;
                }
            }
        }

        let mut tab_updated = false;
        while let Ok((path, tab_idx, payload)) = app.tab_rx.try_recv() {
            crate::debug_log::info(format!(
                "Received tab payload: tab_idx={}, path={}",
                tab_idx, path
            ));
            if let Some(repo::ItemDetail::Repo { resolved, info }) = &mut app.current_detail {
                let resolved_str = resolved.to_string_lossy().to_string();
                if resolved_str == path {
                    crate::debug_log::info(format!("Paths match! Updating tab_idx={}", tab_idx));
                    tab_updated = true;
                    if tab_idx < 8 {
                        info.tab_loading[tab_idx] = false;
                        info.tab_loaded_at[tab_idx] = Some(std::time::Instant::now());
                    }
                    match payload {
                        repo::TabPayload::Files(res) => {
                            info.files = match res {
                                Ok(files) => repo::TabData::Loaded(files),
                                Err(e) => repo::TabData::Error(e),
                            };
                        }
                        repo::TabPayload::Graph(res) => {
                            info.graph_lines = match res {
                                Ok(lines) => repo::TabData::Loaded(lines),
                                Err(e) => repo::TabData::Error(e),
                            };
                        }
                        repo::TabPayload::Branches { local, remote } => {
                            info.local_branches = match local {
                                Ok(b) => repo::TabData::Loaded(b),
                                Err(e) => repo::TabData::Error(e),
                            };
                            info.remote_branches = match remote {
                                Ok(b) => repo::TabData::Loaded(b),
                                Err(e) => repo::TabData::Error(e),
                            };
                        }
                        repo::TabPayload::Tags { local, remote } => {
                            info.local_tags = match local {
                                Ok(t) => repo::TabData::Loaded(t),
                                Err(e) => repo::TabData::Error(e),
                            };
                            info.remote_tags = match remote {
                                Ok(t) => repo::TabData::Loaded(t),
                                Err(e) => repo::TabData::Error(e),
                            };
                        }
                        repo::TabPayload::Remotes(res) => {
                            info.remotes = match res {
                                Ok(r) => repo::TabData::Loaded(r),
                                Err(e) => repo::TabData::Error(e),
                            };
                        }
                        repo::TabPayload::Stashes(res) => {
                            info.stashes = match res {
                                Ok(s) => repo::TabData::Loaded(s),
                                Err(e) => repo::TabData::Error(e),
                            };
                        }
                        repo::TabPayload::Overview(res) => match res {
                            Ok((stats, capped)) => {
                                info.committer_stats = repo::TabData::Loaded(stats);
                                info.committer_stats_limit_reached = capped;
                            }
                            Err(e) => {
                                info.committer_stats = repo::TabData::Error(e);
                            }
                        },
                    }
                }
            }
        }
        if tab_updated {
            app.update_cache_from_current_detail();
            app.rebuild_visible_files();
        }

        if app.pending_git_app {
            app.pending_git_app = false;
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
                    let git_app_name = &app.config.git_app;
                    let status =
                        std::process::Command::new(git_app_name).current_dir(&path).status();

                    let _ = crossterm::terminal::enable_raw_mode();
                    let _ = crossterm::execute!(
                        std::io::stdout(),
                        crossterm::terminal::EnterAlternateScreen,
                        crossterm::event::EnableMouseCapture
                    );
                    let _ = terminal.clear();

                    match status {
                        Ok(s) if s.success() => {
                            app.status_message = Some(format!("Returned from {}", git_app_name));
                            app.refresh_selected_status();
                        }
                        Ok(_) => {
                            app.status_message =
                                Some(format!("{} exited with error", git_app_name));
                            app.refresh_selected_status();
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            app.set_error(format!("{} is not found in the system", git_app_name));
                        }
                        Err(e) => {
                            app.set_error(format!("Could not run {}: {}", git_app_name, e));
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
                    .env("GIT_TERMINAL_PROMPT", "0")
                    .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=accept-new")
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

                let cmd = if app.config.fzf.git_only {
                    format!(
                        "if ! command -v fzf >/dev/null 2>&1; then exit 127; fi; (command -v fd >/dev/null 2>&1 && fd -H '^\\.git$' '{}' --max-depth {} {} 2>/dev/null | xargs -I {{}} dirname {{}} || find '{}' -maxdepth {} {} -name .git -type d 2>/dev/null | xargs -I {{}} dirname {{}}) | fzf",
                        start_dir,
                        max_depth + 1,
                        fd_excludes,
                        start_dir,
                        max_depth + 1,
                        find_prune_clause
                    )
                } else {
                    format!(
                        "if ! command -v fzf >/dev/null 2>&1; then exit 127; fi; (command -v fd >/dev/null 2>&1 && fd . '{}' --type d --max-depth {} {} 2>/dev/null || find '{}' -maxdepth {} {} -type d -print 2>/dev/null) | fzf",
                        start_dir, max_depth, fd_excludes, start_dir, max_depth, find_prune_clause
                    )
                };

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
                            app.set_error("fzf is not installed. Please install fzf.".to_string());
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        app.set_error("fzf is not installed. Please install fzf.".to_string());
                    }
                    Err(e) => {
                        app.set_error(format!("Could not run fzf: {}", e));
                    }
                }
            }
        }

        if app.pending_bulk_fzf {
            app.pending_bulk_fzf = false;
            app.input_buffer.clear();
            app.mode = Mode::Normal;

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
                                app.bulk_add_path(selected);
                            }
                        } else if out.status.code() == Some(127) {
                            app.set_error("fzf is not installed. Please install fzf.".to_string());
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        app.set_error("fzf is not installed. Please install fzf.".to_string());
                    }
                    Err(e) => {
                        app.set_error(format!("Could not run fzf: {}", e));
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
                                for file in files.iter() {
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
                                        app.file_tree.expanded_folders.insert(accumulated.clone());
                                    }
                                    app.rebuild_visible_files();
                                    if let Some(pos) = app.file_tree.visible_files
                                        .iter()
                                        .position(|item| item.full_path == selected)
                                    {
                                        app.file_tree.file_list_selection = pos;
                                        app.file_tree.file_content_scroll = 0;
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
        let inner_area = area.inner(Margin { vertical: 1, horizontal: 1 });

        let available_height = inner_area.height.saturating_sub(app.status_height());
        let visible_count =
            (available_height / ITEM_HEIGHT).min(app.get_items_len() as u16) as usize;
        app.clamp_scroll(visible_count);
        app.clamp_help_scroll(area.height as usize);

        app.trigger_tab_load_if_needed(app.detail_tab);

        // Capture panel rects from the draw pass for mouse hit-testing.
        let mut detail_areas = DetailAreas::default();
        let mut main_areas = Vec::new();
        terminal.draw(|f| {
            ui::draw(f, &app, area, inner_area, visible_count, &mut detail_areas, &mut main_areas)
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

        if event::poll(std::time::Duration::from_millis(app.config.poll_interval_ms))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == crossterm::event::KeyEventKind::Press
                        && !input::handle_key(&mut app, key, visible_count)
                    {
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

fn is_tool_installed(name: &str) -> bool {
    #[cfg(target_os = "windows")]
    let cmd = "where";
    #[cfg(not(target_os = "windows"))]
    let cmd = "which";

    std::process::Command::new(cmd)
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn copy_to_clipboard(text: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        let mut child = std::process::Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        }
        child.wait().map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(target_os = "windows")]
    {
        use std::io::Write;
        let mut child = std::process::Command::new("clip")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| e.to_string())?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        }
        child.wait().map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        use std::io::Write;
        if let Ok(mut child) = std::process::Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                if stdin.write_all(text.as_bytes()).is_ok() {
                    let _ = child.wait();
                    return Ok(());
                }
            }
        }
        if let Ok(mut child) = std::process::Command::new("xsel")
            .arg("-ib")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                if stdin.write_all(text.as_bytes()).is_ok() {
                    let _ = child.wait();
                    return Ok(());
                }
            }
        }
        Err("Could not find xclip or xsel on Linux system".to_string())
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
    fn test_stash_creation_flow() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_stash.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);
        app.mode = Mode::Detail;

        // Verify starting stash creation triggers correct state
        app.start_stash_create();
        assert_eq!(app.mode, Mode::StashCreateInput);
        assert!(app.input_buffer.is_empty());

        // Simulate typing stash name
        app.input_buffer = "my_custom_stash".to_string();

        // Simulate pressing Esc (cancel)
        let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        let consumed = crate::input::handle_key(&mut app, esc_key, 0);
        assert!(consumed);
        assert_eq!(app.mode, Mode::Detail);

        // Re-start and simulate typing again
        app.start_stash_create();
        app.input_buffer = "my_custom_stash".to_string();

        // Simulate backspace and typing character
        let backspace_key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
        crate::input::handle_key(&mut app, backspace_key, 0);
        assert_eq!(app.input_buffer, "my_custom_stas");

        let char_key = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::empty());
        crate::input::handle_key(&mut app, char_key, 0);
        assert_eq!(app.input_buffer, "my_custom_stash");

        // Simulate enter (commit stash)
        let enter_key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        crate::input::handle_key(&mut app, enter_key, 0);

        // Mode returns to detail
        assert_eq!(app.mode, Mode::Detail);

        // Verify we can trigger stash creation from Commits panel if we have uncommitted changes
        app.mode = Mode::Detail;
        app.detail_focus = DetailSection::Commits;

        // Mock uncommitted changes
        let mut mock_info = repo::RepoInfo::default();
        mock_info.changes.unstaged =
            vec![repo::FileEntry { path: "dirty.rs".to_string(), label: "M" }];
        app.current_detail = Some(repo::ItemDetail::Repo {
            resolved: PathBuf::from("a_repo"),
            info: Box::new(mock_info),
        });

        assert!(app.has_uncommitted_changes());

        // Pressing 's' should activate StashCreateInput
        let s_key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::empty());
        let consumed = crate::input::handle_key(&mut app, s_key, 0);
        assert!(consumed);
        assert_eq!(app.mode, Mode::StashCreateInput);
    }

    #[test]
    fn test_network_action_progress_and_error_handling() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_network.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        // Simulating the start of a network action
        app.fetching = true;
        app.status_message = Some("Pushing...".to_string());

        // Assert progress popup is active
        assert!(app.fetching);
        assert_eq!(app.status_message.as_deref(), Some("Pushing..."));

        // Simulate background thread sending a failure message
        app.tx.send("Push failed: git push rejected".to_string()).unwrap();

        // Run receiver check
        while let Ok(msg) = app.rx.try_recv() {
            let is_err = msg.starts_with("Fetch failed:")
                || msg.starts_with("Pull failed:")
                || msg.starts_with("Push failed:")
                || msg.starts_with("Failed to")
                || msg.contains("failed");

            if is_err {
                app.error_message = Some(msg);
            } else {
                app.status_message = Some(msg);
            }
            app.fetching = false;
        }

        // Verify that fetching is cleared and error_message popup is active
        assert!(!app.fetching);
        assert_eq!(app.error_message.as_deref(), Some("Push failed: git push rejected"));

        // Verify keypress dismisses the error popup
        let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        let consumed = crate::input::handle_key(&mut app, esc_key, 0);
        assert!(consumed);
        assert!(app.error_message.is_none());
    }

    #[test]
    fn test_remote_tags_progress_and_error_handling() {
        let config = Config {
            items: vec![".".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_remote_tags_progress.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        let mock_info = crate::repo::RepoInfo {
            branch: Some("main".to_string()),
            remotes: crate::repo::TabData::Loaded(vec![crate::repo::RemoteInfo {
                name: "origin".to_string(),
                url: "git@github.com:tareqmy/gitwig.git".to_string(),
                push_url: None,
                refspecs: vec![],
            }]),
            ..crate::repo::RepoInfo::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: std::path::PathBuf::from("."),
            info: Box::new(mock_info),
        });

        // Trigger fetch with show_progress = true
        app.fetch_remote_tags(true);
        assert!(app.fetching);
        assert_eq!(app.status_message.as_deref(), Some("Fetching tags from 'origin'..."));

        // Simulate background thread sending REMOTE_TAGS_ERR
        app.tx.send("REMOTE_TAGS_ERR:Failed to get remote tags: custom error".to_string()).unwrap();

        // Run rx loop (same as inside app::run)
        if let Ok(msg) = app.rx.try_recv() {
            if let Some(err_msg) = msg.strip_prefix("REMOTE_TAGS_ERR:") {
                app.set_error(err_msg.to_string());
                app.fetching = false;
            }
        }

        assert!(!app.fetching);
        assert_eq!(app.error_message.as_deref(), Some("Failed to get remote tags: custom error"));
    }

    #[test]
    fn test_remote_fetch_progress_and_error_handling() {
        let config = Config {
            items: vec![".".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_remote_fetch_progress.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        let mock_info = crate::repo::RepoInfo {
            branch: Some("main".to_string()),
            remotes: crate::repo::TabData::Loaded(vec![crate::repo::RemoteInfo {
                name: "origin".to_string(),
                url: "git@github.com:tareqmy/gitwig.git".to_string(),
                push_url: None,
                refspecs: vec![],
            }]),
            ..crate::repo::RepoInfo::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: std::path::PathBuf::from("."),
            info: Box::new(mock_info),
        });

        // Trigger fetch from remote tab (fetch_remote)
        app.fetch_remote("origin");
        assert!(app.fetching);
        assert_eq!(app.status_message.as_deref(), Some("Fetching remote 'origin'..."));

        // Simulate background thread sending Fetch failed message
        app.tx.send("Fetch failed: custom fetch error".to_string()).unwrap();

        // Run rx loop (same as inside app::run)
        if let Ok(msg) = app.rx.try_recv() {
            let is_err = msg.starts_with("Fetch failed:")
                || msg.starts_with("Pull failed:")
                || msg.starts_with("Push failed:")
                || msg.starts_with("Failed to")
                || msg.contains("failed");

            if is_err {
                app.set_error(msg);
            }
            app.fetching = false;
        }

        assert!(!app.fetching);
        assert_eq!(app.error_message.as_deref(), Some("Fetch failed: custom fetch error"));
    }

    #[test]
    fn test_set_error_logging() {
        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_set_error.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        let test_error_msg = "Test error message for debugging".to_string();
        app.set_error(test_error_msg.clone());

        assert_eq!(app.error_message.as_ref(), Some(&test_error_msg));

        // Check if debug log contains the message
        let logs = crate::debug_log::get_logs();
        assert!(logs.iter().any(|log| log.contains("ERROR") && log.contains(&test_error_msg)));
    }

    #[test]
    fn test_sorting_logic() {
        let config = Config {
            items: vec!["z_repo".to_string(), "a_repo".to_string(), "m_repo".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_sort.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
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
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_duplicate.toml");
        // Ensure starting with a clean state and clean up upon drop
        let _ = std::fs::remove_file(&temp_path);
        let _guard = TestFileGuard { path: temp_path.clone() };
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
        assert_eq!(app.status_message, Some("Repository already added".to_string()));
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
            assert_eq!(app.status_message, Some("Repository already added".to_string()));
            app.status_message = None; // Reset

            // Try the opposite direction: add a new absolute path, then try to add with tilde
            let new_abs = format!("{}/another_cool_repo", home_str);
            app.input_buffer = new_abs;
            app.commit_add();
            assert_eq!(app.config.items.len(), 3);
            assert_eq!(app.config.items[2], format!("{}/another_cool_repo", home_str));
            assert_eq!(app.status_message, Some("Saved".to_string()));
            app.status_message = None; // Reset

            // Now try to add with tilde
            app.input_buffer = "~/another_cool_repo".to_string();
            app.commit_add();
            // Should be rejected
            assert_eq!(app.config.items.len(), 3);
            assert_eq!(app.status_message, Some("Repository already added".to_string()));
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
        assert_eq!(app.status_message, Some("Repository already added".to_string()));
    }

    #[test]
    fn test_bulk_add_folders() {
        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_dir = std::env::temp_dir().join("gitwig_test_bulk_add_dir");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let repo_a = temp_dir.join("repo_a");
        let repo_b = temp_dir.join("repo_b");
        let repo_c = temp_dir.join("repo_c");
        std::fs::create_dir_all(repo_a.join(".git")).unwrap();
        std::fs::create_dir_all(&repo_b).unwrap();
        std::fs::create_dir_all(repo_c.join(".git")).unwrap();

        let config_path = temp_dir.join("config_bulk.toml");
        let _ = std::fs::remove_file(&config_path);
        let _guard = TestFileGuard { path: config_path.clone() };
        let mut app = App::new(config, config_path);

        // Case 1: git_only is enabled (default)
        app.config.fzf.git_only = true;
        app.input_buffer = temp_dir.to_string_lossy().to_string();
        app.commit_bulk_add();

        // Should include repo_a and repo_c, but NOT repo_b
        assert_eq!(app.config.items.len(), 2);
        assert!(app.config.items.iter().any(|item| item.ends_with("repo_a")));
        assert!(app.config.items.iter().any(|item| item.ends_with("repo_c")));
        assert!(!app.config.items.iter().any(|item| item.ends_with("repo_b")));

        // Clear items and try again with git_only = false
        app.config.items.clear();
        app.original_items.clear();
        app.statuses.clear();

        app.config.fzf.git_only = false;
        app.input_buffer = temp_dir.to_string_lossy().to_string();
        app.commit_bulk_add();

        // Should include repo_a, repo_b, and repo_c
        assert_eq!(app.config.items.len(), 3);
        assert!(app.config.items.iter().any(|item| item.ends_with("repo_a")));
        assert!(app.config.items.iter().any(|item| item.ends_with("repo_b")));
        assert!(app.config.items.iter().any(|item| item.ends_with("repo_c")));

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_pinning_and_sorting() {
        let config = Config {
            items: vec!["z_repo".to_string(), "a_repo".to_string(), "m_repo".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Alphabetical,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_pin.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
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
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_commit_scroll.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
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
    fn test_commit_popup_maximized_toggle() {
        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_commit_maximize.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        assert!(!app.commit_popup_maximized);

        app.toggle_commit_popup_maximized();
        assert!(app.commit_popup_maximized);

        app.toggle_commit_popup_maximized();
        assert!(!app.commit_popup_maximized);

        app.toggle_commit_popup_maximized();
        assert!(app.commit_popup_maximized);

        // Cancel resets it
        app.cancel_commit();
        assert!(!app.commit_popup_maximized);
    }

    #[test]
    fn test_cherry_pick_and_revert_flow() {
        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_cherry_pick.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        // Set up a mock repo detail with commits
        let mock_info = crate::repo::RepoInfo {
            branch: Some("main".to_string()),
            commits: vec![crate::repo::CommitEntry {
                id: "1234567".to_string(),
                oid: "1234567890abcdef1234567890abcdef12345678".to_string(),
                summary: "test commit".to_string(),
                author: "author".to_string(),
                when: "today".to_string(),
                date: "today".to_string(),
                refs: vec![],
                message: "msg".to_string(),
                files: vec![],
                signature_status: "N".to_string(),
            }],
            ..Default::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: std::path::PathBuf::from("/mock/repo"),
            info: Box::new(mock_info),
        });

        // 1. Cherry-pick flow
        app.commit_list.selection = 0;
        app.request_cherry_pick();
        assert_eq!(app.mode, Mode::CherryPickConfirm);
        assert!(app.cherry_pick_target.is_some());
        assert_eq!(
            app.cherry_pick_target.as_ref().unwrap().0,
            "1234567890abcdef1234567890abcdef12345678"
        );

        app.cancel_cherry_pick();
        assert_eq!(app.mode, Mode::Detail);
        assert!(app.cherry_pick_target.is_none());

        // 2. Revert flow
        app.commit_list.selection = 0;
        app.request_revert();
        assert_eq!(app.mode, Mode::RevertConfirm);
        assert!(app.revert_target.is_some());
        assert_eq!(
            app.revert_target.as_ref().unwrap().0,
            "1234567890abcdef1234567890abcdef12345678"
        );

        app.cancel_revert();
        assert_eq!(app.mode, Mode::Detail);
        assert!(app.revert_target.is_none());
    }

    #[test]
    fn test_commit_amend_flow() {
        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_commit_amend.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        assert!(!app.commit_amend);

        app.toggle_commit_amend();
        assert!(app.commit_amend);

        app.toggle_commit_amend();
        assert!(!app.commit_amend);

        // Without HEAD
        app.start_commit_amend();
        assert_eq!(app.status_message.as_deref(), Some("No commit to amend"));
        assert_eq!(app.mode, Mode::Normal);

        // With HEAD
        let info = crate::repo::RepoInfo {
            head: Some(crate::repo::HeadInfo {
                short_id: "dummy_sha".to_string(),
                summary: "dummy message".to_string(),
                author: "author".to_string(),
                when: "now".to_string(),
            }),
            ..Default::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: PathBuf::from("/dummy"),
            info: Box::new(info),
        });

        app.start_commit_amend();
        assert!(app.commit_amend);
        assert!(app.commit_editing);
        assert_eq!(app.mode, Mode::CommitInput);
    }

    #[test]
    fn test_splitter_dragging() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::layout::Rect;

        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_splitter.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
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
    fn test_mouse_row_selection_in_detail_panels() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::layout::Rect;

        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_mouse_select.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);
        app.mode = Mode::Detail;

        // 1. Commits panel click test
        app.detail_areas = crate::ui_detail::DetailAreas::default();
        app.detail_areas.commits = Some(Rect::new(0, 0, 100, 20));
        app.detail_areas.commits_inner = Some(Rect::new(1, 1, 98, 18));
        let mock_info = repo::RepoInfo {
            branch: Some("main".to_string()),
            commits: vec![
                repo::CommitEntry {
                    id: "1".to_string(),
                    oid: "1111111111111111111111111111111111111111".to_string(),
                    summary: "C1".to_string(),
                    author: "A".to_string(),
                    when: "now".to_string(),
                    date: "now".to_string(),
                    refs: vec![],
                    message: "msg".to_string(),
                    files: vec![],
                    signature_status: "N".to_string(),
                },
                repo::CommitEntry {
                    id: "2".to_string(),
                    oid: "2222222222222222222222222222222222222222".to_string(),
                    summary: "C2".to_string(),
                    author: "B".to_string(),
                    when: "now".to_string(),
                    date: "now".to_string(),
                    refs: vec![],
                    message: "msg".to_string(),
                    files: vec![],
                    signature_status: "N".to_string(),
                },
                repo::CommitEntry {
                    id: "3".to_string(),
                    oid: "3333333333333333333333333333333333333333".to_string(),
                    summary: "C3".to_string(),
                    author: "C".to_string(),
                    when: "now".to_string(),
                    date: "now".to_string(),
                    refs: vec![],
                    message: "msg".to_string(),
                    files: vec![],
                    signature_status: "N".to_string(),
                },
            ],
            ..repo::RepoInfo::default()
        };
        app.current_detail = Some(repo::ItemDetail::Repo {
            resolved: PathBuf::from("a_repo"),
            info: Box::new(mock_info),
        });

        let commit_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 3,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, commit_click);
        assert_eq!(app.commit_list.selection, 1);
        assert_eq!(app.detail_focus, DetailSection::Commits);

        // 2. Staged subpanel click test
        app.detail_areas = crate::ui_detail::DetailAreas::default();
        let mut mock_info_2 = repo::RepoInfo::default();
        mock_info_2.changes.staged = vec![
            repo::FileEntry { path: "s1.rs".to_string(), label: "M" },
            repo::FileEntry { path: "s2.rs".to_string(), label: "M" },
        ];
        mock_info_2.changes.unstaged =
            vec![repo::FileEntry { path: "u1.rs".to_string(), label: "M" }];
        app.current_detail = Some(repo::ItemDetail::Repo {
            resolved: PathBuf::from("a_repo"),
            info: Box::new(mock_info_2),
        });

        app.detail_areas.staged_sub = Some(Rect::new(0, 20, 50, 10));
        app.detail_areas.staged_sub_inner = Some(Rect::new(1, 21, 48, 8));
        let staged_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: 22,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, staged_click);
        assert_eq!(app.status_list.staging_file_selection, 1);
        assert_eq!(app.detail_focus, DetailSection::Staged);

        // 3. Unstaged subpanel click test
        app.detail_areas = crate::ui_detail::DetailAreas::default();
        let mut mock_info_2_unstaged = repo::RepoInfo::default();
        mock_info_2_unstaged.changes.unstaged =
            vec![repo::FileEntry { path: "u1.rs".to_string(), label: "M" }];
        app.current_detail = Some(repo::ItemDetail::Repo {
            resolved: PathBuf::from("a_repo"),
            info: Box::new(mock_info_2_unstaged),
        });
        app.detail_areas.unstaged_sub = Some(Rect::new(0, 30, 50, 10));
        app.detail_areas.unstaged_sub_inner = Some(Rect::new(1, 31, 48, 8));
        let unstaged_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: 31,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, unstaged_click);
        assert_eq!(app.status_list.staging_file_selection, 0);
        assert_eq!(app.detail_focus, DetailSection::Unstaged);

        // 4. Local branches click test
        app.detail_areas = crate::ui_detail::DetailAreas::default();
        let mock_info_3 = repo::RepoInfo {
            local_branches: repo::TabData::Loaded(vec![
                repo::BranchInfo {
                    name: "b1".to_string(),
                    is_head: true,
                    short_sha: "123".to_string(),
                    short_message: "msg".to_string(),
                },
                repo::BranchInfo {
                    name: "b2".to_string(),
                    is_head: false,
                    short_sha: "456".to_string(),
                    short_message: "msg".to_string(),
                },
            ]),
            remote_branches: repo::TabData::Loaded(vec![repo::BranchInfo {
                name: "origin/b1".to_string(),
                is_head: false,
                short_sha: "123".to_string(),
                short_message: "msg".to_string(),
            }]),
            ..Default::default()
        };
        app.current_detail = Some(repo::ItemDetail::Repo {
            resolved: PathBuf::from("a_repo"),
            info: Box::new(mock_info_3),
        });

        app.detail_areas.local_branches = Some(Rect::new(0, 0, 50, 20));
        app.detail_areas.local_branches_inner = Some(Rect::new(1, 1, 48, 18));
        let local_branch_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: 2, // inner.y = 1, so row 2 is index 1
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, local_branch_click);
        assert_eq!(app.branch_list.local_branch_selection, 1);
        assert_eq!(app.detail_focus, DetailSection::LocalBranches);

        // 5. Remote branches click test
        app.detail_areas = crate::ui_detail::DetailAreas::default();
        let mock_info_3_remote = repo::RepoInfo {
            remote_branches: repo::TabData::Loaded(vec![repo::BranchInfo {
                name: "origin/b1".to_string(),
                is_head: false,
                short_sha: "123".to_string(),
                short_message: "msg".to_string(),
            }]),
            ..Default::default()
        };
        app.current_detail = Some(repo::ItemDetail::Repo {
            resolved: PathBuf::from("a_repo"),
            info: Box::new(mock_info_3_remote),
        });
        app.detail_areas.remote_branches = Some(Rect::new(50, 0, 50, 20));
        app.detail_areas.remote_branches_inner = Some(Rect::new(51, 1, 48, 18));
        let remote_branch_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 55,
            row: 1, // index 0
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, remote_branch_click);
        assert_eq!(app.branch_list.remote_branch_selection, 0);
        assert_eq!(app.detail_focus, DetailSection::RemoteBranches);

        // 6. Local tags click test
        app.detail_areas = crate::ui_detail::DetailAreas::default();
        let mock_info_4 = repo::RepoInfo {
            local_tags: repo::TabData::Loaded(vec![
                repo::BranchInfo {
                    name: "t1".to_string(),
                    is_head: false,
                    short_sha: "123".to_string(),
                    short_message: "msg".to_string(),
                },
                repo::BranchInfo {
                    name: "t2".to_string(),
                    is_head: false,
                    short_sha: "456".to_string(),
                    short_message: "msg".to_string(),
                },
            ]),
            ..Default::default()
        };
        app.current_detail = Some(repo::ItemDetail::Repo {
            resolved: PathBuf::from("a_repo"),
            info: Box::new(mock_info_4),
        });

        app.detail_areas.local_tags = Some(Rect::new(0, 0, 100, 20));
        app.detail_areas.local_tags_inner = Some(Rect::new(1, 1, 98, 18));
        let tag_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 2, // index 1
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, tag_click);
        assert_eq!(app.tag_list.local_tag_selection, 1);
        assert_eq!(app.detail_focus, DetailSection::LocalTags);

        // 7. Remotes click test
        app.detail_areas = crate::ui_detail::DetailAreas::default();
        let mock_info_5 = repo::RepoInfo {
            remotes: repo::TabData::Loaded(vec![
                repo::RemoteInfo {
                    name: "r1".to_string(),
                    url: "url1".to_string(),
                    push_url: None,
                    refspecs: vec![],
                },
                repo::RemoteInfo {
                    name: "r2".to_string(),
                    url: "url2".to_string(),
                    push_url: None,
                    refspecs: vec![],
                },
            ]),
            ..Default::default()
        };
        app.current_detail = Some(repo::ItemDetail::Repo {
            resolved: PathBuf::from("a_repo"),
            info: Box::new(mock_info_5),
        });

        app.detail_areas.remotes = Some(Rect::new(0, 0, 100, 20));
        app.detail_areas.remotes_inner = Some(Rect::new(1, 1, 98, 18));
        let remote_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 2, // index 1
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, remote_click);
        assert_eq!(app.branch_list.remote_selection, 1);
        assert_eq!(app.detail_focus, DetailSection::Remotes);

        // 8. Stashes and Stashed Files click test
        app.detail_areas = crate::ui_detail::DetailAreas::default();
        let mock_info_6 = repo::RepoInfo {
            stashes: repo::TabData::Loaded(vec![
                repo::StashInfo {
                    index: 0,
                    commit_id: "123".to_string(),
                    message: "s1".to_string(),
                    files: vec![
                        repo::FileEntry { path: "f1.rs".to_string(), label: "M" },
                        repo::FileEntry { path: "f2.rs".to_string(), label: "M" },
                    ],
                },
                repo::StashInfo {
                    index: 1,
                    commit_id: "456".to_string(),
                    message: "s2".to_string(),
                    files: vec![],
                },
            ]),
            ..Default::default()
        };
        app.current_detail = Some(repo::ItemDetail::Repo {
            resolved: PathBuf::from("a_repo"),
            info: Box::new(mock_info_6),
        });

        app.detail_areas.stashes = Some(Rect::new(0, 0, 100, 20));
        app.detail_areas.stashes_inner = Some(Rect::new(1, 1, 98, 18));
        let stash_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 2, // index 1
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, stash_click);
        assert_eq!(app.stash_list.stash_selection, 1);
        assert_eq!(app.detail_focus, DetailSection::Stashes);

        app.detail_areas = crate::ui_detail::DetailAreas::default();
        // re-apply mock info if needed (already in app.current_detail)
        app.stash_list.stash_selection = 0;
        app.detail_areas.stashed_files = Some(Rect::new(0, 20, 100, 20));
        app.detail_areas.stashed_files_inner = Some(Rect::new(1, 21, 98, 18));
        let stash_file_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 22, // index 1 (relative to 21)
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, stash_file_click);
        assert_eq!(app.stash_list.stash_file_selection, 1);
        assert_eq!(app.detail_focus, DetailSection::StashedFiles);
    }

    #[test]
    fn test_settings_mode_navigation_and_editing() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_settings.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
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
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
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

        // Go down to Max Commits (index 6)
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
        assert_eq!(app.settings_selected_index, 6);

        // Edit Max Commits
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(app.settings_editing);
        app.input_buffer = "100".to_string();
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(!app.settings_editing);
        assert_eq!(app.config.max_commits, 100);

        // Go down to Page Size (index 7)
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
        assert_eq!(app.settings_selected_index, 7);

        // Edit Page Size
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(app.settings_editing);
        app.input_buffer = "15".to_string();
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(!app.settings_editing);
        assert_eq!(app.config.page_size, 15);

        // Go down to FZF Excludes (index 8)
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
        assert_eq!(app.settings_selected_index, 8);

        // Edit FZF Excludes
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(app.settings_editing);
        app.input_buffer = "target, node_modules ,.git".to_string();
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(!app.settings_editing);
        assert_eq!(
            app.config.fzf.excludes,
            vec!["target".to_string(), "node_modules".to_string(), ".git".to_string()]
        );

        // Go down to Preferred Git Client (index 9)
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
        assert_eq!(app.settings_selected_index, 9);

        // Edit Preferred Git Client
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(app.settings_editing);
        app.input_buffer = "lazygit".to_string();
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(!app.settings_editing);
        assert_eq!(app.config.git_app, "lazygit");

        // Go down to FZF Git Only (index 10)
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
        assert_eq!(app.settings_selected_index, 10);
        assert!(app.config.fzf.git_only);

        // Toggle FZF Git Only
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(!app.config.fzf.git_only);
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(app.config.fzf.git_only);

        // Go down to Use FZF (index 11)
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
        assert_eq!(app.settings_selected_index, 11);
        assert!(app.config.fzf.enabled);

        // Toggle Use FZF
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(!app.config.fzf.enabled);
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(app.config.fzf.enabled);

        // Go down to Compatibility Mode (index 12)
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
        assert_eq!(app.settings_selected_index, 12);
        assert!(!app.config.compatibility_mode);

        // Toggle Compatibility Mode
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(app.config.compatibility_mode);
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(!app.config.compatibility_mode);

        // Go down to Resync on Tab Change (index 13)
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
        assert_eq!(app.settings_selected_index, 13);
        assert!(!app.config.resync_on_tab_change);

        // Toggle Resync on Tab Change
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(app.config.resync_on_tab_change);
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(!app.config.resync_on_tab_change);

        // Test PageUp, PageDown, Home, and End key navigation in Settings Mode
        app.config.page_size = 3;

        // At index 13: PageUp should go to 13 - 3 = 10
        crate::input::handle_key(&mut app, key_event(KeyCode::PageUp), 10);
        assert_eq!(app.settings_selected_index, 10);

        // PageUp should go to 10 - 3 = 7
        crate::input::handle_key(&mut app, key_event(KeyCode::PageUp), 10);
        assert_eq!(app.settings_selected_index, 7);

        // PageDown should go to 7 + 3 = 10
        crate::input::handle_key(&mut app, key_event(KeyCode::PageDown), 10);
        assert_eq!(app.settings_selected_index, 10);

        // End should go to 13
        crate::input::handle_key(&mut app, key_event(KeyCode::End), 10);
        assert_eq!(app.settings_selected_index, 13);

        // Home should go to 0
        crate::input::handle_key(&mut app, key_event(KeyCode::Home), 10);
        assert_eq!(app.settings_selected_index, 0);

        // Press Esc to exit settings
        crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_remote_add_delete_flow() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_remotes.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        // Put app in Detail Mode on tab 5 (Remotes)
        app.mode = Mode::Detail;
        app.detail_tab = 5;
        app.detail_focus = DetailSection::Remotes;
        app.current_detail = Some(repo::ItemDetail::Repo {
            resolved: std::path::PathBuf::from("."),
            info: Box::new(repo::RepoInfo {
                remotes: repo::TabData::Loaded(vec![repo::RemoteInfo {
                    name: "origin".to_string(),
                    url: "https://github.com/example/repo.git".to_string(),
                    push_url: None,
                    refspecs: vec![],
                }]),
                ..Default::default()
            }),
        });

        // Trigger remote add (a/A)
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('a')), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::RemoteAddNameInput);

        // Type remote name: "upstream"
        app.input_buffer = "upstream".to_string();
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::RemoteAddUrlInput);
        assert_eq!(app.remote_add_name, "upstream");

        // Escape URL input back to Detail Mode
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::Detail);

        // Trigger remote delete (d/D) on the selected remote ("origin")
        app.branch_list.remote_selection = 0;
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('d')), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::RemoteDeleteConfirm);
        assert_eq!(app.remote_action_target.as_deref(), Some("origin"));

        // Press 'n' to cancel deletion
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('n')), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::Detail);
        assert!(app.remote_action_target.is_none());
    }

    #[test]
    fn test_workspace_tab_right_arrow_inspect() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_inspect.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        // Open details view
        app.mode = Mode::Detail;
        app.detail_tab = 0;
        app.detail_focus = DetailSection::Staged;

        let mut changes = crate::repo::WorktreeChanges::default();
        changes.staged.push(crate::repo::FileEntry { path: "dummy.txt".to_string(), label: "M" });
        let info = crate::repo::RepoInfo {
            branch: Some("main".to_string()),
            changes,
            ..crate::repo::RepoInfo::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: PathBuf::from("."),
            info: Box::new(info),
        });
        app.commit_list.selection = 0;

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
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_inspect_enter.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        // Open details view and focus Commits section
        app.mode = Mode::Detail;
        app.detail_tab = 0;
        app.detail_focus = DetailSection::Commits;

        let mut changes = crate::repo::WorktreeChanges::default();
        changes.staged.push(crate::repo::FileEntry { path: "dummy.txt".to_string(), label: "M" });
        let info = crate::repo::RepoInfo {
            branch: Some("main".to_string()),
            changes,
            ..crate::repo::RepoInfo::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: PathBuf::from("."),
            info: Box::new(info),
        });
        app.commit_list.selection = 0;

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
    fn test_inspect_commit_shortcut() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_inspect_commit.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        // Open details view and focus Commits section
        app.mode = Mode::Inspect;
        app.detail_tab = 0;
        app.detail_focus = DetailSection::Staged;

        let mut changes = crate::repo::WorktreeChanges::default();
        changes.staged.push(crate::repo::FileEntry { path: "dummy.txt".to_string(), label: "M" });
        let info = crate::repo::RepoInfo {
            branch: Some("main".to_string()),
            summary: crate::repo::RepoSummary { staged: 1, ..Default::default() },
            changes,
            ..crate::repo::RepoInfo::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: PathBuf::from("."),
            info: Box::new(info),
        });
        app.commit_list.selection = 0;

        assert_eq!(app.mode, Mode::Inspect);
        assert!(app.is_uncommitted_selected());

        // Press 'c' in Inspect mode
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('c')), 10);
        assert!(handled);

        // Verify we transitioned to CommitInput mode
        assert_eq!(app.mode, Mode::CommitInput);
    }

    #[test]
    fn test_workspace_all_changes_shortcuts() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_workspace_all.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        // Open details Workspace view and focus Unstaged section
        app.mode = Mode::Detail;
        app.detail_tab = 0;
        app.detail_focus = DetailSection::Unstaged;

        let mut changes = crate::repo::WorktreeChanges::default();
        changes.unstaged.push(crate::repo::FileEntry { path: "dummy.txt".to_string(), label: "M" });
        let info = crate::repo::RepoInfo {
            branch: Some("main".to_string()),
            summary: crate::repo::RepoSummary { modified: 1, ..Default::default() },
            changes,
            ..crate::repo::RepoInfo::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: PathBuf::from("."),
            info: Box::new(info),
        });
        app.commit_list.selection = 0;

        assert!(app.is_uncommitted_selected());

        // Press 'X' to discard all changes
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('X')), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::DiscardChangesConfirm);
        assert_eq!(app.discard_target.as_ref().unwrap().0, "All Changes");

        // Cancel discard all
        app.cancel_discard_changes();
        assert_eq!(app.mode, Mode::Detail);
    }

    #[test]
    fn test_inspect_workspace_all_changes_shortcuts() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_inspect_workspace_all.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        // Open details Inspect view and focus Unstaged section
        app.mode = Mode::Inspect;
        app.detail_tab = 0;
        app.detail_focus = DetailSection::Unstaged;

        let mut changes = crate::repo::WorktreeChanges::default();
        changes.unstaged.push(crate::repo::FileEntry { path: "dummy.txt".to_string(), label: "M" });
        let info = crate::repo::RepoInfo {
            branch: Some("main".to_string()),
            summary: crate::repo::RepoSummary { modified: 1, ..Default::default() },
            changes,
            ..crate::repo::RepoInfo::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: PathBuf::from("."),
            info: Box::new(info),
        });
        app.commit_list.selection = 0;

        assert!(app.is_uncommitted_selected());

        // Press 'X' to discard all changes in Inspect mode
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('X')), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::DiscardChangesConfirm);
        assert_eq!(app.discard_target.as_ref().unwrap().0, "All Changes");

        // Cancel discard all and reset to Inspect mode
        app.cancel_discard_changes();
        app.mode = Mode::Inspect;

        // Press 'a' (stage all) on Unstaged focus
        app.detail_focus = DetailSection::Unstaged;
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('a')), 10);
        assert!(handled);

        // Press 'a' (unstage all) on Staged focus
        app.detail_focus = DetailSection::Staged;
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('a')), 10);
        assert!(handled);
    }

    #[test]
    fn test_workspace_all_changes_focus_transitions() {
        let mut temp_path = std::env::temp_dir();
        temp_path.push(format!(
            "gitwig_test_app_all_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&temp_path).unwrap();
        let repo = git2::Repository::init(&temp_path).unwrap();

        // Configure author
        let mut config_git = repo.config().unwrap();
        config_git.set_str("user.name", "Test User").unwrap();
        config_git.set_str("user.email", "test@example.com").unwrap();

        // Create initial commit so we have a HEAD
        let file_path = temp_path.join("file.txt");
        std::fs::write(&file_path, "initial").unwrap();

        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let mut app = App::new(config, temp_path.join("config.toml"));

        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: temp_path.clone(),
            info: Box::new(crate::repo::RepoInfo::default()),
        });

        // 1. Stage All Focus Transition (Unstaged -> Staged)
        app.detail_focus = DetailSection::Unstaged;
        app.stage_all_changes();
        assert_eq!(app.detail_focus, DetailSection::Staged);

        // 2. Unstage All Focus Transition (Staged -> Unstaged)
        app.detail_focus = DetailSection::Staged;
        app.unstage_all_changes();
        assert_eq!(app.detail_focus, DetailSection::Unstaged);

        let _ = std::fs::remove_dir_all(&temp_path);
    }

    #[test]
    fn test_workspace_tab_focus_cycle_skips_empty_panels() {
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_cycle.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        // 1. Uncommitted selected, Staged is not empty, Unstaged is empty
        app.mode = Mode::Detail;
        app.detail_tab = 0;
        app.detail_focus = DetailSection::Commits;

        let mut changes = crate::repo::WorktreeChanges::default();
        changes
            .staged
            .push(crate::repo::FileEntry { path: "staged_file.txt".to_string(), label: "M" });
        // Unstaged is empty
        let info = crate::repo::RepoInfo {
            branch: Some("main".to_string()),
            changes,
            ..crate::repo::RepoInfo::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: PathBuf::from("."),
            info: Box::new(info),
        });
        app.commit_list.selection = 0; // index 0 is "<uncommitted>"

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
        app.commit_list.selection = 1; // Not uncommitted

        let empty_info = crate::repo::RepoInfo {
            branch: Some("main".to_string()),
            ..crate::repo::RepoInfo::default()
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
    fn test_git_app_shortcut_triggers_pending() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_git_app.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        assert!(!app.pending_git_app);

        // Pressing 'g' triggers pending_git_app
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('g')), 10);
        assert!(handled);
        assert!(app.pending_git_app);
    }

    #[test]
    fn test_files_fzf_shortcut_triggers_pending() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_files_fzf.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);
        app.mode = Mode::Detail;
        app.detail_tab = 1; // Files tab
        app.detail_focus = DetailSection::Files;

        assert!(!app.pending_files_fzf);

        // Pressing 'f' triggers pending_files_fzf when in files tab
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('f')), 10);
        assert!(handled);
        assert!(app.pending_files_fzf);
    }

    #[test]
    fn test_logs_search_picker_flow() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_logs_search.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);
        app.mode = Mode::Detail;
        app.detail_tab = 0; // Workspace tab
        app.detail_focus = DetailSection::Commits;

        // 1. Press 'f' to open search column picker
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('f')), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::SearchColumnPicker);
        assert_eq!(app.search_column_selection, 0);

        // 2. Select down
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
        assert_eq!(app.search_column_selection, 1);

        // 3. Toggle column message (initially true, should become false)
        assert!(app.search_columns_message);
        crate::input::handle_key(&mut app, key_event(KeyCode::Char(' ')), 10);
        assert!(!app.search_columns_message);

        // 4. Press Enter to transition to LogsSearchInput
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert_eq!(app.mode, Mode::LogsSearchInput);
        assert!(app.in_logs_ui);

        let mock_info = crate::repo::RepoInfo {
            branch: Some("main".to_string()),
            commits: vec![
                crate::repo::CommitEntry {
                    id: "1234567".to_string(),
                    oid: "1234567890abcdef1234567890abcdef12345678".to_string(),
                    summary: "first test".to_string(),
                    author: "test author 1".to_string(),
                    when: "today".to_string(),
                    date: "today".to_string(),
                    refs: vec![],
                    message: "msg".to_string(),
                    files: vec![],
                    signature_status: "N".to_string(),
                },
                crate::repo::CommitEntry {
                    id: "2234567".to_string(),
                    oid: "2234567890abcdef1234567890abcdef12345678".to_string(),
                    summary: "no match".to_string(),
                    author: "author 1".to_string(),
                    when: "today".to_string(),
                    date: "today".to_string(),
                    refs: vec![],
                    message: "msg".to_string(),
                    files: vec![],
                    signature_status: "N".to_string(),
                },
                crate::repo::CommitEntry {
                    id: "2345678".to_string(),
                    oid: "234567890abcdef1234567890abcdef12345678a".to_string(),
                    summary: "second test".to_string(),
                    author: "test author 2".to_string(),
                    when: "today".to_string(),
                    date: "today".to_string(),
                    refs: vec![],
                    message: "msg".to_string(),
                    files: vec![],
                    signature_status: "N".to_string(),
                },
                crate::repo::CommitEntry {
                    id: "3234567".to_string(),
                    oid: "3234567890abcdef1234567890abcdef12345678".to_string(),
                    summary: "no match".to_string(),
                    author: "author 1".to_string(),
                    when: "today".to_string(),
                    date: "today".to_string(),
                    refs: vec![],
                    message: "msg".to_string(),
                    files: vec![],
                    signature_status: "N".to_string(),
                },
                crate::repo::CommitEntry {
                    id: "4234567".to_string(),
                    oid: "4234567890abcdef1234567890abcdef12345678".to_string(),
                    summary: "third test".to_string(),
                    author: "test author 1".to_string(),
                    when: "today".to_string(),
                    date: "today".to_string(),
                    refs: vec![],
                    message: "msg".to_string(),
                    files: vec![],
                    signature_status: "N".to_string(),
                },
            ],
            ..crate::repo::RepoInfo::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: std::path::PathBuf::from("."),
            info: Box::new(mock_info),
        });

        // 5. Input search query characters and hit Enter
        crate::input::handle_key(&mut app, key_event(KeyCode::Char('t')), 10);
        crate::input::handle_key(&mut app, key_event(KeyCode::Char('e')), 10);
        crate::input::handle_key(&mut app, key_event(KeyCode::Char('s')), 10);
        crate::input::handle_key(&mut app, key_event(KeyCode::Char('t')), 10);
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);

        assert_eq!(app.mode, Mode::Logs);
        assert_eq!(app.commit_list.search_query.as_deref(), Some("test"));
        assert_eq!(app.commit_total(), 5);

        // Test scrolling/navigation (should only jump between matches: 0, 2, 4)
        assert_eq!(app.commit_list.selection, 0); // starts at 0 (which is a match)
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
        assert_eq!(app.commit_list.selection, 2); // skips non-match at index 1
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
        assert_eq!(app.commit_list.selection, 4); // skips non-match at index 3
        crate::input::handle_key(&mut app, key_event(KeyCode::Down), 10);
        assert_eq!(app.commit_list.selection, 4); // remains at last match

        crate::input::handle_key(&mut app, key_event(KeyCode::PageUp), 10);
        assert_eq!(app.commit_list.selection, 0); // jumps back to first match
        crate::input::handle_key(&mut app, key_event(KeyCode::PageDown), 10);
        assert_eq!(app.commit_list.selection, 4); // jumps back to last match

        crate::input::handle_key(&mut app, key_event(KeyCode::Up), 10);
        assert_eq!(app.commit_list.selection, 2);
        crate::input::handle_key(&mut app, key_event(KeyCode::Up), 10);
        assert_eq!(app.commit_list.selection, 0);

        // 6. Test match helper
        let matching_commit = crate::repo::CommitEntry {
            id: "1234567".to_string(),
            oid: "1234567890abcdef1234567890abcdef12345678".to_string(),
            summary: "a test message".to_string(), // message column disabled, so shouldn't match message!
            author: "test author".to_string(),     // author column enabled, should match author!
            when: "today".to_string(),
            date: "today".to_string(),
            refs: vec![],
            message: "message body".to_string(),
            files: vec![],
            signature_status: "N".to_string(),
        };
        assert!(app.commit_matches_query(&matching_commit));

        let non_matching_commit = crate::repo::CommitEntry {
            id: "1234567".to_string(),
            oid: "1234567890abcdef1234567890abcdef12345678".to_string(),
            summary: "a test message".to_string(), // message column disabled, message has test but is ignored!
            author: "other author".to_string(),    // author doesn't match!
            when: "today".to_string(),
            date: "today".to_string(),
            refs: vec![],
            message: "message body".to_string(),
            files: vec![],
            signature_status: "N".to_string(),
        };
        assert!(!app.commit_matches_query(&non_matching_commit));

        // Test entering inspect UI via Enter key when in Mode::Logs
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert_eq!(app.mode, Mode::Inspect);
        assert!(app.in_logs_ui);

        // Press 'q' to go back to Mode::Logs
        crate::input::handle_key(&mut app, key_event(KeyCode::Char('q')), 10);
        assert_eq!(app.mode, Mode::Logs);
        assert!(app.in_logs_ui);

        // Press Enter again to transition to Mode::Inspect
        crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert_eq!(app.mode, Mode::Inspect);
        assert!(app.in_logs_ui);

        // Press Esc to go back to Mode::Logs
        crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
        assert_eq!(app.mode, Mode::Logs);
        assert!(app.in_logs_ui);

        // 7. Press Esc to go back to workspace
        crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
        assert_eq!(app.mode, Mode::Detail);
        assert!(!app.in_logs_ui);
        assert!(app.commit_list.search_query.is_none());
    }

    #[test]
    fn test_detail_view_sync_on_tab_change_and_refresh() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec![".".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_sync.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);
        app.mode = Mode::Detail;
        app.detail_tab = 0;

        let mock_info = crate::repo::RepoInfo {
            branch: Some("mock_branch_name_test_xyz".to_string()),
            ..crate::repo::RepoInfo::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: std::path::PathBuf::from("."),
            info: Box::new(mock_info),
        });

        // 1. Simulate tab switch (e.g. key '2') with resync_on_tab_change = false
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('2')), 10);
        assert!(handled);
        assert_eq!(app.detail_tab, 1);
        assert!(app.current_detail.is_some());
        if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
            assert_eq!(info.branch.as_deref(), Some("mock_branch_name_test_xyz"));
        } else {
            panic!("Expected Repo detail");
        }

        // 2. Simulate tab switch (e.g. key '3') with resync_on_tab_change = true
        app.config.resync_on_tab_change = true;
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('3')), 10);
        assert!(handled);
        assert_eq!(app.detail_tab, 2);
        assert!(app.current_detail.is_some());

        // Wait and process the async message
        let (path, detail) = app.detail_rx.recv().unwrap();
        assert_eq!(Some(&path), app.loading_repo_path.as_ref());
        app.apply_detail_snapshot(detail);
        app.loading_repo_path = None;

        if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
            assert_ne!(info.branch.as_deref(), Some("mock_branch_name_test_xyz"));
        } else {
            panic!("Expected Repo detail");
        }

        // Reset to mock info for manual refresh test
        let mock_info_2 = crate::repo::RepoInfo {
            branch: Some("mock_branch_name_test_xyz".to_string()),
            ..crate::repo::RepoInfo::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: std::path::PathBuf::from("."),
            info: Box::new(mock_info_2),
        });

        // 3. Press 'R' to refresh/resync manually (should resync even if resync_on_tab_change is false)
        app.config.resync_on_tab_change = false;
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('R')), 10);
        assert!(handled);
        assert_eq!(app.status_message.as_deref(), Some("Refreshed"));

        // Wait and process the async message
        let (path, detail) = app.detail_rx.recv().unwrap();
        assert_eq!(Some(&path), app.loading_repo_path.as_ref());
        app.apply_detail_snapshot(detail);
        app.loading_repo_path = None;

        if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
            assert_ne!(info.branch.as_deref(), Some("mock_branch_name_test_xyz"));
        } else {
            panic!("Expected Repo detail");
        }
    }

    #[test]
    fn test_branch_and_tag_checkout_confirmation() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec![".gitwig".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_checkout.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);
        app.mode = Mode::Detail;
        app.detail_tab = 3; // branches tab
        app.detail_focus = DetailSection::LocalBranches;

        let mock_info = crate::repo::RepoInfo {
            branch: Some("main".to_string()),
            local_branches: crate::repo::TabData::Loaded(vec![
                crate::repo::BranchInfo {
                    name: "main".to_string(),
                    is_head: true,
                    short_sha: "".to_string(),
                    short_message: "".to_string(),
                },
                crate::repo::BranchInfo {
                    name: "feature-branch".to_string(),
                    is_head: false,
                    short_sha: "".to_string(),
                    short_message: "".to_string(),
                },
            ]),
            remote_branches: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
                name: "origin/feature-branch".to_string(),
                is_head: false,
                short_sha: "".to_string(),
                short_message: "".to_string(),
            }]),
            local_tags: crate::repo::TabData::Loaded(vec![crate::repo::BranchInfo {
                name: "v1.0.0".to_string(),
                is_head: false,
                short_sha: "".to_string(),
                short_message: "".to_string(),
            }]),
            ..crate::repo::RepoInfo::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: std::path::PathBuf::from("."),
            info: Box::new(mock_info),
        });

        // Select the non-head local branch "feature-branch" (index 1)
        app.branch_list.local_branch_selection = 1;

        // Pressing Enter should request confirmation
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::BranchCheckoutConfirm);
        assert_eq!(app.branch_action_target, Some(("feature-branch".to_string(), false)));

        // Cancel branch checkout confirmation via 'n'
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('n')), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::Detail);
        assert_eq!(app.branch_action_target, None);

        // Request again
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::BranchCheckoutConfirm);

        // Confirm branch checkout confirmation via 'y' (it will fail to checkout in dummy/test repo path, but checks handler path)
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('y')), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::Detail);
        assert_eq!(app.branch_action_target, None);

        // Switch to Tags tab (detail_tab = 4)
        app.detail_tab = 4;
        app.tag_list.local_tag_selection = 0;

        // Pressing Enter should request tag checkout confirmation
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::TagCheckoutConfirm);
        assert_eq!(app.tag_checkout_target, Some("v1.0.0".to_string()));

        // Cancel tag checkout confirmation via Esc
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::Detail);
        assert_eq!(app.tag_checkout_target, None);
    }

    #[test]
    fn test_repo_search_filtering() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["z_repo".to_string(), "a_repo".to_string(), "m_repo".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_search.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        // Initially we should have 3 items
        assert_eq!(app.get_items_len(), 3);
        assert_eq!(app.get_filtered_items().len(), 3);

        // Press 'f' to enter search mode
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('f')), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::RepoSearchInput);

        // Type 'a'
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('a')), 10);
        assert!(handled);
        assert_eq!(app.repo_search_query.as_deref(), Some("a"));
        assert_eq!(app.get_items_len(), 1);
        assert_eq!(app.get_filtered_items()[0].1, &"a_repo".to_string());

        // Press Enter to confirm/exit search input mode
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Enter), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.repo_search_query.as_deref(), Some("a"));

        // Press Esc in normal mode to clear the filter
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
        assert!(handled);
        assert_eq!(app.repo_search_query, None);
        assert_eq!(app.get_items_len(), 3);
    }

    #[test]
    fn test_normal_mode_right_arrow_detail() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_right_arrow.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        assert_eq!(app.mode, Mode::Normal);

        // Press Right arrow key in Normal mode
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Right), 10);
        assert!(handled);

        // Verify we opened detail view in loading state
        assert_eq!(app.mode, Mode::Detail);
        assert_eq!(app.loading_repo_path.as_deref(), Some("a_repo"));

        // Wait for background thread message
        let (path, detail) = app.detail_rx.recv().unwrap();
        assert_eq!(path, "a_repo");

        // Manually apply to verify state transition
        app.current_detail = Some(detail);
        app.loading_repo_path = None;

        assert_eq!(app.loading_repo_path, None);
        assert!(app.current_detail.is_some());
    }

    #[test]
    fn test_inspect_full_screen_diff_toggle() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_full_diff.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        // Transition to Mode::Inspect and focus StagingDetails
        app.mode = Mode::Inspect;
        app.detail_focus = DetailSection::StagingDetails;
        app.inspect_full_diff = false;

        // Press Right arrow
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Right), 10);
        assert!(handled);
        assert!(app.inspect_full_diff);

        // Press Left arrow to exit full diff
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Left), 10);
        assert!(handled);
        assert!(!app.inspect_full_diff);

        // Press Right arrow again to enter full diff
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Right), 10);
        assert!(handled);
        assert!(app.inspect_full_diff);

        // Press Esc to exit full diff
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
        assert!(handled);
        assert!(!app.inspect_full_diff);
        assert_eq!(app.mode, Mode::Inspect); // Still in Inspect mode!
    }

    #[test]
    fn test_files_tab_full_screen_toggle() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec!["a_repo".to_string()],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_files_full.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        // Transition to Mode::Detail, select tab 1 (Files) and focus FileContent
        app.mode = Mode::Detail;
        app.detail_tab = 1;
        app.detail_focus = DetailSection::FileContent;
        app.inspect_full_diff = false;

        // Press Right arrow on FileContent
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Right), 10);
        assert!(handled);
        assert!(app.inspect_full_diff);

        // Press Left arrow to exit full screen
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Left), 10);
        assert!(handled);
        assert!(!app.inspect_full_diff);

        // Press Right arrow again
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Right), 10);
        assert!(handled);
        assert!(app.inspect_full_diff);

        // Press Esc to exit full screen
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
        assert!(handled);
        assert!(!app.inspect_full_diff);
        assert_eq!(app.mode, Mode::Detail); // Still in Detail mode!
    }

    #[test]
    fn test_fzf_missing_flow() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());
        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_fzf_missing.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        // Case 1: fzf is missing
        app.force_fzf_missing = Some(true);
        app.mode = Mode::Normal;

        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('a')), 10);
        assert!(handled);
        assert!(!app.pending_fzf);
        assert_eq!(app.mode, Mode::Adding);
        assert!(app.error_message.is_none());

        // Esc should cancel Adding mode
        let handled_dismiss = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
        assert!(handled_dismiss);
        assert_eq!(app.mode, Mode::Normal);

        // A -> should fallback to BulkAddInput (manual typing)
        let handled_bulk = crate::input::handle_key(&mut app, key_event(KeyCode::Char('A')), 10);
        assert!(handled_bulk);
        assert!(!app.pending_bulk_fzf);
        assert_eq!(app.mode, Mode::BulkAddInput);
        assert!(app.error_message.is_none());

        // Case 2: fzf is installed
        app.force_fzf_missing = Some(false);
        app.mode = Mode::Normal;
        let handled_add = crate::input::handle_key(&mut app, key_event(KeyCode::Char('a')), 10);
        assert!(handled_add);
        assert!(app.pending_fzf);
        assert!(app.error_message.is_none());

        app.mode = Mode::Normal;
        let handled_bulk_add =
            crate::input::handle_key(&mut app, key_event(KeyCode::Char('A')), 10);
        assert!(handled_bulk_add);
        assert!(app.pending_bulk_fzf);
        assert!(app.error_message.is_none());
    }

    #[test]
    fn test_initial_setup_and_migration() {
        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let unique_id =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("gitwig_test_migration_{}", unique_id));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let temp_path = temp_dir.join("config.toml");

        // Save initial config
        crate::config::save_config(&config, &temp_path).unwrap();

        // 1. First run: version file does not exist.
        {
            let app = App::new(config.clone(), temp_path.clone());
            let version_path = temp_dir.join(".version");
            assert!(version_path.exists());
            let written_version = std::fs::read_to_string(&version_path).unwrap();
            assert_eq!(written_version.trim(), env!("CARGO_PKG_VERSION"));
            assert_eq!(
                app.status_message,
                Some(format!("Welcome to Gitwig v{}!", env!("CARGO_PKG_VERSION")))
            );
        }

        // 2. Second run: version file matches current version.
        {
            let app = App::new(config.clone(), temp_path.clone());
            // No new status message should be set
            assert!(app.status_message.is_none());
        }

        // 3. Update run: version file has older version.
        {
            let version_path = temp_dir.join(".version");
            std::fs::write(&version_path, "0.1.0").unwrap();

            let app = App::new(config.clone(), temp_path.clone());
            // Check status message
            assert_eq!(
                app.status_message,
                Some(format!(
                    "Gitwig updated to v{}! Configuration verified and backed up.",
                    env!("CARGO_PKG_VERSION")
                ))
            );
            // Check config backup exists
            let backup_path = temp_path.with_extension("toml.bak");
            assert!(backup_path.exists());
            // Check version was updated
            let written_version = std::fs::read_to_string(&version_path).unwrap();
            assert_eq!(written_version.trim(), env!("CARGO_PKG_VERSION"));
        }

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_about_popup_flow() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_about.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);
        let key_event = |code: KeyCode| KeyEvent::new(code, KeyModifiers::empty());

        // Assert initial mode is Normal
        assert_eq!(app.mode, Mode::Normal);

        // Open about popup
        app.open_about();
        assert_eq!(app.mode, Mode::About);

        // Close about popup
        app.close_dialog();
        assert_eq!(app.mode, Mode::Normal);

        // Test key inputs via handle_key
        // 1. In Normal mode, pressing 'v' should open about popup
        app.mode = Mode::Normal;
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('v')), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::About);

        // 2. In About mode, pressing 'v' should close it
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('v')), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::Normal);

        // 3. In Normal mode, pressing 'V' should open about popup
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('V')), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::About);

        // 4. In About mode, pressing 'Esc' should close it
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Esc), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::Normal);

        // 5. In Normal mode, pressing 'v' then closing with 'q'
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('v')), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::About);
        let handled = crate::input::handle_key(&mut app, key_event(KeyCode::Char('q')), 10);
        assert!(handled);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn test_tag_fetch_attempt_and_dismiss_flow() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_tag_fetch.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        let mock_info = crate::repo::RepoInfo {
            branch: Some("main".to_string()),
            summary: crate::repo::RepoSummary {
                branch: Some("main".to_string()),
                staged: 0,
                modified: 0,
                untracked: 0,
                conflicted: 0,
                ahead: 0,
                behind: 0,
            },
            changes: crate::repo::WorktreeChanges {
                staged: vec![],
                unstaged: vec![],
                conflicted: vec![],
                untracked: vec![],
            },
            remotes: crate::repo::TabData::Loaded(vec![crate::repo::RemoteInfo {
                name: "origin".to_string(),
                url: "git@github.com:tareqmy/gitwig.git".to_string(),
                push_url: None,
                refspecs: vec![],
            }]),
            ..crate::repo::RepoInfo::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: std::path::PathBuf::from("."),
            info: Box::new(mock_info),
        });

        // Initially remote_tags_attempted is false
        if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
            assert!(!info.remote_tags_attempted);
        }

        // 1. Switch to tab 4 (Tags tab) and trigger set_default_focus_for_tab
        app.detail_tab = 4;
        app.set_default_focus_for_tab();

        // Should start fetching and set attempted flag to true
        assert!(app.fetching);
        if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
            assert!(info.remote_tags_attempted);
        }

        // 2. Receive error from the background thread
        app.tx
            .send("REMOTE_TAGS_ERR:Failed to get remote tags: network timeout".to_string())
            .unwrap();

        // Process message in receiver
        if let Ok(msg) = app.rx.try_recv() {
            if let Some(err_msg) = msg.strip_prefix("REMOTE_TAGS_ERR:") {
                app.set_error(err_msg.to_string());
                app.fetching = false;
            }
        }

        // Verify fetching is false and error popup is shown
        assert!(!app.fetching);
        assert_eq!(
            app.error_message.as_deref(),
            Some("Failed to get remote tags: network timeout")
        );

        // 3. Trigger set_default_focus_for_tab again.
        // It should NOT call fetch_remote_tags again since attempted is true.
        app.set_default_focus_for_tab();
        assert!(!app.fetching);

        // 4. Test mouse click to dismiss error popup
        let mouse_event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, mouse_event);

        // Error message should be dismissed (None)
        assert_eq!(app.error_message, None);
    }

    #[test]
    fn test_tag_push_all_confirmation_flow() {
        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_tag_push_all.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        // 1. Single Remote Scenario
        let mock_info_single = crate::repo::RepoInfo {
            remotes: crate::repo::TabData::Loaded(vec![crate::repo::RemoteInfo {
                name: "origin".to_string(),
                url: "git@github.com:tareqmy/gitwig.git".to_string(),
                push_url: None,
                refspecs: vec![],
            }]),
            ..crate::repo::RepoInfo::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: std::path::PathBuf::from("."),
            info: Box::new(mock_info_single),
        });

        // Request tag push all
        app.request_tag_push_all();
        // Should go directly to TagPushAllConfirm
        assert_eq!(app.mode, Mode::TagPushAllConfirm);
        assert_eq!(app.remote_action_target.as_deref(), Some("origin"));

        // Cancel
        app.cancel_tag_push_all();
        assert_eq!(app.mode, Mode::Detail);
        assert_eq!(app.remote_action_target, None);

        // 2. Multi-Remote Scenario
        let mock_info_multi = crate::repo::RepoInfo {
            remotes: crate::repo::TabData::Loaded(vec![
                crate::repo::RemoteInfo {
                    name: "origin".to_string(),
                    url: "git@github.com:tareqmy/gitwig.git".to_string(),
                    push_url: None,
                    refspecs: vec![],
                },
                crate::repo::RemoteInfo {
                    name: "upstream".to_string(),
                    url: "git@github.com:parent/gitwig.git".to_string(),
                    push_url: None,
                    refspecs: vec![],
                },
            ]),
            ..crate::repo::RepoInfo::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: std::path::PathBuf::from("."),
            info: Box::new(mock_info_multi),
        });

        // Request tag push all
        app.request_tag_push_all();
        // Should open remote picker
        assert_eq!(app.mode, Mode::RemotePicker);
        assert_eq!(app.remote_picker_action, Some(RemotePickerAction::PushAllTags));

        // Confirm selection in remote picker (index 1 is upstream)
        app.remote_picker_selection = 1;
        app.confirm_remote_picker();

        // Should transition to TagPushAllConfirm and set target to upstream
        assert_eq!(app.mode, Mode::TagPushAllConfirm);
        assert_eq!(app.remote_action_target.as_deref(), Some("upstream"));

        // Confirm push
        app.confirm_tag_push_all();
        // Should trigger pushing, transition to Detail mode and clear target
        assert_eq!(app.mode, Mode::Detail);
        assert_eq!(app.remote_action_target, None);
    }

    #[test]
    fn test_detail_cache_ttl_behavior() {
        let temp_dir = std::env::temp_dir();
        let repo_path = temp_dir.join("test_cache_repo");
        let _ = std::fs::remove_dir_all(&repo_path);
        std::fs::create_dir_all(&repo_path).unwrap();

        // Initialize App
        let config = Config {
            items: vec![repo_path.to_string_lossy().to_string()],
            poll_interval_ms: 100,
            max_commits: 200,
            graph_max_commits: 1000,

            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme_name: "default".to_string(),
            theme: ThemeConfig::default(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: true,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
        };

        let mut app = App::new(config, PathBuf::from(""));

        // Create a mock detail snapshot
        let mock_detail = crate::repo::ItemDetail::Repo {
            resolved: repo_path.clone(),
            info: Box::new(crate::repo::RepoInfo {
                commits: vec![],
                files: crate::repo::TabData::Loaded(vec!["file1.txt".to_string()]),
                ..crate::repo::RepoInfo::default()
            }),
        };

        // 1. Manually add to cache
        app.detail_cache.insert(
            repo_path.to_string_lossy().to_string(),
            DetailCache { detail: mock_detail.clone(), loaded_at: std::time::Instant::now() },
        );

        // 2. Trigger open_detail on this repository (it will load from cache immediately)
        app.open_detail();

        // loading_repo_path should be None because it loaded from cache silently!
        assert!(app.loading_repo_path.is_none());
        assert!(app.current_detail.is_some());

        // Verify loaded files tab data is preserved
        if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
            assert_eq!(info.files.as_slice(), &["file1.txt".to_string()]);
        }

        // Clean up
        let _ = std::fs::remove_dir_all(&repo_path);
    }

    #[test]
    fn test_tab_ttl_behavior() {
        let temp_dir = std::env::temp_dir();
        let repo_path = temp_dir.join("test_tab_ttl_repo");
        let _ = std::fs::remove_dir_all(&repo_path);
        std::fs::create_dir_all(&repo_path).unwrap();

        // Initialize App with a short Tab TTL (e.g. 1s)
        let config = Config {
            items: vec![repo_path.to_string_lossy().to_string()],
            poll_interval_ms: 100,
            max_commits: 200,
            graph_max_commits: 1000,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 1, // 1s TTL
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme_name: "default".to_string(),
            theme: ThemeConfig::default(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: true,
            resync_on_tab_change: false,
        };

        let mut app = App::new(config, PathBuf::from(""));

        // Set up mock current detail
        let mock_info = crate::repo::RepoInfo {
            commits: vec![],
            files: crate::repo::TabData::Loaded(vec!["file1.txt".to_string()]),
            tab_loaded_at: [None; 8],
            tab_loading: [false; 8],
            ..crate::repo::RepoInfo::default()
        };
        app.current_detail = Some(crate::repo::ItemDetail::Repo {
            resolved: repo_path.clone(),
            info: Box::new(mock_info),
        });

        // 1. Initial trigger when NotLoaded
        // Reset state to NotLoaded
        if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &mut app.current_detail {
            info.files = crate::repo::TabData::NotLoaded;
        }
        app.trigger_tab_load_if_needed(1);
        if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
            assert!(info.tab_loading[1]);
            assert!(info.files.is_loading());
        }

        // 2. Receive loaded payload simulation
        if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &mut app.current_detail {
            info.tab_loading[1] = false;
            info.tab_loaded_at[1] =
                Some(std::time::Instant::now() - std::time::Duration::from_secs(5)); // Mark loaded 5s ago (stale)
            info.files = crate::repo::TabData::Loaded(vec!["file_refreshed.txt".to_string()]);
        }

        // 3. Trigger tab load when it is stale (stale-while-revalidate)
        app.trigger_tab_load_if_needed(1);
        if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
            // Should be loading in the background (tab_loading is true)
            assert!(info.tab_loading[1]);
            // But info.files state should still be TabData::Loaded! (no spinner)
            assert!(matches!(info.files, crate::repo::TabData::Loaded(_)));
            assert_eq!(info.files.as_slice(), &["file_refreshed.txt".to_string()]);
        }

        // Clean up
        let _ = std::fs::remove_dir_all(&repo_path);
    }

    #[test]
    fn test_commit_popup_mouse_resize() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::layout::Rect;

        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let temp_path = std::env::temp_dir().join("gitwig_test_config_commit_resize.toml");
        let _guard = TestFileGuard { path: temp_path.clone() };
        let mut app = App::new(config, temp_path);

        app.mode = Mode::CommitInput;
        app.commit_popup_width_pct = 80;
        app.commit_popup_height_pct = 45;

        // Mock detail_areas
        // Parent area is 100x100
        // Popup area with 80% width and 45% height is centered:
        // width = 80, height = 45. x = 10, y = 27
        app.detail_areas.commit_popup_parent = Some(Rect::new(0, 0, 100, 100));
        app.detail_areas.commit_popup = Some(Rect::new(10, 27, 80, 45));

        // Click on the right border (pos.x = 89, pos.y = 50)
        let down_event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 89,
            row: 50,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, down_event);
        assert_eq!(app.active_drag_splitter, Some(Splitter::CommitPopupWidth));

        // Drag right border to column 95 -> new half_width = |95 - 50| = 45 -> new_width = 90 -> 90%
        let drag_event = MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 95,
            row: 50,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, drag_event);
        assert_eq!(app.commit_popup_width_pct, 90);

        // Release mouse
        let up_event = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 95,
            row: 50,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        crate::input::handle_mouse(&mut app, up_event);
        assert_eq!(app.active_drag_splitter, None);
    }

    #[test]
    fn test_yank_selected_commit_hash() {
        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let mut app = App::new(config, PathBuf::from("dummy_path.toml"));

        // Setup mock repo commits
        let mut info = repo::RepoInfo::default();
        info.commits.push(repo::CommitEntry {
            id: "abc1234".to_string(),
            oid: "abc123456789".to_string(),
            author: "Tester".to_string(),
            when: "".to_string(),
            date: "".to_string(),
            summary: "Initial commit".to_string(),
            message: "Initial commit".to_string(),
            refs: vec![],
            files: vec![],
            signature_status: "".to_string(),
        });
        app.current_detail = Some(repo::ItemDetail::Repo {
            resolved: PathBuf::from("/dummy"),
            info: Box::new(info),
        });

        // Select the committed item
        app.commit_list.selection = 0;
        app.detail_tab = 0;

        // Try yanking. Note: since standard clipboards might fail in some test/headless envs,
        // we can test the behavior and see if it sets self.status_message to either success or error.
        app.yank_selected_commit_hash();
        assert!(app.status_message.is_some());
        let msg = app.status_message.as_ref().unwrap();
        assert!(msg.contains("Copied hash abc1234") || msg.contains("Failed to copy"));
    }

    #[test]
    fn test_cherry_pick_destination_branches() {
        let config = Config {
            items: vec![],
            poll_interval_ms: 100,
            max_commits: 0,
            page_size: 10,
            sort_by: SortOrder::Custom,
            visits: HashMap::new(),
            sort_reverse: false,
            pinned: std::collections::HashSet::new(),
            theme: ThemeConfig::default(),
            theme_name: "default".to_string(),
            fzf: FzfConfig::default(),
            git_app: "gitui".to_string(),
            compatibility_mode: false,
            detail_cache_ttl_secs: 30,
            enable_commit_signatures: false,
            tab_ttl_secs: 60,
            resync_on_tab_change: false,
            graph_max_commits: 1000,
        };
        let mut app = App::new(config, PathBuf::from("dummy_path.toml"));

        // Setup mock repo details
        let mut info = repo::RepoInfo { branch: Some("main".to_string()), ..Default::default() };
        info.commits.push(repo::CommitEntry {
            id: "abc1234".to_string(),
            oid: "abc123456789".to_string(),
            author: "Tester".to_string(),
            when: "".to_string(),
            date: "".to_string(),
            summary: "Initial commit".to_string(),
            message: "Initial commit".to_string(),
            refs: vec![],
            files: vec![],
            signature_status: "".to_string(),
        });
        info.local_branches = repo::TabData::Loaded(vec![
            repo::BranchInfo {
                name: "main".to_string(),
                is_head: true,
                short_sha: "abc1234".to_string(),
                short_message: "msg".to_string(),
            },
            repo::BranchInfo {
                name: "feature-1".to_string(),
                is_head: false,
                short_sha: "def5678".to_string(),
                short_message: "msg2".to_string(),
            },
            repo::BranchInfo {
                name: "feature-2".to_string(),
                is_head: false,
                short_sha: "9999999".to_string(),
                short_message: "msg3".to_string(),
            },
        ]);
        app.current_detail = Some(repo::ItemDetail::Repo {
            resolved: PathBuf::from("/dummy"),
            info: Box::new(info),
        });

        // Trigger cherry pick
        app.commit_list.selection = 0;
        app.request_cherry_pick();

        assert_eq!(app.mode, Mode::CherryPickConfirm);
        assert_eq!(app.cherry_pick_dest_branches.len(), 2);
        assert_eq!(app.cherry_pick_dest_branches[0], "feature-1");
        assert_eq!(app.cherry_pick_dest_branches[1], "feature-2");
        assert_eq!(app.cherry_pick_dest_selection, 0);

        // Test navigation
        // Press Down
        let event_down = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Down,
            crossterm::event::KeyModifiers::empty(),
        );
        crate::input::handle_key(&mut app, event_down, 0);
        assert_eq!(app.cherry_pick_dest_selection, 1);

        // Press Down again (should clamp)
        let event_down_again = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Down,
            crossterm::event::KeyModifiers::empty(),
        );
        crate::input::handle_key(&mut app, event_down_again, 0);
        assert_eq!(app.cherry_pick_dest_selection, 1);

        // Press Up
        let event_up = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Up,
            crossterm::event::KeyModifiers::empty(),
        );
        crate::input::handle_key(&mut app, event_up, 0);
        assert_eq!(app.cherry_pick_dest_selection, 0);
    }
}
