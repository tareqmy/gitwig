//! Per-label settings editor. Settings chosen here apply to every repository
//! carrying the label, sitting between the per-repo override and the global
//! default in the resolution order (repo → label → global).

use crate::app::{App, Mode};
use crate::ui::style::{CARD_BORDER, accent_style, muted_style, primary_style};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};

/// Number of rows in the popup (0..ROW_COUNT).
const ROW_COUNT: usize = 6;

pub struct LabelSettingsPopup;

impl LabelSettingsPopup {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        let code = key.code;

        let label = match app.label_settings_target.clone() {
            Some(l) => l,
            None => {
                // No target: nothing to edit — fall back to the picker.
                app.mode = Mode::LabelPicker;
                return true;
            }
        };

        if app.label_settings_editing {
            match code {
                KeyCode::Esc => {
                    app.label_settings_editing = false;
                    return true;
                }
                _ if app.keybindings.matches(crate::keybindings::Action::NavEnter, key) => {
                    let mut lc = app.config.label_configs.get(&label).cloned().unwrap_or_default();
                    match app.label_settings_selected_index {
                        1 => {
                            let val_opt = if app.label_settings_input.is_empty() {
                                None
                            } else if let Ok(val) = app.label_settings_input.parse::<usize>() {
                                Some(val)
                            } else {
                                app.label_settings_editing = false;
                                return true;
                            };
                            lc.page_size = val_opt;
                            app.config.label_configs.insert(label, lc);
                            app.persist("Label page size updated");
                        }
                        2 => {
                            let val_opt = if app.label_settings_input.is_empty() {
                                None
                            } else if let Ok(val) = app.label_settings_input.parse::<usize>() {
                                Some(val)
                            } else {
                                app.label_settings_editing = false;
                                return true;
                            };
                            lc.max_commits = val_opt;
                            app.config.label_configs.insert(label, lc);
                            app.persist("Label max commits updated");
                        }
                        4 => {
                            let val_opt = if app.label_settings_input.is_empty() {
                                None
                            } else if let Ok(val) = app.label_settings_input.parse::<u64>() {
                                Some(val)
                            } else {
                                app.label_settings_editing = false;
                                return true;
                            };
                            lc.auto_fetch_interval_mins = val_opt;
                            app.config.label_configs.insert(label, lc);
                            app.persist("Label auto-fetch interval updated");
                        }
                        5 => {
                            let val_opt = if app.label_settings_input.trim().is_empty() {
                                None
                            } else {
                                Some(app.label_settings_input.trim().to_string())
                            };
                            lc.editor = val_opt;
                            app.config.label_configs.insert(label, lc);
                            app.persist("Label editor updated");
                        }
                        _ => {}
                    }
                    app.label_settings_editing = false;
                    return true;
                }
                KeyCode::Backspace => {
                    app.label_settings_input.pop();
                    return true;
                }
                KeyCode::Char(c) if app.label_settings_selected_index == 5 => {
                    app.label_settings_input.push(c);
                    return true;
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    app.label_settings_input.push(c);
                    return true;
                }
                _ => {}
            }
            return true;
        }

        match code {
            _ if app.keybindings.matches(crate::keybindings::Action::NavEsc, key) => {
                // Return to the label picker the popup was opened from.
                app.mode = Mode::LabelPicker;
                return true;
            }
            _ if app.keybindings.matches(crate::keybindings::Action::NavUp, key) => {
                app.label_settings_selected_index = if app.label_settings_selected_index == 0 {
                    ROW_COUNT - 1
                } else {
                    app.label_settings_selected_index - 1
                };
                return true;
            }
            _ if app.keybindings.matches(crate::keybindings::Action::NavDown, key) => {
                app.label_settings_selected_index =
                    (app.label_settings_selected_index + 1) % ROW_COUNT;
                return true;
            }
            _ if app.keybindings.matches(crate::keybindings::Action::NavLeft, key) => {
                Self::change_setting(app, &label, false);
                return true;
            }
            _ if app.keybindings.matches(crate::keybindings::Action::NavRight, key) => {
                Self::change_setting(app, &label, true);
                return true;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                match app.label_settings_selected_index {
                    0 | 3 => {
                        Self::change_setting(app, &label, true);
                    }
                    1 => {
                        let lc = app.config.label_configs.get(&label).cloned().unwrap_or_default();
                        app.label_settings_input =
                            lc.page_size.map(|v| v.to_string()).unwrap_or_default();
                        app.label_settings_editing = true;
                    }
                    2 => {
                        let lc = app.config.label_configs.get(&label).cloned().unwrap_or_default();
                        app.label_settings_input =
                            lc.max_commits.map(|v| v.to_string()).unwrap_or_default();
                        app.label_settings_editing = true;
                    }
                    4 => {
                        let lc = app.config.label_configs.get(&label).cloned().unwrap_or_default();
                        app.label_settings_input =
                            lc.auto_fetch_interval_mins.map(|v| v.to_string()).unwrap_or_default();
                        app.label_settings_editing = true;
                    }
                    5 => {
                        let lc = app.config.label_configs.get(&label).cloned().unwrap_or_default();
                        app.label_settings_input = lc.editor.clone().unwrap_or_default();
                        app.label_settings_editing = true;
                    }
                    _ => {}
                }
                return true;
            }
            _ => {}
        }
        false
    }

    fn change_setting(app: &mut App, label: &str, right: bool) {
        let mut lc = app.config.label_configs.get(label).cloned().unwrap_or_default();
        match app.label_settings_selected_index {
            0 => {
                // `get_available_themes` already contains "default" exactly
                // once (and every installed theme, sorted). Do NOT prepend
                // another "default": the duplicate makes `position` snap back to
                // index 0 mid-cycle, truncating the reachable set to its first
                // few entries.
                let themes = app.get_available_themes();
                let current_theme = lc.theme.as_deref().unwrap_or("default");
                let current_idx = themes.iter().position(|t| t == current_theme).unwrap_or(0);
                let next_idx = if right {
                    (current_idx + 1) % themes.len()
                } else if current_idx == 0 {
                    themes.len() - 1
                } else {
                    current_idx - 1
                };
                let new_theme = &themes[next_idx];
                if new_theme == "default" {
                    lc.theme = None;
                } else {
                    lc.theme = Some(new_theme.clone());
                }
                app.config.label_configs.insert(label.to_string(), lc);
                app.persist(&format!("Label theme set to '{}'", new_theme));
            }
            3 => {
                let next_state = match lc.resync_on_tab_change {
                    None => {
                        if right {
                            Some(true)
                        } else {
                            Some(false)
                        }
                    }
                    Some(true) => {
                        if right {
                            Some(false)
                        } else {
                            None
                        }
                    }
                    Some(false) => {
                        if right {
                            None
                        } else {
                            Some(true)
                        }
                    }
                };
                lc.resync_on_tab_change = next_state;
                let desc = match next_state {
                    None => "Default".to_string(),
                    Some(true) => "Yes".to_string(),
                    Some(false) => "No".to_string(),
                };
                app.config.label_configs.insert(label.to_string(), lc);
                app.persist(&format!("Label Resync on Tab Change set to {}", desc));
            }
            _ => {}
        }
    }

    pub fn draw(f: &mut Frame, app: &App, area: Rect) {
        let popup_width = 54;
        let popup_height = 17;
        let popup_area = crate::ui::layout::centered_rect_fixed(popup_width, popup_height, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(accent_style())
            .title(Span::styled(" Label Settings ", accent_style()))
            .padding(Padding::horizontal(1));

        f.render_widget(Clear, popup_area);
        f.render_widget(block.clone(), popup_area);

        let inner = block.inner(popup_area);

        let label = app.label_settings_target.as_deref().unwrap_or("");

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Label name
                Constraint::Length(1), // Spacer
                Constraint::Min(6),    // Settings items list
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Shortcuts instructions
            ])
            .split(inner);

        f.render_widget(
            Paragraph::new(vec![Line::from(vec![
                Span::raw("Label: "),
                Span::styled(label.to_string(), primary_style()),
            ])])
            .alignment(Alignment::Center),
            chunks[1],
        );

        let lc = app
            .label_settings_target
            .as_ref()
            .and_then(|l| app.config.label_configs.get(l))
            .cloned()
            .unwrap_or_default();

        let selected = app.label_settings_selected_index;
        let editing = app.label_settings_editing;

        let build_line = |idx: usize,
                          label: &str,
                          value: &str,
                          is_editing: bool|
         -> Line<'static> {
            let is_selected = idx == selected;
            let prefix = if is_selected { "▶ " } else { "  " };

            let mut spans = vec![
                Span::styled(prefix, if is_selected { accent_style() } else { Style::default() }),
                Span::styled(
                    format!("{:<24}", label),
                    if is_selected {
                        primary_style().add_modifier(Modifier::BOLD)
                    } else {
                        primary_style()
                    },
                ),
            ];

            if is_selected && is_editing {
                spans.push(Span::styled("[ ", muted_style()));
                spans.push(Span::styled(
                    value.to_string(),
                    primary_style().add_modifier(Modifier::BOLD),
                ));
                spans.push(Span::styled("█ ]", accent_style().add_modifier(Modifier::BOLD)));
            } else if label == "Theme:" || label == "Resync on Tab Change:" {
                spans.push(Span::styled("< ", muted_style()));
                spans.push(Span::styled(
                    value.to_string(),
                    if is_selected {
                        accent_style().add_modifier(Modifier::BOLD)
                    } else {
                        primary_style()
                    },
                ));
                spans.push(Span::styled(" >", muted_style()));
            } else {
                spans.push(Span::styled("[ ", muted_style()));
                spans.push(Span::styled(
                    value.to_string(),
                    if is_selected {
                        accent_style().add_modifier(Modifier::BOLD)
                    } else {
                        primary_style()
                    },
                ));
                spans.push(Span::styled(" ]", muted_style()));
            }

            let line_style = if is_selected && !is_editing {
                Style::default().bg(ratatui::style::Color::Rgb(60, 60, 60))
            } else {
                Style::default()
            };

            Line::from(spans).style(line_style)
        };

        // Row 0: Theme
        let theme_val = lc.theme.clone().unwrap_or_else(|| "default".to_string());
        let theme_line = build_line(0, "Theme:", &theme_val, false);

        // Row 1: Page Size
        let page_size_val =
            lc.page_size.map(|v| v.to_string()).unwrap_or_else(|| "default".to_string());
        let page_size_line = build_line(
            1,
            "Page Size:",
            if selected == 1 && editing { &app.label_settings_input } else { &page_size_val },
            selected == 1 && editing,
        );

        // Row 2: Max Commits
        let max_commits_val =
            lc.max_commits.map(|v| v.to_string()).unwrap_or_else(|| "default".to_string());
        let max_commits_line = build_line(
            2,
            "Max Commits:",
            if selected == 2 && editing { &app.label_settings_input } else { &max_commits_val },
            selected == 2 && editing,
        );

        // Row 3: Resync on Tab Change
        let resync_val = match lc.resync_on_tab_change {
            None => "default",
            Some(true) => "yes",
            Some(false) => "no",
        };
        let resync_line = build_line(3, "Resync on Tab Change:", resync_val, false);

        // Row 4: Auto Fetch Interval
        let auto_fetch_val = match lc.auto_fetch_interval_mins {
            None => "default".to_string(),
            Some(0) => "0 (disabled)".to_string(),
            Some(v) => v.to_string(),
        };
        let auto_fetch_line = build_line(
            4,
            "Auto Fetch (mins):",
            if selected == 4 && editing { &app.label_settings_input } else { &auto_fetch_val },
            selected == 4 && editing,
        );

        // Row 5: Editor Command
        let editor_val = lc.editor.clone().unwrap_or_else(|| "default".to_string());
        let editor_line = build_line(
            5,
            "Editor Command:",
            if selected == 5 && editing { &app.label_settings_input } else { &editor_val },
            selected == 5 && editing,
        );

        let settings_lines = vec![
            theme_line,
            page_size_line,
            max_commits_line,
            resync_line,
            auto_fetch_line,
            editor_line,
        ];
        f.render_widget(Paragraph::new(settings_lines), chunks[3]);

        // Shortcuts helper bar
        let helper_line = if editing {
            if selected == 5 {
                Line::from(vec![
                    Span::styled(" [Text] ", accent_style()),
                    Span::styled("Type  ", muted_style()),
                    Span::styled(" [Enter] ", accent_style()),
                    Span::styled("Confirm  ", muted_style()),
                    Span::styled(" [Esc] ", accent_style()),
                    Span::styled("Cancel", muted_style()),
                ])
            } else {
                Line::from(vec![
                    Span::styled(" [Digits] ", accent_style()),
                    Span::styled("Type  ", muted_style()),
                    Span::styled(" [Enter] ", accent_style()),
                    Span::styled("Confirm  ", muted_style()),
                    Span::styled(" [Esc] ", accent_style()),
                    Span::styled("Cancel", muted_style()),
                ])
            }
        } else {
            Line::from(vec![
                Span::styled(" [↑/↓/j/k] ", accent_style()),
                Span::styled("Navigate  ", muted_style()),
                Span::styled(" [←/→/h/l/Space] ", accent_style()),
                Span::styled("Change/Edit  ", muted_style()),
                Span::styled(" [Esc] ", accent_style()),
                Span::styled("Back", muted_style()),
            ])
        };
        f.render_widget(Paragraph::new(helper_line).alignment(Alignment::Center), chunks[5]);
    }
}
