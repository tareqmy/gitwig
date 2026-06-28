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

pub fn draw_about_popup(f: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect_fixed(66, 17, area);
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

    // Split horizontally: Left for ASCII Logo, Right for Info details
    let about_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(18), // Left logo pane
            Constraint::Length(1),  // Spacer
            Constraint::Min(0),     // Right details pane
        ])
        .split(inner);

    // Draw ASCII Leaf / Git Twig Logo on the left
    let is_compat = app.config.compatibility_mode;
    let leaf_style = if is_compat { Style::default() } else { Style::default().fg(Color::Green) };
    let git_style = if is_compat { Style::default() } else { accent_style() };

    let logo_text = vec![
        Line::from(""), // spacer
        Line::from(Span::styled("    .-.-.", leaf_style)),
        Line::from(Span::styled("   (_\\_/_)", leaf_style)),
        Line::from(Span::styled("     | |", leaf_style)),
        Line::from(Span::styled("     | /", leaf_style)),
        Line::from(Span::styled("   .-*-/", git_style)),
        Line::from(Span::styled("  /  | \\", git_style)),
        Line::from(Span::styled(" o   |  o", git_style)),
        Line::from(Span::styled("     o", git_style)),
    ];

    let logo_para = Paragraph::new(logo_text).alignment(Alignment::Center);
    f.render_widget(logo_para, about_chunks[0]);

    // Split details vertically on the right
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
            Constraint::Length(3), // Description
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Close instructions
            Constraint::Min(0),
        ])
        .split(about_chunks[2]);

    // Title / Version
    let title_line = Line::from(vec![
        Span::styled("Gitwig v", primary_style().add_modifier(Modifier::BOLD)),
        Span::styled(env!("CARGO_PKG_VERSION"), primary_style().add_modifier(Modifier::BOLD)),
    ]);
    f.render_widget(Paragraph::new(title_line), info_chunks[0]);

    // Creator details
    let creator_line = Line::from(vec![
        Span::styled("Creator:  ", primary_style().add_modifier(Modifier::BOLD)),
        Span::raw("Tareq M. Yousuf "),
        Span::styled("(@tareqmy)", muted_style()),
    ]);
    f.render_widget(Paragraph::new(creator_line), info_chunks[2]);

    let website_line = Line::from(vec![
        Span::styled("Website:  ", primary_style().add_modifier(Modifier::BOLD)),
        Span::styled("https://tareqmy.com/", accent_style()),
    ]);
    f.render_widget(Paragraph::new(website_line), info_chunks[3]);

    let github_line = Line::from(vec![
        Span::styled("GitHub:   ", primary_style().add_modifier(Modifier::BOLD)),
        Span::styled("https://github.com/tareqmy", accent_style()),
    ]);
    f.render_widget(Paragraph::new(github_line), info_chunks[4]);

    let email_line = Line::from(vec![
        Span::styled("Email:    ", primary_style().add_modifier(Modifier::BOLD)),
        Span::styled("tareq.y@gmail.com", accent_style()),
    ]);
    f.render_widget(Paragraph::new(email_line), info_chunks[5]);

    // Description
    let desc_para = Paragraph::new(vec![Line::from(Span::styled(
        "A Rust-based Git TUI, representing repository branches like twigs on a leaf.",
        muted_style(),
    ))])
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
    ]);
    f.render_widget(Paragraph::new(close_line), info_chunks[9]);
}

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
pub struct AboutPopup;
impl AboutPopup {
    pub fn handle_event(app: &mut crate::app::App, key: KeyEvent) -> bool {
        let code = key.code;
        match code {
            KeyCode::Char('v')
            | KeyCode::Char('V')
            | KeyCode::Esc
            | KeyCode::Char('q')
            | KeyCode::Char('Q') => {
                app.close_dialog();
            }
            _ => {}
        }
        false
    }
}
