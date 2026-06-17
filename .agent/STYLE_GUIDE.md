# Twig Style Guide

## Rust Coding Standards
- **Naming:** Follow standard Rust naming conventions (`snake_case` for functions/variables, `PascalCase` for structs/enums).
- **Errors:** Use `thiserror` or `anyhow` for error handling (or standard `Box<dyn Error>` for top-level). Prefer specific error enums for library logic.
- **Documentation:** Use doc comments (`///`) for public items. Explain *why*, not just *what*.
- **Imports:** Group imports: std, external crates, internal modules. Use curly braces for multiple items from the same crate.

## TUI Patterns
- **Immediacy:** The UI should feel snappy. Avoid unnecessary redraws, but ensure state changes are reflected immediately.
- **Keybindings:** Use standard TUI keybindings where appropriate (e.g., `q` for quit, `j`/`k` or arrows for navigation).
- **Layout:** Use `ratatui`'s `Layout` and `Constraint` system to handle terminal resizing gracefully.
- **Widgets:** Keep widgets stateless where possible. Pass the necessary state during the `render` call.

## Git Integration
- **Atomicity:** Ensure that any multi-step Git operation (like a merge or rebase) handles failures gracefully and doesn't leave the repo in a broken state if possible.
- **Performance:** For large lists (like a commit log), use lazy loading or pagination if necessary. `ratatui`'s `List` or `Table` can handle many items if used correctly.
