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

pub fn get_legend_lines_len(app: &App) -> usize {
    get_legend_lines(app).len()
}

fn get_legend_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    // Section 1: Status Indicators
    lines.push(Line::from(Span::styled(
        "Status Indicators (Homepage)",
        primary_style().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(format!("  {:<12}", app.sym("pinned")), Style::default().fg(SUCCESS())),
        Span::raw("Pinned repository"),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!("  {:<12}", app.sym("star")), Style::default().fg(Color::Yellow)),
        Span::raw("Starred / Favorite repository"),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {:<12}", app.sym("bullet_filled").to_string() + " clean"),
            Style::default().fg(SUCCESS()),
        ),
        Span::raw("Repo in sync (no changes)"),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {:<12}", app.sym("bullet_empty").to_string() + " dir"),
            Style::default().fg(WARNING()),
        ),
        Span::raw("Directory exists but is not a git repo"),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {:<12}", app.sym("close").to_string() + " missing"),
            Style::default().fg(DANGER()),
        ),
        Span::raw("Path does not exist or is not a directory"),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "  N+          ",
            Style::default().fg(accent_style().fg.unwrap_or(Color::Cyan)),
        ),
        Span::raw("N Staged changes"),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  N!          ", Style::default().fg(WARNING())),
        Span::raw("N Modified changes"),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  N?          ", muted_style()),
        Span::raw("N Untracked files"),
    ]));
    lines.push(Line::from(vec![
        Span::styled(format!("  N{:<11}", app.sym("action").trim()), Style::default().fg(DANGER())),
        Span::raw("N Conflicted files"),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            format!("  N{:<11}", app.sym("up")),
            Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("N Commits ahead of remote"),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            format!("  N{:<11}", app.sym("down")),
            Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("N Commits behind remote"),
    ]));

    // Separator
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    // Section 2: Repository States
    lines.push(Line::from(Span::styled(
        "Repository States / Badges",
        primary_style().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  ✓ CLEAN     ", muted_style()),
        Span::raw("No active git state/operation"),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  ⚠ MERGE     ", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
        Span::raw("Active Merge session (conflicts)"),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "  🚧 REBASE    ",
            Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("Active Interactive/Normal Rebase"),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "  ⚡ CHERRY    ",
            Style::default()
                .fg(accent_style().fg.unwrap_or(Color::Cyan))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("Active Cherry-pick operation"),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "  ⚡ REVERT    ",
            Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
        ),
        Span::raw("Active Revert operation"),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "  🔍 BISECT    ",
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        ),
        Span::raw("Active Bisect session"),
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "  📬 APPLY     ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::raw("Applying patches (mailbox)"),
    ]));

    lines
}

pub fn draw_legend_popup(f: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect_fixed(80, 20, area);
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
            Constraint::Min(0),    // Legend list (scrollable)
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

    let lines = get_legend_lines(app);
    let list_para = Paragraph::new(lines).scroll((app.legend_scroll as u16, 0));
    f.render_widget(list_para, chunks[2]);

    let close_line = Line::from(vec![
        Span::raw("Press "),
        Span::styled("Esc", accent_style()),
        Span::raw(" or "),
        Span::styled("q", accent_style()),
        Span::raw(" or "),
        Span::styled("h", accent_style()),
        Span::raw(" to close (Use ↑/↓/PgUp/PgDn to scroll)"),
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
            KeyCode::Up | KeyCode::Char('k') => {
                app.legend_scroll_up();
                return true;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.legend_scroll_down();
                return true;
            }
            KeyCode::PageUp => {
                app.legend_scroll_page_up(app.config.page_size);
                return true;
            }
            KeyCode::PageDown => {
                app.legend_scroll_page_down(app.config.page_size);
                return true;
            }
            KeyCode::Home => {
                app.legend_scroll_to_top();
                return true;
            }
            KeyCode::End => {
                app.legend_scroll_to_bottom();
                return true;
            }
            _ => {}
        }
        false
    }
}
