use crate::app::{App, Mode};
use crate::ui::layout::centered_rect_fixed;
use crate::ui::style::{CARD_BORDER, accent_style, muted_style, primary_style};
use chrono::Utc;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub struct StatsPopup;

impl StatsPopup {
    pub fn draw(f: &mut Frame, app: &App, area: Rect) {
        let popup_area = centered_rect_fixed(60, 20, area);
        f.render_widget(Clear, popup_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER())
            .border_style(accent_style())
            .title(
                Line::from(vec![
                    Span::raw(" "),
                    Span::styled("App Usage Statistics", primary_style()),
                    Span::raw(" "),
                ])
                .alignment(Alignment::Center),
            );

        f.render_widget(block.clone(), popup_area);
        let inner = block.inner(popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title
                Constraint::Length(1), // Spacer
                Constraint::Min(0),    // Stats list
                Constraint::Length(1), // Footer instruction
            ])
            .margin(1)
            .split(inner);

        let session_duration =
            std::time::Instant::now().duration_since(app.session_start).as_secs();
        let total_time_secs = app.stats.total_duration_secs + session_duration;
        let hours = total_time_secs / 3600;
        let minutes = (total_time_secs % 3600) / 60;

        let lines = vec![
            Line::from(vec![
                Span::styled("Total Time Spent: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}h {}m", hours, minutes)),
            ]),
            Line::from(vec![
                Span::styled("Commits Authored: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", app.stats.commits_made)),
            ]),
            Line::from(vec![
                Span::styled("Merges: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", app.stats.merges)),
            ]),
            Line::from(vec![
                Span::styled("Branches Created: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", app.stats.branches_created)),
            ]),
            Line::from(vec![
                Span::styled("Branches Deleted: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", app.stats.branches_deleted)),
            ]),
            Line::from(vec![
                Span::styled("Stashes: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", app.stats.stashes)),
            ]),
            Line::from(vec![
                Span::styled("Pulls: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", app.stats.pulls)),
            ]),
            Line::from(vec![
                Span::styled("Pushes: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", app.stats.pushes)),
            ]),
            Line::from(vec![
                Span::styled("Fetches: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}", app.stats.fetches)),
            ]),
        ];

        let content = Paragraph::new(lines).alignment(Alignment::Left);
        f.render_widget(content, chunks[2]);

        let help_text = Paragraph::new(Line::from(vec![
            Span::styled("esc", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / ", muted_style()),
            Span::styled("q", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / ", muted_style()),
            Span::styled("enter", accent_style().add_modifier(Modifier::BOLD)),
            Span::raw(" to close"),
        ]))
        .alignment(Alignment::Center);

        f.render_widget(help_text, chunks[3]);
    }
}
