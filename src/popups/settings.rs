//! Global settings editor supporting themes, sorting preferences, and paths scanner intervals.

use crate::app::{App, Mode};
use crate::config::SortOrder;
use crate::repo::RemoteInfo;
use crate::ui::layout::{centered_rect, centered_rect_fixed};
use crate::ui::style::{
    ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, accent_style, muted_style, parse_color,
    primary_style,
};
use crate::ui::{wrap_excludes, wrap_str};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap};

const GENERAL_SETTING_INDICES: &[usize] =
    &[9, 56, 55, 0, 60, 13, 65, 66, 12, 58, 62, 63, 7, 80, 81];
const SORTING_SETTING_INDICES: &[usize] = &[1, 2, 6, 64];
const SCAN_SETTING_INDICES: &[usize] = &[5, 4, 8, 61];
const THEME_SETTING_INDICES: &[usize] = &[3, 67, 82];
const GLOBAL_NAV_SETTING_INDICES: &[usize] = &[
    16, // Quit / Close Dialog (Close)
    15, // Help (Help)
    14, // Toggle Status Bar (ToggleStatusBar)
    39, // Detail: Close View (CloseDetail)
    40, // Detail: Help (DetailHelp)
    41, // Detail: Cycle Focus Fwd (CycleFocusForward)
    42, // Detail: Cycle Focus Bwd (CycleFocusBackward)
    43, // Detail: Refresh View (RefreshDetail)
    44, // Detail: Cycle Tab Fwd (CycleTabForward)
    45, // Detail: Cycle Tab Bwd (CycleTabBackward)
    46, // Detail: Tab 1
    47, // Detail: Tab 2
    48, // Detail: Tab 3
    49, // Detail: Tab 4
    50, // Detail: Tab 5
    51, // Detail: Tab 6
    52, // Detail: Tab 7
    68, // Detail: Show Overview (Overview)
    69, // Detail: Toggle Advanced Tabs (ToggleAdvancedTabs)
];
const HOME_SETTING_INDICES: &[usize] = &[
    17, // Home: Move Down
    18, // Home: Move Up
    19, // Home: Page Down
    20, // Home: Page Up
    21, // Home: Go to Top
    22, // Home: Go to Bottom
    38, // Home: Open Details
    75, // Home: Toggle Selection
    33, // Home: Toggle Pin
    71, // Home: Toggle Star
    72, // Home: Yank Path
    73, // Home: Jump Picker
    76, // Home: Global Code Search
    37, // Home: Search Repository
    30, // Home: Refresh Status
    31, // Home: Cycle Sort Order
    32, // Home: Toggle Sort Reverse
    77, // Home: Toggle Compact View
    23, // Home: Add Repository
    24, // Home: Bulk Add
    35, // Home: Import Repository
    25, // Home: Edit Repository
    26, // Home: Delete Repository
    28, // Home: Edit Labels
    70, // Home: Open Terminal
    36, // Home: Open Git App
    27, // Home: Open Debug Logs
    29, // Home: Open About Dialog
    78, // Home: Signs & Symbols Legend
    57, // Home: Check Updates
];
const WORKSPACE_SETTING_INDICES: &[usize] =
    &[100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116];
const FILES_BRANCHES_SETTING_INDICES: &[usize] =
    &[120, 121, 122, 123, 124, 125, 126, 127, 130, 131, 132, 133, 134, 135, 136, 137, 138];
const TAGS_REMOTES_STASHES_SETTING_INDICES: &[usize] =
    &[140, 141, 142, 143, 144, 145, 150, 151, 152, 160, 161, 162];
const ADVANCED_TABS_SETTING_INDICES: &[usize] = &[170, 171, 172, 173, 174, 180, 181, 190, 200, 201];
const DIFF_CONFLICT_SETTING_INDICES: &[usize] = &[210, 211, 212, 213, 220, 221, 222, 223, 224];
const SCROLL_NAV_SETTING_INDICES: &[usize] = &[230, 231, 232, 233, 234, 235];
const ALL_KEYBINDINGS_SETTING_INDICES: &[usize] = &[
    // Global & Nav Keys
    16, 15, 14, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 68, 69,
    // Home Screen Keys
    17, 18, 19, 20, 21, 22, 38, 75, 33, 71, 72, 73, 76, 37, 30, 31, 32, 77, 23, 24, 35, 25, 26, 28,
    70, 36, 27, 29, 78, 57, // Workspace Tab Keys
    100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116,
    // Files & Branch Keys
    120, 121, 122, 123, 124, 125, 126, 127, 130, 131, 132, 133, 134, 135, 136, 137, 138,
    // Tags, Remotes & Stashes
    140, 141, 142, 143, 144, 145, 150, 151, 152, 160, 161, 162, // Advanced Tabs Keys
    170, 171, 172, 173, 174, 180, 181, 190, 200, 201, // Diff & Conflict Keys
    210, 211, 212, 213, 220, 221, 222, 223, 224, // Scroll & Nav Keys
    230, 231, 232, 233, 234, 235,
];

fn index_to_action(idx: usize) -> Option<crate::keybindings::Action> {
    crate::keybindings::Action::from_index(idx)
}

fn get_category_indices(cat: usize) -> &'static [usize] {
    match cat {
        0 => GENERAL_SETTING_INDICES,
        1 => SORTING_SETTING_INDICES,
        2 => SCAN_SETTING_INDICES,
        3 => THEME_SETTING_INDICES,
        4 => ALL_KEYBINDINGS_SETTING_INDICES,
        _ => &[],
    }
}

fn get_category_name(cat: usize) -> &'static str {
    match cat {
        0 => "General Settings",
        1 => "Sorting & Limits",
        2 => "Scan Discovery",
        3 => "Theme & Style",
        4 => "Keybindings",
        _ => "",
    }
}

fn get_category_icon(cat: usize, compat: bool) -> &'static str {
    if compat {
        match cat {
            0 => "* ",
            1 => "# ",
            2 => "? ",
            3 => "@ ",
            4 => "K ",
            _ => "",
        }
    } else {
        match cat {
            0 => "⚙ ",
            1 => "📶 ",
            2 => "🔍 ",
            3 => "🎨 ",
            4 => "🔑 ",
            _ => "",
        }
    }
}

fn get_active_category(selected_idx: usize) -> usize {
    if GENERAL_SETTING_INDICES.contains(&selected_idx) {
        0
    } else if SORTING_SETTING_INDICES.contains(&selected_idx) {
        1
    } else if SCAN_SETTING_INDICES.contains(&selected_idx) {
        2
    } else if THEME_SETTING_INDICES.contains(&selected_idx) {
        3
    } else {
        4
    }
}

fn get_sub_index(selected_idx: usize) -> usize {
    let cat = get_active_category(selected_idx);
    let indices = get_category_indices(cat);
    indices.iter().position(|&x| x == selected_idx).unwrap_or(0)
}

pub(crate) fn get_action_section_name(idx: usize) -> &'static str {
    if GLOBAL_NAV_SETTING_INDICES.contains(&idx) {
        "Global & Navigation"
    } else if HOME_SETTING_INDICES.contains(&idx) {
        "Home Screen Keys"
    } else if WORKSPACE_SETTING_INDICES.contains(&idx) {
        "Workspace Tab Keys"
    } else if FILES_BRANCHES_SETTING_INDICES.contains(&idx) {
        "Files & Branch Keys"
    } else if TAGS_REMOTES_STASHES_SETTING_INDICES.contains(&idx) {
        "Tags, Remotes & Stashes Keys"
    } else if ADVANCED_TABS_SETTING_INDICES.contains(&idx) {
        "Advanced Tabs Keys"
    } else if DIFF_CONFLICT_SETTING_INDICES.contains(&idx) {
        "Diff & Conflict Keys"
    } else if SCROLL_NAV_SETTING_INDICES.contains(&idx) {
        "Scroll & Nav Keys"
    } else {
        "Unknown"
    }
}

pub(crate) fn get_label(global_idx: usize) -> &'static str {
    match global_idx {
        0 => "Poll Interval (ms)",
        1 => "Sort By",
        2 => "Sort Reverse",
        3 => "Theme Name",
        4 => "Scan Max Depth",
        5 => "Scan Start Dir",
        6 => "Max Commits",
        7 => "Page Size",
        8 => "Scan Exclude Folders",
        9 => "Preferred Git Client",
        10 => "(removed: always git-only)",
        12 => "Compatibility Mode",
        13 => "Resync on Tab Change",
        58 => "Show Grouping",
        55 => "SSH Strict Host Checking",
        56 => "Editor Command",
        60 => "Auto-Fetch Interval (mins)",
        61 => "Watch Directories",
        62 => "Show CPU/MEM in Status Bar",
        63 => "Enable Commit Signatures",
        64 => "Graph Max Commits",
        65 => "Detail Cache TTL (secs)",
        66 => "Tab Cache TTL (secs)",
        67 => "Compact Layout View",
        82 => "Tile Layout Columns (0=Auto)",
        80 => "Stale Threshold (months)",
        81 => "Show Stale Projects",
        14 => "Toggle Status Bar",
        15 => "Help",
        16 => "Quit / Close Dialog",
        17 => "Home: Move Down",
        18 => "Home: Move Up",
        19 => "Home: Page Down",
        20 => "Home: Page Up",
        21 => "Home: Go to Top",
        22 => "Home: Go to Bottom",
        23 => "Home: Add Repository",
        24 => "Home: Bulk Add",
        25 => "Home: Edit Repository",
        26 => "Home: Delete Repository",
        27 => "Home: Open Debug Logs",
        28 => "Home: Edit Labels",
        29 => "Home: Open About Dialog",
        30 => "Home: Refresh Status",
        31 => "Home: Cycle Sort Order",
        32 => "Home: Toggle Sort Reverse",
        33 => "Home: Toggle Pin",
        34 => "Home: Open Settings",
        35 => "Home: Import Repository",
        36 => "Home: Open Git App",
        37 => "Home: Search Repository",
        38 => "Home: Open Details",
        57 => "Home: Check Updates",
        39 => "Detail: Close View",
        40 => "Detail: Help",
        41 => "Detail: Cycle Focus Fwd",
        42 => "Detail: Cycle Focus Bwd",
        43 => "Detail: Refresh View",
        44 => "Detail: Cycle Tab Fwd",
        45 => "Detail: Cycle Tab Bwd",
        46 => "Detail: Tab 1 (Commits / Worktrees)",
        47 => "Detail: Tab 2 (Files / Submodules)",
        48 => "Detail: Tab 3 (Graph / Reflog)",
        49 => "Detail: Tab 4 (Branches / Forge)",
        50 => "Detail: Tab 5 (Tags)",
        51 => "Detail: Tab 6 (Remotes)",
        52 => "Detail: Tab 7 (Stashes)",

        68 => "Detail: Show Overview",
        70 => "Home: Open Terminal",
        71 => "Home: Toggle Star",
        72 => "Home: Yank Path",
        73 => "Home: Jump Picker",
        74 => "Home: Fetch All",
        75 => "Home: Toggle Selection",
        76 => "Home: Global Code Search",
        77 => "Home: Toggle Compact View",
        78 => "Home: Signs & Symbols Legend",
        69 => "Detail: Toggle Advanced Tabs",

        // Workspace
        100 => "Workspace: Load More",
        101 => "Workspace: Create Tag",
        102 => "Workspace: Create Branch",
        103 => "Workspace: Yank Commit Hash",
        104 => "Workspace: Revert Commit",
        105 => "Workspace: Cherry Pick",
        106 => "Workspace: Interactive Rebase",
        107 => "Workspace: Open Stash UI",
        108 => "Workspace: Commit Changes",
        109 => "Workspace: Commit Amend",
        110 => "Workspace: Fuzzy Search",
        111 => "Workspace: Column Picker",
        112 => "Workspace: Open Full Logs",
        113 => "Workspace: Stage / Unstage Selected",
        114 => "Workspace: Stage / Unstage All",
        115 => "Workspace: Discard Selected Change",
        116 => "Workspace: Discard All Changes",

        // Files
        120 => "Files: Blame Toggle",
        121 => "Files: Line Numbers Toggle",
        122 => "Files: Selected File History",
        123 => "Files: Fuzzy Search Picker",
        124 => "Files: Expand Folder",
        125 => "Files: Collapse Folder",
        126 => "Files: Open in Editor",
        127 => "Files: Toggle Full Screen Diff",

        // Branches
        130 => "Branches: Checkout selected",
        131 => "Branches: Create branch",
        132 => "Branches: Delete branch",
        133 => "Branches: Merge branch",
        134 => "Branches: Rebase current",
        135 => "Branches: Interactive Rebase",
        136 => "Branches: Pull branch changes",
        137 => "Branches: Push branch",
        138 => "Branches: Fuzzy Search",

        // Tags
        140 => "Tags: Checkout tag",
        141 => "Tags: Delete tag",
        142 => "Tags: Push tag",
        143 => "Tags: Push all tags",
        144 => "Tags: Fetch tags",
        145 => "Tags: Fuzzy Search",

        // Remotes
        150 => "Remotes: Add remote",
        151 => "Remotes: Delete remote",
        152 => "Remotes: Fetch remote",

        // Stashes
        160 => "Stashes: Apply stash",
        161 => "Stashes: Create stash",
        162 => "Stashes: Delete stash",

        // Worktrees
        170 => "Worktrees: Open worktree",
        171 => "Worktrees: Add worktree",
        172 => "Worktrees: Remove worktree",
        173 => "Worktrees: Lock/Unlock worktree",
        174 => "Worktrees: Prune worktrees",

        // Submodules
        180 => "Submodules: Add submodule",
        181 => "Submodules: Delete submodule",

        // Reflog
        190 => "Reflog: Checkout entry OID",

        // Forge
        200 => "Forge: Checkout issue branch",
        201 => "Forge: Open issue in browser",

        // Diff
        210 => "Diff: Toggle Line/Hunk Mode",
        211 => "Diff: Stage hunk/line",
        212 => "Diff: Unstage hunk/line",
        213 => "Diff: Discard hunk/line",

        // Conflict
        220 => "Conflict: Accept OURS",
        221 => "Conflict: Accept THEIRS",
        222 => "Conflict: Mark Resolved",
        223 => "Conflict: Abort Merge",
        224 => "Conflict: Continue Merge",

        // Detail List Scroll
        230 => "Scroll: Move Up",
        231 => "Scroll: Move Down",
        232 => "Scroll: Page Up",
        233 => "Scroll: Page Down",
        234 => "Scroll: Go to Top",
        235 => "Scroll: Go to Bottom",

        _ => "",
    }
}

fn get_desc(global_idx: usize) -> &'static str {
    match global_idx {
        0 => "Event-loop poll interval in milliseconds. Sane range: 16-500.",
        1 => "Initial repository sorting criteria.",
        2 => "Reverse the order of repositories.",
        3 => "Active theme configuration name. Press Enter/Space to select from dropdown.",
        4 => "Maximum directory depth to search for git repositories.",
        5 => "Starting directory for interactive repository discovery scanning.",
        6 => "Maximum commits to load in workspace view. Set to 0 for unlimited.",
        7 => "Number of lines/items scrolled by Page Up / Page Down.",
        8 => "Comma-separated list of folders/patterns to exclude from search scans.",
        9 => "External Git application triggered by 'g' key (e.g. gitui or lazygit).",
        10 => "Only scan folders that contain a .git directory.",
        12 => {
            "Use simple ASCII symbols instead of complex Unicode emojis/icons to avoid layout breakage in some terminals."
        }
        13 => {
            "Whether to automatically refresh repository details from disk when switching tabs inside a repository."
        }
        58 => {
            "Enable/disable grouping repositories on the home page (Recent, Starred, Label groups). When disabled, a flat repository list is shown."
        }
        55 => {
            "Enforce strict SSH host key verification (StrictHostKeyChecking=yes) instead of automatically accepting new keys."
        }
        56 => "Terminal text editor to open files with (e.g. vim, nano, or notepad).",
        60 => {
            "Time interval in minutes to automatically run git fetch in the background for all repositories. Set to 0 to disable."
        }
        61 => {
            "Comma-separated list of directories watched recursively for automatic workspace synchronization (e.g. ~/development)."
        }
        62 => "Display memory usage and CPU usage of the Gitwig process in the bottom status bar.",
        63 => "Verify GPG/SSH signatures on commits list (requires spawning git subprocesses).",
        64 => "Maximum commits visualized in the Graph tab history. Set to 0 for unlimited.",
        65 => {
            "How long in seconds repository details (history, files, etc) are cached in memory before reloading."
        }
        66 => {
            "How long in seconds lazy-loaded tab data remains cached in memory before automatic refresh."
        }
        67 => "Show a compact single-line layout for repository cards in the list.",
        82 => {
            "Number of columns in Tile layout. Set to 0 to auto-calculate based on terminal width."
        }
        80 => "Number of months inactive to be considered stale. Cannot be less than 1.",
        81 => "Show or hide stale repositories in the list on the main page.",
        14 => "Toggles the status bar between collapsed and expanded view.",
        15 => "Opens the global help overlay.",
        16 => "Exits the application or closes the active settings/popup dialog.",
        17 => "Moves repository list selection cursor down.",
        18 => "Moves repository list selection cursor up.",
        19 => "Scrolls repository list down by one page.",
        20 => "Scrolls repository list up by one page.",
        21 => "Moves repository list selection cursor to the first item.",
        22 => "Moves repository list selection cursor to the last item.",
        23 => "Adds a new local git repository manually.",
        24 => "Scans a directory structure for git repositories to add in bulk.",
        25 => "Modifies path and settings of the selected repository.",
        26 => "Removes the selected repository from Gitwig.",
        27 => "Shows internal application logs for debugging.",
        28 => "Edits tags/labels associated with the selected repository.",
        29 => "Displays application version and credits info.",
        30 => "Triggers a Git inspect status check for the selected repository.",
        31 => "Cycles repository list sorting order criteria.",
        32 => "Reverses the current repository sorting order.",
        33 => "Pins or unpins the selected repository.",
        34 => "Opens the global settings popup.",
        35 => "Clones a remote git repository into a local path.",
        36 => "Launches the configured external Git CLI utility (e.g. lazygit).",
        37 => "Searches repository list by path or folder name.",
        38 => "Opens the detailed Workspace/History view for the selected repository.",
        57 => "Checks for application updates manually from the home page.",
        39 => "Clones/closes repository detail view and returns to Home.",
        40 => "Opens the detail view help/shortcuts overlay.",
        41 => "Cycles focus forward between active panel widgets.",
        42 => "Cycles focus backward between active panel widgets.",
        43 => "Refreshes the git repository details snapshot manually.",
        44 => "Navigates to the next tab in the repository workspace view.",
        45 => "Navigates to the previous tab in the repository workspace view.",
        46 => {
            "Switches directly to the Commits tab (Primary group) or Worktrees tab (Advanced group)."
        }
        47 => {
            "Switches directly to Working Tree files tab (Primary group) or Submodules tab (Advanced group)."
        }
        48 => {
            "Switches directly to Git History Graph tab (Primary group) or Reflog tab (Advanced group)."
        }
        49 => {
            "Switches directly to Local Branches tab (Primary group) or Forge tab (Advanced group)."
        }
        50 => "Switches directly to Local Tags tab (only active in Primary tab group).",
        51 => "Switches directly to Remotes tab (only active in Primary tab group).",
        52 => "Switches directly to Git Stashes tab (only active in Primary tab group).",

        68 => "Shows the repository Overview overlay from any tab.",
        69 => "Toggles between Primary and Advanced tab groups.",
        70 => "Spawn a new shell (Terminal) in the selected repository.",
        71 => "Toggle Star / Favorite status of selected repository.",
        72 => "Yank absolute path of selected repository to clipboard.",
        73 => "Open fuzzy Jump-to-Repo picker overlay.",
        74 => "Bulk fetch all tracked repositories concurrently.",
        75 => "Toggle selection of repository for batch operations.",
        76 => "Open global code search popup overlay.",
        77 => "Toggles repository list view between standard card layout and compact 1-row layout.",
        78 => "Opens the signs and symbols legend overlay panel.",
        _ => "",
    }
}

pub(crate) fn get_val_str(app: &App, global_idx: usize) -> String {
    let is_selected = app.settings_selected_index == global_idx;
    if index_to_action(global_idx).is_some() {
        if is_selected && app.settings_editing {
            format!("{}█", app.input_buffer)
        } else if let Some(action) = index_to_action(global_idx) {
            app.keybindings.get_action_keys(action).join(", ")
        } else {
            String::new()
        }
    } else {
        match global_idx {
            0 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.poll_interval_ms.to_string()
                }
            }
            1 => {
                let s = match app.config.sort_by {
                    SortOrder::Alphabetical => "Alphabetical",
                    SortOrder::RecentVisit => "Recent Visit",
                    SortOrder::LatestChanges => "Latest Changes",
                    SortOrder::Custom => "Custom",
                };
                s.to_string()
            }
            2 => app.config.sort_reverse.to_string(),
            3 => {
                if is_selected && app.settings_editing {
                    if app.settings_theme_index < app.settings_theme_list.len() {
                        app.settings_theme_list[app.settings_theme_index].clone()
                    } else {
                        app.config.theme_name.clone()
                    }
                } else {
                    app.config.theme_name.clone()
                }
            }
            4 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.scan.max_depth.to_string()
                }
            }
            5 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.scan.start_dir.clone()
                }
            }
            6 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.max_commits.to_string()
                }
            }
            7 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.page_size.to_string()
                }
            }
            8 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.scan.excludes.join(",")
                }
            }
            9 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.git_app.clone()
                }
            }
            10 => app.config.scan.git_only.to_string(),
            12 => app.config.compatibility_mode.to_string(),
            13 => app.config.resync_on_tab_change.to_string(),
            58 => app.config.show_grouping.to_string(),
            55 => app.config.ssh_strict_host_checking.to_string(),
            56 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.editor.clone()
                }
            }
            60 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.auto_fetch_interval_mins.to_string()
                }
            }
            61 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.watch_dirs.join(",")
                }
            }
            62 => app.config.show_system_stats.to_string(),
            63 => app.config.enable_commit_signatures.to_string(),
            64 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.graph_max_commits.to_string()
                }
            }
            65 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.detail_cache_ttl_secs.to_string()
                }
            }
            66 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.tab_ttl_secs.to_string()
                }
            }
            67 => format!("{:?}", app.config.view_mode),
            82 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.tile_columns.to_string()
                }
            }
            80 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.stale_threshold_months.to_string()
                }
            }
            81 => app.config.show_stale_projects.to_string(),
            _ => String::new(),
        }
    }
}

pub fn draw_settings_page(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(75, 75, area);

    // Draw background block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(accent_style())
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("Settings", primary_style()),
                Span::raw(" "),
            ])
            .alignment(Alignment::Center),
        );

    f.render_widget(Clear, popup_area);
    f.render_widget(block.clone(), popup_area);

    let inner_rect = block.inner(popup_area);

    // Split inner_rect horizontally: sidebar vs content
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28), // Sidebar
            Constraint::Length(1),      // Separator
            Constraint::Percentage(71), // Right content pane
        ])
        .split(inner_rect);

    // Draw vertical separator
    let sep_block = Block::default().borders(Borders::LEFT).border_style(muted_style());
    f.render_widget(sep_block, chunks[1]);

    let active_cat = get_active_category(app.settings_selected_index);
    let is_compat = app.config.compatibility_mode;

    // 1. Sidebar Categories Rendering
    let mut sidebar_items = Vec::new();
    for idx in 0..5 {
        let is_selected = idx == active_cat;
        let is_focused = is_selected && app.settings_focus_sidebar;

        let prefix = if is_focused {
            if is_compat { "> " } else { "▶ " }
        } else if is_selected {
            if is_compat { "o " } else { "● " }
        } else {
            "  "
        };

        let icon = get_category_icon(idx, is_compat);
        let name = get_category_name(idx);
        let style = if is_focused {
            accent_style().add_modifier(Modifier::BOLD)
        } else if is_selected {
            primary_style().add_modifier(Modifier::BOLD)
        } else {
            muted_style()
        };

        sidebar_items.push(Line::from(vec![
            Span::styled(prefix, if is_focused { accent_style() } else { muted_style() }),
            Span::styled(icon, style),
            Span::styled(name, style),
        ]));
        sidebar_items.push(Line::from("")); // spacer
    }

    let sidebar_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(if app.settings_focus_sidebar { accent_style() } else { muted_style() })
        .title(Span::styled(" Categories ", primary_style()))
        .padding(Padding::horizontal(1));

    let sidebar_inner = sidebar_block.inner(chunks[0]);
    f.render_widget(sidebar_block, chunks[0]);

    let sidebar_viewport_height = sidebar_inner.height as usize;
    let sidebar_total_height = 5 * 2;
    let mut sidebar_scroll_y = 0;
    if sidebar_viewport_height < sidebar_total_height {
        let cursor_line = active_cat * 2;
        let target_scroll = cursor_line.saturating_sub(sidebar_viewport_height / 2);
        let max_scroll = sidebar_total_height.saturating_sub(sidebar_viewport_height);
        sidebar_scroll_y = target_scroll.min(max_scroll);
    }

    let sidebar_paragraph = Paragraph::new(sidebar_items).scroll((sidebar_scroll_y as u16, 0));
    f.render_widget(sidebar_paragraph, sidebar_inner);

    // 2. Settings Content Pane Rendering
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(5)])
        .split(chunks[2]);

    let indices = get_category_indices(active_cat);
    let right_title = format!(" {} ", get_category_name(active_cat));
    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(if !app.settings_focus_sidebar { accent_style() } else { muted_style() })
        .title(Span::styled(right_title, primary_style()))
        .padding(Padding::horizontal(1));

    let content_inner = right_block.inner(right_chunks[0]);
    let available_text_width = (content_inner.width as usize).saturating_sub(6);

    let mut right_items = Vec::new();
    let mut current_line = 0;
    let mut item_starts = vec![0; indices.len()];

    let mut selected_val_chunks_len = 1;
    let mut selected_last_chunk_char_count = 0;
    let mut selected_val_offset = 11;

    let active_sub_idx = get_sub_index(app.settings_selected_index);

    for (sub_idx, &global_idx) in indices.iter().enumerate() {
        let is_selected = sub_idx == active_sub_idx && !app.settings_focus_sidebar;

        let header_str = if active_cat == 4 {
            match global_idx {
                16 => Some("Global & Navigation"),
                17 => Some("Home Screen Keys"),
                100 => Some("Workspace Tab Keys"),
                120 => Some("Files & Branch Keys"),
                140 => Some("Tags, Remotes & Stashes Keys"),
                170 => Some("Advanced Tabs Keys"),
                210 => Some("Diff & Conflict Keys"),
                230 => Some("Scroll & Nav Keys"),
                _ => None,
            }
        } else {
            None
        };

        if let Some(h_title) = header_str {
            let line_len = available_text_width.saturating_sub(h_title.chars().count() + 4);
            let dashes = "─".repeat(line_len);
            right_items.push(Line::from(vec![Span::styled(
                format!("── {} {}", h_title, dashes),
                accent_style().add_modifier(Modifier::BOLD),
            )]));
            right_items.push(Line::from("")); // spacer
            current_line += 2;
        }

        let label = get_label(global_idx);
        let val_str = get_val_str(app, global_idx);

        let prefix = if is_selected { "> " } else { "  " };
        let label_sep = if is_selected && app.settings_editing {
            if global_idx == 3 { "  [Select]: " } else { "  [Edit]: " }
        } else {
            ": "
        };

        let prefix_len = prefix.chars().count();
        let label_len = label.chars().count();
        let sep_len = label_sep.chars().count();
        let val_offset = prefix_len + label_len + sep_len;

        let val_width = available_text_width.saturating_sub(val_offset).max(10);
        let val_chunks = if global_idx == 8 {
            wrap_excludes(&val_str, val_width)
        } else {
            wrap_str(&val_str, val_width)
        };

        item_starts[sub_idx] = current_line;
        let item_height = val_chunks.len() + 1; // value lines + spacer
        current_line += item_height;

        if is_selected {
            selected_val_chunks_len = val_chunks.len();
            selected_last_chunk_char_count =
                val_chunks.last().map(|c| c.chars().count()).unwrap_or(0);
            selected_val_offset = val_offset;
        }

        // Line 1: Label + First chunk of Value
        let mut val_line_spans = vec![
            Span::styled(prefix, if is_selected { accent_style() } else { muted_style() }),
            Span::styled(
                label,
                if is_selected {
                    accent_style().add_modifier(Modifier::BOLD)
                } else {
                    primary_style()
                },
            ),
            Span::styled(label_sep, muted_style()),
        ];

        let val_style = if is_selected && app.settings_editing {
            Style::default().fg(ACCENT()).add_modifier(Modifier::UNDERLINED)
        } else if is_selected {
            accent_style()
        } else {
            Style::default()
        };

        val_line_spans.push(Span::styled(val_chunks[0].clone(), val_style));
        right_items.push(Line::from(val_line_spans));

        // Subsequent lines of the value (indented by val_offset spaces)
        for chunk in val_chunks.iter().skip(1) {
            let spaces = " ".repeat(val_offset);
            right_items
                .push(Line::from(vec![Span::raw(spaces), Span::styled(chunk.clone(), val_style)]));
        }

        // Spacer
        right_items.push(Line::from(""));
    }

    let viewport_height = content_inner.height as usize;
    let total_height = current_line;
    let mut scroll_y = if viewport_height >= total_height {
        0
    } else {
        let sel_start = item_starts[active_sub_idx];
        let sel_height = if active_sub_idx + 1 < indices.len() {
            item_starts[active_sub_idx + 1] - sel_start
        } else {
            total_height - sel_start
        };
        let item_center = sel_start + sel_height / 2;
        let target_scroll = item_center.saturating_sub(viewport_height / 2);
        let max_scroll = total_height.saturating_sub(viewport_height);
        target_scroll.min(max_scroll)
    };

    if active_sub_idx == indices.len() - 1 {
        let last_start = item_starts[active_sub_idx];
        let last_height = total_height - last_start;
        let last_end = last_start + last_height;
        if last_end > scroll_y + viewport_height {
            scroll_y = last_end.saturating_sub(viewport_height);
        }
    } else {
        let cursor_line = item_starts[active_sub_idx];
        if cursor_line < scroll_y {
            scroll_y = cursor_line;
        } else if cursor_line >= scroll_y + viewport_height {
            scroll_y = cursor_line.saturating_sub(viewport_height).saturating_add(1);
        }
    }

    f.render_widget(right_block, right_chunks[0]);

    let paragraph =
        Paragraph::new(right_items).alignment(Alignment::Left).scroll((scroll_y as u16, 0));
    f.render_widget(paragraph, content_inner);

    // 3. Render description panel at the bottom
    let desc_title = " Description ";
    let desc_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(muted_style())
        .title(Span::styled(desc_title, muted_style()))
        .padding(Padding::horizontal(1));
    let desc_inner = desc_block.inner(right_chunks[1]);
    f.render_widget(desc_block, right_chunks[1]);

    let active_desc = if app.settings_focus_sidebar {
        match active_cat {
            0 => {
                "Configuration for background polling intervals, default git executable, external text editor, caching TTL, and general UI display toggles."
            }
            1 => {
                "Controls repository list sorting rules (alphabetical, custom, recent visits), list sorting direction, and limits on maximum parsed commits."
            }
            2 => {
                "Configures primary repository discovery paths, recursive search depths, directory exclusion lists, and watch paths for automatic workspace sync."
            }
            3 => {
                "Select active TUI visual themes and toggle compact card/row layouts for list views."
            }
            4 => {
                "Custom key mappings and keyboard shortcuts for navigating repositories, committing changes, triggering git actions, and UI overlays."
            }
            _ => "",
        }
    } else {
        get_desc(app.settings_selected_index)
    };
    let desc_width = (desc_inner.width as usize).saturating_sub(2).max(10);
    let desc_lines: Vec<Line> = wrap_str(active_desc, desc_width)
        .into_iter()
        .map(|s| Line::from(Span::styled(s, muted_style())))
        .collect();
    let desc_para = Paragraph::new(desc_lines);
    f.render_widget(desc_para, desc_inner);

    // Dropdown rendering for theme name (index 3)
    if app.settings_editing && app.settings_selected_index == 3 && !app.settings_focus_sidebar {
        let dropdown_width = 30;
        let dropdown_height = (app.settings_theme_list.len() + 2) as u16;

        let theme_row_y = item_starts[active_sub_idx] as u16;
        let dropdown_x = content_inner.x + 15;
        let dropdown_y = (content_inner.y + theme_row_y + 2).saturating_sub(scroll_y as u16);

        let dropdown_area = Rect::new(
            dropdown_x.min(area.right().saturating_sub(dropdown_width)),
            dropdown_y.min(area.bottom().saturating_sub(dropdown_height)),
            dropdown_width.min(area.width),
            dropdown_height.min(area.height),
        );

        let dropdown_block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(accent_style())
            .title(Span::styled(" Select Theme ", accent_style()));

        f.render_widget(Clear, dropdown_area);
        f.render_widget(dropdown_block.clone(), dropdown_area);

        let dropdown_inner = dropdown_block.inner(dropdown_area);

        let mut theme_spans = Vec::new();
        for (idx, theme_name) in app.settings_theme_list.iter().enumerate() {
            let is_active = idx == app.settings_theme_index;
            let prefix = if is_active { if is_compat { "> " } else { "▶ " } } else { "  " };
            let style = if is_active {
                accent_style().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            theme_spans.push(Line::from(Span::styled(format!("{}{}", prefix, theme_name), style)));
        }

        let list = Paragraph::new(theme_spans);
        f.render_widget(list, dropdown_inner);
    }

    // Cursor position rendering
    if app.settings_editing && app.settings_selected_index != 3 && !app.settings_focus_sidebar {
        let cursor_line = item_starts[active_sub_idx] + (selected_val_chunks_len - 1);

        if cursor_line >= scroll_y && cursor_line < scroll_y + viewport_height {
            let cursor_y = (content_inner.y + cursor_line as u16).saturating_sub(scroll_y as u16);
            let cursor_x = content_inner.x
                + selected_val_offset as u16
                + selected_last_chunk_char_count.saturating_sub(1) as u16;
            f.set_cursor_position(Position::new(cursor_x, cursor_y));
        }
    }
}

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
pub struct SettingsPopup;
impl SettingsPopup {
    pub fn handle_event(app: &mut crate::app::App, key: KeyEvent) -> bool {
        app.status_message = None;
        let code = key.code;
        if app.settings_editing && app.settings_selected_index == 3 {
            match code {
                KeyCode::Esc => app.cancel_settings_edit(),
                KeyCode::Enter => app.commit_settings_edit(),
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J')
                    if app.settings_theme_index + 1 < app.settings_theme_list.len() =>
                {
                    app.settings_theme_index += 1;
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K')
                    if app.settings_theme_index > 0 =>
                {
                    app.settings_theme_index -= 1;
                }
                KeyCode::PageUp if app.settings_theme_index > 0 => {
                    app.settings_theme_index =
                        app.settings_theme_index.saturating_sub(app.config.page_size);
                }
                KeyCode::PageDown
                    if app.settings_theme_index + 1 < app.settings_theme_list.len() =>
                {
                    app.settings_theme_index = (app.settings_theme_index + app.config.page_size)
                        .min(app.settings_theme_list.len().saturating_sub(1));
                }
                KeyCode::Home => {
                    app.settings_theme_index = 0;
                }
                KeyCode::End if !app.settings_theme_list.is_empty() => {
                    app.settings_theme_index = app.settings_theme_list.len() - 1;
                }
                _ => {}
            }
        } else if app.settings_editing {
            match code {
                KeyCode::Esc => app.cancel_settings_edit(),
                KeyCode::Enter => app.commit_settings_edit(),
                KeyCode::Backspace => app.input_backspace(),
                KeyCode::Char(c) => app.input_char(c),
                _ => {}
            }
        } else {
            // Non-editing mode
            match code {
                KeyCode::Esc => {
                    if !app.settings_focus_sidebar {
                        app.settings_focus_sidebar = true;
                    } else {
                        app.mode = Mode::Normal;
                    }
                }
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    app.mode = Mode::Normal;
                }
                KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
                    app.settings_focus_sidebar = true;
                }
                KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                    app.settings_focus_sidebar = false;
                    let cat = get_active_category(app.settings_selected_index);
                    app.settings_selected_index = get_category_indices(cat)[0];
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    app.settings_focus_sidebar = false;
                    let cat = get_active_category(app.settings_selected_index);
                    app.settings_selected_index = get_category_indices(cat)[0];
                }
                KeyCode::Char('1') => {
                    app.settings_selected_index = GENERAL_SETTING_INDICES[0];
                    app.settings_focus_sidebar = false;
                }
                KeyCode::Char('2') => {
                    app.settings_selected_index = SORTING_SETTING_INDICES[0];
                    app.settings_focus_sidebar = false;
                }
                KeyCode::Char('3') => {
                    app.settings_selected_index = SCAN_SETTING_INDICES[0];
                    app.settings_focus_sidebar = false;
                }
                KeyCode::Char('4') => {
                    app.settings_selected_index = THEME_SETTING_INDICES[0];
                    app.settings_focus_sidebar = false;
                }
                KeyCode::Char('5') => {
                    app.settings_selected_index = ALL_KEYBINDINGS_SETTING_INDICES[0];
                    app.settings_focus_sidebar = false;
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    if app.settings_focus_sidebar {
                        let cat = get_active_category(app.settings_selected_index);
                        if cat + 1 < 5 {
                            app.settings_selected_index = get_category_indices(cat + 1)[0];
                        }
                    } else {
                        let cat = get_active_category(app.settings_selected_index);
                        let indices = get_category_indices(cat);
                        let sub = get_sub_index(app.settings_selected_index);
                        if sub + 1 < indices.len() {
                            app.settings_selected_index = indices[sub + 1];
                        }
                    }
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    if app.settings_focus_sidebar {
                        let cat = get_active_category(app.settings_selected_index);
                        if cat > 0 {
                            app.settings_selected_index = get_category_indices(cat - 1)[0];
                        }
                    } else {
                        let cat = get_active_category(app.settings_selected_index);
                        let indices = get_category_indices(cat);
                        let sub = get_sub_index(app.settings_selected_index);
                        if sub > 0 {
                            app.settings_selected_index = indices[sub - 1];
                        }
                    }
                }
                KeyCode::PageUp => {
                    if app.settings_focus_sidebar {
                        app.settings_selected_index = GENERAL_SETTING_INDICES[0];
                    } else {
                        let cat = get_active_category(app.settings_selected_index);
                        let indices = get_category_indices(cat);
                        let sub = get_sub_index(app.settings_selected_index);
                        let page = app.config.page_size.min(indices.len()).max(1);
                        let new_sub = sub.saturating_sub(page);
                        app.settings_selected_index = indices[new_sub];
                    }
                }
                KeyCode::PageDown => {
                    if app.settings_focus_sidebar {
                        app.settings_selected_index = ALL_KEYBINDINGS_SETTING_INDICES[0];
                    } else {
                        let cat = get_active_category(app.settings_selected_index);
                        let indices = get_category_indices(cat);
                        let sub = get_sub_index(app.settings_selected_index);
                        let page = app.config.page_size.min(indices.len()).max(1);
                        let new_sub = (sub + page).min(indices.len() - 1);
                        app.settings_selected_index = indices[new_sub];
                    }
                }
                KeyCode::Home => {
                    if app.settings_focus_sidebar {
                        app.settings_selected_index = GENERAL_SETTING_INDICES[0];
                    } else {
                        let cat = get_active_category(app.settings_selected_index);
                        app.settings_selected_index = get_category_indices(cat)[0];
                    }
                }
                KeyCode::End => {
                    if app.settings_focus_sidebar {
                        app.settings_selected_index = ALL_KEYBINDINGS_SETTING_INDICES[0];
                    } else {
                        let cat = get_active_category(app.settings_selected_index);
                        let indices = get_category_indices(cat);
                        app.settings_selected_index = indices[indices.len() - 1];
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if app.settings_focus_sidebar {
                        app.settings_focus_sidebar = false;
                    } else {
                        app.toggle_or_edit_setting();
                    }
                }
                _ => {}
            }
        }
        true
    }
}
