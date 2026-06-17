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
| `?`                  | Normal / Help   | Toggle the shortcut overlay       |
| `q`                  | Normal          | Quit                              |
| `Enter`              | Adding / Editing| Save the typed text and persist   |
| `Esc`                | Adding / Editing| Cancel without saving             |
| `Backspace`          | Adding / Editing| Erase one character               |
| `y`                  | Confirm Delete  | Confirm deletion                  |
| `n` / `Esc`          | Confirm Delete  | Cancel deletion                   |
| `?` / `Esc` / `q`    | Help            | Close the help overlay            |

Press `?` at any time in normal mode to see the full keybinding reference as a centered popup. The help overlay only handles the dismissal keys — your selection and scroll position are preserved underneath.

When the prompt at the bottom switches color you've entered an input mode:

- **Blue** — Normal browsing.
- **Yellow** — Typing (adding or editing).
- **Red** — Awaiting delete confirmation.
- **Cyan** — Help overlay open.

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
