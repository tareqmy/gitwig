//! Gitwig entry point — thin wrapper around `gitwig::run`.
//!
//! Application logic lives in the `gitwig` library crate (`src/lib.rs`),
//! shared with the `gtg` binary (`src/bin/gtg.rs`).

fn main() -> Result<(), Box<dyn std::error::Error>> {
    gitwig::run()
}
