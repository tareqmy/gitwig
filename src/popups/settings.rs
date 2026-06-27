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
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState, Padding, Paragraph, Wrap,
};

pub fn draw_settings_page(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(65, 75, area);

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

    let available_text_width = (inner_rect.width as usize).saturating_sub(2);

    // First pass: compute chunks and item layout
    let mut items_data = Vec::new();
    let mut current_line = 0;
    let mut item_starts = [0; 14];

    let mut selected_val_chunks_len = 1;
    let mut selected_last_chunk_char_count: usize = 0;
    let mut selected_val_offset = 11;

    for i in 0..14 {
        let is_selected = app.settings_selected_index == i;
        let label = match i {
            0 => "Poll Interval (ms)",
            1 => "Sort By",
            2 => "Sort Reverse",
            3 => "Theme Name",
            4 => "FZF Max Depth",
            5 => "FZF Start Dir",
            6 => "Max Commits",
            7 => "Page Size",
            8 => "FZF Exclude Folders",
            9 => "Preferred Git Client",
            10 => "FZF Git Only",
            11 => "Use FZF",
            12 => "Compatibility Mode",
            13 => "Resync on Tab Change",
            _ => "",
        };

        let desc = match i {
            0 => "Event-loop poll interval in milliseconds. Sane range: 16-500.",
            1 => "Initial repository sorting criteria.",
            2 => "Reverse the order of repositories.",
            3 => "Active theme configuration name. Press Enter/Space to select from dropdown.",
            4 => "Maximum directory depth to search for git repositories.",
            5 => "Starting directory for interactive repository discovery via FZF.",
            6 => "Maximum commits to load in workspace view. Set to 0 for unlimited.",
            7 => "Number of lines/items scrolled by Page Up / Page Down.",
            8 => "Comma-separated list of folders/patterns to exclude from FZF search.",
            9 => "External Git application triggered by 'g' key (e.g. gitui or lazygit).",
            10 => "Only scan folders that contain a .git directory.",
            11 => {
                "Whether to use FZF for repository discovery. If disabled, manual text input is used."
            }
            12 => {
                "Use simple ASCII symbols instead of complex Unicode emojis/icons to avoid layout breakage in some terminals."
            }
            13 => {
                "Whether to automatically refresh repository details from disk when switching tabs inside a repository."
            }
            _ => "",
        };

        let val_str = match i {
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
                    app.config.fzf.max_depth.to_string()
                }
            }
            5 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.fzf.start_dir.clone()
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
                    app.config.fzf.excludes.join(",")
                }
            }
            9 => {
                if is_selected && app.settings_editing {
                    format!("{}█", app.input_buffer)
                } else {
                    app.config.git_app.clone()
                }
            }
            10 => app.config.fzf.git_only.to_string(),
            11 => app.config.fzf.enabled.to_string(),
            12 => app.config.compatibility_mode.to_string(),
            13 => app.config.resync_on_tab_change.to_string(),
            _ => String::new(),
        };

        let val_offset = if is_selected && app.settings_editing { 11 } else { 5 };
        let val_width = available_text_width.saturating_sub(val_offset).max(10);
        let val_chunks =
            if i == 8 { wrap_excludes(&val_str, val_width) } else { wrap_str(&val_str, val_width) };

        let desc_offset = 5;
        let desc_width = available_text_width.saturating_sub(desc_offset).max(10);
        let desc_chunks = wrap_str(desc, desc_width);

        item_starts[i] = current_line;
        let item_height = 1 + val_chunks.len() + desc_chunks.len() + 1; // label + value + desc + spacer
        current_line += item_height;

        if is_selected {
            selected_val_chunks_len = val_chunks.len();
            selected_last_chunk_char_count =
                val_chunks.last().map(|c| c.chars().count()).unwrap_or(0);
            selected_val_offset = val_offset;
        }

        items_data.push((label, is_selected, val_chunks, desc_chunks, val_offset));
    }

    let total_height = current_line;
    let viewport_height = inner_rect.height as usize;
    let mut scroll_y = if viewport_height >= total_height {
        0
    } else {
        let sel_idx = app.settings_selected_index;
        let sel_start = item_starts[sel_idx];
        let sel_height = if sel_idx < 13 {
            item_starts[sel_idx + 1] - sel_start
        } else {
            total_height - sel_start
        };
        let item_center = sel_start + sel_height / 2;
        let half_viewport = viewport_height / 2;
        let target_scroll = item_center.saturating_sub(half_viewport);
        let max_scroll = total_height.saturating_sub(viewport_height);
        target_scroll.min(max_scroll)
    };

    if app.settings_editing && app.settings_selected_index != 3 {
        let sel_idx = app.settings_selected_index;
        let cursor_line = item_starts[sel_idx] + 1 + (selected_val_chunks_len - 1);
        if cursor_line < scroll_y {
            scroll_y = cursor_line;
        } else if cursor_line >= scroll_y + viewport_height {
            scroll_y = cursor_line.saturating_sub(viewport_height).saturating_add(1);
        }
    }

    let mut items = Vec::new();
    for (i, (label, is_selected, val_chunks, desc_chunks, val_offset)) in
        items_data.into_iter().enumerate()
    {
        let prefix = if is_selected { " > " } else { "   " };

        // Line 1: Label line
        items.push(Line::from(vec![
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
            let label_edit = if i == 3 { "   [Select]: " } else { "   [Edit]: " };
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
        items.push(Line::from(val_line_spans));

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
            items.push(Line::from(vec![Span::raw(spaces), span]));
        }

        // Description lines (indented by 5 spaces)
        for chunk in desc_chunks {
            items.push(Line::from(vec![Span::raw("     "), Span::styled(chunk, muted_style())]));
        }

        // Spacer
        items.push(Line::from(""));
    }

    let paragraph = Paragraph::new(items)
        .block(Block::default().padding(Padding::horizontal(1)))
        .alignment(Alignment::Left)
        .scroll((scroll_y as u16, 0));

    f.render_widget(paragraph, inner_rect);

    if app.settings_editing && app.settings_selected_index == 3 {
        // Draw the dropdown box
        let dropdown_width = 30;
        let dropdown_height = (app.settings_theme_list.len() + 2) as u16;

        // Position it near the theme name row
        let theme_row_y = item_starts[3] as u16;
        let dropdown_x = inner_rect.x + 25;
        let dropdown_y = (inner_rect.y + theme_row_y + 2).saturating_sub(scroll_y as u16);

        let dropdown_area = Rect::new(
            dropdown_x.min(area.right().saturating_sub(dropdown_width)),
            dropdown_y.min(area.bottom().saturating_sub(dropdown_height)),
            dropdown_width.min(area.width),
            dropdown_height.min(area.height),
        );

        let dropdown_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(accent_style())
            .title(Span::styled(" Select Theme ", accent_style()));

        f.render_widget(Clear, dropdown_area);
        f.render_widget(dropdown_block.clone(), dropdown_area);

        let dropdown_inner = dropdown_block.inner(dropdown_area);

        let mut theme_spans = Vec::new();
        for (idx, theme_name) in app.settings_theme_list.iter().enumerate() {
            let is_active = idx == app.settings_theme_index;
            let prefix = if is_active { "▶ " } else { "  " };
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

    if app.settings_editing && app.settings_selected_index != 3 {
        let sel_idx = app.settings_selected_index;
        let cursor_line = item_starts[sel_idx] + 1 + (selected_val_chunks_len - 1);

        if cursor_line >= scroll_y && cursor_line < scroll_y + viewport_height {
            let cursor_y = (inner_rect.y + cursor_line as u16).saturating_sub(scroll_y as u16);
            let cursor_x = inner_rect.x
                + 1
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
                KeyCode::Down if app.settings_theme_index + 1 < app.settings_theme_list.len() => {
                    app.settings_theme_index += 1;
                }
                KeyCode::Up if app.settings_theme_index > 0 => {
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
        } else {
            match code {
                KeyCode::Esc if app.settings_editing => app.cancel_settings_edit(),
                KeyCode::Esc => app.mode = Mode::Normal,
                KeyCode::Char('q') if !app.settings_editing => app.mode = Mode::Normal,
                KeyCode::Down if !app.settings_editing => {
                    if app.settings_selected_index + 1 < 14 {
                        app.settings_selected_index += 1;
                    }
                }
                KeyCode::Up if !app.settings_editing => {
                    if app.settings_selected_index > 0 {
                        app.settings_selected_index -= 1;
                    }
                }
                KeyCode::PageUp if !app.settings_editing => {
                    app.settings_selected_index =
                        app.settings_selected_index.saturating_sub(app.config.page_size);
                }
                KeyCode::PageDown if !app.settings_editing => {
                    app.settings_selected_index =
                        (app.settings_selected_index + app.config.page_size).min(13);
                }
                KeyCode::Home if !app.settings_editing => {
                    app.settings_selected_index = 0;
                }
                KeyCode::End if !app.settings_editing => {
                    app.settings_selected_index = 13;
                }
                KeyCode::Enter if app.settings_editing => app.commit_settings_edit(),
                KeyCode::Enter => app.toggle_or_edit_setting(),
                KeyCode::Char(' ') if !app.settings_editing => app.toggle_or_edit_setting(),
                KeyCode::Backspace if app.settings_editing => app.input_backspace(),
                KeyCode::Char(c) if app.settings_editing => app.input_char(c),
                _ => {}
            }
        }
        true
    }
}
