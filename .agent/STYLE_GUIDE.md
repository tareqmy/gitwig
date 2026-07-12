# Gitwig Style Guide

## Rust Coding Standards
- **Naming:** Follow standard Rust naming conventions (`snake_case` for functions/variables, `PascalCase` for structs/enums).
- **Errors:** Use `thiserror` or `anyhow` for error handling (or standard `Box<dyn Error>` for top-level). Prefer specific error enums for library logic.
- **No Panics in Hot Paths:** Avoid `.unwrap()` or `.expect()` inside the main rendering or event loop. Handle failures gracefully by appending to an in-app error log or the transient status-bar notification.
- **Documentation:** Use doc comments (`///`) for public items. Explain *why*, not just *what*. Use a top-of-file `//!` doc comment to state each module's single responsibility.
- **Imports:** Group imports: std, external crates, internal modules. Use curly braces for multiple items from the same crate.
- **File Size:** Aim to keep files under ~300 lines. When one grows beyond that, extract along a single responsibility line (state vs. rendering vs. input vs. persistence). See the Module Layout section of `.agent/INSTRUCTIONS.md` for the current map and the splitting rules.

## TUI Patterns
- **Immediacy:** The UI should feel snappy. Avoid unnecessary redraws, but ensure state changes are reflected immediately.
- **Keybindings:** Use standard TUI keybindings where appropriate (e.g., `q` for quit, `j`/`k` or arrows for navigation).
- **Layout:** Use `ratatui`'s `Layout` and `Constraint` system to handle terminal resizing gracefully.
- **Widgets:** Keep widgets stateless where possible. Pass the necessary state during the `render` call.
- **State vs. Widgets:** Keep long-lived data and component states (e.g., `ListState`, `TableState`) inside the core `App` or component structs. Widgets are short-lived, ephemeral view structures constructed on-the-fly during each rendering frame. State modifications must only occur within event-handling functions — never mutate state inside rendering functions.
- **Draw Cycle Efficiency:** Trust ratatui's double-buffered frame diffing. Avoid creating custom buffers or copying large datasets inside the rendering loop. Pass data to widgets via lightweight references (`&str` or cloned slices) rather than transferring ownership.
- **Theme:** Centralize color and border choices as `const`s at the top of the module (`ACCENT`, `WARNING`, `DANGER`, `CARD_BORDER`) plus the style helpers (`muted_style`, `primary_style`, `accent_style`). Use rounded borders (`BorderType::Rounded`) by default — they read as modern and avoid the heavy "box-in-box" effect of double or thick borders.
- **Background-agnostic text:** Never hard-code `Color::White`, `Color::Gray`, `Color::DarkGray`, or `Color::Black` for regular text or borders — pick whichever looks fine on your dev terminal and it will be unreadable on the opposite background. Leave the foreground at the terminal default and use `Modifier::DIM` for muted text and `Modifier::BOLD` for emphasis. Explicit foreground colors are reserved for accents (cyan / yellow / red) and badge foregrounds (where the badge has its own solid background).
- **Selection feedback:** Indicate the selected element with three reinforcing signals — a left-edge `▌` mark in the accent color, an accent-colored border, and bold primary text. Switch the border color (not the text color) to communicate pending destructive or text-edit actions.
- **Status bar:** Use a `MODE` badge with a colored background on the left and muted contextual text on the right, separated by ` ⟩ ` (or ` > ` in compatibility mode). Use the real terminal cursor (`Frame::set_cursor_position`) in input modes rather than a fake `_` character — it tracks blink and theme correctly.

## Git Integration
- **Atomicity:** Ensure that any multi-step Git operation (like a merge or rebase) handles failures gracefully and doesn't leave the repo in a broken state if possible.
- **Performance:** For large lists (like a commit log), use lazy loading or pagination if necessary. `ratatui`'s `List` or `Table` can handle many items if used correctly.
