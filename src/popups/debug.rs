use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect, Margin};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap, Padding, Gauge, List, ListItem};
use crate::app::{App, Mode};
use crate::ui::style::{accent_style, muted_style, primary_style, ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, parse_color};
use crate::ui::layout::{centered_rect, centered_rect_fixed};

pub fn draw_debug_logs(f: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(90, 90, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(accent_style())
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("Debug Logs", primary_style()),
                Span::raw(" "),
            ])
            .alignment(Alignment::Center),
        );

    f.render_widget(Clear, popup_area);
    f.render_widget(block.clone(), popup_area);

    let inner_rect = block.inner(popup_area);

    let logs = crate::debug_log::get_logs();
    let height = inner_rect.height as usize;
    let total_logs = logs.len();

    let start_idx = app.debug_log_scroll;
    let end_idx = (start_idx + height).min(total_logs);

    let visible_lines: Vec<Line> = logs[start_idx..end_idx]
        .iter()
        .map(|log_str| {
            let mut spans = Vec::new();
            if log_str.len() > 21 {
                let time_part = &log_str[0..10];
                let level_part = &log_str[10..18];
                let rest = &log_str[18..];

                spans.push(Span::styled(time_part, muted_style()));
                if level_part.contains("ERROR") {
                    spans.push(Span::styled(
                        level_part,
                        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                    ));
                } else if level_part.contains("WARN") {
                    spans.push(Span::styled(
                        level_part,
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ));
                } else if level_part.contains("INFO") {
                    spans.push(Span::styled(level_part, Style::default().fg(Color::Green)));
                } else {
                    spans.push(Span::styled(level_part, Style::default().fg(Color::Blue)));
                }
                spans.push(Span::raw(rest));
            } else {
                spans.push(Span::raw(log_str));
            }
            Line::from(spans)
        })
        .collect();

    let paragraph = Paragraph::new(visible_lines).style(Style::default());

    f.render_widget(paragraph, inner_rect);
}

