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
pub fn draw_branch_create_popup(
    f: &mut Frame,
    input_buffer: &str,
    base_branch: Option<&str>,
    area: Rect,
) {
    let popup_area = centered_rect(50, 20, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" "),
        Span::styled("Create Branch", primary_style()),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let base_name = base_branch.unwrap_or("HEAD");
    let content = vec![
        Line::from(vec![
            Span::styled("Base: ", muted_style()),
            Span::styled(base_name, primary_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("New Branch Name: ", muted_style()),
            Span::styled(input_buffer, primary_style().add_modifier(Modifier::BOLD)),
        ]),
    ];

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let paragraph = Paragraph::new(content);
    f.render_widget(paragraph, inner_area);

    let cursor_y = inner_area
        .y
        .saturating_add(2)
        .min(inner_area.y.saturating_add(inner_area.height.saturating_sub(1)));
    let label_width = "New Branch Name: ".chars().count() as u16;
    let cursor_offset = label_width.saturating_add(input_buffer.chars().count() as u16);
    let cursor_x = inner_area
        .x
        .saturating_add(cursor_offset)
        .min(inner_area.x.saturating_add(inner_area.width.saturating_sub(1)));
    f.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
}
