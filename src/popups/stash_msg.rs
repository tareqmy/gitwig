//! Comments input popup for creating named stashes.

use crate::app::{App, Mode};
use crate::repo::RemoteInfo;
use crate::ui::layout::{centered_rect, centered_rect_fixed};
use crate::ui::style::{
    ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, accent_style, muted_style, parse_color,
    primary_style,
};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState, Padding, Paragraph, Wrap,
};

use crate::ui::*;

pub fn draw_stash_create_popup(f: &mut Frame, input_buffer: &str, area: Rect, app: &App) {
    let popup_area = centered_rect(50, 32, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Stash Changes", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::uniform(1));

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Message label
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Message text
            Constraint::Length(1), // Spacer
            Constraint::Length(4), // Options block (needs height 4 for borders + 2 lines of text)
            Constraint::Length(1), // Spacer
            Constraint::Min(0),    // Help hint
        ])
        .split(inner_area);

    // 1. Message label
    f.render_widget(
        Paragraph::new(Span::styled("Stash Name / Message:", muted_style())),
        chunks[0],
    );

    // 3. Message text
    let text = if input_buffer.is_empty() {
        Paragraph::new(Span::styled("(optional message...)", muted_style()))
    } else {
        Paragraph::new(input_buffer).style(Style::default())
    };
    f.render_widget(text, chunks[2]);

    // 5. Options Block
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
        Line::from(vec![
            Span::styled(format!("{} ", untracked_chk), untracked_style),
            Span::raw("Stash untracked files  "),
            Span::styled("(toggle: [⌃U])", muted_style()),
        ]),
        Line::from(vec![
            Span::styled(format!("{} ", keep_index_chk), keep_index_style),
            Span::raw("Keep index             "),
            Span::styled("(toggle: [⌃I])", muted_style()),
        ]),
    ];
    f.render_widget(
        Paragraph::new(options_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(CARD_BORDER())
                .border_style(muted_style())
                .title(Line::from(vec![
                    Span::raw(" "),
                    Span::styled("Stash Options", primary_style()),
                    Span::raw(" "),
                ])),
        ),
        chunks[4],
    );

    // 7. Help Hint
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[⌃U]", primary_style().add_modifier(Modifier::BOLD)),
            Span::raw(" Toggle Untracked  "),
            Span::styled("[⌃I]", primary_style().add_modifier(Modifier::BOLD)),
            Span::raw(" Toggle Keep Index  "),
            Span::styled("[Enter]", primary_style().add_modifier(Modifier::BOLD)),
            Span::raw(" Confirm  "),
            Span::styled("[Esc]", primary_style().add_modifier(Modifier::BOLD)),
            Span::raw(" Cancel"),
        ]))
        .alignment(Alignment::Center),
        chunks[6],
    );

    // Set cursor on the input field
    let cursor_y = chunks[2].y;
    let cursor_offset = input_buffer.chars().count() as u16;
    let cursor_x = chunks[2]
        .x
        .saturating_add(cursor_offset)
        .min(chunks[2].x.saturating_add(chunks[2].width.saturating_sub(1)));
    f.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
}
