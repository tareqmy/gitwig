//! Popup dialog rendering version and project authorship info.

use crate::app::{App, Mode};
use crate::ui::layout::centered_rect_fixed;
use crate::ui::style::{CARD_BORDER, accent_style, muted_style, primary_style};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

pub fn draw_about_popup(f: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect_fixed(70, 18, area);
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

    let logo_text = if is_compat {
        vec![
            Line::from(""), // spacer
            Line::from(Span::styled("    .-.-.", leaf_style)),
            Line::from(Span::styled("   (_\\_/_)", leaf_style)),
            Line::from(Span::styled("     | |", leaf_style)),
            Line::from(Span::styled("     | /", leaf_style)),
            Line::from(Span::styled("   .-*-/", git_style)),
            Line::from(Span::styled("  /  | \\", git_style)),
            Line::from(Span::styled(" o   |  o", git_style)),
            Line::from(Span::styled("     o", git_style)),
        ]
    } else {
        vec![
            Line::from(""), // spacer
            Line::from(Span::styled("     ╭───╮", leaf_style)),
            Line::from(Span::styled("   ╭─╯ 🌿 ╰─╮", leaf_style)),
            Line::from(Span::styled("   ╰─╮   ╭──╯", leaf_style)),
            Line::from(Span::styled("     ╰─┬─╯", leaf_style)),
            Line::from(Span::styled("       │", git_style)),
            Line::from(Span::styled("     ╭─┴─╮", git_style)),
            Line::from(Span::styled("    ╭┴─●─╯", git_style)),
            Line::from(Span::styled("   ╱   │   ╲", git_style)),
            Line::from(Span::styled("  ●    │    ●", git_style)),
            Line::from(Span::styled("       ●", git_style)),
        ]
    };

    let logo_para = Paragraph::new(logo_text).alignment(Alignment::Center);
    f.render_widget(logo_para, about_chunks[0]);

    // Split details vertically on the right
    let info_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title/version
            Constraint::Length(1), // Spacer
            Constraint::Length(5), // Metadata Table (Creator, Website, GitHub, License, Email)
            Constraint::Length(1), // Spacer
            Constraint::Length(2), // Description
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Inspired by / Credits
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

    // Metadata Details
    let metadata_lines = vec![
        Line::from(vec![
            Span::styled("Creator:  ", primary_style().add_modifier(Modifier::BOLD)),
            Span::raw("Tareq M. Yousuf "),
            Span::styled("(@tareqmy)", muted_style()),
        ]),
        Line::from(vec![
            Span::styled("Website:  ", primary_style().add_modifier(Modifier::BOLD)),
            Span::styled("https://tareqmy.com/", accent_style()),
        ]),
        Line::from(vec![
            Span::styled("GitHub:   ", primary_style().add_modifier(Modifier::BOLD)),
            Span::styled("https://github.com/tareqmy/gitwig", accent_style()),
        ]),
        Line::from(vec![
            Span::styled("License:  ", primary_style().add_modifier(Modifier::BOLD)),
            Span::styled("MIT", Style::default().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled("Email:    ", primary_style().add_modifier(Modifier::BOLD)),
            Span::styled("tareq.y@gmail.com", accent_style()),
        ]),
    ];
    f.render_widget(Paragraph::new(metadata_lines), info_chunks[2]);

    // Description
    let desc_para = Paragraph::new(vec![Line::from(Span::styled(
        "A Rust-based Git TUI, representing repository branches like twigs on a leaf.",
        muted_style(),
    ))])
    .wrap(Wrap { trim: true });
    f.render_widget(desc_para, info_chunks[4]);

    // Inspired By / Credits
    let credits_lines = vec![
        Line::from(Span::styled("Inspired by:", primary_style().add_modifier(Modifier::BOLD))),
        Line::from(vec![
            Span::raw("  • "),
            Span::styled("Sourcetree", primary_style()),
            Span::styled(" (staging & tree visualization)", muted_style()),
        ]),
        Line::from(vec![
            Span::raw("  • "),
            Span::styled("lazygit", primary_style()),
            Span::styled(" & ", muted_style()),
            Span::styled("gitui", primary_style()),
            Span::styled(" (fast modal terminal UX)", muted_style()),
        ]),
    ];
    f.render_widget(Paragraph::new(credits_lines), info_chunks[6]);

    // Close instruction
    let compat = app.config.compatibility_mode;
    let about_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::HomeAbout, compat);
    let close_key =
        app.keybindings.format_action_keys(crate::keybindings::Action::CloseDetail, compat);
    let close_line = Line::from(vec![
        Span::styled("Press ", muted_style()),
        Span::styled(close_key, accent_style()),
        Span::styled(" / ", muted_style()),
        Span::styled(about_key, accent_style()),
        Span::styled(" to close", muted_style()),
    ]);
    f.render_widget(Paragraph::new(close_line), info_chunks[8]);
}

use crossterm::event::{KeyCode, KeyEvent};
pub struct AboutPopup;
impl AboutPopup {
    pub fn handle_event(app: &mut crate::app::App, key: KeyEvent) -> bool {
        if app.is_bound(crate::keybindings::Action::HomeAbout, key)
            || app.is_bound(crate::keybindings::Action::CloseDetail, key)
        {
            app.close_dialog();
            return true;
        }
        false
    }
}
