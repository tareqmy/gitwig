use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect, Margin, Position};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap, Padding, Gauge, List, ListItem, ListState};
use crate::app::{App, Mode};
use crate::repo::RemoteInfo;
use crate::ui_detail::DetailAreas;
use crate::ui::style::{accent_style, muted_style, primary_style, ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, parse_color};
use crate::ui::layout::{centered_rect, centered_rect_fixed};

use crate::ui::*;
pub fn draw_commit_popup(
    f: &mut Frame,
    input_buffer: &str,
    editing: bool,
    commit_amend: bool,
    scroll: usize,
    area: Rect,
    app: &crate::app::App,
    areas: &mut DetailAreas,
) {
    let popup_area = if app.commit_popup_maximized {
        let width = area.width.saturating_sub(20).max(area.width.min(40));
        let height = area.height.saturating_sub(20).max(area.height.min(15));
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        Rect::new(x, y, width, height)
    } else {
        centered_rect(app.commit_popup_width_pct, app.commit_popup_height_pct, area)
    };
    areas.commit_popup = Some(popup_area);
    areas.commit_popup_parent = Some(area);
    f.render_widget(Clear, popup_area);

    let border_color = if editing { ACCENT() } else { WARNING() };
    let border_style = Style::default().fg(border_color);

    let title_text = if editing {
        if commit_amend { " Amend Commit Message " } else { " Commit Message " }
    } else {
        if commit_amend { " Confirm Amend Commit " } else { " Confirm Commit " }
    };

    let title =
        Line::from(vec![Span::raw(" "), Span::styled(title_text, primary_style()), Span::raw(" ")]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title)
        .padding(Padding::horizontal(1));

    let text = if input_buffer.is_empty() {
        Paragraph::new(Span::styled("(type commit message here...)", muted_style()))
            .wrap(Wrap { trim: true })
            .scroll((scroll as u16, 0))
    } else {
        Paragraph::new(input_buffer).wrap(Wrap { trim: true }).scroll((scroll as u16, 0))
    };

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Split inner area vertically: top is the commit message text area, bottom is the amend option.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner_area);

    f.render_widget(text, chunks[0]);

    // Render the amend checkbox.
    let checkbox = if commit_amend { "[X]" } else { "[ ]" };
    let checkbox_style = if commit_amend {
        Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)
    } else {
        muted_style()
    };
    let checkbox_line = Line::from(vec![
        Span::styled(format!("{} ", checkbox), checkbox_style),
        Span::styled("Amend last commit", primary_style()),
        if !editing {
            Span::styled(" (toggle: [a/space])", muted_style())
        } else {
            Span::styled(" (toggle: [⌃A])", muted_style())
        },
    ]);
    f.render_widget(Paragraph::new(checkbox_line), chunks[1]);

    if editing {
        let lines: Vec<&str> = input_buffer.split('\n').collect();
        let last_line = lines.last().copied().unwrap_or("");
        let line_count = lines.len();
        let cursor_y = chunks[0]
            .y
            .saturating_add(line_count.saturating_sub(1) as u16)
            .min(chunks[0].y.saturating_add(chunks[0].height.saturating_sub(1)));
        let cursor_offset = last_line.chars().count() as u16;
        let cursor_x =
            chunks[0].x.saturating_add(cursor_offset.min(chunks[0].width.saturating_sub(1)));
        f.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
    }
}

