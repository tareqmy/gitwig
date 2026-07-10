use crate::app::{App, DetailSection};
use crate::repo;
use crate::repo::RepoInfo;
use crate::ui::style::{ACCENT, CARD_BORDER, SUCCESS, WARNING, muted_style, primary_style};
use crate::ui_detail::{DetailAreas, error_style};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table, Wrap};

pub fn draw_forge_view(
    f: &mut Frame,
    info: &RepoInfo,
    focus: DetailSection,
    selection: usize,
    areas: &mut DetailAreas,
    app: &App,
    area: Rect,
) {
    // 1. Loading State
    if info.forge_issues.is_loading() || info.forge_issues.is_not_loaded() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(muted_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Forge - Issues", primary_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let loading_text = Paragraph::new("⟳ Loading issues from Forge...")
            .style(muted_style())
            .alignment(ratatui::layout::Alignment::Center);
        let center_area = Rect::new(inner.x, inner.y + inner.height / 2, inner.width, 1);
        f.render_widget(loading_text, center_area);
        return;
    }

    // 2. Error State
    if let repo::TabData::Error(err) = &info.forge_issues {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(error_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Forge - Error", error_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let error_text = Paragraph::new(format!("Error loading Forge issues: {}", err))
            .style(error_style())
            .wrap(Wrap { trim: false });
        f.render_widget(error_text, inner);
        return;
    }

    // 3. Main Data layout: Top (issues list), Bottom (issue detail)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(app.forge_vertical_split_pct),
            Constraint::Length(1), // splitter
            Constraint::Percentage(100 - app.forge_vertical_split_pct),
        ])
        .split(area);

    areas.forge_issues = Some(chunks[0]);
    areas.forge_issue_details = Some(chunks[2]);
    areas.forge_vertical_splitter = Some(chunks[1]);

    // Draw a small line for splitter
    let splitter_str = "─".repeat(chunks[1].width as usize);
    f.render_widget(Paragraph::new(splitter_str).style(muted_style()), chunks[1]);

    let issues = match &info.forge_issues {
        repo::TabData::Loaded(issues) => issues,
        _ => return,
    };

    // Draw Issues List (Top Panel)
    let focused_issues = focus == DetailSection::ForgeIssues;
    let list_border_style =
        if focused_issues { Style::default().fg(ACCENT()) } else { muted_style() };
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(list_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(
                if app.forge_issues_assigned_only {
                    "Issues (Assigned to me)"
                } else {
                    "Issues (All Open)"
                },
                primary_style(),
            ),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(0));
    let list_inner = list_block.inner(chunks[0]);
    f.render_widget(list_block, chunks[0]);

    if issues.is_empty() {
        let empty_text = Paragraph::new("No issues found.")
            .style(muted_style())
            .alignment(ratatui::layout::Alignment::Center);
        let center_area =
            Rect::new(list_inner.x, list_inner.y + list_inner.height / 2, list_inner.width, 1);
        f.render_widget(empty_text, center_area);
    } else {
        let header_cells = vec![
            Cell::from(""),
            Cell::from("#"),
            Cell::from("Title"),
            Cell::from("State"),
            Cell::from("Author"),
            Cell::from("Assignees"),
        ];
        let header = Row::new(header_cells)
            .style(Style::default().add_modifier(Modifier::BOLD).fg(ACCENT()))
            .bottom_margin(1);

        let rows: Vec<Row> = issues
            .iter()
            .enumerate()
            .map(|(idx, issue)| {
                let is_selected = idx == selection;

                let cell_sel = if is_selected {
                    Cell::from(Span::styled(
                        app.sym("selection_mark"),
                        if focused_issues {
                            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)
                        } else {
                            muted_style()
                        },
                    ))
                } else {
                    Cell::from(Span::raw(" "))
                };

                let cell_num = Cell::from(Span::styled(
                    format!("#{}", issue.number),
                    Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
                ));

                let cell_title = Cell::from(Span::raw(&issue.title));

                let state_style = match issue.state.to_uppercase().as_str() {
                    "OPEN" => Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
                    "CLOSED" => Style::default(),
                    _ => Style::default(),
                };
                let cell_state = Cell::from(Span::styled(&issue.state, state_style));

                let cell_author = Cell::from(Span::raw(format!("@{}", issue.author)));

                let assignees_str = if issue.assignees.is_empty() {
                    "-".to_string()
                } else {
                    issue.assignees.iter().map(|a| format!("@{}", a)).collect::<Vec<_>>().join(", ")
                };
                let cell_assignees = Cell::from(Span::raw(assignees_str));

                let row_style = if is_selected {
                    if focused_issues {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default().add_modifier(Modifier::DIM)
                    }
                } else {
                    Style::default()
                };

                Row::new(vec![
                    cell_sel,
                    cell_num,
                    cell_title,
                    cell_state,
                    cell_author,
                    cell_assignees,
                ])
                .style(row_style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(2),
                Constraint::Length(8),
                Constraint::Percentage(50),
                Constraint::Length(10),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
            ],
        )
        .header(header);

        let mut state = ratatui::widgets::TableState::default();
        state.select(Some(selection));
        f.render_stateful_widget(table, list_inner, &mut state);
    }

    // Draw Selected Issue Details (Bottom Panel)
    let focused_details = focus == DetailSection::ForgeIssueDetails;
    let detail_border_style =
        if focused_details { Style::default().fg(ACCENT()) } else { muted_style() };
    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(detail_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Issue Details", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));
    let detail_inner = detail_block.inner(chunks[2]);
    f.render_widget(detail_block, chunks[2]);

    if let Some(selected_issue) = issues.get(selection) {
        let detail_lines = vec![
            Line::from(vec![
                Span::styled("Number: ", muted_style()),
                Span::styled(
                    format!("#{}", selected_issue.number),
                    Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Title:  ", muted_style()),
                Span::styled(&selected_issue.title, Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("State:  ", muted_style()),
                Span::styled(&selected_issue.state, Style::default().fg(WARNING())),
            ]),
            Line::from(vec![
                Span::styled("Author: ", muted_style()),
                Span::styled(format!("@{}", selected_issue.author), Style::default().fg(ACCENT())),
            ]),
            Line::from(vec![
                Span::styled("URL:    ", muted_style()),
                Span::styled(
                    &selected_issue.url,
                    Style::default().fg(ACCENT()).add_modifier(Modifier::UNDERLINED),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Actions: ", muted_style()),
                Span::styled("Enter", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)),
                Span::raw(" - Checkout/Create branch | "),
                Span::styled("o", Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
                Span::raw(" - Open in browser | "),
                Span::styled("a", Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
                Span::raw(" - Toggle all/assigned"),
            ]),
        ];

        let paragraph = Paragraph::new(detail_lines).wrap(Wrap { trim: false });
        f.render_widget(paragraph, detail_inner);
    }
}
