//! Git worktrees status and layout manager list widget.

use crate::app::{App, DetailSection};
use crate::repo;
use crate::repo::RepoInfo;
use crate::ui::style::{ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, muted_style, primary_style};
use crate::ui_detail::{DetailAreas, error_style};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table, Wrap};

pub fn draw_worktrees_view(
    f: &mut Frame,
    info: &RepoInfo,
    focus: DetailSection,
    selection: usize,
    areas: &mut DetailAreas,
    app: &App,
    area: Rect,
) {
    if info.worktrees.is_loading() || info.worktrees.is_not_loaded() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(muted_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Worktrees", primary_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let loading_text = Paragraph::new("⟳ Loading worktrees...")
            .style(muted_style())
            .alignment(ratatui::layout::Alignment::Center);
        let center_area = Rect::new(inner.x, inner.y + inner.height / 2, inner.width, 1);
        f.render_widget(loading_text, center_area);
        return;
    }

    if let repo::TabData::Error(err) = &info.worktrees {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(error_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Worktrees - Error", error_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let error_text = Paragraph::new(format!("Error loading worktrees: {}", err))
            .style(error_style())
            .wrap(Wrap { trim: false });
        f.render_widget(error_text, inner);
        return;
    }

    areas.worktrees = Some(area);

    let focused = focus == DetailSection::Worktrees;
    let border_style = if focused { Style::default().fg(ACCENT()) } else { muted_style() };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Worktrees", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let inner = block.inner(area);
    areas.worktrees_inner = Some(inner);
    f.render_widget(block, area);

    let worktrees = match &info.worktrees {
        repo::TabData::Loaded(wts) => wts,
        _ => return,
    };

    if worktrees.is_empty() {
        let empty_text = Paragraph::new("No git worktrees found. Press 'a' to add a new worktree.")
            .style(muted_style())
            .alignment(ratatui::layout::Alignment::Center);
        let center_area = Rect::new(inner.x, inner.y + inner.height / 2, inner.width, 1);
        f.render_widget(empty_text, center_area);
        return;
    }

    let header_cells = vec![
        Cell::from(""),
        Cell::from("Name"),
        Cell::from("Branch"),
        Cell::from("Lock Status"),
        Cell::from("Path"),
    ];
    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD).fg(ACCENT()))
        .bottom_margin(1);

    let rows: Vec<Row> = worktrees
        .iter()
        .enumerate()
        .map(|(idx, wt)| {
            let is_selected = idx == selection;
            let name = &wt.name;
            let branch = wt.branch.as_deref().unwrap_or("(no branch)");

            // 1. Selection indicator
            let cell_sel = if is_selected {
                Cell::from(Span::styled(
                    app.sym("selection_mark"),
                    if focused {
                        Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)
                    } else {
                        muted_style()
                    },
                ))
            } else {
                Cell::from(Span::raw(" "))
            };

            // 2. Name cell with folder icon
            let folder_icon = if app.config.compatibility_mode { "" } else { "📁 " };
            let cell_name = Cell::from(Line::from(vec![
                Span::styled(folder_icon, muted_style()),
                Span::styled(
                    name.clone(),
                    if is_selected && focused {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
            ]));

            // 3. Branch cell with branch icon
            let branch_icon = app.sym("branch");
            let cell_branch = if wt.branch.is_none() {
                Cell::from(Span::styled("(no branch)", muted_style()))
            } else {
                Cell::from(Line::from(vec![
                    Span::styled(branch_icon, Style::default().fg(SUCCESS())),
                    Span::raw(branch.to_string()),
                ]))
            };

            // 4. Lock Status cell with lock icons
            let cell_lock = if wt.is_locked {
                let reason = wt.lock_reason.as_deref().unwrap_or("locked");
                let lock_icon = if app.config.compatibility_mode { "" } else { "🔒 " };
                Cell::from(Line::from(vec![
                    Span::styled(lock_icon, Style::default().fg(WARNING())),
                    Span::styled(format!("Locked ({})", reason), Style::default().fg(WARNING())),
                ]))
            } else {
                let unlock_icon = if app.config.compatibility_mode { "" } else { "🔓 " };
                Cell::from(Line::from(vec![
                    Span::styled(unlock_icon, Style::default().fg(SUCCESS())),
                    Span::styled("Unlocked", muted_style()),
                ]))
            };

            // 5. Path cell with status check
            let path_exists = wt.path.exists();
            let cell_path = if !path_exists {
                let warning_icon = app.sym("warning");
                Cell::from(Line::from(vec![
                    Span::styled(format!("{} ", warning_icon), Style::default().fg(DANGER())),
                    Span::styled(
                        format!("{} (Missing)", wt.path.display()),
                        Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
                    ),
                ]))
            } else {
                Cell::from(Span::styled(wt.path.display().to_string(), muted_style()))
            };

            // Selected row styles
            let row_style = if is_selected {
                if focused {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().add_modifier(Modifier::DIM)
                }
            } else {
                Style::default()
            };

            Row::new(vec![cell_sel, cell_name, cell_branch, cell_lock, cell_path]).style(row_style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),      // Selection indicator
            Constraint::Percentage(15), // Name
            Constraint::Percentage(20), // Branch
            Constraint::Percentage(23), // Lock Status
            Constraint::Percentage(40), // Path
        ],
    )
    .header(header)
    .column_spacing(2)
    .block(Block::default());

    f.render_widget(table, inner);
}
