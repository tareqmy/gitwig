use crate::app::App;
use crate::repo::{FileEntry, RepoInfo};
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
    let popup_area = centered_rect(75, 65, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Stash Workspace Changes", primary_style().add_modifier(Modifier::BOLD)),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
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

    // Build the flat list of all changes
    let mut files = Vec::new();
    for entry in &info.changes.conflicted {
        files.push((entry, "conflict", Style::default().fg(DANGER())));
    }
    for entry in &info.changes.staged {
        files.push((entry, "staged", Style::default().fg(SUCCESS())));
    }
    for entry in &info.changes.unstaged {
        files.push((entry, "unstaged", Style::default().fg(WARNING())));
    }
    if app.stash_untracked {
        for entry in &info.changes.untracked {
            files.push((entry, "untracked", Style::default().fg(Color::Cyan)));
        }
    }

    // 1. Files List Panel (Left)
    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(muted_style())
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(format!("Files to Stash ({})", files.len()), primary_style()),
            Span::raw(" "),
        ]));

    let list_items: Vec<ListItem> = files
        .iter()
        .enumerate()
        .map(|(idx, (entry, category, style))| {
            let prefix = format!("[{}] ", entry.label);
            let select_marker = if idx == app.stashing_ui_selection { "▶ " } else { "  " };
            let select_style = if idx == app.stashing_ui_selection {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(select_marker, Style::default().fg(ACCENT())),
                Span::styled(prefix, *style),
                Span::styled(entry.path.clone(), select_style),
                Span::styled(format!(" ({})", category), muted_style()),
            ]))
        })
        .collect();

    let list = List::new(list_items)
        .block(list_block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    // Stateful scroll list
    let mut list_state = ratatui::widgets::ListState::default();
    if !files.is_empty() {
        list_state.select(Some(app.stashing_ui_selection));
    }
    f.render_stateful_widget(list, main_chunks[0], &mut list_state);

    // 2. Options Panel (Right)
    let options_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
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
    let help_line = Line::from(vec![
        Span::styled(" [s]", accent_style()),
        Span::raw(" Save Stash  "),
        Span::styled("[u]", accent_style()),
        Span::raw(" Toggle Untracked  "),
        Span::styled("[i]", accent_style()),
        Span::raw(" Toggle Keep Index  "),
        Span::styled("[Esc/q]", accent_style()),
        Span::raw(" Cancel  "),
        Span::styled("[↑↓/k/j]", accent_style()),
        Span::raw(" Navigate Files"),
    ]);
    f.render_widget(Paragraph::new(help_line).alignment(Alignment::Center), outer_chunks[1]);
}
