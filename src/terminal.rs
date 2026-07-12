//! Terminal setup, CLI parsing, and crash handler/panic hooks.

use std::{env, error::Error, io, path::PathBuf};

use crossterm::{
    event::DisableMouseCapture,
    execute,
    terminal::{LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};

/// Represents the action determined from command line arguments.
pub enum CliAction {
    Run(Option<PathBuf>),
    Exit,
}

/// Parses CLI arguments and validates system dependency on git command.
pub fn check_cli() -> Result<CliAction, Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--version" | "-v" => {
                println!("gitwig {}", env!("CARGO_PKG_VERSION"));
                return Ok(CliAction::Exit);
            }
            "--help" | "-h" => {
                println!("Gitwig - A Rust-based terminal user interface (TUI) for Git");
                println!();
                println!("Usage:");
                println!("  gitwig [config_path]    Start Gitwig with the specified config file");
                println!("  gitwig -v, --version    Print version info and exit");
                println!("  gitwig -h, --help       Print help info and exit");
                return Ok(CliAction::Exit);
            }
            _ => {}
        }
    }

    // Verify system 'git' is present on PATH before entering TUI
    if let Err(e) = std::process::Command::new("git").arg("--version").output() {
        eprintln!("Error: 'git' command-line tool not found on PATH.");
        eprintln!(
            "Gitwig requires a system installation of 'git' for network operations, staging, and diffing."
        );
        eprintln!("Detailed error: {:?}", e);
        std::process::exit(1);
    }

    let cli_path = env::args().nth(1).map(PathBuf::from);
    Ok(CliAction::Run(cli_path))
}

/// Installs a panic hook that cleanly restores the terminal state on crash
/// before executing the default backtrace/panic printing logic.
pub fn setup_panic_hook() {
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
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
}

/// Scope guard to ensure terminal raw mode and alternate screen are cleaned up when dropped.
pub struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
        let _ = execute!(stdout, crossterm::cursor::Show);
        print!("\x1b[23;0t");
        let _ = io::Write::flush(&mut stdout);
    }
}

/// Initializes raw mode, alternate screen buffer, title stack, and returns a guard.
pub fn init_terminal() -> Result<TerminalGuard, Box<dyn Error>> {
    enable_raw_mode()?;
    let guard = TerminalGuard;

    let mut stdout = io::stdout();
    // Push terminal title stack
    print!("\x1b[22;0t");
    let _ = io::Write::flush(&mut stdout);

    // Switch to alternate screen buffer and enable mouse support
    execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;

    Ok(guard)
}
