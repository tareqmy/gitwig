use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect, Margin, Position};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap, Padding, Gauge, List, ListItem, ListState, Table, Row, Cell};
use crate::app::{App, Mode, DetailSection};
use crate::repo::{RemoteInfo, DiffLineKind};
use crate::ui::style::{accent_style, muted_style, primary_style, ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, parse_color};
use crate::ui::layout::{centered_rect, centered_rect_fixed};
use crate::ui_detail::{error_style, read_file_content, file_entry_line, DetailAreas};
use crate::repo::FileEntry;
use crate::repo;
use crate::components::diff::draw_file_subpanel;
use crate::repo::{RepoInfo, CommitEntry, DiffLine, WorktreeChanges};


pub fn draw_stashes_view(
    f: &mut Frame,
    info: &RepoInfo,
    focus: DetailSection,
    stash_selection: usize,
    stash_file_selection: usize,
    file_diff: &[DiffLine],
    diff_scroll: usize,
    areas: &mut DetailAreas,
    stashes_horizontal_split_pct: u16,
    stashes_vertical_split_pct: u16,
    app: &crate::app::App,
    area: Rect,
) {
    if info.stashes.is_loading() || info.stashes.is_not_loaded() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(muted_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Stashes", primary_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let loading_text = Paragraph::new("⟳ Loading stashes...")
            .style(muted_style())
            .alignment(ratatui::layout::Alignment::Center);
        let center_area = Rect::new(inner.x, inner.y + inner.height / 2, inner.width, 1);
        f.render_widget(loading_text, center_area);
        return;
    }
    if let repo::TabData::Error(err) = &info.stashes {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(error_style())
            .title(Line::from(vec![
                Span::raw(" "),
                Span::styled("Stashes - Error", error_style()),
                Span::raw(" "),
            ]))
            .padding(Padding::uniform(1));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let error_text = Paragraph::new(format!("Error loading stashes: {}", err))
            .style(error_style())
            .wrap(Wrap { trim: false });
        f.render_widget(error_text, inner);
        return;
    }

    areas.bottom_left = None;
    areas.bottom_right = None;
    areas.commits = None;
    areas.local_branches = None;
    areas.remote_branches = None;
    areas.local_tags = None;
    areas.remote_tags = None;
    areas.files = None;
    areas.file_content = None;
    areas.remotes = None;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(stashes_horizontal_split_pct),
            Constraint::Percentage(100 - stashes_horizontal_split_pct),
        ])
        .split(area);

    let left_area = chunks[0];
    let right_area = chunks[1];

    areas.bottom_right = Some(right_area);

    // Record horizontal splitter boundary in stashes tab
    let split_col = area.x + left_area.width;
    areas.stashes_horizontal_splitter =
        Some(Rect::new(split_col.saturating_sub(1), area.y, 2, area.height));

    // Split left area vertically: top = Stashes list, bottom = Stashed files
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(stashes_vertical_split_pct),
            Constraint::Percentage(100 - stashes_vertical_split_pct),
        ])
        .split(left_area);

    // Record vertical splitter boundary in left panel of stashes tab
    let split_row = left_area.y + left_chunks[0].height;
    areas.stashes_vertical_splitter =
        Some(Rect::new(left_area.x, split_row.saturating_sub(1), left_area.width, 2));

    areas.stashes = Some(left_chunks[0]);
    areas.stashed_files = Some(left_chunks[1]);

    // ── Stashes List Panel ──
    let stashes_focused = focus == DetailSection::Stashes;
    let stashes_border_style =
        if stashes_focused { Style::default().fg(ACCENT()) } else { muted_style() };

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(stashes_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Stashes", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let list_items: Vec<ListItem> = info
        .stashes
        .iter()
        .map(|s| {
            ListItem::new(Line::from(vec![Span::styled(
                format!("  stash@{{{}}}: {}", s.index, s.message),
                primary_style(),
            )]))
        })
        .collect();

    let inner = list_block.inner(left_chunks[0]);
    areas.stashes_inner = Some(inner);

    let list = List::new(list_items)
        .block(list_block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut list_state = app.stash_list_state.borrow_mut();
    if stashes_focused || !info.stashes.is_empty() {
        list_state.select(Some(stash_selection));
    } else {
        list_state.select(None);
    }
    f.render_stateful_widget(list, left_chunks[0], &mut *list_state);

    // ── Stashed Files List Panel ──
    let files_focused = focus == DetailSection::StashedFiles;
    let selected_stash = info.stashes.get(stash_selection);
    let stashed_files = selected_stash.map(|s| s.files.as_slice()).unwrap_or(&[]);

    let stashed_files_inner = draw_file_subpanel(
        f,
        "Stashed Files",
        WARNING(),
        stashed_files,
        "No files in this stash",
        Borders::ALL,
        files_focused,
        if files_focused || !stashed_files.is_empty() { Some(stash_file_selection) } else { None },
        &app.stash_file_list_state,
        left_chunks[1],
    );
    areas.stashed_files_inner = Some(stashed_files_inner);

    // ── Right panel: Diff/Stash Details ──
    let diff_focused = focus == DetailSection::StagingDetails;
    let right_border_style =
        if diff_focused { Style::default().fg(ACCENT()) } else { muted_style() };

    let selected_file_name: Option<String> =
        stashed_files.get(stash_file_selection).map(|e| e.path.clone());

    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(right_border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Stash Diff", primary_style()),
            if let Some(ref name) = selected_file_name {
                Span::styled(format!("  {}", name), muted_style())
            } else {
                Span::raw("")
            },
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let right_inner = right_block.inner(right_area);
    f.render_widget(right_block, right_area);

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
        let diff_spans: Vec<Line> = file_diff
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
            Paragraph::new(diff_spans).scroll((diff_scroll as u16, 0)).wrap(Wrap { trim: false }),
            right_inner,
        );
    }
}

