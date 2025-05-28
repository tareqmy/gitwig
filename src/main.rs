// Crossterm provides terminal control like input events and screen manipulation
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

// Standard library imports
use std::{env, error::Error, io, path::PathBuf};

// tui-rs is used for terminal-based user interfaces
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

// Custom config module
mod config;
use crate::config::{load_config, Config};

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

    // Load configuration from file or default
    let config = load_config(cli_path)?;

    // Run the application logic
    let res = run_app(&mut terminal, config);

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
fn run_app<B: tui::backend::Backend>(
    terminal: &mut Terminal<B>,
    config: Config,
) -> io::Result<()> {
    let mut selected_index: usize = 0; // Currently selected item index
    let mut scroll_top: usize = 0;     // Index of the topmost visible item
    const ITEM_HEIGHT: u16 = 3;        // Height of each item block in rows
    const STATUS_HEIGHT: u16 = 1;      // Height reserved for the status/help bar

    loop {
        // Determine available screen area
        let size = terminal.size()?;
        let inner_area = size.inner(&tui::layout::Margin {
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

        // Draw UI
        terminal.draw(|f| {
            // Outer frame with title
            let outer_block = Block::default()
                .title("Twig")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White));
            f.render_widget(outer_block, size);

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
                    Style::default().fg(Color::Black).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray).add_modifier(Modifier::DIM)
                };

                // Render the item as a bordered paragraph
                let paragraph = Paragraph::new(item.as_str())
                    .style(style)
                    .block(Block::default().borders(Borders::ALL));

                f.render_widget(paragraph, chunks[i]);
            }

            // Status/help bar at the bottom
            let status_text = "[↑/↓]: Navigate  [q]: Quit";
            let status = Paragraph::new(status_text)
                .style(Style::default().fg(Color::Blue).add_modifier(Modifier::ITALIC));
            f.render_widget(status, *chunks.last().unwrap());
        })?;

        // Handle keypress events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    // Quit
                    KeyCode::Char('q') => return Ok(()),

                    // Move selection down
                    KeyCode::Down | KeyCode::Char('j') => {
                        if selected_index + 1 < config.items.len() {
                            selected_index += 1;
                            let bottom = scroll_top + visible_count_usize;
                            if selected_index >= bottom {
                                scroll_top = scroll_top.saturating_add(1);
                            }
                        }
                    }

                    // Move selection up
                    KeyCode::Up | KeyCode::Char('k') => {
                        if selected_index > 0 {
                            selected_index -= 1;
                            if selected_index < scroll_top {
                                scroll_top = scroll_top.saturating_sub(1);
                            }
                        }
                    }

                    // Ignore other keys
                    _ => {}
                }
            }
        }
    }
}
