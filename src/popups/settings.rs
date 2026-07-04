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

const GENERAL_SETTING_INDICES: &[usize] = &[0, 7, 9, 12, 13, 58, 55, 56, 60];
const SORTING_SETTING_INDICES: &[usize] = &[1, 2, 6];
const SCAN_SETTING_INDICES: &[usize] = &[5, 4, 10, 8, 61];
const THEME_SETTING_INDICES: &[usize] = &[3];
const KEYBINDINGS_SETTING_INDICES: &[usize] = &[
    14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37,
    38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 59, 57,
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

fn get_label(global_idx: usize) -> &'static str {
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
        10 => "Scan Git Only",
        12 => "Compatibility Mode",
        13 => "Resync on Tab Change",
        58 => "Show Grouping",
        55 => "SSH Strict Host Checking",
        56 => "Editor Command",
        60 => "Auto-Fetch Interval (mins)",
        61 => "Watch Directories",
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
        46 => "Detail: Tab 1 (Commits)",
        47 => "Detail: Tab 2 (Files)",
        48 => "Detail: Tab 3 (Graph)",
        49 => "Detail: Tab 4 (Branches)",
        50 => "Detail: Tab 5 (Tags)",
        51 => "Detail: Tab 6 (Remotes)",
        52 => "Detail: Tab 7 (Stashes)",
        53 => "Detail: Tab 8 (Worktrees)",
        54 => "Detail: Tab 9 (Submodules)",
        59 => "Detail: Tab 10 (Reflog)",
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
        46 => "Switches directly to the Commits list tab.",
        47 => "Switches directly to the Working Tree files tab.",
        48 => "Switches directly to the Git History Graph tab.",
        49 => "Switches directly to the local branches list tab.",
        50 => "Switches directly to the local tags list tab.",
        51 => "Switches directly to the configured remotes tab.",
        52 => "Switches directly to the git stashes list tab.",
        53 => "Switches directly to the repository Worktrees list tab.",
        54 => "Switches directly to the repository Submodules tab.",
        59 => "Switches directly to the repository Reflog list tab.",
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
    let indices = get_category_indices(active_cat);
    let right_inner_rect = chunks[2].inner(Margin { horizontal: 1, vertical: 1 });
    let available_text_width = (right_inner_rect.width as usize).saturating_sub(6);

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
        let desc = get_desc(global_idx);
        let val_str = get_val_str(app, global_idx);

        let val_offset = if is_selected && app.settings_editing { 11 } else { 5 };
        let val_width = available_text_width.saturating_sub(val_offset).max(10);
        let val_chunks = if global_idx == 8 {
            wrap_excludes(&val_str, val_width)
        } else {
            wrap_str(&val_str, val_width)
        };

        let desc_offset = 5;
        let desc_width = available_text_width.saturating_sub(desc_offset).max(10);
        let desc_chunks = wrap_str(desc, desc_width);

        item_starts[sub_idx] = current_line;
        let item_height = 1 + val_chunks.len() + desc_chunks.len() + 1; // label + value + desc + spacer
        current_line += item_height;

        if is_selected {
            selected_val_chunks_len = val_chunks.len();
            selected_last_chunk_char_count =
                val_chunks.last().map(|c| c.chars().count()).unwrap_or(0);
            selected_val_offset = val_offset;
        }

        let prefix = if is_selected { "> " } else { "  " };

        // Line 1: Label line
        right_items.push(Line::from(vec![
            Span::styled(prefix, if is_selected { accent_style() } else { muted_style() }),
            Span::styled(
                label,
                if is_selected {
                    accent_style().add_modifier(Modifier::BOLD)
                } else {
                    primary_style()
                },
            ),
        ]));

        // Line 2: First line of value (indented by val_offset)
        let mut val_line_spans = Vec::new();
        if is_selected && app.settings_editing {
            let label_edit = if global_idx == 3 { "   [Select]: " } else { "   [Edit]: " };
            val_line_spans.push(Span::styled(label_edit, muted_style()));
            val_line_spans.push(Span::styled(
                val_chunks[0].clone(),
                Style::default().fg(ACCENT()).add_modifier(Modifier::UNDERLINED),
            ));
        } else {
            val_line_spans.push(Span::styled("   : ", muted_style()));
            val_line_spans.push(Span::styled(
                val_chunks[0].clone(),
                if is_selected { accent_style() } else { Style::default() },
            ));
        }
        right_items.push(Line::from(val_line_spans));

        // Subsequent lines of the value (indented by val_offset spaces)
        for chunk in val_chunks.iter().skip(1) {
            let spaces = " ".repeat(val_offset);
            let span = Span::styled(
                chunk.clone(),
                if is_selected && app.settings_editing {
                    Style::default().fg(ACCENT()).add_modifier(Modifier::UNDERLINED)
                } else if is_selected {
                    accent_style()
                } else {
                    Style::default()
                },
            );
            right_items.push(Line::from(vec![Span::raw(spaces), span]));
        }

        // Description lines (indented by 5 spaces)
        for chunk in desc_chunks {
            right_items
                .push(Line::from(vec![Span::raw("     "), Span::styled(chunk, muted_style())]));
        }

        // Spacer
        right_items.push(Line::from(""));
    }

    let viewport_height = right_inner_rect.height as usize;
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

    if app.settings_editing && app.settings_selected_index != 3 && !app.settings_focus_sidebar {
        let cursor_line = item_starts[active_sub_idx] + 1 + (selected_val_chunks_len - 1);
        if cursor_line < scroll_y {
            scroll_y = cursor_line;
        } else if cursor_line >= scroll_y + viewport_height {
            scroll_y = cursor_line.saturating_sub(viewport_height).saturating_add(1);
        }
    }

    let right_title = format!(" {} ", get_category_name(active_cat));
    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(if !app.settings_focus_sidebar { accent_style() } else { muted_style() })
        .title(Span::styled(right_title, primary_style()))
        .padding(Padding::horizontal(1));

    let content_inner = right_block.inner(chunks[2]);
    f.render_widget(right_block, chunks[2]);

    let paragraph =
        Paragraph::new(right_items).alignment(Alignment::Left).scroll((scroll_y as u16, 0));
    f.render_widget(paragraph, content_inner);

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
        let cursor_line = item_starts[active_sub_idx] + 1 + (selected_val_chunks_len - 1);

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
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    app.settings_focus_sidebar = false;
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
