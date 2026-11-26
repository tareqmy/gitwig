//! Child-process construction that can never write to, or read from, the TUI.
//!
//! Gitwig draws on the terminal's alternate screen. Any child that inherits the
//! parent's stdio can scribble over that screen (git's `error: cannot run gpg`
//! on a signed commit did exactly this from the Graph tab) or block on a prompt
//! the user cannot see. Every non-interactive child in the workspace is
//! therefore built here, with all three stdio streams detached from the tty by
//! default, and a clippy `disallowed-methods` rule fails the build on any bare
//! `std::process::Command::new`.
//!
//! - [`git_command`]: hardened `git` with prompts disabled and stdio piped.
//! - [`tool_command`]: any other helper (`gh`, `curl`, `pbpaste`, …) with stdio piped.
//! - [`detached_command`]: fire-and-forget helpers (`open`, `xdg-open`) with stdio nulled.
//! - [`spawn_pipe_reader`]: drains a pipe on a thread so a chatty child never deadlocks.

use std::io::Read;
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// How often the supervising thread checks whether a bounded child has exited.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Why a bounded subprocess run did not produce a normal exit status.
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

    // Drain both pipes concurrently; otherwise a chatty child fills a pipe
    // buffer, blocks, and the timeout fires on a process only waiting on us.
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

/// Whether SSH must strictly verify host keys instead of `accept-new`.
/// Stored atomically so toggling it in Settings never mutates process env vars
/// across active background threads.
static SSH_STRICT_HOST_CHECKING: AtomicBool = AtomicBool::new(false);

/// Updates the host-key policy read by [`ssh_command_val`] and [`git_command`].
pub fn set_ssh_strict_host_checking(strict: bool) {
    SSH_STRICT_HOST_CHECKING.store(strict, Ordering::Relaxed);
}

/// `BatchMode=yes` is load-bearing: without it, ssh bypasses the captured
/// stdio pipes and prompts for passphrases or confirmations directly on
/// `/dev/tty`, painting raw text over the TUI's alternate screen. With it,
/// ssh fails immediately and the error is captured and reported normally.
pub fn ssh_command_val() -> &'static str {
    if SSH_STRICT_HOST_CHECKING.load(Ordering::Relaxed) {
        "ssh -o BatchMode=yes -o StrictHostKeyChecking=yes"
    } else {
        "ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new"
    }
}

/// Builds a `git` command that can neither prompt nor print on the terminal.
///
/// - `GIT_TERMINAL_PROMPT=0` stops git asking for a username/password on the tty.
/// - `GIT_SSH_COMMAND` pins host-key handling so ssh never asks to confirm a
///   fingerprint (see [`ssh_command_val`]).
/// - `GIT_ASKPASS`/`SSH_ASKPASS` are neutralised so no GUI or console credential
///   helper can be spawned behind the alternate screen.
/// - The protocol allowlist blocks `ext::`-style transports from a hostile
///   `.gitmodules` or remote URL.
/// - stdin is `/dev/null`, stdout and stderr are pipes. A caller that only
///   reads stdout must drain stderr too (see [`spawn_pipe_reader`]), or use
///   `output()` / `wait_with_output()` which drain both.
pub fn git_command() -> Command {
    let mut cmd = tool_command("git");
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd.env("GIT_SSH_COMMAND", ssh_command_val());
    cmd.env("GIT_ALLOW_PROTOCOL", "https:ssh:git:file");
    cmd.env("GIT_PROTOCOL_FROM_USER", "0");
    // An askpass helper would pop a prompt *outside* the alternate screen.
    cmd.env("GIT_ASKPASS", "");
    cmd.env("SSH_ASKPASS", "");
    cmd.env("GCM_INTERACTIVE", "Never");
    cmd
}

/// Builds a non-git helper command with every stdio stream detached from the tty.
///
/// stdin is `/dev/null` so nothing can steal keystrokes; stdout and stderr are
/// pipes so nothing can reach the screen. Callers that need to feed stdin
/// override it with `Stdio::piped()` after construction.
#[allow(clippy::disallowed_methods)]
pub fn tool_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd
}

/// Builds a fire-and-forget helper whose output nobody will read.
///
/// All three streams go to `/dev/null`, so `status()` can be called without a
/// reader and the child can never block on a full pipe or touch the screen.
#[allow(clippy::disallowed_methods)]
pub fn detached_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    cmd
}

/// Takes the child's stderr pipe and drains it on a background thread.
///
/// A child whose stderr is piped but never read blocks once the pipe buffer
/// fills, which turns a noisy `git log` into a hang. Join the handle after the
/// child exits to get everything it wrote.
pub fn spawn_pipe_reader(child: &mut Child) -> JoinHandle<Vec<u8>> {
    let mut pipe = child.stderr.take();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = pipe.as_mut() {
            let _ = p.read_to_end(&mut buf);
        }
        buf
    })
}

/// Turns a child's captured stderr into a one-line message fit for the status bar.
///
/// Git repeats `error: cannot run gpg: No such file or directory` once per
/// signed commit; the user only needs to see it once, with its `error: ` prefix
/// stripped. Returns `None` when stderr was empty or whitespace.
pub fn summarize_stderr(stderr: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(stderr);
    let mut seen: Vec<&str> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        let line =
            line.strip_prefix("error: ").or_else(|| line.strip_prefix("fatal: ")).unwrap_or(line);
        if !line.is_empty() && !seen.contains(&line) {
            seen.push(line);
        }
    }
    match seen.as_slice() {
        [] => None,
        [only] => Some((*only).to_string()),
        [first, rest @ ..] => Some(format!("{} (+{} more)", first, rest.len())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_command_detaches_every_stdio_stream() {
        let mut child = git_command().arg("--version").spawn().expect("git must be installed");
        assert!(child.stdin.is_none(), "stdin must be /dev/null, not inherited");
        assert!(child.stdout.is_some(), "stdout must be a pipe, not the terminal");
        assert!(child.stderr.is_some(), "stderr must be a pipe, not the terminal");
        let _ = child.wait();
    }

    #[test]
    fn tool_command_detaches_every_stdio_stream() {
        let mut child =
            tool_command("git").arg("--version").spawn().expect("git must be installed");
        assert!(child.stdin.is_none());
        assert!(child.stdout.is_some());
        assert!(child.stderr.is_some());
        let _ = child.wait();
    }

    #[test]
    fn detached_command_owns_no_pipes_and_can_use_status() {
        let status =
            detached_command("git").arg("--version").status().expect("git must be installed");
        assert!(status.success());
    }

    #[test]
    fn summarize_stderr_dedupes_and_strips_git_prefixes() {
        let raw = b"error: cannot run gpg: No such file or directory\n\
                    error: cannot run gpg: No such file or directory\n\
                    fatal: something else\n";
        assert_eq!(
            summarize_stderr(raw).as_deref(),
            Some("cannot run gpg: No such file or directory (+1 more)")
        );
        assert_eq!(summarize_stderr(b"  \n"), None);
        assert_eq!(summarize_stderr(b"error: just one\n").as_deref(), Some("just one"));
    }

    #[test]
    fn set_ssh_strict_host_checking_switches_policy_atomically() {
        struct ResetGuard(bool);
        impl Drop for ResetGuard {
            fn drop(&mut self) {
                set_ssh_strict_host_checking(self.0);
            }
        }
        let prev = SSH_STRICT_HOST_CHECKING.load(Ordering::Relaxed);
        let _guard = ResetGuard(prev);

        set_ssh_strict_host_checking(true);
        assert_eq!(ssh_command_val(), "ssh -o BatchMode=yes -o StrictHostKeyChecking=yes");

        set_ssh_strict_host_checking(false);
        assert_eq!(ssh_command_val(), "ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new");
    }

    #[test]
    fn run_git_with_timeout_returns_timeout_on_hung_child() {
        let cmd = if cfg!(target_os = "windows") {
            let mut c = tool_command("cmd");
            c.args(["/c", "timeout", "/t", "10", "/nobreak"]);
            c
        } else {
            let mut c = tool_command("sleep");
            c.arg("10");
            c
        };
        let res = run_git_with_timeout(cmd, Duration::from_millis(120));
        assert!(
            matches!(res, Err(GitRunError::Timeout(_))),
            "expected timeout error, got {:?}",
            res
        );
    }
}
