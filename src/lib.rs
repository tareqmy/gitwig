//! Gitwig library root.
//!
//! `run` is the single implementation shared by the `gitwig` and `gtg`
//! binaries (`src/main.rs` and `src/bin/gtg.rs`), which are both thin
//! wrappers. Keeping the real logic here — rather than in `main.rs` twice —
//! means Cargo compiles and tests it once instead of once per binary.

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
pub mod fetch_error;
pub mod git_cmd;
mod input;
pub mod keybindings;
pub mod mouse;
pub use gitwig_core as repo;
pub mod components;
mod keys;
pub mod popups;
mod queue;
pub mod stats;
pub mod tabs;
mod terminal;
mod ui;
pub use crate::ui::ui_detail;

use crate::app::{App, run as run_app};
use crate::config::load_config;
use crate::terminal::{CliAction, check_cli, init_terminal, setup_panic_hook};

/// Entry point shared by the `gitwig` and `gtg` binaries.
pub fn run() -> Result<(), Box<dyn Error>> {
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
        let _ = execute!(io::stdout(), SetTitle("Gitwig"));
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

    let res = run_app(&mut terminal, app);

    drop(guard);

    if let Err(ref err) = res {
        println!("{:?}", err);
    }

    res
}
