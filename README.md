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

Items support `~` and `~/...` expansion, so `~/code/twig` resolves to your home directory. Statuses are recomputed only when you add, edit, or delete an item — they are not polled in the background, so changes you make outside the app won't be reflected until you re-launch.

## 🔍 Detail view

Press `Enter` on a selected item to open a full-screen Detail view. Press `Esc` or `q` to return to the list.

For a **git repository** the detail view shows:

- The resolved path (after `~` expansion).
- The current branch (or `(detached HEAD)` / `(empty repository)`).
- The HEAD commit's short hash, summary, author, and a relative time ("3 days ago").
- All configured remotes with their URLs.
- The working-tree status — `clean`, or counts of staged / modified / untracked / conflicted files.

For a **plain directory** the view confirms the resolved path and explains that no `.git` entry was found.

For a **missing** item the view confirms the resolved path and notes that the path does not exist or isn't accessible.

The detail snapshot is taken **once** when you press Enter — it is not refreshed while open. Close and re-open to re-read the repository state.

After every successful add / edit / delete, the status bar briefly shows `Saved` or `Deleted`. If the write fails, the status bar shows `Save failed: <reason>` instead — your in-memory list still reflects the change, but the file on disk does not.

---

## 🔧 Configuration

Twig loads its layout from a `config.toml` file. The resolution order is:

1. A path passed as the first CLI argument (`twig path/to/config.toml`).
2. `./config/config.toml` relative to the current working directory.
3. `./config/config.toml` relative to the executable.
4. `~/.config/twig/config.toml` (the user's XDG config directory).
5. A built-in fallback list if none of the above exist. Edits made against the fallback are saved to `~/.config/twig/config.toml`, creating it if needed.

Twig writes back to whichever file it loaded from, so edits made in the UI persist across runs.

### Example: `config.toml`

```toml
items = ["Repo A", "Repo B", "Side Project", "Test Repo"]
```

You can hand-edit this file when Twig isn't running, or let the in-app `a` / `e` / `d` shortcuts maintain it for you.

---

## 🚀 Building & Running

```sh
cargo build --release
cargo run                       # uses default config resolution
cargo run -- path/to/config.toml  # explicit config path
```
