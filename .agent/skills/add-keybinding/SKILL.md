---
name: add-keybinding
description: Checklist for adding, rebinding, or removing a Gitwig keyboard shortcut so key routing, the status bar, help overlays, and docs never drift apart.
---
# Add/Change a Keybinding

_Mirrored for Claude Code at `.claude/skills/add-keybinding/SKILL.md` — keep both in sync._

Gitwig's key handling is mode-dependent (the `Mode` enum in `src/app/mod.rs` — do not
assume its variants, always read the source file first). A keybinding is not done when
the key does the thing — it is done when every place a user could look to learn about it
agrees with the code.

## Process
When adding, rebinding, or removing a keybinding, atomically update:

1. `src/input.rs`: the `handle_key` route that matches the key to an action for the relevant `Mode`.
2. The relevant `src/app/*.rs` (`mod.rs`, `actions.rs`, `git.rs`, `workspace.rs`, or `navigation.rs`): the state mutation the key triggers.
3. `src/popups/help.rs` or `src/popups/detail_help.rs`: the help-overlay line for the key, in whichever overlay is active in that mode.
4. `src/components/cmd_bar/` (`mod.rs`, or the mode-specific `main.rs` / `detail.rs` / `popups.rs`): the status-bar hint entries shown at the bottom of the screen.

Miss any one of these and the UI lies to the user about what a key does — the status bar
and help overlay are the app's only real-time indicators of available actions.

Then update docs in the same commit (see the `sync-docs` skill):
- `docs/keybindings.md`: add the binding to its mode's table.
- `docs/panels.md`: if the binding is specific to a panel described there.

## Post-Run
Run the `rust-quality` skill's checks, and add or update a test in `src/app/tests.rs`
covering the new key's effect.
