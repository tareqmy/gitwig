# Twig Roadmap

This roadmap outlines the progression of Twig from a basic list viewer to a full-featured Git TUI.

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

## Phase 2: Working Tree & Status
- [ ] Display list of changed files (staged and unstaged).
- [ ] Support staging/unstaging individual files (shortcut `s` and `u`).
- [ ] Basic commit functionality (shortcut `c`).
- [ ] Side-by-side or unified diff view for the selected file.

## Phase 3: History & Log
- [ ] Display Git commit log with author, date, and message.
- [ ] Navigate through history.
- [ ] View diff for a specific commit.
- [ ] Branch visualization (graph view).

## Phase 4: Branch Management
- [ ] List local and remote branches.
- [ ] Checkout branches (shortcut `o`).
- [ ] Create and delete branches.
- [ ] Merge and Rebase (basic support).

## Phase 5: Remotes & Sync
- [ ] Fetch, Pull, and Push operations.
- [ ] Manage multiple remotes.
- [ ] Progress bars for network operations.

## Phase 6: Advanced Features
- [ ] Stashing (list, apply, drop, pop).
- [ ] Interactive Rebase.
- [ ] Conflict resolution UI.
- [ ] Custom themes and keybindings.
- [ ] Search and filter in history and file lists.
