# Gitwig Roadmap

This roadmap outlines the progression of Gitwig from a basic list viewer to a full-featured Git TUI.

## Phase 1: Foundation (Current)
- [x] Basic TUI setup with `crossterm` and `tui-rs`.
- [x] Configuration loading (TOML).
- [x] Migrate to `ratatui` (0.30) for modern TUI features and active maintenance.
- [x] In-app config editing — add/edit/delete items with `a`/`e`/`d`, persisted back to the loaded config file.
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

- [x] Fetch and Push operations (Fetch via `Shift+F`, Push via `Shift+P` in Branches tab).
- [x] Pull operations (shortcut `p` in Branches tab).
- [x] Manage multiple remotes.
- [x] Progress bars for network operations.

## Phase 6: Advanced Features
- [x] Mouse-click support to change the panel focus in the Detail view.
- [x] Mouse-click the selection and double-click to open the detail view in the main list.
- [x] Mice click on tab headers to change active tabs in the Detail view.
- [x] Mouse wheel vertical scroll support for all scrollable views (lists, trees, diffs).
- [x] Files tab showing tracked repository files in an interactive nested tree structure with split-panel preview (file contents or folder list).
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
- [ ] **Git Worktrees:** Tab/view to list, create, and remove Git worktrees.
- [ ] **Git Submodules:** Detect, list, initialize, and update submodules.
- [ ] **Cherry-pick & Revert:** Apply a specific commit (`cherry-pick`) or create a reverting commit (`revert`) from the log view.
- [ ] **Reflog Viewer:** A dedicated panel to inspect the git reflog, allowing users to recover lost commits/branches.
- [ ] **Stash Pop:** Perform a single-action "Pop Stash" (apply and delete).
- [ ] **Commit Signatures:** Display GPG/SSH commit verification status in the history log list.

## Phase 8: Intelligent AI Integrations (Optional/Configurable)
- [ ] **Semantic Commit Generator:** Press a key (e.g., `⌃G`) in the Commit popup to generate conventional commit messages from staged diffs using Gemini/Ollama/OpenAI APIs.
- [ ] **Diff Summarizer:** Highlight a large diff/hunk and get a brief explanation of what the change does.
- [ ] **Smart Conflict Explainer:** Provide a natural-language description of merge conflicts, highlighting the logical differences between "Ours" and "Theirs".
- [ ] **Local LLM Support:** Integration with local Ollama instances for offline code analysis.
- [ ] **Git Remote add/delete option**
- [ ] **Add fzf as dependency**
