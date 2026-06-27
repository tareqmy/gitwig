use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect, Margin};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap, Padding, Gauge, List, ListItem};
use crate::app::{App, Mode};
use crate::ui::style::{accent_style, muted_style, primary_style, ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, parse_color};
use crate::ui::layout::{centered_rect, centered_rect_fixed};

pub fn draw_about_popup(f: &mut Frame, area: Rect, _app: &App) {
    let popup_area = centered_rect_fixed(60, 15, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(accent_style())
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("About Gitwig", primary_style()),
                Span::raw(" "),
            ])
            .alignment(Alignment::Center),
        );

    f.render_widget(block.clone(), popup_area);
    let inner = block.inner(popup_area);

    let info_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title/version
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Creator
            Constraint::Length(1), // Website
            Constraint::Length(1), // GitHub
            Constraint::Length(1), // Email
            Constraint::Length(1), // Spacer
            Constraint::Length(2), // Description
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Close instructions
            Constraint::Min(0),
        ])
        .split(inner);

    // Title / Version
    let title_line = Line::from(vec![
        Span::styled("Gitwig v", primary_style().add_modifier(Modifier::BOLD)),
        Span::styled(env!("CARGO_PKG_VERSION"), primary_style().add_modifier(Modifier::BOLD)),
    ])
    .alignment(Alignment::Center);
    f.render_widget(Paragraph::new(title_line), info_chunks[0]);

    // Creator profile details
    let creator_line = Line::from(vec![
        Span::styled("  Creator:  ", primary_style().add_modifier(Modifier::BOLD)),
        Span::raw("Tareq Mohammad Yousuf "),
        Span::styled("(@tareqmy)", muted_style()),
    ]);
    f.render_widget(Paragraph::new(creator_line), info_chunks[2]);

    let website_line = Line::from(vec![
        Span::styled("  Website:  ", primary_style().add_modifier(Modifier::BOLD)),
        Span::styled("https://tareqmy.com/", accent_style()),
    ]);
    f.render_widget(Paragraph::new(website_line), info_chunks[3]);

    let github_line = Line::from(vec![
        Span::styled("  GitHub:   ", primary_style().add_modifier(Modifier::BOLD)),
        Span::styled("https://github.com/tareqmy", accent_style()),
    ]);
    f.render_widget(Paragraph::new(github_line), info_chunks[4]);

    let email_line = Line::from(vec![
        Span::styled("  Email:    ", primary_style().add_modifier(Modifier::BOLD)),
        Span::styled("tareq.y@gmail.com", accent_style()),
    ]);
    f.render_widget(Paragraph::new(email_line), info_chunks[5]);

    // Description
    let desc_para = Paragraph::new(vec![
        Line::from(Span::styled(
            "A Rust-based Git TUI, alternative to SourceTree and gitui.",
            muted_style(),
        ))
        .alignment(Alignment::Center),
    ])
    .wrap(Wrap { trim: true });
    f.render_widget(desc_para, info_chunks[7]);

    // Close instruction
    let close_line = Line::from(vec![
        Span::styled("Press ", muted_style()),
        Span::styled("Esc", accent_style()),
        Span::styled(" / ", muted_style()),
        Span::styled("q", accent_style()),
        Span::styled(" / ", muted_style()),
        Span::styled("v", accent_style()),
        Span::styled(" to close", muted_style()),
    ])
    .alignment(Alignment::Center);
    f.render_widget(Paragraph::new(close_line), info_chunks[9]);
}

