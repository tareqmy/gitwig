---
name: add-keybinding
description: Checklist for adding, rebinding, or removing a Gitwig keyboard shortcut — the exact set of files that must be updated atomically so key routing, the status bar, help overlays, and docs never drift apart. Trigger whenever a task adds, changes, or removes a keybinding.
---

# Add/Change a Keybinding

Gitwig's key handling is mode-dependent (the `Mode` enum in `src/app/mod.rs` — don't
assume its variants, read the source; the list is long and changes often). A keybinding
isn't done when the key does the thing — it's done when every place a user could look to
learn about it agrees with the code.

## Files to update atomically (`.agent/INSTRUCTIONS.md` §3)

1. **`src/input.rs`** — the `handle_key` route that matches the key to an action for the
   relevant `Mode`.
2. **The relevant `src/app/*.rs`** (`mod.rs`, `actions.rs`, `git.rs`, `workspace.rs`, or
   `navigation.rs`) — the state mutation the key triggers.
3. **`src/popups/help.rs`** or **`src/popups/detail_help.rs`** — the help-overlay line for
   the key, in whichever overlay is active in that mode.
4. **`src/components/cmd_bar/`** (`mod.rs`, or the mode-specific `main.rs` / `detail.rs` /
   `popups.rs`) — the status-bar hint entries shown at the bottom of the screen.

Miss any one of these and the UI lies to the user about what a key does — the status bar
and help overlay are the app's only real-time indicators of available actions, so drift
here is a visible bug, not a nitpick.

## Docs — same commit (see the `sync-docs` skill)
- `docs/keybindings.md` — add the binding to its mode's table.
- `docs/panels.md` — if the binding is specific to a panel described there.

## Before committing
Run the `rust-quality` skill's checks, and add or update a test in `src/app/tests.rs`
covering the new key's effect (`.agent/INSTRUCTIONS.md` §5: every method gets a test).
