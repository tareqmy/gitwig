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

use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// How often the supervising thread checks whether the child has exited.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Re-exported from `gitwig-core` so both crates build `git` the same way:
/// prompts disabled, stdin `/dev/null`, stdout and stderr piped.
pub use gitwig_core::{detached_command, git_command, tool_command};

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

/// Why a bounded `git` run did not produce a normal exit status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitRunError {
    /// The child outlived its budget and was killed.
    Timeout(Duration),
    /// The child could not be spawned or supervised.
    Spawn(String),
}

impl std::fmt::Display for GitRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitRunError::Timeout(d) => {
                write!(f, "git did not respond within {}s and was cancelled", d.as_secs())
            }
            GitRunError::Spawn(e) => write!(f, "could not run git: {}", e),
        }
    }
}

impl std::error::Error for GitRunError {}

/// Runs `cmd` to completion, killing it if it exceeds `timeout`.
///
/// `Command::output()` blocks forever on a remote that accepts the connection but
/// never replies, which is exactly what a firewalled or blackholed host does. This
/// drains stdout and stderr on dedicated threads (a bare `try_wait` poll loop
/// deadlocks once a pipe buffer fills) while a timer supervises the child.
///
/// A zero timeout means "no limit" and simply defers to `output()`.
pub fn run_git_with_timeout(mut cmd: Command, timeout: Duration) -> Result<Output, GitRunError> {
    if timeout.is_zero() {
        return cmd.output().map_err(|e| GitRunError::Spawn(e.to_string()));
    }

    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| GitRunError::Spawn(e.to_string()))?;

    // Drain both pipes concurrently; otherwise a chatty `git fetch` fills the
    // stderr buffer, blocks the child, and the timeout fires on a process that
    // was only ever waiting on us.
    let mut out_pipe = child.stdout.take();
    let mut err_pipe = child.stderr.take();
    let out_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = out_pipe.as_mut() {
            let _ = p.read_to_end(&mut buf);
        }
        buf
    });
    let err_reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = err_pipe.as_mut() {
            let _ = p.read_to_end(&mut buf);
        }
        buf
    });

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(GitRunError::Spawn(e.to_string()));
            }
        }
    };

    let stdout = out_reader.join().unwrap_or_default();
    let stderr = err_reader.join().unwrap_or_default();

    match status {
        Some(status) => Ok(Output { status, stdout, stderr }),
        None => Err(GitRunError::Timeout(timeout)),
    }
}
