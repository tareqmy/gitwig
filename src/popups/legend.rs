use crate::app::App;
use crate::ui::layout::centered_rect_fixed;
use crate::ui::style::{
    CARD_BORDER, DANGER, SUCCESS, WARNING, accent_style, muted_style, primary_style,
};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn draw_legend_popup(f: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect_fixed(72, 20, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(accent_style())
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("Signs & Symbols Legend", primary_style()),
                Span::raw(" "),
            ])
            .alignment(Alignment::Center),
        );

    f.render_widget(block.clone(), popup_area);
    let inner = block.inner(popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Subtitle
            Constraint::Length(1), // Spacer
            Constraint::Min(0),    // Legend table/columns
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Close instructions
        ])
        .split(inner);

    let subtitle = Line::from(Span::styled(
        "A guide to status indicators and badges used in Gitwig",
        muted_style(),
    ))
    .alignment(Alignment::Center);
    f.render_widget(Paragraph::new(subtitle), chunks[0]);

    // Split table horizontally: left for status indicators, right for repo states
    let table_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .split(chunks[2]);

    // Left Column: Status Indicators
    let left_lines = vec![
        Line::from(Span::styled("Status Indicators (Homepage)", primary_style())),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!(" {} clean  ", app.sym("bullet_filled")),
                Style::default().fg(SUCCESS()),
            ),
            Span::raw("Repo in sync (no changes)"),
        ]),
        Line::from(vec![
            Span::styled(
                " N+        ",
                Style::default().fg(accent_style().fg.unwrap_or(Color::Cyan)),
            ),
            Span::raw("N Staged changes"),
        ]),
        Line::from(vec![
            Span::styled(" N!        ", Style::default().fg(WARNING())),
            Span::raw("N Modified changes"),
        ]),
        Line::from(vec![
            Span::styled(" N?        ", muted_style()),
            Span::raw("N Untracked files"),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" N{}        ", app.sym("action").trim()),
                Style::default().fg(DANGER()),
            ),
            Span::raw("N Conflicted files"),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" N{}        ", app.sym("up")),
                Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
            ),
            Span::raw("N Commits ahead of remote"),
        ]),
        Line::from(vec![
            Span::styled(
                format!(" N{}        ", app.sym("down")),
                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
            ),
            Span::raw("N Commits behind remote"),
        ]),
    ];
    f.render_widget(Paragraph::new(left_lines), table_chunks[0]);

    // Right Column: Repository States & General
    let right_lines = vec![
        Line::from(Span::styled("Repository States / Badges", primary_style())),
        Line::from(""),
        Line::from(vec![
            Span::styled(" ✓ CLEAN    ", muted_style()),
            Span::raw("No active git state/operation"),
        ]),
        Line::from(vec![
            Span::styled(
                " ⚠ MERGE    ",
                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Active Merge session (conflicts)"),
        ]),
        Line::from(vec![
            Span::styled(
                " 🚧 REBASE   ",
                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Active Interactive/Normal Rebase"),
        ]),
        Line::from(vec![
            Span::styled(
                " ⚡ CHERRY   ",
                Style::default()
                    .fg(accent_style().fg.unwrap_or(Color::Cyan))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("Active Cherry-pick operation"),
        ]),
        Line::from(vec![
            Span::styled(
                " ⚡ REVERT   ",
                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Active Revert operation"),
        ]),
        Line::from(vec![
            Span::styled(
                " 🔍 BISECT   ",
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Active Bisect session"),
        ]),
        Line::from(vec![
            Span::styled(
                " 📬 APPLY    ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Applying patches (mailbox)"),
        ]),
    ];
    f.render_widget(Paragraph::new(right_lines), table_chunks[1]);

    let close_line = Line::from(vec![
        Span::raw("Press "),
        Span::styled("Esc", accent_style()),
        Span::raw(" or "),
        Span::styled("q", accent_style()),
        Span::raw(" or "),
        Span::styled("h", accent_style()),
        Span::raw(" to close"),
    ])
    .alignment(Alignment::Center);
    f.render_widget(Paragraph::new(close_line), chunks[4]);
}

use crossterm::event::{KeyCode, KeyEvent};
pub struct LegendPopup;
impl LegendPopup {
    pub fn handle_event(app: &mut crate::app::App, key: KeyEvent) -> bool {
        let code = key.code;
        match code {
            KeyCode::Char('h')
            | KeyCode::Char('H')
            | KeyCode::Esc
            | KeyCode::Char('q')
            | KeyCode::Char('Q') => {
                app.close_dialog();
                return true;
            }
            _ => {}
        }
        false
    }
}
