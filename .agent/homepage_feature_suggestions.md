# Homepage Feature Suggestions

This document lists proposed enhancements for Gitwig's homepage — the repository list
screen where the user sees all tracked repositories at a glance.

---

## 🔥 High-Impact Visual Enhancements

### 1. Repository Health / State Indicators
Show the current *repo state* as a badge on each card:
`⚠ MERGE_HEAD`, `🚧 REBASING`, `⚡ CHERRY-PICK`, `✓ CLEAN`, etc.
Currently the compound indicator shows file counts but not the special HEAD state.

### 2. Last Activity Timestamp
Display the relative time of the most recent commit directly on each card
(e.g., `2h ago`, `3d ago`, `1w ago`). Pairs naturally with the existing
`Latest Changes` sort mode and helps users spot stale repos instantly.

### 3. Compact / Dense View Toggle
A keypress (e.g., `v`) to switch between the current 4-row cards and a
1-row compact list. Useful when tracking many repos. The compact row format:
`icon  name  branch  dirty-indicator  last-commit-ago`.

### 4. Color-coded Divergence Badge
A visually distinct `↑3 ↓1` badge, color-coded red/yellow/green, showing
how far ahead/behind origin the current branch is — without opening the
Detail view.

---

## 🗂️ Organisation & Navigation

### 5. Label / Group Collapsing
Allow `←`/`→` or `Space` on a label group header to collapse or expand all
repos belonging to that label. Reduces visual clutter when many repos are
tracked and organised into groups.

### 6. Fuzzy Jump-to-Repo Overlay
A quick `/`-triggered full-screen fuzzy-search overlay that shows all repo
names and lets the user jump instantly. Unlike the existing `RepoSearchInput`
(which filters the list inline), this overlay shows ranked matches in a
floating popup.

### 7. Recently Opened MRU Stack
Persist a Most-Recently-Used history of opened repos across sessions
(stored in config). Surface the last N repos at the top or in a dedicated
`Recent` label group for fast re-entry.

### 8. Favorite / Star Repos
A semantic ★ bookmark, separate from the positional pin.
Starred repos float to a dedicated section and are displayed with a visual
star glyph. Persisted in config.

---

## 📊 At-a-Glance Stats

### 9. Global Summary Header Bar
A 1–2 row header above the list showing aggregate counts:

```
N repos  •  M dirty  •  P ahead of origin  •  Q stale (no remote)
```

Gives an instant "morning dashboard" view of the entire workspace.

### 10. Uncommitted Work Warning Badge
Visually distinguish cards that have *both* staged and unstaged changes
simultaneously (partial staging), which often indicates forgotten or
in-progress work.

### 11. Background Auto-Refresh (Live Dashboard)
Extend the existing `poll_interval` setting to auto-refresh *all* repo
statuses in the background, keeping divergence counts and dirty indicators
live without any manual `r` press.

---

## ⚡ Power-User Workflows

### 12. Multi-select with `Space`
Select multiple repos and batch-operate: e.g., fetch all selected, bulk-
delete entries, or open all in terminals. Essential for users managing many
repos or multiple worktrees.

### 13. Bulk Fetch All (`F` from Home)
A single keypress that fetches *all* tracked repos concurrently and updates
their ahead/behind counts. Shows a progress overlay or per-card spinners
while running.

### 14. Open in Terminal (`t`)
Press `t` to `cd` into the selected repo path and spawn a new shell/terminal
at that location. Common in GUI Git clients (Tower, SourceTree, etc.).

### 15. Copy Path to Clipboard (`y` / yank)
Press `y` to copy the selected repo's absolute filesystem path to the system
clipboard — handy for quickly pasting into other tools or scripts.

### 16. Per-Repo Note / Rule on Card
Show a one-line user-defined note directly on the card (e.g., "client's prod
repo — be careful!"). Related to the "Per Repository rule" roadmap item;
the note would be set via the repo settings popup and shown in smaller text
below the branch line.

---

## 🎨 Polish & UX

### 17. Animated Status Spinner During Background Fetch
When a background fetch is in progress for a specific repo, replace its
static indicators with a Braille spinner (`⠋⠙⠹⠸⠼⠴⠦⠧`) so the user
knows that card is being updated.

### 18. Empty State with Onboarding Prompt
When the config has zero repos, render a welcoming centered panel instead
of a blank list:

```
No repositories tracked yet.

  a  — add a repository path
  i  — import (clone) from a URL
  b  — bulk add from a directory
```

---

## Implementation Priority Notes

| # | Feature                        | Effort | Impact |
|---|--------------------------------|--------|--------|
| 5 | Group collapsing               | Low    | High   |
| 9 | Global summary header          | Low    | High   |
| 3 | Compact view toggle            | Low    | Medium |
| 18| Empty state onboarding         | Low    | Medium |
| 11| Background auto-refresh        | Medium | High   |
| 2 | Last activity timestamp        | Medium | High   |
| 1 | Repo health/state indicator    | Medium | High   |
| 4 | Divergence badge               | Medium | Medium |
| 12| Multi-select + batch ops       | High   | High   |
| 13| Bulk fetch all                 | Medium | High   |
| 6 | Fuzzy jump overlay             | Medium | Medium |
| 7 | MRU recent stack               | Medium | Medium |
| 14| Open in terminal               | Low    | Medium |
| 15| Copy path to clipboard         | Low    | Medium |
| 8 | Favorite / star                | Medium | Low    |
| 16| Per-repo note on card          | Low    | Medium |
| 17| Fetch spinner animation        | Low    | Low    |
| 10| Partial-staging warning badge  | Medium | Low    |
