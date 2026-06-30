use crate::app::{App, DetailSection};
use crate::repo;
use crate::repo::RepoInfo;
use crate::ui::style::{
    ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, muted_style, parse_color, primary_style,
};
use crate::ui_detail::{DetailAreas, error_style};
use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Row, Table, Wrap};

pub fn draw_worktrees_view(
    f: &mut Frame,
    info: &RepoInfo,
    focus: DetailSection,
    selection: usize,
    areas: &mut DetailAreas,
    _app: &App,
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

    let title_suffix = if focused {
        " (Use ↑/↓ to navigate, 'a' to add, 'd' to delete, 'l' to lock, 'p' to prune, 'Enter' to open)"
    } else {
        ""
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(format!("Worktrees{}", title_suffix), primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::uniform(1));

    let inner = block.inner(area);
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

    let header_cells = vec!["Name", "Branch", "Lock Status", "Path"];
    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD).fg(ACCENT()))
        .bottom_margin(1);

    let rows: Vec<Row> = worktrees
        .iter()
        .enumerate()
        .map(|(idx, wt)| {
            let name = &wt.name;
            let branch = wt.branch.as_deref().unwrap_or("(no branch)");

            let (lock_str, lock_style) = if wt.is_locked {
                let reason = wt.lock_reason.as_deref().unwrap_or("locked");
                (format!("Locked 🔒 ({})", reason), Style::default().fg(WARNING()))
            } else {
                ("Unlocked 🔓".to_string(), Style::default().fg(SUCCESS()))
            };

            let path_exists = wt.path.exists();
            let (path_str, path_style) = if !path_exists {
                (format!("{} (Missing ⚠)", wt.path.display()), Style::default().fg(DANGER()))
            } else {
                (wt.path.display().to_string(), Style::default())
            };

            let row_style = if idx == selection && focused {
                Style::default().bg(parse_color("#333333")).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Span::raw(name.clone()),
                Span::raw(branch.to_string()),
                Span::styled(lock_str, lock_style),
                Span::styled(path_str, path_style),
            ])
            .style(row_style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(15),
            Constraint::Percentage(20),
            Constraint::Percentage(25),
            Constraint::Percentage(40),
        ],
    )
    .header(header)
    .block(Block::default());

    f.render_widget(table, inner);
}
