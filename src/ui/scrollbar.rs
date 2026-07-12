//! Scrollbar drawing utility helper for TUI panels.

use crate::ui::style::ACCENT;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

pub fn draw_vertical_scrollbar(
    f: &mut Frame,
    area: Rect,
    scroll: usize,
    content_length: usize,
    viewport_length: usize,
) {
    if content_length > viewport_length {
        let mut scrollbar_state = ScrollbarState::new(content_length)
            .position(scroll)
            .viewport_content_length(viewport_length);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .thumb_style(Style::default().fg(ACCENT()));
        f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}
