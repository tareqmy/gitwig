// Crossterm provides terminal control like input events and screen manipulation
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

// Standard library imports
use std::{env, error::Error, io, path::PathBuf};

// ratatui is used for terminal-based user interfaces
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

// Custom config module
mod config;
use crate::config::{Config, load_config, save_config};

/// Interaction modes for the item list. The mode dictates how keystrokes
/// are interpreted and what guidance the status bar shows.
enum Mode {
    /// Browsing the list. Navigation + add/edit/delete shortcuts are active.
    Normal,
    /// Typing a new item to append. Enter commits, Esc cancels.
    Adding,
    /// Typing replacement text for the selected item. Enter commits, Esc cancels.
    Editing,
    /// Asking the user to confirm deletion of the selected item.
    ConfirmDelete,
    /// Showing the full shortcut reference as a centered overlay.
    Help,
}

/// Lines of the help overlay. Kept as a constant so any binding change
/// has one place to update.
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

fn main() -> Result<(), Box<dyn Error>> {
    // Enable raw mode to capture input without line buffering
    enable_raw_mode()?;
    let mut stdout = io::stdout();

    // Switch to alternate screen buffer and enable mouse support
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    // Parse optional CLI argument for config path
    let cli_path = env::args().nth(1).map(PathBuf::from);

    // Create Crossterm backend and initialize terminal UI
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Load configuration plus the path we should persist edits to.
    let (config, config_path) = load_config(cli_path)?;

    // Run the application logic
    let res = run_app(&mut terminal, config, config_path);

    // Cleanup terminal: disable raw mode, leave alt screen, disable mouse
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Log any error returned from app logic
    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

/// Main application loop
fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut config: Config,
    config_path: PathBuf,
) -> Result<(), Box<dyn Error>>
where
    <B as ratatui::backend::Backend>::Error: 'static,
{
    let mut selected_index: usize = 0; // Currently selected item index
    let mut scroll_top: usize = 0; // Index of the topmost visible item
    let mut mode = Mode::Normal; // Current interaction mode
    let mut input_buffer = String::new(); // In-progress text for add/edit
    let mut status_message: Option<String> = None; // Transient feedback (e.g. save errors)
    const ITEM_HEIGHT: u16 = 3; // Height of each item block in rows
    const STATUS_HEIGHT: u16 = 1; // Height reserved for the status/help bar

    loop {
        // Clamp selection inside the current list bounds (handles post-delete state)
        if !config.items.is_empty() && selected_index >= config.items.len() {
            selected_index = config.items.len() - 1;
        }
        if config.items.is_empty() {
            selected_index = 0;
        }

        // Determine available screen area
        let size = terminal.size()?;
        let area = Rect::new(0, 0, size.width, size.height);
        let inner_area = area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        let available_height = inner_area.height.saturating_sub(STATUS_HEIGHT);
        let visible_count = (available_height / ITEM_HEIGHT).min(config.items.len() as u16);
        let visible_count_usize = visible_count as usize;
        let max_scroll = config.items.len().saturating_sub(visible_count_usize);

        // Adjust scroll if necessary to avoid going past item list
        if scroll_top > max_scroll {
            scroll_top = max_scroll;
        }

        // Build the status/help line that fits the current mode.
        let status_text = match &mode {
            Mode::Normal => {
                "[↑/↓ j/k] Navigate  [a] Add  [e] Edit  [d] Delete  [?] Help  [q] Quit".to_string()
            }
            Mode::Adding => format!("Add item: {}_   [Enter] Save  [Esc] Cancel", input_buffer),
            Mode::Editing => format!("Edit item: {}_   [Enter] Save  [Esc] Cancel", input_buffer),
            Mode::ConfirmDelete => {
                let target = config
                    .items
                    .get(selected_index)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                format!("Delete \"{}\"? [y] Confirm  [n/Esc] Cancel", target)
            }
            Mode::Help => "[?/Esc/q] Close help".to_string(),
        };
        let status_text = match &status_message {
            Some(msg) => format!("{} | {}", msg, status_text),
            None => status_text,
        };

        // Draw UI
        terminal.draw(|f| {
            // Outer frame with title
            let outer_block = Block::default()
                .title("Twig")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White));
            f.render_widget(outer_block, area);

            // Slice visible items based on scroll
            let visible_items = &config.items
                [scroll_top..(scroll_top + visible_count_usize).min(config.items.len())];

            // Layout constraints per item + 1 for the status bar
            let mut constraints = vec![Constraint::Length(ITEM_HEIGHT); visible_items.len()];
            constraints.push(Constraint::Length(STATUS_HEIGHT));

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(inner_area);

            // Render each item
            for (i, item) in visible_items.iter().enumerate() {
                let actual_index = i + scroll_top;

                // Highlight the selected item
                let style = if actual_index == selected_index {
                    Style::default()
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray).add_modifier(Modifier::DIM)
                };

                // Render the item as a bordered paragraph
                let paragraph = Paragraph::new(item.as_str())
                    .style(style)
                    .block(Block::default().borders(Borders::ALL));

                f.render_widget(paragraph, chunks[i]);
            }

            // Status/help bar at the bottom — color shifts by mode so input
            // modes are visually distinct from the resting browse state.
            let status_style = match mode {
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
            let status = Paragraph::new(status_text.as_str()).style(status_style);
            f.render_widget(status, *chunks.last().unwrap());

            // Help overlay — rendered last so it sits on top of the list.
            if matches!(mode, Mode::Help) {
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
        })?;

        // Clear transient feedback once it has been shown for a frame.
        status_message = None;

        // Handle keypress events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match &mode {
                    Mode::Normal => match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Down | KeyCode::Char('j') => {
                            if selected_index + 1 < config.items.len() {
                                selected_index += 1;
                                let bottom = scroll_top + visible_count_usize;
                                if selected_index >= bottom {
                                    scroll_top = scroll_top.saturating_add(1);
                                }
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if selected_index > 0 {
                                selected_index -= 1;
                                if selected_index < scroll_top {
                                    scroll_top = scroll_top.saturating_sub(1);
                                }
                            }
                        }
                        KeyCode::Char('a') => {
                            input_buffer.clear();
                            mode = Mode::Adding;
                        }
                        KeyCode::Char('e') => {
                            if let Some(current) = config.items.get(selected_index) {
                                input_buffer = current.clone();
                                mode = Mode::Editing;
                            }
                        }
                        KeyCode::Char('d') if !config.items.is_empty() => {
                            mode = Mode::ConfirmDelete;
                        }
                        KeyCode::Char('?') => {
                            mode = Mode::Help;
                        }
                        _ => {}
                    },
                    Mode::Adding => match key.code {
                        KeyCode::Esc => {
                            input_buffer.clear();
                            mode = Mode::Normal;
                        }
                        KeyCode::Enter => {
                            let trimmed = input_buffer.trim();
                            if !trimmed.is_empty() {
                                config.items.push(trimmed.to_string());
                                selected_index = config.items.len() - 1;
                                status_message = match save_config(&config, &config_path) {
                                    Ok(()) => Some("Saved".to_string()),
                                    Err(e) => Some(format!("Save failed: {}", e)),
                                };
                            }
                            input_buffer.clear();
                            mode = Mode::Normal;
                        }
                        KeyCode::Backspace => {
                            input_buffer.pop();
                        }
                        KeyCode::Char(c) => {
                            input_buffer.push(c);
                        }
                        _ => {}
                    },
                    Mode::Editing => match key.code {
                        KeyCode::Esc => {
                            input_buffer.clear();
                            mode = Mode::Normal;
                        }
                        KeyCode::Enter => {
                            let trimmed = input_buffer.trim();
                            if !trimmed.is_empty()
                                && let Some(slot) = config.items.get_mut(selected_index)
                            {
                                *slot = trimmed.to_string();
                                status_message = match save_config(&config, &config_path) {
                                    Ok(()) => Some("Saved".to_string()),
                                    Err(e) => Some(format!("Save failed: {}", e)),
                                };
                            }
                            input_buffer.clear();
                            mode = Mode::Normal;
                        }
                        KeyCode::Backspace => {
                            input_buffer.pop();
                        }
                        KeyCode::Char(c) => {
                            input_buffer.push(c);
                        }
                        _ => {}
                    },
                    Mode::ConfirmDelete => match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if selected_index < config.items.len() {
                                config.items.remove(selected_index);
                                status_message = match save_config(&config, &config_path) {
                                    Ok(()) => Some("Deleted".to_string()),
                                    Err(e) => Some(format!("Save failed: {}", e)),
                                };
                            }
                            mode = Mode::Normal;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                            mode = Mode::Normal;
                        }
                        _ => {}
                    },
                    Mode::Help => match key.code {
                        KeyCode::Char('?')
                        | KeyCode::Esc
                        | KeyCode::Char('q')
                        | KeyCode::Char('Q') => {
                            mode = Mode::Normal;
                        }
                        _ => {}
                    },
                }
            }
        }
    }
}
