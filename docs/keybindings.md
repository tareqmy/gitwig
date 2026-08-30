# ⌨️ Keybindings

| Key                  | Mode            | Action                            |
| -------------------- | --------------- | --------------------------------- |
| `↑` / `k`            | Normal          | Move selection up                 |
| `↓` / `j`            | Normal          | Move selection down               |
| `a`                  | Normal          | Add a new repository (via directory scanner) |
| `A`                  | Normal          | Bulk add folders in a directory   |
| `i`                  | Normal          | Import remote repository          |
| `e`                  | Normal          | Edit the selected item            |
| `D`                  | Normal          | Delete the selected item (asks)   |
| `l`                  | Normal          | Edit labels of the selected item  |
| `R`                  | Normal          | Refresh status of selected item   |
| `F`                  | Normal          | Bulk fetch all tracked repositories concurrently |
| `E`                  | Normal          | Show why the selected repository's last fetch failed |
| `f`                  | Normal          | Enter repository search mode      |
| `Tab` / `Shift+Tab`  | Normal          | Cycle summary-bar filter (repos → dirty → ahead → stale) forward / backward |
| `L`                  | Normal          | Open label picker: filter the list to one label (sticky until deselected; persists across restarts) |
| `Esc`                | LabelPicker     | Close label picker without changing the filter |
| `↑` / `↓`            | LabelPicker     | Navigate label list |
| `Enter`              | LabelPicker     | Apply selected label (re-selecting the active label, or "All repositories", clears it) |
| `Home` / `End` / `PgUp` / `PgDn` | Fuzzy pickers | Jump to top / bottom or page through the match list. Works in all fuzzy picker overlays: label picker, Jump-to-Repo, add/bulk-add repo scanners, and branch/file/commit/tag searches |
| `ctrl+f`             | Normal          | Open global code search popup overlay |
| `p`                  | Normal          | Toggle pin status of selected item |
| `*`                  | Normal          | Toggle Favorite / Star status of selected item |
| `Space`              | Normal          | Toggle selection of item for batch operations (fetch, delete, terminal) |
| `y`                  | Normal          | Yank absolute path of selected item to clipboard |
| `/`                  | Normal          | Open fuzzy Jump-to-Repo picker overlay |
| `o`                  | Normal          | Cycle list sorting mode (Custom → Alphabetical → Recent → Changes) |
| `O`                  | Normal          | Toggle list sorting direction (ascending vs. reversed) |
| `g`                  | Normal          | Launch the preferred Git client for selected repository (configurable in settings, default is gitui) |
| `t`                  | Normal          | Spawn a new shell (Terminal) in the selected repository's directory |
| `s`                  | Normal          | Open options/settings page        |
| `d`                  | Normal          | Open debug logs panel             |
| `V`                  | Normal          | Show about popup / creator profile |
| `v`                  | Normal          | Cycle between standard cards, compact 1-row view, and tile grid view |
| `h`                  | Normal          | Show signs & symbols legend popup |
| `Shift+U`            | Normal          | Open App Usage Stats Dashboard    |
| `u`                  | Normal          | Check for application updates manually |
| `.`                  | Normal / Detail | Toggle status bar between collapsed and expanded view |
| `Enter`              | Normal / Commits list | Open Detail view for selected item / Inspect selected commit |
| `?`                  | Normal / Help   | Toggle the shortcut overlay       |
| `ctrl+q`             | Global / Anywhere | Quit application from anywhere    |
| `Esc`                | Normal          | Clear active search filter or cancel all repository selections |
| `Enter`              | Editing / Input Modals | Save / submit typed text and persist |
| `Esc`                | Editing / Input Modals | Cancel modal input without saving (letters like `q`/`Q` are typed as regular input) |
| `ctrl+v` / Terminal Paste | Editing / Input Modals | Paste text from system clipboard (strips newlines for single-line inputs) |
| `Esc`                | RepoSearchInput | Clear repository search and return to Normal mode |
| `Esc`                | GlobalSearch    | Return to Normal mode             |
| `Tab`                | GlobalSearch    | Toggle focus between search query input and results |
| `↑` / `↓`            | GlobalSearch    | Navigate search results           |
| `Enter` (Query focus)| GlobalSearch    | Trigger/execute search scan       |
| `Enter` (Results focus) | GlobalSearch | Select matched result, jump to repository, and open Detail view |
| `Char`               | GlobalSearch    | Type query (when input is focused)|
| `Backspace`          | GlobalSearch    | Erase query character (when input is focused) |
| `Enter`              | Settings (Edit) | Save settings edit                |
| `Esc`                | Settings (Edit) | Cancel settings edit              |
| `Esc` / `q`          | Settings        | Exit Settings and return to Home  |
| `↑` / `↓` / `k` / `j` | Settings        | Navigate setting fields / categories |
| `Enter` / `Space`    | Settings        | Toggle / Edit selected setting    |
| `Tab` / `Shift+Tab`  | Settings        | Switch focus between category list (sidebar) and settings list |
| `←` / `h` / `H`      | Settings        | Focus category list (sidebar)     |
| `→` / `l` / `L` / `w` / `W` | Settings  | Focus settings list               |
| `1` - `5`            | Settings        | Jump directly to category: General (1), Sorting (2), Scan (3), Theme (4), Keybindings (5) |
| `Esc` / `q`          | DebugLogs       | Exit Debug Logs (or clear filter if active) and return to Home |
| `c` / `C` / `x`      | DebugLogs       | Clear all debug logs and reset scroll |
| `/`                  | DebugLogs       | Enter fuzzy search query mode    |
| `Enter`              | DebugLogs (Search) | Finish typing query and lock focus to log list scrolling |
| `Esc`                | DebugLogs (Search) | Clear query character/text or exit search mode |
| `Backspace`          | DebugLogs (Search) | Erase query character            |
| `Char`               | DebugLogs (Search) | Type query character             |
| `↑` / `↓` / `k` / `j` | DebugLogs       | Scroll debug logs list            |
| `PgUp` / `PgDn`      | DebugLogs       | Scroll debug logs list by page size |
| `Home` / `End`       | DebugLogs       | Jump to top / bottom of debug logs |
| `Backspace`          | Editing         | Erase one character               |
| `Tab`                | Tag Creation    | Switch focus between Tag Name and Tag Message input fields |
| `y` / `Y`            | Confirm Dialog  | Confirm action (delete item/branch/tag, push branch/tag/all tags, tag overwrite, abort/continue merge) |
| `f` / `F`            | Confirm Dialog  | Force push tag (`--force`) in Tag Push confirmation dialog |
| `t` / `T`            | Confirm Dialog  | Push branch with tags (`--tags`) in Branch Push confirmation dialog |
| `n` / `N` / `Esc`    | Confirm Dialog  | Cancel action                     |
| `?` / `Esc` / `q`    | Help            | Close the help overlay            |
| `Esc` / `q`          | Detail          | Return to the list                |
| `Tab` / `Shift+Tab`  | Detail          | Cycle active detail view tabs within the active group (Primary vs. Advanced) |
| `w` / `W`            | Detail          | Cycle panel focus forward (w) / backward (W) |
| `+` / `-`            | Detail          | Grow (`+`) or shrink (`-`) the focused panel by moving its layout split (same splits the mouse can drag) |
| `1` - `7`            | Detail          | Jump directly to Primary tab: Workspace (1), Files (2), Graph (3), Branches (4), Tags (5), Remotes (6), Stashes (7) |
| `1` - `5`            | Detail          | Jump directly to Advanced tab: Worktrees (1), Submodules (2), Reflog (3), Issues (4), PRs (5) (when Advanced group is active) |
| `Z`                  | Detail          | Toggle between Primary and Advanced tab groups |
| `O`                  | Detail          | Toggle full-screen repository Overview overlay |
| `↑` / `k`            | Detail          | Move selection or scroll list/diff/tree up |
| `↓` / `j`            | Detail          | Move selection or scroll list/diff/tree down |
| `PgUp` / `PgDn`      | Detail / Normal / Settings | Scroll list/diff/tree/settings by configured `page_size` |
| `Home` / `End`       | Detail / Normal / Settings | Jump to top / bottom of list/diff/tree/settings |
| `Enter`              | Detail          | Stage/Unstage file (Workspace tab), checkout branch (Branches tab), checkout tag (Tags tab), open worktree in new context (Worktrees tab), checkout commit (Reflog tab), or Inspect commit |
| `F` (or `f`/`F` in Remotes) | Detail   | Fetch remote repository (Branches / Tags / Remotes tabs) |
| `p`                  | Detail          | Pull selected local branch from remote (Branches tab), Push selected tag (Tags tab; asks confirmation), or Prune stale worktree metadata (Worktrees tab) |
| `Shift+P`            | Detail          | Push selected local branch to remote (Branches tab) or Push all tags (Tags tab; asks confirmation) |
| `←` / `→`            | Detail          | Focus Local/Remote branch (Branches tab) or Local/Remote tag (Tags tab) |
| `←` / `→` or `<` / `>` (also `,` to collapse) | Detail | Collapse/Expand directory (Files tab) |
| `/`                  | Detail          | Fuzzy find files (Files tab) / commits (Workspace commits list / Logs view) / branches (Branches tab) / tags (Tags tab) |
| `e` / `o`            | Detail          | Open selected file in configured terminal editor (Files tab)      |
| `Shift+H`            | Detail          | View selected file's commit/revision history (Files tab) |
| `c`                  | Detail          | Open commit prompt (Workspace tab or Inspect view), or Create branch from HEAD (Branches tab) |
| `a`                  | Detail          | Toggle "Assigned to me" / "All Open" issues (Issues tab), Stage All (Workspace tab Unstaged focus) / Unstage All (Workspace tab Staged focus), Apply stash (Stashes tab), or Add worktree (Worktrees tab) |
| `n`                  | Detail          | Add line comment to selected PR (PRs tab)                        |
| `x`                  | Detail          | Discard selected file changes (Workspace tab or Inspect view; asks confirmation) |
| `X`                  | Detail          | Discard all changes in repository (Workspace tab or Inspect view; asks confirmation) |
| `i`                  | Detail          | Interactive rebase from selected commit (Workspace tab commits list) |
| `G`                  | Detail          | Load more commits (Workspace commits list / Logs view)            |
| `l`                  | Detail          | Open Logs view (Workspace tab commits list focus) or Toggle lock status (Worktrees tab; asks reason/unlocks) |
| `D`                  | Detail          | Delete selected branch (Branches tab; asks confirmation), tag (Tags tab; asks confirmation), stash (Stashes tab; asks confirmation), or remove worktree (Worktrees tab; asks confirmation) |
| `s` / `S`            | Detail          | Stage selected hunk/line (Workspace tab StagingDetails focus), open Stashing UI overlay (Workspace tab files list focus), Prompt to save stash (Stashing UI / Stashes tab), or Open Repository Settings popup (Overview overlay) |
| `u` / `U`            | Detail          | Unstage selected hunk/line (Workspace tab StagingDetails focus), or Toggle "Stash untracked files" option (Stashing UI) |
| `i`                  | Detail          | Toggle "Keep index" option (Stashing UI)                         |
| `Ctrl+U`             | Input (Stash)   | Toggle "Stash untracked files" option (Stash Create popup)       |
| `Ctrl+I`             | Input (Stash)   | Toggle "Keep index" option (Stash Create popup)                 |
| `m`                  | Detail          | Merge selected branch into current branch (Branches tab; asks confirmation) |
| `M`                  | Detail          | Checkout selected local branch and merge the current branch into it (Branches tab, Local panel; asks confirmation) |
| `r`                  | Detail          | Rebase current branch onto selected branch (Branches tab; asks confirmation) |
| `o` / `O`            | Detail          | Checkout selected commit (Workspace commits list), or Accept OURS version of conflict (Workspace tab Conflicts / ConflictDiff) |
| `t`                  | Detail          | Accept THEIRS version of conflict (Workspace tab Conflicts / ConflictDiff) |
| `r`                  | Detail          | Mark conflict as resolved (Workspace tab Conflicts / ConflictDiff) |
| `A`                  | Detail          | Abort the merge (Workspace tab Conflicts / ConflictDiff; asks confirmation) |
| `C`                  | Detail          | Continue the merge (Workspace tab Conflicts / ConflictDiff; asks confirmation) |
| `M`                  | Detail          | Open external mergetool (Workspace tab Conflicts / ConflictDiff) |
| `f`                  | Detail          | Open search column picker and go to logs (Workspace tab) |
| `R`                  | Detail          | Resync the active tab state       |
| `?`                  | Detail          | Toggle detail help overlay        |
| `Esc` / `q` / `?`    | DetailHelp      | Close detail help overlay         |
| `⌃S` (Ctrl+S)        | CommitInput (Edit) | Submit / execute Git commit directly from edit mode |
| `⌃A` (Ctrl+A)        | CommitInput (Edit) | Toggle amend last commit option |
| `⌃D` (Ctrl+D)        | CommitInput (Edit) | Maximize / restore the popup window size |
| `⌃C` (Ctrl+C)        | CommitInput (Edit) | Finish editing commit message (switches to confirm state) |
| `⌃H` (Ctrl+H)        | CommitInput (Edit) | Open commit history picker |
| `⌃U` (Ctrl+U)        | CommitInput (Edit) | Clear entire commit message buffer |
| `↵` (Enter)          | CommitInput (Edit) | Insert a newline                  |
| `Backspace`          | CommitInput (Edit) | Erase one character from commit message |
| `Esc`                | CommitInput     | Cancel commit and return to Detail view |
| `↵` (Enter)          | CommitInput (Confirm) | Submit / execute Git commit      |
| `e` / `E`            | CommitInput (Confirm) | Edit / resume typing commit message |
| `a` / `A` / `Space`  | CommitInput (Confirm) | Toggle amend last commit option   |
| `x` / `X` / `u` / `U` | CommitInput (Confirm) | Clear entire commit message buffer |
| `Left-Click` (Mouse) | Normal          | Select the clicked item           |
| `Double-Click` (Mouse)| Normal         | Open Detail view for clicked item |
| `Left-Click` (Mouse) | Detail          | Shift focus to the clicked panel or change active tab |
| `Left-Click + Drag` (Mouse) | Detail    | Drag panel boundary splitters to dynamically resize layout splits |
| `Scroll Wheel` (Mouse)| Normal / Detail | Scroll selected list, graph view, branches, tree items, or unified diffs |

Press `?` at any time in normal mode to see the full keybinding reference as a centered popup. The help overlay only handles the dismissal keys — your selection and scroll position are preserved underneath.

The selected card's border color mirrors the current operation mode:
- **Default (Muted)** — browsing the list (unselected).
- **Cyan** — browsing the list (selected).
- **Yellow** — typing into the input field during add/edit; the selected card's border turns yellow so you can see exactly which item will be replaced.
- **Red** — awaiting delete confirmation; the doomed card's border turns red.

The selected item is marked with a left-edge `▌` accent, a colored border, and bold text. In `ADDING` and `EDITING` modes the real terminal cursor sits at the end of your input so you can see exactly where the next character will land.

### Repository Settings Popup controls:
When viewing the Repository Settings popup (entered via `s` on the repository Overview page):
- `↑` / `↓` / `j` / `k` — Navigate settings rows.
- `Enter` / `Space` — Toggle/Edit the selected setting:
  * For editable fields (Page Size, Max Commits, Editor Command, User Note, Git LFS Track): starts/enters editing mode (type and press `Enter` to confirm, `Esc` to cancel).
  * For action fields (Git LFS Pull, LFS Storage Size): triggers the pull background job or refreshes the storage size.
- `←` / `→` / `h` / `l` — Cycle options (Theme, Resync on Tab Change).
- `Esc` / `q` — Close repository settings popup and return to repository Detail view.


## Customizing Keybindings

All bindings are stored in `keybindings.toml` next to your `config.toml` (e.g. `~/.gitwig/keybindings.toml` or `~/.config/gitwig/keybindings.toml`). Edit them in-app via Settings (`s`) → Keybindings, or in the file directly — each action holds a `keys` list, for example:

```toml
[navigation.grow_panel]
keys = ["+"]
description = "Grow the focused panel"
```

Entries in this file **override the built-in defaults**. Actions missing from the file are backfilled with their defaults on startup, but existing entries are never rewritten — so if a default changes in a newer Gitwig version (for example, panel grow was briefly bound to both `+` and `=` in development builds before settling on `+` alone), a previously written entry keeps the old keys until you edit it or delete the entry to re-adopt the default.
