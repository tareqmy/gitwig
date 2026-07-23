use crate::app::{App, Mode};
use crate::ui::layout::centered_rect_fixed;
use crate::ui::style::{CARD_BORDER, accent_style, muted_style, primary_style};
use chrono::{Datelike, Duration, Local};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub struct StatsPopup;

impl StatsPopup {
    pub fn draw(f: &mut Frame, app: &App, area: Rect) {
        // We increase the popup width to 80 to fit 52 weeks (52 columns * 1 chars) + labels + padding.
        // And height to 25 to fit the stats list (9 lines) + Heatmap (7 lines + title) + spacing.
        let popup_area = centered_rect_fixed(80, 25, area);
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
                Constraint::Length(9), // Stats list
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Heatmap Title
                Constraint::Length(7), // Heatmap
                Constraint::Min(0),    // Flexible space
                Constraint::Length(1), // Footer instruction
            ])
            .margin(1)
            .split(inner);

        let session_duration = std::time::Instant::now().duration_since(app.session_start).as_secs();
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

        // Heatmap title
        let heatmap_title = Paragraph::new(Line::from(vec![Span::styled(
            "Activity Heatmap (Last 52 Weeks)",
            Style::default().add_modifier(Modifier::BOLD),
        )]))
        .alignment(Alignment::Left);
        f.render_widget(heatmap_title, chunks[4]);

        // Heatmap Grid Generator
        let today = Local::now().date_naive();
        // A week starts on Sunday (num_days_from_sunday = 0..=6)
        let days_in_last_week = today.weekday().num_days_from_sunday() + 1;
        let total_days = (51 * 7) + days_in_last_week;
        let start_date = today - Duration::days((total_days - 1) as i64);

        let mut grid: Vec<Vec<Span>> = vec![vec![]; 7];

        for i in 0..total_days {
            let current_date = start_date + Duration::days(i as i64);
            let row = current_date.weekday().num_days_from_sunday() as usize;

            let date_str = current_date.format("%Y-%m-%d").to_string();
            let count = app.stats.daily_activity.get(&date_str).copied().unwrap_or(0);

            let span = if count == 0 {
                Span::styled("·", muted_style())
            } else if count <= 2 {
                Span::styled("░", Style::default().fg(crate::ui::style::SUCCESS()))
            } else if count <= 5 {
                Span::styled("▒", Style::default().fg(crate::ui::style::SUCCESS()))
            } else if count <= 10 {
                Span::styled("▓", Style::default().fg(crate::ui::style::SUCCESS()))
            } else {
                Span::styled("█", Style::default().fg(crate::ui::style::SUCCESS()))
            };

            grid[row].push(span);
        }

        // Pad last column if needed so all rows have exactly 52 items
        for row in 0..7 {
            if grid[row].len() < 52 {
                grid[row].push(Span::raw(" "));
            }
        }

        let labels = ["    ", "Mon ", "    ", "Wed ", "    ", "Fri ", "    "];
        let mut heatmap_lines = Vec::new();

        for row in 0..7 {
            let mut spans = vec![Span::styled(labels[row], muted_style())];
            for col_span in &grid[row] {
                spans.push(col_span.clone());
            }
            heatmap_lines.push(Line::from(spans));
        }

        let heatmap = Paragraph::new(heatmap_lines).alignment(Alignment::Left);
        f.render_widget(heatmap, chunks[5]);

        let help_text = Paragraph::new(Line::from(vec![
            Span::styled("esc", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / ", muted_style()),
            Span::styled("q", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" / ", muted_style()),
            Span::styled("enter", accent_style().add_modifier(Modifier::BOLD)),
            Span::raw(" to close"),
        ]))
        .alignment(Alignment::Center);

        f.render_widget(help_text, chunks[7]);
    }
}
