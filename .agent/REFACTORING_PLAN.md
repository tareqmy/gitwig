# Refactoring Plan for Gitwig

Based on a deep comparative analysis with the `gitui` codebase — including the exact file
structure of every folder — this plan maps every piece of `gitwig`'s monolith to its target
location in the modular layout.

---

## The Core Principle: One Type = One File

`gitui`'s defining discipline is simple:

> **Every struct lives in its own `.rs` file. Every folder has a `mod.rs` that only declares
> submodules and re-exports. No type is defined in `mod.rs`.**

This is enforced across all folders: `popups/`, `tabs/`, `components/`, `keys/`, `ui/`.

For example:
- `popups/confirm.rs` → contains exactly `struct ConfirmPopup`
- `popups/commit.rs` → contains exactly `struct CommitPopup`
- `popups/mod.rs` → only `mod commit; pub use commit::CommitPopup;`

This makes every type instantly findable by filename, and keeps each file under ~300 lines.

---

## Phase 0: Fix Performance — Large Repositories (DONE ✅)

All 4 performance problems identified (commit diffs per load, blocking graph subprocess,
monolithic `collect_info`, synchronous `refresh_detail`) have been resolved. See previous
audit for details.

---

## Phase 1: Establish Strict Compile-Time Lint Gates

> Zero-risk, immediate payoff. Catch regressions before they compound.

Add to `gitwig/src/main.rs`:
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

Add `.clippy.toml` and `.rustfmt.toml` to enforce consistent code style.

---

## Phase 2: Cargo Workspace & Crate Extraction

Convert to a `[workspace]` and extract `repo.rs` into `gitwig-core`:

```toml
[workspace]
members = [".", "gitwig-core"]
```

`gitwig-core` has zero UI dependencies. It exposes the public API consumed by the UI crate.

---

## Phase 3: UI Directory Structure — Exact File Layout

This is the main phase. The target structure mirrors gitui's exactly.
Below is a precise mapping: **what currently exists → where it goes**.

---

### `src/ui/` — Theme, Styles, Layout Utilities

Modelled on `gitui/src/ui/` which contains 7 files, each with one responsibility.

| Target File | What Goes There | Comes From |
|---|---|---|
| `ui/mod.rs` | Re-exports only: `pub use style::Theme; pub use theme::THEME;` | — |
| `ui/style.rs` | `struct Theme`, all `fn accent()`, `fn danger()`, etc., `ThemeState`, `THEME` static | `ui.rs` lines ~28–95 |
| `ui/scrollbar.rs` | Scrollbar rendering helpers | `ui.rs` or `ui_detail.rs` |
| `ui/layout.rs` | `centered_rect()`, layout constraint helpers, padding utilities | `ui.rs` and `ui_detail.rs` |
| `ui/syntax.rs` | Syntax highlighting utilities | Extracted from `ui_detail.rs` |

**Key rule**: `ui/style.rs` is the single source of truth for colors. No other file calls
`Color::Cyan` directly — they call `theme.accent()` or `THEME.read()`.

---

### `src/keys/` — Keybinding Configuration

Modelled on `gitui/src/keys/` which has 4 files, each with one role.

| Target File | What Goes There |
|---|---|
| `keys/mod.rs` | `pub use key_config::KeyConfig; pub use key_list::KeyList; pub use symbols::KeySymbols;` |
| `keys/key_config.rs` | `struct KeyConfig`, `fn init()`, `fn format_key()` |
| `keys/key_list.rs` | `struct KeyList` — the full list of all named key bindings (e.g. `move_up`, `quit`, `commit`) |
| `keys/symbols.rs` | `struct KeySymbols` — display strings for each key (e.g. `"↑"`, `"↓"`, `"⏎"`) |

Currently `gitwig` has no `keys/` folder. All key constants are hardcoded inline in `input.rs`.
Extracting to a `KeyList` struct enables users to remap keys via a config file (like gitui).

---

### `src/components/` — Reusable, Stateful UI Widgets

Modelled on `gitui/src/components/` which has 10 files + 2 subdirectories.
Each file contains **one struct** implementing both `DrawableComponent` and `Component`.

**First, define the two traits in `components/mod.rs`:**

```rust
// components/mod.rs
pub trait DrawableComponent {
    fn draw(&self, f: &mut Frame, rect: Rect) -> Result<()>;
}
pub trait Component: DrawableComponent {
    fn event(&mut self, ev: &Event) -> Result<EventState>;
    fn commands(&self, out: &mut Vec<CommandInfo>, force_all: bool) -> CommandBlocking;
    fn focused(&self) -> bool { false }
    fn focus(&mut self, _focus: bool) {}
    fn is_visible(&self) -> bool { true }
    fn hide(&mut self) {}
    fn show(&mut self) -> Result<()> { Ok(()) }
}
pub enum EventState { Consumed, NotConsumed }
pub enum CommandBlocking { Blocking, PassingOn }
```

**Then, one struct per file:**

| Target File | Struct Name | State it Owns | Comes From |
|---|---|---|---|
| `components/mod.rs` | Traits + `event_pump()` + macros | — | New |
| `components/commit_list.rs` | `CommitListComponent` | `commit_selection`, `commit_search_query`, `commits_table_state` | `ui_detail.rs` |
| `components/diff.rs` | `DiffComponent` | `file_diff`, `diff_scroll`, `diff_hunk_selection`, `diff_line_mode`, `diff_line_selection` | `ui_detail.rs` |
| `components/file_tree.rs` | `FileTreeComponent` | `expanded_folders`, `visible_files`, `file_list_selection`, `file_content_scroll` | `app.rs`, `ui_detail.rs` |
| `components/status_list.rs` | `StatusListComponent` | `staged_list_state`, `unstaged_list_state`, `staging_file_selection` | `ui_detail.rs` |
| `components/branch_list.rs` | `BranchListComponent` | `local_branch_selection`, `remote_branch_selection`, `local_branch_list_state`, `remote_branch_list_state` | `ui_detail.rs` |
| `components/tag_list.rs` | `TagListComponent` | `local_tag_selection`, `remote_tag_selection`, `local_tag_list_state` | `ui_detail.rs` |
| `components/stash_list.rs` | `StashListComponent` | `stash_selection`, `stash_file_selection`, `stash_list_state` | `ui_detail.rs` |
| `components/text_input.rs` | `TextInputComponent` | `input_buffer`, cursor position | `input.rs`, `app.rs` |
| `components/graph_view.rs` | `GraphViewComponent` | `graph_scroll` | `ui_detail.rs` |
| `components/cmdbar.rs` | `CmdBar` | Current visible command hints | New (from `ui.rs`) |
| `components/utils/` | Helper sub-components | `scroll_vertical.rs`, `scroll_horizontal.rs`, `statustree.rs` | Various |

---

### `src/popups/` — Modal Dialogs

Modelled on `gitui/src/popups/` which has **30 files**, one popup per file.
`gitwig`'s `Mode` enum has 35+ variants — every `*Confirm`, `*Input`, and overlay variant
becomes **its own file with its own struct**.

**`popups/mod.rs`** only declares modules and re-exports:
```rust
mod confirm_delete;     pub use confirm_delete::ConfirmDeletePopup;
mod commit_input;       pub use commit_input::CommitInputPopup;
mod help;               pub use help::HelpPopup;
// ... one line per popup
```

**Full mapping — current `Mode` variant → target file → struct name:**

| Mode Variant (current) | Target File | Struct |
|---|---|---|
| `ConfirmDelete` | `popups/confirm_delete.rs` | `ConfirmDeletePopup` |
| `Help` | `popups/help.rs` | `HelpPopup` |
| `DetailHelp` | `popups/detail_help.rs` | `DetailHelpPopup` |
| `CommitInput` | `popups/commit.rs` | `CommitPopup` |
| `BranchCreateInput` | `popups/create_branch.rs` | `CreateBranchPopup` |
| `TagCreateInput` | `popups/create_tag.rs` | `CreateTagPopup` |
| `BranchDeleteConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `BranchPushConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `TagDeleteConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `TagPushConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `TagPushAllConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `StashDeleteConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `StashApplyConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `BranchMergeConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `BranchRebaseConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `BranchInteractiveRebaseConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `DiscardChangesConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `RemoteDeleteConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `BranchCheckoutConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `TagCheckoutConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `MergeAbortConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `MergeContinueConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `CherryPickConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `RevertConfirm` | `popups/confirm.rs` (generic) | `ConfirmPopup { action }` |
| `StashCreateInput` | `popups/stash_msg.rs` | `StashMsgPopup` |
| `RemotePicker` | `popups/remote_picker.rs` | `RemotePickerPopup` |
| `CommitSearchInput` | `popups/log_search.rs` | `LogSearchPopup` |
| `ImportUrlInput`, `ImportDestInput`, `ImportNameInput` | `popups/import.rs` | `ImportPopup { step }` |
| `BulkAddInput` | `popups/bulk_add.rs` | `BulkAddPopup` |
| `SearchColumnPicker` | `popups/search_columns.rs` | `SearchColumnsPopup` |
| `RemoteAddNameInput`, `RemoteAddUrlInput` | `popups/add_remote.rs` | `AddRemotePopup { step }` |
| `About` | `popups/about.rs` | `AboutPopup` |

> **Note**: All `*Confirm` variants map to a single **generic** `ConfirmPopup` (like gitui's
> `confirm.rs`) parameterised by an `Action` enum — not 15 separate confirm popup files.

---

### `src/tabs/` — Full-Screen Views

Modelled on `gitui/src/tabs/` which has 5 tab files + `mod.rs`.
Each tab is a struct owning its own child components.

**`tabs/mod.rs`:**
```rust
mod home;       pub use home::HomeTab;
mod workspace;  pub use workspace::WorkspaceTab;
mod branches;   pub use branches::BranchesTab;
mod tags;       pub use tags::TagsTab;
mod files;      pub use files::FilesTab;
mod stashes;    pub use stashes::StashesTab;
mod overview;   pub use overview::OverviewTab;
mod logs;       pub use logs::LogsTab;
```

| Target File | Struct | Child Components it Owns | Replaces `Mode` Variants |
|---|---|---|---|
| `tabs/home.rs` | `HomeTab` | repo card list, status bar | `Normal`, `RepoSearchInput`, `Editing`, `ConfirmDelete` |
| `tabs/workspace.rs` | `WorkspaceTab` | `CommitListComponent`, `DiffComponent`, `StatusListComponent` | `Detail` (commits/staging area) |
| `tabs/branches.rs` | `BranchesTab` | `BranchListComponent`, `DiffComponent` | `Detail` (branches section) |
| `tabs/tags.rs` | `TagsTab` | `TagListComponent` | `Detail` (tags section) |
| `tabs/files.rs` | `FilesTab` | `FileTreeComponent`, syntax preview | `Detail` (files section) |
| `tabs/stashes.rs` | `StashesTab` | `StashListComponent`, `DiffComponent` | `Detail` (stashes section) |
| `tabs/overview.rs` | `OverviewTab` | stats graph, `CommitterStatsComponent` | `Detail` (overview section) |
| `tabs/logs.rs` | `LogsTab` | `CommitListComponent` (full-screen) | `Logs`, `LogsSearchInput` |

---

### `src/queue.rs` — Inter-Component Event Bus (New File)

This is the most important **new** file. Modelled directly on `gitui/src/queue.rs`.

```rust
// src/queue.rs
pub enum Action {
    DeleteRepo,
    DeleteBranch(String),
    DeleteTag(String, bool),
    DeleteStash,
    ApplyStash,
    Commit,
    Push(String, bool),
    Checkout(String),
    Merge(String),
    Rebase(String),
    Discard(String, bool),
    CherryPick(String),
    Revert(String),
    // ...
}

pub enum InternalEvent {
    ConfirmAction(Action),
    ConfirmedAction(Action),
    ShowError(String),
    ShowStatus(String),
    Update(NeedsUpdate),
    OpenCommitPopup,
    OpenCreateBranch,
    SwitchTab(Tab),
    // ...
}

#[derive(Clone, Default)]
pub struct Queue {
    data: Rc<RefCell<VecDeque<InternalEvent>>>,
}
```

All components get a `Queue` clone. Instead of `App` methods being called from `input.rs`,
components push to the queue. `App` drains the queue once per frame.

---

## Phase 4: Deconstructing `app.rs` and `input.rs`

### 4a. `App` struct shrinks to an orchestrator

After Phase 3, `App` only holds:
- Active `Tab` enum variant
- All popup structs (one field per popup, each with `is_visible`)
- The `Queue`
- `config`, `config_path`
- Background channels (`detail_tx/rx`, `tab_tx/rx`)
- `current_detail: Option<ItemDetail>`
- Global `error_message: Option<String>`

**Target: < 15 fields** (currently ~85 fields).

### 4b. `input.rs` shrinks to a router

```rust
pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    // 1. Drain Queue
    app.drain_queue();

    // 2. Try active popup (highest priority)
    if app.event_to_visible_popup(&Event::Key(key)).is_consumed() {
        return true;
    }

    // 3. Try active tab
    if app.active_tab.event(&Event::Key(key)).is_consumed() {
        return true;
    }

    // 4. Global shortcuts only
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => return false,
        KeyCode::Tab => app.cycle_tab(),
        _ => {}
    }
    true
}
```

The 2,800 lines of `input.rs` collapse to ~50 lines. All the mode-specific logic moves
into each tab/popup's own `fn event()`.

---

## Phase 5: Build & DX Optimizations

### 5a. Cargo Profile Optimizations
```toml
[profile.release]
lto = true
opt-level = 'z'
codegen-units = 1
strip = "debuginfo"

[profile.dev.package."ratatui"]
opt-level = 3
[profile.dev.package."git2"]
opt-level = 3
```

### 5b. `rust-toolchain.toml`
```toml
[toolchain]
channel = "stable"
```

### 5c. Makefile additions
```makefile
lint:
	cargo clippy -- -D warnings -D clippy::unwrap_used

fmt-check:
	cargo fmt -- --check

ci: fmt-check lint test
```

### 5d. CI — `.github/workflows/ci.yml`
Run `make ci` on every push/PR. Reference `gitui`'s `.github/` for cross-compilation and
`cargo deny` (license/supply-chain).

---

## Final Target File Tree

```
gitwig/src/
├── main.rs              (terminal setup only, ~95 lines — already good)
├── app.rs               (App struct + queue drainer, target ~300 lines)
├── input.rs             (event router only, target ~50 lines)
├── config.rs            (unchanged)
├── debug_log.rs         (unchanged)
├── queue.rs             ← NEW: InternalEvent, Action, Queue
│
├── ui/
│   ├── mod.rs           (re-exports only)
│   ├── style.rs         ← Theme, ThemeState, THEME static, color helpers
│   ├── layout.rs        ← centered_rect(), layout helpers
│   ├── scrollbar.rs     ← scrollbar rendering
│   └── syntax.rs        ← syntax highlight helpers
│
├── keys/
│   ├── mod.rs           (re-exports only)
│   ├── key_config.rs    ← KeyConfig struct
│   ├── key_list.rs      ← KeyList (all named bindings)
│   └── symbols.rs       ← KeySymbols (display strings)
│
├── components/
│   ├── mod.rs           ← Component + DrawableComponent traits, event_pump(), macros
│   ├── commit_list.rs   ← CommitListComponent
│   ├── diff.rs          ← DiffComponent
│   ├── file_tree.rs     ← FileTreeComponent
│   ├── status_list.rs   ← StatusListComponent (staged/unstaged)
│   ├── branch_list.rs   ← BranchListComponent
│   ├── tag_list.rs      ← TagListComponent
│   ├── stash_list.rs    ← StashListComponent
│   ├── text_input.rs    ← TextInputComponent
│   ├── graph_view.rs    ← GraphViewComponent
│   ├── cmdbar.rs        ← CmdBar (status/shortcut bar)
│   └── utils/
│       ├── mod.rs
│       ├── scroll_vertical.rs
│       └── scroll_horizontal.rs
│
├── tabs/
│   ├── mod.rs           (re-exports only)
│   ├── home.rs          ← HomeTab (repo card list)
│   ├── workspace.rs     ← WorkspaceTab (commits + staging)
│   ├── branches.rs      ← BranchesTab
│   ├── tags.rs          ← TagsTab
│   ├── files.rs         ← FilesTab
│   ├── stashes.rs       ← StashesTab
│   ├── overview.rs      ← OverviewTab
│   └── logs.rs          ← LogsTab
│
└── popups/
    ├── mod.rs           (re-exports only)
    ├── confirm.rs       ← ConfirmPopup (generic, replaces all *Confirm Mode variants)
    ├── commit.rs        ← CommitPopup
    ├── help.rs          ← HelpPopup
    ├── detail_help.rs   ← DetailHelpPopup
    ├── create_branch.rs ← CreateBranchPopup
    ├── create_tag.rs    ← CreateTagPopup
    ├── stash_msg.rs     ← StashMsgPopup
    ├── remote_picker.rs ← RemotePickerPopup
    ├── log_search.rs    ← LogSearchPopup
    ├── import.rs        ← ImportPopup
    ├── bulk_add.rs      ← BulkAddPopup
    ├── search_columns.rs← SearchColumnsPopup
    ├── add_remote.rs    ← AddRemotePopup
    └── about.rs         ← AboutPopup
```

---

## Summary of What Was Missing from the Previous Plan

| Gap | Now addressed |
|---|---|
| **No file-by-file decomposition map** | Every `Mode` variant, field, and UI section is mapped to an exact target file |
| **`mod.rs` purpose not defined** | Now explicit: `mod.rs` re-exports only — no types defined in it |
| **`ConfirmPopup` pattern not mentioned** | All 15+ `*Confirm` variants → one generic `ConfirmPopup { action }`, not 15 files |
| **`keys/` folder had no detail** | Full 4-file breakdown: `key_config`, `key_list`, `symbols`, `mod` |
| **`ui/` folder had no detail** | 5-file breakdown covering style, layout, scrollbar, syntax |
| **Tab structs not listed** | All 8 tabs listed with their child components and the `Mode` variants they replace |
| **Component state ownership not mapped** | Each component file lists exactly which `App` fields it absorbs |
| **Target line counts not specified** | `app.rs` → ~300 lines, `input.rs` → ~50 lines |
