---
name: sync-docs
description: Keeps documentation synchronized with code changes, ensuring UI updates, workflow changes, and architecture decisions are documented.
---
# Sync Documentation

Gitwig requires documentation to be kept in lockstep with code changes. 

## When to trigger
Trigger this skill whenever you modify:
- Codebase conventions
- UI panels or layouts
- User workflows and keybindings
- Installer scripts

## Process
You **MUST** update the affected documentation in the same commit. Review the following files and update them if necessary:

1. `README.md`: Update if user-facing behaviors or the CLI surface area have changed.
2. `.agent/ROADMAP.md`: Check off items that have been shipped or add new items if scope has shifted.
3. `.agent/STYLE_GUIDE.md`: Update if you introduce new coding standards, modular components, or TUI patterns.
4. `docs/panels.md`: Update if you added, removed, or changed UI directories, panels, or their associated keyboard shortcuts.
5. `scripts/*.sha256`: If you modified any script in `scripts/`, recalculate the SHA-256 checksum and update its `.sha256` file.
