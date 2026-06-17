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
use crate::repo::{ItemStatus, RepoSummary};

// ── Theme ──────────────────────────────────────────────────────────────────
// Colors are kept minimal so the app works on both dark and light terminal
// backgrounds. Plain text is left at the terminal's default foreground —
// never hard-code `White` / `Gray` / `DarkGray` for text, because what reads
// as "muted" on one background reads as invisible on the other. Use the
// helpers below: `muted_style()` for de-emphasis, bold for emphasis, and the
// accent colors only for true accents (selection, mode, warnings).

pub(crate) const ACCENT: Color = Color::Cyan;
pub(crate) const WARNING: Color = Color::Yellow;
pub(crate) const DANGER: Color = Color::Red;
pub(crate) const SUCCESS: Color = Color::Green;
pub(crate) const CARD_BORDER: BorderType = BorderType::Rounded;

/// Width of the per-item status zone on the right of each card. Wide
/// enough to fit a busy repo's worth of indicators ("● 99+ 99! 99? 99↑")
/// with a little breathing room. Right-aligned, so it crops on the left
/// for the unusual case of 3-digit counts on every indicator.
const STATUS_ZONE_WIDTH: u16 = 22;

/// Marker shown on the left edge of the selected card.
const SELECTION_MARK: &str = "▌ ";
const UNSELECTED_INDENT: &str = "  ";

/// "Muted" / secondary text. Uses the terminal's own foreground so it stays
/// readable on both light and dark backgrounds, then applies `DIM` to fade
/// it relative to primary text.
pub(crate) fn muted_style() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// Emphasized text. Bold over the terminal default — also theme-agnostic.
pub(crate) fn primary_style() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

/// Accent-colored, bold. Used for keys in the status bar / help overlay,
/// and the app title.
pub(crate) fn accent_style() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// Lines of the help overlay. Update this whenever a binding is added or
/// renamed — it is the user's only complete shortcut reference.
const HELP_LINES: &[(&str, &str)] = &[
    ("↑ / k", "Move selection up"),
    ("↓ / j", "Move selection down"),
    ("Enter", "View selected item details (or commit input)"),
    ("a", "Add a new item"),
    ("e", "Edit selected item"),
    ("d", "Delete selected item"),
    ("r", "Refresh status of selected item"),
    ("Esc", "Cancel input, close dialog, or leave detail view"),
    ("Backspace", "Erase character while typing"),
    ("?", "Toggle this help overlay"),
    ("q", "Quit (also closes detail view)"),
];

/// Top-level draw entry point invoked from inside `terminal.draw`.
pub fn draw(f: &mut Frame, app: &App, area: Rect, inner_area: Rect, visible_count: usize) {
    draw_outer_frame(f, area);

    // Always reserve the bottom row for the status bar, regardless of mode.
    let (content_area, status_chunk) = content_and_status_chunks(inner_area);

    if matches!(app.mode, Mode::Detail) {
        if let Some(detail) = &app.current_detail {
            let item_name = app
                .config
                .items
                .get(app.selected_index)
                .map(String::as_str)
                .unwrap_or("");
            crate::ui_detail::draw(f, item_name, detail, content_area);
        }
    } else {
        let list_chunks = item_chunks(content_area, visible_count);
        draw_items(f, app, &list_chunks);
    }

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

/// Reserve the bottom row for the status bar. The remainder is the
/// "content area" — list view, detail view, or anything else a mode
/// wants to draw.
fn content_and_status_chunks(inner_area: Rect) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(STATUS_HEIGHT)])
        .split(inner_area);
    (chunks[0], chunks[1])
}

/// Within the content area, split into N item rows + a flex spacer so the
/// list is top-aligned and never pushes against the status bar.
fn item_chunks(content_area: Rect, visible_count: usize) -> Vec<Rect> {
    let mut constraints = vec![Constraint::Length(ITEM_HEIGHT); visible_count];
    constraints.push(Constraint::Min(0));

    let chunks: Vec<Rect> = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(content_area)
        .to_vec();
    // Drop the trailing spacer.
    chunks[..visible_count].to_vec()
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

        // Render the block first so we can split its inner area into two
        // horizontal zones (name on the left, status indicator on the right).
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(CARD_BORDER)
            .border_style(border_style)
            .padding(Padding::horizontal(1));
        let inner = block.inner(chunks[i]);
        f.render_widget(block, chunks[i]);

        let card_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(STATUS_ZONE_WIDTH)])
            .split(inner);

        let name_line = Line::from(vec![
            Span::styled(mark, mark_style),
            Span::styled(item.as_str(), text_style),
        ]);
        f.render_widget(Paragraph::new(name_line), card_chunks[0]);

        let fallback = ItemStatus::Missing;
        let status = app.statuses.get(actual_index).unwrap_or(&fallback);
        let status_line = status_indicator_line(status).alignment(Alignment::Right);
        f.render_widget(Paragraph::new(status_line), card_chunks[1]);
    }
}

/// Renders the per-item status as a colored symbol + (for git repos) a
/// compact set of `N+` (staged), `N!` (modified), `N?` (untracked),
/// `N↑` (commits ahead), `N↓` (commits behind) suffixes. Only non-zero
/// counts are shown so the indicator stays compact for the common case.
fn status_indicator_line(status: &ItemStatus) -> Line<'static> {
    match status {
        ItemStatus::Missing => Line::from(vec![
            Span::styled("✕", Style::default().fg(DANGER)),
            Span::raw(" "),
            Span::styled("missing", muted_style()),
        ]),
        ItemStatus::Directory => Line::from(vec![
            Span::styled("○", Style::default().fg(WARNING)),
            Span::raw(" "),
            Span::styled("dir", muted_style()),
        ]),
        ItemStatus::GitRepo(None) => Line::from(vec![
            Span::styled("●", Style::default().fg(SUCCESS)),
            Span::raw(" "),
            Span::styled("?", muted_style()),
        ]),
        ItemStatus::GitRepo(Some(summary)) => repo_indicator_line(summary),
    }
}

fn repo_indicator_line(summary: &RepoSummary) -> Line<'static> {
    let mut spans = vec![Span::styled("●", Style::default().fg(SUCCESS))];
    if summary.unchanged() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled("clean", muted_style()));
        return Line::from(spans);
    }
    // Each (count, symbol, style) is rendered only if count > 0. The
    // ordering matches the Detail view's worktree section for consistency.
    let parts: [(usize, &str, Style); 5] = [
        (summary.staged, "+", Style::default().fg(ACCENT)),
        (summary.modified, "!", Style::default().fg(WARNING)),
        (summary.untracked, "?", muted_style()),
        (summary.ahead, "↑", primary_style()),
        (summary.behind, "↓", Style::default().fg(WARNING)),
    ];
    for (count, symbol, style) in parts {
        if count > 0 {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(format!("{}{}", count, symbol), style));
        }
    }
    Line::from(spans)
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
        Mode::Detail => {
            draw_status_content(f, content_area, detail_dismiss_spans());
        }
    }
}

fn detail_dismiss_spans() -> Vec<Span<'static>> {
    vec![
        Span::raw("  "),
        Span::styled("Esc / q", accent_style()),
        Span::raw("  back to list"),
    ]
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
        Mode::Detail => ("DETAIL", Color::Black, ACCENT),
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
        ("r", "refresh"),
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

    let mut lines: Vec<Line> = Vec::with_capacity(HELP_LINES.len() + 8);
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
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Status indicators", primary_style()),
    ]));
    let pad_symbol = |sym: &'static str, color: Color, label: &'static str, desc: &'static str| {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(sym, Style::default().fg(color)),
            Span::raw(" "),
            Span::styled(format!("{:<8}", label), muted_style()),
            Span::raw(desc),
        ])
    };
    lines.push(pad_symbol(
        "●",
        SUCCESS,
        "git",
        "Directory is a git repository",
    ));
    lines.push(pad_symbol(
        "○",
        WARNING,
        "dir",
        "Directory exists but is not a git repo",
    ));
    lines.push(pad_symbol(
        "✕",
        DANGER,
        "missing",
        "Path does not exist or is not a directory",
    ));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Repo state suffixes", primary_style()),
    ]));
    let pad_suffix = |sym: &'static str, style: Style, desc: &'static str| {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("N{}", sym), style),
            Span::raw("        "),
            Span::raw(desc),
        ])
    };
    lines.push(pad_suffix(
        "+",
        Style::default().fg(ACCENT),
        "files staged for commit",
    ));
    lines.push(pad_suffix(
        "!",
        Style::default().fg(WARNING),
        "files modified but not staged",
    ));
    lines.push(pad_suffix("?", muted_style(), "untracked files"));
    lines.push(pad_suffix(
        "↑",
        primary_style(),
        "commits ahead of upstream (need push)",
    ));
    lines.push(pad_suffix(
        "↓",
        Style::default().fg(WARNING),
        "commits behind upstream",
    ));
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
