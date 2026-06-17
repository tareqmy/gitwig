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
- **Modal Input:** The app uses a `Mode` enum (`Normal`, `Adding`, `Editing`, `ConfirmDelete`, `Help`) defined in `src/app.rs` to interpret keystrokes. When adding a new keybinding: add the route in `src/input.rs` (`handle_key`), add a corresponding `App` method in `src/app.rs` if it mutates state, add an entry to the `HELP_LINES` constant in `src/ui.rs` (the source of truth for the `?` overlay), and update the status-bar text in `src/ui.rs::draw_status_bar` — all in the same change.
- **Visual Theme:** Pull every color, border-type, and selection marker from the `Theme` constants block at the top of `src/ui.rs` (`ACCENT`, `WARNING`, `DANGER`, `CARD_BORDER`, `SELECTION_MARK`) plus the style helpers (`muted_style`, `primary_style`, `accent_style`). Do not inline raw `Color::Cyan` or `BorderType::Rounded` calls in widget code — add or reuse a constant. **Never hard-code `Color::White`, `Color::Gray`, `Color::Black`, or `Color::DarkGray` for plain text or borders** — they invert visibility between light and dark terminal backgrounds (light gray vanishes on light bg, dark gray vanishes on dark bg). Instead, leave the foreground at the terminal default (`Style::default()`) and use `Modifier::DIM` for muted text and `Modifier::BOLD` for emphasis — the terminal renders these correctly on either theme. Specific fg colors are only acceptable for: accents (`ACCENT`, `WARNING`, `DANGER`), and badge foregrounds where the badge has its own solid background to provide contrast. Selection is communicated through three layers: the left `▌` marker, the accent border color, and bold text — keep all three in sync if you change the look. Mode-dependent border colors (cyan = selected, yellow = editing, red = confirm-delete) are the user's primary feedback that a destructive or text-input action is pending.
- **Config Persistence:** `load_config` returns `(Config, PathBuf)` where the path is the destination for `save_config`. Any mutation of `Config` from the UI must be followed by a `save_config` call so disk and memory don't diverge. Surface save errors via the transient status-bar message rather than crashing. The shared `App::persist` helper does this — prefer it over inline `save_config` calls.

## Module Layout
The crate is organized so each file has a single clear responsibility. Keep it that way as the codebase grows.

- `src/main.rs` — entry point only. Terminal setup/teardown and the call into `app::run`. **Should stay small** (under ~80 lines). No state, no rendering, no key handling here.
- `src/app.rs` — application state (`App` struct), the `Mode` enum, state-mutation methods, and the event loop (`run`). All mutation lives here.
- `src/ui.rs` — pure rendering. Reads `&App`, writes to a `Frame`. Owns `HELP_LINES` and the help-overlay layout. Never mutates state.
- `src/input.rs` — keystroke dispatch. Maps `(Mode, KeyCode)` to `App` method calls and signals quit via a `bool` return. No business logic here — just routing.
- `src/config.rs` — TOML load/save plus the `Config` struct. No UI awareness.

### When to split a file further
- If any file grows past **~300 lines**, look for an extraction. Common split lines: a new mode that has its own state/rendering, a widget that has its own builder logic, a new config concern (e.g. theme, keybinding overrides).
- A new view (e.g. a History panel) should get its own file under `src/ui/` (promote `ui.rs` to `ui/mod.rs` when this happens).
- A new domain concept (e.g. `Repository`, `Branch`) should get its own module, not be jammed into `app.rs`.
- Prefer adding a new module over expanding an existing one when the new code doesn't share state with what's already there.

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
