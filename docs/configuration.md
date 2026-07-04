# 🔧 Configuration

Gitwig stores its config in `~/.gitwig/config.toml`. The directory is created automatically on first launch.

### First-run migration

If `~/.gitwig/config.toml` doesn't exist yet, Gitwig looks for an existing config to migrate from:

1. A path passed as the first CLI argument (`gitwig path/to/config.toml`).
2. `./config/config.toml` relative to the current working directory.
3. `./config/config.toml` relative to the executable.
4. `~/.config/gitwig/config.toml` (new XDG location), `~/.config/twig/config.toml` (legacy Twig XDG location), or `~/.twig/config.toml` (legacy Twig home location).
5. Nothing found — a default config is written to `~/.gitwig/config.toml`.

After the first run the migrated (or generated) file becomes the sole source of truth; the original is left untouched.

### Example: `config.toml`

```toml
items = ["Repo A", "Repo B", "Side Project", "Test Repo"]

# Event-loop poll interval in milliseconds (default: 100).
# Lower → more responsive input, higher → less CPU usage. Sane range: 16–500.
poll_interval_ms = 100

# Sorting preferences for the main page list
sort_by = "custom"
sort_reverse = false

# Enable compatibility mode to use simple ASCII symbols
compatibility_mode = false

# Directories to watch recursively for automatic workspace syncing
watch_dirs = ["~/development"]
```

### Config keys

| Key | Type | Default | Description |
| --- | ---- | ------- | ----------- |
| `items` | `[String]` | `[]` | Paths shown in the main list. Managed by the in-app `a` (directory scan) / `e` / `d` shortcuts. |
| `watch_dirs` | `[String]` | `[]` | Directories watched recursively for automatic workspace synchronization. When a new Git repository is cloned or created in these directories, it is automatically added to `items` and persisted. |
| `poll_interval_ms` | `Integer` | `100` | How long (ms) the event loop waits between input checks. Lower feels snappier; higher saves CPU. |
| `max_commits` | `Integer` | `0` | Maximum commits to load in workspace view. Set to `0` for unlimited. |
| `page_size` | `Integer` | `10` | Number of lines/items scrolled by Page Up / Page Down. |
| `sort_by` | `String` | `"custom"` | Main list sorting preference (`"custom"`, `"alphabetical"`, `"recent_visit"`, `"latest_changes"`). Managed by `o`. |
| `sort_reverse` | `Boolean` | `false` | Inverts the main list sorting direction (ascending vs. descending). Managed by `O`. |
| `theme` | `String` | `"default"` | Active theme configuration name. Managed in Settings `s`. |
| `compatibility_mode` | `Boolean` | `false` | Enable to use simple ASCII symbols instead of rich Unicode icons/emojis (prevents layout alignment issues in restricted terminals like RustRover's built-in terminal). |
| `scan.max_depth` | `Integer` | `6` | Maximum directory depth to search for git repositories during discovery. |
| `scan.start_dir` | `String` | `"$HOME"` | Starting directory for interactive repository discovery scanning. |
| `scan.excludes` | `[String]` | `[]` | Directory names excluded from discovery scanning. |
| `scan.git_only` | `Boolean` | `true` | Only scan folders that contain a .git directory. |

Gitwig writes back to whichever file it loaded from, so edits made in the UI persist across runs.
