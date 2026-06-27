use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect, Margin, Position};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap, Padding, Gauge, List, ListItem, ListState};
use crate::app::{App, Mode};
use crate::repo::RemoteInfo;
use crate::ui::style::{accent_style, muted_style, primary_style, ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, parse_color};
use crate::ui::layout::{centered_rect, centered_rect_fixed};

use crate::ui::*;
pub fn draw_search_column_picker(f: &mut Frame, app: &crate::app::App, area: Rect) {
    let popup_area = centered_rect(50, 30, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(ACCENT()))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Search Columns Selector", primary_style()),
            Span::raw(" "),
        ]));

    let inner = block.inner(popup_area);
    f.render_widget(Clear, popup_area);
    f.render_widget(block, popup_area);

    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Spacer
            Constraint::Length(4), // Columns
            Constraint::Min(0),    // Instructions
        ])
        .split(inner);

    let columns = [
        ("SHA", app.search_columns_sha),
        ("Message", app.search_columns_message),
        ("Author", app.search_columns_author),
        ("Date", app.search_columns_date),
    ];

    let mut lines = Vec::new();
    for (idx, (name, enabled)) in columns.iter().enumerate() {
        let is_selected = idx == app.search_column_selection;
        let checkbox = if *enabled { "[x]" } else { "[ ]" };

        let checkbox_span = if *enabled {
            Span::styled(checkbox, Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(checkbox, muted_style())
        };

        let style = if is_selected {
            Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let select_indicator = if is_selected { "▸ " } else { "  " };

        lines.push(Line::from(vec![
            Span::styled(select_indicator, style),
            checkbox_span,
            Span::raw(" "),
            Span::styled(name.to_string(), style),
        ]));
    }

    f.render_widget(Paragraph::new(lines), vertical_chunks[1]);

    let instructions = Line::from(vec![
        Span::styled(" [Space]", accent_style()),
        Span::raw(" Toggle  "),
        Span::styled("[Enter]", accent_style()),
        Span::raw(" Confirm  "),
        Span::styled("[Esc]", accent_style()),
        Span::raw(" Cancel "),
    ]);
    f.render_widget(Paragraph::new(instructions).alignment(Alignment::Center), vertical_chunks[2]);
}

