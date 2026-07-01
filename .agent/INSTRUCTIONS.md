# Agent Instructions for Gitwig

Welcome, Agent. You are tasked with helping build **Gitwig**, a high-performance Git TUI.

## Your Role
- **Researcher:** Analyze the current codebase and Git's internal state to propose the best implementation paths.
- **Implementer:** Write clean, idiomatic Rust code. Prioritize safety and performance.
- **Architect:** Help design modular UI components and efficient data structures for Git state.

## Core Principles
1. **Safety First:** Never perform destructive Git operations without user confirmation (e.g., hard reset, force push).
2. **Context Awareness:** Always check the current Git repository state before making changes or proposing UI updates.
3. **Rust Best Practices:** Use standard libraries where possible, leverage the type system for safety, and minimize `unsafe` blocks.
4. **TUI Excellence:** Aim for a responsive UI. Avoid blocking the main thread with heavy Git operations.

## Working with the Codebase
- **TUI Framework:** Use `ratatui` (currently 0.30) with the `crossterm_0_29` feature. Do not reintroduce `tui-rs` imports. Note that ratatui 0.30's `Backend` trait uses an associated `Error` type (not `io::Error`) — return `Result<(), Box<dyn Error>>` from functions that propagate it, with a `where <B as Backend>::Error: 'static` bound where needed.
- **Git Integration:** Use `git2-rs` for most operations. For complex things like interactive rebase, we may shell out to `git`.
- **Modularity:** Keep UI logic separate from Git logic. Create traits or structs to abstract Git operations.
- **Modal Input:** The app uses a `Mode` enum (`Normal`, `Adding`, `Editing`, `ConfirmDelete`, `Help`, `Detail`, `DetailHelp`, `About`, `CommitInput`, `BranchCreateInput`, `TagCreateInput`, `BranchDeleteConfirm`, `BranchPushConfirm`, `BranchMergeConfirm`, `BranchRebaseConfirm`, `BranchInteractiveRebaseConfirm`, `TagDeleteConfirm`, `TagPushConfirm`, `TagPushAllConfirm`, `StashDeleteConfirm`, `StashApplyConfirm`, `StashCreateInput`, `StashingUI`, `RemotePicker`, `CommitSearchInput`, `DiscardChangesConfirm`, `Inspect`, `Settings`, `DebugLogs`, `ImportUrlInput`, `ImportDestInput`, `ImportNameInput`, `BulkAddInput`, `SearchColumnPicker`, `RemoteAddNameInput`, `RemoteAddUrlInput`, `RemoteDeleteConfirm`, `Logs`, `LogsSearchInput`, `BranchCheckoutConfirm`, `TagCheckoutConfirm`, `RepoSearchInput`, `RepoJump`, `MergeAbortConfirm`, `MergeContinueConfirm`, `CherryPickConfirm`, `RevertConfirm`, `FileHistory`, `LabelInput`, `Overview`, `Legend`) defined in `src/app/mod.rs` to interpret keystrokes. When adding a new keybinding: add the route in `src/input.rs` (`handle_key`), add a corresponding `App` method in `src/app/mod.rs` if it mutates state, add an entry to the `HELP_LINES` constant in `src/ui.rs` or `DETAIL_HELP_LINES` in `src/ui_detail.rs` (the source of truth for overlays), and update the status-bar text in `src/ui.rs::draw_status_bar` — all in the same change.
- **Detail-view focus and Mouse support:** When in `Mode::Detail` the active tab is tracked by `App.detail_tab` (0 = Details, 1 = Files, 2 = Graph, 3 = Branches, 4 = Tags, 5 = Remotes, 6 = Stashes, 7 = Worktrees, 8 = Submodules) and the focused panel/widget is tracked by `App.detail_focus: DetailSection`. `DetailSection` variants include `Commits`, `Staged`, `Unstaged`, `Conflicts`, `CommitDetails`, `StagingDetails`, `ConflictDiff`, `LocalBranches`, `RemoteBranches`, `LocalTags`, `RemoteTags`, `Files`, `FileContent`, `Remotes`, `Stashes`, `StashedFiles`, `Worktrees`, `Submodules`. Pressing `Tab` / `Shift+Tab` (`BackTab`) cycles between the tabs, updating the active panel to a default focus for the chosen tab (`Commits` for Details, `Files` for Files, `LocalBranches` for Branches, `LocalTags` for Tags, `Remotes` for Remotes, `Stashes` for Stashes, `Worktrees` for Worktrees, `Submodules` for Submodules). Pressing `w` / `W` cycles panel focus within the Details, Files, Branches, and Tags tabs. Branch and tag panel selections also support `Left` / `Right` arrow keys. Directories inside the Files tab support expansion (`Right` / `>` / `.`) and collapse (`Left` / `<` / `,`) and are stored in a `HashSet<String>` containing paths of expanded folders. Mouse left-clicks support switching active tabs by clicking tab headers and changing focus to clicked panels using the recorded panel bounds stored in `App.detail_areas` (computed during drawing). Double-clicking an item in `Mode::Normal` opens the detail view directly. Mouse wheel scroll is supported across all scrollable lists, graphs, unified diffs, and the files tab content preview panel. Left-click and dragging splitter boundaries/lines in Workspace, Files, Branches, Stashes, Worktrees, and Submodules tabs or the Overview overlay resizes the split panels dynamically, modifying `inspect_horizontal_split_pct`, `inspect_vertical_split_pct`, `workspace_main_split_pct`, `files_horizontal_split_pct`, `branches_horizontal_split_pct`, `stashes_horizontal_split_pct`, `stashes_vertical_split_pct`, and `overview_horizontal_split_pct` fields on the `App` struct. Additionally, in `Mode::CommitInput`, left-clicking and dragging the borders or corners of the commit message popup resizes it dynamically, modifying `commit_popup_width_pct` and `commit_popup_height_pct` on the `App` struct.
- **Visual Theme:** Pull every color, border-type, and selection marker from the `Theme` constants block at the top of `src/ui.rs` (`ACCENT`, `WARNING`, `DANGER`, `CARD_BORDER`, `SELECTION_MARK`) plus the style helpers (`muted_style`, `primary_style`, `accent_style`). Do not inline raw `Color::Cyan` or `BorderType::Rounded` calls in widget code — add or reuse a constant. **Never hard-code `Color::White`, `Color::Gray`, `Color::Black`, or `Color::DarkGray` for plain text or borders** — they invert visibility between light and dark terminal backgrounds (light gray vanishes on light bg, dark gray vanishes on dark bg). Instead, leave the foreground at the terminal default (`Style::default()`) and use `Modifier::DIM` for muted text and `Modifier::BOLD` for emphasis — the terminal renders these correctly on either theme. Specific fg colors are only acceptable for: accents (`ACCENT`, `WARNING`, `DANGER`), and badge foregrounds where the badge has its own solid background to provide contrast. Selection is communicated through three layers: the left `▌` marker, the accent border color, and bold text — keep all three in sync if you change the look. Mode-dependent border colors (cyan = selected, yellow = editing, red = confirm-delete) are the user's primary feedback that a destructive or text-input action is pending.
- **Config Persistence:** `load_config` returns `(Config, PathBuf)` where the path is the destination for `save_config`. On every normal launch (no CLI override) the write target is `~/.gitwig/config.toml`; the directory is created on startup if absent. On first run, any legacy config found at `./config/config.toml`, `~/.twig/config.toml`, or `~/.config/twig/config.toml` is copied there automatically. Any mutation of `Config` from the UI must be followed by a `save_config` call so disk and memory don't diverge. Surface save errors via the transient status-bar message rather than crashing. The shared `App::persist` helper does this — prefer it over inline `save_config` calls.
- **Item Statuses:** `App.statuses: Vec<ItemStatus>` runs parallel to `App.config.items`. Any mutation that adds, edits, or removes an item **must** update `statuses` at the same index in the same method (`commit_add` pushes, `commit_edit` overwrites at `selected_index`, `confirm_delete` removes). Drift between the two vectors causes wrong indicators after edits. `repo::inspect_summary` (the card-level call) now opens libgit2 for git repos to collect staged/modified/untracked/conflicted/ahead/behind counts — so the per-mutation cost is slightly higher than a pure filesystem check, but still runs synchronously because the user just typed a path. The `unchanged()` / `is_clean()` / `is_synced()` helpers on `RepoSummary` are the canonical way to branch on repo state. The richer `repo::inspect_detail` is **only** invoked from `App::open_detail` — it reuses the same `collect_summary` internally so card counts and Detail-view counts always agree.
- **git2 API surprises in 0.21:** `Reference::shorthand` returns `Result<&str, Error>` (not `Option<&str>`); `Commit::summary` returns `Result<Option<&str>, Error>` (outer = read success, inner = UTF-8 validity); `StringArray::iter` yields `Result<Option<&str>, Error>`; `Reference::name` returns `Result<&str, Error>` (not `Option<&str>`) — this matters when reading HEAD's full ref name for `branch_upstream_name`. Collapse with `.ok()` / `.ok().flatten()` / `let Ok(Some(name)) = name else continue;` — see `src/repo.rs` for the canonical patterns.
- **File Status Labels:** The file status labels shown in the Details and Files tabs are shortened to a single character to optimize horizontal space: `"N"` for Added/New, `"D"` for Deleted, `"M"` for Modified, `"R"` for Renamed, `"T"` for Typechange, `"C"` for Conflict, and `"?"` for Untracked. The formatting width (`FILE_LABEL_WIDTH`) is set to `2`. Ensure any new status mapping conforms to this single-letter layout.

## Module Layout
The crate is organized so each file has a single clear responsibility. Keep it that way as the codebase grows.

- `src/main.rs` — entry point only. Terminal setup/teardown and call into `app::run`. Should stay small (under ~80 lines). No state, no rendering, no key handling here.
- `src/app/` — module directory for application state (`App`) and main loop:
  - `mod.rs`: holds struct definition, `App::new`, `App::run`, and main orchestration / queue draining.
  - `actions.rs`: home card list state mutations.
  - `git.rs`: git network and repository mutations (branches, tags, remotes, fetch, push, pull, rebase, merge).
  - `workspace.rs`: workspace staging/unstaging, diff refreshers, commits, cherry-pick/revert, and conflicts.
  - `navigation.rs`: list scrolling, cycle selections, tilde expansion, theme updates, and settings.
  - `tests.rs`: contains the full unit test suite, conditionally compiled (`#[cfg(test)]`).
- `src/input.rs` — event routing dispatcher. Maps keyboard event routing directly to active tabs and popups.
- `src/mouse.rs` — mouse event handler. Manages left-clicks, scrolling, and split-panel resizing.
- `src/config.rs` — configuration loading, saving, and Theme list generation.
- `src/ui/` — main rendering logic, layout, styling/theme utilities (e.g., `draw.rs`, `layout.rs`, `style.rs`, `ui_detail.rs`).
- `src/tabs/` — layout drawing logic per tab (e.g., `workspace.rs`, `files.rs`, `branches.rs`, `tags.rs`, `stashes.rs`, `home.rs`).
- `src/popups/` — centered modal popup renderers (e.g., `commit.rs`, `confirm.rs`, `help.rs`, `settings.rs`).
- `src/components/` — modular reusable widgets (e.g., `file_tree.rs`, `commit_list.rs`, `branch_list.rs`, `tag_list.rs`, `stash_list.rs`, `diff.rs`, `status_list.rs`).
- `gitwig-core/` — workspace member crate. Handles all repository inspection (zero UI dependencies).

### When to split a file further
- If any file grows past **~300 lines**, look for an extraction. Common split lines: a new mode that has its own state/rendering, a widget that has its own builder logic, a new config concern (e.g. theme, keybinding overrides).
- A new view (e.g. a History panel) should get its own file under `src/ui/` (promote `ui.rs` to `ui/mod.rs` when this happens).
- A new domain concept (e.g. `Repository`, `Branch`) should get its own module, not be jammed into `app.rs`.
- Prefer adding a new module over expanding an existing one when the new code doesn't share state with what's already there.

## Keeping Docs In Sync
- **Whenever you change code, update the relevant documentation in the same task.** The docs are the contract for future agents and contributors — stale docs are worse than no docs.
- `GEMINI.md` — update when the tech stack, architectural patterns, or development workflow change.
- `.agent/ROADMAP.md` — check off items as they ship; add new ones as scope shifts; never leave a completed feature unchecked.
- `.agent/INSTRUCTIONS.md` — update when codebase conventions, framework guidance, or working rules change.
- `.agent/STYLE_GUIDE.md` — update when coding standards, naming, error-handling patterns, or TUI patterns change.
- `README.md` — update when user-facing behavior, install steps, or CLI surface change.
- If a change touches multiple concerns, update each affected doc. If you are unsure where something belongs, add it where a future agent is most likely to look.

## Communication
- Be concise.
- Provide technical rationale for your decisions.
- If you find a bug in the existing TUI logic, fix it as part of your task.
