# Twig - Project Instructions

## Project Overview
Twig is a Rust-based Terminal User Interface (TUI) for Git, aiming to provide a SourceTree-like experience in the terminal.

## Core Mandates
- **Performance:** Must be fast even in large repositories.
- **Reliability:** Git operations must be safe and atomic.
- **Intuitive UI:** Navigation should be familiar to users of SourceTree and other Git GUIs.

## Tech Stack
- **Language:** Rust (Edition 2024)
- **TUI Framework:** `ratatui` (0.30) with the `crossterm_0_29` feature, paired with `crossterm` 0.29 for terminal control and input events.
- **Git Backend:** To be decided (prefer `git2-rs` or `gix`).
- **Configuration:** TOML based.

## Architectural Patterns
- **State Management:** Clear separation between Git state and UI state.
- **Async Operations:** Long-running Git commands (fetch, clone, large diffs) should not block the UI thread.
- **Component-based UI:** Use modular widgets for different views (History, Working Tree, Branches).

## Development Workflow
- Follow the Roadmap in `.agent/ROADMAP.md`.
- Ensure all new features are accompanied by tests.
- Use `cargo fmt` and `cargo clippy` before every commit.

## Agent Resources
Detailed instructions and roadmap for AI agents can be found in the `.agent/` directory.
