#[derive(Default)]
pub struct DiffComponent {
    pub queue: crate::queue::Queue,
    pub file_diff: Vec<crate::repo::DiffLine>,
    pub diff_scroll: usize,
    pub diff_hunk_selection: usize,
    pub diff_line_mode: bool,
    pub diff_line_selection: usize,
}

use crate::app::{App, DetailSection, Mode};
use crate::components::commit_list::draw_commit_details_widget;
use crate::repo::FileEntry;
use crate::repo::{CommitEntry, DiffLine, RepoInfo, WorktreeChanges};
use crate::repo::{DiffLineKind, RemoteInfo};
use crate::ui::layout::{centered_rect, centered_rect_fixed};
use crate::ui::style::{
    ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, accent_style, muted_style, parse_color,
    primary_style,
};
use crate::ui_detail::{DetailAreas, error_style, file_entry_line, read_file_content};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Borders, Cell, Clear, Gauge, List, ListItem, ListState, Padding, Paragraph,
    Row, Table, Wrap,
};

pub fn draw_file_subpanel(
    f: &mut Frame,
    title: &'static str,
    title_color: ratatui::style::Color,
    files: &[FileEntry],
    empty_msg: &'static str,
    borders: Borders,
    focused: bool,
    selection: Option<usize>,
    list_state: &std::cell::RefCell<ListState>,
    lfs_files: &std::collections::HashSet<String>,
    area: Rect,
) -> Rect {
    // When focused, highlight the title in accent; border stays muted (contained inside outer).
    let title_style = if focused {
        Style::default().fg(ACCENT()).add_modifier(ratatui::style::Modifier::BOLD)
    } else {
        Style::default().fg(title_color)
    };
    let border_style = if focused { Style::default().fg(ACCENT()) } else { muted_style() };
    // Sub-panel block — bottom border separates Staged from Unstaged.
    let block =
        Block::default().borders(borders).border_style(border_style).title(Line::from(vec![
            Span::raw(" "),
            Span::styled(title, title_style),
            Span::raw("  "),
            Span::styled(format!("({})", files.len()), muted_style()),
            Span::raw(" "),
        ]));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if files.is_empty() {
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(40), Constraint::Length(1), Constraint::Min(0)])
            .split(inner);
        f.render_widget(
            Paragraph::new(Span::styled(empty_msg, muted_style())).alignment(Alignment::Center),
            v[1],
        );
        return inner;
    }

    if let Some(sel_idx) = selection {
        // Focused: render as a selectable list with highlight.
        let items: Vec<ListItem> =
            files.iter().map(|e| ListItem::new(file_entry_line(e, lfs_files.contains(&e.path)))).collect();
        let list =
            List::new(items).highlight_style(Style::default().add_modifier(Modifier::REVERSED));
        let mut state = list_state.borrow_mut();
        state.select(Some(sel_idx));
        f.render_stateful_widget(list, inner, &mut *state);
    } else {
        // Not focused: plain paragraph.
        let file_lines: Vec<Line<'static>> = files.iter().map(|e| file_entry_line(e, lfs_files.contains(&e.path))).collect();
        f.render_widget(Paragraph::new(file_lines).wrap(Wrap { trim: false }), inner);
    }
    inner
}

pub fn draw_inspect_window(
    f: &mut Frame,
    commit: &CommitEntry,
    focus: DetailSection,
    file_selection: usize,
    file_diff: &[DiffLine],
    diff_scroll: usize,
    commit_details_scroll: usize,
    areas: &mut DetailAreas,
    inspect_horizontal_split_pct: u16,
    inspect_vertical_split_pct: u16,
    app: &crate::app::App,
    area: Rect,
) {
    let right_focused = focus == DetailSection::StagingDetails;

    let right_inner = if app.inspect_full_diff {
        areas.bottom_left = None;
        areas.bottom_right = Some(area);
        areas.commit_details = None;
        areas.inspect_horizontal_splitter = None;
        areas.inspect_vertical_splitter = None;

        let right_block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(Style::default().fg(ACCENT()))
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Diff", primary_style()),
                if right_focused && diff_scroll > 0 {
                    Span::styled(format!("  ↕ line {}", diff_scroll + 1), muted_style())
                } else {
                    Span::raw("")
                },
                if right_focused {
                    Span::styled(
                        format!("  {} scroll  (Full Screen)", app.sym("up_down")),
                        muted_style(),
                    )
                } else {
                    Span::raw("")
                },
                Span::raw(" "),
            ]));
        let right_inner = right_block.inner(area);
        f.render_widget(right_block, area);
        right_inner
    } else {
        // Body: divided vertically: Left panel, Right panel
        let panels = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(inspect_horizontal_split_pct),
                Constraint::Percentage(100 - inspect_horizontal_split_pct),
            ])
            .split(area);

        // Record horizontal splitter boundary
        let split_col = area.x + panels[0].width;
        areas.inspect_horizontal_splitter =
            Some(Rect::new(split_col.saturating_sub(1), area.y, 2, area.height));

        // Split left panel vertically: top is Commit Details, bottom is Changed Files
        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(inspect_vertical_split_pct),
                Constraint::Percentage(100 - inspect_vertical_split_pct),
            ])
            .split(panels[0]);

        // Record vertical splitter boundary in left panel
        let split_row = panels[0].y + left_chunks[0].height;
        areas.inspect_vertical_splitter =
            Some(Rect::new(panels[0].x, split_row.saturating_sub(1), panels[0].width, 2));

        // Record panel areas for mouse hit testing/scrolling
        areas.commit_details = Some(left_chunks[0]);
        areas.bottom_left = Some(left_chunks[1]);
        areas.bottom_right = Some(panels[1]);

        let details_focused = focus == DetailSection::CommitDetails;
        let left_focused = focus == DetailSection::Staged;

        // ── Left Top: Commit Info (Commit Details) ─────────────────────────
        draw_commit_details_widget(
            f,
            commit,
            details_focused,
            commit_details_scroll,
            left_chunks[0],
        );

        // ── Left Bottom: Changed Files ─────────────────────────────────────
        let left_border_style =
            if left_focused { Style::default().fg(ACCENT()) } else { muted_style() };
        let left_block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(left_border_style)
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Changed Files", primary_style()),
                Span::raw("  "),
                Span::styled(format!("({})", commit.files.len()), muted_style()),
                Span::raw(" "),
            ]));
        let left_inner = left_block.inner(left_chunks[1]);
        areas.changed_files_inner = Some(left_inner);
        f.render_widget(left_block, left_chunks[1]);

        if commit.files.is_empty() {
            let v = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Length(1),
                    Constraint::Min(0),
                ])
                .split(left_inner);
            f.render_widget(
                Paragraph::new(Span::styled("No files changed", muted_style()))
                    .alignment(Alignment::Center),
                v[1],
            );
        } else {
            let empty_set = std::collections::HashSet::new();
            let lfs_files = if let Some(crate::repo::ItemDetail::Repo { info, .. }) = &app.current_detail {
                &info.lfs_files
            } else {
                &empty_set
            };
            let items: Vec<ListItem> =
                commit.files.iter().map(|f| ListItem::new(file_entry_line(f, lfs_files.contains(&f.path)))).collect();
            let list =
                List::new(items).highlight_style(Style::default().add_modifier(Modifier::REVERSED));
            let mut state = app.status_list.changed_files_list_state.borrow_mut();
            if left_focused {
                state.select(Some(file_selection));
            } else {
                state.select(None);
            }
            f.render_stateful_widget(list, left_inner, &mut state);
        }

        // ── Right: Diff ───────────────────────────────────────────────────
        let right_border_style =
            if right_focused { Style::default().fg(ACCENT()) } else { muted_style() };
        let right_block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(right_border_style)
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Diff", primary_style()),
                if right_focused && diff_scroll > 0 {
                    Span::styled(format!("  ↕ line {}", diff_scroll + 1), muted_style())
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
        let right_inner = right_block.inner(panels[1]);
        f.render_widget(right_block, panels[1]);
        right_inner
    };

    if file_diff.is_empty() {
        let v_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(45), Constraint::Length(1), Constraint::Min(0)])
            .split(right_inner);
        f.render_widget(
            Paragraph::new(Span::styled("Select a file to view its diff", muted_style()))
                .alignment(Alignment::Center),
            v_center[1],
        );
    } else {
        let hunk_ranges = app.get_diff_hunk_ranges();
        let selected_hunk_range = hunk_ranges.get(app.diff.diff_hunk_selection);

        let diff_spans: Vec<Line> = file_diff
            .iter()
            .enumerate()
            .map(|(idx, line)| {
                let is_selected_hunk =
                    selected_hunk_range.map(|r| r.contains(&idx)).unwrap_or(false);
                let (prefix, prefix_style) = if right_focused {
                    if app.diff.diff_line_mode {
                        if idx == app.diff.diff_line_selection {
                            ("▎", Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD))
                        } else if is_selected_hunk {
                            ("▏", Style::default().fg(Color::DarkGray))
                        } else {
                            (" ", Style::default())
                        }
                    } else {
                        if is_selected_hunk {
                            ("▎", Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD))
                        } else {
                            (" ", Style::default())
                        }
                    }
                } else {
                    (" ", Style::default())
                };

                let content_style = match line.kind {
                    DiffLineKind::Added => Style::default().fg(SUCCESS()),
                    DiffLineKind::Removed => Style::default().fg(DANGER()),
                    DiffLineKind::Header => Style::default().fg(ACCENT()),
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

                Line::from(vec![
                    Span::styled(prefix, prefix_style),
                    Span::styled(line.content.clone(), content_style),
                ])
            })
            .collect();
        f.render_widget(
            Paragraph::new(diff_spans).scroll((diff_scroll as u16, 0)).wrap(Wrap { trim: false }),
            right_inner,
        );
    }
}

impl DiffComponent {
    pub fn diff_scroll_up(&mut self) {
        self.diff_scroll = self.diff_scroll.saturating_sub(1);
    }
    /// Scroll the diff panel down by one line, clamped so the last line stays visible.
    pub fn diff_scroll_down(&mut self) {
        let max = self.file_diff.len().saturating_sub(1);
        if self.diff_scroll < max {
            self.diff_scroll += 1;
        }
    }
    pub fn diff_scroll_page_up(&mut self, page: usize) {
        self.diff_scroll = self.diff_scroll.saturating_sub(page);
    }
    /// Scroll the diff panel down by  lines.
    pub fn diff_scroll_page_down(&mut self, page: usize) {
        let max = self.file_diff.len().saturating_sub(1);
        self.diff_scroll = (self.diff_scroll + page).min(max);
    }
    pub fn diff_scroll_to_top(&mut self) {
        self.diff_scroll = 0;
    }
    pub fn diff_scroll_to_bottom(&mut self) {
        let max = self.file_diff.len().saturating_sub(1);
        self.diff_scroll = max;
    }
}

impl DiffComponent {
    pub fn new(queue: crate::queue::Queue) -> Self {
        Self { queue, ..Default::default() }
    }
}

use crate::components::{Component, DrawableComponent, EventState};
use crate::queue::InternalEvent;
use crossterm::event::{Event, KeyCode};

impl DrawableComponent for DiffComponent {
    fn draw(&self, _f: &mut ratatui::Frame, _rect: ratatui::layout::Rect) -> std::io::Result<()> {
        Ok(())
    }
}

impl Component for DiffComponent {
    fn event(&mut self, ev: &Event) -> std::io::Result<EventState> {
        if let Event::Key(key) = ev {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.queue.push(InternalEvent::DiffScrollUp);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.queue.push(InternalEvent::DiffScrollDown);
                    return Ok(EventState::Consumed);
                }
                KeyCode::PageUp => {
                    self.queue.push(InternalEvent::DiffScrollPageUp);
                    return Ok(EventState::Consumed);
                }
                KeyCode::PageDown => {
                    self.queue.push(InternalEvent::DiffScrollPageDown);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Home => {
                    self.queue.push(InternalEvent::DiffScrollTop);
                    return Ok(EventState::Consumed);
                }
                KeyCode::End => {
                    self.queue.push(InternalEvent::DiffScrollBottom);
                    return Ok(EventState::Consumed);
                }
                _ => {}
            }
        }
        Ok(EventState::NotConsumed)
    }
}
