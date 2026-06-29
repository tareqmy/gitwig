use crate::app::{App, Mode};
use crate::ui::style::{accent_style, CARD_BORDER, muted_style, primary_style};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::text::{Line, Span};
use ratatui::Frame;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Style, Modifier};

pub struct RepoThemePopup;

impl RepoThemePopup {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        let code = key.code;
        match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.mode = Mode::Detail;
                return true;
            }
            KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                if !app.settings_theme_list.is_empty() {
                    app.settings_theme_index = app.settings_theme_index.saturating_sub(1);
                }
                return true;
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                if !app.settings_theme_list.is_empty() {
                    app.settings_theme_index = (app.settings_theme_index + 1)
                        .min(app.settings_theme_list.len().saturating_sub(1));
                }
                return true;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                if app.settings_theme_index < app.settings_theme_list.len() {
                    let theme_name = app.settings_theme_list[app.settings_theme_index].clone();
                    if let Some(repo_path) = app.get_selected_item().cloned() {
                        let mut repo_cfg = app.config.repo_configs.get(&repo_path).cloned().unwrap_or_default();
                        if theme_name == "default" {
                            repo_cfg.theme = None;
                        } else {
                            repo_cfg.theme = Some(theme_name.clone());
                        }
                        app.config.repo_configs.insert(repo_path, repo_cfg);
                        app.persist(&format!("Theme set to '{}'", theme_name));
                    }
                }
                app.mode = Mode::Detail;
                return true;
            }
            _ => {}
        }
        false
    }

    pub fn draw(f: &mut Frame, app: &App, area: Rect) {
        let popup_width = 46;
        let list_len = app.settings_theme_list.len();
        let popup_height = (8 + list_len) as u16;

        let popup_area = crate::ui::layout::centered_rect_fixed(popup_width, popup_height, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(accent_style())
            .title(Span::styled(" Set Repository Theme ", accent_style()));

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
                Constraint::Min(3),    // Dropdown list
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Shortcuts instructions
            ])
            .split(inner);

        f.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::raw("Repo: "),
                    Span::styled(repo_name, primary_style()),
                ]),
            ])
            .alignment(Alignment::Center),
            chunks[1],
        );

        // Render the themes list dropdown inside chunks[3]
        let mut theme_spans = Vec::new();
        let is_compat = app.config.compatibility_mode;
        for (idx, theme_name) in app.settings_theme_list.iter().enumerate() {
            let is_selected = idx == app.settings_theme_index;
            let prefix = if is_selected { if is_compat { "> " } else { "▶ " } } else { "  " };
            let style = if is_selected {
                accent_style()
            } else {
                Style::default()
            };
            
            // Check if this is the currently configured theme for the repo
            let configured_theme = app.get_selected_item().and_then(|path| {
                app.config.repo_configs.get(path).and_then(|rc| rc.theme.as_ref())
            });
            
            let suffix = match configured_theme {
                Some(ct) if ct == theme_name => " (active)",
                None if theme_name == "default" => " (active)",
                _ => "",
            };

            let line_style = if is_selected {
                Style::default().bg(ratatui::style::Color::Rgb(60, 60, 60))
            } else {
                Style::default()
            };

            theme_spans.push(Line::from(vec![
                Span::styled(format!("{}{}", prefix, theme_name), style),
                Span::styled(suffix, muted_style()),
            ]).style(line_style));
        }

        let dropdown_block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(muted_style())
            .title(Span::styled(" Themes ", muted_style()));
        
        let dropdown_inner = dropdown_block.inner(chunks[3]);
        f.render_widget(dropdown_block, chunks[3]);

        f.render_widget(
            Paragraph::new(theme_spans),
            dropdown_inner,
        );

        // Shortcuts helper bar
        let helper_line = Line::from(vec![
            Span::styled(" [↑/↓/j/k] ", accent_style()),
            Span::styled("Move  ", muted_style()),
            Span::styled(" [Enter] ", accent_style()),
            Span::styled("Apply  ", muted_style()),
            Span::styled(" [Esc/q] ", accent_style()),
            Span::styled("Cancel", muted_style()),
        ]);
        f.render_widget(
            Paragraph::new(helper_line).alignment(Alignment::Center),
            chunks[5],
        );
    }
}
