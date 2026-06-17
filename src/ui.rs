//! Rendering for the main list + status bar + help overlay.
//!
//! All drawing reads from `&App`; nothing here mutates state. Adding a new
//! keybinding means updating `HELP_LINES` here AND the status text below,
//! so the help overlay and the bottom bar stay aligned.
//!
//! Visual conventions live in the `theme` block at the top — keep visual
//! choices centralized so the look stays coherent.

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap};

use crate::app::{App, ITEM_HEIGHT, Mode, STATUS_HEIGHT};

// ── Theme ──────────────────────────────────────────────────────────────────
// Colors are kept minimal so the app works on both dark and light terminal
// backgrounds. Plain text is left at the terminal's default foreground —
// never hard-code `White` / `Gray` / `DarkGray` for text, because what reads
// as "muted" on one background reads as invisible on the other. Use the
// helpers below: `muted_style()` for de-emphasis, bold for emphasis, and the
// accent colors only for true accents (selection, mode, warnings).

const ACCENT: Color = Color::Cyan;
const WARNING: Color = Color::Yellow;
const DANGER: Color = Color::Red;
const CARD_BORDER: BorderType = BorderType::Rounded;

/// Marker shown on the left edge of the selected card.
const SELECTION_MARK: &str = "▌ ";
const UNSELECTED_INDENT: &str = "  ";

/// "Muted" / secondary text. Uses the terminal's own foreground so it stays
/// readable on both light and dark backgrounds, then applies `DIM` to fade
/// it relative to primary text.
fn muted_style() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// Emphasized text. Bold over the terminal default — also theme-agnostic.
fn primary_style() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

/// Accent-colored, bold. Used for keys in the status bar / help overlay,
/// and the app title.
fn accent_style() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// Lines of the help overlay. Update this whenever a binding is added or
/// renamed — it is the user's only complete shortcut reference.
const HELP_LINES: &[(&str, &str)] = &[
    ("↑ / k", "Move selection up"),
    ("↓ / j", "Move selection down"),
    ("a", "Add a new item"),
    ("e", "Edit selected item"),
    ("d", "Delete selected item"),
    ("Enter", "Commit add or edit"),
    ("Esc", "Cancel input or close dialog"),
    ("Backspace", "Erase character while typing"),
    ("?", "Toggle this help overlay"),
    ("q", "Quit"),
];

/// Top-level draw entry point invoked from inside `terminal.draw`.
pub fn draw(f: &mut Frame, app: &App, area: Rect, inner_area: Rect, visible_count: usize) {
    draw_outer_frame(f, area);
    let (list_chunks, status_chunk) = list_and_status_chunks(inner_area, visible_count);
    draw_items(f, app, &list_chunks);
    draw_status_bar(f, app, status_chunk);

    if matches!(app.mode, Mode::Help) {
        draw_help_overlay(f, area);
    }
}

fn draw_outer_frame(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER)
        .border_style(muted_style())
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("Twig", accent_style()),
                Span::raw(" "),
            ])
            .alignment(Alignment::Left),
        )
        .title(
            Line::from(format!(" v{} ", env!("CARGO_PKG_VERSION")))
                .style(muted_style())
                .alignment(Alignment::Right),
        );
    f.render_widget(block, area);
}

/// Splits `inner_area` into N item rows + one status row, returning the
/// per-item chunks and the status chunk separately so the caller doesn't
/// have to remember the last index.
fn list_and_status_chunks(inner_area: Rect, visible_count: usize) -> (Vec<Rect>, Rect) {
    let mut constraints = vec![Constraint::Length(ITEM_HEIGHT); visible_count];
    constraints.push(Constraint::Length(STATUS_HEIGHT));

    let chunks: Vec<Rect> = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner_area)
        .to_vec();

    let (status, items) = chunks.split_last().expect("status chunk always present");
    (items.to_vec(), *status)
}

fn draw_items(f: &mut Frame, app: &App, chunks: &[Rect]) {
    let upper = (app.scroll_top + chunks.len()).min(app.config.items.len());
    let visible_items = &app.config.items[app.scroll_top..upper];

    for (i, item) in visible_items.iter().enumerate() {
        let actual_index = i + app.scroll_top;
        let is_selected = actual_index == app.selected_index;
        let pending_delete = is_selected && matches!(app.mode, Mode::ConfirmDelete);
        let pending_edit = is_selected && matches!(app.mode, Mode::Editing);

        // Selected/pending cards use an accent color; unselected cards use
        // the terminal's default foreground (dimmed) so they stay legible
        // on both light and dark backgrounds.
        let border_style = if pending_delete {
            Style::default().fg(DANGER)
        } else if pending_edit {
            Style::default().fg(WARNING)
        } else if is_selected {
            Style::default().fg(ACCENT)
        } else {
            muted_style()
        };

        let (mark, mark_style, text_style) = if is_selected {
            (SELECTION_MARK, border_style, primary_style())
        } else {
            (UNSELECTED_INDENT, Style::default(), Style::default())
        };

        let line = Line::from(vec![
            Span::styled(mark, mark_style),
            Span::styled(item.as_str(), text_style),
        ]);

        let card = Paragraph::new(line).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(CARD_BORDER)
                .border_style(border_style)
                .padding(Padding::horizontal(1)),
        );
        f.render_widget(card, chunks[i]);
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let (badge_text, badge_fg, badge_bg) = mode_badge(&app.mode);

    let badge_width = badge_text.chars().count() as u16 + 2; // " BADGE "
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(badge_width), Constraint::Min(0)])
        .split(area);

    // Left: mode badge with background fill.
    let badge = Paragraph::new(Line::from(vec![
        Span::raw(" "),
        Span::styled(
            badge_text,
            Style::default()
                .fg(badge_fg)
                .bg(badge_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
    ]))
    .style(Style::default().bg(badge_bg));
    f.render_widget(badge, chunks[0]);

    // Right: contextual content (help text, input line, etc.).
    let content_area = chunks[1];
    match &app.mode {
        Mode::Normal => {
            draw_status_content(f, content_area, normal_status_spans(&app.status_message));
        }
        Mode::Adding => {
            draw_input_status(f, content_area, "Add", &app.input_buffer);
        }
        Mode::Editing => {
            draw_input_status(f, content_area, "Edit", &app.input_buffer);
        }
        Mode::ConfirmDelete => {
            let target = app
                .config
                .items
                .get(app.selected_index)
                .map(|s| s.as_str())
                .unwrap_or("");
            draw_status_content(f, content_area, confirm_delete_spans(target));
        }
        Mode::Help => {
            draw_status_content(f, content_area, help_dismiss_spans());
        }
    }
}

/// Returns the badge text plus its foreground and background colors.
/// Badge backgrounds are solid colors, so the foreground only needs to
/// contrast with the badge bg — not the terminal background — and is
/// chosen for max readability against each bg.
fn mode_badge(mode: &Mode) -> (&'static str, Color, Color) {
    match mode {
        Mode::Normal => ("NORMAL", Color::Black, ACCENT),
        Mode::Adding => ("ADDING", Color::Black, WARNING),
        Mode::Editing => ("EDITING", Color::Black, WARNING),
        Mode::ConfirmDelete => ("CONFIRM", Color::White, DANGER),
        Mode::Help => ("HELP", Color::Black, ACCENT),
    }
}

fn draw_status_content(f: &mut Frame, area: Rect, spans: Vec<Span<'_>>) {
    // No paragraph-level style — each span carries its own style so the
    // muted/accent distinction survives on light terminals.
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn normal_status_spans(status_message: &Option<String>) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    spans.push(Span::raw("  "));
    if let Some(msg) = status_message {
        spans.push(Span::styled(format!("{}  ", msg), accent_style()));
        spans.push(Span::styled("·  ", muted_style()));
    }
    let entries = [
        ("↑↓", "navigate"),
        ("a", "add"),
        ("e", "edit"),
        ("d", "delete"),
        ("?", "help"),
        ("q", "quit"),
    ];
    for (i, (key, label)) in entries.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ·  ", muted_style()));
        }
        spans.push(Span::styled((*key).to_string(), accent_style()));
        spans.push(Span::raw(" "));
        spans.push(Span::raw((*label).to_string()));
    }
    spans
}

fn confirm_delete_spans(target: &str) -> Vec<Span<'static>> {
    vec![
        Span::raw("  "),
        Span::raw("Delete "),
        Span::styled(
            format!("\"{}\"", target),
            Style::default().fg(DANGER).add_modifier(Modifier::BOLD),
        ),
        Span::raw("?"),
        Span::styled("  ·  ", muted_style()),
        Span::styled(
            "y",
            Style::default().fg(DANGER).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" confirm"),
        Span::styled("  ·  ", muted_style()),
        Span::styled("n / Esc", accent_style()),
        Span::raw(" cancel"),
    ]
}

fn help_dismiss_spans() -> Vec<Span<'static>> {
    vec![
        Span::raw("  "),
        Span::styled("? / Esc / q", accent_style()),
        Span::raw("  close help"),
    ]
}

/// Renders the input prompt for Add/Edit modes and places the real
/// terminal cursor at the end of the typed buffer.
fn draw_input_status(f: &mut Frame, area: Rect, verb: &str, buffer: &str) {
    let prefix = format!("  {} › ", verb);
    let hints = "  ·  Enter save  ·  Esc cancel";

    let line = Line::from(vec![
        Span::styled(
            prefix.clone(),
            Style::default().fg(WARNING).add_modifier(Modifier::BOLD),
        ),
        Span::styled(buffer.to_string(), primary_style()),
        Span::styled(hints, muted_style()),
    ]);
    let para = Paragraph::new(line);
    f.render_widget(para, area);

    // Place the real terminal cursor at the end of the input. Clamp to
    // the visible width so a long input doesn't push the cursor off-screen.
    let cursor_offset = (prefix.chars().count() + buffer.chars().count()) as u16;
    let cursor_x = area
        .x
        .saturating_add(cursor_offset.min(area.width.saturating_sub(1)));
    f.set_cursor_position(Position::new(cursor_x, area.y));
}

fn draw_help_overlay(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 70, area);

    let key_width = HELP_LINES
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);

    let mut lines: Vec<Line> = Vec::with_capacity(HELP_LINES.len() + 2);
    lines.push(Line::from(""));
    for (key, desc) in HELP_LINES {
        let padded_key = format!("{:>width$}", key, width = key_width);
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(padded_key, accent_style()),
            Span::raw("   "),
            Span::raw((*desc).to_string()),
        ]));
    }
    lines.push(Line::from(""));

    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_type(CARD_BORDER)
        .border_style(Style::default().fg(ACCENT))
        .title(
            Line::from(vec![
                Span::raw(" "),
                Span::styled("Shortcuts", accent_style()),
                Span::raw(" "),
            ])
            .alignment(Alignment::Left),
        )
        .padding(Padding::horizontal(1));

    let help = Paragraph::new(lines)
        .block(help_block)
        .wrap(Wrap { trim: false });

    // Clear wipes the underlying cells so the list doesn't bleed through.
    f.render_widget(Clear, popup_area);
    f.render_widget(help, popup_area);
}

/// Returns a `Rect` of `(percent_x, percent_y)` dimensions, centered inside `area`.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
