use crate::app::{App, Mode};
use crate::repo::DiffLineKind;
use crate::ui::style::{ACCENT, DANGER, SUCCESS, muted_style, primary_style};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap};

pub struct FileHistoryTab;

impl FileHistoryTab {
    pub fn handle_event(app: &mut App, key: KeyEvent) -> bool {
        let code = key.code;

        match code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.mode = Mode::Detail;
                return true;
            }
            KeyCode::Tab | KeyCode::Char('w') | KeyCode::Char('W') => {
                if app.file_history_revisions.is_empty() {
                    return true;
                }
                app.file_history_focus = if app.file_history_focus == 0 { 1 } else { 0 };
                return true;
            }
            KeyCode::Left | KeyCode::Right => {
                if !app.file_history_revisions.is_empty() {
                    app.file_history_focus = if app.file_history_focus == 0 { 1 } else { 0 };
                }
                return true;
            }
            _ => {}
        }

        if app.file_history_focus == 0 {
            // Revisions list focused
            match code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    if !app.file_history_revisions.is_empty() && app.file_history_selection > 0 {
                        app.file_history_selection -= 1;
                        app.refresh_file_history_diff();
                    }
                    return true;
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    if !app.file_history_revisions.is_empty()
                        && app.file_history_selection + 1 < app.file_history_revisions.len()
                    {
                        app.file_history_selection += 1;
                        app.refresh_file_history_diff();
                    }
                    return true;
                }
                KeyCode::Home => {
                    if !app.file_history_revisions.is_empty() {
                        app.file_history_selection = 0;
                        app.refresh_file_history_diff();
                    }
                    return true;
                }
                KeyCode::End => {
                    if !app.file_history_revisions.is_empty() {
                        app.file_history_selection =
                            app.file_history_revisions.len().saturating_sub(1);
                        app.refresh_file_history_diff();
                    }
                    return true;
                }
                _ => {}
            }
        } else {
            // Diff panel focused
            match code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    app.file_history_diff_scroll = app.file_history_diff_scroll.saturating_sub(1);
                    return true;
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    let max = app.file_history_diff.len().saturating_sub(1);
                    if app.file_history_diff_scroll < max {
                        app.file_history_diff_scroll += 1;
                    }
                    return true;
                }
                KeyCode::PageUp => {
                    app.file_history_diff_scroll = app.file_history_diff_scroll.saturating_sub(10);
                    return true;
                }
                KeyCode::PageDown => {
                    let max = app.file_history_diff.len().saturating_sub(1);
                    app.file_history_diff_scroll = (app.file_history_diff_scroll + 10).min(max);
                    return true;
                }
                KeyCode::Home => {
                    app.file_history_diff_scroll = 0;
                    return true;
                }
                KeyCode::End => {
                    app.file_history_diff_scroll = app.file_history_diff.len().saturating_sub(1);
                    return true;
                }
                _ => {}
            }
        }

        false
    }

    pub fn draw_file_history(f: &mut Frame, app: &App, area: Rect) {
        let left_focused = app.file_history_focus == 0;
        let right_focused = app.file_history_focus == 1;

        // Split horizontally: left revisions panel (35%), right diff panel (65%)
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Min(0)])
            .split(area);

        // Draw left Revisions block
        let left_border_style =
            if left_focused { Style::default().fg(ACCENT()) } else { muted_style() };

        let left_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(left_border_style)
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Revisions", primary_style()),
                Span::raw(" ("),
                Span::styled(&app.file_history_path, muted_style()),
                Span::raw(") "),
            ]));

        let left_inner = left_block.inner(chunks[0]);
        f.render_widget(left_block, chunks[0]);

        if app.file_history_revisions.is_empty() {
            let v_center = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(45),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(left_inner);
            f.render_widget(
                Paragraph::new(Span::styled("No history found", muted_style()))
                    .alignment(Alignment::Center),
                v_center[1],
            );
        } else {
            let items: Vec<ListItem> = app
                .file_history_revisions
                .iter()
                .enumerate()
                .map(|(i, rev)| {
                    let is_selected = i == app.file_history_selection;
                    let style = if is_selected {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };

                    let oid_span = Span::styled(
                        format!("{:.7}", rev.commit_oid),
                        Style::default().fg(ratatui::style::Color::Cyan),
                    );
                    let when_span = Span::styled(format!(" ({})", rev.when), muted_style());
                    let author_span = Span::styled(format!(" - {}", rev.author), muted_style());

                    let title_line = Line::from(vec![
                        if is_selected {
                            Span::styled(
                                format!("{} ", app.sym("selection_mark")),
                                Style::default().fg(ACCENT()),
                            )
                        } else {
                            Span::raw("  ")
                        },
                        oid_span,
                        when_span,
                        author_span,
                    ]);

                    let summary_line = Line::from(vec![
                        Span::raw("    "),
                        Span::styled(&rev.summary, Style::default()),
                    ]);

                    ListItem::new(vec![title_line, summary_line]).style(style)
                })
                .collect();

            let mut list_state = ratatui::widgets::ListState::default();
            list_state.select(Some(app.file_history_selection));

            let list =
                List::new(items).highlight_style(Style::default().add_modifier(Modifier::BOLD));
            f.render_stateful_widget(list, left_inner, &mut list_state);
        }

        // Draw right Diff block
        let right_border_style =
            if right_focused { Style::default().fg(ACCENT()) } else { muted_style() };

        let right_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(right_border_style)
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Diff", primary_style()),
                if right_focused && app.file_history_diff_scroll > 0 {
                    Span::styled(
                        format!("  ↕ line {}", app.file_history_diff_scroll + 1),
                        muted_style(),
                    )
                } else {
                    Span::raw("")
                },
                if right_focused {
                    Span::styled(format!("  {} scroll", app.sym("up_down")), muted_style())
                } else {
                    Span::raw("")
                },
                Span::raw(" "),
            ]));

        let right_inner = right_block.inner(chunks[1]);
        f.render_widget(right_block, chunks[1]);

        if app.file_history_diff.is_empty() {
            let v_center = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(45),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(right_inner);
            f.render_widget(
                Paragraph::new(Span::styled("No changes or binary file", muted_style()))
                    .alignment(Alignment::Center),
                v_center[1],
            );
        } else {
            let diff_spans: Vec<Line> = app
                .file_history_diff
                .iter()
                .map(|line| {
                    let style = match line.kind {
                        DiffLineKind::Added => Style::default().fg(SUCCESS()),
                        DiffLineKind::Removed => Style::default().fg(DANGER()),
                        DiffLineKind::Header => Style::default().fg(ratatui::style::Color::Cyan),
                        DiffLineKind::Context => Style::default(),
                        DiffLineKind::ConflictOurs => {
                            Style::default().fg(ratatui::style::Color::LightRed)
                        }
                        DiffLineKind::ConflictTheirs => {
                            Style::default().fg(ratatui::style::Color::LightBlue)
                        }
                        DiffLineKind::ConflictSeparator => Style::default()
                            .fg(ratatui::style::Color::Yellow)
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    };
                    Line::from(Span::styled(line.content.clone(), style))
                })
                .collect();

            f.render_widget(
                Paragraph::new(diff_spans)
                    .scroll((app.file_history_diff_scroll as u16, 0))
                    .wrap(Wrap { trim: false }),
                right_inner,
            );
        }
    }
}
