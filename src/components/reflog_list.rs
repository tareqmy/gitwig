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

pub fn draw_reflog_view(
    f: &mut Frame,
    info: &RepoInfo,
    focus: DetailSection,
    selection: usize,
    areas: &mut DetailAreas,
    app: &App,
    area: Rect,
) {
    if info.reflog.is_loading() || info.reflog.is_not_loaded() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(muted_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Reflog", primary_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let loading_text = Paragraph::new("⟳ Loading reflog...")
            .style(muted_style())
            .alignment(ratatui::layout::Alignment::Center);
        let center_area = Rect::new(inner.x, inner.y + inner.height / 2, inner.width, 1);
        f.render_widget(loading_text, center_area);
        return;
    }

    if let repo::TabData::Error(err) = &info.reflog {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(error_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Reflog - Error", error_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let error_text = Paragraph::new(format!("Error loading reflog: {}", err))
            .style(error_style())
            .wrap(Wrap { trim: false });
        f.render_widget(error_text, inner);
        return;
    }

    areas.reflog = Some(area);

    let focused = focus == DetailSection::Reflog;
    let border_style = if focused { Style::default().fg(ACCENT()) } else { muted_style() };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Reflog", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let inner = block.inner(area);
    areas.reflog_inner = Some(inner);
    f.render_widget(block, area);

    let entries = match &info.reflog {
        repo::TabData::Loaded(entries) => entries,
        _ => return,
    };

    if entries.is_empty() {
        let empty_text = Paragraph::new("No reflog entries found.")
            .style(muted_style())
            .alignment(ratatui::layout::Alignment::Center);
        let center_area = Rect::new(inner.x, inner.y + inner.height / 2, inner.width, 1);
        f.render_widget(empty_text, center_area);
        return;
    }

    let header_cells = vec![
        Cell::from(""),
        Cell::from("Selector"),
        Cell::from("Commit OID"),
        Cell::from("Action"),
        Cell::from("Message"),
        Cell::from("Relative Time"),
        Cell::from("UTC Date"),
    ];
    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD).fg(ACCENT()))
        .bottom_margin(1);

    let rows: Vec<Row> = entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
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

            // 2. Selector cell (e.g. HEAD@{0})
            let cell_sel_text = Cell::from(Span::styled(
                &entry.selector,
                Style::default().add_modifier(Modifier::BOLD),
            ));

            // 3. Commit OID cell
            let cell_oid = Cell::from(Span::styled(
                &entry.target_oid[..7.min(entry.target_oid.len())],
                Style::default().fg(SUCCESS()),
            ));

            // 4. Command/Action cell (highlighted based on action type)
            let action_style = match entry.command.as_str() {
                "checkout" => Style::default().fg(SUCCESS()),
                "commit" => Style::default().fg(ACCENT()),
                "rebase" => Style::default().fg(WARNING()),
                "reset" => Style::default().fg(DANGER()),
                _ => Style::default(),
            };
            let cell_cmd =
                Cell::from(Span::styled(&entry.command, action_style.add_modifier(Modifier::BOLD)));

            // 5. Message cell
            let cell_msg = Cell::from(Span::raw(&entry.message));

            // 6. Relative time
            let cell_when = Cell::from(Span::raw(&entry.when));

            // 7. UTC Date
            let cell_date = Cell::from(Span::styled(&entry.date, muted_style()));

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

            Row::new(vec![
                cell_sel,
                cell_sel_text,
                cell_oid,
                cell_cmd,
                cell_msg,
                cell_when,
                cell_date,
            ])
            .style(row_style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Percentage(50),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
        ],
    )
    .header(header);

    // Scroll table state to follow selection
    let mut state = ratatui::widgets::TableState::default();
    state.select(Some(selection));

    f.render_stateful_widget(table, inner, &mut state);
}
