//! `gtg` — short alias entry point for Gitwig, identical to `gitwig`.
//!
//! Kept as its own compiled binary (rather than a symlink) so that
//! `cargo install gitwig` installs both names directly. Every other
//! distribution channel installs `gitwig` and symlinks/copies `gtg` to it
//! post-install; see `gitwig::run` for the shared implementation.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    gitwig::run()
}
