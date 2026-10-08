//! The stashing panel (`S` in the Workspace): the files a new stash takes,
//! which of them are picked, and the stash options.

use crate::app::{App, StashFileKind, stash_file_rows};
use crate::repo::RepoInfo;
use crate::ui::layout::{centered_rect, centered_rect_fixed};
use crate::ui::style::{
    ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, accent_style, muted_style, parse_color,
    primary_style,
};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph};

pub fn draw_stashing_ui(f: &mut Frame, info: &RepoInfo, app: &App, area: Rect) {
    let popup_area = centered_rect(85, 75, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Stash Workspace Changes", primary_style().add_modifier(Modifier::BOLD)),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title);

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Split inner area vertically: top is split horizontally (Files list vs Options), bottom is help hint
    let outer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(inner_area);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(68), Constraint::Percentage(32)])
        .split(outer_chunks[0]);

    let rows = stash_file_rows(&info.changes, app.stash_untracked);
    let picked = rows.iter().filter(|row| app.stash_marked.contains(&row.path)).count();

    // 1. Files List Panel (Left). Nothing picked stashes every listed file;
    // once something is picked, the checkboxes say which files go.
    let list_title = if picked == 0 {
        format!("Files to Stash (all {})", rows.len())
    } else {
        format!("Files to Stash ({} of {} picked)", picked, rows.len())
    };
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(muted_style())
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(list_title, primary_style()),
            Span::raw(" "),
        ]));

    let list_items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            let is_picked = app.stash_marked.contains(&row.path);
            let goes = picked == 0 || is_picked;
            let select_marker = if idx == app.stashing_ui_selection { "▶ " } else { "  " };
            let mut path_style = if goes { Style::default() } else { muted_style() };
            if idx == app.stashing_ui_selection {
                path_style = path_style.add_modifier(Modifier::BOLD);
            }
            let kind_style = match row.kind {
                StashFileKind::Conflicted => Style::default().fg(DANGER()),
                StashFileKind::Staged => Style::default().fg(SUCCESS()),
                StashFileKind::Unstaged | StashFileKind::StagedAndUnstaged => {
                    Style::default().fg(WARNING())
                }
                StashFileKind::Untracked => Style::default().fg(ACCENT()),
            };

            let mut spans = vec![Span::styled(select_marker, Style::default().fg(ACCENT()))];
            if picked > 0 {
                let (check, check_style) = if is_picked {
                    ("[x] ", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD))
                } else {
                    ("[ ] ", muted_style())
                };
                spans.push(Span::styled(check, check_style));
            }
            spans.push(Span::styled(format!("[{}] ", row.label), kind_style));
            spans.push(Span::styled(row.path.clone(), path_style));
            spans.push(Span::styled(format!(" ({})", row.kind.caption()), muted_style()));
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(list_items)
        .block(list_block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    // Stateful scroll list
    let mut list_state = ratatui::widgets::ListState::default();
    if !rows.is_empty() {
        list_state.select(Some(app.stashing_ui_selection.min(rows.len() - 1)));
    }
    f.render_stateful_widget(list, main_chunks[0], &mut list_state);

    // 2. Options Panel (Right)
    let options_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(muted_style())
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Stashing Options", primary_style()),
            Span::raw(" "),
        ]));

    let untracked_chk = if app.stash_untracked { "[X]" } else { "[ ]" };
    let keep_index_chk = if app.stash_keep_index { "[X]" } else { "[ ]" };

    let untracked_style = if app.stash_untracked {
        Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)
    } else {
        muted_style()
    };
    let keep_index_style = if app.stash_keep_index {
        Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)
    } else {
        muted_style()
    };

    let options_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {} ", untracked_chk), untracked_style),
            Span::raw("Stash untracked files"),
        ]),
        Line::from(vec![
            Span::styled("      Toggle: ", muted_style()),
            Span::styled("[u]", accent_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {} ", keep_index_chk), keep_index_style),
            Span::raw("Keep index"),
        ]),
        Line::from(vec![
            Span::styled("      Toggle: ", muted_style()),
            Span::styled("[i]", accent_style()),
        ]),
    ];
    f.render_widget(Paragraph::new(options_text).block(options_block), main_chunks[1]);

    // 3. Help bar (Bottom)
    let scope_line = if picked == 0 {
        Line::from(Span::styled(
            "Stashes every listed file. Pick files to stash only those.",
            muted_style(),
        ))
    } else {
        Line::from(Span::styled(
            format!("Stashes only the {} picked file(s); the rest stay as they are.", picked),
            muted_style(),
        ))
    };
    let help_line = Line::from(vec![
        Span::styled(" [s]", accent_style()),
        Span::raw(" Save Stash  "),
        Span::styled("[Space]", accent_style()),
        Span::raw(" Pick File  "),
        Span::styled("[a]", accent_style()),
        Span::raw(" Pick All/None  "),
        Span::styled("[u]", accent_style()),
        Span::raw(" Toggle Untracked  "),
        Span::styled("[i]", accent_style()),
        Span::raw(" Toggle Keep Index  "),
        Span::styled("[Esc/q]", accent_style()),
        Span::raw(" Cancel  "),
        Span::styled("[↑↓/k/j]", accent_style()),
        Span::raw(" Navigate"),
    ]);
    f.render_widget(
        Paragraph::new(vec![scope_line, help_line]).alignment(Alignment::Center),
        outer_chunks[1],
    );
}
