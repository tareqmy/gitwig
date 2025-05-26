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
    style::{Color, Modifier, Style},
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
    const STATUS_HEIGHT: u16 = 1;

    loop {
        // We need terminal size here for scroll calculations outside the closure
        let size = terminal.size()?;
        let inner_area = size.inner(&tui::layout::Margin {
            vertical: 1,
            horizontal: 1,
        });
        let available_height = inner_area.height.saturating_sub(STATUS_HEIGHT);
        let visible_count = (available_height / ITEM_HEIGHT).min(config.items.len() as u16);
        let visible_count_usize = visible_count as usize;
        let max_scroll = config.items.len().saturating_sub(visible_count_usize);

        // Clamp scroll_top if needed (optional safety)
        if scroll_top > max_scroll {
            scroll_top = max_scroll;
        }

        terminal.draw(|f| {
            let outer_block = Block::default()
                .title("Twig")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White));
            f.render_widget(outer_block, size);

            let visible_items = &config.items
                [scroll_top..(scroll_top + visible_count_usize).min(config.items.len())];

            let mut constraints = vec![Constraint::Length(ITEM_HEIGHT); visible_items.len()];
            constraints.push(Constraint::Length(STATUS_HEIGHT));

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(inner_area);

            // Render items
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

            // Status line
            let status_text = "[↑/↓]: Navigate  [q]: Quit";
            let status = Paragraph::new(status_text)
                .style(Style::default().fg(Color::Blue).add_modifier(Modifier::ITALIC));
            f.render_widget(status, *chunks.last().unwrap());

            // Scrollbar
            if config.items.len() > visible_count_usize {
                let scrollbar_height = available_height;
                let scrollbar_x = inner_area.x + inner_area.width - 1;

                let thumb_height = (scrollbar_height * visible_count) / config.items.len() as u16;
                let thumb_top =
                    ((scrollbar_height - thumb_height) * scroll_top as u16) / max_scroll.max(1) as u16;

                for i in 0..scrollbar_height {
                    let symbol = if i >= thumb_top && i < thumb_top + thumb_height {
                        "█"
                    } else {
                        "│"
                    };

                    let scrollbar_cell = tui::layout::Rect {
                        x: scrollbar_x,
                        y: inner_area.y + i,
                        width: 1,
                        height: 1,
                    };

                    let bar = Paragraph::new(symbol)
                        .style(Style::default().fg(Color::DarkGray));
                    f.render_widget(bar, scrollbar_cell);
                }
            }
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
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
                    _ => {}
                }
            }
        }
    }
}
