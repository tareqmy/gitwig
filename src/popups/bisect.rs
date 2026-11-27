//! Interactive Git Bisect popup overlay rendering.

use crate::app::App;
use crate::repo::RepoInfo;
use crate::ui::layout::centered_rect;
use crate::ui::style::{
    ACCENT, CARD_BORDER, DANGER, SUCCESS, WARNING, accent_style, muted_style, primary_style,
};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap};

pub fn draw_bisect_popup(f: &mut Frame, info: &RepoInfo, app: &App, area: Rect) {
    let popup_area = centered_rect(72, 72, area);
    f.render_widget(Clear, popup_area);

    let bisect = &info.bisect;
    let title_spans = if bisect.active {
        vec![
            Span::raw(" "),
            Span::styled("Git Bisect", accent_style().add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled("🔍 ACTIVE", Style::default().fg(WARNING()).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]
    } else {
        vec![
            Span::raw(" "),
            Span::styled("Git Bisect", accent_style().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]
    };

    let border_color = if bisect.active { WARNING() } else { ACCENT() };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER())
        .border_style(Style::default().fg(border_color))
        .title(Line::from(title_spans))
        .padding(Padding::horizontal(1));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let selected_commit = app.get_selected_commit();

    if !bisect.active {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);

        let mut lines = vec![
            Line::from(Span::styled(
                "Binary search commit history to pinpoint the commit that introduced a bug.",
                primary_style(),
            )),
            Line::from(""),
        ];

        if let Some(c) = selected_commit {
            lines.push(Line::from(vec![
                Span::styled("Selected commit: ", muted_style()),
                Span::styled(c.id.clone(), Style::default().fg(WARNING())),
                Span::raw(" "),
                Span::styled(c.summary.clone(), primary_style()),
            ]));
            lines.push(Line::from(""));
        }

        if let Some(ref msg) = app.bisect_last_message {
            lines.push(Line::from(vec![
                Span::styled("Last update: ", muted_style()),
                Span::styled(msg.clone(), accent_style()),
            ]));
            lines.push(Line::from(""));
        }

        lines
            .push(Line::from(Span::styled("Actions", accent_style().add_modifier(Modifier::BOLD))));
        lines.push(Line::from(vec![
            Span::styled("  s  ", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled("Start bisect session (wait for manual good/bad bounds)", primary_style()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  b  ", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
            Span::styled("Start bisect and mark current HEAD as bad", primary_style()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  g  ", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)),
            Span::styled(
                if let Some(c) = selected_commit {
                    format!("Start bisect: HEAD bad, {} good", c.id)
                } else {
                    "Start bisect: HEAD bad, selected commit good (select a commit first)"
                        .to_string()
                },
                primary_style(),
            ),
        ]));

        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), chunks[0]);
        let hint = Line::from(vec![
            Span::styled("s", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" start  ", muted_style()),
            Span::styled("b", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" start+bad  ", muted_style()),
            Span::styled("g", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" start+selected good  ", muted_style()),
            Span::styled("Esc/q", accent_style().add_modifier(Modifier::BOLD)),
            Span::styled(" close", muted_style()),
        ]);
        f.render_widget(Paragraph::new(hint), chunks[1]);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Status summary
            Constraint::Min(3),    // Scrollable bisect log
            Constraint::Length(1), // Footer hint
        ])
        .split(inner);

    let start_ref = bisect.start_ref.as_deref().unwrap_or("HEAD");
    let head_short = bisect.current_short_oid.as_deref().unwrap_or("-");
    let head_summary = bisect.current_summary.as_deref().unwrap_or("(no commit)");
    let bad_short: String =
        bisect.bad_oid.as_deref().map(|s| s.chars().take(7).collect()).unwrap_or("-".to_string());

    let progress_line = if let Some(ref culprit) = bisect.culprit {
        Line::from(vec![
            Span::styled(
                "First bad commit: ",
                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                culprit.clone(),
                Style::default().fg(DANGER()).add_modifier(Modifier::BOLD),
            ),
        ])
    } else if let (Some(revs), Some(steps)) = (bisect.remaining_revisions, bisect.remaining_steps) {
        Line::from(vec![
            Span::styled("Progress:         ", muted_style()),
            Span::styled(
                format!("~{} revisions remaining (~{} steps)", revs, steps),
                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled("Progress:         ", muted_style()),
            Span::styled("Waiting for both a good and bad commit", muted_style()),
        ])
    };

    let selected_hint_line = if let Some(c) = selected_commit {
        Line::from(vec![
            Span::styled("Selected commit:  ", muted_style()),
            Span::styled(c.id.clone(), Style::default().fg(WARNING())),
            Span::raw(" "),
            Span::styled(c.summary.clone(), primary_style()),
        ])
    } else if let Some(ref msg) = app.bisect_last_message {
        Line::from(vec![
            Span::styled("Last result:      ", muted_style()),
            Span::styled(msg.clone(), accent_style()),
        ])
    } else {
        Line::from("")
    };

    let summary_lines = vec![
        Line::from(vec![
            Span::styled("Started from:     ", muted_style()),
            Span::styled(start_ref.to_string(), accent_style()),
        ]),
        Line::from(vec![
            Span::styled("Testing HEAD:     ", muted_style()),
            Span::styled(head_short.to_string(), Style::default().fg(WARNING())),
            Span::raw(" "),
            Span::styled(head_summary.to_string(), primary_style()),
        ]),
        Line::from(vec![
            Span::styled("Bounds:           ", muted_style()),
            Span::styled("bad=", muted_style()),
            Span::styled(bad_short, Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled("good=", muted_style()),
            Span::styled(
                bisect.good_oids.len().to_string(),
                Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("skipped=", muted_style()),
            Span::styled(
                bisect.skip_oids.len().to_string(),
                Style::default().fg(WARNING()).add_modifier(Modifier::BOLD),
            ),
        ]),
        progress_line,
        selected_hint_line,
        Line::from(vec![
            Span::styled(
                format!("Bisect Log ({})", bisect.log.len()),
                accent_style().add_modifier(Modifier::BOLD),
            ),
            if let Some(ref msg) = app.bisect_last_message {
                Span::styled(format!("  —  {}", msg), muted_style())
            } else {
                Span::raw("")
            },
        ]),
    ];
    f.render_widget(Paragraph::new(summary_lines), chunks[0]);

    let log_block = Block::default().borders(Borders::TOP).border_style(muted_style());
    let log_inner = log_block.inner(chunks[1]);
    f.render_widget(log_block, chunks[1]);

    if bisect.log.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "No bisect decisions recorded yet — press g (good), b (bad), or k (skip).",
                muted_style(),
            )),
            log_inner,
        );
    } else {
        let log_lines: Vec<Line<'static>> = bisect
            .log
            .iter()
            .skip(app.bisect_log_scroll.min(bisect.log.len().saturating_sub(1)))
            .map(|entry| {
                let (badge, badge_style) = match entry.verdict.as_str() {
                    "bad" => ("[BAD] ", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
                    "good" => {
                        ("[GOOD]", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD))
                    }
                    _ => ("[SKIP]", Style::default().fg(WARNING()).add_modifier(Modifier::BOLD)),
                };
                Line::from(vec![
                    Span::styled(badge, badge_style),
                    Span::raw(" "),
                    Span::styled(entry.short_oid.clone(), Style::default().fg(WARNING())),
                    Span::raw(" "),
                    Span::styled(entry.summary.clone(), primary_style()),
                ])
            })
            .collect();
        f.render_widget(Paragraph::new(log_lines), log_inner);
    }

    let hint = Line::from(vec![
        Span::styled("g", Style::default().fg(SUCCESS()).add_modifier(Modifier::BOLD)),
        Span::styled(" good  ", muted_style()),
        Span::styled("b", Style::default().fg(DANGER()).add_modifier(Modifier::BOLD)),
        Span::styled(" bad  ", muted_style()),
        Span::styled("k", Style::default().fg(WARNING()).add_modifier(Modifier::BOLD)),
        Span::styled(" skip  ", muted_style()),
        Span::styled("G/B", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" mark selected  ", muted_style()),
        Span::styled("r", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" reset  ", muted_style()),
        Span::styled("Esc/q", accent_style().add_modifier(Modifier::BOLD)),
        Span::styled(" close", muted_style()),
    ]);
    f.render_widget(Paragraph::new(hint), chunks[2]);
}
