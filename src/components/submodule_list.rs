//! Git submodules status and update actions list widget.

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

pub fn draw_submodules_view(
    f: &mut Frame,
    info: &RepoInfo,
    focus: DetailSection,
    selection: usize,
    areas: &mut DetailAreas,
    app: &App,
    area: Rect,
) {
    if info.submodules.is_loading() || info.submodules.is_not_loaded() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(muted_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Submodules", primary_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let loading_text = Paragraph::new("⟳ Loading submodules...")
            .style(muted_style())
            .alignment(ratatui::layout::Alignment::Center);
        let center_area = Rect::new(inner.x, inner.y + inner.height / 2, inner.width, 1);
        f.render_widget(loading_text, center_area);
        return;
    }

    if let repo::TabData::Error(err) = &info.submodules {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(error_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Submodules - Error", error_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let err_text = Paragraph::new(format!("Failed to load submodules: {}", err))
            .style(error_style())
            .wrap(Wrap { trim: true });
        f.render_widget(err_text, inner);
        return;
    }

    areas.submodules = Some(area);

    let focused = focus == DetailSection::Submodules;
    let border_style = if focused { Style::default().fg(ACCENT()) } else { muted_style() };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Submodules", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let inner = block.inner(area);
    areas.submodules_inner = Some(inner);
    f.render_widget(block, area);

    let submodules = match &info.submodules {
        repo::TabData::Loaded(subs) => subs,
        _ => return,
    };

    if submodules.is_empty() {
        let empty_text = Paragraph::new("No git submodules found.")
            .style(muted_style())
            .alignment(ratatui::layout::Alignment::Center);
        let center_area = Rect::new(inner.x, inner.y + inner.height / 2, inner.width, 1);
        f.render_widget(empty_text, center_area);
        return;
    }

    let header_cells = vec![
        Cell::from(""),
        Cell::from("Name"),
        Cell::from("Status"),
        Cell::from("Commit (Index)"),
        Cell::from("Commit (HEAD)"),
        Cell::from("URL"),
    ];
    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD).fg(ACCENT()))
        .bottom_margin(1);

    let rows: Vec<Row> = submodules
        .iter()
        .enumerate()
        .map(|(idx, sub)| {
            let is_selected = idx == selection;

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

            // 2. Name cell
            let submodule_icon = if app.config.compatibility_mode { "" } else { "📦 " };
            let cell_name = Cell::from(Line::from(vec![
                Span::styled(submodule_icon, muted_style()),
                Span::styled(
                    sub.name.clone(),
                    if is_selected && focused {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
            ]));

            // 3. Status cell
            let cell_status = if !sub.is_initialized {
                Cell::from(Span::styled("Uninitialized", Style::default().fg(WARNING())))
            } else if sub.is_dirty {
                Cell::from(Span::styled("Modified", Style::default().fg(DANGER())))
            } else {
                Cell::from(Span::styled("Clean", Style::default().fg(SUCCESS())))
            };

            // 4. Commit (Index) cell
            let cell_commit_idx = match &sub.commit_id {
                Some(sha) => Cell::from(Span::raw(&sha[..8.min(sha.len())])),
                None => Cell::from(Span::styled("-", muted_style())),
            };

            // 5. Commit (HEAD) cell
            let cell_commit_head = match &sub.head_id {
                Some(sha) => {
                    let style = if sub.commit_id.as_ref() != Some(sha) {
                        Style::default().fg(WARNING())
                    } else {
                        Style::default()
                    };
                    Cell::from(Span::styled(&sha[..8.min(sha.len())], style))
                }
                None => Cell::from(Span::styled("unborn", muted_style())),
            };

            // 6. URL cell
            let cell_url = Cell::from(Span::raw(&sub.url));

            let mut row = Row::new(vec![
                cell_sel,
                cell_name,
                cell_status,
                cell_commit_idx,
                cell_commit_head,
                cell_url,
            ]);

            if is_selected {
                if focused {
                    row = row.style(Style::default().add_modifier(Modifier::REVERSED));
                } else {
                    row = row.style(Style::default().add_modifier(Modifier::DIM));
                }
            }

            row
        })
        .collect();

    let widths = [
        Constraint::Length(2),
        Constraint::Percentage(25),
        Constraint::Length(15),
        Constraint::Length(15),
        Constraint::Length(15),
        Constraint::Percentage(30),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().padding(Padding::uniform(0)));

    f.render_widget(table, inner);
}
