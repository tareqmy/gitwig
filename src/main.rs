use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, fs, io, path::PathBuf};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use serde::Deserialize;
use toml;

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
    loop {
        terminal.draw(|f| {
            let size = f.size();

            // Main border
            let outer_block = Block::default()
                .title("Twig")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White));
            f.render_widget(outer_block, size);

            // Inner layout
            let inner_area = size.inner(&tui::layout::Margin {
                vertical: 1,
                horizontal: 1,
            });

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    config
                        .items
                        .iter()
                        .map(|_| Constraint::Length(3))
                        .collect::<Vec<_>>(),
                )
                .split(inner_area);

            for (i, item) in config.items.iter().enumerate() {
                let paragraph = Paragraph::new(item.as_str())
                    .block(Block::default().borders(Borders::ALL));
                f.render_widget(paragraph, chunks[i]);
            }
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    return Ok(());
                }
            }
        }
    }
}
