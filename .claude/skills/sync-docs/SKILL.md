---
name: sync-docs
description: Keeps documentation in lockstep with code changes — README, roadmap, style guide, keybindings/panels docs, and installer script checksums. Trigger whenever a change touches UI panels, keybindings, workflows, conventions, or scripts/.
---

# Sync Documentation

Gitwig requires docs to be updated in the **same commit** as the change that makes them
stale (`.agent/INSTRUCTIONS.md` §6). Mirrors `.agent/skills/sync-docs/SKILL.md`.

## When to trigger
Whenever a change modifies:
- Codebase conventions or architecture
- UI panels, layouts, or tabs
- User-facing workflows or keybindings
- Installer scripts under `scripts/`

## Process
Review and update as needed:

1. `README.md` — user-facing behavior or the CLI surface changed.
2. `.agent/ROADMAP.md` — check off shipped items; add new ones if scope shifted.
3. `.agent/STYLE_GUIDE.md` — new coding standards, modular components, or TUI patterns.
4. `docs/panels.md` — added/removed/changed UI directories, panels, or their keyboard
   shortcuts.
5. `docs/keybindings.md` — any new, changed, or removed keybinding. See the
   `add-keybinding` skill for the full set of files a keybinding change touches.
6. `scripts/*.sha256` — recalculate with `shasum -a 256 <script>` for any modified script
   in `scripts/` and update the matching `.sha256` file.

A stale help overlay or `docs/keybindings.md` entry is a shipped bug even though nothing
fails to compile — don't stop at "does it build."
