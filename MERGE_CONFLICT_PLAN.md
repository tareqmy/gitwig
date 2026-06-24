# Merge Conflict Resolution — Implementation Plan

> **Scope**: Detect, surface, and resolve merge conflicts entirely within the Twig TUI
> without leaving the terminal.  
> **Target files**: `src/repo.rs`, `src/app.rs`, `src/input.rs`, `src/ui_detail.rs`,
> `src/ui.rs`, `.agent/ROADMAP.md`, `.agent/CODEMAP.md`, `README.md`.

---

## Current State

### What already exists

| Layer | What's there |
|---|---|
| `repo.rs` | `WorktreeChanges.conflicted: Vec<FileEntry>` is populated by `populate_file_changes()` when `flags.is_conflicted()` is true. Conflicted files receive `label: "C"` and skip the staged/unstaged buckets. `RepoSummary.conflicted: usize` drives the card badge. |
| `app.rs` | `confirm_branch_merge()` detects "CONFLICT" in stderr and shows a generic status message. Same for rebase. `is_dirty` checks include `!changes.conflicted.is_empty()`. No further handling. |
| `ui_detail.rs` | `changes.conflicted` is used only in boolean `is_dirty` guards. **The conflicted file list is never rendered to the user.** |
| `input.rs` | No key bindings specific to conflict resolution. |

### What's missing

- A **Conflicts sub-panel** in the Workspace tab staging area showing conflicted files.
- A **conflict-marker diff viewer** (with `<<<<<<<`/`=======`/`>>>>>>>` coloring).
- In-TUI actions: **accept ours**, **accept theirs**, **mark resolved**, **abort merge**, **continue merge**.
- New `Mode` variants: `MergeAbortConfirm`, `MergeContinueConfirm`.
- A new `DetailSection::Conflicts` (and `ConflictDiff`) focus variant.
- `repo.rs` functions: `is_merging`, `get_conflict_markers_diff`, `resolve_ours`,
  `resolve_theirs`, `abort_merge`, `continue_merge`.

---

## Implementation Phases

---

### Phase 1 — Detect & Surface (No Resolution Yet)

**Goal**: Make the user aware that conflicts exist without burying them behind a generic
error message. Zero risk to existing staging workflows.

#### 1.1 `repo.rs` — New helper functions

```rust
/// Returns `true` when `.git/MERGE_HEAD` exists — i.e. a merge is in progress.
/// Cheap file-existence check, no libgit2 required.
pub fn is_merging(repo_path: &Path) -> bool {
    repo_path.join(".git/MERGE_HEAD").exists()
}

/// Returns the conflict-marker diff for a conflicted file.
/// Shells out to `git diff` which includes <<<<<<< / ======= / >>>>>>> markers.
/// Returns an empty Vec on any error.
pub fn get_conflict_markers_diff(repo_path: &Path, file_path: &str) -> Vec<DiffLine>
```

`get_conflict_markers_diff` parses `git diff` output into `DiffLine` values, adding new
`DiffLineKind` variants so the UI can color them distinctively:

```rust
pub enum DiffLineKind {
    Header,
    Added,
    Removed,
    Context,
    ConflictOurs,      // NEW: lines between <<<<<<< and =======
    ConflictTheirs,    // NEW: lines between ======= and >>>>>>>
    ConflictSeparator, // NEW: <<<<<<< / ======= / >>>>>>> marker lines
}
```

#### 1.2 `app.rs` — Post-merge navigation

In `confirm_branch_merge()`, when "CONFLICT" is detected in stderr:
1. Keep the error-style status message as today.
2. Call `self.refresh_detail()` so `current_detail` is immediately updated.
3. Set `self.detail_focus = DetailSection::Conflicts`.

```rust
// Before (app.rs ~2037):
err_msg = "Merge conflicts detected. Please resolve conflicts."

// After:
err_msg = "Merge conflicts — resolve them in the Conflicts panel (Tab to navigate)."
self.refresh_detail();
self.detail_focus = DetailSection::Conflicts;
```

#### 1.3 `app.rs` — New `DetailSection` variants

```rust
pub enum DetailSection {
    // … existing variants …
    Conflicts,    // NEW: conflicted-files list panel
    ConflictDiff, // NEW: conflict-marker diff viewer panel
}
```

The Tab cycle in `cycle_detail_focus` includes `Conflicts` **only** when
`changes.conflicted` is non-empty, so it never appears in clean repos.

New App state fields:

```rust
/// Selected row in the Conflicts file list.
pub conflict_file_selection: usize,
/// Cached conflict-marker diff for the selected conflicted file.
pub conflict_diff: Vec<DiffLine>,
/// Scroll offset for the ConflictDiff panel.
pub conflict_diff_scroll: usize,
```

#### 1.4 `ui_detail.rs` — Conflicts sub-panel in the Workspace tab

Add a **Conflicts section** inside `draw_staging_panels()`, rendered below the Unstaged
section when `changes.conflicted` is non-empty.

Visual design (existing `draw_file_subpanel` pattern, new header color):

```
╔══════════════════════════════╗
║  ⚡ CONFLICTS (2)             ║  ← bold Red header, count badge
╠══════════════════════════════╣
║  C  src/main.rs              ║  ← highlighted when focused
║  C  src/lib.rs               ║
╚══════════════════════════════╝
```

- The `C` label already exists in `FileEntry.label` for conflicted files.
- Focused entry uses `Modifier::REVERSED` styling (same as staged/unstaged panels).
- When `changes.conflicted` is empty this section collapses to zero height.
- The vertical layout of the left staging panel becomes a 3-way split:
  Staged (top) / Unstaged (middle) / Conflicts (bottom).

#### 1.5 `ui.rs` — Persistent merge indicator in the status bar

When `repo::is_merging(&repo_path)` is true, prepend a **⚡ MERGING** badge to the
status bar (similar to the existing `fetching` spinner). This is visible regardless of
which tab is active.

```
[ ⚡ MERGING ]  Normal  │  main  │  2 conflicts  │  …
```

`is_merging` is checked once per frame inside `draw_status_bar`; it is a cheap
file-existence check so performance is negligible.

---

### Phase 2 — Conflict Diff Viewer

**Goal**: Let the user inspect conflict markers for the selected conflicted file.

#### 2.1 `app.rs` — Load conflict diff

```rust
pub fn load_conflict_diff(&mut self) {
    let Some(ref detail) = self.current_detail else { return };
    let ItemDetail::GitRepo { ref info, .. } = detail else { return };
    let Some(ref resolved) = info.repo_path else { return };

    if let Some(entry) = info.changes.conflicted.get(self.conflict_file_selection) {
        self.conflict_diff =
            repo::get_conflict_markers_diff(resolved, &entry.path);
        self.conflict_diff_scroll = 0;
    }
}
```

Called whenever `conflict_file_selection` changes or focus moves to
`DetailSection::Conflicts`.

#### 2.2 `input.rs` — Navigation in Conflicts panel

When `detail_focus == DetailSection::Conflicts`:

| Key | Action |
|-----|--------|
| `↑` / `k` | `conflict_file_selection.saturating_sub(1)` + `load_conflict_diff()` |
| `↓` / `j` | increment selection (clamped to list length) + `load_conflict_diff()` |
| `Enter` / `→` | `detail_focus = DetailSection::ConflictDiff` |
| `Esc` | return focus to `DetailSection::Unstaged` |

When `detail_focus == DetailSection::ConflictDiff`:

| Key | Action |
|-----|--------|
| `↑` / `k` | scroll up (`conflict_diff_scroll.saturating_sub(1)`) |
| `↓` / `j` | scroll down |
| `PgUp` | scroll up 10 lines |
| `PgDn` | scroll down 10 lines |
| `Esc` / `←` | return to `DetailSection::Conflicts` |

#### 2.3 `ui_detail.rs` — Conflict diff panel rendering

The right panel of `draw_staging_panels` shows `conflict_diff` when
`detail_focus == ConflictDiff` or `last_staging_focus == Conflicts`.

Coloring added to the diff render loop:

| `DiffLineKind` | Style |
|---|---|
| `ConflictSeparator` | Bold + `Color::Yellow` (or Cyan) |
| `ConflictOurs` | `Color::LightRed` bg tint |
| `ConflictTheirs` | `Color::LightBlue` bg tint |
| `Header` | Cyan (unchanged) |
| `Added` / `Removed` | Green / Red (unchanged) |
| `Context` | Default (unchanged) |

Title: `"Conflict Markers  <filename>"`.

---

### Phase 3 — Resolution Actions

**Goal**: Allow the user to resolve conflicts without leaving the TUI.

#### 3.1 `repo.rs` — Resolution functions

All shell out to `git` via `std::process::Command`, matching the existing pattern.

```rust
/// git checkout --ours <file> && git add <file>
pub fn resolve_ours(repo_path: &Path, file_path: &str) -> Result<(), String>

/// git checkout --theirs <file> && git add <file>
pub fn resolve_theirs(repo_path: &Path, file_path: &str) -> Result<(), String>

/// git add <file>  — stage a manually edited file to mark it resolved
pub fn mark_resolved(repo_path: &Path, file_path: &str) -> Result<(), String>
// (semantic alias over the existing `stage_file`)

/// git merge --abort
pub fn abort_merge(repo_path: &Path) -> Result<(), String>

/// git merge --continue  (or git commit if all conflicts resolved)
pub fn continue_merge(repo_path: &Path) -> Result<(), String>
```

#### 3.2 `app.rs` — Action methods

```rust
pub fn resolve_conflict_ours(&mut self)    { /* repo::resolve_ours + refresh */ }
pub fn resolve_conflict_theirs(&mut self)  { /* repo::resolve_theirs + refresh */ }
pub fn mark_conflict_resolved(&mut self)   { /* repo::mark_resolved + refresh */ }
pub fn request_abort_merge(&mut self)      { self.mode = Mode::MergeAbortConfirm; }
pub fn confirm_abort_merge(&mut self)      { /* repo::abort_merge + refresh + mode=Detail */ }
pub fn request_continue_merge(&mut self)   { self.mode = Mode::MergeContinueConfirm; }
pub fn confirm_continue_merge(&mut self)   { /* repo::continue_merge + refresh + mode=Detail */ }
```

Each action calls `self.refresh_detail()` after success, clamps `conflict_file_selection`,
and shows a status message.

#### 3.3 `app.rs` — New `Mode` variants

```rust
/// "Abort the in-progress merge? [y/N]"
MergeAbortConfirm,
/// "All conflicts resolved. Commit the merge? [y/N]"
MergeContinueConfirm,
```

#### 3.4 `input.rs` — Resolution key bindings

In Conflicts panel (`DetailSection::Conflicts`):

| Key | Action |
|-----|--------|
| `o` | `resolve_conflict_ours()` |
| `t` | `resolve_conflict_theirs()` |
| `r` | `mark_conflict_resolved()` |
| `A` | `request_abort_merge()` → `Mode::MergeAbortConfirm` |
| `C` | `request_continue_merge()` → `Mode::MergeContinueConfirm` |

In ConflictDiff panel (`DetailSection::ConflictDiff`):

| Key | Action |
|-----|--------|
| `o` | `resolve_conflict_ours()` |
| `t` | `resolve_conflict_theirs()` |
| `r` | `mark_conflict_resolved()` |

In confirmation modes:

```
Mode::MergeAbortConfirm:
    y/Y → confirm_abort_merge()
    n/N / Esc → mode = Mode::Detail

Mode::MergeContinueConfirm:
    y/Y → confirm_continue_merge()
    n/N / Esc → mode = Mode::Detail
```

#### 3.5 `ui_detail.rs` — Confirmation popups

Add `draw_merge_abort_confirm_popup()` and `draw_merge_continue_confirm_popup()` following
the existing `draw_branch_merge_popup()` pattern (centered popup, rounded border, y/N
prompt).

---

### Phase 4 — Advanced: Three-Way Editor (Optional / Future)

**Goal**: Provide a side-by-side OURS / BASE / THEIRS view for granular resolution.

```
┌────────────┬────────────┬────────────┐
│   OURS     │    BASE    │   THEIRS   │
│  (HEAD)    │ (ancestor) │ (incoming) │
├────────────┼────────────┼────────────┤
│ …          │ …          │ …          │
└────────────┴────────────┴────────────┘
```

Requires:
- New `repo::get_conflict_three_way()` using `git show :1:path`, `:2:path`, `:3:path`.
- A new `ConflictResolution` mode driving a 3-pane layout.
- Line-level "accept from this pane" actions.
- Writing resolved content back to disk via `std::fs::write` then staging.

**Verdict**: Implement Phases 1–3 first. Phase 4 is a separate roadmap item.

---

## File-by-File Change Summary

| File | Changes |
|------|---------|
| `src/repo.rs` | Add `is_merging`, `get_conflict_markers_diff`, `resolve_ours`, `resolve_theirs`, `abort_merge`, `continue_merge`, `mark_resolved`. Extend `DiffLineKind` with `ConflictOurs`, `ConflictTheirs`, `ConflictSeparator`. |
| `src/app.rs` | Add `DetailSection::Conflicts`, `DetailSection::ConflictDiff`. Add `Mode::MergeAbortConfirm`, `Mode::MergeContinueConfirm`. Add fields: `conflict_file_selection`, `conflict_diff`, `conflict_diff_scroll`. Add action methods. Update `confirm_branch_merge` to navigate to Conflicts panel on conflict. |
| `src/input.rs` | Add key handlers for `DetailSection::Conflicts` and `DetailSection::ConflictDiff`. Add handlers for `Mode::MergeAbortConfirm` and `Mode::MergeContinueConfirm`. |
| `src/ui_detail.rs` | Add Conflicts sub-panel in `draw_staging_panels`. Add conflict-marker coloring. Add `draw_merge_abort_confirm_popup`, `draw_merge_continue_confirm_popup`. Update `DETAIL_HELP_LINES`. |
| `src/ui.rs` | Add `⚡ MERGING` badge in status bar when `is_merging()`. Update help menu. |
| `.agent/ROADMAP.md` | Add merge conflict resolution milestone. |
| `.agent/CODEMAP.md` | Document new `DetailSection` variants and `Mode` variants. |
| `README.md` | Document conflict resolution workflow. |

---

## Key Design Decisions

1. **Shell out for resolution actions**: `resolve_ours`, `resolve_theirs`, `abort_merge`,
   `continue_merge` all shell out to `git` — matching the existing `git merge`, `git rebase`,
   `git stash` patterns. This avoids libgit2 complexity for three-way merge index manipulation.

2. **Conflict diff reuses `DiffLine` infrastructure**: Rather than a bespoke data structure,
   conflict marker diffs are parsed into `Vec<DiffLine>` with new `DiffLineKind` variants.
   The existing diff render loop handles them with minimal added branching.

3. **No inline text editing**: The TUI does not become a text editor. Users who need
   fine-grained edits open the file in their `$EDITOR`, then press `r` to mark it resolved.

4. **`is_merging()` is cheap**: It checks for `.git/MERGE_HEAD` existence only — no
   subprocess, no libgit2. Safe to call every frame in `draw_status_bar`.

5. **Single `staging_file_selection` is NOT reused for conflicts**: A separate
   `conflict_file_selection` index keeps conflict navigation isolated from the
   staged/unstaged selection, avoiding accidental cross-panel index corruption.

6. **Tab focus cycle is conflict-aware**: `DetailSection::Conflicts` is only inserted into
   the Tab cycle when `changes.conflicted` is non-empty, so the normal workflow for clean
   repos is unaffected.

---

## Testing Plan

| Test | Location | Type |
|------|----------|------|
| `test_is_merging_true` / `test_is_merging_false` | `repo.rs` | Unit |
| `test_resolve_ours` | `repo.rs` | Integration (temp repo with injected conflict) |
| `test_resolve_theirs` | `repo.rs` | Integration |
| `test_abort_merge` | `repo.rs` | Integration |
| `test_continue_merge` | `repo.rs` | Integration |
| `test_conflict_markers_diff_non_empty` | `repo.rs` | Integration |
| `test_conflict_panel_hidden_when_clean` | `ui_detail.rs` | Unit |
| `test_conflict_file_selection_clamps` | `app.rs` | Unit |

**Test repo setup pattern** (matches existing tests in `repo.rs`):

```rust
// 1. init repo, add+commit a file on main
// 2. create branch "feature", edit same line, commit
// 3. switch to main, edit same line differently, commit
// 4. shell: git merge feature  → conflict
// 5. assert is_merging() == true
// 6. assert changes.conflicted is non-empty
// 7. resolve_ours() → assert is_merging() == false (after continue_merge)
```

---

## Estimated Effort

| Phase | Complexity | Est. LOC | Notes |
|-------|-----------|----------|-------|
| 1 — Detect & Surface | Low–Medium | ~220 | UI wiring; `collect_summary` already has data |
| 2 — Diff Viewer | Medium | ~160 | Reuses diff panel; adds coloring logic |
| 3 — Resolution Actions | Medium | ~280 | Shell-out functions + key bindings + popups |
| 4 — Three-Way Editor | High | ~500+ | New layout; optional |

**Recommended order**: 1 → 2 → 3 in a single PR, then revisit Phase 4.
