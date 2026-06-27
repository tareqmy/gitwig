use crate::app::{App, Mode};
use crate::repo::RemoteInfo;
use crate::ui::HELP_LINES;
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

pub fn draw_help_overlay(f: &mut Frame, app: &App, area: Rect, scroll: usize) {
    let popup_area = centered_rect(60, 70, area);

    let key_width = HELP_LINES
        .iter()
        .map(|(k, _)| {
            let mut display_key = k.to_string();
            if app.config.compatibility_mode {
                display_key = display_key
                    .replace("↑", "^")
                    .replace("↓", "v")
                    .replace("⇟", "PgDn")
                    .replace("⇞", "PgUp")
                    .replace("↵", "Enter")
                    .replace("→", ">")
                    .replace("⎋", "Esc")
                    .replace("⌫", "Backspace")
                    .replace("⇥", "Tab")
                    .replace("⇧⇥", "Shift+Tab")
                    .replace("⇧", "Shift+");
            }
            display_key.chars().count()
        })
        .max()
        .unwrap_or(0);

    let mut lines: Vec<Line> = Vec::with_capacity(HELP_LINES.len() + 8);
    lines.push(Line::from(""));
    for (key, desc) in HELP_LINES {
        let mut display_key = key.to_string();
        if app.config.compatibility_mode {
            display_key = display_key
                .replace("↑", "^")
                .replace("↓", "v")
                .replace("⇟", "PgDn")
                .replace("⇞", "PgUp")
                .replace("↵", "Enter")
                .replace("→", ">")
                .replace("⎋", "Esc")
                .replace("⌫", "Backspace")
                .replace("⇥", "Tab")
                .replace("⇧⇥", "Shift+Tab")
                .replace("⇧", "Shift+");
        }
        let padded_key = format!("{:>width$}", display_key, width = key_width);
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(padded_key, accent_style()),
            Span::raw("   "),
            Span::raw((*desc).to_string()),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Status indicators", primary_style()),
    ]));
    let pad_symbol = |sym: &str, color: Color, label: &'static str, desc: &'static str| {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(sym.to_string(), Style::default().fg(color)),
            Span::raw(" "),
            Span::styled(format!("{:<8}", label), muted_style()),
            Span::raw(desc),
        ])
    };
    lines.push(pad_symbol(
        app.sym("bullet_filled"),
        SUCCESS(),
        "git",
        "Directory is a git repository",
    ));
    lines.push(pad_symbol(
        app.sym("bullet_empty"),
        WARNING(),
        "dir",
        "Directory exists but is not a git repo",
    ));
    lines.push(pad_symbol(
        app.sym("close"),
        DANGER(),
        "missing",
        "Path does not exist or is not a directory",
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Repo state suffixes", primary_style()),
    ]));
    let pad_suffix = |sym: &str, style: Style, desc: &'static str| {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("N{}", sym), style),
            Span::raw("        "),
            Span::raw(desc),
        ])
    };
    lines.push(pad_suffix("+", Style::default().fg(ACCENT()), "files staged for commit"));
    lines.push(pad_suffix("!", Style::default().fg(WARNING()), "files modified but not staged"));
    lines.push(pad_suffix("?", muted_style(), "untracked files"));
    lines.push(pad_suffix(app.sym("up"), primary_style(), "commits ahead of upstream (need push)"));
    lines.push(pad_suffix(
        app.sym("down"),
        Style::default().fg(WARNING()),
        "commits behind upstream",
    ));
    lines.push(Line::from(""));

    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(Style::default().fg(ACCENT()))
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("Shortcuts", accent_style()),
                Span::raw(" "),
            ])
            .alignment(Alignment::Left),
        )
        .padding(Padding::horizontal(1));

    let inner_height = popup_area.height.saturating_sub(2) as usize;
    let max_scroll = lines.len().saturating_sub(inner_height);
    let scroll = scroll.min(max_scroll);

    let lines_len = lines.len();
    let help = Paragraph::new(lines)
        .block(help_block)
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));

    // Clear wipes the underlying cells so the list doesn't bleed through.
    f.render_widget(Clear, popup_area);
    f.render_widget(help, popup_area);

    if max_scroll > 0 {
        let mut scrollbar_state = ratatui::widgets::ScrollbarState::new(lines_len)
            .position(scroll)
            .viewport_content_length(inner_height);
        let scrollbar =
            ratatui::widgets::Scrollbar::new(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .thumb_style(Style::default().fg(ACCENT()));
        f.render_stateful_widget(scrollbar, popup_area, &mut scrollbar_state);
    }
}

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
pub struct HelpPopup;
impl HelpPopup {
    pub fn handle_event(app: &mut crate::app::App, key: KeyEvent) -> bool {
        let code = key.code;
        match code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.close_dialog();
            }
            KeyCode::Up => {
                app.help_scroll_up();
            }
            KeyCode::Down => {
                app.help_scroll_down();
            }
            KeyCode::PageUp => {
                app.help_scroll_page_up(app.config.page_size);
            }
            KeyCode::PageDown => {
                app.help_scroll_page_down(app.config.page_size);
            }
            KeyCode::Home => {
                app.help_scroll_to_top();
            }
            KeyCode::End => {
                app.help_scroll_to_bottom();
            }
            _ => {}
        }
        false
    }
}

pub struct DetailHelpPopup;
impl DetailHelpPopup {
    pub fn handle_event(app: &mut crate::app::App, key: KeyEvent) -> bool {
        let code = key.code;
        match code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.close_detail_help();
            }
            KeyCode::Up => {
                app.help_scroll_up();
            }
            KeyCode::Down => {
                app.help_scroll_down();
            }
            KeyCode::PageUp => {
                app.help_scroll_page_up(app.config.page_size);
            }
            KeyCode::PageDown => {
                app.help_scroll_page_down(app.config.page_size);
            }
            KeyCode::Home => {
                app.help_scroll_to_top();
            }
            KeyCode::End => {
                app.help_scroll_to_bottom();
            }
            _ => {}
        }
        false
    }
}
