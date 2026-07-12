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

pub fn draw_forge_prs_view(
    f: &mut Frame,
    info: &RepoInfo,
    focus: DetailSection,
    selection: usize,
    areas: &mut DetailAreas,
    app: &App,
    area: Rect,
) {
    // 1. Loading State
    if info.forge_prs.is_loading() || info.forge_prs.is_not_loaded() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(muted_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Forge - Pull Requests", primary_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let loading_text = Paragraph::new("⟳ Loading pull requests from Forge...")
            .style(muted_style())
            .alignment(ratatui::layout::Alignment::Center);
        let center_area = Rect::new(inner.x, inner.y + inner.height / 2, inner.width, 1);
        f.render_widget(loading_text, center_area);
        return;
    }

    // 2. Error State
    if let repo::TabData::Error(err) = &info.forge_prs {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(error_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Forge PRs - Error", error_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let error_text = Paragraph::new(format!("Error loading Forge PRs: {}", err))
            .style(error_style())
            .wrap(Wrap { trim: false });
        f.render_widget(error_text, inner);
        return;
    }

    // 3. Main Data layout: Top (PRs list), Bottom (PR detail)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(app.forge_pr_vertical_split_pct),
            Constraint::Length(1), // splitter
            Constraint::Percentage(100 - app.forge_pr_vertical_split_pct),
        ])
        .split(area);

    areas.forge_prs = Some(chunks[0]);
    areas.forge_pr_details = Some(chunks[2]);
    areas.forge_pr_vertical_splitter = Some(chunks[1]);

    // Draw a small line for splitter
    let splitter_str = "─".repeat(chunks[1].width as usize);
    f.render_widget(Paragraph::new(splitter_str).style(muted_style()), chunks[1]);

    let prs = match &info.forge_prs {
        repo::TabData::Loaded(prs) => prs,
        _ => return,
    };

    // Draw PRs List (Top Panel)
    let focused_prs = focus == DetailSection::ForgePRs;
    let list_border_style = if focused_prs { Style::default().fg(ACCENT()) } else { muted_style() };
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(list_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Pull Requests", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(0));
    let list_inner = list_block.inner(chunks[0]);
    f.render_widget(list_block, chunks[0]);

    if prs.is_empty() {
        let empty_text = Paragraph::new("No pull requests found.")
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
            Cell::from("Branch"),
        ];
        let header = Row::new(header_cells)
            .style(Style::default().add_modifier(Modifier::BOLD).fg(ACCENT()))
            .bottom_margin(1);

        let rows: Vec<Row> = prs
            .iter()
            .enumerate()
            .map(|(idx, pr)| {
                let is_selected = idx == selection;

                let cell_sel = if is_selected {
                    Cell::from(Span::styled(
                        app.sym("selection_mark"),
                        if focused_prs {
                            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)
                        } else {
                            muted_style()
                        },
                    ))
                } else {
                    Cell::from(Span::raw(" "))
                };

                let cell_num = Cell::from(Span::styled(
                    format!("#{}", pr.number),
                    Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
                ));

                let cell_title = Cell::from(Span::raw(&pr.title));

                let state_style = match pr.state.to_uppercase().as_str() {
                    "OPEN" => Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
                    "CLOSED" => Style::default(),
                    "MERGED" => Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
                    _ => Style::default(),
                };
                let cell_state = Cell::from(Span::styled(&pr.state, state_style));

                let cell_author = Cell::from(Span::raw(format!("@{}", pr.author)));
                let cell_branch = Cell::from(Span::raw(&pr.head_ref));

                let row_style = if is_selected {
                    if focused_prs {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default().add_modifier(Modifier::DIM)
                    }
                } else {
                    Style::default()
                };

                Row::new(vec![cell_sel, cell_num, cell_title, cell_state, cell_author, cell_branch])
                    .style(row_style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(2),
                Constraint::Length(8),
                Constraint::Percentage(45),
                Constraint::Length(10),
                Constraint::Percentage(15),
                Constraint::Percentage(20),
            ],
        )
        .header(header);

        let mut state = ratatui::widgets::TableState::default();
        state.select(Some(selection));
        f.render_stateful_widget(table, list_inner, &mut state);
    }

    // Draw Selected PR Details (Bottom Panel)
    let focused_details = focus == DetailSection::ForgePRDetails;
    let detail_border_style =
        if focused_details { Style::default().fg(ACCENT()) } else { muted_style() };
    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(detail_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Pull Request Details", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));
    let detail_inner = detail_block.inner(chunks[2]);
    f.render_widget(detail_block, chunks[2]);

    if let Some(selected_pr) = prs.get(selection) {
        let mut detail_lines = vec![
            Line::from(vec![
                Span::styled("Number: ", muted_style()),
                Span::styled(
                    format!("#{}", selected_pr.number),
                    Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Title:  ", muted_style()),
                Span::styled(&selected_pr.title, Style::default().add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("State:  ", muted_style()),
                Span::styled(&selected_pr.state, Style::default().fg(WARNING())),
            ]),
            Line::from(vec![
                Span::styled("Author: ", muted_style()),
                Span::styled(format!("@{}", selected_pr.author), Style::default().fg(ACCENT())),
            ]),
            Line::from(vec![
                Span::styled("Branch: ", muted_style()),
                Span::styled(&selected_pr.head_ref, Style::default().fg(SUCCESS())),
            ]),
            Line::from(vec![
                Span::styled("URL:    ", muted_style()),
                Span::styled(
                    &selected_pr.url,
                    Style::default().fg(ACCENT()).add_modifier(Modifier::UNDERLINED),
                ),
            ]),
        ];

        // CI/CD status check rollup
        if !selected_pr.status_checks.is_empty() {
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(vec![Span::styled(
                "CI/CD Checks:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            for check in &selected_pr.status_checks {
                let check_state =
                    check.conclusion.as_deref().or(check.state.as_deref()).unwrap_or("PENDING");
                let check_style = match check_state.to_uppercase().as_str() {
                    "SUCCESS" => Style::default().fg(SUCCESS()),
                    "FAILURE" | "ERROR" => Style::default().fg(crate::ui::style::DANGER()),
                    _ => Style::default().fg(WARNING()),
                };
                detail_lines.push(Line::from(vec![
                    Span::raw("  • "),
                    Span::raw(&check.name),
                    Span::raw(": "),
                    Span::styled(check_state, check_style),
                ]));
            }
        }

        // Reviews
        if !selected_pr.reviews.is_empty() {
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(vec![Span::styled(
                "Reviews:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            for review in &selected_pr.reviews {
                let rev_style = match review.state.to_uppercase().as_str() {
                    "APPROVED" => Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
                    "CHANGES_REQUESTED" => {
                        Style::default().fg(crate::ui::style::DANGER()).add_modifier(Modifier::BOLD)
                    }
                    _ => Style::default().fg(WARNING()),
                };
                detail_lines.push(Line::from(vec![
                    Span::raw("  • @"),
                    Span::raw(&review.author),
                    Span::raw(" ["),
                    Span::styled(&review.state, rev_style),
                    Span::raw("]: "),
                    Span::raw(if review.body.trim().is_empty() {
                        "(no comment body)"
                    } else {
                        review.body.trim()
                    }),
                ]));
            }
        }

        // Line Comments
        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(vec![Span::styled(
            "Line Comments:",
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        if app.forge_pr_comments_loading {
            detail_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Loading line comments...", muted_style()),
            ]));
        } else if let Some(comments) = &app.forge_pr_comments {
            if comments.is_empty() {
                detail_lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled("No line comments.", muted_style()),
                ]));
            } else {
                for comment in comments {
                    let file_line = if let Some(line) = comment.line {
                        format!("{}:{}", comment.path, line)
                    } else {
                        comment.path.clone()
                    };
                    detail_lines.push(Line::from(vec![
                        Span::raw("  • "),
                        Span::styled(file_line, Style::default().fg(ACCENT())),
                        Span::raw(" by @"),
                        Span::raw(&comment.author),
                        Span::raw(": "),
                        Span::raw(comment.body.trim()),
                    ]));
                }
            }
        } else {
            detail_lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("Failed to load comments or not authenticated.", muted_style()),
            ]));
        }

        // Description/Body
        if !selected_pr.body.trim().is_empty() {
            detail_lines.push(Line::from(""));
            detail_lines.push(Line::from(vec![Span::styled(
                "Description:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            for line in selected_pr.body.lines().take(10) {
                detail_lines.push(Line::from(format!("  {}", line)));
            }
            if selected_pr.body.lines().count() > 10 {
                detail_lines
                    .push(Line::from(vec![Span::styled("  ... (truncated)", muted_style())]));
            }
        }

        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(vec![
            Span::styled("Actions: ", muted_style()),
            Span::styled("Enter", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)),
            Span::raw(" - Checkout PR branch | "),
            Span::styled("o", Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
            Span::raw(" - Open in browser | "),
            Span::styled("n", Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
            Span::raw(" - Add line comment"),
        ]));

        let paragraph = Paragraph::new(detail_lines).wrap(Wrap { trim: false });
        f.render_widget(paragraph, detail_inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_forge_prs_view() {
        let config = crate::config::Config::default();
        let app = App::new(config, std::path::PathBuf::from("test.toml"));

        let info = RepoInfo {
            forge_prs: repo::TabData::Loaded(vec![repo::ForgePR {
                number: 456,
                title: "PR Title".to_string(),
                state: "open".to_string(),
                author: "author".to_string(),
                assignees: vec![],
                url: "https://example.com".to_string(),
                head_ref: "feature-branch".to_string(),
                head_ref_oid: "xyz789".to_string(),
                body: "PR Description".to_string(),
                status_checks: vec![repo::CIStatusCheck {
                    name: "CI".to_string(),
                    state: Some("SUCCESS".to_string()),
                    status: None,
                    conclusion: Some("SUCCESS".to_string()),
                }],
                reviews: vec![repo::PRReview {
                    author: "reviewer".to_string(),
                    state: "APPROVED".to_string(),
                    body: "LGTM".to_string(),
                }],
            }]),
            ..RepoInfo::default()
        };

        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let mut areas = DetailAreas::default();
                draw_forge_prs_view(
                    f,
                    &info,
                    DetailSection::ForgePRs,
                    0,
                    &mut areas,
                    &app,
                    Rect::new(0, 0, 80, 24),
                );
            })
            .unwrap();
    }
}
