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

Press `?` at any time in normal mode to see the full keybinding reference as a centered popup. The help overlay only handles the dismissal keys — your selection and scroll position are preserved underneath.

The bottom status bar shows a colored mode badge that mirrors what is happening:

- **Cyan `NORMAL`** — browsing the list.
- **Yellow `ADDING` / `EDITING`** — typing into the input field; the selected card's border turns yellow so you can see exactly which item will be replaced.
- **Red `CONFIRM`** — awaiting delete confirmation; the doomed card's border turns red.
- **Cyan `HELP`** — the shortcut overlay is open.

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

Press `Enter` on a selected item to open a full-screen Detail view. Press `Esc` or `q` to return to the list.

For a **git repository** the detail view shows:

- The resolved path (after `~` expansion).
- The current branch (or `(detached HEAD)` / `(empty repository)`).
- The HEAD commit's short hash, summary, author, and a relative time ("3 days ago").
- All configured remotes with their URLs.
- **Upstream** — the tracking branch (e.g. `origin/main`), or `(not configured)` if the branch has no upstream.
- **Sync** — `in sync`, `N ahead`, `N behind`, or a combination; `—` when no upstream is configured.
- The working-tree status — `clean`, or counts of staged / modified / untracked / conflicted files.

For a **plain directory** the view confirms the resolved path and explains that no `.git` entry was found.

For a **missing** item the view confirms the resolved path and notes that the path does not exist or isn't accessible.

The detail snapshot is taken **once** when you press Enter — it is not refreshed while open. Close and re-open to re-read the repository state.

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
