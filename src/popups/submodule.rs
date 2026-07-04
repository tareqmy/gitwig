use crate::ui::layout::centered_rect;
use crate::ui::style::{ACCENT, CARD_BORDER, DANGER, muted_style, primary_style};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph};

pub fn draw_submodule_add_url_popup(f: &mut Frame, input_buffer: &str, area: Rect) {
    let popup_area = centered_rect(60, 20, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(Style::default().fg(ACCENT()))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Add Submodule - Step 1/2", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let content = vec![
        Line::from(vec![Span::styled(
            "Enter submodule repository URL (e.g. https://github.com/foo/bar.git):",
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

pub fn draw_submodule_add_path_popup(f: &mut Frame, input_buffer: &str, url: &str, area: Rect) {
    let popup_area = centered_rect(60, 20, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(Style::default().fg(ACCENT()))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Add Submodule - Step 2/2", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let content = vec![
        Line::from(vec![
            Span::styled("Repository URL: ", muted_style()),
            Span::styled(url, primary_style()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled("Enter destination path (e.g. libs/bar):", muted_style())]),
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

pub fn draw_submodule_delete_popup(f: &mut Frame, name: &str, area: Rect) {
    let popup_area = centered_rect(55, 20, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(Style::default().fg(DANGER()))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Delete Submodule", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let content = vec![
        Line::from(vec![Span::styled(
            "Are you sure you want to completely delete the submodule:",
            primary_style(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(name, Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "This will deinitialize and remove the submodule directory.",
            Style::default().fg(DANGER()),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Confirm: ", muted_style()),
            Span::styled("y", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
            Span::styled(" / Cancel: ", muted_style()),
            Span::styled("n", Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
        ]),
    ];

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);
    f.render_widget(Paragraph::new(content), inner_area);
}
