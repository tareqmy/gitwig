//! Git worktrees management input modal.

use crate::ui::layout::centered_rect;
use crate::ui::style::{ACCENT, CARD_BORDER, DANGER, muted_style, primary_style};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};

pub fn draw_worktree_add_branch_popup(f: &mut Frame, input_buffer: &str, area: Rect) {
    let popup_area = centered_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(Style::default().fg(ACCENT()))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Add Worktree - Step 1/2", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let content = vec![
        Line::from(vec![Span::styled(
            "Enter base branch/commit name (e.g. main): ",
            muted_style(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("> ", Style::default().fg(ACCENT())),
            Span::styled(input_buffer, Style::default()),
        ]),
    ];

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);
    f.render_widget(Paragraph::new(content), inner_area);

    let cursor_y = inner_area.y.saturating_add(2);
    let cursor_x =
        inner_area.x.saturating_add(2).saturating_add(input_buffer.chars().count() as u16);
    f.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
}

pub fn draw_worktree_add_path_popup(
    f: &mut Frame,
    input_buffer: &str,
    branch_name: &str,
    area: Rect,
) {
    let popup_area = centered_rect(55, 20, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(Style::default().fg(ACCENT()))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Add Worktree - Step 2/2", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let content = vec![
        Line::from(vec![
            Span::styled("Selected Branch: ", muted_style()),
            Span::styled(branch_name, Style::default()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Enter destination path (e.g. ../my-worktree): ",
            muted_style(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("> ", Style::default().fg(ACCENT())),
            Span::styled(input_buffer, Style::default()),
        ]),
    ];

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);
    f.render_widget(Paragraph::new(content), inner_area);

    let cursor_y = inner_area.y.saturating_add(4);
    let cursor_x =
        inner_area.x.saturating_add(2).saturating_add(input_buffer.chars().count() as u16);
    f.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
}

pub fn draw_worktree_lock_reason_popup(
    f: &mut Frame,
    input_buffer: &str,
    wt_name: &str,
    area: Rect,
) {
    let popup_area = centered_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(Style::default().fg(ACCENT()))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Lock Worktree", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let content = vec![
        Line::from(vec![
            Span::styled("Locking worktree: ", muted_style()),
            Span::styled(wt_name, Style::default()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Enter reason (optional): ", muted_style())]),
        Line::from(""),
        Line::from(vec![
            Span::styled("> ", Style::default().fg(ACCENT())),
            Span::styled(input_buffer, Style::default()),
        ]),
    ];

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);
    f.render_widget(Paragraph::new(content), inner_area);

    let cursor_y = inner_area.y.saturating_add(4);
    let cursor_x =
        inner_area.x.saturating_add(2).saturating_add(input_buffer.chars().count() as u16);
    f.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
}
