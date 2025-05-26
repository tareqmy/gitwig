# 🌿 Twig — A Minimal Terminal Git UI

**Twig** is a lightweight terminal-based Git UI, designed as a fast and minimal alternative to GUI tools like SourceTree. Built with Rust and `tui-rs`, Twig presents your Git-related items in a clean, bordered layout directly in the terminal.

---

## 📸 Preview

> _(Coming soon — or add an asciinema/screenshot here!)_

---

## ✨ Features

- Fullscreen terminal UI using `tui` and `crossterm`
- Config-driven layout using `config.toml`
- Bordered items displayed inside a main border
- Quits gracefully with the `q` key

---

## 🔧 Configuration

Twig loads its layout from a `config.toml` file located in the root of the repository.

### Example: `config.toml`

```toml
items = ["Repo A", "Repo B", "Side Project", "Test Repo"]
