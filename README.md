# 🌿 Twig — A Minimal Terminal Git UI

**Twig** is a lightweight terminal-based Git UI, designed as a fast and minimal alternative to GUI tools like SourceTree. Built with Rust and `ratatui`, Twig presents your Git-related items in a clean, bordered layout directly in the terminal.

---

## 📸 Preview

> _(Coming soon — or add an asciinema/screenshot here!)_

---

## ✨ Features

- Fullscreen terminal UI using `ratatui` and `crossterm`
- Config-driven layout using `config.toml`
- Add / edit / delete items directly from the UI — changes persist back to the config file
- Per-item status indicators — see at a glance whether each item is a git repo, a plain directory, or missing
- Press `Enter` on any item to open a full-screen Detail view with branch, HEAD commit, remotes, and working-tree status (for git repos) or a clear "plain directory" / "missing" report otherwise
- Bordered items displayed inside a main border
- Mode-aware status bar that always shows the relevant shortcuts

---

## ⌨️ Keybindings

| Key                  | Mode            | Action                            |
| -------------------- | --------------- | --------------------------------- |
| `↑` / `k`            | Normal          | Move selection up                 |
| `↓` / `j`            | Normal          | Move selection down               |
| `a`                  | Normal          | Add a new item                    |
| `e`                  | Normal          | Edit the selected item            |
| `d`                  | Normal          | Delete the selected item (asks)   |
| `r`                  | Normal          | Refresh status of selected item   |
| `g`                  | Normal          | Launch gitui for selected repository |
| `Enter`              | Normal          | Open Detail view for selected item|
| `?`                  | Normal / Help   | Toggle the shortcut overlay       |
| `q`                  | Normal          | Quit                              |
| `Enter`              | Adding / Editing| Save the typed text and persist   |
| `Esc`                | Adding / Editing| Cancel without saving             |
| `Backspace`          | Adding / Editing| Erase one character               |
| `y`                  | Confirm Delete  | Confirm deletion                  |
| `n` / `Esc`          | Confirm Delete  | Cancel deletion                   |
| `?` / `Esc` / `q`    | Help            | Close the help overlay            |
| `Esc` / `q`          | Detail          | Return to the list                |
| `Tab`                | Detail          | Cycle panel focus (`Commits` → `Staged` → `Unstaged` → `StagingDetails`) |
| `↑` / `k`            | Detail          | Move selection or scroll diff up  |
| `↓` / `j`            | Detail          | Move selection or scroll diff down |
| `PgUp` / `PgDn`      | Detail          | Jump 10 rows or page scroll diff  |
| `Enter`              | Detail          | Stage/Unstage file (in staging lists) |
| `c`                  | Detail          | Open commit message input prompt  |
| `o`                  | Detail          | Toggle repository overview popup  |
| `1`                  | Detail          | Go to Details tab                 |
| `2`                  | Detail          | Go to Graph View tab              |
| `3`                  | Detail          | Go to Branches tab                |
| `Enter`              | Detail          | Stage/Unstage file, or Checkout selected branch |
| `Shift+F`            | Detail          | Fetch selected local branch from remote |
| `?`                  | Detail          | Toggle detail help overlay        |
| `Esc` / `q` / `o`    | DetailOverview  | Close repository overview popup   |
| `Esc` / `q` / `?`    | DetailHelp      | Close detail help overlay         |
| `⌃C`                 | CommitInput (Edit) | Finish editing commit message (switches to confirm state) |
| `↵` (Enter)          | CommitInput (Edit) | Insert a newline                  |
| `Backspace`          | CommitInput (Edit) | Erase one character from commit message |
| `Esc`                | CommitInput     | Cancel commit and return to Detail view |
| `↵` (Enter)          | CommitInput (Confirm) | Submit / execute Git commit      |
| `e` / `E`            | CommitInput (Confirm) | Edit / resume typing commit message |
| `Left-Click` (Mouse) | Normal          | Select the clicked item           |
| `Double-Click` (Mouse)| Normal         | Open Detail view for clicked item |
| `Left-Click` (Mouse) | Detail          | Shift focus to the clicked panel  |

Press `?` at any time in normal mode to see the full keybinding reference as a centered popup. The help overlay only handles the dismissal keys — your selection and scroll position are preserved underneath.

The selected card's border color mirrors the current operation mode:
- **Default (Muted)** — browsing the list (unselected).
- **Cyan** — browsing the list (selected).
- **Yellow** — typing into the input field during add/edit; the selected card's border turns yellow so you can see exactly which item will be replaced.
- **Red** — awaiting delete confirmation; the doomed card's border turns red.

The selected item is marked with a left-edge `▌` accent, a colored border, and bold text. In `ADDING` and `EDITING` modes the real terminal cursor sits at the end of your input so you can see exactly where the next character will land.

## 📂 Item status indicators

Each card shows a colored symbol on the right reflecting the item's filesystem state:

- `● git` — the item is a directory containing a `.git` entry (a git repository, worktree, or submodule).
- `○ dir` — the item is a directory, but not a git repository.
- `✕ missing` — the item is not a directory on this machine (doesn't exist, is a file, or isn't accessible).

For git repositories the indicator also shows compact counts for any non-zero values:

| Suffix | Meaning | Colour |
| ------ | ------- | ------ |
| `N+`   | N files staged for commit | Cyan |
| `N!`   | N files modified but not staged | Yellow |
| `N?`   | N untracked files | Muted |
| `N↑`   | N commits ahead of upstream (needs push) | Bold |
| `N↓`   | N commits behind upstream (needs pull/fetch) | Yellow |

When all counts are zero the indicator shows `● clean`. When the branch has no configured upstream, only the worktree counts appear (no `↑`/`↓`). Press `?` at any time to see the legend inside the app.

Items support `~` and `~/...` expansion, so `~/code/twig` resolves to your home directory. Statuses are recomputed only when you add, edit, or delete an item — they are not polled in the background. Press `r` to manually refresh the selected item's status if you've changed the filesystem outside the app (e.g. `git init` in a directory that was previously `○ dir`); the status bar briefly flashes `Refreshed` so you know the check ran.

## 🔍 Detail view

Press `Enter` on a selected item to open a full-screen Detail view. The detail view supports three tabs for git repositories: **Details**, **Graph**, and **Branches**. Press `1` to switch to the Details tab, `2` to switch to the Graph tab, and `3` to switch to the Branches tab. Press `Esc` or `q` to return to the list.

### Panel Layout & Navigation (Details Tab)

For a **git repository**, the **Details** tab is split into multiple rounded panels with a 40:60 width ratio on the bottom:
- **Commits (top 50%):** Lists the recent commits in the repository. If there are uncommitted changes, a special row named `Uncommitted changes` will be pinned to the very top.
- **Staging Area / Changed Files (bottom-left 40%):** Lists files that are modified, staged, or untracked. When `Uncommitted changes` is selected at the top, this panel is split vertically into `Staged` and `Unstaged` sections. When a real commit is selected, it is split horizontally, showing the list of files modified in that commit in the top half, and full commit details (hash, author, date, refs, and message) in the bottom half.
- **Staging Details (bottom-right 60%):** Displays the unified diff of the selected file.

### Graph Tab

The **Graph** tab is currently reserved for the future git branch visualization graph.

### Branches Tab

The **Branches** tab is split vertically into left and right panels:
- **Local Branches (left):** Lists local branches in the repository, sorting the active branch to the top marked with ``.
- **Remote Branches (right):** Lists remote tracking branches.

You can navigate and interact with these panels in the following ways:
- **Cycle focus:** Cycle focus between active panels using `Tab` (`Commits` → `Staged` → `Unstaged` → `StagingDetails` in the Details tab, and toggles between `Local Branches` and `Remote Branches` in the Branches tab).
- **Mouse Click to Focus/Select:** In normal mode, left-click on any repository item in the main list to select it, and double-click to open its detail view directly. In detail mode, click anywhere inside any panel's boundaries (including branch list panels) to focus it immediately.
- **Navigate Lists:** Use `↑`/`k` and `↓`/`j` to select a commit, file, or branch in the active list.
- **Scroll Diff:** When the `Staging Details` panel is focused, you can scroll the unified diff text vertically using `↑`/`k` and `↓`/`j` (line-by-line) or `PgUp`/`PgDn` (page-by-page).
- **Stage/Unstage Files:** Select the `Uncommitted changes` row at the top, select a file in either the `Staged` or `Unstaged` list, and press `Enter` to stage or unstage that file instantly.
- **Checkout and Fetch Branches:** While on the **Branches** tab:
  - Select any **local branch** (other than the currently active one) and press `Enter` to check it out (safe checkout strategy applies).
  - Select any **local branch** that has an upstream configuration and press `Shift+F` to fetch updates for its remote in the background.
  - Select any **remote branch** and press `Enter` to check it out (this will switch to the branch, creating a local tracking branch if it doesn't already exist).
- **Commit Staged Changes:** Press `c` from the detail view to open a centered Commit popup window (only active if there are staged changes). 
  - **Compose Mode:** Type your commit message. Press `Ctrl+C` to lock in the text and switch to confirmation state, press `Enter` to insert a newline, or `Esc` to cancel.
  - **Confirm Mode:** Press `Enter` to execute the commit, `e` to return to composing/editing the message, or `Esc`/`q` to close the popup.
- **Repository Overview:** Press `o` to open a repository overview popup showing details such as resolved paths, branch upstream tracking, configured remotes, and general status counts.

For a **plain directory** the view confirms the resolved path and explains that no `.git` entry was found.

For a **missing** item the view confirms the resolved path and notes that the path does not exist or isn't accessible.

The detail snapshot is taken **once** when you press Enter — it is not refreshed while open (except for staging/unstaging actions which refresh dynamically). Close and re-open to re-read the repository state.

After every successful add / edit / delete, the status bar briefly shows `Saved` or `Deleted`. If the write fails, the status bar shows `Save failed: <reason>` instead — your in-memory list still reflects the change, but the file on disk does not.

---

## 🔧 Configuration

Twig stores its config in `~/.twig/config.toml`. The directory is created automatically on first launch.

### First-run migration

If `~/.twig/config.toml` doesn't exist yet, Twig looks for an existing config to migrate from:

1. A path passed as the first CLI argument (`twig path/to/config.toml`).
2. `./config/config.toml` relative to the current working directory.
3. `./config/config.toml` relative to the executable.
4. `~/.config/twig/config.toml` (previous XDG location).
5. Nothing found — a default config is written to `~/.twig/config.toml`.

After the first run the migrated (or generated) file becomes the sole source of truth; the original is left untouched.

### Example: `config.toml`

```toml
items = ["Repo A", "Repo B", "Side Project", "Test Repo"]

# Event-loop poll interval in milliseconds (default: 100).
# Lower → more responsive input, higher → less CPU usage. Sane range: 16–500.
poll_interval_ms = 100
```

### Config keys

| Key | Type | Default | Description |
| --- | ---- | ------- | ----------- |
| `items` | `[String]` | `[]` | Paths shown in the main list. Managed by the in-app `a`/`e`/`d` shortcuts. |
| `poll_interval_ms` | `Integer` | `100` | How long (ms) the event loop waits between input checks. Lower feels snappier; higher saves CPU. |

Twig writes back to whichever file it loaded from, so edits made in the UI persist across runs.

---

## 🚀 Building & Running

```sh
cargo build --release
cargo run                       # uses default config resolution
cargo run -- path/to/config.toml  # explicit config path
```
