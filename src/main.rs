//! Gitwig entry point.
//!
//! `main` is intentionally thin: it sets up the terminal, hands control to
//! `app::run`, and tears the terminal down on the way out. Application
//! logic lives in the `app`, `ui`, `input`, and `config` modules.

#![allow(unsafe_code)]
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

use std::{env, error::Error, io};

use crossterm::{execute, terminal::SetTitle};
use ratatui::{Terminal, backend::CrosstermBackend};

mod app;
mod config;
mod debug_log;
mod input;
pub mod keybindings;
pub mod mouse;
pub use gitwig_core as repo;
pub mod components;
mod keys;
pub mod popups;
mod queue;
pub mod tabs;
mod terminal;
mod ui;
pub use crate::ui::ui_detail;

use crate::app::{App, run};
use crate::config::load_config;
use crate::terminal::{CliAction, check_cli, init_terminal, setup_panic_hook};

fn main() -> Result<(), Box<dyn Error>> {
    let config_path = match check_cli()? {
        CliAction::Run(path) => path,
        CliAction::Exit => return Ok(()),
    };

    setup_panic_hook();
    let guard = init_terminal()?;

    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (config, config_path, warning) = load_config(config_path)?;

    // Set terminal title
    if config.compatibility_mode {
        let _ = execute!(io::stdout(), SetTitle("[Gitwig]"));
    } else {
        let _ = execute!(io::stdout(), SetTitle("🌿 Gitwig"));
    }

    unsafe {
        if config.ssh_strict_host_checking {
            env::set_var("GITWIG_SSH_STRICT", "1");
        } else {
            env::set_var("GITWIG_SSH_STRICT", "0");
        }
    }

    let mut app = App::new(config, config_path);
    if let Some(warn) = warning {
        app.status_message = Some(warn);
    }

    let res = run(&mut terminal, app);

    drop(guard);

    if let Err(ref err) = res {
        println!("{:?}", err);
    }

    res
}
