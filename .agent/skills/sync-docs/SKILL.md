---
name: sync-docs
description: Keeps documentation in lockstep with code changes — README, roadmap, style guide, keybindings/panels docs, and installer script checksums. Trigger whenever a change touches UI panels, keybindings, workflows, conventions, or scripts/.
---
# Sync Documentation

_Mirrored for Claude Code at `.claude/skills/sync-docs/SKILL.md` — keep both in sync._

Gitwig requires documentation to be updated in the **same commit** as the change that makes it stale (`.agent/INSTRUCTIONS.md` §5).

## When to trigger
Trigger this skill whenever you modify:
- Codebase conventions or architecture
- UI panels, layouts, or tabs
- User workflows and keybindings
- Installer scripts under `scripts/`

## Process
You **MUST** update the affected documentation in the same commit. Review the following files and update them if necessary:

1. `README.md`: Update if user-facing behaviors or the CLI surface area have changed.
2. `.agent/ROADMAP.md`: Check off items that have been shipped or add new items if scope has shifted.
3. `.agent/STYLE_GUIDE.md`: Update if you introduce new coding standards, modular components, or TUI patterns.
4. `docs/panels.md`: Update if you added, removed, or changed UI directories, panels, or their associated keyboard shortcuts.
5. `docs/keybindings.md`: Update for any new, changed, or removed keybinding — see the `add-keybinding` skill for the full set of files a keybinding change touches.
6. `scripts/*.sha256`: If you modified any script in `scripts/`, recalculate the SHA-256 checksum with `shasum -a 256 <script>` and update its `.sha256` file.

A stale help overlay or `docs/keybindings.md` entry is a shipped bug even though nothing
fails to compile — don't stop at "does it build."
