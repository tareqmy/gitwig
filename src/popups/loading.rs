//! Background loading status overlay for blocking async Git commands.

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

pub fn draw_loading_screen(f: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect_fixed(60, 7, area);

    let border_style = Style::default().fg(ACCENT());

    let loading_msg = if let Some(path) = &app.loading_repo_path {
        let repo_name =
            std::path::Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or(path);
        format!("Loading repository '{}'...", repo_name)
    } else {
        "Loading repository...".to_string()
    };

    let spinner_chars = if app.config.compatibility_mode {
        vec!["|", "/", "-", "\\"]
    } else {
        vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
    };

    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let spinner = spinner_chars[(millis / 100) as usize % spinner_chars.len()];

    let title = Line::from(vec![
        Span::raw(" ⌛ "),
        Span::styled("Please Wait", Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
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
            Constraint::Length(1), // spacer
            Constraint::Length(1), // message + spinner
            Constraint::Length(1), // spacer
            Constraint::Length(1), // cancel hint
        ])
        .split(inner);

    let text_line = Line::from(vec![
        Span::raw(format!("{}  ", loading_msg)),
        Span::styled(spinner, Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
    ])
    .alignment(Alignment::Center);

    let cancel_line = Line::from(vec![
        Span::styled("Press ", muted_style()),
        Span::styled("Esc", Style::default().fg(WARNING())),
        Span::styled(" to cancel", muted_style()),
    ])
    .alignment(Alignment::Center);

    f.render_widget(Paragraph::new(text_line), chunks[1]);
    f.render_widget(Paragraph::new(cancel_line), chunks[3]);
}

pub fn draw_progress_popup(f: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect_fixed(50, 7, area);
    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(ACCENT());
    let title = Line::from(vec![
        Span::raw(" ⇆ "),
        Span::styled("Network Sync", Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
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
            Constraint::Length(1), // status text
            Constraint::Length(1), // spacer
            Constraint::Length(1), // gauge
            Constraint::Length(1), // spacer
            Constraint::Length(1), // dismiss hint
        ])
        .split(inner);

    let spinner = if app.config.compatibility_mode {
        ["-", "\\", "|", "/"][((app.fetch_progress / 5) % 4) as usize]
    } else {
        let spinner_idx = ((app.fetch_progress / 5) % 10) as usize;
        ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"][spinner_idx]
    };

    let status_text = app.status_message.as_deref().unwrap_or("Executing Git network operation...");
    let status_para = Paragraph::new(Line::from(vec![
        Span::styled(spinner, Style::default().fg(ACCENT()).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(status_text, primary_style()),
    ]));
    f.render_widget(status_para, chunks[0]);

    let gauge = Gauge::default()
        .block(Block::default().padding(Padding::ZERO))
        .gauge_style(Style::default().fg(ACCENT()))
        .style(muted_style())
        .percent(app.fetch_progress)
        .use_unicode(true);
    f.render_widget(gauge, chunks[2]);

    let hint = Paragraph::new(Line::from(vec![
        Span::styled("Press ", muted_style()),
        Span::styled("Esc", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
        Span::styled(" to dismiss if stuck", muted_style()),
    ]))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(hint, chunks[4]);
}
