#[derive(Default)]
pub struct StatusListComponent {
    pub queue: crate::queue::Queue,
    pub staged_list_state: std::cell::RefCell<ratatui::widgets::ListState>,
    pub unstaged_list_state: std::cell::RefCell<ratatui::widgets::ListState>,
    pub conflicts_list_state: std::cell::RefCell<ratatui::widgets::ListState>,
    pub changed_files_list_state: std::cell::RefCell<ratatui::widgets::ListState>,
    pub staging_file_selection: usize,
    pub conflict_file_selection: usize,
    pub file_selection: usize,
}

use crate::app::{App, DetailSection, Mode};
use crate::components::diff::draw_file_subpanel;
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

pub fn draw_staging_panels(
    f: &mut Frame,
    changes: &WorktreeChanges,
    focus: DetailSection,
    last_staging_focus: DetailSection,
    staging_file_selection: usize,
    file_diff: &[DiffLine],
    diff_scroll: usize,
    areas: &mut DetailAreas,
    inspect_horizontal_split_pct: u16,
    _inspect_vertical_split_pct: u16,
    app: &crate::app::App,
    area: Rect,
) {
    let right_focused =
        focus == DetailSection::StagingDetails || focus == DetailSection::ConflictDiff;
    let selected_file_name: Option<String> = {
        let (files, idx) = match focus {
            DetailSection::Staged => (Some(&changes.staged), staging_file_selection),
            DetailSection::Unstaged => (Some(&changes.unstaged), staging_file_selection),
            DetailSection::Conflicts => {
                (Some(&changes.conflicted), app.status_list.conflict_file_selection)
            }
            _ => match last_staging_focus {
                DetailSection::Staged => (Some(&changes.staged), staging_file_selection),
                DetailSection::Unstaged => (Some(&changes.unstaged), staging_file_selection),
                DetailSection::Conflicts => {
                    (Some(&changes.conflicted), app.status_list.conflict_file_selection)
                }
                _ => {
                    if !changes.conflicted.is_empty() {
                        (Some(&changes.conflicted), app.status_list.conflict_file_selection)
                    } else if !changes.staged.is_empty() {
                        (Some(&changes.staged), staging_file_selection)
                    } else if !changes.unstaged.is_empty() {
                        (Some(&changes.unstaged), staging_file_selection)
                    } else {
                        (None, 0)
                    }
                }
            },
        };
        files.and_then(|f| f.get(idx)).map(|e| e.path.clone())
    };

    let right_inner = if app.inspect_full_diff {
        areas.bottom_left = None;
        areas.bottom_right = Some(area);
        areas.commit_details = None;
        areas.inspect_horizontal_splitter = None;
        areas.inspect_vertical_splitter = None;
        areas.staged_sub = None;
        areas.unstaged_sub = None;

        let right_block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(Style::default().fg(ACCENT()))
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Staging Details", primary_style()),
                if let Some(ref name) = selected_file_name {
                    Span::styled(format!("  {} (Full Screen)", name), muted_style())
                } else {
                    Span::raw("")
                },
                Span::raw(" "),
            ]));
        let inner = right_block.inner(area);
        f.render_widget(right_block, area);
        inner
    } else {
        let panels = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(inspect_horizontal_split_pct),
                Constraint::Percentage(100 - inspect_horizontal_split_pct),
            ])
            .split(area);

        areas.bottom_left = Some(panels[0]);
        areas.bottom_right = Some(panels[1]);
        areas.commit_details = None;

        // Record horizontal splitter boundary
        let split_col = area.x + panels[0].width;
        areas.inspect_horizontal_splitter =
            Some(Rect::new(split_col.saturating_sub(1), area.y, 2, area.height));

        // Focus-aware border helpers.
        let left_focused = focus == DetailSection::Staged
            || focus == DetailSection::Unstaged
            || focus == DetailSection::Conflicts;

        // ── Left panel: outer border labelled "Staging Area" ──────────────────
        let left_border_style =
            if left_focused { Style::default().fg(ACCENT()) } else { muted_style() };
        let left_outer = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(left_border_style)
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Staging Area", primary_style()),
                Span::raw(" "),
            ]));
        let left_inner = left_outer.inner(panels[0]);
        f.render_widget(left_outer, panels[0]);

        // Split left inner vertically: top = Staged, middle = Unstaged, bottom = Conflicts (if any)
        let has_conflicts = !changes.conflicted.is_empty();
        let left_split = if has_conflicts {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                    Constraint::Ratio(1, 3),
                ])
                .split(left_inner)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(left_inner)
        };

        // Record vertical splitter boundary in left inner
        let split_row = left_inner.y + left_split[0].height;
        areas.inspect_vertical_splitter =
            Some(Rect::new(left_inner.x, split_row.saturating_sub(1), left_inner.width, 2));

        areas.staged_sub = Some(left_split[0]);
        areas.unstaged_sub = Some(left_split[1]);
        if has_conflicts {
            areas.conflicts_sub = Some(left_split[2]);
        } else {
            areas.conflicts_sub = None;
        }

        let staged_inner = draw_file_subpanel(
            f,
            "Staged",
            SUCCESS(),
            &changes.staged,
            "Nothing staged",
            Borders::BOTTOM,
            focus == DetailSection::Staged,
            if focus == DetailSection::Staged { Some(staging_file_selection) } else { None },
            &app.status_list.staged_list_state,
            left_split[0],
        );
        areas.staged_sub_inner = Some(staged_inner);

        let unstaged_inner = draw_file_subpanel(
            f,
            "Unstaged",
            WARNING(),
            &changes.unstaged,
            "No unstaged changes",
            if has_conflicts { Borders::BOTTOM } else { Borders::empty() },
            focus == DetailSection::Unstaged,
            if focus == DetailSection::Unstaged { Some(staging_file_selection) } else { None },
            &app.status_list.unstaged_list_state,
            left_split[1],
        );
        areas.unstaged_sub_inner = Some(unstaged_inner);

        if has_conflicts {
            let conflicts_inner = draw_file_subpanel(
                f,
                "Conflicts",
                DANGER(),
                &changes.conflicted,
                "No conflicts",
                Borders::empty(),
                focus == DetailSection::Conflicts,
                if focus == DetailSection::Conflicts {
                    Some(app.status_list.conflict_file_selection)
                } else {
                    None
                },
                &app.status_list.conflicts_list_state,
                left_split[2],
            );
            areas.conflicts_sub_inner = Some(conflicts_inner);
        } else {
            areas.conflicts_sub_inner = None;
        }

        // ── Right panel – Staging Details ─────────────────────────────────────
        let right_border_style =
            if right_focused { Style::default().fg(ACCENT()) } else { muted_style() };
        let right_block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(right_border_style)
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    if last_staging_focus == DetailSection::Conflicts {
                        "Conflict Markers"
                    } else {
                        "Staging Details"
                    },
                    primary_style(),
                ),
                if let Some(ref name) = selected_file_name {
                    Span::styled(format!("  {}", name), muted_style())
                } else {
                    Span::raw("")
                },
                Span::raw(" "),
            ]));
        let inner = right_block.inner(panels[1]);
        f.render_widget(right_block, panels[1]);
        inner
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
            .map(|(i, line)| {
                let is_selected_hunk = selected_hunk_range.map(|r| r.contains(&i)).unwrap_or(false);
                let (prefix, prefix_style) = if right_focused {
                    if app.diff.diff_line_mode {
                        if i == app.diff.diff_line_selection {
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

impl StatusListComponent {
    pub fn new(queue: crate::queue::Queue) -> Self {
        Self { queue, ..Default::default() }
    }
}

use crate::components::{Component, DrawableComponent, EventState};
use crate::queue::InternalEvent;
use crossterm::event::{Event, KeyCode};

impl DrawableComponent for StatusListComponent {
    fn draw(&self, _f: &mut ratatui::Frame, _rect: ratatui::layout::Rect) -> std::io::Result<()> {
        Ok(())
    }
}

impl Component for StatusListComponent {
    fn event(&mut self, ev: &Event) -> std::io::Result<EventState> {
        if let Event::Key(key) = ev {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.queue.push(InternalEvent::StagingFileUp);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.queue.push(InternalEvent::StagingFileDown);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.queue.push(InternalEvent::StageAllChanges);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('x') => {
                    self.queue.push(InternalEvent::RequestDiscardChanges);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('X') => {
                    self.queue.push(InternalEvent::RequestDiscardAllChanges);
                    return Ok(EventState::Consumed);
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.queue.push(InternalEvent::StartStashCreate);
                    return Ok(EventState::Consumed);
                }
                _ => {}
            }
        }
        Ok(EventState::NotConsumed)
    }
}
