---
name: add-keybinding
description: Checklist for adding, rebinding, or removing a Gitwig keyboard shortcut — the exact set of files that must be updated atomically so key routing, the status bar, help overlays, and docs never drift apart. Trigger whenever a task adds, changes, or removes a keybinding.
---
# Add/Change a Keybinding

_Mirrored for Claude Code at `.claude/skills/add-keybinding/SKILL.md` — keep both in sync._

Gitwig's key handling is mode-dependent (the `Mode` enum in `src/app/mod.rs` — do not
assume its variants, always read the source file first). A keybinding is not done when
the key does the thing — it is done when every place a user could look to learn about it
agrees with the code.

## Process (`.agent/INSTRUCTIONS.md` §2 "Modal Input")
When adding, rebinding, or removing a keybinding, atomically update:

1. `src/keybindings.rs`: bindings are data-driven. Add the `Action` variant, its entries in `Action::from_index` and `Action::to_index`, a field on the relevant `*Keybindings` struct, a default in `default_config()`, and the match arms in `get_action_keys`, `get`, and `update_action_keys` (plus `find_conflict`'s list, and `is_global_action` if it is a global). Then add the new index to the category list in `src/popups/settings.rs` (`*_SETTING_INDICES` / `ALL_KEYBINDINGS_SETTING_INDICES`) so it shows on the Settings page.
2. `src/input.rs`: the `handle_key` route that matches the key to an action for the relevant `Mode`.
3. The relevant `src/app/*.rs` (`mod.rs`, `actions.rs`, `git.rs`, `workspace.rs`, or `navigation.rs`): the state mutation the key triggers.
4. `src/popups/help.rs` or `src/popups/detail_help.rs`: the help-overlay line for the key, in whichever overlay is active in that mode.
5. `src/components/cmd_bar/` (`mod.rs`, or the mode-specific `main.rs` / `detail.rs` / `popups.rs`): the status-bar hint entries shown at the bottom of the screen.

Miss any one of these and the UI lies to the user about what a key does — the status bar
and help overlay are the app's only real-time indicators of available actions.

Then update docs in the same commit (see the `sync-docs` skill):
- `docs/keybindings.md`: add the binding to its mode's table.
- `docs/panels.md`: if the binding is specific to a panel described there.

## Post-Run
Run the `rust-quality` skill's checks, and add or update a test in `src/app/tests.rs`
covering the new key's effect (`.agent/INSTRUCTIONS.md` §4: every method gets a test).
