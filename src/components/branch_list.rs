use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect, Margin, Position};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap, Padding, Gauge, List, ListItem, ListState, Table, Row, Cell};
use crate::app::{App, Mode, DetailSection};
use crate::repo::RemoteInfo;
use crate::ui::style::{accent_style, muted_style, primary_style, ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, parse_color};
use crate::ui::layout::{centered_rect, centered_rect_fixed};
use crate::ui_detail::{error_style, read_file_content, file_entry_line, DetailAreas};
use crate::repo::FileEntry;
use crate::repo;
use crate::repo::{RepoInfo, CommitEntry, DiffLine, WorktreeChanges};


pub fn draw_branches_view(
    f: &mut Frame,
    info: &RepoInfo,
    focus: DetailSection,
    local_branch_selection: usize,
    remote_branch_selection: usize,
    areas: &mut DetailAreas,
    branches_horizontal_split_pct: u16,
    app: &crate::app::App,
    area: Rect,
) {
    if info.local_branches.is_loading() || info.local_branches.is_not_loaded() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(muted_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Branches", primary_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let loading_text = Paragraph::new("⟳ Loading branches...")
            .style(muted_style())
            .alignment(ratatui::layout::Alignment::Center);
        let center_area = Rect::new(inner.x, inner.y + inner.height / 2, inner.width, 1);
        f.render_widget(loading_text, center_area);
        return;
    }
    if let repo::TabData::Error(err) = &info.local_branches {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(error_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Branches - Error", error_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let error_text = Paragraph::new(format!("Error loading branches: {}", err))
            .style(error_style())
            .wrap(Wrap { trim: false });
        f.render_widget(error_text, inner);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(branches_horizontal_split_pct),
            Constraint::Percentage(100 - branches_horizontal_split_pct),
        ])
        .split(area);

    let left_area = chunks[0];
    let right_area = chunks[1];

    areas.local_branches = Some(left_area);
    areas.remote_branches = Some(right_area);

    // Record horizontal splitter boundary in branches tab
    let split_col = area.x + left_area.width;
    areas.branches_horizontal_splitter =
        Some(Rect::new(split_col.saturating_sub(1), area.y, 2, area.height));

    // ── Local Branches Panel ──
    let local_focused = focus == DetailSection::LocalBranches;
    let local_border_style =
        if local_focused { Style::default().fg(ACCENT()) } else { muted_style() };
    let local_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(local_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Local Branches", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let local_items: Vec<ListItem> = info
        .local_branches
        .iter()
        .map(|b| {
            let style = if b.is_head {
                Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let prefix = if b.is_head { app.sym("branch") } else { "  " };
            let mut spans = vec![
                Span::styled(prefix, Style::default().fg(SUCCESS())),
                Span::styled(b.name.clone(), style),
            ];
            if !b.short_sha.is_empty() {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(format!("[{}]", b.short_sha), accent_style()));
                if !b.short_message.is_empty() {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled("·", muted_style()));
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(b.short_message.clone(), muted_style()));
                }
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let inner = local_block.inner(left_area);
    areas.local_branches_inner = Some(inner);

    let local_list = List::new(local_items)
        .block(local_block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut local_state = app.local_branch_list_state.borrow_mut();
    if local_focused {
        local_state.select(Some(local_branch_selection));
    } else {
        local_state.select(None);
    }
    f.render_stateful_widget(local_list, left_area, &mut *local_state);

    // ── Remote Branches Panel ──
    let remote_focused = focus == DetailSection::RemoteBranches;
    let remote_border_style =
        if remote_focused { Style::default().fg(ACCENT()) } else { muted_style() };
    let remote_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(remote_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Remote Branches", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let remote_items: Vec<ListItem> = info
        .remote_branches
        .iter()
        .map(|b| {
            let mut spans = vec![Span::raw("  "), Span::styled(b.name.clone(), primary_style())];
            if !b.short_sha.is_empty() {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(format!("[{}]", b.short_sha), accent_style()));
                if !b.short_message.is_empty() {
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled("·", muted_style()));
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(b.short_message.clone(), muted_style()));
                }
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let inner = remote_block.inner(right_area);
    areas.remote_branches_inner = Some(inner);

    let remote_list = List::new(remote_items)
        .block(remote_block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut remote_state = app.remote_branch_list_state.borrow_mut();
    if remote_focused {
        remote_state.select(Some(remote_branch_selection));
    } else {
        remote_state.select(None);
    }
    f.render_stateful_widget(remote_list, right_area, &mut *remote_state);
}

pub fn draw_remotes_view(
    f: &mut Frame,
    info: &RepoInfo,
    focus: DetailSection,
    remote_selection: usize,
    areas: &mut DetailAreas,
    app: &crate::app::App,
    area: Rect,
) {
    if info.remotes.is_loading() || info.remotes.is_not_loaded() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(muted_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Remotes", primary_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let loading_text = Paragraph::new("⟳ Loading remotes...")
            .style(muted_style())
            .alignment(ratatui::layout::Alignment::Center);
        let center_area = Rect::new(inner.x, inner.y + inner.height / 2, inner.width, 1);
        f.render_widget(loading_text, center_area);
        return;
    }
    if let repo::TabData::Error(err) = &info.remotes {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(error_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Remotes - Error", error_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let error_text = Paragraph::new(format!("Error loading remotes: {}", err))
            .style(error_style())
            .wrap(Wrap { trim: false });
        f.render_widget(error_text, inner);
        return;
    }

    areas.remotes = Some(area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    let left_area = chunks[0];
    let right_area = chunks[1];

    let focused = focus == DetailSection::Remotes;
    let border_style = if focused { Style::default().fg(ACCENT()) } else { muted_style() };

    // ── Remotes List Panel ──
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Remotes", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let list_items: Vec<ListItem> = info
        .remotes
        .iter()
        .map(|r| {
            ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(r.name.clone(), primary_style()),
            ]))
        })
        .collect();

    let inner = list_block.inner(left_area);
    areas.remotes_inner = Some(inner);

    let list = List::new(list_items)
        .block(list_block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut list_state = app.remote_list_state.borrow_mut();
    if focused {
        list_state.select(Some(remote_selection));
    } else {
        list_state.select(None);
    }
    f.render_stateful_widget(list, left_area, &mut *list_state);

    // ── Remote Details Panel ──
    let details_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(muted_style())
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Remote Details", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let mut details_lines = Vec::new();
    if let Some(selected_remote) = info.remotes.get(remote_selection) {
        details_lines.push(Line::from(""));
        details_lines.push(Line::from(vec![
            Span::styled("  Name:      ", accent_style()),
            Span::styled(selected_remote.name.clone(), primary_style()),
        ]));
        details_lines.push(Line::from(vec![
            Span::styled("  Fetch URL: ", accent_style()),
            Span::raw(selected_remote.url.clone()),
        ]));
        if let Some(push_url) = &selected_remote.push_url {
            details_lines.push(Line::from(vec![
                Span::styled("  Push URL:  ", accent_style()),
                Span::raw(push_url.clone()),
            ]));
        } else {
            details_lines.push(Line::from(vec![
                Span::styled("  Push URL:  ", accent_style()),
                Span::raw(selected_remote.url.clone()),
            ]));
        }
        details_lines.push(Line::from(""));
        details_lines
            .push(Line::from(vec![Span::raw("  "), Span::styled("Refspecs:", primary_style())]));
        for spec in &selected_remote.refspecs {
            details_lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(spec.clone(), muted_style()),
            ]));
        }
    } else {
        details_lines.push(Line::from(""));
        details_lines.push(Line::from(Span::styled("  No remotes configured", muted_style())));
    }

    let details_paragraph = Paragraph::new(details_lines).block(details_block);
    f.render_widget(details_paragraph, right_area);
}

