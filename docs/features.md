# ✨ Features

- **Fullscreen Terminal UI**: Designed using `ratatui` and `crossterm` with modern visual styling.
- **Config-Driven Layout**: Driven by `config.toml` for persistence and custom configurations.
- **In-App Card Management**: Add, edit, delete, and label items directly from the UI with automatic disk persistence.
- **Global Summary Header Bar**: High-level dashboard stats showing aggregate counts of repos, dirty repos, ahead counts, and stale repos.
- **Background Auto-Refresh**: Non-blocking background status checking every 10 seconds to keep your repo cards live and up-to-date.
- **Uncommitted Work Warning Badge**: Highlights repositories with a `⚠ PARTIAL` badge when staged and unstaged changes coexist simultaneously.
- **Fuzzy Jump Picker**: Instantly jump to any repository by name using the `/` overlay.
- **Favorite / Star Repositories**: Bookmark important repositories with `*` separate from pinned items.
- **Compact View Toggle**: Press `v` to toggle between standard 4-row cards and a dense 1-row view.
- **Label / Group Collapsing**: Organize repositories on the home page with collapsible label headers.
- **Full-Screen Detail View**: Press `Enter` to open a multi-tab inspection interface (Workspace, Files tree with preview, Graph log, Branches, Tags, Remotes, Stashes, Worktrees, Submodules, Reflog). Tab headers dynamically fallback to their first character under restricted widths to prevent overflow.
- **Mode-Aware Status Bar**: Shows contextual shortcuts dynamically with support for collapsed/expanded view (`.`). When expanded, it dynamically wraps the shortcut items and calculates the exact number of rows needed based on terminal width.
- **Global Code Search**: Press `Ctrl-F` to search for string patterns across all tracked repositories using a fast, multithreaded offline file scanner. Hitting `Enter` on a result automatically jumps to and opens that repository.
- **Automatic Workspace Sync**: Monitor directories (e.g. `watch_dirs = ["~/development"]`) to automatically sync newly created or cloned repositories to your workspace in real-time.
