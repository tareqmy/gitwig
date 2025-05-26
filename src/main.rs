use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use serde::Deserialize;
use std::{error::Error, fs, io, path::PathBuf};
use toml;
use tui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

#[derive(Debug, Deserialize)]
struct Config {
    items: Vec<String>,
}

fn load_config() -> Result<Config, Box<dyn Error>> {
    let config_path = PathBuf::from("./config/config.toml");
    let contents = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let config = load_config()?;

    let res = run_app(&mut terminal, config);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: tui::backend::Backend>(
    terminal: &mut Terminal<B>,
    config: Config,
) -> io::Result<()> {
    let mut selected_index: usize = 0;
    let mut scroll_top: usize = 0;
    const ITEM_HEIGHT: u16 = 3;

    loop {
        terminal.draw(|f| {
            let size = f.size();

            let outer_block = Block::default()
                .title("Twig")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White));
            f.render_widget(outer_block, size);

            let inner_area = size.inner(&tui::layout::Margin {
                vertical: 1,
                horizontal: 1,
            });

            let visible_count = (inner_area.height / ITEM_HEIGHT).min(config.items.len() as u16);
            let max_scroll = config.items.len().saturating_sub(visible_count as usize);

            let visible_items = &config.items[scroll_top..(scroll_top + visible_count as usize)];

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Length(ITEM_HEIGHT); visible_items.len()])
                .split(inner_area);

            for (i, item) in visible_items.iter().enumerate() {
                let actual_index = i + scroll_top;
                let style = if actual_index == selected_index {
                    Style::default().fg(Color::Black).bg(Color::Yellow)
                } else {
                    Style::default().fg(Color::White)
                };

                let paragraph = Paragraph::new(item.as_str())
                    .style(style)
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(paragraph, chunks[i]);
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Down | KeyCode::Char('j') => {
                        if selected_index + 1 < config.items.len() {
                            selected_index += 1;
                            // Scroll down if needed
                            let bottom = scroll_top + (terminal.size()?.height / ITEM_HEIGHT) as usize;
                            if selected_index >= bottom {
                                scroll_top = scroll_top.saturating_add(1);
                            }
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if selected_index > 0 {
                            selected_index -= 1;
                            // Scroll up if needed
                            if selected_index < scroll_top {
                                scroll_top = scroll_top.saturating_sub(1);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
