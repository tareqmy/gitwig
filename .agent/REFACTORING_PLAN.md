# Refactoring Plan for Gitwig

Based on a deep comparative analysis with the `gitui` codebase, `gitwig` currently suffers from a
monolithic architecture with massive files:

| File | Size | Lines |
|---|---|---|
| `src/app.rs` | 366 KB | 9,291 |
| `src/ui_detail.rs` | 169 KB | ~5,000+ |
| `src/ui.rs` | 148 KB | 4,212 |
| `src/input.rs` | 121 KB | 2,800 |
| `src/repo.rs` | 110 KB | 3,106 |

`gitui` solves these issues through a highly modular, component-based workspace architecture.
The refactoring is broken into **6 phases**, ordered by impact and safety.

---

## ⚠️ Phase 0: Fix Performance — Large Repositories (CRITICAL, Do First)

> **Why first**: This is the most user-facing problem. On a repo like `36oyield` with thousands of
> files and commits, `gitwig` freezes the UI. `gitui` handles the same repo smoothly. This phase
> must be addressed **before** any architecture refactoring, because the root causes are concrete
> and fixable today without structural changes.

### Root cause diagnosis

By reading the actual code, there are **four specific performance crimes** in `repo.rs` and `app.rs`:

---

#### Problem 1: `collect_info` is a synchronous monolith that does ALL work upfront

When you open a repo (press Enter on a card), `open_detail()` spawns a thread that calls
`collect_info()`. This single function, **before returning anything**, sequentially:

1. Opens the repository
2. Walks **all commits** (or up to `max_commits`, which defaults to `0` = unlimited)
3. For **every commit** in the walk, calls `commit_changed_files()` — which runs a full
   `diff_tree_to_tree` for that commit. On a repo with 10,000 commits that is **10,000 full diffs**
4. Runs `collect_graph_lines()` which **spawns a `git log --graph --all` subprocess** and waits for
   it synchronously (this can take 5–30 seconds on a large repo)
5. Walks all commits again for committer statistics
6. Iterates **every tracked file** in the index to build the file list
7. Collects branches, tags, stashes — all before any data appears in the UI

**The fix**: Split `collect_info` into fast and slow sections. Return immediately with the cheap
data (branch, HEAD, status counts, staged/unstaged files), then fetch commits, graph, and stats
in separate background tasks that update the UI incrementally.

```rust
// Fast path — returns in <50ms even on huge repos
pub fn collect_summary_fast(path: &Path) -> Result<FastRepoInfo, git2::Error>;

// Slow path — run in background, sends partial updates via channel
pub fn collect_commits_async(path: &Path, limit: usize, tx: Sender<PartialUpdate>);
pub fn collect_graph_async(path: &Path, tx: Sender<PartialUpdate>);
pub fn collect_stats_async(path: &Path, tx: Sender<PartialUpdate>);
```

---

#### Problem 2: `commit_changed_files` runs a full diff for every commit during load

In `collect_commits()` (line 887 of `repo.rs`), for every commit in the log walk:
```rust
let files = commit_changed_files(repo, &commit);
```
This calls `repo.diff_tree_to_tree(parent_tree, commit_tree, None)` — a full tree diff — for
every single commit. For a 10,000-commit repo this is 10,000 tree comparisons happening
**before the UI becomes responsive**.

**The fix**: **Lazy-load commit file lists.** The commit list widget only needs `id`, `author`,
`date`, `summary`, and `refs` to render the table rows. The file list for a specific commit
should only be fetched when the user selects that commit and the diff panel needs to display it.

```rust
// Cheap: walk commits collecting only metadata
fn collect_commits_metadata(repo: &Repository, limit: usize) -> Vec<CommitMeta>;

// Lazy: only called when user selects a commit
pub fn get_commit_files(repo_path: &Path, commit_oid: &str) -> Vec<FileEntry>;
```

This alone will reduce initial load time by **80–95%** on large repos.

---

#### Problem 3: `collect_graph_lines` blocks by spawning and awaiting a subprocess

```rust
fn collect_graph_lines(repo_path: &Path) -> Vec<GraphLine> {
    let output = std::process::Command::new("git")
        .args(["log", "--graph", "--all", ...])
        .output(); // ← blocks until git finishes — can be 30+ seconds on large repos
```

This is called **synchronously inside `collect_info`** on the background thread. Even though
`collect_info` runs in a thread, it blocks all commit and file data from appearing until the
graph is done. If the user is on the Commits tab (not the Graph tab), this is wasted work.

**The fix**: 
1. Move graph loading to a completely separate background task that only starts when the user
   navigates to the Graph tab.
2. Use `gitui`'s approach: run `git log --graph` with `--max-count=N` and load incrementally,
   sending batches through a channel so the graph renders progressively.

---

#### Problem 4: `refresh_detail` re-runs the entire `collect_info` synchronously on the main thread

```rust
pub fn refresh_detail(&mut self) {
    self.current_detail = Some(self.inspect_repo_detail(item)); // ← BLOCKING, on main thread
```

`refresh_detail` is called after **every git action** — staging a file, committing, creating a
branch, applying a stash, etc. On a large repo, this freezes the UI for several seconds after
every action because `inspect_repo_detail` → `inspect_detail` → `collect_info` re-runs
everything synchronously from scratch.

**The fix**: After a git action, only refresh the specific data that changed:

```rust
// After staging a file: only re-collect worktree status
pub fn refresh_status_only(&mut self);

// After a commit: refresh status + re-fetch recent commits (not all of them)
pub fn refresh_after_commit(&mut self);

// Full refresh: only when explicitly requested (R key), and always async
pub fn refresh_full_async(&mut self);
```

---

### Summary of performance fixes

| Problem | Current behaviour | Fix | Expected improvement |
|---|---|---|---|
| Commit file diffs at load time | Full diff for every commit | Lazy-load per selection | 80–95% faster initial load |
| `git log --graph` blocks load | Subprocess blocks thread | Load lazily per tab, incrementally | Immediate UI response |
| `collect_info` all-at-once | All data before any UI renders | Fast path first, slow path async | < 100ms to first render |
| `refresh_detail` after actions | Full blocking re-collect | Targeted partial refresh | No freeze after git actions |

---

## Phase 1: Establish Strict Compile-Time Lint Gates

> **Why second**: Zero-risk, immediate payoff. Forces disciplined coding from the outset of
> refactoring, catching regressions early in subsequent phases.

`gitui`'s `main.rs` opens with a comprehensive `#![deny(...)]` block. `gitwig` has none.

**Actions:**
1. Add the following gates to `gitwig/src/main.rs`:
   ```rust
   #![forbid(unsafe_code)]
   #![deny(
       unused_imports,
       unused_must_use,
       dead_code,
       unused_assignments,
   )]
   #![deny(clippy::all, clippy::perf, clippy::nursery)]
   #![deny(clippy::unwrap_used, clippy::panic)]
   ```
2. Add `.clippy.toml` and `.rustfmt.toml` to the repo root to enforce consistent code style.
3. Fix all warnings and lints surfaced by this change. Many of these (`dead_code`,
   `unused_imports`) will reveal areas of `app.rs` and `input.rs` that are already stale.

---

## Phase 2: Cargo Workspace & Crate Extraction

> **Why third**: `gitwig-core` must exist as a clean API before the UI can be refactored to
> use it. This is the foundational structural change.

Currently, `gitwig` is a single crate. `gitui` separates git logic into the `asyncgit` crate,
with its own `Cargo.toml`, and uses a `[workspace]` in the root to bind everything together.

**Actions:**

1. **Convert to a Workspace**: Update root `Cargo.toml` to add a `[workspace]` members list
   pointing at the main UI crate and the new core crate:
   ```toml
   [workspace]
   members = [".", "gitwig-core"]
   ```
2. **Extract `repo.rs` into `gitwig-core`**: Move all `git2` calls and data types (`RepoSummary`,
   `ItemDetail`, `RepoInfo`, `BranchInfo`, `DiffLine`, etc.) into `gitwig-core/src/`. This crate
   has zero UI dependencies (`ratatui`, `crossterm`) — only `git2` and `serde`.
3. **Define a clean public API in `gitwig-core`**: The public surface of `gitwig-core` should be
   narrow and well-documented. The UI crate consumes data types, not raw `git2` objects.
4. **Implement the async job infrastructure** (solving Phase 0 properly): Mirror `gitui`'s
   `asyncgit` pattern of `Arc<Mutex<T>>` shared state with `AtomicBool` pending flags and
   a `crossbeam-channel` sender for notifications:
   ```rust
   pub struct AsyncLog {
       current: Arc<Mutex<Vec<CommitMeta>>>, // partial results, readable any time
       pending: Arc<AtomicBool>,
       sender: Sender<GitwigNotification>,
   }
   ```

---

## Phase 3: UI Componentization & Directory Structure

> **Why fourth**: The most impactful change for maintainability. After `gitwig-core` exists, the
> UI can be safely decomposed without touching any git logic.

`gitwig` lumps all UI code into `ui.rs` and `ui_detail.rs`. `gitui` has 11 focused components,
30 popup modules, 6 tab modules, and a `keys/` subsystem — all independently testable.

**Actions:**

### 3a. Establish the directory structure
Create the following directories inside `gitwig/src/`:
- `components/` — Reusable, stateful UI pieces (diff viewer, commit list, file tree, etc.)
- `popups/` — Modal dialogs (confirm delete, commit message, branch create, help, etc.)
- `tabs/` — Full-screen views switched by the top tab bar (Workspace, Branches, Tags, etc.)
- `keys/` — Keybinding configuration, display symbols, and key list
- `ui/` — Common layout helpers, theme/color utilities, style functions

### 3b. Define the Component trait (in `src/components/mod.rs`)
Model closely after `gitui`'s trait split:
```rust
/// Handles drawing. Separated from Component to allow read-only access during draws.
pub trait DrawableComponent {
    fn draw(&self, f: &mut Frame, rect: Rect) -> Result<()>;
}

/// Handles behavior: events, visibility, focus, and command reporting.
pub trait Component: DrawableComponent {
    fn event(&mut self, ev: &Event) -> Result<EventState>;
    fn commands(&self, out: &mut Vec<CommandInfo>, force_all: bool) -> CommandBlocking;
    fn focused(&self) -> bool { false }
    fn focus(&mut self, _focus: bool) {}
    fn is_visible(&self) -> bool { true }
    fn hide(&mut self) {}
    fn show(&mut self) -> Result<()> { Ok(()) }
}
```
Also adopt `gitui`'s `accessors!`, `any_popup_visible!`, and `draw_popups!` macros for
reducing boilerplate when composing components.

### 3c. Introduce an `InternalEvent` / `Queue` for inter-component messaging
This is the most important architectural pattern in `gitui` that is **entirely missing** from
`gitwig`. `gitui`'s `src/queue.rs` defines a shared, single-threaded `Queue<InternalEvent>`:
```rust
pub enum InternalEvent {
    ConfirmAction(Action),
    ShowErrorMsg(String),
    Update(NeedsUpdate),
    OpenCommit,
    // ...
}
```
Components push events onto the queue instead of calling methods directly on `App`. `App`
drains the queue each frame and dispatches. This **decouples** components from each other and
from `App`, eliminating the need for `App` to be a God-object.

**Gitwig currently uses a raw `mpsc::channel::<String>`** for background fetch notifications
only. The internal UI events are all handled via `App` method calls inside `input.rs`. This
needs to be replaced by a proper `Queue`.

### 3d. Break down `ui.rs` and `ui_detail.rs`
Migrate rendering logic into the new component structs. Each component owns its own state
(scroll offsets, selections, etc.) instead of storing them as flat fields on `App`.

- `DiffComponent` — owns `diff_scroll`, `diff_hunk_selection`, `diff_line_mode`, etc.
- `CommitListComponent` — owns `commit_selection`, `commit_search_query`, scroll state.
- `BranchListComponent` — owns `local_branch_selection`, `remote_branch_selection`, list states.
- `FileTreeComponent` — owns `expanded_folders`, `visible_files`, `file_list_selection`.
- `StashListComponent` — owns `stash_selection`, `stash_file_selection`.

### 3e. Extract the `Mode` enum into a proper `Popup` / `Tab` model
The current `Mode` enum in `app.rs` has **35+ variants** covering tabs, popups, text inputs,
and confirm dialogs all in one flat enum. `gitui` models each popup as a separate struct with
its own `is_visible` flag. Replace `Mode` with:
- A `Tab` enum with `~6` variants (one per major view).
- Each popup/modal as an independent struct with `is_visible: bool`, owned by `App`.

---

## Phase 4: Deconstructing `app.rs` and `input.rs`

> **Why fifth**: After components and a Queue exist, `app.rs` and `input.rs` shrink naturally.

### 4a. Decentralize Input Handling
`input.rs` is a single 2,800-line `handle_key` function with a giant `match app.mode { ... }`.
`gitui` routes input through `app.event()` which calls `event_pump()` over the active
component list. Only global shortcuts (quit, tab switch) stay at the top level.

After Phase 3's component split, `input.rs` reduces to:
1. Try the currently focused popup → if consumed, done.
2. Try the active tab → if consumed, done.
3. Handle global shortcuts (quit, tab switch).

### 4b. Shrink `App` struct to an orchestrator
After component state is moved into components (Phase 3d), `App` should only hold:
- The active `Tab`.
- The event `Queue`.
- The background channel (tx/rx) from `gitwig-core`.
- Global context: `config`, `config_path`, error message.

The current `App` struct has **~80 fields**. The target is **< 15 fields**.

---

## Phase 5: Build & DX Optimizations

> **Why last**: These are polish. They should not block feature work but should be done before
> any public release.

### 5a. Cargo Profile Optimizations
`gitui` carefully configures `Cargo.toml` profiles. Add to `gitwig/Cargo.toml`:
```toml
[profile.release]
lto = true
opt-level = 'z'      # optimize for binary size
codegen-units = 1
strip = "debuginfo"

# Speed up debug builds: compile heavy dependencies at opt-level 3
# so the TUI doesn't feel slow in dev mode
[profile.dev.package."ratatui"]
opt-level = 3
[profile.dev.package."git2"]
opt-level = 3
```

### 5b. `rust-toolchain.toml`
Pin the Rust toolchain for reproducibility:
```toml
[toolchain]
channel = "stable"
```

### 5c. Expand Makefile
The existing `Makefile` already has good basics. Add stricter targets:
```makefile
lint:
	cargo clippy -- -D warnings -D clippy::unwrap_used

fmt-check:
	cargo fmt -- --check

ci: fmt-check lint test
	@echo "All CI checks passed"
```

### 5d. CI Pipeline (`.github/workflows/`)
Add a GitHub Actions workflow that runs `make ci` on every push and PR. Reference `gitui`'s
`.github/` directory for a complete example (it includes cross-compilation, release builds,
and `cargo deny` for license/supply-chain checks).

---

## Summary: What the previous plan missed

| Gap | Why it matters |
|---|---|
| **Performance: lazy commit file diffs** | Biggest performance problem. 10k commits = 10k full diffs at load time. |
| **Performance: async `collect_info`** | UI freezes until ALL data is collected. Fast path needed. |
| **Performance: `git log --graph` subprocess** | Blocks the loading thread for 5–30s. Must be tab-lazy and incremental. |
| **Performance: `refresh_detail` on main thread** | Freezes UI after every git action. Needs targeted partial refresh. |
| **Lint gates in `main.rs`** (`#![deny(...)]`) | `gitui` opens with strict compile-time enforcement; `gitwig` has none. |
| **`InternalEvent` / `Queue` pattern** | Most important architectural pattern in `gitui`. Without it, decoupling is impossible. |
| **`DrawableComponent` vs `Component` trait split** | Required by Rust's borrow checker in a TUI (read vs mutable borrows). |
| **35+ `Mode` variants → popup structs** | Specific solution: each popup gets its own `is_visible` flag. |
| **`App` field count target** | Concrete goal: ~80 fields today → < 15 after componentization. |
| **`[profile.dev.package.*]` for git2** | `git2` is slow to link in debug; per-package opt-level fixes this. |
| **`rust-toolchain.toml`** | Ensures reproducible builds across machines and CI. |
| **`cargo deny`** | License and supply-chain security checking. |
