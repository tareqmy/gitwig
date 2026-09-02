//! Embedded terminal session: a shell running in a PTY, rendered inside the
//! TUI by the terminal panel (`components::terminal_panel`).
//!
//! A detached reader thread pumps PTY output into a shared `vt100::Parser`;
//! the run loop polls `poll_exit` each frame and the draw pass renders the
//! parser's screen. Dropping the session kills and reaps the shell.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Lines of scrollback history kept by the parser.
pub const SCROLLBACK_LINES: usize = 1000;

const MIN_ROWS: u16 = 3;
const MIN_COLS: u16 = 10;

/// Panel-level state owned by `App`: the (single) session plus visibility
/// and the user's preferred panel height.
pub struct TerminalPanelState {
    pub session: Option<TerminalSession>,
    pub visible: bool,
    /// Desired outer panel height in rows, including the 2 border rows.
    pub height: u16,
}

impl Default for TerminalPanelState {
    fn default() -> Self {
        Self { session: None, visible: false, height: 12 }
    }
}

/// A live shell attached to a PTY.
pub struct TerminalSession {
    /// Shared with the reader thread: it locks to `process()` output bytes,
    /// the draw pass locks to read the screen.
    parser: Arc<Mutex<vt100::Parser>>,
    /// Kept for `resize()`; dropping it closes the PTY, which unblocks the
    /// reader thread and hangs up the shell.
    master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Box<dyn std::io::Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    /// Set by the reader thread on EOF/read error, or here on write failure.
    exited: Arc<AtomicBool>,
    exit_status: Option<portable_pty::ExitStatus>,
    /// Last (rows, cols) applied to master + parser; skips no-op resizes.
    size: (u16, u16),
    /// Repo name captured at spawn, shown in the panel title for the whole
    /// life of the session even if the selection moves elsewhere.
    pub repo_label: String,
}

impl TerminalSession {
    /// Spawn `$SHELL` (fallback `/bin/sh`) in a fresh PTY at `cwd`.
    pub fn spawn(
        cwd: &std::path::Path,
        repo_label: String,
        rows: u16,
        cols: u16,
    ) -> Result<Self, String> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let mut cmd = portable_pty::CommandBuilder::new(&shell);
        cmd.cwd(cwd);
        cmd.env("GITWIG", "1");
        cmd.env("GITWIG_SHELL", "1");
        cmd.env("TERM", "xterm-256color");
        Self::spawn_command(cmd, repo_label, rows, cols)
    }

    /// Spawn an arbitrary command in a fresh PTY. Seam for integration tests.
    pub fn spawn_command(
        cmd: portable_pty::CommandBuilder,
        repo_label: String,
        rows: u16,
        cols: u16,
    ) -> Result<Self, String> {
        let rows = rows.max(MIN_ROWS);
        let cols = cols.max(MIN_COLS);
        let pty_system = portable_pty::native_pty_system();
        let pair = pty_system
            .openpty(portable_pty::PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| format!("Could not open PTY: {}", e))?;

        let child =
            pair.slave.spawn_command(cmd).map_err(|e| format!("Could not start shell: {}", e))?;
        // Keep only the child holding the slave end, so closing the master
        // later reliably delivers EOF/SIGHUP.
        drop(pair.slave);

        let writer =
            pair.master.take_writer().map_err(|e| format!("Could not open PTY writer: {}", e))?;
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| format!("Could not open PTY reader: {}", e))?;

        let parser = Arc::new(Mutex::new(vt100::Parser::new(rows, cols, SCROLLBACK_LINES)));
        let exited = Arc::new(AtomicBool::new(false));
        spawn_reader(reader, Arc::clone(&parser), Arc::clone(&exited));

        Ok(Self {
            parser,
            master: pair.master,
            writer,
            child,
            exited,
            exit_status: None,
            size: (rows, cols),
            repo_label,
        })
    }

    /// Poison-safe access to the parser; the single lock path everywhere.
    pub fn lock_parser(&self) -> MutexGuard<'_, vt100::Parser> {
        self.parser.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Resize the PTY and the parser grid. No-op when unchanged.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        let rows = rows.max(MIN_ROWS);
        let cols = cols.max(MIN_COLS);
        if (rows, cols) == self.size {
            return;
        }
        self.size = (rows, cols);
        // Failure is non-fatal: the shell just keeps its old size.
        let _ = self.master.resize(portable_pty::PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
        self.lock_parser().screen_mut().set_size(rows, cols);
    }

    /// Whether the shell is gone (reader saw EOF, or a write failed).
    pub fn exited(&self) -> bool {
        self.exited.load(Ordering::Relaxed)
    }
}

impl TerminalSession {
    /// Called once per run-loop iteration. Reaps the child after the reader
    /// saw EOF. Returns `true` exactly once, when the exit is first observed.
    pub fn poll_exit(&mut self) -> bool {
        if !self.exited() || self.exit_status.is_some() {
            return false;
        }
        match self.child.try_wait() {
            Ok(Some(status)) => {
                self.exit_status = Some(status);
                true
            }
            // EOF seen but the child has not been reaped yet; retry next frame.
            Ok(None) => false,
            Err(_) => {
                self.exit_status = Some(portable_pty::ExitStatus::with_exit_code(1));
                true
            }
        }
    }

    /// Exit code of the shell, once reaped.
    pub fn exit_code(&self) -> Option<u32> {
        self.exit_status.as_ref().map(portable_pty::ExitStatus::exit_code)
    }

    /// Write raw bytes to the shell. A broken pipe converges on the exited path.
    pub fn send_bytes(&mut self, bytes: &[u8]) {
        if self.exited() {
            return;
        }
        use std::io::Write;
        if self.writer.write_all(bytes).and_then(|_| self.writer.flush()).is_err() {
            self.exited.store(true, Ordering::Relaxed);
        }
    }

    /// Encode and forward one keystroke; snaps the view back to the live screen.
    pub fn send_key(&mut self, key: KeyEvent) {
        let app_cursor = self.lock_parser().screen().application_cursor();
        if let Some(bytes) = encode_key(key, app_cursor) {
            self.lock_parser().screen_mut().set_scrollback(0);
            self.send_bytes(&bytes);
        }
    }

    /// Forward pasted text, honoring bracketed-paste mode when the shell
    /// enabled it. The end-paste marker is stripped from the payload so a
    /// malicious paste cannot break out of the bracket.
    pub fn send_paste(&mut self, text: &str) {
        let sanitized = text.replace("\x1b[201~", "");
        let bracketed = self.lock_parser().screen().bracketed_paste();
        self.lock_parser().screen_mut().set_scrollback(0);
        if bracketed {
            self.send_bytes(b"\x1b[200~");
            let payload = sanitized.into_bytes();
            self.send_bytes(&payload);
            self.send_bytes(b"\x1b[201~");
        } else {
            self.send_bytes(sanitized.as_bytes());
        }
    }

    /// Move the scrollback view by `delta` rows (positive = further back).
    pub fn scroll_by(&mut self, delta: isize) {
        let mut guard = self.lock_parser();
        let screen = guard.screen_mut();
        let current = screen.scrollback();
        let next = if delta >= 0 {
            current.saturating_add(delta as usize)
        } else {
            current.saturating_sub(delta.unsigned_abs())
        };
        screen.set_scrollback(next);
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if !self.exited() && self.exit_status.is_none() {
            let _ = self.child.kill();
        }
        // Reap so no zombie outlives the session; returns promptly after
        // kill or a prior exit. Dropping master/writer then closes the PTY,
        // which unblocks the detached reader thread.
        let _ = self.child.wait();
    }
}

/// Detached PTY output pump: reads until EOF, feeding the shared parser.
fn spawn_reader(
    mut reader: Box<dyn std::io::Read + Send>,
    parser: Arc<Mutex<vt100::Parser>>,
    exited: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    parser.lock().unwrap_or_else(PoisonError::into_inner).process(&buf[..n]);
                }
            }
        }
        exited.store(true, Ordering::Relaxed);
    });
}

/// Translate a crossterm key event into the byte sequence a terminal would
/// send (legacy xterm encoding). `None` means the key has no terminal
/// representation and is swallowed.
pub fn encode_key(key: KeyEvent, application_cursor: bool) -> Option<Vec<u8>> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);

    // xterm modifier parameter: 1 + shift(1) + alt(2) + ctrl(4).
    let mod_param = 1 + u8::from(shift) + 2 * u8::from(alt) + 4 * u8::from(ctrl);

    // CSI 1;{m}X for arrows/Home/End, CSI {n};{m}~ for the tilde keys.
    let csi_modified = |c: u8| format!("\x1b[1;{}{}", mod_param, c as char).into_bytes();
    let tilde = |n: u8| {
        if mod_param > 1 {
            format!("\x1b[{};{}~", n, mod_param).into_bytes()
        } else {
            format!("\x1b[{}~", n).into_bytes()
        }
    };
    let arrow = |c: u8| {
        if mod_param > 1 {
            csi_modified(c)
        } else if application_cursor {
            vec![0x1b, b'O', c]
        } else {
            vec![0x1b, b'[', c]
        }
    };

    let bytes = match key.code {
        KeyCode::Char(c) => {
            let mut out = Vec::new();
            if alt {
                out.push(0x1b);
            }
            if ctrl {
                out.push(ctrl_byte(c)?);
            } else {
                let mut utf8 = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut utf8).as_bytes());
            }
            out
        }
        KeyCode::Enter => {
            if alt {
                vec![0x1b, b'\r']
            } else {
                vec![b'\r']
            }
        }
        KeyCode::Tab => vec![b'\t'],
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Backspace => {
            let base = if ctrl { 0x08 } else { 0x7f };
            if alt { vec![0x1b, base] } else { vec![base] }
        }
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => arrow(b'A'),
        KeyCode::Down => arrow(b'B'),
        KeyCode::Right => arrow(b'C'),
        KeyCode::Left => arrow(b'D'),
        KeyCode::Home => {
            if mod_param > 1 {
                csi_modified(b'H')
            } else if application_cursor {
                b"\x1bOH".to_vec()
            } else {
                b"\x1b[H".to_vec()
            }
        }
        KeyCode::End => {
            if mod_param > 1 {
                csi_modified(b'F')
            } else if application_cursor {
                b"\x1bOF".to_vec()
            } else {
                b"\x1b[F".to_vec()
            }
        }
        KeyCode::Insert => tilde(2),
        KeyCode::Delete => tilde(3),
        KeyCode::PageUp => tilde(5),
        KeyCode::PageDown => tilde(6),
        KeyCode::F(n @ 1..=4) => vec![0x1b, b'O', b'P' + (n - 1)],
        KeyCode::F(n @ 5..=12) => {
            let code = match n {
                5 => 15,
                6 => 17,
                7 => 18,
                8 => 19,
                9 => 20,
                10 => 21,
                11 => 23,
                _ => 24,
            };
            tilde(code)
        }
        _ => return None,
    };
    Some(bytes)
}

/// Control-key byte for `ctrl+<c>`, per the ASCII control-character mapping.
fn ctrl_byte(c: char) -> Option<u8> {
    match c {
        'a'..='z' => Some(c as u8 & 0x1f),
        'A'..='Z' => Some(c.to_ascii_lowercase() as u8 & 0x1f),
        ' ' | '@' => Some(0x00),
        '[' => Some(0x1b),
        '\\' => Some(0x1c),
        ']' => Some(0x1d),
        '^' => Some(0x1e),
        '_' => Some(0x1f),
        '?' => Some(0x7f),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn key_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    fn enc(code: KeyCode) -> Vec<u8> {
        encode_key(key(code), false).expect("mapped key")
    }

    #[test]
    fn encodes_plain_chars_and_controls() {
        assert_eq!(enc(KeyCode::Char('a')), b"a");
        assert_eq!(enc(KeyCode::Char('é')), "é".as_bytes());
        assert_eq!(enc(KeyCode::Enter), b"\r");
        assert_eq!(enc(KeyCode::Tab), b"\t");
        assert_eq!(enc(KeyCode::BackTab), b"\x1b[Z");
        assert_eq!(enc(KeyCode::Backspace), &[0x7f]);
        assert_eq!(enc(KeyCode::Esc), &[0x1b]);
    }

    #[test]
    fn encodes_ctrl_and_alt_chars() {
        let c = |ch| encode_key(key_mod(KeyCode::Char(ch), KeyModifiers::CONTROL), false);
        assert_eq!(c('c'), Some(vec![0x03]));
        assert_eq!(c('d'), Some(vec![0x04]));
        assert_eq!(c('z'), Some(vec![0x1a]));
        assert_eq!(c(' '), Some(vec![0x00]));
        assert_eq!(c('?'), Some(vec![0x7f]));
        assert_eq!(
            encode_key(key_mod(KeyCode::Char('x'), KeyModifiers::ALT), false),
            Some(b"\x1bx".to_vec())
        );
        assert_eq!(
            encode_key(
                key_mod(KeyCode::Char('c'), KeyModifiers::ALT | KeyModifiers::CONTROL),
                false
            ),
            Some(vec![0x1b, 0x03])
        );
    }

    #[test]
    fn encodes_arrows_and_navigation() {
        assert_eq!(enc(KeyCode::Up), b"\x1b[A");
        assert_eq!(enc(KeyCode::Left), b"\x1b[D");
        // Application-cursor mode switches to SS3.
        assert_eq!(encode_key(key(KeyCode::Up), true), Some(b"\x1bOA".to_vec()));
        assert_eq!(encode_key(key(KeyCode::Home), true), Some(b"\x1bOH".to_vec()));
        assert_eq!(enc(KeyCode::Home), b"\x1b[H");
        assert_eq!(enc(KeyCode::End), b"\x1b[F");
        assert_eq!(enc(KeyCode::PageUp), b"\x1b[5~");
        assert_eq!(enc(KeyCode::PageDown), b"\x1b[6~");
        assert_eq!(enc(KeyCode::Insert), b"\x1b[2~");
        assert_eq!(enc(KeyCode::Delete), b"\x1b[3~");
    }

    #[test]
    fn encodes_modified_arrows_with_xterm_parameter() {
        // shift=+1, alt=+2, ctrl=+4 on top of the base 1.
        assert_eq!(
            encode_key(key_mod(KeyCode::Up, KeyModifiers::SHIFT), false),
            Some(b"\x1b[1;2A".to_vec())
        );
        assert_eq!(
            encode_key(key_mod(KeyCode::Right, KeyModifiers::CONTROL), false),
            Some(b"\x1b[1;5C".to_vec())
        );
        assert_eq!(
            encode_key(
                key_mod(
                    KeyCode::Left,
                    KeyModifiers::SHIFT | KeyModifiers::ALT | KeyModifiers::CONTROL
                ),
                false
            ),
            Some(b"\x1b[1;8D".to_vec())
        );
        assert_eq!(
            encode_key(key_mod(KeyCode::Delete, KeyModifiers::SHIFT), false),
            Some(b"\x1b[3;2~".to_vec())
        );
    }

    #[test]
    fn encodes_function_keys() {
        assert_eq!(enc(KeyCode::F(1)), b"\x1bOP");
        assert_eq!(enc(KeyCode::F(4)), b"\x1bOS");
        assert_eq!(enc(KeyCode::F(5)), b"\x1b[15~");
        assert_eq!(enc(KeyCode::F(12)), b"\x1b[24~");
    }

    #[test]
    fn unmapped_keys_are_swallowed() {
        assert_eq!(encode_key(key(KeyCode::CapsLock), false), None);
        assert_eq!(encode_key(key(KeyCode::F(20)), false), None);
    }

    #[cfg(unix)]
    #[test]
    fn pty_session_runs_command_and_exits() {
        let mut cmd = portable_pty::CommandBuilder::new("/bin/sh");
        cmd.args(["-c", "printf gitwig_pty_hello"]);
        let mut session = TerminalSession::spawn_command(cmd, "test".to_string(), 10, 60)
            .expect("spawn PTY session");

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        loop {
            let contents = session.lock_parser().screen().contents();
            if contents.contains("gitwig_pty_hello") {
                break;
            }
            assert!(std::time::Instant::now() < deadline, "output never arrived: {contents:?}");
            std::thread::sleep(std::time::Duration::from_millis(25));
        }

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        loop {
            session.poll_exit();
            if session.exited() {
                break;
            }
            assert!(std::time::Instant::now() < deadline, "shell never exited");
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
    }
}
