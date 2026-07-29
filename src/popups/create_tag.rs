//! Modal input popup for creating local annotated tags.

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
pub fn draw_tag_create_popup(
    f: &mut Frame,
    input_buffer: &str,
    tag_message: &str,
    focus_message: bool,
    target_commit_oid: Option<&str>,
    area: Rect,
) {
    let popup_area = centered_rect(55, 30, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Create / Update Tag", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let commit_hash = target_commit_oid
        .map(|oid| if oid.len() >= 7 { &oid[..7] } else { oid })
        .unwrap_or("unknown");

    let name_style = if !focus_message {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let msg_style = if focus_message {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let content = vec![
        Line::from(vec![
            Span::styled("Target Commit: ", muted_style()),
            Span::styled(commit_hash, primary_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tag Name:      ", muted_style()),
            Span::styled(input_buffer, name_style),
        ]),
        Line::from(vec![
            Span::styled("Message (-m):  ", muted_style()),
            Span::styled(tag_message, msg_style),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Tab]", primary_style()),
            Span::styled(" Switch field  ", muted_style()),
            Span::styled("[Enter]", primary_style()),
            Span::styled(" Create  ", muted_style()),
            Span::styled("[Esc]", primary_style()),
            Span::styled(" Cancel", muted_style()),
        ]),
    ];

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let paragraph = Paragraph::new(content);
    f.render_widget(paragraph, inner_area);

    let cursor_line_offset = if !focus_message { 2 } else { 3 };
    let active_text = if !focus_message { input_buffer } else { tag_message };
    let label_width = "Tag Name:      ".chars().count() as u16;

    let cursor_y = inner_area
        .y
        .saturating_add(cursor_line_offset)
        .min(inner_area.y.saturating_add(inner_area.height.saturating_sub(1)));
    let cursor_offset = label_width.saturating_add(active_text.chars().count() as u16);
    let cursor_x = inner_area
        .x
        .saturating_add(cursor_offset)
        .min(inner_area.x.saturating_add(inner_area.width.saturating_sub(1)));
    f.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_tag_create_popup() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = Rect::new(0, 0, 80, 24);
                draw_tag_create_popup(
                    f,
                    "v1.0.0",
                    "Release v1.0.0",
                    false,
                    Some("1234567890"),
                    area,
                );
            })
            .unwrap();
    }
}
