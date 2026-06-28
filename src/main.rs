//! Gitwig entry point.
//!
//! `main` is intentionally thin: it sets up the terminal, hands control to
//! `app::run`, and tears the terminal down on the way out. Application
//! logic lives in the `app`, `ui`, `input`, and `config` modules.

#![deny(unsafe_code)]
#![deny(unused_imports, unused_must_use, dead_code, unused_assignments)]
#![deny(clippy::all, clippy::perf)]
#![allow(
    clippy::collapsible_if,
    clippy::collapsible_else_if,
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::derivable_impls,
    clippy::empty_line_after_doc_comments,
    clippy::empty_line_after_outer_attr
)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::panic))]

use std::{env, error::Error, io, path::PathBuf};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

mod app;
mod config;
mod debug_log;
mod input;
pub use gitwig_core as repo;
pub mod components;
mod keys;
pub mod popups;
mod queue;
pub mod tabs;
mod ui;
pub use crate::ui::ui_detail;

use crate::app::{App, run};
use crate::config::load_config;

fn main() -> Result<(), Box<dyn Error>> {
    // Install a panic hook to clean up the terminal state on crash
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let mut stdout = std::io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
        let _ = execute!(std::io::stdout(), crossterm::cursor::Show);

        // Also log the panic!
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown".to_string());

        let backtrace = std::backtrace::Backtrace::capture();
        let panic_msg = format!("Panic at {}: {}\nBacktrace:\n{}", location, msg, backtrace);
        for line in panic_msg.lines() {
            crate::debug_log::log("PANIC_FATAL", line);
        }

        default_panic(info);
    }));

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
    let app = App::new(config, config_path);

    // Run the application logic
    let res = run(&mut terminal, app);

    // Cleanup terminal: disable raw mode, leave alt screen, disable mouse
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    // Log any error returned from app logic
    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}
pub mod mouse;
