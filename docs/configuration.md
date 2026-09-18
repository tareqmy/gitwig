# 🔧 Configuration

Gitwig stores its config in `~/.gitwig/config.toml`. The directory is created automatically on first launch.

Settings you choose live in `config.toml`. Things that change just by using the app — when you last opened each repository, your recent commit messages, the quick-label slots and the sticky label filter — live in a separate `state.toml` beside it (see [Usage state](#usage-state-statetoml)), so opening a repository or committing never rewrites the file you edit by hand.

### First-run migration

A path passed as the first CLI argument (`gitwig path/to/config.toml`) always wins: Gitwig reads and writes that file only, and never looks at `~/.gitwig` or the migration sources below. If the file does not exist yet, Gitwig starts on the built-in defaults and only creates it the first time a setting is saved (a `themes/` directory is still written beside it).

Without a CLI path, if `~/.gitwig/config.toml` doesn't exist yet, Gitwig looks for an existing config to migrate from, in this order:

1. `./config/config.toml` relative to the current working directory.
2. `./config/config.toml` relative to the executable.
3. `~/.twig/config.toml` (legacy Twig home location).
4. `~/.config/gitwig/config.toml` (XDG location).
5. `~/.config/twig/config.toml` (legacy Twig XDG location).
6. Nothing found — a default config is written to `~/.gitwig/config.toml`.

The first match is copied to `~/.gitwig/config.toml`; from then on that file is the sole source of truth and the original is left untouched.

### Example: `config.toml`

```toml
items = ["Repo A", "Repo B", "Side Project", "Test Repo"]

# Event-loop poll interval in milliseconds (default: 100).
# Lower → more responsive input, higher → less CPU usage. Sane range: 16–500.
poll_interval_ms = 100

# Seconds a background `git fetch` may run before it is cancelled (default: 30).
# Prevents an unreachable remote from pinning a repository card forever.
# Set to 0 to disable the limit.
fetch_timeout_secs = 30

# Sorting preferences for the main page list
sort_by = "custom"
sort_reverse = false

# Compatibility mode is on by default (simple ASCII symbols).
# Set to false to use rich Unicode icons/emojis instead.
compatibility_mode = false

# Directories to watch recursively for automatic workspace syncing
watch_dirs = ["~/development"]

# Number of months inactive to be considered stale
stale_threshold_months = 1

# Hide/show stale projects on the main page list
show_stale_projects = true
```

### Config keys

| Key | Type | Default | Description |
| --- | ---- | ------- | ----------- |
| `items` | `[String]` | `[]` | Paths shown in the main list. Managed by the in-app `a` (directory scan) / `A` (bulk add) / `e` (edit) / `D` (delete) shortcuts. |
| `enable_watch_dirs` | `Boolean` | `true` | Enable or disable the Watch Directories automatic workspace sync functionality. |
| `watch_dirs` | `[String]` | `[]` | Directories watched recursively for automatic workspace synchronization. When a new Git repository is cloned or created in these directories, it is automatically added to `items` and persisted. Paths matching `scan.excludes` are ignored. |
| `poll_interval_ms` | `Integer` | `100` | How long (ms) the event loop waits between input checks. Lower feels snappier; higher saves CPU. |
| `fetch_timeout_secs` | `Integer` | `30` | Seconds a background fetch may run before Gitwig cancels it. Guards against remotes that accept a connection but never reply. `0` disables the limit. The Settings editor refuses values `1`–`4` (shorter than a TLS handshake); a hand-edited file is used as written. |
| `max_commits` | `Integer` | `500` | Maximum commits to load in workspace view. Set to `0` for unlimited. |
| `page_size` | `Integer` | `10` | Number of lines/items scrolled by Page Up / Page Down. |
| `sort_by` | `String` | `"custom"` | Main list sorting preference (`"custom"`, `"alphabetical"`, `"recent_visit"`, `"latest_changes"`). Managed by `o`. |
| `sort_reverse` | `Boolean` | `false` | Inverts the main list sorting direction (ascending vs. descending). Managed by `O`. |
| `theme` | `String` | `"default"` | Active theme configuration name. Managed in Settings `s`. |
| `compatibility_mode` | `Boolean` | `true` | Enabled by default: simple ASCII symbols are used instead of rich Unicode icons/emojis (prevents layout alignment issues in restricted terminals like RustRover's built-in terminal). Set to `false` for rich Unicode icons. |
| `scan.max_depth` | `Integer` | `6` | Maximum directory depth to search for git repositories during discovery. |
| `scan.start_dir` | `String` | your home directory, with a trailing separator (e.g. `"/Users/me/"`) | Starting directory for interactive repository discovery scanning. The default is written as the expanded absolute path. A leading `~` is expanded; environment variables (`$HOME`) are not. |
| `scan.excludes` | `[String]` | `["node_modules", "target", "venv", ".venv", "checkout"]` | Directory names excluded from discovery scanning and filesystem watching. An empty list is reset to the defaults on load, and `checkout` is added automatically whenever `target` is present. |
| `scan.git_only` | `Boolean` | `true` | **Deprecated / ignored.** Discovery scans are always git-only; the key is still accepted for backwards compatibility but nothing reads it. |
| `auto_fetch_interval_mins` | `Integer` | `10` | Time interval in minutes to automatically run background fetches for all repositories. Set to `0` to disable. Individual repositories can override this cadence (or opt out with `0`) via the Repository Settings popup (`s` on the Overview screen), stored under `repo_configs`; a whole label group can override it too via Label Settings, stored under `label_configs` (see [Per-label settings](#per-label-settings)). |
| `show_system_stats` | `Boolean` | `false` | Display CPU and Memory utilization of the Gitwig process in the bottom status bar. |
| `enable_commit_signatures` | `Boolean` | `false` | Verify GPG/SSH signatures on commits list (requires spawning git subprocesses). |
| `graph_max_commits` | `Integer` | `1000` | Maximum commits visualized in the Graph tab history. Set to `0` for unlimited. |
| `detail_cache_ttl_secs` | `Integer` | `30` | How long in seconds repository details are cached in memory before reloading. |
| `tab_ttl_secs` | `Integer` | `60` | How long in seconds lazy-loaded tab data remains cached in memory before automatic refresh. |
| `stale_threshold_months` | `Integer` | `1` | Number of months inactive (no commits) for a repository to be considered stale. |
| `show_stale_projects` | `Boolean` | `true` | Show or hide stale repositories in the list on the main page. |
| `editor` | `String` | `$EDITOR`, else `$VISUAL`, else `vim` (`notepad` on Windows) | Custom terminal editor executable to open files with from the Files tab (`e`/`o`). |
| `ssh_strict_host_checking` | `Boolean` | `false` | Enforce strict SSH host key checking (`StrictHostKeyChecking=yes`). All Gitwig SSH operations additionally run with `BatchMode=yes`: ssh can never prompt for passphrases or confirmations over the TUI — an operation needing interactive auth fails fast and the error is shown in the error popup (per-repo fetch failures show a compact `✗` marker; press `E` for details). Load your key into `ssh-agent` for passphrase-protected keys. |
| `git_app` | `String` | `"gitui"` | Preferred external Git client launched with `g`: `"git"`, `"gitui"` or `"lazygit"`. Any other value is reset to `"gitui"` on load (with a warning), and on the first run after an upgrade `gitui` falls back to `lazygit` when only the latter is installed. |
| `show_grouping` | `Boolean` | `true` | Enable or disable repository label grouping sidebar on the home page. |
| `labels` | `Map<String, [String]>` | `{}` | Repository path → list of labels. Managed by the in-app `l` shortcut. |
| `repo_configs` | `Map<String, Table>` | `{}` | Per-repository setting overrides and note (see [Per-repository settings](#per-repository-settings)). Managed by the Repository Settings popup (`s` on the Overview screen). |
| `label_configs` | `Map<String, Table>` | `{}` | Per-label settings shared by every repository carrying that label (see [Per-label settings](#per-label-settings)). Managed by the Label Settings popup (`→` on a label in the `L` picker). Auto-pruned when no repository carries the label. |
| `view_mode` | `String` | `"normal"` | Home page repository list layout mode (`"normal"`, `"compact"`, `"tile"`). Managed by `v`. |
| `tile_columns` | `Integer` | `0` | Number of columns in tile layout mode (`0` = auto-calculate based on terminal width). |
| `resync_on_tab_change` | `Boolean` | `false` | Automatically reload repository details from disk when switching tabs. |
| `prompt_cwd_repo` | `Boolean` | `true` | On startup, offer to track the repository you launched Gitwig from when it is not on the list yet. Set to `false` to never prompt. |

### Keybindings

Keyboard shortcuts live in a separate `keybindings.toml` beside `config.toml`. Entries there override the built-in defaults and are preserved across upgrades — see [Customizing Keybindings](keybindings.md#customizing-keybindings) for the format and override semantics.

### Per-repository settings

Any single repository can override six settings — `theme`, `page_size`, `max_commits`, `resync_on_tab_change`, `auto_fetch_interval_mins`, and `editor` — and carry a free-text `note`. They are edited in the **Repository Settings** popup, opened with `s` on the repository's Overview screen, and stored as a `[repo_configs."<path>"]` table keyed by the repository path exactly as it appears in `items`. A per-repository value wins over both the label tier and the global default; a row left empty inherits from the label (if any) and then the global key. `auto_fetch_interval_mins = 0` opts that one repository out of background fetching.

```toml
[repo_configs."~/development/gitwig"]
theme = "gitwig"
max_commits = 2000
auto_fetch_interval_mins = 0
note = "Main project — fetch manually before releases"
```

### Per-label settings

Settings can be attached to a **label** and are then shared by every repository carrying that label. They are the settings subset of the per-repository overrides — `theme`, `page_size`, `max_commits`, `resync_on_tab_change`, `auto_fetch_interval_mins`, and `editor` — and are edited in the **Label Settings** popup, opened with `→` on a highlighted label in the `L` label picker.

Each setting resolves through three tiers, most specific first:

1. **Per-repository override** (`repo_configs`, set in the Repository Settings popup).
2. **Label** (`label_configs`) — the first of the repository's labels, in the order they are stored, that defines the setting.
3. **Global default** (the top-level config key).

A row left empty at one tier inherits the next tier down. `auto_fetch_interval_mins = 0` at the label tier opts the whole group out of background fetching (handy for an `archive` label). When a label sets a `theme`, the home repository-list view is tinted with that theme while that label's filter ("project view") is active. Entries are stored as `[label_configs.<label>]` tables and are pruned automatically once no repository carries the label.

```toml
[label_configs.work]
theme = "nord"
auto_fetch_interval_mins = 2

[label_configs.archive]
auto_fetch_interval_mins = 0
```

### Keys Gitwig manages for you

These appear in `config.toml` but are written by the app, not meant to be edited by hand. They record deliberate choices you make in the UI, which is why they stay with the rest of your settings:

| Key | Written when |
| :--- | :--- |
| `pinned` / `starred` | You pin (`p`) or star (`*`) a repository. |
| `labels` | You edit a repository's labels (`l`). |
| `repo_configs` / `label_configs` | You change a setting (or a repository note) in Repository Settings or Label Settings. |

`compact_view` is **deprecated**. It is read once on load, converted to `view_mode = "compact"`, and then cleared — set `view_mode` instead.

### Usage state: `state.toml`

Anything that changes as a side effect of simply using Gitwig is kept out of `config.toml` and written to `state.toml` in the same directory (`~/.gitwig/state.toml`, or beside whichever config file you passed on the command line). It is entirely app-managed; deleting it only forgets the history below.

| Key | Written when |
| :--- | :--- |
| `visits` | A repository is opened — repository path → last-visit time; feeds the `recent_visit` sort and the Recent group. |
| `commit_history` | You commit — resolved repository path → its ten most recent commit messages, newest first, backing the commit-history picker in the commit dialog. |
| `active_label_filter` | You pick a label in the `L` picker (or a quick slot). The sticky "project view"; persists across restarts until deselected, and auto-clears if the label no longer exists on any tracked repository. |
| `label_slots` | You view a label through the filter for the first time. The quick-label slots behind the numbered chips and the `1`-`9` keys, in slot order (FIFO: once all nine are taken the oldest is evicted; labels no repository carries are pruned on save). |

```toml
active_label_filter = "work"
label_slots = ["work", "oss"]

[visits]
"~/development/gitwig" = 1757600000

[commit_history]
"/Users/me/development/gitwig" = ["fix(input): give the shared text inputs a caret", "release v2.5.17"]
```

**Upgrading:** versions before this one stored these keys in `config.toml` (`commit_history` under each `[repo_configs.<path>]` table). On the first launch that finds them there, Gitwig moves them into `state.toml` — without overriding anything `state.toml` already holds — and rewrites `config.toml` without them. On an upgrade both files are backed up first, as `config.toml.bak` and `state.toml.bak`.

### Themes

The active theme is selected via the `theme` key and lives in `~/.gitwig/themes/<name>.theme`. A set of popular themes is written there on first launch (`catppuccin`, `cyberpunk`, `dracula`, `forest`, `gitwig`, `gruvbox`, `monokai`, `nord`, `oceanic`, `onedark`, `rosepine`, `solarized_dark`, `tokyonight`), alongside `default`. Themes are managed in-app via Settings `s`.

A `.theme` file sets five keys:

```toml
# Gitwig — Verdigris brand theme (gitwig.theme)
accent = "#4db08a"      # selections, focus borders, active tabs
warning = "#bd6b3d"     # edit state, modified badges
danger = "#b2402e"      # delete prompts, conflicts, removed lines
success = "#3c8a6b"     # committed badges, added lines
border_type = "rounded" # "plain", "rounded", "double", "thick"
```

Colors accept either one of the 16 named terminal colors (`"black"`, `"red"`, `"green"`, `"yellow"`, `"blue"`, `"magenta"`, `"cyan"`, `"gray"`, `"darkgray"`, `"lightred"`, `"lightgreen"`, `"lightyellow"`, `"lightblue"`, `"lightmagenta"`, `"lightcyan"`, `"white"`) or a true-color hex value like `"#4db08a"` (requires a terminal with true-color support).

Gitwig writes back to whichever file it loaded from, so edits made in the UI persist across runs.
