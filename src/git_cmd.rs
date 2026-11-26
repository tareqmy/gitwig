//! Subprocess construction for the TUI crate and bounded `git` execution.
//!
//! A bare `Command::new` inherits the controlling terminal, so a child can write
//! straight into the alternate screen (git complaining that gpg is missing,
//! ssh asking to confirm a fingerprint) or block forever on `/dev/tty` with no
//! way for the user to cancel. The constructors re-exported here from
//! `gitwig-core` detach every stdio stream; [`interactive_command`] is the one
//! sanctioned exception for launches that are meant to own the tty.
//! [`run_git_with_timeout`] guarantees a remote-touching child is reaped even
//! when the remote never answers.

use std::process::Command;

/// Re-exported from `gitwig-core` so both crates build and bound `git` the
/// same way: prompts disabled, stdin `/dev/null`, stdout and stderr piped.
pub use gitwig_core::{
    GitRunError, detached_command, git_command, run_git_with_timeout, tool_command,
};

/// Builds a command that is *meant* to own the terminal: an editor, a shell,
/// `git rebase -i`, `git mergetool`.
///
/// This is the one sanctioned way to inherit the tty. Call it only between
/// `LeaveAlternateScreen` and `EnterAlternateScreen` (see the launch sites in
/// `app/mod.rs`), and clear the terminal afterwards. Anywhere else, use
/// [`git_command`], [`tool_command`] or [`detached_command`].
#[allow(clippy::disallowed_methods)]
pub fn interactive_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    Command::new(program)
}
