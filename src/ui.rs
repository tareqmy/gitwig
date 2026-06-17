//! Rendering for the main list + status bar + help overlay.
//!
//! All drawing reads from `&App`; nothing here mutates state. Adding a new
//! keybinding means updating `HELP_LINES` here AND the status text below,
//! so the help overlay and the bottom bar stay aligned.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::app::{App, ITEM_HEIGHT, Mode, STATUS_HEIGHT};

/// Lines of the help overlay. Update this whenever a binding is added or
/// renamed — it is the user's only complete shortcut reference.
const HELP_LINES: &[(&str, &str)] = &[
    ("↑ / k", "Move selection up"),
    ("↓ / j", "Move selection down"),
    ("a", "Add a new item (Enter saves, Esc cancels)"),
    ("e", "Edit selected item (Enter saves, Esc cancels)"),
    ("d", "Delete selected item (y confirms, n / Esc cancels)"),
    ("Backspace", "Erase character while typing"),
    ("?", "Toggle this help overlay"),
    ("q", "Quit"),
];

/// Top-level draw entry point invoked from inside `terminal.draw`.
pub fn draw(f: &mut Frame, app: &App, area: Rect, inner_area: Rect, visible_count: usize) {
    // Outer frame with title
    let outer_block = Block::default()
        .title("Twig")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White));
    f.render_widget(outer_block, area);

    let upper_bound = (app.scroll_top + visible_count).min(app.config.items.len());
    let visible_items = &app.config.items[app.scroll_top..upper_bound];

    let mut constraints = vec![Constraint::Length(ITEM_HEIGHT); visible_items.len()];
    constraints.push(Constraint::Length(STATUS_HEIGHT));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner_area);

    for (i, item) in visible_items.iter().enumerate() {
        let actual_index = i + app.scroll_top;
        let style = if actual_index == app.selected_index {
            Style::default()
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray).add_modifier(Modifier::DIM)
        };
        let paragraph = Paragraph::new(item.as_str())
            .style(style)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(paragraph, chunks[i]);
    }

    draw_status_bar(f, app, *chunks.last().unwrap());

    if matches!(app.mode, Mode::Help) {
        draw_help_overlay(f, area);
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let base = match &app.mode {
        Mode::Normal => {
            "[↑/↓ j/k] Navigate  [a] Add  [e] Edit  [d] Delete  [?] Help  [q] Quit".to_string()
        }
        Mode::Adding => format!(
            "Add item: {}_   [Enter] Save  [Esc] Cancel",
            app.input_buffer
        ),
        Mode::Editing => format!(
            "Edit item: {}_   [Enter] Save  [Esc] Cancel",
            app.input_buffer
        ),
        Mode::ConfirmDelete => {
            let target = app
                .config
                .items
                .get(app.selected_index)
                .map(|s| s.as_str())
                .unwrap_or("");
            format!("Delete \"{}\"? [y] Confirm  [n/Esc] Cancel", target)
        }
        Mode::Help => "[?/Esc/q] Close help".to_string(),
    };
    let text = match &app.status_message {
        Some(msg) => format!("{} | {}", msg, base),
        None => base,
    };

    let style = match app.mode {
        Mode::Normal => Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::ITALIC),
        Mode::Adding | Mode::Editing => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        Mode::ConfirmDelete => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        Mode::Help => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    };

    f.render_widget(Paragraph::new(text).style(style), area);
}

fn draw_help_overlay(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 60, area);
    let key_width = HELP_LINES
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);
    let body: String = HELP_LINES
        .iter()
        .map(|(k, desc)| format!("  {:width$}   {}\n", k, desc, width = key_width))
        .collect();
    let help_block = Block::default()
        .title(" Shortcuts ")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Cyan));
    let help = Paragraph::new(body)
        .block(help_block)
        .style(Style::default().fg(Color::White))
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
