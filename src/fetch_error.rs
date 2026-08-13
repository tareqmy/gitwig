//! Classification of `git fetch` failures into actionable categories.
//!
//! Raw git/ssh stderr is verbose, multi-line, and frequently carries ANSI escapes
//! and progress carriage returns — none of which can be dropped into a card label
//! or a status bar safely. This module reduces that noise to a small enum with a
//! compact card label plus a one-line human explanation, while preserving the full
//! (sanitised) text for the details popup.

/// The reason a fetch against a remote did not succeed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchFailure {
    /// Credentials were rejected, missing, or the account lacks permission.
    Auth,
    /// The remote's host key is unknown, changed, or verification failed.
    HostKey,
    /// The remote exists as a URL but not as a repository we may see.
    NotFound,
    /// DNS, routing, TLS or connection-level failure.
    Network,
    /// The repository has no remote (or no remote for the current branch).
    NoRemote,
    /// Killed after exceeding the configured fetch timeout.
    Timeout,
    /// Local git problem — lock file, corrupt ref, non-fast-forward, etc.
    Local,
    /// Nothing matched; show the raw text.
    Unknown,
}

impl FetchFailure {
    /// Compact label for the repository card. Kept short so it does not wrap the
    /// status column on narrow terminals.
    pub fn label(self) -> &'static str {
        match self {
            FetchFailure::Auth => "auth denied",
            FetchFailure::HostKey => "host key",
            FetchFailure::NotFound => "not found",
            FetchFailure::Network => "unreachable",
            FetchFailure::NoRemote => "no remote",
            FetchFailure::Timeout => "timed out",
            FetchFailure::Local => "local error",
            FetchFailure::Unknown => "failed",
        }
    }

    /// One-line explanation with the most likely remedy, for the status bar and
    /// as the heading of the details popup.
    pub fn summary(self) -> &'static str {
        match self {
            FetchFailure::Auth => {
                "Authentication failed or you lack permission on this remote. Check your \
                 credentials, SSH key, or token scope."
            }
            FetchFailure::HostKey => {
                "The remote's host key could not be verified. Connect to the host once outside \
                 Gitwig to record its fingerprint."
            }
            FetchFailure::NotFound => {
                "The remote repository does not exist or is not visible to this account. Check \
                 the remote URL."
            }
            FetchFailure::Network => {
                "The remote could not be reached. Check your network, VPN, or proxy settings."
            }
            FetchFailure::NoRemote => {
                "This repository has no remote configured, so there is nothing to fetch."
            }
            FetchFailure::Timeout => {
                "The remote accepted the connection but did not respond in time. Raise \
                 fetch_timeout_secs if the host is simply slow."
            }
            FetchFailure::Local => {
                "Git refused the fetch for a local reason. See the details below."
            }
            FetchFailure::Unknown => "The fetch failed. See the details below.",
        }
    }
}

/// Strips ANSI escape sequences and progress carriage returns, collapses blank
/// lines, and trims each line.
///
/// git writes transfer progress as `\r`-separated redraws of the same line; left
/// intact these render as a single unreadable smear inside a ratatui `Paragraph`.
pub fn sanitize(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // CSI sequence: consume up to and including the final byte.
            if chars.peek() == Some(&'[') {
                chars.next();
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        // Treat a carriage return as a line break; the dedupe below discards the
        // superseded progress frames.
        if c == '\r' {
            out.push('\n');
            continue;
        }
        if c.is_control() && c != '\n' && c != '\t' {
            continue;
        }
        out.push(c);
    }

    let mut lines: Vec<&str> = Vec::new();
    for line in out.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Drop the intermediate frames of a progress counter.
        let current_prefix = progress_prefix(trimmed);
        if current_prefix.is_some() {
            let superseded = match lines.last() {
                Some(last) => progress_prefix(last) == current_prefix,
                None => false,
            };
            if superseded {
                lines.pop();
            }
        }
        lines.push(trimmed);
    }
    lines.join("\n")
}

/// The label part of a `remote: Counting objects:  42%` style progress line, used
/// to recognise successive redraws of the same counter.
fn progress_prefix(line: &str) -> Option<&str> {
    if !line.contains('%') {
        return None;
    }
    // Split on the last colon so "remote: Counting objects" and
    // "remote: Compressing objects" are treated as different counters rather
    // than collapsing into a shared "remote" prefix.
    line.rsplit_once(':').map(|(head, _)| head.trim())
}

/// Maps git's stderr onto a [`FetchFailure`].
///
/// Ordering matters: an ssh permission failure also mentions the host, and a
/// missing repository on GitHub is reported as an authentication error to avoid
/// leaking private repository names — so the most specific patterns are tested
/// first.
pub fn classify(stderr: &str) -> FetchFailure {
    let s = stderr.to_ascii_lowercase();

    if s.contains("host key verification failed")
        || s.contains("remote host identification has changed")
        || s.contains("no matching host key")
        || s.contains("key verification failed")
    {
        return FetchFailure::HostKey;
    }

    if s.contains("permission denied")
        || s.contains("authentication failed")
        || s.contains("could not read username")
        || s.contains("could not read password")
        || s.contains("terminal prompts disabled")
        || s.contains("invalid username or password")
        || s.contains("access denied")
        || s.contains("403 forbidden")
        || s.contains("401 unauthorized")
        // git reports HTTP failures as "The requested URL returned error: 403",
        // which matches none of the phrasings above and would otherwise fall
        // through to the network block.
        || s.contains("error: 401")
        || s.contains("error: 403")
        || s.contains("publickey")
        || s.contains("authentication is not supported")
        || s.contains("support for password authentication was removed")
    {
        return FetchFailure::Auth;
    }

    if s.contains("repository not found")
        || s.contains("404 not found")
        || s.contains("error: 404")
        || s.contains("does not appear to be a git repository")
        || s.contains("not found: did you run git update-server-info")
    {
        return FetchFailure::NotFound;
    }

    // Must precede the network block: our own cancellation message would
    // otherwise be swallowed by the generic "timed out" patterns below.
    if s.contains("did not respond within") && s.contains("cancelled") {
        return FetchFailure::Timeout;
    }

    if s.contains("could not resolve host")
        || s.contains("connection refused")
        || s.contains("connection timed out")
        || s.contains("connection reset")
        || s.contains("network is unreachable")
        || s.contains("no route to host")
        || s.contains("failed to connect")
        || s.contains("unable to access")
        || s.contains("ssl certificate problem")
        || s.contains("tls certificate")
        || s.contains("proxy")
        || s.contains("temporary failure in name resolution")
        || s.contains("operation timed out")
    {
        return FetchFailure::Network;
    }

    if s.contains("no remote repository specified")
        || s.contains("does not appear to have a remote")
        || s.contains("no such remote")
        || s.contains("no configured push destination")
    {
        return FetchFailure::NoRemote;
    }

    if s.contains("index.lock")
        || s.contains("unable to create")
        || s.contains("cannot lock ref")
        || s.contains("not a git repository")
        || s.contains("would clobber existing tag")
    {
        return FetchFailure::Local;
    }

    FetchFailure::Unknown
}
