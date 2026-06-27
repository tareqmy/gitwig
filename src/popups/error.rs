use crate::app::{App, Mode};
use crate::repo::RemoteInfo;
use crate::ui::layout::{centered_rect, centered_rect_fixed};
use crate::ui::style::{
    ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, accent_style, muted_style, parse_color,
    primary_style,
};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, Gauge, List, ListItem, ListState, Padding, Paragraph, Wrap,
};

pub fn draw_error_popup(f: &mut Frame, app: &App, area: Rect, err: &str) {
    let popup_width = (area.width * 80 / 100).clamp(60, 80).min(area.width);
    let inner_width = (popup_width as usize).saturating_sub(6).max(20);

    let mut msg_height = 0;
    if err.is_empty() {
        msg_height = 1;
    } else {
        for line in err.lines() {
            let mut current_line_width = 0;
            for word in line.split_whitespace() {
                let word_len = word.chars().count();
                if current_line_width + word_len + (if current_line_width > 0 { 1 } else { 0 })
                    > inner_width
                {
                    msg_height += 1;
                    current_line_width = word_len;
                    while current_line_width > inner_width {
                        msg_height += 1;
                        current_line_width -= inner_width;
                    }
                } else {
                    if current_line_width > 0 {
                        current_line_width += 1;
                    }
                    current_line_width += word_len;
                }
            }
            msg_height += 1;
        }
    }

    let max_height = (area.height * 80 / 100).clamp(10, 30).min(area.height);
    let popup_height = ((msg_height + 4) as u16).min(max_height);

    let popup_area = centered_rect_fixed(popup_width, popup_height, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(DANGER());
    let title = Line::from(vec![
        Span::raw(format!(" {} ", app.sym("warning").trim())),
        Span::styled("Error", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(2));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // error message
            Constraint::Length(1), // spacer
            Constraint::Length(1), // dismiss hint
        ])
        .split(inner);

    let err_para =
        Paragraph::new(err).wrap(ratatui::widgets::Wrap { trim: true }).style(Style::default());
    f.render_widget(err_para, chunks[0]);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled("Press ", muted_style()),
        Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" or ", muted_style()),
        Span::styled("Enter", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" to dismiss", muted_style()),
    ]))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(hint, chunks[2]);
}
