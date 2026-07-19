//! User keybindings definitions and custom layout mappings.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    // Global
    ToggleStatusBar,
    Help,
    Close,

    // Home Page
    HomeMoveDown,
    HomeMoveUp,
    HomePageDown,
    HomePageUp,
    HomeHome,
    HomeEnd,
    HomeAddRepo,
    HomeBulkAdd,
    HomeEditRepo,
    HomeDeleteRepo,
    HomeOpenDebugLogs,
    HomeEditLabels,
    HomeAbout,
    HomeSymbolsHelp,
    HomeRefresh,
    HomeCycleSort,
    HomeToggleSortReverse,
    HomeTogglePin,
    HomeOpenSettings,
    HomeImportRepo,
    HomeOpenGitApp,
    HomeSearchRepo,
    HomeOpenDetail,
    HomeCheckUpdate,
    HomeCycleViewMode,
    HomeOpenTerminal,
    HomeToggleStar,
    HomeYankPath,
    HomeJumpPicker,
    HomeFetchAll,
    HomeSelect,
    HomeGlobalSearch,

    // Detail / Workspace Tab Navigation
    CloseDetail,
    DetailHelp,
    CycleFocusForward,
    CycleFocusBackward,
    RefreshDetail,
    CycleTabForward,
    CycleTabBackward,
    GoToTab1,
    GoToTab2,
    GoToTab3,
    GoToTab4,
    GoToTab5,
    GoToTab6,
    GoToTab7,
    Overview,
    ToggleAdvancedTabs,

    // Workspace
    WorkspaceLoadMore,
    WorkspaceCreateTag,
    WorkspaceCreateBranch,
    WorkspaceYankHash,
    WorkspaceCheckout,
    WorkspaceRevert,
    WorkspaceCherryPick,
    WorkspaceInteractiveRebase,
    WorkspaceStashUI,
    WorkspaceCommit,
    WorkspaceCommitAmend,
    WorkspaceFuzzySearch,
    WorkspaceColumnPicker,
    WorkspaceLogsView,
    WorkspaceStage,
    WorkspaceStageAll,
    WorkspaceDiscard,
    WorkspaceDiscardAll,

    // Files
    FilesBlame,
    FilesLineNumbers,
    FilesHistory,
    FilesSearch,
    FilesExpand,
    FilesCollapse,
    FilesEditor,
    FilesFullScreen,

    // Branches
    BranchesCheckout,
    BranchesCreate,
    BranchesDelete,
    BranchesMerge,
    BranchesRebase,
    BranchesInteractiveRebase,
    BranchesPull,
    BranchesPush,
    BranchesSearch,

    // Tags
    TagsCheckout,
    TagsDelete,
    TagsPush,
    TagsPushAll,
    TagsFetch,
    TagsSearch,

    // Remotes
    RemotesAdd,
    RemotesDelete,
    RemotesFetch,

    // Stashes
    StashesApply,
    StashesCreate,
    StashesDelete,

    // Worktrees
    WorktreesOpen,
    WorktreesAdd,
    WorktreesDelete,
    WorktreesLock,
    WorktreesPrune,

    // Submodules
    SubmodulesAdd,
    SubmodulesDelete,

    // Reflog
    ReflogCheckout,

    // Forge
    ForgeCheckout,
    ForgeOpenBrowser,
    ForgeToggleAssigned,
    ForgeAddComment,

    // Diff
    DiffLineMode,
    DiffStage,
    DiffUnstage,
    DiffDiscard,

    // Conflict
    ConflictOurs,
    ConflictTheirs,
    ConflictResolve,
    ConflictAbort,
    ConflictContinue,

    // Detail navigation
    DetailMoveUp,
    DetailMoveDown,
    DetailPageUp,
    DetailPageDown,
    DetailHome,
    DetailEnd,
}

impl Action {
    pub fn from_index(idx: usize) -> Option<Self> {
        match idx {
            14 => Some(Action::ToggleStatusBar),
            15 => Some(Action::Help),
            16 => Some(Action::Close),
            17 => Some(Action::HomeMoveDown),
            18 => Some(Action::HomeMoveUp),
            19 => Some(Action::HomePageDown),
            20 => Some(Action::HomePageUp),
            21 => Some(Action::HomeHome),
            22 => Some(Action::HomeEnd),
            23 => Some(Action::HomeAddRepo),
            24 => Some(Action::HomeBulkAdd),
            25 => Some(Action::HomeEditRepo),
            26 => Some(Action::HomeDeleteRepo),
            27 => Some(Action::HomeOpenDebugLogs),
            28 => Some(Action::HomeEditLabels),
            29 => Some(Action::HomeAbout),
            30 => Some(Action::HomeRefresh),
            31 => Some(Action::HomeCycleSort),
            32 => Some(Action::HomeToggleSortReverse),
            33 => Some(Action::HomeTogglePin),
            34 => Some(Action::HomeOpenSettings),
            35 => Some(Action::HomeImportRepo),
            36 => Some(Action::HomeOpenGitApp),
            37 => Some(Action::HomeSearchRepo),
            38 => Some(Action::HomeOpenDetail),
            39 => Some(Action::CloseDetail),
            40 => Some(Action::DetailHelp),
            41 => Some(Action::CycleFocusForward),
            42 => Some(Action::CycleFocusBackward),
            43 => Some(Action::RefreshDetail),
            44 => Some(Action::CycleTabForward),
            45 => Some(Action::CycleTabBackward),
            46 => Some(Action::GoToTab1),
            47 => Some(Action::GoToTab2),
            48 => Some(Action::GoToTab3),
            49 => Some(Action::GoToTab4),
            50 => Some(Action::GoToTab5),
            51 => Some(Action::GoToTab6),
            52 => Some(Action::GoToTab7),
            57 => Some(Action::HomeCheckUpdate),
            77 => Some(Action::HomeCycleViewMode),
            78 => Some(Action::HomeSymbolsHelp),
            68 => Some(Action::Overview),
            69 => Some(Action::ToggleAdvancedTabs),
            70 => Some(Action::HomeOpenTerminal),
            71 => Some(Action::HomeToggleStar),
            72 => Some(Action::HomeYankPath),
            73 => Some(Action::HomeJumpPicker),
            74 => Some(Action::HomeFetchAll),
            75 => Some(Action::HomeSelect),
            76 => Some(Action::HomeGlobalSearch),

            // Workspace
            100 => Some(Action::WorkspaceLoadMore),
            101 => Some(Action::WorkspaceCreateTag),
            102 => Some(Action::WorkspaceCreateBranch),
            103 => Some(Action::WorkspaceYankHash),
            117 => Some(Action::WorkspaceCheckout),
            104 => Some(Action::WorkspaceRevert),
            105 => Some(Action::WorkspaceCherryPick),
            106 => Some(Action::WorkspaceInteractiveRebase),
            107 => Some(Action::WorkspaceStashUI),
            108 => Some(Action::WorkspaceCommit),
            109 => Some(Action::WorkspaceCommitAmend),
            110 => Some(Action::WorkspaceFuzzySearch),
            111 => Some(Action::WorkspaceColumnPicker),
            112 => Some(Action::WorkspaceLogsView),
            113 => Some(Action::WorkspaceStage),
            114 => Some(Action::WorkspaceStageAll),
            115 => Some(Action::WorkspaceDiscard),
            116 => Some(Action::WorkspaceDiscardAll),

            // Files
            120 => Some(Action::FilesBlame),
            121 => Some(Action::FilesLineNumbers),
            122 => Some(Action::FilesHistory),
            123 => Some(Action::FilesSearch),
            124 => Some(Action::FilesExpand),
            125 => Some(Action::FilesCollapse),
            126 => Some(Action::FilesEditor),
            127 => Some(Action::FilesFullScreen),

            // Branches
            130 => Some(Action::BranchesCheckout),
            131 => Some(Action::BranchesCreate),
            132 => Some(Action::BranchesDelete),
            133 => Some(Action::BranchesMerge),
            134 => Some(Action::BranchesRebase),
            135 => Some(Action::BranchesInteractiveRebase),
            136 => Some(Action::BranchesPull),
            137 => Some(Action::BranchesPush),
            138 => Some(Action::BranchesSearch),

            // Tags
            140 => Some(Action::TagsCheckout),
            141 => Some(Action::TagsDelete),
            142 => Some(Action::TagsPush),
            143 => Some(Action::TagsPushAll),
            144 => Some(Action::TagsFetch),
            145 => Some(Action::TagsSearch),

            // Remotes
            150 => Some(Action::RemotesAdd),
            151 => Some(Action::RemotesDelete),
            152 => Some(Action::RemotesFetch),

            // Stashes
            160 => Some(Action::StashesApply),
            161 => Some(Action::StashesCreate),
            162 => Some(Action::StashesDelete),

            // Worktrees
            170 => Some(Action::WorktreesOpen),
            171 => Some(Action::WorktreesAdd),
            172 => Some(Action::WorktreesDelete),
            173 => Some(Action::WorktreesLock),
            174 => Some(Action::WorktreesPrune),

            // Submodules
            180 => Some(Action::SubmodulesAdd),
            181 => Some(Action::SubmodulesDelete),

            // Reflog
            190 => Some(Action::ReflogCheckout),

            // Forge
            200 => Some(Action::ForgeCheckout),
            201 => Some(Action::ForgeOpenBrowser),
            202 => Some(Action::ForgeToggleAssigned),
            203 => Some(Action::ForgeAddComment),

            // Diff
            210 => Some(Action::DiffLineMode),
            211 => Some(Action::DiffStage),
            212 => Some(Action::DiffUnstage),
            213 => Some(Action::DiffDiscard),

            // Conflict
            220 => Some(Action::ConflictOurs),
            221 => Some(Action::ConflictTheirs),
            222 => Some(Action::ConflictResolve),
            223 => Some(Action::ConflictAbort),
            224 => Some(Action::ConflictContinue),

            // Detail List Scroll
            230 => Some(Action::DetailMoveUp),
            231 => Some(Action::DetailMoveDown),
            232 => Some(Action::DetailPageUp),
            233 => Some(Action::DetailPageDown),
            234 => Some(Action::DetailHome),
            235 => Some(Action::DetailEnd),

            _ => None,
        }
    }

    pub fn to_index(self) -> usize {
        match self {
            Action::ToggleStatusBar => 14,
            Action::Help => 15,
            Action::Close => 16,
            Action::HomeMoveDown => 17,
            Action::HomeMoveUp => 18,
            Action::HomePageDown => 19,
            Action::HomePageUp => 20,
            Action::HomeHome => 21,
            Action::HomeEnd => 22,
            Action::HomeAddRepo => 23,
            Action::HomeBulkAdd => 24,
            Action::HomeEditRepo => 25,
            Action::HomeDeleteRepo => 26,
            Action::HomeOpenDebugLogs => 27,
            Action::HomeEditLabels => 28,
            Action::HomeAbout => 29,
            Action::HomeRefresh => 30,
            Action::HomeCycleSort => 31,
            Action::HomeToggleSortReverse => 32,
            Action::HomeTogglePin => 33,
            Action::HomeOpenSettings => 34,
            Action::HomeImportRepo => 35,
            Action::HomeOpenGitApp => 36,
            Action::HomeSearchRepo => 37,
            Action::HomeOpenDetail => 38,
            Action::CloseDetail => 39,
            Action::DetailHelp => 40,
            Action::CycleFocusForward => 41,
            Action::CycleFocusBackward => 42,
            Action::RefreshDetail => 43,
            Action::CycleTabForward => 44,
            Action::CycleTabBackward => 45,
            Action::GoToTab1 => 46,
            Action::GoToTab2 => 47,
            Action::GoToTab3 => 48,
            Action::GoToTab4 => 49,
            Action::GoToTab5 => 50,
            Action::GoToTab6 => 51,
            Action::GoToTab7 => 52,
            Action::HomeCycleViewMode => 77,
            Action::HomeSymbolsHelp => 78,
            Action::HomeCheckUpdate => 57,
            Action::Overview => 68,
            Action::ToggleAdvancedTabs => 69,
            Action::HomeOpenTerminal => 70,
            Action::HomeToggleStar => 71,
            Action::HomeYankPath => 72,
            Action::HomeJumpPicker => 73,
            Action::HomeFetchAll => 74,
            Action::HomeSelect => 75,
            Action::HomeGlobalSearch => 76,

            // Workspace
            Action::WorkspaceLoadMore => 100,
            Action::WorkspaceCreateTag => 101,
            Action::WorkspaceCreateBranch => 102,
            Action::WorkspaceYankHash => 103,
            Action::WorkspaceCheckout => 117,
            Action::WorkspaceRevert => 104,
            Action::WorkspaceCherryPick => 105,
            Action::WorkspaceInteractiveRebase => 106,
            Action::WorkspaceStashUI => 107,
            Action::WorkspaceCommit => 108,
            Action::WorkspaceCommitAmend => 109,
            Action::WorkspaceFuzzySearch => 110,
            Action::WorkspaceColumnPicker => 111,
            Action::WorkspaceLogsView => 112,
            Action::WorkspaceStage => 113,
            Action::WorkspaceStageAll => 114,
            Action::WorkspaceDiscard => 115,
            Action::WorkspaceDiscardAll => 116,

            // Files
            Action::FilesBlame => 120,
            Action::FilesLineNumbers => 121,
            Action::FilesHistory => 122,
            Action::FilesSearch => 123,
            Action::FilesExpand => 124,
            Action::FilesCollapse => 125,
            Action::FilesEditor => 126,
            Action::FilesFullScreen => 127,

            // Branches
            Action::BranchesCheckout => 130,
            Action::BranchesCreate => 131,
            Action::BranchesDelete => 132,
            Action::BranchesMerge => 133,
            Action::BranchesRebase => 134,
            Action::BranchesInteractiveRebase => 135,
            Action::BranchesPull => 136,
            Action::BranchesPush => 137,
            Action::BranchesSearch => 138,

            // Tags
            Action::TagsCheckout => 140,
            Action::TagsDelete => 141,
            Action::TagsPush => 142,
            Action::TagsPushAll => 143,
            Action::TagsFetch => 144,
            Action::TagsSearch => 145,

            // Remotes
            Action::RemotesAdd => 150,
            Action::RemotesDelete => 151,
            Action::RemotesFetch => 152,

            // Stashes
            Action::StashesApply => 160,
            Action::StashesCreate => 161,
            Action::StashesDelete => 162,

            // Worktrees
            Action::WorktreesOpen => 170,
            Action::WorktreesAdd => 171,
            Action::WorktreesDelete => 172,
            Action::WorktreesLock => 173,
            Action::WorktreesPrune => 174,

            // Submodules
            Action::SubmodulesAdd => 180,
            Action::SubmodulesDelete => 181,

            // Reflog
            Action::ReflogCheckout => 190,

            // Forge
            Action::ForgeCheckout => 200,
            Action::ForgeOpenBrowser => 201,
            Action::ForgeToggleAssigned => 202,
            Action::ForgeAddComment => 203,

            // Diff
            Action::DiffLineMode => 210,
            Action::DiffStage => 211,
            Action::DiffUnstage => 212,
            Action::DiffDiscard => 213,

            // Conflict
            Action::ConflictOurs => 220,
            Action::ConflictTheirs => 221,
            Action::ConflictResolve => 222,
            Action::ConflictAbort => 223,
            Action::ConflictContinue => 224,

            // Detail List Scroll
            Action::DetailMoveUp => 230,
            Action::DetailMoveDown => 231,
            Action::DetailPageUp => 232,
            Action::DetailPageDown => 233,
            Action::DetailHome => 234,
            Action::DetailEnd => 235,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Keybind {
    pub keys: Vec<String>,
    pub description: String,
}

impl Keybind {
    pub fn new(keys: &[&str], description: &str) -> Self {
        Self {
            keys: keys.iter().map(|s| s.to_string()).collect(),
            description: description.to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct GlobalKeybindings {
    pub toggle_status_bar: Option<Keybind>,
    pub help: Option<Keybind>,
    pub close: Option<Keybind>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct HomeKeybindings {
    pub move_down: Option<Keybind>,
    pub move_up: Option<Keybind>,
    pub page_down: Option<Keybind>,
    pub page_up: Option<Keybind>,
    pub home: Option<Keybind>,
    pub end: Option<Keybind>,
    pub add_repo: Option<Keybind>,
    pub bulk_add: Option<Keybind>,
    pub edit_repo: Option<Keybind>,
    pub delete_repo: Option<Keybind>,
    pub open_debug_logs: Option<Keybind>,
    pub edit_labels: Option<Keybind>,
    pub about: Option<Keybind>,
    pub symbols_help: Option<Keybind>,
    pub refresh: Option<Keybind>,
    pub cycle_sort: Option<Keybind>,
    pub toggle_sort_reverse: Option<Keybind>,
    pub toggle_pin: Option<Keybind>,
    pub open_settings: Option<Keybind>,
    pub import_repo: Option<Keybind>,
    pub open_git_app: Option<Keybind>,
    pub search_repo: Option<Keybind>,
    pub open_detail: Option<Keybind>,
    pub check_update: Option<Keybind>,
    pub cycle_view_mode: Option<Keybind>,
    pub open_terminal: Option<Keybind>,
    pub toggle_star: Option<Keybind>,
    pub yank_path: Option<Keybind>,
    pub jump_picker: Option<Keybind>,
    pub fetch_all: Option<Keybind>,
    pub select: Option<Keybind>,
    pub global_search: Option<Keybind>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct NavigationKeybindings {
    pub close_detail: Option<Keybind>,
    pub detail_help: Option<Keybind>,
    pub cycle_focus_forward: Option<Keybind>,
    pub cycle_focus_backward: Option<Keybind>,
    pub refresh_detail: Option<Keybind>,
    pub cycle_tab_forward: Option<Keybind>,
    pub cycle_tab_backward: Option<Keybind>,
    pub go_to_tab_1: Option<Keybind>,
    pub go_to_tab_2: Option<Keybind>,
    pub go_to_tab_3: Option<Keybind>,
    pub go_to_tab_4: Option<Keybind>,
    pub go_to_tab_5: Option<Keybind>,
    pub go_to_tab_6: Option<Keybind>,
    pub go_to_tab_7: Option<Keybind>,
    pub overview: Option<Keybind>,
    pub toggle_advanced_tabs: Option<Keybind>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct WorkspaceKeybindings {
    pub load_more: Option<Keybind>,
    pub create_tag: Option<Keybind>,
    pub create_branch: Option<Keybind>,
    pub yank_hash: Option<Keybind>,
    pub checkout: Option<Keybind>,
    pub revert: Option<Keybind>,
    pub cherry_pick: Option<Keybind>,
    pub interactive_rebase: Option<Keybind>,
    pub stash_ui: Option<Keybind>,
    pub commit: Option<Keybind>,
    pub commit_amend: Option<Keybind>,
    pub fuzzy_search: Option<Keybind>,
    pub column_picker: Option<Keybind>,
    pub logs_view: Option<Keybind>,
    pub stage: Option<Keybind>,
    pub stage_all: Option<Keybind>,
    pub discard: Option<Keybind>,
    pub discard_all: Option<Keybind>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct FilesKeybindings {
    pub blame: Option<Keybind>,
    pub line_numbers: Option<Keybind>,
    pub history: Option<Keybind>,
    pub search: Option<Keybind>,
    pub expand: Option<Keybind>,
    pub collapse: Option<Keybind>,
    pub editor: Option<Keybind>,
    pub full_screen: Option<Keybind>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct BranchesKeybindings {
    pub checkout: Option<Keybind>,
    pub create: Option<Keybind>,
    pub delete: Option<Keybind>,
    pub merge: Option<Keybind>,
    pub rebase: Option<Keybind>,
    pub interactive_rebase: Option<Keybind>,
    pub pull: Option<Keybind>,
    pub push: Option<Keybind>,
    pub search: Option<Keybind>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct TagsKeybindings {
    pub checkout: Option<Keybind>,
    pub delete: Option<Keybind>,
    pub push: Option<Keybind>,
    pub push_all: Option<Keybind>,
    pub fetch: Option<Keybind>,
    pub search: Option<Keybind>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct RemotesKeybindings {
    pub add: Option<Keybind>,
    pub delete: Option<Keybind>,
    pub fetch: Option<Keybind>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct StashesKeybindings {
    pub apply: Option<Keybind>,
    pub create: Option<Keybind>,
    pub delete: Option<Keybind>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct WorktreesKeybindings {
    pub open: Option<Keybind>,
    pub add: Option<Keybind>,
    pub delete: Option<Keybind>,
    pub lock: Option<Keybind>,
    pub prune: Option<Keybind>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct SubmodulesKeybindings {
    pub add: Option<Keybind>,
    pub delete: Option<Keybind>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct ReflogKeybindings {
    pub checkout: Option<Keybind>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct ForgeKeybindings {
    pub checkout: Option<Keybind>,
    pub open_browser: Option<Keybind>,
    pub toggle_assigned: Option<Keybind>,
    pub add_comment: Option<Keybind>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct DiffKeybindings {
    pub line_mode: Option<Keybind>,
    pub stage: Option<Keybind>,
    pub unstage: Option<Keybind>,
    pub discard: Option<Keybind>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct ConflictKeybindings {
    pub ours: Option<Keybind>,
    pub theirs: Option<Keybind>,
    pub resolve: Option<Keybind>,
    pub abort: Option<Keybind>,
    pub continue_merge: Option<Keybind>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct DetailKeybindings {
    pub move_up: Option<Keybind>,
    pub move_down: Option<Keybind>,
    pub page_up: Option<Keybind>,
    pub page_down: Option<Keybind>,
    pub home: Option<Keybind>,
    pub end: Option<Keybind>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
pub struct KeybindingsConfig {
    #[serde(default)]
    pub global: GlobalKeybindings,
    #[serde(default)]
    pub home: HomeKeybindings,
    #[serde(default)]
    pub navigation: NavigationKeybindings,
    #[serde(default)]
    pub workspace: WorkspaceKeybindings,
    #[serde(default)]
    pub files: FilesKeybindings,
    #[serde(default)]
    pub branches: BranchesKeybindings,
    #[serde(default)]
    pub tags: TagsKeybindings,
    #[serde(default)]
    pub remotes: RemotesKeybindings,
    #[serde(default)]
    pub stashes: StashesKeybindings,
    #[serde(default)]
    pub worktrees: WorktreesKeybindings,
    #[serde(default)]
    pub submodules: SubmodulesKeybindings,
    #[serde(default)]
    pub reflog: ReflogKeybindings,
    #[serde(default)]
    pub forge: ForgeKeybindings,
    #[serde(default)]
    pub diff: DiffKeybindings,
    #[serde(default)]
    pub conflict: ConflictKeybindings,
    #[serde(default)]
    pub detail: DetailKeybindings,
}

pub fn parse_key(s: &str) -> Option<(KeyCode, KeyModifiers)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let parts: Vec<&str> = s.split('-').collect();
    let mut modifiers = KeyModifiers::empty();
    let key_str = if parts.len() > 1 {
        for part in &parts[..parts.len() - 1] {
            match part.to_lowercase().as_str() {
                "ctrl" | "control" => modifiers.insert(KeyModifiers::CONTROL),
                "alt" | "meta" => modifiers.insert(KeyModifiers::ALT),
                "shift" => modifiers.insert(KeyModifiers::SHIFT),
                _ => {}
            }
        }
        parts.last().cloned().unwrap_or("")
    } else {
        s
    };

    let key_lower = key_str.to_lowercase();
    let code = match key_lower.as_str() {
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "enter" | "return" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Esc,
        "backspace" => KeyCode::Backspace,
        "tab" => KeyCode::Tab,
        "backtab" => KeyCode::BackTab,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "pgup" => KeyCode::PageUp,
        "pagedown" | "pgdn" => KeyCode::PageDown,
        "delete" | "del" => KeyCode::Delete,
        "space" => KeyCode::Char(' '),
        "comma" => KeyCode::Char(','),
        "dot" | "period" => KeyCode::Char('.'),
        _ => {
            if let Some(c) = key_str.chars().next() {
                if key_str.len() == 1 {
                    KeyCode::Char(c)
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
    };

    Some((code, modifiers))
}

pub fn keys_equal(
    code_a: KeyCode,
    mods_a: KeyModifiers,
    code_b: KeyCode,
    mods_b: KeyModifiers,
) -> bool {
    let (mut code_a, mut mods_a) = (code_a, mods_a);
    let (mut code_b, mut mods_b) = (code_b, mods_b);

    if code_a == KeyCode::BackTab {
        code_a = KeyCode::Tab;
        mods_a.insert(KeyModifiers::SHIFT);
    }
    if code_b == KeyCode::BackTab {
        code_b = KeyCode::Tab;
        mods_b.insert(KeyModifiers::SHIFT);
    }

    if let (KeyCode::Char(c_a), KeyCode::Char(c_b)) = (code_a, code_b) {
        if c_a != c_b {
            return false;
        }
        let mask = KeyModifiers::CONTROL | KeyModifiers::ALT;
        return (mods_a & mask) == (mods_b & mask);
    }

    if code_a != code_b {
        return false;
    }

    let mask = KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT;
    (mods_a & mask) == (mods_b & mask)
}

impl KeybindingsConfig {
    pub fn default_config() -> Self {
        Self {
            global: GlobalKeybindings {
                toggle_status_bar: Some(Keybind::new(&["."], "Toggle status bar visibility")),
                help: Some(Keybind::new(&["?"], "Show help overlay")),
                close: Some(Keybind::new(&["ctrl-q"], "Quit Gitwig")),
            },
            home: HomeKeybindings {
                move_down: Some(Keybind::new(&["j", "down"], "Move selection down")),
                move_up: Some(Keybind::new(&["k", "up"], "Move selection up")),
                page_down: Some(Keybind::new(&["pagedown"], "Scroll selection down by page")),
                page_up: Some(Keybind::new(&["pageup"], "Scroll selection up by page")),
                home: Some(Keybind::new(&["home"], "Go to top of list")),
                end: Some(Keybind::new(&["end"], "Go to bottom of list")),
                add_repo: Some(Keybind::new(&["a"], "Add repository path manually")),
                bulk_add: Some(Keybind::new(&["A"], "Bulk add repositories in a directory")),
                edit_repo: Some(Keybind::new(&["e"], "Edit selected repository details")),
                delete_repo: Some(Keybind::new(&["D"], "Delete selected repository from config")),
                open_debug_logs: Some(Keybind::new(&["d"], "Open debug logs popup")),
                edit_labels: Some(Keybind::new(
                    &["l"],
                    "Edit custom labels of selected repository",
                )),
                about: Some(Keybind::new(&["V"], "Open about dialog")),
                symbols_help: Some(Keybind::new(&["h"], "Show signs & symbols legend popup")),
                refresh: Some(Keybind::new(&["R"], "Refresh selected repository status")),
                cycle_sort: Some(Keybind::new(&["o"], "Cycle sorting criteria")),
                toggle_sort_reverse: Some(Keybind::new(
                    &["O"],
                    "Toggle sorting direction (ascending/descending)",
                )),
                toggle_pin: Some(Keybind::new(&["p"], "Toggle pin status of selected repository")),
                open_settings: Some(Keybind::new(&["s"], "Open settings view")),
                import_repo: Some(Keybind::new(&["i"], "Import / clone remote repository")),
                open_git_app: Some(Keybind::new(&["g"], "Launch preferred external Git client")),
                search_repo: Some(Keybind::new(
                    &["f"],
                    "Enter search query to filter repositories",
                )),
                open_detail: Some(Keybind::new(
                    &["enter", "right"],
                    "Open selected repository detail view",
                )),
                check_update: Some(Keybind::new(&["u"], "Check for application updates")),
                cycle_view_mode: Some(Keybind::new(
                    &["v"],
                    "Cycle repository list layout (Normal/Compact/Tile)",
                )),
                open_terminal: Some(Keybind::new(&["t"], "Open terminal shell at repository path")),
                toggle_star: Some(Keybind::new(
                    &["*"],
                    "Toggle Starred/Favorite status of repository",
                )),
                yank_path: Some(Keybind::new(&["y"], "Yank repository absolute path to clipboard")),
                jump_picker: Some(Keybind::new(&["/"], "Open fuzzy Jump-to-Repo picker overlay")),
                fetch_all: Some(Keybind::new(
                    &["F"],
                    "Fetch all tracked repositories concurrently",
                )),
                select: Some(Keybind::new(&["space"], "Toggle selection for batch operations")),
                global_search: Some(Keybind::new(&["ctrl-f"], "Open global code search popup")),
            },
            navigation: NavigationKeybindings {
                close_detail: Some(Keybind::new(&["esc", "q", "Q"], "Close detail view / Go back")),
                detail_help: Some(Keybind::new(&["?"], "Show detail view help overlay")),
                cycle_focus_forward: Some(Keybind::new(
                    &["w"],
                    "Cycle focus forward through panels",
                )),
                cycle_focus_backward: Some(Keybind::new(
                    &["W"],
                    "Cycle focus backward through panels",
                )),
                refresh_detail: Some(Keybind::new(&["R"], "Resync active tab details manually")),
                cycle_tab_forward: Some(Keybind::new(&["tab"], "Cycle tab forward")),
                cycle_tab_backward: Some(Keybind::new(
                    &["backtab", "shift-tab"],
                    "Cycle tab backward",
                )),
                go_to_tab_1: Some(Keybind::new(&["1"], "Go to Tab 1")),
                go_to_tab_2: Some(Keybind::new(&["2"], "Go to Tab 2")),
                go_to_tab_3: Some(Keybind::new(&["3"], "Go to Tab 3")),
                go_to_tab_4: Some(Keybind::new(&["4"], "Go to Tab 4")),
                go_to_tab_5: Some(Keybind::new(&["5"], "Go to Tab 5")),
                go_to_tab_6: Some(Keybind::new(&["6"], "Go to Tab 6")),
                go_to_tab_7: Some(Keybind::new(&["7"], "Go to Tab 7")),
                overview: Some(Keybind::new(&["O"], "Show repository Overview screen")),
                toggle_advanced_tabs: Some(Keybind::new(
                    &["Z"],
                    "Toggle between Primary and Advanced tab groups",
                )),
            },
            workspace: WorkspaceKeybindings {
                load_more: Some(Keybind::new(&["G"], "Load more commits")),
                create_tag: Some(Keybind::new(&["t", "T"], "Create tag at selected commit")),
                create_branch: Some(Keybind::new(&["b", "B"], "Create branch at selected commit")),
                yank_hash: Some(Keybind::new(&["y", "Y"], "Yank selected commit hash")),
                checkout: Some(Keybind::new(&["o", "O"], "Checkout selected commit")),
                revert: Some(Keybind::new(&["v", "V"], "Revert selected commit")),
                cherry_pick: Some(Keybind::new(&["p", "P"], "Cherry-pick selected commit")),
                interactive_rebase: Some(Keybind::new(
                    &["i", "I"],
                    "Interactive rebase from selected commit",
                )),
                stash_ui: Some(Keybind::new(&["s", "S"], "Open stashing UI panel")),
                commit: Some(Keybind::new(&["c"], "Commit changes")),
                commit_amend: Some(Keybind::new(&["C"], "Commit and amend last commit")),
                fuzzy_search: Some(Keybind::new(&["/"], "Fuzzy search commits list")),
                column_picker: Some(Keybind::new(&["f"], "Open search column picker")),
                logs_view: Some(Keybind::new(&["l", "L"], "Open full-screen Logs view")),
                stage: Some(Keybind::new(&["enter"], "Stage / unstage selected file")),
                stage_all: Some(Keybind::new(&["a", "A"], "Stage all / Unstage all files")),
                discard: Some(Keybind::new(&["x"], "Discard changes in selected unstaged file")),
                discard_all: Some(Keybind::new(&["X"], "Discard all unstaged changes")),
            },
            files: FilesKeybindings {
                blame: Some(Keybind::new(&["b", "B"], "Toggle git blame panel")),
                line_numbers: Some(Keybind::new(
                    &["n", "N"],
                    "Toggle line numbers in content viewer",
                )),
                history: Some(Keybind::new(&["H"], "View commit history of the selected file")),
                search: Some(Keybind::new(&["/"], "Launch fuzzy file search picker")),
                expand: Some(Keybind::new(&[">", "."], "Expand folder in tree")),
                collapse: Some(Keybind::new(&["<", ","], "Collapse folder in tree")),
                editor: Some(Keybind::new(&["e", "o"], "Open selected file in terminal editor")),
                full_screen: Some(Keybind::new(
                    &["right"],
                    "Toggle full-screen view mode in viewer",
                )),
            },
            branches: BranchesKeybindings {
                checkout: Some(Keybind::new(&["enter"], "Checkout selected branch")),
                create: Some(Keybind::new(&["c", "C"], "Create new branch")),
                delete: Some(Keybind::new(&["D"], "Delete selected branch")),
                merge: Some(Keybind::new(&["m", "M"], "Merge selected branch into current branch")),
                rebase: Some(Keybind::new(&["r"], "Rebase current branch onto selected branch")),
                interactive_rebase: Some(Keybind::new(
                    &["i", "I"],
                    "Interactive rebase of current branch onto selected branch",
                )),
                pull: Some(Keybind::new(&["p"], "Pull remote changes")),
                push: Some(Keybind::new(&["P"], "Push selected branch to remote")),
                search: Some(Keybind::new(&["/"], "Fuzzy search branches")),
            },
            tags: TagsKeybindings {
                checkout: Some(Keybind::new(&["enter"], "Checkout selected tag")),
                delete: Some(Keybind::new(&["D"], "Delete selected tag")),
                push: Some(Keybind::new(&["p"], "Push selected tag to remote")),
                push_all: Some(Keybind::new(&["P"], "Push all tags to remote")),
                fetch: Some(Keybind::new(&["f", "F"], "Fetch remote tags")),
                search: Some(Keybind::new(&["/"], "Fuzzy search tags")),
            },
            remotes: RemotesKeybindings {
                add: Some(Keybind::new(&["a", "A"], "Add new remote")),
                delete: Some(Keybind::new(&["D"], "Delete selected remote")),
                fetch: Some(Keybind::new(&["f", "F"], "Fetch selected remote")),
            },
            stashes: StashesKeybindings {
                apply: Some(Keybind::new(&["a", "A"], "Apply selected stash")),
                create: Some(Keybind::new(&["s", "S"], "Create new stash")),
                delete: Some(Keybind::new(&["D"], "Delete selected stash")),
            },
            worktrees: WorktreesKeybindings {
                open: Some(Keybind::new(
                    &["enter"],
                    "Open selected worktree in new Gitwig context",
                )),
                add: Some(Keybind::new(&["a"], "Add new worktree")),
                delete: Some(Keybind::new(&["D"], "Remove selected worktree")),
                lock: Some(Keybind::new(&["l"], "Toggle lock status of selected worktree")),
                prune: Some(Keybind::new(&["p"], "Prune stale worktree metadata")),
            },
            submodules: SubmodulesKeybindings {
                add: Some(Keybind::new(&["a"], "Add new submodule")),
                delete: Some(Keybind::new(&["D"], "Delete selected submodule")),
            },
            reflog: ReflogKeybindings {
                checkout: Some(Keybind::new(
                    &["enter", "space"],
                    "Checkout commit OID of selected reflog entry",
                )),
            },
            forge: ForgeKeybindings {
                checkout: Some(Keybind::new(
                    &["enter"],
                    "Checkout branch corresponding to selected issue",
                )),
                open_browser: Some(Keybind::new(&["o"], "Open selected issue in web browser")),
                toggle_assigned: Some(Keybind::new(
                    &["a"],
                    "Toggle between all issues and assigned issues",
                )),
                add_comment: Some(Keybind::new(
                    &["n"],
                    "Add a line comment to the current Pull Request",
                )),
            },
            diff: DiffKeybindings {
                line_mode: Some(Keybind::new(
                    &["l", "L"],
                    "Toggle line-by-line stage/discard mode",
                )),
                stage: Some(Keybind::new(&["s", "S"], "Stage selected hunk/line")),
                unstage: Some(Keybind::new(&["u", "U"], "Unstage selected hunk/line")),
                discard: Some(Keybind::new(&["x", "delete"], "Discard selected hunk/line")),
            },
            conflict: ConflictKeybindings {
                ours: Some(Keybind::new(&["o"], "Accept OURS version of conflict")),
                theirs: Some(Keybind::new(&["t"], "Accept THEIRS version of conflict")),
                resolve: Some(Keybind::new(&["r"], "Mark conflict as resolved")),
                abort: Some(Keybind::new(&["A"], "Abort merge")),
                continue_merge: Some(Keybind::new(&["C"], "Continue merge")),
            },
            detail: DetailKeybindings {
                move_up: Some(Keybind::new(&["k", "up"], "Move selection up in detail panels")),
                move_down: Some(Keybind::new(
                    &["j", "down"],
                    "Move selection down in detail panels",
                )),
                page_up: Some(Keybind::new(
                    &["pageup"],
                    "Scroll selection up by page in detail panels",
                )),
                page_down: Some(Keybind::new(
                    &["pagedown"],
                    "Scroll selection down by page in detail panels",
                )),
                home: Some(Keybind::new(&["home"], "Go to top of list in detail panels")),
                end: Some(Keybind::new(&["end"], "Go to bottom of list in detail panels")),
            },
        }
    }

    pub fn get_default_keys(action: Action) -> Vec<String> {
        let defaults = Self::default_config();
        defaults.get_action_keys(action)
    }

    pub fn get_action_keys(&self, action: Action) -> Vec<String> {
        let keys_opt = match action {
            // Global
            Action::ToggleStatusBar => self.global.toggle_status_bar.as_ref(),
            Action::Help => self.global.help.as_ref(),
            Action::Close => self.global.close.as_ref(),

            // Home
            Action::HomeMoveDown => self.home.move_down.as_ref(),
            Action::HomeMoveUp => self.home.move_up.as_ref(),
            Action::HomePageDown => self.home.page_down.as_ref(),
            Action::HomePageUp => self.home.page_up.as_ref(),
            Action::HomeHome => self.home.home.as_ref(),
            Action::HomeEnd => self.home.end.as_ref(),
            Action::HomeAddRepo => self.home.add_repo.as_ref(),
            Action::HomeBulkAdd => self.home.bulk_add.as_ref(),
            Action::HomeEditRepo => self.home.edit_repo.as_ref(),
            Action::HomeDeleteRepo => self.home.delete_repo.as_ref(),
            Action::HomeOpenDebugLogs => self.home.open_debug_logs.as_ref(),
            Action::HomeEditLabels => self.home.edit_labels.as_ref(),
            Action::HomeAbout => self.home.about.as_ref(),
            Action::HomeSymbolsHelp => self.home.symbols_help.as_ref(),
            Action::HomeRefresh => self.home.refresh.as_ref(),
            Action::HomeCycleSort => self.home.cycle_sort.as_ref(),
            Action::HomeToggleSortReverse => self.home.toggle_sort_reverse.as_ref(),
            Action::HomeTogglePin => self.home.toggle_pin.as_ref(),
            Action::HomeOpenSettings => self.home.open_settings.as_ref(),
            Action::HomeImportRepo => self.home.import_repo.as_ref(),
            Action::HomeOpenGitApp => self.home.open_git_app.as_ref(),
            Action::HomeSearchRepo => self.home.search_repo.as_ref(),
            Action::HomeOpenDetail => self.home.open_detail.as_ref(),
            Action::HomeCheckUpdate => self.home.check_update.as_ref(),
            Action::HomeCycleViewMode => self.home.cycle_view_mode.as_ref(),
            Action::HomeOpenTerminal => self.home.open_terminal.as_ref(),
            Action::HomeToggleStar => self.home.toggle_star.as_ref(),
            Action::HomeYankPath => self.home.yank_path.as_ref(),
            Action::HomeJumpPicker => self.home.jump_picker.as_ref(),
            Action::HomeFetchAll => self.home.fetch_all.as_ref(),
            Action::HomeSelect => self.home.select.as_ref(),
            Action::HomeGlobalSearch => self.home.global_search.as_ref(),

            // Navigation
            Action::CloseDetail => self.navigation.close_detail.as_ref(),
            Action::DetailHelp => self.navigation.detail_help.as_ref(),
            Action::CycleFocusForward => self.navigation.cycle_focus_forward.as_ref(),
            Action::CycleFocusBackward => self.navigation.cycle_focus_backward.as_ref(),
            Action::RefreshDetail => self.navigation.refresh_detail.as_ref(),
            Action::CycleTabForward => self.navigation.cycle_tab_forward.as_ref(),
            Action::CycleTabBackward => self.navigation.cycle_tab_backward.as_ref(),
            Action::GoToTab1 => self.navigation.go_to_tab_1.as_ref(),
            Action::GoToTab2 => self.navigation.go_to_tab_2.as_ref(),
            Action::GoToTab3 => self.navigation.go_to_tab_3.as_ref(),
            Action::GoToTab4 => self.navigation.go_to_tab_4.as_ref(),
            Action::GoToTab5 => self.navigation.go_to_tab_5.as_ref(),
            Action::GoToTab6 => self.navigation.go_to_tab_6.as_ref(),
            Action::GoToTab7 => self.navigation.go_to_tab_7.as_ref(),
            Action::Overview => self.navigation.overview.as_ref(),
            Action::ToggleAdvancedTabs => self.navigation.toggle_advanced_tabs.as_ref(),

            // Workspace
            Action::WorkspaceLoadMore => self.workspace.load_more.as_ref(),
            Action::WorkspaceCreateTag => self.workspace.create_tag.as_ref(),
            Action::WorkspaceCreateBranch => self.workspace.create_branch.as_ref(),
            Action::WorkspaceYankHash => self.workspace.yank_hash.as_ref(),
            Action::WorkspaceCheckout => self.workspace.checkout.as_ref(),
            Action::WorkspaceRevert => self.workspace.revert.as_ref(),
            Action::WorkspaceCherryPick => self.workspace.cherry_pick.as_ref(),
            Action::WorkspaceInteractiveRebase => self.workspace.interactive_rebase.as_ref(),
            Action::WorkspaceStashUI => self.workspace.stash_ui.as_ref(),
            Action::WorkspaceCommit => self.workspace.commit.as_ref(),
            Action::WorkspaceCommitAmend => self.workspace.commit_amend.as_ref(),
            Action::WorkspaceFuzzySearch => self.workspace.fuzzy_search.as_ref(),
            Action::WorkspaceColumnPicker => self.workspace.column_picker.as_ref(),
            Action::WorkspaceLogsView => self.workspace.logs_view.as_ref(),
            Action::WorkspaceStage => self.workspace.stage.as_ref(),
            Action::WorkspaceStageAll => self.workspace.stage_all.as_ref(),
            Action::WorkspaceDiscard => self.workspace.discard.as_ref(),
            Action::WorkspaceDiscardAll => self.workspace.discard_all.as_ref(),

            // Files
            Action::FilesBlame => self.files.blame.as_ref(),
            Action::FilesLineNumbers => self.files.line_numbers.as_ref(),
            Action::FilesHistory => self.files.history.as_ref(),
            Action::FilesSearch => self.files.search.as_ref(),
            Action::FilesExpand => self.files.expand.as_ref(),
            Action::FilesCollapse => self.files.collapse.as_ref(),
            Action::FilesEditor => self.files.editor.as_ref(),
            Action::FilesFullScreen => self.files.full_screen.as_ref(),

            // Branches
            Action::BranchesCheckout => self.branches.checkout.as_ref(),
            Action::BranchesCreate => self.branches.create.as_ref(),
            Action::BranchesDelete => self.branches.delete.as_ref(),
            Action::BranchesMerge => self.branches.merge.as_ref(),
            Action::BranchesRebase => self.branches.rebase.as_ref(),
            Action::BranchesInteractiveRebase => self.branches.interactive_rebase.as_ref(),
            Action::BranchesPull => self.branches.pull.as_ref(),
            Action::BranchesPush => self.branches.push.as_ref(),
            Action::BranchesSearch => self.branches.search.as_ref(),

            // Tags
            Action::TagsCheckout => self.tags.checkout.as_ref(),
            Action::TagsDelete => self.tags.delete.as_ref(),
            Action::TagsPush => self.tags.push.as_ref(),
            Action::TagsPushAll => self.tags.push_all.as_ref(),
            Action::TagsFetch => self.tags.fetch.as_ref(),
            Action::TagsSearch => self.tags.search.as_ref(),

            // Remotes
            Action::RemotesAdd => self.remotes.add.as_ref(),
            Action::RemotesDelete => self.remotes.delete.as_ref(),
            Action::RemotesFetch => self.remotes.fetch.as_ref(),

            // Stashes
            Action::StashesApply => self.stashes.apply.as_ref(),
            Action::StashesCreate => self.stashes.create.as_ref(),
            Action::StashesDelete => self.stashes.delete.as_ref(),

            // Worktrees
            Action::WorktreesOpen => self.worktrees.open.as_ref(),
            Action::WorktreesAdd => self.worktrees.add.as_ref(),
            Action::WorktreesDelete => self.worktrees.delete.as_ref(),
            Action::WorktreesLock => self.worktrees.lock.as_ref(),
            Action::WorktreesPrune => self.worktrees.prune.as_ref(),

            // Submodules
            Action::SubmodulesAdd => self.submodules.add.as_ref(),
            Action::SubmodulesDelete => self.submodules.delete.as_ref(),

            // Reflog
            Action::ReflogCheckout => self.reflog.checkout.as_ref(),

            // Forge
            Action::ForgeCheckout => self.forge.checkout.as_ref(),
            Action::ForgeOpenBrowser => self.forge.open_browser.as_ref(),
            Action::ForgeToggleAssigned => self.forge.toggle_assigned.as_ref(),
            Action::ForgeAddComment => self.forge.add_comment.as_ref(),

            // Diff
            Action::DiffLineMode => self.diff.line_mode.as_ref(),
            Action::DiffStage => self.diff.stage.as_ref(),
            Action::DiffUnstage => self.diff.unstage.as_ref(),
            Action::DiffDiscard => self.diff.discard.as_ref(),

            // Conflict
            Action::ConflictOurs => self.conflict.ours.as_ref(),
            Action::ConflictTheirs => self.conflict.theirs.as_ref(),
            Action::ConflictResolve => self.conflict.resolve.as_ref(),
            Action::ConflictAbort => self.conflict.abort.as_ref(),
            Action::ConflictContinue => self.conflict.continue_merge.as_ref(),

            // Detail
            Action::DetailMoveUp => self.detail.move_up.as_ref(),
            Action::DetailMoveDown => self.detail.move_down.as_ref(),
            Action::DetailPageUp => self.detail.page_up.as_ref(),
            Action::DetailPageDown => self.detail.page_down.as_ref(),
            Action::DetailHome => self.detail.home.as_ref(),
            Action::DetailEnd => self.detail.end.as_ref(),
        };

        keys_opt.map(|k| k.keys.clone()).unwrap_or_default()
    }

    pub fn get_action_description(&self, action: Action) -> String {
        let desc_opt = match action {
            // Global
            Action::ToggleStatusBar => self.global.toggle_status_bar.as_ref(),
            Action::Help => self.global.help.as_ref(),
            Action::Close => self.global.close.as_ref(),

            // Home
            Action::HomeMoveDown => self.home.move_down.as_ref(),
            Action::HomeMoveUp => self.home.move_up.as_ref(),
            Action::HomePageDown => self.home.page_down.as_ref(),
            Action::HomePageUp => self.home.page_up.as_ref(),
            Action::HomeHome => self.home.home.as_ref(),
            Action::HomeEnd => self.home.end.as_ref(),
            Action::HomeAddRepo => self.home.add_repo.as_ref(),
            Action::HomeBulkAdd => self.home.bulk_add.as_ref(),
            Action::HomeEditRepo => self.home.edit_repo.as_ref(),
            Action::HomeDeleteRepo => self.home.delete_repo.as_ref(),
            Action::HomeOpenDebugLogs => self.home.open_debug_logs.as_ref(),
            Action::HomeEditLabels => self.home.edit_labels.as_ref(),
            Action::HomeAbout => self.home.about.as_ref(),
            Action::HomeSymbolsHelp => self.home.symbols_help.as_ref(),
            Action::HomeRefresh => self.home.refresh.as_ref(),
            Action::HomeCycleSort => self.home.cycle_sort.as_ref(),
            Action::HomeToggleSortReverse => self.home.toggle_sort_reverse.as_ref(),
            Action::HomeTogglePin => self.home.toggle_pin.as_ref(),
            Action::HomeOpenSettings => self.home.open_settings.as_ref(),
            Action::HomeImportRepo => self.home.import_repo.as_ref(),
            Action::HomeOpenGitApp => self.home.open_git_app.as_ref(),
            Action::HomeSearchRepo => self.home.search_repo.as_ref(),
            Action::HomeOpenDetail => self.home.open_detail.as_ref(),
            Action::HomeCheckUpdate => self.home.check_update.as_ref(),
            Action::HomeCycleViewMode => self.home.cycle_view_mode.as_ref(),
            Action::HomeOpenTerminal => self.home.open_terminal.as_ref(),
            Action::HomeToggleStar => self.home.toggle_star.as_ref(),
            Action::HomeYankPath => self.home.yank_path.as_ref(),
            Action::HomeJumpPicker => self.home.jump_picker.as_ref(),
            Action::HomeFetchAll => self.home.fetch_all.as_ref(),
            Action::HomeSelect => self.home.select.as_ref(),
            Action::HomeGlobalSearch => self.home.global_search.as_ref(),

            // Navigation
            Action::CloseDetail => self.navigation.close_detail.as_ref(),
            Action::DetailHelp => self.navigation.detail_help.as_ref(),
            Action::CycleFocusForward => self.navigation.cycle_focus_forward.as_ref(),
            Action::CycleFocusBackward => self.navigation.cycle_focus_backward.as_ref(),
            Action::RefreshDetail => self.navigation.refresh_detail.as_ref(),
            Action::CycleTabForward => self.navigation.cycle_tab_forward.as_ref(),
            Action::CycleTabBackward => self.navigation.cycle_tab_backward.as_ref(),
            Action::GoToTab1 => self.navigation.go_to_tab_1.as_ref(),
            Action::GoToTab2 => self.navigation.go_to_tab_2.as_ref(),
            Action::GoToTab3 => self.navigation.go_to_tab_3.as_ref(),
            Action::GoToTab4 => self.navigation.go_to_tab_4.as_ref(),
            Action::GoToTab5 => self.navigation.go_to_tab_5.as_ref(),
            Action::GoToTab6 => self.navigation.go_to_tab_6.as_ref(),
            Action::GoToTab7 => self.navigation.go_to_tab_7.as_ref(),
            Action::Overview => self.navigation.overview.as_ref(),
            Action::ToggleAdvancedTabs => self.navigation.toggle_advanced_tabs.as_ref(),

            // Workspace
            Action::WorkspaceLoadMore => self.workspace.load_more.as_ref(),
            Action::WorkspaceCreateTag => self.workspace.create_tag.as_ref(),
            Action::WorkspaceCreateBranch => self.workspace.create_branch.as_ref(),
            Action::WorkspaceYankHash => self.workspace.yank_hash.as_ref(),
            Action::WorkspaceCheckout => self.workspace.checkout.as_ref(),
            Action::WorkspaceRevert => self.workspace.revert.as_ref(),
            Action::WorkspaceCherryPick => self.workspace.cherry_pick.as_ref(),
            Action::WorkspaceInteractiveRebase => self.workspace.interactive_rebase.as_ref(),
            Action::WorkspaceStashUI => self.workspace.stash_ui.as_ref(),
            Action::WorkspaceCommit => self.workspace.commit.as_ref(),
            Action::WorkspaceCommitAmend => self.workspace.commit_amend.as_ref(),
            Action::WorkspaceFuzzySearch => self.workspace.fuzzy_search.as_ref(),
            Action::WorkspaceColumnPicker => self.workspace.column_picker.as_ref(),
            Action::WorkspaceLogsView => self.workspace.logs_view.as_ref(),
            Action::WorkspaceStage => self.workspace.stage.as_ref(),
            Action::WorkspaceStageAll => self.workspace.stage_all.as_ref(),
            Action::WorkspaceDiscard => self.workspace.discard.as_ref(),
            Action::WorkspaceDiscardAll => self.workspace.discard_all.as_ref(),

            // Files
            Action::FilesBlame => self.files.blame.as_ref(),
            Action::FilesLineNumbers => self.files.line_numbers.as_ref(),
            Action::FilesHistory => self.files.history.as_ref(),
            Action::FilesSearch => self.files.search.as_ref(),
            Action::FilesExpand => self.files.expand.as_ref(),
            Action::FilesCollapse => self.files.collapse.as_ref(),
            Action::FilesEditor => self.files.editor.as_ref(),
            Action::FilesFullScreen => self.files.full_screen.as_ref(),

            // Branches
            Action::BranchesCheckout => self.branches.checkout.as_ref(),
            Action::BranchesCreate => self.branches.create.as_ref(),
            Action::BranchesDelete => self.branches.delete.as_ref(),
            Action::BranchesMerge => self.branches.merge.as_ref(),
            Action::BranchesRebase => self.branches.rebase.as_ref(),
            Action::BranchesInteractiveRebase => self.branches.interactive_rebase.as_ref(),
            Action::BranchesPull => self.branches.pull.as_ref(),
            Action::BranchesPush => self.branches.push.as_ref(),
            Action::BranchesSearch => self.branches.search.as_ref(),

            // Tags
            Action::TagsCheckout => self.tags.checkout.as_ref(),
            Action::TagsDelete => self.tags.delete.as_ref(),
            Action::TagsPush => self.tags.push.as_ref(),
            Action::TagsPushAll => self.tags.push_all.as_ref(),
            Action::TagsFetch => self.tags.fetch.as_ref(),
            Action::TagsSearch => self.tags.search.as_ref(),

            // Remotes
            Action::RemotesAdd => self.remotes.add.as_ref(),
            Action::RemotesDelete => self.remotes.delete.as_ref(),
            Action::RemotesFetch => self.remotes.fetch.as_ref(),

            // Stashes
            Action::StashesApply => self.stashes.apply.as_ref(),
            Action::StashesCreate => self.stashes.create.as_ref(),
            Action::StashesDelete => self.stashes.delete.as_ref(),

            // Worktrees
            Action::WorktreesOpen => self.worktrees.open.as_ref(),
            Action::WorktreesAdd => self.worktrees.add.as_ref(),
            Action::WorktreesDelete => self.worktrees.delete.as_ref(),
            Action::WorktreesLock => self.worktrees.lock.as_ref(),
            Action::WorktreesPrune => self.worktrees.prune.as_ref(),

            // Submodules
            Action::SubmodulesAdd => self.submodules.add.as_ref(),
            Action::SubmodulesDelete => self.submodules.delete.as_ref(),

            // Reflog
            Action::ReflogCheckout => self.reflog.checkout.as_ref(),

            // Forge
            Action::ForgeCheckout => self.forge.checkout.as_ref(),
            Action::ForgeOpenBrowser => self.forge.open_browser.as_ref(),
            Action::ForgeToggleAssigned => self.forge.toggle_assigned.as_ref(),
            Action::ForgeAddComment => self.forge.add_comment.as_ref(),

            // Diff
            Action::DiffLineMode => self.diff.line_mode.as_ref(),
            Action::DiffStage => self.diff.stage.as_ref(),
            Action::DiffUnstage => self.diff.unstage.as_ref(),
            Action::DiffDiscard => self.diff.discard.as_ref(),

            // Conflict
            Action::ConflictOurs => self.conflict.ours.as_ref(),
            Action::ConflictTheirs => self.conflict.theirs.as_ref(),
            Action::ConflictResolve => self.conflict.resolve.as_ref(),
            Action::ConflictAbort => self.conflict.abort.as_ref(),
            Action::ConflictContinue => self.conflict.continue_merge.as_ref(),

            // Detail
            Action::DetailMoveUp => self.detail.move_up.as_ref(),
            Action::DetailMoveDown => self.detail.move_down.as_ref(),
            Action::DetailPageUp => self.detail.page_up.as_ref(),
            Action::DetailPageDown => self.detail.page_down.as_ref(),
            Action::DetailHome => self.detail.home.as_ref(),
            Action::DetailEnd => self.detail.end.as_ref(),
        };

        desc_opt.map(|k| k.description.clone()).unwrap_or_else(|| "".to_string())
    }

    pub fn format_action_keys(&self, action: Action, compatibility_mode: bool) -> String {
        let keys = self.get_action_keys(action);
        if keys.is_empty() {
            return "-".to_string();
        }
        keys.iter()
            .map(|k| {
                let key = k.clone();
                if !compatibility_mode {
                    match key.as_str() {
                        "esc" | "escape" => "⎋".to_string(),
                        "tab" => "⇥".to_string(),
                        "backtab" | "shift-tab" => "⇧⇥".to_string(),
                        "enter" | "return" => "↵".to_string(),
                        _ => key,
                    }
                } else {
                    match key.as_str() {
                        "esc" | "escape" => "Esc".to_string(),
                        "tab" => "Tab".to_string(),
                        "backtab" | "shift-tab" => "Shift+Tab".to_string(),
                        "enter" | "return" => "Enter".to_string(),
                        _ => key,
                    }
                }
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    pub fn matches(&self, action: Action, key: KeyEvent) -> bool {
        let user_keys = self.get_action_keys(action);
        let mut matched = false;
        let mut has_valid_user_binding = false;

        for key_str in &user_keys {
            if let Some((code, mods)) = parse_key(key_str) {
                has_valid_user_binding = true;
                if keys_equal(key.code, key.modifiers, code, mods) {
                    matched = true;
                }
            }
        }

        if has_valid_user_binding {
            return matched;
        }

        // Fallback to default
        let default_keys = Self::get_default_keys(action);
        for key_str in &default_keys {
            if let Some((code, mods)) = parse_key(key_str) {
                if keys_equal(key.code, key.modifiers, code, mods) {
                    return true;
                }
            }
        }

        false
    }

    pub fn find_conflict(&self, target_action: Action, proposed_keys: &[String]) -> Option<Action> {
        let parsed_proposed: Vec<(KeyCode, KeyModifiers)> =
            proposed_keys.iter().filter_map(|k| parse_key(k)).collect();

        if parsed_proposed.is_empty() {
            return None;
        }

        // Gather all actions dynamically (Global, Home, and Detail Navigation actions)
        let all_actions = [
            Action::ToggleStatusBar,
            Action::Help,
            Action::Close,
            Action::HomeMoveDown,
            Action::HomeMoveUp,
            Action::HomePageDown,
            Action::HomePageUp,
            Action::HomeHome,
            Action::HomeEnd,
            Action::HomeAddRepo,
            Action::HomeBulkAdd,
            Action::HomeEditRepo,
            Action::HomeDeleteRepo,
            Action::HomeOpenDebugLogs,
            Action::HomeEditLabels,
            Action::HomeAbout,
            Action::HomeSymbolsHelp,
            Action::HomeRefresh,
            Action::HomeCycleSort,
            Action::HomeToggleSortReverse,
            Action::HomeTogglePin,
            Action::HomeOpenSettings,
            Action::HomeImportRepo,
            Action::HomeOpenGitApp,
            Action::HomeSearchRepo,
            Action::HomeOpenDetail,
            Action::HomeCheckUpdate,
            Action::HomeCycleViewMode,
            Action::HomeOpenTerminal,
            Action::HomeToggleStar,
            Action::HomeYankPath,
            Action::HomeJumpPicker,
            Action::HomeFetchAll,
            Action::HomeSelect,
            Action::HomeGlobalSearch,
            Action::CloseDetail,
            Action::DetailHelp,
            Action::CycleFocusForward,
            Action::CycleFocusBackward,
            Action::RefreshDetail,
            Action::CycleTabForward,
            Action::CycleTabBackward,
            Action::GoToTab1,
            Action::GoToTab2,
            Action::GoToTab3,
            Action::GoToTab4,
            Action::GoToTab5,
            Action::GoToTab6,
            Action::GoToTab7,
            Action::Overview,
            Action::ToggleAdvancedTabs,
        ];

        for &other in &all_actions {
            if other == target_action {
                continue;
            }
            // Check context rules:
            // Home actions only conflict with Home actions & Global actions.
            // Detail actions only conflict with Detail actions & Global actions.
            let both_global = self.is_global_action(target_action) || self.is_global_action(other);
            let both_home = self.is_home_action(target_action) && self.is_home_action(other);
            let both_detail = self.is_detail_action(target_action) && self.is_detail_action(other);

            if !both_global && !both_home && !both_detail {
                continue;
            }

            let other_keys = self.get_action_keys(other);
            for ok_str in &other_keys {
                if let Some((ok_code, ok_mods)) = parse_key(ok_str) {
                    for &(prop_code, prop_mods) in &parsed_proposed {
                        if keys_equal(prop_code, prop_mods, ok_code, ok_mods) {
                            return Some(other);
                        }
                    }
                }
            }
        }
        None
    }

    fn is_global_action(&self, action: Action) -> bool {
        matches!(action, Action::ToggleStatusBar | Action::Help | Action::Close)
    }

    fn is_home_action(&self, action: Action) -> bool {
        matches!(
            action,
            Action::HomeMoveDown
                | Action::HomeMoveUp
                | Action::HomePageDown
                | Action::HomePageUp
                | Action::HomeHome
                | Action::HomeEnd
                | Action::HomeAddRepo
                | Action::HomeBulkAdd
                | Action::HomeEditRepo
                | Action::HomeDeleteRepo
                | Action::HomeOpenDebugLogs
                | Action::HomeEditLabels
                | Action::HomeAbout
                | Action::HomeSymbolsHelp
                | Action::HomeRefresh
                | Action::HomeCycleSort
                | Action::HomeToggleSortReverse
                | Action::HomeTogglePin
                | Action::HomeOpenSettings
                | Action::HomeImportRepo
                | Action::HomeOpenGitApp
                | Action::HomeSearchRepo
                | Action::HomeOpenDetail
                | Action::HomeCheckUpdate
                | Action::HomeCycleViewMode
                | Action::HomeOpenTerminal
                | Action::HomeToggleStar
                | Action::HomeYankPath
                | Action::HomeJumpPicker
                | Action::HomeFetchAll
                | Action::HomeSelect
                | Action::HomeGlobalSearch
        )
    }

    fn is_detail_action(&self, action: Action) -> bool {
        matches!(
            action,
            Action::CloseDetail
                | Action::DetailHelp
                | Action::CycleFocusForward
                | Action::CycleFocusBackward
                | Action::RefreshDetail
                | Action::CycleTabForward
                | Action::CycleTabBackward
                | Action::GoToTab1
                | Action::GoToTab2
                | Action::GoToTab3
                | Action::GoToTab4
                | Action::GoToTab5
                | Action::GoToTab6
                | Action::GoToTab7
                | Action::Overview
                | Action::ToggleAdvancedTabs
        )
    }

    pub fn check_conflicts(&self) {
        // Keeps a general audit check. Optional logging can be added here if needed.
    }

    pub fn update_action_keys(&mut self, action: Action, keys: Vec<String>) {
        let desc = self.get_action_description(action);
        let keybind = Some(Keybind { keys, description: desc });
        match action {
            // Global
            Action::ToggleStatusBar => self.global.toggle_status_bar = keybind,
            Action::Help => self.global.help = keybind,
            Action::Close => self.global.close = keybind,

            // Home
            Action::HomeMoveDown => self.home.move_down = keybind,
            Action::HomeMoveUp => self.home.move_up = keybind,
            Action::HomePageDown => self.home.page_down = keybind,
            Action::HomePageUp => self.home.page_up = keybind,
            Action::HomeHome => self.home.home = keybind,
            Action::HomeEnd => self.home.end = keybind,
            Action::HomeAddRepo => self.home.add_repo = keybind,
            Action::HomeBulkAdd => self.home.bulk_add = keybind,
            Action::HomeEditRepo => self.home.edit_repo = keybind,
            Action::HomeDeleteRepo => self.home.delete_repo = keybind,
            Action::HomeOpenDebugLogs => self.home.open_debug_logs = keybind,
            Action::HomeEditLabels => self.home.edit_labels = keybind,
            Action::HomeAbout => self.home.about = keybind,
            Action::HomeSymbolsHelp => self.home.symbols_help = keybind,
            Action::HomeRefresh => self.home.refresh = keybind,
            Action::HomeCycleSort => self.home.cycle_sort = keybind,
            Action::HomeToggleSortReverse => self.home.toggle_sort_reverse = keybind,
            Action::HomeTogglePin => self.home.toggle_pin = keybind,
            Action::HomeOpenSettings => self.home.open_settings = keybind,
            Action::HomeImportRepo => self.home.import_repo = keybind,
            Action::HomeOpenGitApp => self.home.open_git_app = keybind,
            Action::HomeSearchRepo => self.home.search_repo = keybind,
            Action::HomeOpenDetail => self.home.open_detail = keybind,
            Action::HomeCheckUpdate => self.home.check_update = keybind,
            Action::HomeCycleViewMode => self.home.cycle_view_mode = keybind,
            Action::HomeOpenTerminal => self.home.open_terminal = keybind,
            Action::HomeToggleStar => self.home.toggle_star = keybind,
            Action::HomeYankPath => self.home.yank_path = keybind,
            Action::HomeJumpPicker => self.home.jump_picker = keybind,
            Action::HomeFetchAll => self.home.fetch_all = keybind,
            Action::HomeSelect => self.home.select = keybind,
            Action::HomeGlobalSearch => self.home.global_search = keybind,

            // Navigation
            Action::CloseDetail => self.navigation.close_detail = keybind,
            Action::DetailHelp => self.navigation.detail_help = keybind,
            Action::CycleFocusForward => self.navigation.cycle_focus_forward = keybind,
            Action::CycleFocusBackward => self.navigation.cycle_focus_backward = keybind,
            Action::RefreshDetail => self.navigation.refresh_detail = keybind,
            Action::CycleTabForward => self.navigation.cycle_tab_forward = keybind,
            Action::CycleTabBackward => self.navigation.cycle_tab_backward = keybind,
            Action::GoToTab1 => self.navigation.go_to_tab_1 = keybind,
            Action::GoToTab2 => self.navigation.go_to_tab_2 = keybind,
            Action::GoToTab3 => self.navigation.go_to_tab_3 = keybind,
            Action::GoToTab4 => self.navigation.go_to_tab_4 = keybind,
            Action::GoToTab5 => self.navigation.go_to_tab_5 = keybind,
            Action::GoToTab6 => self.navigation.go_to_tab_6 = keybind,
            Action::GoToTab7 => self.navigation.go_to_tab_7 = keybind,
            Action::Overview => self.navigation.overview = keybind,
            Action::ToggleAdvancedTabs => self.navigation.toggle_advanced_tabs = keybind,

            // Workspace
            Action::WorkspaceLoadMore => self.workspace.load_more = keybind,
            Action::WorkspaceCreateTag => self.workspace.create_tag = keybind,
            Action::WorkspaceCreateBranch => self.workspace.create_branch = keybind,
            Action::WorkspaceYankHash => self.workspace.yank_hash = keybind,
            Action::WorkspaceCheckout => self.workspace.checkout = keybind,
            Action::WorkspaceRevert => self.workspace.revert = keybind,
            Action::WorkspaceCherryPick => self.workspace.cherry_pick = keybind,
            Action::WorkspaceInteractiveRebase => self.workspace.interactive_rebase = keybind,
            Action::WorkspaceStashUI => self.workspace.stash_ui = keybind,
            Action::WorkspaceCommit => self.workspace.commit = keybind,
            Action::WorkspaceCommitAmend => self.workspace.commit_amend = keybind,
            Action::WorkspaceFuzzySearch => self.workspace.fuzzy_search = keybind,
            Action::WorkspaceColumnPicker => self.workspace.column_picker = keybind,
            Action::WorkspaceLogsView => self.workspace.logs_view = keybind,
            Action::WorkspaceStage => self.workspace.stage = keybind,
            Action::WorkspaceStageAll => self.workspace.stage_all = keybind,
            Action::WorkspaceDiscard => self.workspace.discard = keybind,
            Action::WorkspaceDiscardAll => self.workspace.discard_all = keybind,

            // Files
            Action::FilesBlame => self.files.blame = keybind,
            Action::FilesLineNumbers => self.files.line_numbers = keybind,
            Action::FilesHistory => self.files.history = keybind,
            Action::FilesSearch => self.files.search = keybind,
            Action::FilesExpand => self.files.expand = keybind,
            Action::FilesCollapse => self.files.collapse = keybind,
            Action::FilesEditor => self.files.editor = keybind,
            Action::FilesFullScreen => self.files.full_screen = keybind,

            // Branches
            Action::BranchesCheckout => self.branches.checkout = keybind,
            Action::BranchesCreate => self.branches.create = keybind,
            Action::BranchesDelete => self.branches.delete = keybind,
            Action::BranchesMerge => self.branches.merge = keybind,
            Action::BranchesRebase => self.branches.rebase = keybind,
            Action::BranchesInteractiveRebase => self.branches.interactive_rebase = keybind,
            Action::BranchesPull => self.branches.pull = keybind,
            Action::BranchesPush => self.branches.push = keybind,
            Action::BranchesSearch => self.branches.search = keybind,

            // Tags
            Action::TagsCheckout => self.tags.checkout = keybind,
            Action::TagsDelete => self.tags.delete = keybind,
            Action::TagsPush => self.tags.push = keybind,
            Action::TagsPushAll => self.tags.push_all = keybind,
            Action::TagsFetch => self.tags.fetch = keybind,
            Action::TagsSearch => self.tags.search = keybind,

            // Remotes
            Action::RemotesAdd => self.remotes.add = keybind,
            Action::RemotesDelete => self.remotes.delete = keybind,
            Action::RemotesFetch => self.remotes.fetch = keybind,

            // Stashes
            Action::StashesApply => self.stashes.apply = keybind,
            Action::StashesCreate => self.stashes.create = keybind,
            Action::StashesDelete => self.stashes.delete = keybind,

            // Worktrees
            Action::WorktreesOpen => self.worktrees.open = keybind,
            Action::WorktreesAdd => self.worktrees.add = keybind,
            Action::WorktreesDelete => self.worktrees.delete = keybind,
            Action::WorktreesLock => self.worktrees.lock = keybind,
            Action::WorktreesPrune => self.worktrees.prune = keybind,

            // Submodules
            Action::SubmodulesAdd => self.submodules.add = keybind,
            Action::SubmodulesDelete => self.submodules.delete = keybind,

            // Reflog
            Action::ReflogCheckout => self.reflog.checkout = keybind,

            // Forge
            Action::ForgeCheckout => self.forge.checkout = keybind,
            Action::ForgeOpenBrowser => self.forge.open_browser = keybind,
            Action::ForgeToggleAssigned => self.forge.toggle_assigned = keybind,
            Action::ForgeAddComment => self.forge.add_comment = keybind,

            // Diff
            Action::DiffLineMode => self.diff.line_mode = keybind,
            Action::DiffStage => self.diff.stage = keybind,
            Action::DiffUnstage => self.diff.unstage = keybind,
            Action::DiffDiscard => self.diff.discard = keybind,

            // Conflict
            Action::ConflictOurs => self.conflict.ours = keybind,
            Action::ConflictTheirs => self.conflict.theirs = keybind,
            Action::ConflictResolve => self.conflict.resolve = keybind,
            Action::ConflictAbort => self.conflict.abort = keybind,
            Action::ConflictContinue => self.conflict.continue_merge = keybind,

            // Detail
            Action::DetailMoveUp => self.detail.move_up = keybind,
            Action::DetailMoveDown => self.detail.move_down = keybind,
            Action::DetailPageUp => self.detail.page_up = keybind,
            Action::DetailPageDown => self.detail.page_down = keybind,
            Action::DetailHome => self.detail.home = keybind,
            Action::DetailEnd => self.detail.end = keybind,
        }
    }

    pub fn save(&self, config_dir: &Path) -> Result<(), std::io::Error> {
        let keybindings_path = config_dir.join("keybindings.toml");
        let serialized =
            toml::to_string_pretty(self).map_err(|e| std::io::Error::other(e.to_string()))?;
        std::fs::write(&keybindings_path, serialized)?;
        Ok(())
    }

    pub fn load(config_dir: &Path) -> Self {
        let keybindings_path = config_dir.join("keybindings.toml");
        if keybindings_path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&keybindings_path) {
                if let Ok(cfg) = toml::from_str::<KeybindingsConfig>(&contents) {
                    return cfg;
                }
            }
        }

        let default_cfg = Self::default_config();
        if let Ok(serialized) = toml::to_string_pretty(&default_cfg) {
            let _ = std::fs::write(&keybindings_path, serialized);
        }
        default_cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keybindings_coverage_boost() {
        // 1. from_index / to_index
        let action = Action::Close;
        let idx = action.to_index();
        let parsed = Action::from_index(idx);
        assert_eq!(parsed, Some(action));
        assert!(Action::from_index(9999).is_none());

        // 2. save / load / check_conflicts
        let temp_dir = std::env::temp_dir().join(format!(
            "gitwig_kb_{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let config = KeybindingsConfig::default_config();
        config.check_conflicts();
        config.save(&temp_dir).unwrap();

        let loaded = KeybindingsConfig::load(&temp_dir);
        assert_eq!(
            loaded.format_action_keys(Action::Close, false),
            config.format_action_keys(Action::Close, false)
        );

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
