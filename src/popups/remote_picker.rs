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
pub fn draw_remote_picker_popup(f: &mut Frame, remotes: &[RemoteInfo], selection: usize, area: Rect) {
    let popup_area = centered_rect(50, 60, area);

    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled("Select Remote", primary_style()),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    // Split inner: list on top, hint at bottom.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(inner);

    let items: Vec<ListItem> = remotes
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let style = if i == selection {
                accent_style().add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                primary_style()
            };
            ListItem::new(Line::from(vec![
                Span::raw("  "),
                Span::styled(r.name.clone(), style),
                Span::styled("  ", muted_style()),
                Span::styled(r.url.clone(), muted_style()),
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(selection));
    f.render_stateful_widget(List::new(items), chunks[0], &mut list_state);

    let hint = Line::from(vec![
        Span::styled("↑↓ navigate  ", muted_style()),
        Span::styled("Enter", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" confirm  ", muted_style()),
        Span::styled("Esc", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" cancel", muted_style()),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[1]);
}

