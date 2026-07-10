use crate::app::{App, Mode};
use crate::repo::RemoteInfo;
use crate::ui::layout::{centered_rect, centered_rect_fixed};
use crate::ui::style::{
    ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, accent_style, muted_style, parse_color,
    primary_style,
};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState, Padding, Paragraph, Wrap,
};

pub fn draw_debug_logs(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(90, 90, area);

    let footer_spans = if app.debug_log_search_editing {
        vec![
            Span::raw(" "),
            Span::styled("Enter", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" focus list  •  ", muted_style()),
            Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" clear/back ", muted_style()),
        ]
    } else if app.debug_log_search_query.is_some() {
        vec![
            Span::raw(" "),
            Span::styled("/", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled("edit query  •  ", muted_style()),
            Span::styled("c", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled("lear  •  ", muted_style()),
            Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" clear filter/quit ", muted_style()),
        ]
    } else {
        vec![
            Span::raw(" "),
            Span::styled("/", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled("find  •  ", muted_style()),
            Span::styled("c", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled("lear  •  ", muted_style()),
            Span::styled("q", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled("uit ", muted_style()),
        ]
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(accent_style())
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("Debug Logs", primary_style()),
                Span::raw(" "),
            ])
            .alignment(Alignment::Center),
        )
        .title_bottom(Line::from(footer_spans).alignment(Alignment::Right));

    f.render_widget(Clear, popup_area);
    f.render_widget(block.clone(), popup_area);

    let inner_rect = block.inner(popup_area);

    let (list_rect, input_rect) = if app.debug_log_search_query.is_some() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Input box
                Constraint::Min(1),    // Log list
            ])
            .split(inner_rect);
        (chunks[1], Some(chunks[0]))
    } else {
        (inner_rect, None)
    };

    if let Some(rect) = input_rect {
        let query = app.debug_log_search_query.as_deref().unwrap_or("");
        let border_color = if app.debug_log_search_editing { ACCENT() } else { Color::DarkGray };
        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(Style::default().fg(border_color))
            .title(" Fuzzy Search Logs ");
        let input_p = Paragraph::new(Line::from(vec![
            Span::raw("> "),
            Span::styled(query, primary_style()),
        ]))
        .block(input_block);
        f.render_widget(input_p, rect);

        if app.debug_log_search_editing {
            let query_len = query.chars().count();
            let cursor_x = rect.x + 3 + query_len as u16;
            let cursor_y = rect.y + 1;
            f.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
        }
    }

    let logs = crate::debug_log::get_logs();
    let filtered_logs: Vec<String> = if let Some(ref query) = app.debug_log_search_query {
        let q_lower = query.to_lowercase();
        logs.into_iter()
            .filter(|log_line| {
                let log_lower = log_line.to_lowercase();
                if q_lower.is_empty() {
                    true
                } else if log_lower.contains(&q_lower) {
                    true
                } else {
                    let mut log_chars = log_lower.chars();
                    let mut matched = true;
                    for qc in q_lower.chars() {
                        if !log_chars.any(|nc| nc == qc) {
                            matched = false;
                            break;
                        }
                    }
                    matched
                }
            })
            .collect()
    } else {
        logs
    };

    let height = list_rect.height as usize;
    let total_logs = filtered_logs.len();
    let start_idx = app.debug_log_scroll.min(total_logs.saturating_sub(1));
    let end_idx = (start_idx + height).min(total_logs);

    let visible_lines: Vec<Line> = if total_logs > 0 {
        filtered_logs[start_idx..end_idx]
            .iter()
            .map(|log_str| {
                let mut spans = Vec::new();
                if log_str.len() > 21 {
                    let time_part = &log_str[0..10];
                    let level_part = &log_str[10..18];
                    let rest = &log_str[18..];

                    spans.push(Span::styled(time_part, muted_style()));
                    if level_part.contains("ERROR") {
                        spans.push(Span::styled(
                            level_part,
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ));
                    } else if level_part.contains("WARN") {
                        spans.push(Span::styled(
                            level_part,
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                        ));
                    } else if level_part.contains("INFO") {
                        spans.push(Span::styled(level_part, Style::default().fg(Color::Green)));
                    } else {
                        spans.push(Span::styled(level_part, Style::default().fg(Color::Blue)));
                    }
                    spans.push(Span::raw(rest));
                } else {
                    spans.push(Span::raw(log_str));
                }
                Line::from(spans)
            })
            .collect()
    } else {
        Vec::new()
    };

    let paragraph = Paragraph::new(visible_lines)
        .style(Style::default())
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, list_rect);
}

fn get_filtered_logs_count(query_opt: &Option<String>) -> usize {
    let logs = crate::debug_log::get_logs();
    if let Some(query) = query_opt {
        let q_lower = query.to_lowercase();
        logs.into_iter()
            .filter(|l| {
                let l_lower = l.to_lowercase();
                if q_lower.is_empty() {
                    true
                } else if l_lower.contains(&q_lower) {
                    true
                } else {
                    let mut l_chars = l_lower.chars();
                    let mut matched = true;
                    for qc in q_lower.chars() {
                        if !l_chars.any(|nc| nc == qc) {
                            matched = false;
                            break;
                        }
                    }
                    matched
                }
            })
            .count()
    } else {
        logs.len()
    }
}

use crossterm::event::{KeyCode, KeyEvent};
pub struct DebugLogsPopup;
impl DebugLogsPopup {
    pub fn handle_event(app: &mut crate::app::App, key: KeyEvent) -> bool {
        let code = key.code;

        if app.debug_log_search_editing {
            match code {
                KeyCode::Esc => {
                    if let Some(ref mut query) = app.debug_log_search_query {
                        if !query.is_empty() {
                            query.clear();
                            app.debug_log_scroll = 0;
                            return true;
                        }
                    }
                    app.debug_log_search_query = None;
                    app.debug_log_search_editing = false;
                    app.debug_log_scroll = 0;
                }
                KeyCode::Enter => {
                    app.debug_log_search_editing = false;
                }
                KeyCode::Backspace => {
                    if let Some(ref mut query) = app.debug_log_search_query {
                        query.pop();
                        app.debug_log_scroll = 0;
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(ref mut query) = app.debug_log_search_query {
                        query.push(c);
                        app.debug_log_scroll = 0;
                    }
                }
                KeyCode::Down => {
                    let total_logs = get_filtered_logs_count(&app.debug_log_search_query);
                    let max_scroll = total_logs.saturating_sub(1);
                    if app.debug_log_scroll < max_scroll {
                        app.debug_log_scroll += 1;
                    }
                }
                KeyCode::Up if app.debug_log_scroll > 0 => {
                    app.debug_log_scroll -= 1;
                }
                KeyCode::PageUp => {
                    app.debug_log_scroll = app.debug_log_scroll.saturating_sub(app.config.page_size);
                }
                KeyCode::PageDown => {
                    let total_logs = get_filtered_logs_count(&app.debug_log_search_query);
                    let max_scroll = total_logs.saturating_sub(1);
                    app.debug_log_scroll =
                        (app.debug_log_scroll + app.config.page_size).min(max_scroll);
                }
                KeyCode::Home => {
                    app.debug_log_scroll = 0;
                }
                KeyCode::End => {
                    let total_logs = get_filtered_logs_count(&app.debug_log_search_query);
                    app.debug_log_scroll = total_logs.saturating_sub(1);
                }
                _ => {}
            }
            return true;
        }

        match code {
            KeyCode::Esc if app.debug_log_search_query.is_some() => {
                app.debug_log_search_query = None;
                app.debug_log_scroll = 0;
            }
            KeyCode::Esc
            | KeyCode::Char('q')
            | KeyCode::Char('D')
            | KeyCode::Char('l')
            | KeyCode::Char('L') => {
                crate::debug_log::info("Exiting debug logs");
                app.debug_log_search_query = None;
                app.debug_log_search_editing = false;
                app.mode = Mode::Normal;
            }
            KeyCode::Char('/') => {
                if app.debug_log_search_query.is_none() {
                    app.debug_log_search_query = Some(String::new());
                }
                app.debug_log_search_editing = true;
                app.debug_log_scroll = 0;
            }
            KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Char('x') => {
                crate::debug_log::clear();
                app.debug_log_scroll = 0;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let total_logs = get_filtered_logs_count(&app.debug_log_search_query);
                let max_scroll = total_logs.saturating_sub(1);
                if app.debug_log_scroll < max_scroll {
                    app.debug_log_scroll += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') if app.debug_log_scroll > 0 => {
                app.debug_log_scroll -= 1;
            }
            KeyCode::PageUp => {
                app.debug_log_scroll = app.debug_log_scroll.saturating_sub(app.config.page_size);
            }
            KeyCode::PageDown => {
                let total_logs = get_filtered_logs_count(&app.debug_log_search_query);
                let max_scroll = total_logs.saturating_sub(1);
                app.debug_log_scroll =
                    (app.debug_log_scroll + app.config.page_size).min(max_scroll);
            }
            KeyCode::Home => {
                app.debug_log_scroll = 0;
            }
            KeyCode::End => {
                let total_logs = get_filtered_logs_count(&app.debug_log_search_query);
                app.debug_log_scroll = total_logs.saturating_sub(1);
            }
            _ => {}
        }
        true
    }
}
