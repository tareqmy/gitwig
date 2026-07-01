# Gitwig Roadmap

This roadmap outlines the progression of Gitwig from a basic list viewer to a full-featured Git TUI.

## Phase 1: Foundation (Current)
- [x] Basic TUI setup with `crossterm` and `tui-rs`.
- [x] Configuration loading (TOML).
- [x] Migrate to `ratatui` (0.30) for modern TUI features and active maintenance.
- [x] In-app config editing — add/edit/delete items with `a`/`e`/`D`, persisted back to the loaded config file.
- [x] In-app help overlay — `?` toggles a centered popup listing every shortcut.
- [x] Per-item filesystem status indicator (missing / directory / git repo) using a lightweight `.git`-existence check; supports `~` expansion.
- [x] Integrate `git2` for the per-item detail view: branch, HEAD commit, remotes, working-tree status (staged / modified / untracked / conflicted). Snapshot is captured once on Enter.
- [x] Detail view modal (Enter to open, Esc/q to close) with mode-aware status bar (`DETAIL` badge).
- [x] Manual status refresh — `r` in Normal mode re-runs `repo::inspect_summary` for the selected item and flashes "Refreshed".
- [x] Per-card compound indicator: `● clean` for a fully in-sync repo, or `● N+ N! N? N↑ N↓` showing staged / modified / untracked / conflicted / ahead / behind counts (only non-zero values shown). `status.rs` was folded into `repo.rs`; a shared `collect_summary` helper ensures the card and Detail view always agree.
- [x] Detail view gains **Upstream** and **Sync** rows (powered by `branch_upstream_name` + `graph_ahead_behind`); shows `(not configured)` when the branch has no tracking branch.
- [x] Detail view restructured into named rounded panels: `Commits` (top 50%) and `Staging Area` / `Staging Details` side-by-side (bottom 50%). Breadcrumb header shows item name (left) and active branch name with `` glyph (right).
- [x] Detail view panel focus cycling via `Tab`: `Commits → Staged → Unstaged → Staging Details → Commits`. Focused panel highlighted with accent border; `Tab  cycle focus` shown in status bar.
- [x] Support `Esc` key in addition to `q` to quit the application from the home page.

## Phase 2: Working Tree & Status
- [x] Display a list of changed files (staged and unstaged).
- [x] Support staging/unstaging individual files (shortcut `Enter` in staging lists).
- [x] Stage all, unstage all, and discard all changes (shortcuts `a`/`X` in Workspace tab).
- [x] Basic commit functionality (shortcut `c` in Workspace tab or Inspect view).
- [x] Side-by-side or unified diff view for the selected file.

## Phase 3: History & Log
- [x] Display Git commit log with author, date, and message.
- [x] Navigate through history (`↑`/`k`, `↓`/`j`, `PgUp`/`PgDn` in Commits panel).
- [x] View diff for a specific commit (select a commit to see its changed files, and select a file to view its diff on the right).
- [x] Branch visualization (graph view).

## Phase 4: Branch Management
- [x] List local and remote branches.
- [x] Checkout branches (shortcut `Enter` in Branches tab).
- [x] Create and delete branches.
- [x] Dedicated Tags tab to list, check out local/remote tags, delete local tags, and push tag(s) with confirmation dialogs.
- [x] Merge and Rebase (basic support).

- [x] Fetch and Push operations (Fetch via `f`/`F`, Push via `Shift+P` in Branches tab).
- [x] Pull operations (shortcut `p` in Branches tab).
- [x] Manage multiple remotes.
- [x] Progress bars for network operations.

## Phase 6: Advanced Features
- [x] Mouse-click support to change the panel focus in the Detail view.
- [x] Mouse-click the selection and double-click to open the detail view in the main list.
- [x] Mice click on tab headers to change active tabs in the Detail view.
- [x] Mouse wheel vertical scroll support for all scrollable views (lists, trees, diffs).
- [x] Files tab showing tracked repository files in an interactive nested tree structure with split-panel preview (file contents or folder list).
- [x] Per-file revision history view (shortcut `Shift+H` in Files tab).
- [x] Stash list (dedicated Stashes detail tab).
- [x] Stashing actions: apply and delete stash (Details / Stashes).
- [x] Main page sorting (Alphabetical, Recent Visit, Latest Changes, Custom) and direction toggle (o / O).
- [x] Commit amending support (a / A / Space in confirmation mode).
- [x] Interactive fzf directory picker to add items (a).
- [x] Search and filter in history and file lists (commit search).
- [x] Interactive Rebase.
- [x] Custom themes and keybindings.
- [x] Allow an option to revert a dirty file.
- [x] In-app settings page (accessed with `s` shortcut) to edit and persist settings in `config.toml` (poll interval, sort mode, reverse sort, active theme, fzf max depth, and fzf start dir).
- [x] Hunk-by-hunk diffs allow or decline.
- [x] Line-by-line diffs allow or decline.
- [x] Dynamic focus-aware status bar shortcuts showing only actions available to the focused panel.
- [x] Auto-shift focus between Staged and Unstaged lists when all changes are staged/unstaged (shortcut `a`).
- [x] Mouse selection support for branches, tags, remotes, and stashes lists in Detail panels.
- [x] Modal error popups for failed Git network and stash operations.
- [x] Stash uncommitted files with a stash comment/name input popup.
- [x] Conflict resolution UI.

## Phase 7: Advanced Git Workflows & Power-User Tools
- [x] Add a debug panel
- [x] Git clone
- [x] Bulk add repo option
- [x] Optional usage of fzf
- [x] Git Remote add/delete option
- [x] Ensure single character input to avoid multiple same commands being applied.
- [x] About
- [x] Solve windows’ button press bug
- [x] Commit Signatures: Display GPG/SSH commit verification status in the history log list.
- [x] Stash Pop: Perform a single-action "Pop Stash" (apply and delete).
- [x] Cherry-pick & Revert: Apply a specific commit (`cherry-pick`) or create a reverting commit (`revert`) from the log view.
- [x] Commit window resize
- [x] Show memory and cpu usage from within the app
- [x] For large repo load time is high
- [x] Refactor the codebase to make it manageable

## Phase 8:
- [x] Per-file history view
- [x] Apply label for repositories: so that from the home page it can be viewed as a group
- [x] Distribute application via a curl-to-sh script (`install.sh`).
- [x] Add fzf as dependency
- [x] Git logs pagination
- [x] Per repository settings, theme
- [x] Check version updates and notify the user and the option to update
- [x] Keybindings
- [x] Git Worktrees: Tab/view to list, create, and remove Git worktrees.
- [x] Git Submodules: Detect, list, initialize, and update submodules.
- [x] If network action is done from schedule without user explicitly triggering it, need to show that network action is happenning somewhere, definitely not a popup for implicit network call.
- [x] Self Update: Shortcut to trigger update check, check on start, show badge beside version when update available
- [x] Editor Support: Option to open file from file tab with custom terminal editor from settings
- [ ] **Per Repository rule**
- [ ] **Reflog Viewer:** A dedicated panel to inspect the git reflog, allowing users to recover lost commits/branches.
- [ ] Remove temporary the installation scripts from the base directory.

## Phase 9: Homepage Enhancements
> Full details and implementation priority table: see `.agent/homepage_feature_suggestions.md`

### Visual Enhancements
- [x] Repo Health / State Indicators: Show repo HEAD state as a badge on each card (`⚠ MERGE_HEAD`, `🚧 REBASING`, `⚡ CHERRY-PICK`, `✓ CLEAN`).
- [x] Last Activity Timestamp: Display relative time of the last commit on each card (e.g., `2h ago`, `3d ago`).
- [x] Compact / Dense View Toggle: Press `v` to switch between 4-row cards and a 1-row compact list — useful for large repo collections.
- [x] Color-coded Divergence Badge: Color the `↑N ↓M` ahead/behind indicator red/yellow/green based on how out-of-sync the branch is.
- [x] A popup to explain different signs and symbols used throughout the application.
- [x] In workspace show the whole file in the diff, since it does not have anything to compare with.

### Organisation & Navigation
- [x] Label / Group Collapsing: `←`/`→` or `Space` on a label group header to collapse/expand repos in that group.
- [x] Fuzzy Jump-to-Repo Overlay: `/`-triggered floating popup with ranked fuzzy matches across all repo names for instant navigation.
- [x] Recently Opened MRU Stack: Persist most-recently-used repo history across sessions; surface in a dedicated `Recent` group.
- [x] Favorite / Star Repos: Semantic ★ bookmark separate from positional pin; starred repos float to a dedicated section.

### At-a-Glance Stats
- [x] **Global Summary Header Bar:** 1–2 row header showing aggregate counts (`N repos • M dirty • P ahead • Q stale`).
- [x] **Uncommitted Work Warning Badge:** Visually highlight cards with *both* staged and unstaged changes simultaneously.
- [x] **Background Auto-Refresh (Live Dashboard):** Extend `poll_interval` to auto-refresh all repo statuses in the background.

### Power-User Workflows
- [ ] **Multi-select with `Space`:** Select multiple repos and batch-operate (fetch, delete entries, open in terminal).
- [ ] **Bulk Fetch All (`F`):** Fetch all tracked repos concurrently from the home screen; show per-card progress.
- [x] **Open in Terminal (`t`):** Spawn a new shell `cd`-ed into the selected repo path.
- [x] **Copy Path to Clipboard (`y`):** Yank the selected repo's absolute path to the system clipboard.
- [x] **Per-Repo Note on Card:** Display a one-line user-defined note below the branch line (ties into "Per Repository rule").

### Polish & UX
- [ ] **Animated Fetch Spinner:** Replace static status indicators with a Braille spinner while a background fetch is active for that repo.
- [ ] **Empty State Onboarding Prompt:** Welcoming centred panel when zero repos are tracked, listing key shortcuts to get started.

## Phase 10: Intelligent AI Integrations (Optional/Configurable)
- [ ] **Semantic Commit Generator:** Press a key (e.g., `⌃G`) in the Commit popup to generate conventional commit messages from staged diffs using Gemini/Ollama/OpenAI APIs.
- [ ] **Diff Summarizer:** Highlight a large diff/hunk and get a brief explanation of what the change does.
- [ ] **Smart Conflict Explainer:** Provides a natural-language description of merge conflicts, highlighting the logical differences between "Ours" and "Theirs".
- [ ] **Local LLM Support:** Integration with local Ollama instances for offline code analysis.
