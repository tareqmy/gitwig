//! Embedded terminal panel: renders the PTY screen kept by
//! `crate::terminal_session::TerminalSession` into a bottom panel.

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::ui::{ACCENT, CARD_BORDER, muted_style, primary_style};

/// Draw the terminal panel into `area` (borders included).
pub fn draw_terminal_panel(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.terminal_focused;
    let border_style = if focused { Style::default().fg(ACCENT()) } else { muted_style() };

    let mut title_spans = vec![Span::raw(" "), Span::styled("Terminal", primary_style())];
    let mut scrollback_rows = 0;
    if let Some(session) = &app.terminal_panel.session {
        title_spans.push(Span::styled(format!(" — {}", session.repo_label), muted_style()));
        if !session.exited() {
            scrollback_rows = session.lock_parser().screen().scrollback();
        }
    }
    if scrollback_rows > 0 {
        title_spans.push(Span::styled(format!(" [+{}]", scrollback_rows), muted_style()));
    }
    title_spans.push(Span::raw(" "));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(border_style)
        .title(Line::from(title_spans));
    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let Some(session) = app.terminal_panel.session.as_ref() else {
        return;
    };

    if session.exited() {
        let code = session.exit_code().map(|c| format!(" (exit code {})", c)).unwrap_or_default();
        let msg = format!("shell exited{} — press any key to close", code);
        let vertical_pad = inner.height / 2;
        let msg_area = Rect::new(inner.x, inner.y + vertical_pad, inner.width, 1);
        f.render_widget(
            Paragraph::new(msg).style(muted_style()).alignment(Alignment::Center),
            msg_area,
        );
        return;
    }

    let guard = session.lock_parser();
    let screen = guard.screen();
    let cursor = tui_term::widget::Cursor::default()
        .visibility(focused && !screen.hide_cursor() && scrollback_rows == 0);
    let widget = tui_term::widget::PseudoTerminal::new(screen).cursor(cursor);
    f.render_widget(widget, inner);
}
