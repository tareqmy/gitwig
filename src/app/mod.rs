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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GlobalFilter {
    Dirty,
    Ahead,
    Stale,
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
    /// Showing the stashing UI panel with options and file list.
    StashingUI,
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
    /// Per-file history view.
    FileHistory,
    /// Editing repository labels.
    LabelInput,
    /// Choosing custom settings for the repository inside Overview.
    RepoSettings,
    /// Confirming self-update of the application.
    UpdateConfirm,
    /// Typing a branch name for a new worktree.
    WorktreeAddBranchInput,
    /// Typing a path for a new worktree.
    WorktreeAddPathInput,
    /// Typing a lock reason for a worktree.
    WorktreeLockReasonInput,
    /// Confirming removal options of a worktree.
    WorktreeRemoveConfirm,
    /// Showing the repository overview in a full window popup.
    Overview,
    /// Typing a URL for a new submodule.
    SubmoduleAddUrlInput,
    /// Typing a path for a new submodule.
    SubmoduleAddPathInput,
    /// Confirming deletion of a submodule.
    SubmoduleDeleteConfirm,
    /// Showing the signs and symbols legend popup.
    Legend,
    /// Floating popup with ranked fuzzy matches for repository navigation.
    RepoJump,
    /// Floating popup for built-in repository scanning and selection.
    RepoScanPicker,
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
    Worktrees,
    Submodules,
    Reflog,
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
            Self::Worktrees => Self::Worktrees,
            Self::Submodules => Self::Submodules,
            Self::Reflog => Self::Reflog,
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
            Self::Worktrees => Self::Worktrees,
            Self::Submodules => Self::Submodules,
            Self::Reflog => Self::Reflog,
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
    pub confirm_popup: crate::popups::confirm::ConfirmPopup,
    pub generic_input_popup: crate::popups::commit::GenericInputPopup,
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
    pub legend_scroll: usize,
    pub collapsed_groups: std::collections::HashSet<String>,
    pub repo_jump_selection: usize,
    /// Panel bounding boxes recorded after each draw, used for mouse hit-testing.
    pub detail_areas: DetailAreas,
    /// Main panel item bounding boxes recorded after each draw, used for mouse hit-testing.
    pub main_areas: Vec<Rect>,
    pub global_filter: Option<GlobalFilter>,
    pub global_summary_area: Option<Rect>,

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
    /// Whether the status bar is expanded.
    pub status_expanded: bool,
    /// Whether the settings panel focus is on the sidebar categories.
    pub settings_focus_sidebar: bool,
    /// Sender for background task events.
    pub tx: std::sync::mpsc::Sender<String>,
    /// Receiver for background task events.
    pub rx: std::sync::mpsc::Receiver<String>,
    /// Whether a background fetch is active.
    pub fetching: bool,
    /// Store the latest version if an update is available.
    pub update_available: Option<String>,
    /// Whether the update check was triggered manually.
    pub update_check_manual: bool,
    /// Number of active background (implicit) network actions.
    pub implicit_network_count: usize,
    /// Stored previous mode to restore after confirmation/popups.
    pub previous_mode: Option<Mode>,
    pub scanned_repos: Vec<(String, String)>,
    pub repo_scan_selection: usize,
    pub repo_scan_active: bool,
    pub repo_scan_count: usize,
    /// Row selection index for the repository settings popup.
    pub repo_settings_selected_index: usize,
    /// Whether we are currently text-editing a repository setting.
    pub repo_settings_editing: bool,
    /// Temporary text input buffer for repository settings.
    pub repo_settings_input: String,
    /// Loaded user/default keybindings configuration.
    pub keybindings: crate::keybindings::KeybindingsConfig,
    /// Whether external Git application launch is pending.
    pub pending_git_app: bool,
    /// Whether terminal launch is pending.
    pub pending_terminal: bool,
    /// Whether fzf search launch is pending.
    pub pending_fzf: bool,
    /// Whether bulk fzf search launch is pending.
    pub pending_bulk_fzf: bool,
    /// Whether fzf files search launch is pending.
    pub pending_files_fzf: bool,
    /// Whether terminal editor launch is pending with a file path.
    pub pending_editor_file: Option<String>,
    /// Whether interactive rebase is pending.
    pub pending_interactive_rebase: Option<(PathBuf, String)>,
    /// Repository paths currently being fetched in bulk.
    pub bulk_fetching: std::collections::HashSet<String>,
    /// Results of the latest bulk fetch (path -> Ok(msg) or Err(err)).
    pub bulk_fetch_results: std::collections::HashMap<String, Result<String, String>>,
    /// Repository paths currently selected for batch operations.
    pub multi_selected: std::collections::HashSet<String>,
    /// Whether we are currently viewing logs UI.
    pub in_logs_ui: bool,
    /// Cached resolved ThemeConfigs for repositories.
    /// Keyed by repository absolute path.
    pub repo_theme_cache: std::collections::HashMap<String, crate::config::ThemeConfig>,
    /// Whether we are in full-screen diff mode under inspect view.
    pub inspect_full_diff: bool,
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
    /// Option to stash untracked files.
    pub stash_untracked: bool,
    /// Option to keep the index after stashing.
    pub stash_keep_index: bool,
    /// Track the targeted stash by (commit_id, message) for validation on confirm
    pub stash_action_target: Option<(String, String)>,
    /// Selection index in the Stashing UI file list.
    pub stashing_ui_selection: usize,
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
    pub file_history_revisions: Vec<repo::FileRevision>,
    pub file_history_selection: usize,
    pub file_history_diff: Vec<repo::DiffLine>,
    pub file_history_diff_scroll: usize,
    pub file_history_path: String,
    pub file_history_focus: usize,
    pub worktree_selection: usize,
    pub submodule_selection: usize,
    pub reflog_selection: usize,
    pub worktree_add_branch: String,
    pub worktree_add_path: String,
    pub worktree_lock_reason: String,
    pub worktree_remove_delete_folder: bool,
    pub worktree_remove_force: bool,
    pub submodule_add_url: String,
    pub submodule_add_path: String,
    pub submodule_delete_target: Option<String>,
    pub cpu_tracker: std::sync::Mutex<Option<(f64, std::time::Instant, f64, f64)>>,
    pub watcher: Option<notify::RecommendedWatcher>,
    pub status_refresh_tx: std::sync::mpsc::Sender<Vec<(usize, String, ItemStatus)>>,
    pub status_refresh_rx: std::sync::mpsc::Receiver<Vec<(usize, String, ItemStatus)>>,
    pub last_background_refresh: std::time::Instant,
    pub background_refresh_running: bool,
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

mod actions;
mod git;
mod navigation;
pub use navigation::HomeRow;
#[cfg(test)]
mod tests;
mod workspace;

impl App {
    pub fn resolve_repo_themes(&mut self) {
        self.repo_theme_cache.clear();
        let themes_dir = self.config_path.parent().unwrap_or(&self.config_path).join("themes");
        if !themes_dir.exists() {
            return;
        }
        for (repo_path, repo_cfg) in &self.config.repo_configs {
            if let Some(theme_name) = &repo_cfg.theme {
                let theme_path = themes_dir.join(format!("{}.theme", theme_name));
                if theme_path.exists() {
                    if let Ok(theme_contents) = std::fs::read_to_string(&theme_path) {
                        if let Ok(theme) =
                            toml::from_str::<crate::config::ThemeConfig>(&theme_contents)
                        {
                            self.repo_theme_cache.insert(repo_path.clone(), theme);
                        }
                    }
                }
            }
        }
    }

    pub fn setup_watcher(&mut self) {
        use notify::{RecursiveMode, Watcher};

        self.watcher = None;

        let tx = self.tx.clone();
        let mut watcher =
            match notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| {
                if let Ok(event) = res {
                    for path in event.paths {
                        let path_str = path.to_string_lossy();
                        let clean_path =
                            path_str.replace("\\.git\\", "/.git/").replace("\\.git", "/.git");
                        if let Some(pos) = clean_path.find("/.git") {
                            let repo_root = &clean_path[..pos];
                            if !path_str.ends_with(".lock")
                                && (path_str.contains("/.git/refs/")
                                    || path_str.ends_with("/.git/index")
                                    || path_str.ends_with("/.git/HEAD"))
                            {
                                let _ = tx.send(format!("REFRESH_REPO:{}", repo_root));
                            }
                        }
                    }
                }
            }) {
                Ok(w) => w,
                Err(e) => {
                    crate::debug_log::warn(format!("Failed to initialize file watcher: {}", e));
                    return;
                }
            };

        for item in &self.config.items {
            let canon = match std::fs::canonicalize(item) {
                Ok(c) => c,
                Err(_) => PathBuf::from(item),
            };
            let git_dir = canon.join(".git");
            if git_dir.exists() && git_dir.is_dir() {
                if let Err(e) = watcher.watch(&git_dir, RecursiveMode::Recursive) {
                    crate::debug_log::warn(format!(
                        "Failed to watch repository {:?}: {}",
                        git_dir, e
                    ));
                }
            }
        }

        self.watcher = Some(watcher);
    }

    pub fn drain_queue(&mut self) {
        while let Some(ev) = self.queue.pop() {
            match ev {
                crate::queue::InternalEvent::ClosePopup => self.mode = Mode::Detail,

                crate::queue::InternalEvent::ConfirmYes => match self.mode {
                    Mode::BranchDeleteConfirm => self.confirm_branch_delete(),
                    Mode::BranchPushConfirm => self.confirm_branch_push(),
                    Mode::BranchMergeConfirm => self.confirm_branch_merge(),
                    Mode::MergeAbortConfirm => self.confirm_abort_merge(),
                    Mode::MergeContinueConfirm => self.confirm_continue_merge(),
                    Mode::BranchRebaseConfirm => self.confirm_branch_rebase(),
                    Mode::BranchInteractiveRebaseConfirm => {
                        self.confirm_branch_interactive_rebase()
                    }
                    Mode::DiscardChangesConfirm => self.confirm_discard_changes(),
                    Mode::RevertConfirm => self.confirm_revert(),
                    Mode::TagDeleteConfirm => self.confirm_tag_delete(),
                    Mode::TagPushConfirm => self.confirm_tag_push(),
                    Mode::TagPushAllConfirm => self.confirm_tag_push_all(),
                    Mode::StashDeleteConfirm => self.confirm_stash_delete(),
                    Mode::BranchCheckoutConfirm => self.confirm_branch_checkout(),
                    Mode::TagCheckoutConfirm => self.confirm_tag_checkout(),
                    Mode::RemoteDeleteConfirm => self.confirm_remote_delete(),
                    Mode::UpdateConfirm => self.trigger_self_update(),
                    Mode::SubmoduleDeleteConfirm => self.confirm_submodule_delete(),
                    _ => {}
                },
                crate::queue::InternalEvent::ConfirmNo => match self.mode {
                    Mode::BranchDeleteConfirm => self.cancel_branch_delete(),
                    Mode::BranchPushConfirm => self.cancel_branch_push(),
                    Mode::BranchMergeConfirm => self.cancel_branch_merge(),
                    Mode::MergeAbortConfirm => {
                        self.mode = Mode::Detail;
                    }
                    Mode::MergeContinueConfirm => {
                        self.mode = Mode::Detail;
                    }
                    Mode::BranchRebaseConfirm => self.cancel_branch_rebase(),
                    Mode::BranchInteractiveRebaseConfirm => self.cancel_branch_interactive_rebase(),
                    Mode::DiscardChangesConfirm => self.cancel_discard_changes(),
                    Mode::RevertConfirm => self.cancel_revert(),
                    Mode::TagDeleteConfirm => self.cancel_tag_delete(),
                    Mode::TagPushConfirm => self.cancel_tag_push(),
                    Mode::TagPushAllConfirm => self.cancel_tag_push_all(),
                    Mode::StashDeleteConfirm => self.cancel_stash_delete(),
                    Mode::BranchCheckoutConfirm => self.cancel_branch_checkout(),
                    Mode::TagCheckoutConfirm => self.cancel_tag_checkout(),
                    Mode::RemoteDeleteConfirm => {
                        self.remote_action_target = None;
                        self.mode = Mode::Detail;
                    }
                    Mode::SubmoduleDeleteConfirm => self.cancel_submodule_delete(),
                    Mode::UpdateConfirm => {
                        self.mode = self.previous_mode.take().unwrap_or(Mode::Normal);
                    }
                    _ => {
                        self.mode = Mode::Detail;
                    }
                },
                crate::queue::InternalEvent::InputChar(c) => self.input_char(c),
                crate::queue::InternalEvent::InputBackspace => self.input_backspace(),
                crate::queue::InternalEvent::InputEnter => match self.mode {
                    Mode::BranchCreateInput => self.commit_branch_create(),
                    Mode::TagCreateInput => self.commit_tag_create(),
                    Mode::StashCreateInput => self.commit_stash_create(),
                    Mode::RemoteAddNameInput => self.commit_remote_add_name(),
                    Mode::RemoteAddUrlInput => self.commit_remote_add_url(),
                    Mode::WorktreeAddBranchInput => self.commit_worktree_add_branch(),
                    Mode::WorktreeAddPathInput => self.commit_worktree_add_path(),
                    Mode::WorktreeLockReasonInput => self.commit_worktree_lock_reason(),
                    Mode::WorktreeRemoveConfirm => self.commit_worktree_remove(),
                    Mode::SubmoduleAddUrlInput => self.commit_submodule_add_url(),
                    Mode::SubmoduleAddPathInput => self.commit_submodule_add_path(),
                    _ => {}
                },
                crate::queue::InternalEvent::InputEsc => {
                    self.input_buffer.clear();
                    match self.mode {
                        Mode::BranchCreateInput => self.cancel_branch_create(),
                        Mode::TagCreateInput => {
                            self.tag_action_target_oid = None;
                            self.mode = Mode::Detail;
                        }
                        _ => {
                            self.mode = Mode::Detail;
                        }
                    }
                }
                // simplified
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
                crate::queue::InternalEvent::YankSelectedCommitHash => {
                    self.yank_selected_commit_hash()
                }
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
                    let page = self.get_current_page_size();
                    self.detail_commit_page_up(page);
                }

                crate::queue::InternalEvent::CommitSelectionTop => self.detail_commit_to_top(),
                crate::queue::InternalEvent::CommitSelectionBottom => {
                    self.detail_commit_to_bottom()
                }
                crate::queue::InternalEvent::LoadMoreCommits => {
                    if self.commit_list.limit > 0 {
                        let add_amount = if self.get_current_max_commits() > 0 {
                            self.get_current_max_commits()
                        } else {
                            200
                        };
                        self.commit_list.limit = self.commit_list.limit.saturating_add(add_amount);
                        self.resync_detail();
                        self.status_message = Some("Loading more commits...".to_string());
                    }
                }
                crate::queue::InternalEvent::CommitDetailsUp => {
                    self.commit_list.details_scroll_up()
                }
                crate::queue::InternalEvent::CommitDetailsDown => {
                    self.commit_list.details_scroll_down()
                }
                crate::queue::InternalEvent::StagingFileUp => {
                    if self.is_uncommitted_selected() {
                        self.staging_file_up()
                    } else {
                        self.detail_file_up()
                    }
                }
                crate::queue::InternalEvent::StagingFileDown => {
                    if self.is_uncommitted_selected() {
                        self.staging_file_down()
                    } else {
                        self.detail_file_down()
                    }
                }
                crate::queue::InternalEvent::ConflictFileUp => self.conflict_file_up(),
                crate::queue::InternalEvent::ConflictFileDown => self.conflict_file_down(),
                crate::queue::InternalEvent::StageSelectedFile => self.stage_selected_file(),
                crate::queue::InternalEvent::UnstageSelectedFile => self.unstage_selected_file(),
                crate::queue::InternalEvent::ResolveConflictOurs => self.resolve_conflict_ours(),
                crate::queue::InternalEvent::ResolveConflictTheirs => {
                    self.resolve_conflict_theirs()
                }
                crate::queue::InternalEvent::MarkConflictResolved => self.mark_conflict_resolved(),
                crate::queue::InternalEvent::MergeAbortConfirm => {
                    self.mode = Mode::MergeAbortConfirm
                }
                crate::queue::InternalEvent::MergeContinueConfirm => {
                    self.mode = Mode::MergeContinueConfirm
                }
                crate::queue::InternalEvent::StageSelectedHunk => self.stage_selected_hunk(),
                crate::queue::InternalEvent::UnstageSelectedHunk => self.unstage_selected_hunk(),
                crate::queue::InternalEvent::StageAllChanges => self.stage_all_changes(),
                crate::queue::InternalEvent::UnstageAllChanges => self.unstage_all_changes(),
                crate::queue::InternalEvent::RequestDiscardChanges => {
                    self.request_discard_changes()
                }
                crate::queue::InternalEvent::RequestDiscardAllChanges => {
                    self.request_discard_all_changes()
                }
                crate::queue::InternalEvent::StartStashCreate => self.start_stash_create(),
                crate::queue::InternalEvent::DiffScrollUp => self.diff.diff_scroll_up(),
                crate::queue::InternalEvent::DiffScrollDown => self.diff.diff_scroll_down(),
                crate::queue::InternalEvent::DiffScrollPageUp => {
                    let page = self.get_current_page_size();
                    self.diff.diff_scroll_page_up(page);
                }
                crate::queue::InternalEvent::DiffScrollPageDown => {
                    let page = self.get_current_page_size();
                    self.diff.diff_scroll_page_down(page);
                }
                crate::queue::InternalEvent::DiffScrollTop => self.diff.diff_scroll_to_top(),
                crate::queue::InternalEvent::DiffScrollBottom => self.diff.diff_scroll_to_bottom(),

                // FileTree
                crate::queue::InternalEvent::FileTreeUp => self.file_list_up(),
                crate::queue::InternalEvent::FileTreeDown => self.file_list_down(),
                crate::queue::InternalEvent::FileTreePageUp => {
                    let p = self.get_current_page_size();
                    self.file_list_page_up(p)
                }
                crate::queue::InternalEvent::FileTreePageDown => {
                    let p = self.get_current_page_size();
                    self.file_list_page_down(p)
                }
                crate::queue::InternalEvent::FileTreeTop => self.file_list_to_top(),
                crate::queue::InternalEvent::FileTreeBottom => self.file_list_to_bottom(),
                crate::queue::InternalEvent::FileContentUp => {
                    self.file_tree.file_content_scroll_up()
                }
                crate::queue::InternalEvent::FileContentDown => {
                    self.file_tree.file_content_scroll_down()
                }
                crate::queue::InternalEvent::FileContentPageUp => {
                    let p = self.get_current_page_size();
                    self.file_tree.file_content_scroll_page_up(p)
                }
                crate::queue::InternalEvent::FileContentPageDown => {
                    let p = self.get_current_page_size();
                    self.file_tree.file_content_scroll_page_down(p)
                }
                crate::queue::InternalEvent::FileContentTop => {
                    self.file_tree.file_content_scroll_to_top()
                }
                crate::queue::InternalEvent::FileContentBottom => {
                    self.file_tree.file_content_scroll_to_bottom()
                }
                crate::queue::InternalEvent::ToggleFolderExpanded => self.toggle_folder_expanded(),
                crate::queue::InternalEvent::CollapseAllFolders => self.collapse_all_folders(),
                crate::queue::InternalEvent::RequestDiscardFile => self.request_discard_changes(),

                // BranchList
                crate::queue::InternalEvent::LocalBranchUp => self.local_branch_up(),
                crate::queue::InternalEvent::LocalBranchDown => self.local_branch_down(),
                crate::queue::InternalEvent::LocalBranchPageUp => {
                    let p = self.get_current_page_size();
                    self.local_branch_page_up(p)
                }
                crate::queue::InternalEvent::LocalBranchPageDown => {
                    let p = self.get_current_page_size();
                    self.local_branch_page_down(p)
                }
                crate::queue::InternalEvent::LocalBranchTop => self.local_branch_to_top(),
                crate::queue::InternalEvent::LocalBranchBottom => self.local_branch_to_bottom(),
                crate::queue::InternalEvent::RemoteBranchUp => self.remote_branch_up(),
                crate::queue::InternalEvent::RemoteBranchDown => self.remote_branch_down(),
                crate::queue::InternalEvent::RemoteBranchPageUp => {
                    let p = self.get_current_page_size();
                    self.remote_branch_page_up(p)
                }
                crate::queue::InternalEvent::RemoteBranchPageDown => {
                    let p = self.get_current_page_size();
                    self.remote_branch_page_down(p)
                }
                crate::queue::InternalEvent::RemoteBranchTop => self.remote_branch_to_top(),
                crate::queue::InternalEvent::RemoteBranchBottom => self.remote_branch_to_bottom(),
                crate::queue::InternalEvent::CheckoutBranch => self.request_branch_checkout(),
                crate::queue::InternalEvent::RequestDeleteBranch => self.request_branch_delete(),
                crate::queue::InternalEvent::StartBranchCreate => self.start_branch_create(),
                crate::queue::InternalEvent::StartBranchMerge => self.request_branch_merge(),
                crate::queue::InternalEvent::StartBranchRebase => self.request_branch_rebase(),
                crate::queue::InternalEvent::RequestBranchPush => self.request_branch_push(),
                crate::queue::InternalEvent::FetchRemote => {
                    let remote_name = if let Some(crate::repo::ItemDetail::Repo { info, .. }) =
                        &self.current_detail
                    {
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
                crate::queue::InternalEvent::StartRemoteAdd => self.start_remote_add(),
                crate::queue::InternalEvent::RequestDeleteRemote => self.request_remote_delete(),

                // TagList
                crate::queue::InternalEvent::TagUp => self.local_tag_up(),
                crate::queue::InternalEvent::TagDown => self.local_tag_down(),
                crate::queue::InternalEvent::TagPageUp => {
                    let p = self.get_current_page_size();
                    self.local_tag_page_up(p)
                }
                crate::queue::InternalEvent::TagPageDown => {
                    let p = self.get_current_page_size();
                    self.local_tag_page_down(p)
                }
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
                crate::queue::InternalEvent::StashPageUp => {
                    let p = self.get_current_page_size();
                    self.stash_page_up(p)
                }
                crate::queue::InternalEvent::StashPageDown => {
                    let p = self.get_current_page_size();
                    self.stash_page_down(p)
                }
                crate::queue::InternalEvent::StashTop => self.stash_to_top(),
                crate::queue::InternalEvent::StashBottom => self.stash_to_bottom(),
                crate::queue::InternalEvent::StashFileUp => self.stash_file_up(),
                crate::queue::InternalEvent::StashFileDown => self.stash_file_down(),
                crate::queue::InternalEvent::StashFilePageUp => {
                    let p = self.get_current_page_size();
                    self.stash_file_page_up(p)
                }
                crate::queue::InternalEvent::StashFilePageDown => {
                    let p = self.get_current_page_size();
                    self.stash_file_page_down(p)
                }
                crate::queue::InternalEvent::StashFileTop => self.stash_file_to_top(),
                crate::queue::InternalEvent::StashFileBottom => self.stash_file_to_bottom(),
                crate::queue::InternalEvent::RequestDeleteStash => self.request_stash_delete(),
                crate::queue::InternalEvent::RequestApplyStash => self.request_stash_apply(),

                crate::queue::InternalEvent::CommitSelectionPageDown => {
                    let page = self.get_current_page_size();
                    self.detail_commit_page_down(page);
                }
                _ => {}
            }
        }
    }

    pub fn sym(&self, key: &str) -> &'static str {
        self.config.sym(key)
    }

    pub fn is_bound(
        &self,
        action: crate::keybindings::Action,
        key: crossterm::event::KeyEvent,
    ) -> bool {
        self.keybindings.matches(action, key)
    }

    pub fn new(config: Config, config_path: PathBuf) -> Self {
        crate::debug_log::info("Initializing Gitwig application state");
        crate::ui::update_theme(&config.theme);
        let config_dir = config_path.parent().unwrap_or(&config_path);
        let keybindings = crate::keybindings::KeybindingsConfig::load(config_dir);
        let original_items = config.items.clone();
        let max_commits = config.max_commits;
        let statuses = config.items.iter().map(|s| repo::inspect_summary(s)).collect();
        let (tx, rx) = std::sync::mpsc::channel();
        let (detail_tx, detail_rx) = std::sync::mpsc::channel();
        let (tab_tx, tab_rx) = std::sync::mpsc::channel();
        let (status_refresh_tx, status_refresh_rx) = std::sync::mpsc::channel();
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
            commit_list: crate::components::commit_list::CommitListComponent {
                limit: max_commits,
                queue: queue.clone(),
                ..Default::default()
            },
            commit_popup: crate::popups::commit::CommitPopup::new(queue.clone()),
            confirm_popup: crate::popups::confirm::ConfirmPopup::new(queue.clone()),
            generic_input_popup: crate::popups::commit::GenericInputPopup::new(queue.clone()),

            repo_search_query: None,

            diff: crate::components::diff::DiffComponent::new(queue.clone()),

            commit_input_scroll: 0,
            help_scroll: 0,
            legend_scroll: 0,
            collapsed_groups: std::collections::HashSet::new(),
            repo_jump_selection: 0,
            detail_areas: DetailAreas::default(),
            main_areas: Vec::new(),
            global_filter: None,
            global_summary_area: None,

            status_list: crate::components::status_list::StatusListComponent::new(queue.clone()),

            last_click: None,
            detail_tab: 0,
            graph_scroll: 0,
            status_expanded: false,
            settings_focus_sidebar: true,
            tx,
            rx,
            fetching: false,
            update_available: None,
            update_check_manual: false,
            implicit_network_count: 0,
            previous_mode: None,
            scanned_repos: Vec::new(),
            repo_scan_selection: 0,
            repo_scan_active: false,
            repo_scan_count: 0,
            repo_settings_selected_index: 0,
            repo_settings_editing: false,
            repo_settings_input: String::new(),
            keybindings,
            pending_git_app: false,
            pending_terminal: false,
            pending_fzf: false,
            pending_bulk_fzf: false,
            pending_files_fzf: false,
            pending_editor_file: None,
            pending_interactive_rebase: None,
            bulk_fetching: std::collections::HashSet::new(),
            bulk_fetch_results: std::collections::HashMap::new(),
            multi_selected: std::collections::HashSet::new(),
            in_logs_ui: false,
            repo_theme_cache: std::collections::HashMap::new(),
            inspect_full_diff: false,
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
            stash_untracked: true,
            stash_keep_index: false,
            stash_action_target: None,
            stashing_ui_selection: 0,
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
            file_history_revisions: Vec::new(),
            file_history_selection: 0,
            file_history_diff: Vec::new(),
            file_history_diff_scroll: 0,
            file_history_path: String::new(),
            file_history_focus: 0,
            worktree_selection: 0,
            submodule_selection: 0,
            reflog_selection: 0,
            worktree_add_branch: String::new(),
            worktree_add_path: String::new(),
            worktree_lock_reason: String::new(),
            worktree_remove_delete_folder: false,
            worktree_remove_force: false,
            submodule_add_url: String::new(),
            submodule_add_path: String::new(),
            submodule_delete_target: None,
            cpu_tracker: std::sync::Mutex::new(None),
            watcher: None,
            status_refresh_tx,
            status_refresh_rx,
            last_background_refresh: std::time::Instant::now(),
            background_refresh_running: false,
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

        #[cfg(not(test))]
        {
            app.trigger_update_check_internal(false);
        }

        app.resolve_repo_themes();
        app.setup_watcher();

        app
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
        // Trigger background status auto-refresh if 10 seconds have elapsed
        if !app.background_refresh_running
            && !app.config.items.is_empty()
            && app.last_background_refresh.elapsed() >= std::time::Duration::from_secs(10)
        {
            app.background_refresh_running = true;
            app.last_background_refresh = std::time::Instant::now();
            let paths = app.config.items.clone();
            let tx = app.status_refresh_tx.clone();
            std::thread::spawn(move || {
                let mut updates = Vec::new();
                for (idx, path) in paths.into_iter().enumerate() {
                    let status = repo::inspect_summary(&path);
                    updates.push((idx, path, status));
                }
                let _ = tx.send(updates);
            });
        }
        while let Ok(raw_msg) = app.rx.try_recv() {
            if let Some(repo_info) = raw_msg.strip_prefix("REPO_SCAN_FOUND:") {
                if let Some(pos) = repo_info.find("|||") {
                    let name = repo_info[..pos].to_string();
                    let path = repo_info[pos + 3..].to_string();
                    if !app.scanned_repos.iter().any(|(_, p)| p == &path) {
                        app.scanned_repos.push((name, path));
                    }
                }
                continue;
            }
            if raw_msg.starts_with("REPO_SCAN_COMPLETE:") {
                app.repo_scan_active = false;
                continue;
            }
            if let Some(count_str) = raw_msg.strip_prefix("REPO_SCAN_COUNT:") {
                if let Ok(count) = count_str.parse::<usize>() {
                    app.repo_scan_count = count;
                }
                continue;
            }

            if let Some(success_path) = raw_msg.strip_prefix("BULK_FETCH_SUCCESS:") {
                app.bulk_fetching.remove(success_path);
                if let Some(idx) = app.config.items.iter().position(|item| item == success_path) {
                    app.statuses[idx] = repo::inspect_summary(&app.config.items[idx]);
                }
                app.bulk_fetch_results
                    .insert(success_path.to_string(), Ok("Fetched successfully".to_string()));
                if app.bulk_fetching.is_empty() {
                    app.status_message = Some("Bulk fetch completed successfully".to_string());
                }
                continue;
            }
            if let Some(error_data) = raw_msg.strip_prefix("BULK_FETCH_ERROR:") {
                if let Some(pos) = error_data.find("|||") {
                    let err_path = &error_data[..pos];
                    let err_msg = &error_data[pos + 3..];
                    app.bulk_fetching.remove(err_path);
                    app.bulk_fetch_results.insert(err_path.to_string(), Err(err_msg.to_string()));
                }
                if app.bulk_fetching.is_empty() {
                    app.status_message = Some("Bulk fetch completed".to_string());
                }
                continue;
            }

            let (repo_path, msg) = if let Some(pos) = raw_msg.find("|||") {
                (Some(raw_msg[..pos].to_string()), raw_msg[pos + 3..].to_string())
            } else {
                (None, raw_msg)
            };

            let is_relevant = if let Some(ref path_str) = repo_path {
                if let Some(repo::ItemDetail::Repo { resolved, .. }) = &app.current_detail {
                    resolved.to_string_lossy() == *path_str
                } else {
                    false
                }
            } else {
                true
            };

            if is_relevant {
                if let Some(repo_path) = msg.strip_prefix("REFRESH_REPO:") {
                    let canon_target = std::fs::canonicalize(repo_path)
                        .unwrap_or_else(|_| PathBuf::from(repo_path));
                    if let Some(idx) = app.config.items.iter().position(|item| {
                        let canon_item =
                            std::fs::canonicalize(item).unwrap_or_else(|_| PathBuf::from(item));
                        canon_item == canon_target
                    }) {
                        app.statuses[idx] = repo::inspect_summary(&app.config.items[idx]);
                        if let Some(repo::ItemDetail::Repo { resolved, .. }) = &app.current_detail {
                            if resolved == &canon_target {
                                app.resync_detail();
                            }
                        }
                    }
                } else if let Some(latest_version) = msg.strip_prefix("UPDATE_CHECK:") {
                    let current_version = env!("CARGO_PKG_VERSION");
                    if is_newer_version(current_version, latest_version) {
                        app.update_available = Some(latest_version.to_string());
                        if app.update_check_manual {
                            app.status_message =
                                Some(format!("Update available: v{}", latest_version));
                        }
                    } else if app.update_check_manual {
                        app.status_message = Some("Gitwig is up to date".to_string());
                    }
                    if !app.update_check_manual {
                        app.decrement_implicit_network();
                    }
                    app.update_check_manual = false;
                } else if msg.starts_with("UPDATE_CHECK_FAILED:") {
                    if app.update_check_manual {
                        app.status_message = Some("Failed to check for updates".to_string());
                    }
                    if !app.update_check_manual {
                        app.decrement_implicit_network();
                    }
                    app.update_check_manual = false;
                } else if let Some(success_msg) = msg.strip_prefix("CHECKOUT_SUCCESS:") {
                    app.fetching = false;
                    app.status_message = Some(success_msg.to_string());
                    app.resync_detail();
                } else if let Some(err_msg) = msg.strip_prefix("CHECKOUT_ERROR:") {
                    app.fetching = false;
                    app.set_error(err_msg.to_string());
                } else if let Some(success_msg) = msg.strip_prefix("UPDATE_SUCCESS:") {
                    app.fetching = false;
                    app.status_message = Some(success_msg.to_string());
                } else if let Some(err_msg) = msg.strip_prefix("UPDATE_ERROR:") {
                    app.fetching = false;
                    app.set_error(err_msg.to_string());
                } else if let Some(dest_path) = msg.strip_prefix("CLONE_SUCCESS:") {
                    crate::debug_log::info(format!(
                        "Network Action: Cloning succeeded to {}",
                        dest_path
                    ));
                    app.fetching = false;
                    app.status_message = Some("Cloning completed successfully".to_string());
                    app.add_repo_path(dest_path.to_string());
                } else if let Some(tags_data) = msg.strip_prefix("REMOTE_TAGS:") {
                    crate::debug_log::info("Network Action: Fetching remote tags succeeded");
                    let tags = repo::deserialize_tags(tags_data);
                    if let Some(repo::ItemDetail::Repo { info, .. }) = &mut app.current_detail {
                        info.remote_tags = repo::TabData::Loaded(tags);
                        info.remote_tags_loaded = true;
                    }
                    if app.fetching {
                        app.fetching = false;
                    } else {
                        app.decrement_implicit_network();
                    }
                } else if let Some(err_msg) = msg.strip_prefix("REMOTE_TAGS_ERR:") {
                    crate::debug_log::warn(format!(
                        "Network Action: Fetching remote tags failed: {}",
                        err_msg
                    ));
                    if app.fetching {
                        app.set_error(err_msg.to_string());
                        app.fetching = false;
                    } else {
                        app.decrement_implicit_network();
                    }
                } else {
                    let success_fetch = msg.starts_with("Fetched remote ");
                    let is_err = msg.starts_with("Fetch failed:")
                        || msg.starts_with("Pull failed:")
                        || msg.starts_with("Push failed:")
                        || msg.starts_with("Failed to")
                        || msg.contains("failed");

                    if is_err {
                        let has_conflict = msg.contains("conflict") || msg.contains("CONFLICT");
                        crate::debug_log::error(format!(
                            "Network Action: Operation failed: {}",
                            msg
                        ));
                        app.set_error(msg);
                        if has_conflict {
                            app.detail_focus = DetailSection::Conflicts;
                        }
                    } else {
                        crate::debug_log::info(format!(
                            "Network Action: Operation succeeded: {}",
                            msg
                        ));
                        app.status_message = Some(msg);
                    }
                    app.fetching = false;
                    app.resync_detail();
                    if success_fetch {
                        app.fetch_remote_tags(false);
                    }
                }
            } else {
                app.fetching = false;
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

        while let Ok(updates) = app.status_refresh_rx.try_recv() {
            app.background_refresh_running = false;
            for (idx, path, status) in updates {
                if app.config.items.get(idx) == Some(&path) {
                    if idx < app.statuses.len() {
                        app.statuses[idx] = status;
                    }
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
                    if tab_idx < 10 {
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
                            if !info.remote_tags_loaded {
                                info.remote_tags = match remote {
                                    Ok(t) => repo::TabData::Loaded(t),
                                    Err(e) => repo::TabData::Error(e),
                                };
                            }
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
                        repo::TabPayload::Worktrees(res) => {
                            info.worktrees = match res {
                                Ok(w) => repo::TabData::Loaded(w),
                                Err(e) => repo::TabData::Error(e),
                            };
                        }
                        repo::TabPayload::Submodules(res) => {
                            info.submodules = match res {
                                Ok(s) => repo::TabData::Loaded(s),
                                Err(e) => repo::TabData::Error(e),
                            };
                        }
                        repo::TabPayload::Reflog(res) => {
                            info.reflog = match res {
                                Ok(r) => repo::TabData::Loaded(r),
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
                    let mut cmd = if cfg!(target_os = "windows") {
                        let mut c = std::process::Command::new("cmd");
                        c.arg("/c").arg(git_app_name);
                        c
                    } else {
                        std::process::Command::new(git_app_name)
                    };
                    let status = cmd.current_dir(&path).status();

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

        if app.pending_terminal {
            app.pending_terminal = false;

            let mut paths_to_open = Vec::new();
            if !app.multi_selected.is_empty() {
                paths_to_open = app.multi_selected.iter().cloned().collect::<Vec<_>>();
                app.multi_selected.clear();
            } else if let Some(item) = app.config.items.get(app.selected_index) {
                paths_to_open.push(item.clone());
            }

            if !paths_to_open.is_empty() {
                let raw_res = crossterm::terminal::disable_raw_mode();
                let exec_res = crossterm::execute!(
                    std::io::stdout(),
                    crossterm::terminal::LeaveAlternateScreen,
                    crossterm::event::DisableMouseCapture
                );
                let cursor_res = terminal.show_cursor();

                if raw_res.is_ok() && exec_res.is_ok() && cursor_res.is_ok() {
                    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

                    // Helpers to translate active ACCENT color to ANSI codes
                    let accent_color = crate::ui::style::ACCENT();
                    let ansi_normal = match accent_color {
                        ratatui::style::Color::Black => "\x1b[30m",
                        ratatui::style::Color::Red => "\x1b[31m",
                        ratatui::style::Color::Green => "\x1b[32m",
                        ratatui::style::Color::Yellow => "\x1b[33m",
                        ratatui::style::Color::Blue => "\x1b[34m",
                        ratatui::style::Color::Magenta => "\x1b[35m",
                        ratatui::style::Color::Cyan => "\x1b[36m",
                        ratatui::style::Color::Gray => "\x1b[37m",
                        ratatui::style::Color::DarkGray => "\x1b[90m",
                        ratatui::style::Color::LightRed => "\x1b[91m",
                        ratatui::style::Color::LightGreen => "\x1b[92m",
                        ratatui::style::Color::LightYellow => "\x1b[93m",
                        ratatui::style::Color::LightBlue => "\x1b[94m",
                        ratatui::style::Color::LightMagenta => "\x1b[95m",
                        ratatui::style::Color::LightCyan => "\x1b[96m",
                        ratatui::style::Color::White => "\x1b[97m",
                        _ => "\x1b[36m",
                    };
                    let ansi_bold = match accent_color {
                        ratatui::style::Color::Black => "\x1b[1;30m",
                        ratatui::style::Color::Red => "\x1b[1;31m",
                        ratatui::style::Color::Green => "\x1b[1;32m",
                        ratatui::style::Color::Yellow => "\x1b[1;33m",
                        ratatui::style::Color::Blue => "\x1b[1;34m",
                        ratatui::style::Color::Magenta => "\x1b[1;35m",
                        ratatui::style::Color::Cyan => "\x1b[1;36m",
                        ratatui::style::Color::Gray => "\x1b[1;37m",
                        ratatui::style::Color::DarkGray => "\x1b[1;90m",
                        ratatui::style::Color::LightRed => "\x1b[1;91m",
                        ratatui::style::Color::LightGreen => "\x1b[1;92m",
                        ratatui::style::Color::LightYellow => "\x1b[1;93m",
                        ratatui::style::Color::LightBlue => "\x1b[1;94m",
                        ratatui::style::Color::LightMagenta => "\x1b[1;95m",
                        ratatui::style::Color::LightCyan => "\x1b[1;96m",
                        ratatui::style::Color::White => "\x1b[1;97m",
                        _ => "\x1b[1;36m",
                    };

                    for item in &paths_to_open {
                        let path = repo::expand_tilde(item);
                        let repo_name =
                            path.file_name().and_then(|n| n.to_str()).unwrap_or("repository");

                        // Update terminal title for the subshell
                        if app.config.compatibility_mode {
                            let _ = crossterm::execute!(
                                std::io::stdout(),
                                crossterm::terminal::SetTitle(format!(
                                    "[Gitwig] Shell ({})",
                                    repo_name
                                ))
                            );
                        } else {
                            let _ = crossterm::execute!(
                                std::io::stdout(),
                                crossterm::terminal::SetTitle(format!(
                                    "🌿 Gitwig Shell ({})",
                                    repo_name
                                ))
                            );
                        }

                        // Determine borders and branding based on compatibility mode
                        let banner_width = 80;
                        let inner_width = banner_width - 6; // 74 chars
                        let (top_border, left_border, right_border, bottom_border, gitwig_prefix) =
                            if app.config.compatibility_mode {
                                (
                                    "-".repeat(banner_width),
                                    "|",
                                    "|",
                                    "-".repeat(banner_width),
                                    "[Gitwig]",
                                )
                            } else {
                                (
                                    "┌".to_string() + &"─".repeat(banner_width - 2) + "┐",
                                    "│",
                                    "│",
                                    "└".to_string() + &"─".repeat(banner_width - 2) + "┘",
                                    "🌿 Gitwig",
                                )
                            };

                        println!();
                        println!("{}{}\x1b[0m", ansi_normal, top_border);

                        let line1_text = format!(
                            "{} Subshell — Type 'exit' or press Ctrl+D to return to Gitwig",
                            gitwig_prefix
                        );
                        let padding1 = inner_width.saturating_sub(line1_text.chars().count());
                        println!(
                            "{}{}\x1b[0m  {}{}\x1b[0m{}  {}{}\x1b[0m",
                            ansi_normal,
                            left_border,
                            ansi_bold,
                            line1_text,
                            " ".repeat(padding1),
                            ansi_normal,
                            right_border
                        );

                        let line2_text = format!("Workspace: {}", path.display());
                        let line2_chars: Vec<char> = line2_text.chars().collect();
                        let display_path = if line2_chars.len() > inner_width {
                            let keep_len = inner_width - 15;
                            let truncated: String =
                                line2_chars[line2_chars.len() - keep_len..].iter().collect();
                            format!("Workspace: ...{}", truncated)
                        } else {
                            line2_text
                        };
                        let padding2 = inner_width.saturating_sub(display_path.chars().count());
                        println!(
                            "{}{}\x1b[0m  \x1b[2m{}\x1b[0m{}  {}{}\x1b[0m",
                            ansi_normal,
                            left_border,
                            display_path,
                            " ".repeat(padding2),
                            ansi_normal,
                            right_border
                        );

                        println!("{}{}\x1b[0m", ansi_normal, bottom_border);
                        println!();

                        let _ = std::process::Command::new(&shell)
                            .current_dir(&path)
                            .env("GITWIG", "1")
                            .env("GITWIG_SHELL", "1")
                            .status();
                    }

                    // Restore Gitwig terminal title
                    if app.config.compatibility_mode {
                        let _ = crossterm::execute!(
                            std::io::stdout(),
                            crossterm::terminal::SetTitle("[Gitwig]")
                        );
                    } else {
                        let _ = crossterm::execute!(
                            std::io::stdout(),
                            crossterm::terminal::SetTitle("🌿 Gitwig")
                        );
                    }

                    let _ = crossterm::terminal::enable_raw_mode();
                    let _ = crossterm::execute!(
                        std::io::stdout(),
                        crossterm::terminal::EnterAlternateScreen,
                        crossterm::event::EnableMouseCapture
                    );
                    let _ = terminal.clear();

                    app.status_message = Some("Returned from terminal".to_string());
                    app.refresh_selected_status();
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
                    .env("GIT_SSH_COMMAND", crate::config::ssh_command_val())
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
                let git_only = app.config.fzf.git_only;
                let excludes = app.config.fzf.excludes.clone();
                let expanded_start_dir = crate::repo::expand_tilde(&app.config.fzf.start_dir);

                let output = run_fzf_picker(&expanded_start_dir, git_only, max_depth, &excludes);

                let _ = crossterm::terminal::enable_raw_mode();
                let _ = crossterm::execute!(
                    std::io::stdout(),
                    crossterm::terminal::EnterAlternateScreen,
                    crossterm::event::EnableMouseCapture
                );
                let _ = terminal.clear();

                match output {
                    Ok(Some(selected)) => {
                        app.add_repo_path(selected);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        let err_str = e.to_string();
                        if err_str.contains("not found") || err_str.contains("No such file") {
                            app.set_error("fzf is not installed. Please install fzf.".to_string());
                        } else {
                            app.set_error(format!("Could not run fzf: {}", e));
                        }
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
                let excludes = app.config.fzf.excludes.clone();
                let expanded_start_dir = crate::repo::expand_tilde(&app.config.fzf.start_dir);

                let output = run_fzf_picker(
                    &expanded_start_dir,
                    false, // bulk search is git_only = false
                    max_depth,
                    &excludes,
                );

                let _ = crossterm::terminal::enable_raw_mode();
                let _ = crossterm::execute!(
                    std::io::stdout(),
                    crossterm::terminal::EnterAlternateScreen,
                    crossterm::event::EnableMouseCapture
                );
                let _ = terminal.clear();

                match output {
                    Ok(Some(selected)) => {
                        app.bulk_add_path(selected);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        let err_str = e.to_string();
                        if err_str.contains("not found") || err_str.contains("No such file") {
                            app.set_error("fzf is not installed. Please install fzf.".to_string());
                        } else {
                            app.set_error(format!("Could not run fzf: {}", e));
                        }
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
                    let mut child_cmd = if cfg!(target_os = "windows") {
                        let mut c = std::process::Command::new("cmd");
                        c.arg("/c").arg("fzf");
                        c
                    } else {
                        std::process::Command::new("fzf")
                    };
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
                                    if let Some(pos) = app
                                        .file_tree
                                        .visible_files
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

        if let Some(file_rel_path) = app.pending_editor_file.take() {
            if let Some(repo::ItemDetail::Repo { resolved, .. }) = &app.current_detail {
                let repo_path = resolved.clone();
                let file_path = repo_path.join(&file_rel_path);
                let repo_path_str = repo_path.to_string_lossy().to_string();
                let editor = app
                    .config
                    .repo_configs
                    .get(&repo_path_str)
                    .and_then(|c| c.editor.clone())
                    .unwrap_or_else(|| app.config.editor.clone());

                let raw_res = crossterm::terminal::disable_raw_mode();
                let exec_res = crossterm::execute!(
                    std::io::stdout(),
                    crossterm::terminal::LeaveAlternateScreen,
                    crossterm::event::DisableMouseCapture
                );
                let cursor_res = terminal.show_cursor();

                if raw_res.is_ok() && exec_res.is_ok() && cursor_res.is_ok() {
                    let mut cmd = if cfg!(target_os = "windows") {
                        let mut c = std::process::Command::new("cmd");
                        c.arg("/c").arg(&editor);
                        c
                    } else {
                        std::process::Command::new(&editor)
                    };
                    let status = cmd.arg(&file_path).current_dir(&repo_path).status();

                    let _ = crossterm::terminal::enable_raw_mode();
                    let _ = crossterm::execute!(
                        std::io::stdout(),
                        crossterm::terminal::EnterAlternateScreen,
                        crossterm::event::EnableMouseCapture
                    );
                    let _ = terminal.clear();

                    match status {
                        Ok(s) if s.success() => {
                            app.status_message = Some(format!("Returned from {}", editor));
                            app.refresh_selected_status();
                        }
                        Ok(_) => {
                            app.status_message = Some(format!("{} exited with error", editor));
                            app.refresh_selected_status();
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            app.set_error(format!("{} is not found in the system", editor));
                        }
                        Err(e) => {
                            app.set_error(format!("Could not run {}: {}", editor, e));
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
        let mut list_height = if app.config.compact_view {
            available_height.saturating_sub(1)
        } else {
            available_height
        };
        if !app.config.items.is_empty() {
            list_height = list_height.saturating_sub(2);
        }
        let rows = app.get_home_rows();
        let mut accumulated_height = 0;
        let mut visible_count = 0;
        for row in rows.iter().skip(app.scroll_top) {
            let h = match row {
                crate::app::HomeRow::GroupHeader { .. } => {
                    if app.config.compact_view {
                        1
                    } else {
                        2
                    }
                }
                crate::app::HomeRow::Repo { .. } => {
                    if app.config.compact_view {
                        1
                    } else {
                        4
                    }
                }
            };
            if accumulated_height + h <= list_height {
                accumulated_height += h;
                visible_count += 1;
            } else {
                break;
            }
        }
        if visible_count == 0 && !rows.is_empty() {
            visible_count = 1;
        }
        app.clamp_scroll(visible_count);
        app.clamp_help_scroll(area.height as usize);
        app.clamp_legend_scroll();

        app.trigger_tab_load_if_needed(app.detail_tab);

        // Capture panel rects from the draw pass for mouse hit-testing.
        let mut detail_areas = DetailAreas::default();
        let mut main_areas = Vec::new();
        let mut global_summary_area = None;
        terminal.draw(|f| {
            ui::draw(
                f,
                &app,
                area,
                inner_area,
                visible_count,
                &mut detail_areas,
                &mut main_areas,
                &mut global_summary_area,
            )
        })?;
        app.detail_areas = detail_areas;
        app.main_areas = main_areas;
        app.global_summary_area = global_summary_area;

        // Transient feedback disappears after one frame, unless we are fetching.
        if app.fetching || app.implicit_network_count > 0 {
            if app.fetching && app.status_message.is_none() {
                app.status_message = Some("Executing Git operation...".to_string());
            }
            app.fetch_progress = (app.fetch_progress + 5) % 105;
        } else {
            if !app.fetching {
                app.status_message = None;
            }
            app.fetch_progress = 0;
        }

        let poll_dur = if !app.bulk_fetching.is_empty() || app.fetching {
            std::time::Duration::from_millis(80)
        } else {
            std::time::Duration::from_millis(app.config.poll_interval_ms)
        };
        if event::poll(poll_dur)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == crossterm::event::KeyEventKind::Press
                        && !input::handle_key(&mut app, key, visible_count)
                    {
                        return Ok(());
                    }
                }
                Event::Mouse(mouse) => {
                    crate::mouse::handle_mouse(&mut app, mouse);
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

pub(crate) fn copy_to_clipboard(text: &str) -> Result<(), String> {
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

impl App {
    pub fn is_msi_install(&self) -> bool {
        #[cfg(target_os = "windows")]
        {
            if let Ok(exe_path) = std::env::current_exe() {
                let path_str = exe_path.to_string_lossy().to_lowercase();
                if path_str.contains("program files") || path_str.contains("programfiles") {
                    return true;
                }
            }
        }
        false
    }

    pub fn item_height(&self) -> u16 {
        if self.config.compact_view { 1 } else { ITEM_HEIGHT }
    }

    pub fn increment_implicit_network(&mut self) {
        self.implicit_network_count = self.implicit_network_count.saturating_add(1);
    }

    pub fn decrement_implicit_network(&mut self) {
        self.implicit_network_count = self.implicit_network_count.saturating_sub(1);
    }

    pub fn trigger_update_check(&mut self) {
        self.trigger_update_check_internal(true);
    }

    pub fn trigger_update_check_internal(&mut self, manual: bool) {
        crate::debug_log::info(format!("Network Action: Checking for updates (manual={})", manual));
        self.update_check_manual = manual;
        if manual {
            self.status_message = Some("Checking for updates...".to_string());
        } else {
            self.increment_implicit_network();
        }
        let tx_clone = self.tx.clone();
        std::thread::spawn(move || {
            let res = (|| -> Result<String, Box<dyn std::error::Error>> {
                let output = std::process::Command::new("curl")
                    .arg("--max-time")
                    .arg("5")
                    .arg("-fsSL")
                    .arg("https://raw.githubusercontent.com/tareqmy/gitwig/master/.version")
                    .output();
                if let Ok(out) = output {
                    if out.status.success() {
                        let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        if !version.is_empty() {
                            return Ok(version);
                        }
                    }
                }
                let output = std::process::Command::new("wget")
                    .arg("--timeout=5")
                    .arg("-qO-")
                    .arg("https://raw.githubusercontent.com/tareqmy/gitwig/master/.version")
                    .output();
                if let Ok(out) = output {
                    if out.status.success() {
                        let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
                        if !version.is_empty() {
                            return Ok(version);
                        }
                    }
                }
                Err("Failed to query update version".into())
            })();
            if let Ok(latest_version) = res {
                crate::debug_log::info(format!(
                    "Network Action: Update check succeeded. Latest version: v{}",
                    latest_version
                ));
                let _ = tx_clone.send(format!("UPDATE_CHECK:{}", latest_version));
            } else {
                crate::debug_log::warn("Network Action: Update check failed");
                let _ = tx_clone.send("UPDATE_CHECK_FAILED:".to_string());
            }
        });
    }

    pub fn trigger_self_update(&mut self) {
        if self.is_msi_install() {
            self.error_message = Some(
                "Self-update is disabled for system-wide MSI installations.\n\n\
                 Please download the latest MSI package from:\n\
                 https://github.com/tareqmy/gitwig/releases"
                    .to_string(),
            );
            self.mode = self.previous_mode.take().unwrap_or(self.mode);
            return;
        }

        crate::debug_log::info("Network Action: Triggering self-update");
        self.fetching = true;
        self.status_message = Some("Updating Gitwig...".to_string());
        self.mode = self.previous_mode.take().unwrap_or(self.mode);

        let version = self.update_available.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let res = (|| -> Result<String, Box<dyn std::error::Error>> {
                use sha2::{Digest, Sha256};
                use std::fs::File;
                use std::io::Read;

                let target_ref = match version {
                    Some(ref v) => {
                        if v.starts_with('v') {
                            v.clone()
                        } else {
                            format!("v{}", v)
                        }
                    }
                    None => "master".to_string(),
                };

                let temp_dir = std::env::temp_dir();
                let unique_id = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                let is_windows = cfg!(target_os = "windows");
                let script_name = if is_windows { "install.ps1" } else { "install.sh" };
                let sha_name = if is_windows { "install.ps1.sha256" } else { "install.sh.sha256" };

                let script_path = temp_dir.join(format!("gitwig_{}_{}", unique_id, script_name));
                let sha_path = temp_dir.join(format!("gitwig_{}_{}", unique_id, sha_name));

                let script_url = format!(
                    "https://raw.githubusercontent.com/tareqmy/gitwig/{}/scripts/{}",
                    target_ref, script_name
                );
                let sha_url = format!(
                    "https://raw.githubusercontent.com/tareqmy/gitwig/{}/scripts/{}",
                    target_ref, sha_name
                );

                // Helper to download via curl or wget
                let download =
                    |url: &str, dest: &std::path::Path| -> Result<(), Box<dyn std::error::Error>> {
                        let curl_res = std::process::Command::new("curl")
                            .arg("-fsSL")
                            .arg("-o")
                            .arg(dest)
                            .arg(url)
                            .output();
                        if let Ok(out) = curl_res {
                            if out.status.success() {
                                return Ok(());
                            }
                        }
                        let wget_res = std::process::Command::new("wget")
                            .arg("-q")
                            .arg("-O")
                            .arg(dest)
                            .arg(url)
                            .output();
                        match wget_res {
                            Ok(out) if out.status.success() => Ok(()),
                            _ => Err(format!("Failed to download {}", url).into()),
                        }
                    };

                // Download files
                download(&script_url, &script_path)?;
                download(&sha_url, &sha_path)?;

                // Compute SHA-256 of downloaded script
                let mut file = File::open(&script_path)?;
                let mut hasher = Sha256::new();
                let mut buffer = [0; 1024];
                loop {
                    let count = file.read(&mut buffer)?;
                    if count == 0 {
                        break;
                    }
                    hasher.update(&buffer[..count]);
                }
                let computed_hash = format!("{:02x}", hasher.finalize());

                // Read expected hash
                let mut sha_content = String::new();
                File::open(&sha_path)?.read_to_string(&mut sha_content)?;
                let expected_hash = sha_content
                    .split_whitespace()
                    .next()
                    .ok_or_else(|| "Empty checksum file".to_string())?
                    .trim();

                if computed_hash != expected_hash {
                    let _ = std::fs::remove_file(&script_path);
                    let _ = std::fs::remove_file(&sha_path);
                    return Err(format!(
                        "Checksum verification failed!\nExpected: {}\nGot: {}",
                        expected_hash, computed_hash
                    )
                    .into());
                }

                // Execute the verified script
                let output = if is_windows {
                    std::process::Command::new("powershell")
                        .arg("-NoProfile")
                        .arg("-ExecutionPolicy")
                        .arg("Bypass")
                        .arg("-File")
                        .arg(&script_path)
                        .output()
                } else {
                    std::process::Command::new("sh").arg(&script_path).output()
                };

                let _ = std::fs::remove_file(&script_path);
                let _ = std::fs::remove_file(&sha_path);

                match output {
                    Ok(out) => {
                        if out.status.success() {
                            Ok("Gitwig updated successfully! Please restart the application."
                                .to_string())
                        } else {
                            let err_msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
                            Err(format!("Update failed: {}", err_msg).into())
                        }
                    }
                    Err(e) => Err(format!("Update failed: {}", e).into()),
                }
            })();

            match res {
                Ok(success_msg) => {
                    let _ = tx.send(format!("UPDATE_SUCCESS:{}", success_msg));
                }
                Err(err) => {
                    let _ = tx.send(format!("UPDATE_ERROR:{}", err));
                }
            }
        });
    }
}

fn is_newer_version(current: &str, latest: &str) -> bool {
    let parse = |s: &str| -> Vec<u32> {
        s.trim_start_matches('v').split('.').map(|part| part.parse::<u32>().unwrap_or(0)).collect()
    };
    let cur_parts = parse(current);
    let lat_parts = parse(latest);
    for (c, l) in cur_parts.iter().zip(lat_parts.iter()) {
        if l > c {
            return true;
        } else if c > l {
            return false;
        }
    }
    lat_parts.len() > cur_parts.len()
}

fn run_fzf_picker(
    start_dir: &std::path::Path,
    git_only: bool,
    max_depth: usize,
    excludes: &[String],
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut use_fd = false;
    let mut check_cmd = if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("cmd");
        c.arg("/c").arg("fd");
        c
    } else {
        std::process::Command::new("fd")
    };
    if let Ok(output) = check_cmd.arg("--version").output() {
        if output.status.success() {
            use_fd = true;
        }
    }

    let mut paths = Vec::new();

    if use_fd {
        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = std::process::Command::new("cmd");
            c.arg("/c").arg("fd");
            c
        } else {
            std::process::Command::new("fd")
        };
        if git_only {
            cmd.arg("-H").arg(r"^\.git$");
        } else {
            cmd.arg(".").arg("--type").arg("d");
        }
        cmd.arg("--max-depth").arg((max_depth + if git_only { 1 } else { 0 }).to_string());
        for excl in excludes {
            cmd.arg("--exclude").arg(excl);
        }
        cmd.arg(start_dir);

        let output = cmd.output()?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let p = line.trim();
                if !p.is_empty() {
                    if git_only {
                        if let Some(parent) = std::path::Path::new(p).parent() {
                            paths.push(parent.to_string_lossy().to_string());
                        }
                    } else {
                        paths.push(p.to_string());
                    }
                }
            }
        }
    } else {
        fn walk_dir(
            dir: &std::path::Path,
            current_depth: usize,
            max_depth: usize,
            git_only: bool,
            excludes: &[String],
            paths: &mut Vec<String>,
        ) {
            if current_depth > max_depth {
                return;
            }

            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if excludes.iter().any(|excl| name == excl) {
                                continue;
                            }
                            if name == ".git" {
                                if git_only {
                                    if let Some(parent) = path.parent() {
                                        paths.push(parent.to_string_lossy().to_string());
                                    }
                                }
                                continue;
                            }
                        }
                        if !git_only {
                            paths.push(path.to_string_lossy().to_string());
                        }
                        walk_dir(&path, current_depth + 1, max_depth, git_only, excludes, paths);
                    }
                }
            }
        }

        let effective_max_depth = max_depth + if git_only { 1 } else { 0 };
        walk_dir(start_dir, 1, effective_max_depth, git_only, excludes, &mut paths);
    }

    if paths.is_empty() {
        return Ok(None);
    }

    // Now spawn fzf and pipe paths to its stdin
    let mut fzf_cmd = if cfg!(target_os = "windows") {
        let mut c = std::process::Command::new("cmd");
        c.arg("/c").arg("fzf");
        c
    } else {
        std::process::Command::new("fzf")
    };
    let mut child = fzf_cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        for path in paths {
            let _ = writeln!(stdin, "{}", path);
        }
    }

    let output = child.wait_with_output()?;
    if output.status.success() {
        let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !selected.is_empty() {
            return Ok(Some(selected));
        }
    }
    Ok(None)
}

fn run_directory_scan(
    start_dir: std::path::PathBuf,
    max_depth: usize,
    excludes: Vec<String>,
    tx: std::sync::mpsc::Sender<String>,
) {
    std::thread::spawn(move || {
        let root = start_dir;
        let mut count = 0;

        fn scan(
            dir: &std::path::Path,
            depth: usize,
            max_depth: usize,
            excludes: &[String],
            tx: &std::sync::mpsc::Sender<String>,
            count: &mut usize,
        ) {
            *count += 1;
            if (*count).is_multiple_of(50) {
                let _ = tx.send(format!("REPO_SCAN_COUNT:{}", *count));
            }

            let git_dir = dir.join(".git");
            if git_dir.exists() {
                let name = dir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| dir.to_string_lossy().into_owned());
                let path = dir.to_string_lossy().into_owned();
                let _ = tx.send(format!("REPO_SCAN_FOUND:{}|||{}", name, path));
                return;
            }

            if depth >= max_depth {
                return;
            }

            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_dir() {
                            let path = entry.path();
                            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                if name.starts_with('.') && name != "." && name != ".." {
                                    continue;
                                }
                                if excludes
                                    .iter()
                                    .any(|ex| name == ex || path.to_string_lossy().contains(ex))
                                {
                                    continue;
                                }
                            }
                            scan(&path, depth + 1, max_depth, excludes, tx, count);
                        }
                    }
                }
            }
        }

        scan(&root, 0, max_depth, &excludes, &tx, &mut count);
        let _ = tx.send(format!("REPO_SCAN_COUNT:{}", count));
        let _ = tx.send("REPO_SCAN_COMPLETE:".to_string());
    });
}
