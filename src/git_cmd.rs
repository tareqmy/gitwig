//! Hardened `git` subprocess construction and bounded execution.
//!
//! Every `git` invocation that can touch a remote **must** be built here. A bare
//! `Command::new("git")` inherits the controlling terminal, so an unreachable or
//! permission-denied remote lets `ssh` or a credential helper write its prompt
//! straight into the alternate screen — corrupting the TUI — or block forever on
//! `/dev/tty` with no way for the user to cancel.
//!
//! [`git_command`] removes that class of failure by disabling every interactive
//! prompt path, and [`run_git_with_timeout`] guarantees the child is reaped even
//! when the remote simply never answers.

use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// How often the supervising thread checks whether the child has exited.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Builds a `git` command that can never block on an interactive prompt.
///
/// - `GIT_TERMINAL_PROMPT=0` stops git asking for a username/password on the tty.
/// - `GIT_SSH_COMMAND` pins host-key handling so ssh never asks to confirm a
///   fingerprint (see [`crate::config::ssh_command_val`]).
/// - `GIT_ASKPASS`/`SSH_ASKPASS` are neutralised so no GUI or console credential
///   helper can be spawned behind the alternate screen.
/// - The protocol allowlist blocks `ext::`-style transports from a hostile
///   `.gitmodules` or remote URL.
/// - stdin is `/dev/null`, so anything that still tries to read gets EOF instead
///   of stealing the user's keystrokes.
pub fn git_command() -> Command {
    let mut cmd = Command::new("git");
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd.env("GIT_SSH_COMMAND", crate::config::ssh_command_val());
    cmd.env("GIT_ALLOW_PROTOCOL", "https:ssh:git:file");
    cmd.env("GIT_PROTOCOL_FROM_USER", "0");
    // An askpass helper would pop a prompt *outside* our alternate screen.
    cmd.env("GIT_ASKPASS", "");
    cmd.env("SSH_ASKPASS", "");
    cmd.env("GCM_INTERACTIVE", "Never");
    cmd.stdin(Stdio::null());
    cmd
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
