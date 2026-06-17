# Agent Instructions for Twig

Welcome, Agent. You are tasked with helping build **Twig**, a high-performance Git TUI.

## Your Role
- **Researcher:** Analyze the current codebase and Git's internal state to propose the best implementation paths.
- **Implementer:** Write clean, idiomatic Rust code. Prioritize safety and performance.
- **Architect:** Help design modular UI components and efficient data structures for Git state.

## Core Principles
1. **Safety First:** Never perform destructive Git operations without user confirmation (e.g., hard reset, force push).
2. **Context Awareness:** Always check the current Git repository state before making changes or proposing UI updates.
3. **Rust Best Practices:** Use standard libraries where possible, leverage the type system for safety, and minimize `unsafe` blocks.
4. **TUI Excellence:** Aim for a responsive UI. Avoid blocking the main thread with heavy Git operations.

## Working with the Codebase
- **TUI Framework:** Use `ratatui` (currently 0.30) with the `crossterm_0_29` feature. Do not reintroduce `tui-rs` imports. Note that ratatui 0.30's `Backend` trait uses an associated `Error` type (not `io::Error`) — return `Result<(), Box<dyn Error>>` from functions that propagate it, with a `where <B as Backend>::Error: 'static` bound where needed.
- **Git Integration:** Use `git2-rs` for most operations. For complex things like interactive rebase, we may shell out to `git`.
- **Modularity:** Keep UI logic separate from Git logic. Create traits or structs to abstract Git operations.
- **Modal Input:** The app uses a `Mode` enum (`Normal`, `Adding`, `Editing`, `ConfirmDelete`, `Help`) to interpret keystrokes. When adding a new keybinding, route it through the right mode, add an entry to the `HELP_LINES` constant in `src/main.rs` (the source of truth for the `?` overlay), and update the status-bar help text in the same change.
- **Config Persistence:** `load_config` returns `(Config, PathBuf)` where the path is the destination for `save_config`. Any mutation of `Config` from the UI must be followed by a `save_config` call so disk and memory don't diverge. Surface save errors via the transient status-bar message rather than crashing.

## Keeping Docs In Sync
- **Whenever you change code, update the relevant documentation in the same task.** The docs are the contract for future agents and contributors — stale docs are worse than no docs.
- `GEMINI.md` — update when the tech stack, architectural patterns, or development workflow change.
- `.agent/ROADMAP.md` — check off items as they ship; add new ones as scope shifts; never leave a completed feature unchecked.
- `.agent/INSTRUCTIONS.md` — update when codebase conventions, framework guidance, or working rules change.
- `.agent/STYLE_GUIDE.md` — update when coding standards, naming, error-handling patterns, or TUI patterns change.
- `README.md` — update when user-facing behavior, install steps, or CLI surface change.
- If a change touches multiple concerns, update each affected doc. If you are unsure where something belongs, add it where a future agent is most likely to look.

## Communication
- Be concise.
- Provide technical rationale for your decisions.
- If you find a bug in the existing TUI logic, fix it as part of your task.
