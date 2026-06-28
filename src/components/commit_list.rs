#[derive(Default)]
pub struct CommitListComponent {
    pub queue: crate::queue::Queue,
    pub selection: usize,
    pub search_query: Option<String>,
    pub table_state: std::cell::RefCell<ratatui::widgets::TableState>,
    pub limit: usize,
    pub details_scroll: usize,
}

use crate::app::{App, DetailSection, Mode};
use crate::repo::FileEntry;
use crate::repo::RemoteInfo;
use crate::repo::{CommitEntry, DiffLine, RepoInfo, WorktreeChanges};
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
    Row, Table, TableState, Wrap,
};

pub fn draw_detail_commits(
    f: &mut Frame,
    info: &RepoInfo,
    focus: DetailSection,
    commit_selection: usize,
    commit_search_query: &Option<String>,
    area: Rect,
    commits_table_state: &std::cell::RefCell<TableState>,
    areas: &mut DetailAreas,
    commit_limit: usize,
) {
    let focused = focus == DetailSection::Commits;
    let border_style = if focused { Style::default().fg(ACCENT()) } else { muted_style() };

    // Filter commits based on search query
    let filtered_commits: Vec<&crate::repo::CommitEntry> = if let Some(query) = commit_search_query
    {
        let q = query.to_lowercase();
        info.commits
            .iter()
            .filter(|c| {
                c.id.to_lowercase().contains(&q)
                    || c.author.to_lowercase().contains(&q)
                    || c.when.to_lowercase().contains(&q)
                    || c.summary.to_lowercase().contains(&q)
            })
            .collect()
    } else {
        info.commits.iter().collect()
    };

    // Dirty = any uncommitted change in staged / unstaged / untracked / conflicted.
    let dirty = !info.changes.staged.is_empty()
        || !info.changes.unstaged.is_empty()
        || !info.changes.untracked.is_empty()
        || !info.changes.conflicted.is_empty();

    let show_dirty = if dirty {
        if let Some(query) = commit_search_query {
            "<uncommitted>".contains(&query.to_lowercase())
        } else {
            true
        }
    } else {
        false
    };

    let total_entries = filtered_commits.len() + if show_dirty { 1 } else { 0 };
    let selected_num =
        if total_entries > 0 { (commit_selection + 1).min(total_entries) } else { 0 };

    let count_text = if total_entries > 0 {
        format!("({}/{})", selected_num, total_entries)
    } else {
        "(0/0)".to_string()
    };

    let title_spans = if let Some(q) = commit_search_query {
        vec![
            Span::raw(" "),
            Span::styled("Commits (Filter: ", primary_style()),
            Span::styled(q.clone(), accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(")", primary_style()),
            Span::raw("  "),
            Span::styled(count_text, muted_style()),
            Span::raw(" "),
        ]
    } else {
        vec![
            Span::raw(" "),
            Span::styled("Commits", primary_style()),
            Span::raw("  "),
            Span::styled(count_text, muted_style()),
            Span::raw(" "),
        ]
    };

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(title_spans));

    if info.commits.len() >= commit_limit {
        let footer_text =
            format!(" Showing latest {} commits — press G to load more ", info.commits.len());
        block = block.title_bottom(
            Line::from(vec![Span::styled(footer_text, muted_style())])
                .alignment(ratatui::layout::Alignment::Center),
        );
    }

    // Show empty placeholder only when truly empty (no commits AND clean).
    if filtered_commits.is_empty() && !show_dirty {
        let inner = block.inner(area);
        areas.commits_inner = Some(inner);
        f.render_widget(block, area);
        let v = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(40), Constraint::Length(1), Constraint::Min(0)])
            .split(inner);
        f.render_widget(
            Paragraph::new(Span::styled("No commits yet / empty repository", muted_style()))
                .alignment(Alignment::Center),
            v[1],
        );
        return;
    }

    let header = Row::new(vec![
        Cell::from("ID"),
        Cell::from("Author"),
        Cell::from("Date"),
        Cell::from("Summary"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD).fg(ACCENT()));

    // Prepend a virtual "uncommitted changes" row when the worktree is dirty.
    let mut rows: Vec<Row> = Vec::new();
    if show_dirty {
        rows.push(Row::new(vec![
            Cell::from(Span::styled("-", muted_style())),
            Cell::from(Span::styled("-", muted_style())),
            Cell::from(Span::styled("-", muted_style())),
            Cell::from(Span::styled("<uncommitted>", Style::default().fg(WARNING()))),
        ]));
    }
    rows.extend(filtered_commits.iter().map(|commit| {
        // Build the summary cell: optional ref badges then the commit message.
        let mut spans: Vec<Span<'static>> = Vec::new();
        for r in &commit.refs {
            let (label, style) = if let Some(tag) = r.strip_prefix("tag:") {
                // Tag — yellow
                (format!("[{}]", tag), Style::default().fg(WARNING()).add_modifier(Modifier::BOLD))
            } else if let Some(remote) = r.strip_prefix("remote:") {
                // Remote tracking branch — green
                (
                    format!("[{}]", remote),
                    Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
                )
            } else {
                // Local branch — cyan
                (format!("[{}]", r), Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD))
            };
            spans.push(Span::styled(label, style));
            spans.push(Span::raw(" "));
        }
        spans.push(Span::styled(commit.summary.clone(), Style::default()));

        let id_span = if !commit.signature_status.is_empty() && commit.signature_status != "N" {
            let (sig_char, sig_style) = match commit.signature_status.as_str() {
                "G" => ("✓", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)),
                "B" => ("✗", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
                "U" | "X" | "Y" | "R" => ("✓", Style::default().fg(WARNING())),
                _ => ("?", muted_style()),
            };
            Line::from(vec![
                Span::styled(sig_char, sig_style),
                Span::raw(" "),
                Span::styled(commit.id.clone(), Style::default().fg(WARNING())),
            ])
        } else {
            Line::from(vec![Span::styled(commit.id.clone(), Style::default().fg(WARNING()))])
        };

        Row::new(vec![
            Cell::from(id_span),
            Cell::from(Span::styled(commit.author.clone(), Style::default())),
            Cell::from(Span::styled(commit.when.clone(), muted_style())),
            Cell::from(Line::from(spans)),
        ])
    }));

    let widths = [
        Constraint::Length(11), // signature icon + "c7a45e2" + 2 padding
        Constraint::Length(18), // Author name
        Constraint::Length(16), // Date
        Constraint::Min(20),    // Summary
    ];

    let inner = block.inner(area);
    areas.commits_inner = Some(inner);

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .column_spacing(2);

    let mut state = commits_table_state.borrow_mut();
    if focused {
        state.select(Some(commit_selection));
    } else {
        state.select(None);
    }
    f.render_stateful_widget(table, area, &mut *state);
}

pub fn draw_commit_details_widget(
    f: &mut Frame,
    commit: &CommitEntry,
    focused: bool,
    scroll: usize,
    area: Rect,
) {
    let border_style = if focused { Style::default().fg(ACCENT()) } else { muted_style() };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Commit Info", primary_style()),
            if focused && scroll > 0 {
                Span::styled(format!("  ↕ line {}", scroll + 1), muted_style())
            } else {
                Span::raw("")
            },
            if focused { Span::styled("  ↑↓ scroll", muted_style()) } else { Span::raw("") },
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines = Vec::new();

    // Hash
    lines.push(Line::from(vec![Span::styled("Hash:   ", primary_style()), Span::raw(&commit.oid)]));

    // Author
    lines.push(Line::from(vec![
        Span::styled("Author: ", primary_style()),
        Span::raw(&commit.author),
    ]));

    // Date
    lines.push(Line::from(vec![
        Span::styled("Date:   ", primary_style()),
        Span::raw(format!("{} ({})", commit.date, commit.when)),
    ]));

    // Refs
    if !commit.refs.is_empty() {
        let mut ref_spans = vec![Span::styled("Refs:   ", primary_style())];
        for (idx, r) in commit.refs.iter().enumerate() {
            if idx > 0 {
                ref_spans.push(Span::raw(", "));
            }
            let style = if r.starts_with("tag:") {
                Style::default().fg(ratatui::style::Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)
            };
            ref_spans.push(Span::styled(r.clone(), style));
        }
        lines.push(Line::from(ref_spans));
    }

    // Empty separator
    lines.push(Line::from(""));

    // Message
    lines.push(Line::from(vec![Span::styled("Message:", primary_style())]));
    for line in commit.message.lines() {
        lines.push(Line::from(vec![Span::raw(line.to_string())]));
    }

    let paragraph = Paragraph::new(lines).scroll((scroll as u16, 0)).wrap(Wrap { trim: true });
    f.render_widget(paragraph, inner);
}

pub fn draw_logs_view(
    f: &mut Frame,
    info: &RepoInfo,
    commit_selection: usize,
    _commit_search_query: &Option<String>,
    app: &crate::app::App,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(Style::default().fg(ACCENT()))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Git Commits Logs", primary_style()),
            Span::raw(" "),
        ]));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if info.commits.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled("No commits yet", muted_style()))
                .alignment(Alignment::Center),
            inner,
        );
        return;
    }

    let header = Row::new(vec![
        Cell::from("ID"),
        Cell::from("Author"),
        Cell::from("Date"),
        Cell::from("Summary"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD).fg(ACCENT()));

    let mut rows: Vec<Row> = Vec::new();
    for (i, commit) in info.commits.iter().enumerate() {
        let is_selected = i == commit_selection;
        let is_match = app.commit_matches_query(commit);

        let mut spans: Vec<Span<'static>> = Vec::new();
        if is_match {
            spans.push(Span::styled(
                format!("{} ", app.sym("star")),
                Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
            ));
        }

        for r in &commit.refs {
            let (label, style) = if let Some(tag) = r.strip_prefix("tag:") {
                (format!("[{}]", tag), Style::default().fg(WARNING()).add_modifier(Modifier::BOLD))
            } else if let Some(remote) = r.strip_prefix("remote:") {
                (
                    format!("[{}]", remote),
                    Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
                )
            } else {
                (format!("[{}]", r), Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD))
            };
            spans.push(Span::styled(label, style));
            spans.push(Span::raw(" "));
        }

        let summary_style = if is_match {
            Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        spans.push(Span::styled(commit.summary.clone(), summary_style));

        let mut row_style = Style::default();
        if is_selected {
            row_style = row_style.bg(ratatui::style::Color::Rgb(60, 60, 60));
        }

        rows.push(
            Row::new(vec![
                Cell::from(Span::styled(
                    commit.id.clone(),
                    if is_match {
                        Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(WARNING())
                    },
                )),
                Cell::from(Span::styled(
                    commit.author.clone(),
                    if is_match {
                        Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                )),
                Cell::from(Span::styled(
                    commit.when.clone(),
                    if is_match {
                        Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)
                    } else {
                        muted_style()
                    },
                )),
                Cell::from(Line::from(spans)),
            ])
            .style(row_style),
        );
    }

    let widths = [
        Constraint::Length(9),
        Constraint::Length(18),
        Constraint::Length(16),
        Constraint::Min(20),
    ];

    let mut state = TableState::default();
    state.select(Some(commit_selection));

    let table = Table::new(rows, widths).header(header).column_spacing(2);

    f.render_stateful_widget(table, inner, &mut state);
}

impl CommitListComponent {
    pub fn details_scroll_up(&mut self) {
        self.details_scroll = self.details_scroll.saturating_sub(1);
    }
    pub fn details_scroll_down(&mut self) {
        self.details_scroll = self.details_scroll.saturating_add(1);
    }
}

impl CommitListComponent {
    pub fn new(queue: crate::queue::Queue) -> Self {
        Self { queue, ..Default::default() }
    }
}

use crate::components::{Component, EventState};
use crate::queue::InternalEvent;
use crossterm::event::{Event, KeyCode, KeyEvent};

impl Component for CommitListComponent {
    fn event(&mut self, ev: &Event) -> std::io::Result<EventState> {
        if let Event::Key(key) = ev {
            match key.code {
                KeyCode::Char('f') => {
                    self.queue.push(InternalEvent::SearchColumnPicker);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('c') => {
                    self.queue.push(InternalEvent::StartCommit);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('C') => {
                    self.queue.push(InternalEvent::StartCommitAmend);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    self.queue.push(InternalEvent::StartTagCreate);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('b') | KeyCode::Char('B') => {
                    self.queue.push(InternalEvent::StartBranchCreate);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    self.queue.push(InternalEvent::RunInteractiveRebase);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    self.queue.push(InternalEvent::RequestCherryPick);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.queue.push(InternalEvent::YankSelectedCommitHash);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('v') | KeyCode::Char('V') => {
                    self.queue.push(InternalEvent::RequestRevert);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Right | KeyCode::Enter => {
                    self.queue.push(InternalEvent::InspectCommit);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Home => {
                    self.queue.push(InternalEvent::CommitSelectionTop);
                    return Ok(EventState::Consumed);
                }
                KeyCode::End => {
                    self.queue.push(InternalEvent::CommitSelectionBottom);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('G') => {
                    self.queue.push(InternalEvent::LoadMoreCommits);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.queue.push(InternalEvent::CommitSelectionUp);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.queue.push(InternalEvent::CommitSelectionDown);
                    return Ok(EventState::Consumed);
                }
                KeyCode::PageUp => {
                    self.queue.push(InternalEvent::CommitSelectionPageUp);
                    return Ok(EventState::Consumed);
                }
                KeyCode::PageDown => {
                    self.queue.push(InternalEvent::CommitSelectionPageDown);
                    return Ok(EventState::Consumed);
                }
                _ => {}
            }
        }
        Ok(EventState::NotConsumed)
    }
}

impl crate::components::DrawableComponent for CommitListComponent {
    fn draw(&self, _f: &mut ratatui::Frame, _rect: ratatui::layout::Rect) -> std::io::Result<()> {
        // Drawing logic is currently in ui.rs / ui_detail.rs
        // Will be moved here soon.
        Ok(())
    }
}
