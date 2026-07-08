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

const GENERAL_SETTING_INDICES: &[usize] = &[9, 56, 55, 0, 60, 13, 65, 66, 12, 58, 62, 63, 7];
const SORTING_SETTING_INDICES: &[usize] = &[1, 2, 6, 64];
const SCAN_SETTING_INDICES: &[usize] = &[5, 4, 8, 61];
const THEME_SETTING_INDICES: &[usize] = &[3, 67];
const KEYBINDINGS_SETTING_INDICES: &[usize] = &[
    // --- Global Actions ---
    16, // Quit / Close Dialog (Close)
    15, // Help (Help)
    14, // Toggle Status Bar (ToggleStatusBar)
    // --- Home Navigation & Actions ---
    17, // Home: Move Down (HomeMoveDown)
    18, // Home: Move Up (HomeMoveUp)
    19, // Home: Page Down (HomePageDown)
    20, // Home: Page Up (HomePageUp)
    21, // Home: Go to Top (HomeHome)
    22, // Home: Go to Bottom (HomeEnd)
    38, // Home: Open Details (HomeOpenDetail)
    75, // Home: Toggle Selection (HomeSelect)
    33, // Home: Toggle Pin (HomeTogglePin)
    71, // Home: Toggle Star (HomeToggleStar)
    72, // Home: Yank Path (HomeYankPath)
    73, // Home: Jump Picker (HomeJumpPicker)
    76, // Home: Global Code Search (HomeGlobalSearch)
    37, // Home: Search Repository (HomeSearchRepo)
    30, // Home: Refresh Status (HomeRefresh)
    31, // Home: Cycle Sort Order (HomeCycleSort)
    32, // Home: Toggle Sort Reverse (HomeToggleSortReverse)
    77, // Home: Toggle Compact View (HomeToggleCompactView)
    23, // Home: Add Repository (HomeAddRepo)
    24, // Home: Bulk Add (HomeBulkAdd)
    35, // Home: Import Repository (HomeImportRepo)
    25, // Home: Edit Repository (HomeEditRepo)
    26, // Home: Delete Repository (HomeDeleteRepo)
    28, // Home: Edit Labels (HomeEditLabels)
    70, // Home: Open Terminal (HomeOpenTerminal)
    36, // Home: Open Git App (HomeOpenGitApp)
    27, // Home: Open Debug Logs (HomeOpenDebugLogs)
    29, // Home: Open About Dialog (HomeAbout)
    78, // Home: Signs & Symbols Legend (HomeSymbolsHelp)
    57, // Home: Check Updates (HomeCheckUpdate)
    // --- Detail/Workspace Navigation & Actions ---
    39, // Detail: Close View (CloseDetail)
    40, // Detail: Help (DetailHelp)
    41, // Detail: Cycle Focus Fwd (CycleFocusForward)
    42, // Detail: Cycle Focus Bwd (CycleFocusBackward)
    43, // Detail: Refresh View (RefreshDetail)
    44, // Detail: Cycle Tab Fwd (CycleTabForward)
    45, // Detail: Cycle Tab Bwd (CycleTabBackward)
    46, // Detail: Tab 1 (Commits / Worktrees)
    47, // Detail: Tab 2 (Files / Submodules)
    48, // Detail: Tab 3 (Graph / Reflog)
    49, // Detail: Tab 4 (Branches / Forge)
    50, // Detail: Tab 5 (Tags)
    51, // Detail: Tab 6 (Remotes)
    52, // Detail: Tab 7 (Stashes)
    68, // Detail: Show Overview (Overview)
    69, // Detail: Toggle Advanced Tabs (ToggleAdvancedTabs)
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
        4 => KEYBINDINGS_SETTING_INDICES,
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
        46 => "Detail: Tab 1 (Commits) / Advanced 1 (Worktrees)",
        47 => "Detail: Tab 2 (Files) / Advanced 2 (Submodules)",
        48 => "Detail: Tab 3 (Graph) / Advanced 3 (Reflog)",
        49 => "Detail: Tab 4 (Branches) / Advanced 4 (Forge)",
        50 => "Detail: Tab 5 (Tags)",
        51 => "Detail: Tab 6 (Remotes)",
        52 => "Detail: Tab 7 (Stashes)",

        68 => "Detail: Show Overview",
        69 => "Detail: Toggle Advanced Tabs",
        70 => "Home: Open Terminal",
        71 => "Home: Toggle Star",
        72 => "Home: Yank Path",
        73 => "Home: Jump Picker",
        74 => "Home: Fetch All",
        75 => "Home: Toggle Selection",
        76 => "Home: Global Code Search",
        77 => "Home: Toggle Compact View",
        78 => "Home: Signs & Symbols Legend",
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
    if KEYBINDINGS_SETTING_INDICES.contains(&global_idx) {
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
            67 => app.config.compact_view.to_string(),
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

    let sidebar_paragraph = Paragraph::new(sidebar_items);
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
                    app.settings_selected_index = KEYBINDINGS_SETTING_INDICES[0];
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
                        app.settings_selected_index = get_category_indices(cat)[0];
                    }
                }
                KeyCode::PageDown => {
                    if app.settings_focus_sidebar {
                        app.settings_selected_index = KEYBINDINGS_SETTING_INDICES[0];
                    } else {
                        let cat = get_active_category(app.settings_selected_index);
                        let indices = get_category_indices(cat);
                        app.settings_selected_index = indices[indices.len() - 1];
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
                        app.settings_selected_index = KEYBINDINGS_SETTING_INDICES[0];
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
