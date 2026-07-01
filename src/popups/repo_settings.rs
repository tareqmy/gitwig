use crate::app::{App, Mode};
use crate::ui::style::{CARD_BORDER, accent_style, muted_style, primary_style};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};

pub struct RepoSettingsPopup;

impl RepoSettingsPopup {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        let code = key.code;

        let repo_path = match app.get_selected_item().cloned() {
            Some(p) => p,
            None => return false,
        };

        if app.repo_settings_editing {
            match code {
                KeyCode::Esc => {
                    app.repo_settings_editing = false;
                    return true;
                }
                KeyCode::Enter => {
                    let mut repo_cfg =
                        app.config.repo_configs.get(&repo_path).cloned().unwrap_or_default();
                    match app.repo_settings_selected_index {
                        1 => {
                            let val_opt = if app.repo_settings_input.is_empty() {
                                None
                            } else if let Ok(val) = app.repo_settings_input.parse::<usize>() {
                                Some(val)
                            } else {
                                app.repo_settings_editing = false;
                                return true;
                            };
                            repo_cfg.page_size = val_opt;
                            app.config.repo_configs.insert(repo_path, repo_cfg);
                            app.persist("Repository page size updated");
                        }
                        2 => {
                            let val_opt = if app.repo_settings_input.is_empty() {
                                None
                            } else if let Ok(val) = app.repo_settings_input.parse::<usize>() {
                                Some(val)
                            } else {
                                app.repo_settings_editing = false;
                                return true;
                            };
                            repo_cfg.max_commits = val_opt;
                            app.config.repo_configs.insert(repo_path, repo_cfg);
                            app.persist("Repository max commits updated");
                        }
                        4 => {
                            let val_opt = if app.repo_settings_input.trim().is_empty() {
                                None
                            } else {
                                Some(app.repo_settings_input.trim().to_string())
                            };
                            repo_cfg.editor = val_opt;
                            app.config.repo_configs.insert(repo_path, repo_cfg);
                            app.persist("Repository editor updated");
                        }
                        5 => {
                            let val_opt = if app.repo_settings_input.trim().is_empty() {
                                None
                            } else {
                                Some(app.repo_settings_input.trim().to_string())
                            };
                            repo_cfg.note = val_opt;
                            app.config.repo_configs.insert(repo_path, repo_cfg);
                            app.persist("Repository note updated");
                        }
                        _ => {}
                    }
                    app.repo_settings_editing = false;
                    return true;
                }
                KeyCode::Backspace => {
                    app.repo_settings_input.pop();
                    return true;
                }
                KeyCode::Char(c) if app.repo_settings_selected_index == 4 || app.repo_settings_selected_index == 5 => {
                    app.repo_settings_input.push(c);
                    return true;
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    app.repo_settings_input.push(c);
                    return true;
                }
                _ => {}
            }
            return true;
        }

        match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.mode = Mode::Detail;
                return true;
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                app.repo_settings_selected_index = if app.repo_settings_selected_index == 0 {
                    5
                } else {
                    app.repo_settings_selected_index - 1
                };
                return true;
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                app.repo_settings_selected_index = (app.repo_settings_selected_index + 1) % 6;
                return true;
            }
            KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('H') => {
                Self::change_setting(app, &repo_path, false);
                return true;
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Char('L') => {
                Self::change_setting(app, &repo_path, true);
                return true;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                match app.repo_settings_selected_index {
                    0 | 3 => {
                        Self::change_setting(app, &repo_path, true);
                    }
                    1 => {
                        let repo_cfg =
                            app.config.repo_configs.get(&repo_path).cloned().unwrap_or_default();
                        app.repo_settings_input =
                            repo_cfg.page_size.map(|v| v.to_string()).unwrap_or_default();
                        app.repo_settings_editing = true;
                    }
                    2 => {
                        let repo_cfg =
                            app.config.repo_configs.get(&repo_path).cloned().unwrap_or_default();
                        app.repo_settings_input =
                            repo_cfg.max_commits.map(|v| v.to_string()).unwrap_or_default();
                        app.repo_settings_editing = true;
                    }
                    4 => {
                        let repo_cfg =
                            app.config.repo_configs.get(&repo_path).cloned().unwrap_or_default();
                        app.repo_settings_input = repo_cfg.editor.clone().unwrap_or_default();
                        app.repo_settings_editing = true;
                    }
                    5 => {
                        let repo_cfg =
                            app.config.repo_configs.get(&repo_path).cloned().unwrap_or_default();
                        app.repo_settings_input = repo_cfg.note.clone().unwrap_or_default();
                        app.repo_settings_editing = true;
                    }
                    _ => {}
                }
                return true;
            }
            _ => {}
        }
        false
    }

    fn change_setting(app: &mut App, repo_path: &str, right: bool) {
        let mut repo_cfg = app.config.repo_configs.get(repo_path).cloned().unwrap_or_default();
        match app.repo_settings_selected_index {
            0 => {
                let mut themes = vec!["default".to_string()];
                themes.extend(app.get_available_themes());
                let current_theme = repo_cfg.theme.as_deref().unwrap_or("default");
                let current_idx = themes.iter().position(|t| t == current_theme).unwrap_or(0);
                let next_idx = if right {
                    (current_idx + 1) % themes.len()
                } else {
                    if current_idx == 0 { themes.len() - 1 } else { current_idx - 1 }
                };
                let new_theme = &themes[next_idx];
                if new_theme == "default" {
                    repo_cfg.theme = None;
                } else {
                    repo_cfg.theme = Some(new_theme.clone());
                }
                app.config.repo_configs.insert(repo_path.to_string(), repo_cfg);
                app.persist(&format!("Repository theme set to '{}'", new_theme));
            }
            3 => {
                let next_state = match repo_cfg.resync_on_tab_change {
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
                repo_cfg.resync_on_tab_change = next_state;
                let desc = match next_state {
                    None => "Default".to_string(),
                    Some(true) => "Yes".to_string(),
                    Some(false) => "No".to_string(),
                };
                app.config.repo_configs.insert(repo_path.to_string(), repo_cfg);
                app.persist(&format!("Repository Resync on Tab Change set to {}", desc));
            }
            _ => {}
        }
    }

    pub fn draw(f: &mut Frame, app: &App, area: Rect) {
        let popup_width = 54;
        let popup_height = 16;
        let popup_area = crate::ui::layout::centered_rect_fixed(popup_width, popup_height, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(accent_style())
            .title(Span::styled(" Repository Settings ", accent_style()))
            .padding(Padding::horizontal(1));

        f.render_widget(Clear, popup_area);
        f.render_widget(block.clone(), popup_area);

        let inner = block.inner(popup_area);

        let repo_path = app.get_selected_item().map(|s| s.as_str()).unwrap_or("");
        let repo_name = std::path::Path::new(repo_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(repo_path);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Repository Name
                Constraint::Length(1), // Spacer
                Constraint::Min(6),    // Settings items list
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Shortcuts instructions
            ])
            .split(inner);

        f.render_widget(
            Paragraph::new(vec![Line::from(vec![
                Span::raw("Repo: "),
                Span::styled(repo_name, primary_style()),
            ])])
            .alignment(Alignment::Center),
            chunks[1],
        );

        let repo_cfg = app
            .get_selected_item()
            .and_then(|p| app.config.repo_configs.get(p))
            .cloned()
            .unwrap_or_default();

        let build_line = |idx: usize,
                          label: &str,
                          value: &str,
                          is_editing: bool|
         -> Line<'static> {
            let is_selected = idx == app.repo_settings_selected_index;
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
        let theme_val = repo_cfg.theme.clone().unwrap_or_else(|| "default".to_string());
        let theme_line = build_line(0, "Theme:", &theme_val, false);

        // Row 1: Page Size
        let page_size_val =
            repo_cfg.page_size.map(|v| v.to_string()).unwrap_or_else(|| "default".to_string());
        let page_size_line = build_line(
            1,
            "Page Size:",
            if app.repo_settings_selected_index == 1 && app.repo_settings_editing {
                &app.repo_settings_input
            } else {
                &page_size_val
            },
            app.repo_settings_selected_index == 1 && app.repo_settings_editing,
        );

        // Row 2: Max Commits
        let max_commits_val =
            repo_cfg.max_commits.map(|v| v.to_string()).unwrap_or_else(|| "default".to_string());
        let max_commits_line = build_line(
            2,
            "Max Commits:",
            if app.repo_settings_selected_index == 2 && app.repo_settings_editing {
                &app.repo_settings_input
            } else {
                &max_commits_val
            },
            app.repo_settings_selected_index == 2 && app.repo_settings_editing,
        );

        // Row 3: Resync on Tab Change
        let resync_val = match repo_cfg.resync_on_tab_change {
            None => "default",
            Some(true) => "yes",
            Some(false) => "no",
        };
        let resync_line = build_line(3, "Resync on Tab Change:", resync_val, false);

        // Row 4: Editor Command
        let editor_val = repo_cfg.editor.clone().unwrap_or_else(|| "default".to_string());
        let editor_line = build_line(
            4,
            "Editor Command:",
            if app.repo_settings_selected_index == 4 && app.repo_settings_editing {
                &app.repo_settings_input
            } else {
                &editor_val
            },
            app.repo_settings_selected_index == 4 && app.repo_settings_editing,
        );

        // Row 5: User Note
        let note_val = repo_cfg.note.clone().unwrap_or_else(|| "none".to_string());
        let note_line = build_line(
            5,
            "User Note:",
            if app.repo_settings_selected_index == 5 && app.repo_settings_editing {
                &app.repo_settings_input
            } else {
                &note_val
            },
            app.repo_settings_selected_index == 5 && app.repo_settings_editing,
        );

        let settings_lines =
            vec![theme_line, page_size_line, max_commits_line, resync_line, editor_line, note_line];
        f.render_widget(Paragraph::new(settings_lines), chunks[3]);

        // Shortcuts helper bar
        let helper_line = if app.repo_settings_editing {
            if app.repo_settings_selected_index == 4 || app.repo_settings_selected_index == 5 {
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
                Span::styled(" [Esc/q] ", accent_style()),
                Span::styled("Close", muted_style()),
            ])
        };
        f.render_widget(Paragraph::new(helper_line).alignment(Alignment::Center), chunks[5]);
    }
}
