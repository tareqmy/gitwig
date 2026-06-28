# Gitwig - Project Instructions

## Project Overview
Gitwig is a Rust-based Terminal User Interface (TUI) for Git, aiming to provide a SourceTree-like experience in the terminal.

## Core Mandates
- **Performance:** Must be fast even in large repositories.
- **Reliability:** Git operations must be safe and atomic.
- **Intuitive UI:** Navigation should be familiar to users of SourceTree and other Git GUIs.

## Tech Stack
- **Language:** Rust (Edition 2024)
- **TUI Framework:** `ratatui` (0.30) with the `crossterm_0_29` feature, paired with `crossterm` 0.29 for terminal control and input events.
- **Git Backend:** `git2` (libgit2 bindings), default features off — used for both the per-card status indicator (`inspect_summary`) and the on-demand Detail view (`inspect_detail`). Both paths share a single `collect_summary` helper inside `repo.rs`.
- **Configuration:** TOML based. Editable in-app — the UI persists adds/edits/deletes back to the file it loaded from.

## Architectural Patterns
- **State Management:** Clear separation between Git state and UI state.
- **Async Operations:** Long-running Git commands (fetch, clone, large diffs) should not block the UI thread.
- **Component-based UI:** Use modular widgets for different views (History, Working Tree, Branches).
- **Modal Input:** Keystroke meaning is mode-dependent (Normal / Adding / Editing / ConfirmDelete / Help / Detail / DetailHelp / About / CommitInput / BranchCreateInput / TagCreateInput / BranchDeleteConfirm / BranchPushConfirm / BranchMergeConfirm / BranchRebaseConfirm / BranchInteractiveRebaseConfirm / TagDeleteConfirm / TagPushConfirm / TagPushAllConfirm / StashDeleteConfirm / StashApplyConfirm / RemotePicker / CommitSearchInput / Inspect / RepoSearchInput). The status bar always reflects the current mode so the user can recover orientation at a glance.
- **Single-responsibility modules:**
  - `main.rs` (entry)
  - `src/app/` (application state and orchestration split into `mod.rs`, `actions.rs`, `git.rs`, `workspace.rs`, `navigation.rs`, and `tests.rs`)
  - `src/ui/` (theme, styles, and layout config)
  - `src/tabs/`, `src/popups/`, `src/components/` (componentized UI drawing logic)
  - `src/input.rs` (event routing dispatcher)
  - `src/config.rs` (TOML load/save)
  - `gitwig-core` workspace crate (all repository inspection, isolated from UI dependencies)

## Development Workflow
- Follow the Roadmap in `.agent/ROADMAP.md`.
- Ensure all new features are accompanied by tests.
- Use `cargo fmt` and `cargo clippy` before every commit.
- Keep documentation in lockstep with code — see `.agent/INSTRUCTIONS.md` for the rule.

## Agent Resources
Detailed instructions and roadmap for AI agents can be found in the `.agent/` directory.
