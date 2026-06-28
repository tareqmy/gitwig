# Large Repository Performance Plan

> **Status:** Phase A, B, and C Implemented — Phase D Planning  
> **Created:** 2026-06-26  
> **Target repo profiled:** `360yield` (azerion / vivasoft — tens of thousands of commits, hundreds of branches)

---

## 1. Diagnosis — What Is Slow Today

### 1.1 The Loading Path

When the user presses **Enter** on a card, `App::open_detail` (`app.rs:971`) spawns a background thread that calls `repo::inspect_detail → collect_info` (`repo.rs:985`).  
**All of the following happen sequentially inside that single thread before any data is shown:**

| Step | Location | Scales with? |
|---|---|---|
| `Repository::open` | `repo.rs:986` | O(1) |
| `collect_summary` — full status walk + ahead/behind | `repo.rs:987` | **O(files in worktree)** |
| HEAD / upstream lookup | `repo.rs:993–1026` | O(1) |
| Remotes enumeration | `repo.rs:1028–1047` | O(remotes × refspecs) |
| `build_ref_map` — iterate **ALL** refs | `repo.rs:1049, 1183–1216` | **O(branches + tags)** |
| `collect_commits` — revwalk + **diff per commit** | `repo.rs:1049, 847–901` | **O(max\_commits × diff\_size)** |
| `collect_graph_lines` — shell `git log --graph --all` **unbounded** | `repo.rs:1053, 1543–1605` | **O(all commits × all branches)** |
| `populate_file_changes` — **second** full status walk | `repo.rs:1055, 1226–1305` | **O(files in worktree)** |
| `collect_committer_stats` — walks 10 000 commits, **not gated** | `repo.rs:1057, 904–938` | **O(10 000)** |
| Local branches — `peel_to_commit` per branch | `repo.rs:1062–1086` | **O(local\_branches)** |
| Remote branches — `peel_to_commit` per branch | `repo.rs:1088–1114` | **O(remote\_branches)** |
| Local tags — `find_reference + peel_to_commit` per tag | `repo.rs:1116–1141` | **O(tags)** |
| Index walk for Files tab | `repo.rs:1145–1153` | **O(tracked\_files)** |
| Stash loop — `commit_changed_files` per stash | `repo.rs:1155–1174` | O(stashes × stash\_size) |

### 1.2 Critical Bottlenecks (ranked by impact)

#### 🔴 #1 — `collect_graph_lines` is completely unbounded (`repo.rs:1547`)

```rust
std::process::Command::new("git")
    .args(["log", "--graph", "--all", "--date=relative", ...])
    .output();   // ← no --max-count, waits for ALL output synchronously
```

Runs `git log --graph --all` with **no `--max-count`** flag. On a repo with 50 000+ commits and 200 branches this produces millions of characters, takes 10–30+ seconds, and buffers the entire output into RAM before returning.

#### 🔴 #2 — `max_commits` defaults to **0 = unlimited** (`config.rs:30`)

```rust
fn default_max_commits() -> usize { 0 }   // 0 = walk.collect() = ALL commits
```

In `collect_commits` (`repo.rs:859–863`), `limit == 0` triggers `walk.collect()` — every commit in history. For each commit, `commit_changed_files` calls `repo.diff_tree_to_tree(...)`. On 50k commits that is **50 000 diff computations** in a single blocking call.

#### 🔴 #3 — Two separate full `repo.statuses()` walks (`repo.rs:1332`, `repo.rs:1232`)

`collect_summary` → `populate_worktree` scans the whole worktree once.  
`populate_file_changes` scans it **again** with `recurse_untracked_dirs(true)`.  
In a monorepo with 100 000 files this doubles disk I/O.

#### 🟠 #4 — `collect_committer_stats` walks 10 000 commits unconditionally (`repo.rs:1057`)

Hard-coded limit of 10 000, not gated by `max_commits`. Runs even when the user never visits the Overview tab.

#### 🟠 #5 — All synchronous refreshes block the **main UI thread** (`app.rs:1029`, `5395`, etc.)

`resync_detail` (`app.rs:1029`) calls `inspect_repo_detail` → full `collect_info` pipeline on the event loop thread. Also called at lines 1722, 1783, 1812, 1860, 1885, 1924, 2130, 2189, 3459, 5222, 5270, 5310, **5395** (after a successful fetch — called directly inside the run loop). All of these freeze the UI.

#### 🟠 #6 — Branch / tag peeling is O(branches + tags) × libgit2 round-trips

Each local branch (`repo.rs:1069`), remote branch (`repo.rs:1096`), and local tag (`repo.rs:1123`) calls `peel_to_commit()` individually. With 500 branches + 2 000 tags: 2 500 sequential object lookups.

#### 🟡 #7 — `App::new` runs `inspect_summary` for every configured repo synchronously (`app.rs:424–428`)

```rust
let statuses = config.items.iter()
    .map(|s| repo::inspect_summary(s))   // blocks UI before first frame
    .collect();
```

With 20 repos, this runs 20 sequential libgit2 status walks on the main thread before anything is displayed.

#### 🟡 #8 — Index walk for Files tab is always eager (`repo.rs:1146`)

The full index (`repo.index().iter()`) is always loaded, even when the user opens the Workspace tab and never visits Files.

---

## 2. The Fix — Phased Plan

### Phase A — Quick Wins (hours, no architecture change)

#### A1. Cap `max_commits` default to 500

**File:** `src/config.rs:30`  
**Change:**
```rust
fn default_max_commits() -> usize { 500 }
```
**Impact:** Eliminates unbounded commit walk. Users who want full history can set `max_commits = 0` in `~/.gitwig/config.toml`.

#### A2. Add `--max-count` to `collect_graph_lines`

**File:** `src/repo.rs:1547–1556`  
**Change:** Add `&format!("--max-count={}", graph_max_commits)` to the git CLI args. Expose `graph_max_commits: usize` in `Config` with default `1000`.
```rust
.args([
    "log", "--graph", "--all",
    "--date=relative",
    &format!("--max-count={}", graph_max_commits),
    &format!("--pretty=format:{}", format_str),
    "--color=never",
])
```
**Impact:** Graph generation becomes O(1 000) instead of O(all commits). Eliminates the single biggest bottleneck. The Graph tab will note "showing last 1 000 commits" when the limit is hit.

#### A3. Merge the two status walks into one

**Files:** `repo.rs:1226–1305` and `repo.rs:1331–1364`  
**Change:**  
- Run `repo.statuses(...)` **once** in `collect_info` with union of all required flags.
- Pass the resulting `Statuses` object into a combined function that fills both the `RepoSummary` counters **and** the `FileEntry` lists in a single loop.
- Delete the separate `populate_worktree` and `populate_file_changes` calls.

**Impact:** Halves status scan I/O; especially significant on large worktrees.

#### A4. Gate `collect_committer_stats` behind `max_commits`

**File:** `repo.rs:1057`  
**Change:**
```rust
let stats_limit = if commit_limit > 0 { commit_limit.min(10_000) } else { 10_000 };
if let Ok((stats, limit_reached)) = collect_committer_stats(&repo, stats_limit) { ... }
```
**Impact:** Committer stats walk is bounded by the same limit the user set for commits.

#### A5. Make `resync_detail` and all post-action refreshes async

**File:** `src/app.rs:1029`, and all other sites listed under Bottleneck #5  
**Change:** Replace each synchronous `self.current_detail = Some(self.inspect_repo_detail(item))` with the same spawn+channel pattern used by `open_detail`:
```rust
let tx = self.detail_tx.clone();
let item = item.to_string();
let max_commits = self.config.max_commits;
std::thread::spawn(move || {
    let _ = tx.send((item.clone(), repo::inspect_detail(&item, max_commits)));
});
```
Show `loading_repo_path` spinner during the refresh.  
**Impact:** The UI is never frozen after commit/stage/push/fetch operations.

---

### Phase B — Tab-Lazy Loading (3–5 days, architectural)

The biggest architectural win is to stop computing data for every tab when only Tab 0 (Workspace/Commits) is visible on open.

#### B1. Split `RepoInfo` into `CoreInfo` + lazy `TabPayload`

Introduce a `TabData<T>` wrapper:
```rust
pub enum TabData<T> {
    NotLoaded,
    Loading,
    Loaded(T),
    Error(String),
}
```

Refactor `RepoInfo` so that only the fields needed by Tab 0 are part of `CoreInfo` (loaded immediately on Enter):
- HEAD info, branch, upstream, summary, staged/unstaged/conflicted lists, first N commits.

All other fields become lazy `TabData`:

| Tab | Data type | Loaded when |
|---|---|---|
| **0 — Workspace** | `CoreInfo` (commits, staged, unstaged) | Immediately on Enter |
| **2 — Graph** | `GraphData` (graph lines) | First visit to Graph tab |
| **3 — Branches** | `BranchData` (local + remote branches) | First visit to Branches tab |
| **4 — Tags** | `TagData` (local + remote tags) | First visit to Tags tab |
| **5 — Remotes** | `RemoteData` (remotes list) | First visit to Remotes tab |
| **6 — Stashes** | `StashData` (stash list) | First visit to Stashes tab |
| **7 — Overview** | `OverviewData` (committer stats, files) | First visit to Overview tab |
| **1 — Files** | `FilesData` (index file list) | First visit to Files tab |

Add a second channel pair `(tab_tx, tab_rx)` alongside `(detail_tx, detail_rx)` in `App`.

In `handle_key` → Tab switch logic: if the new tab's `TabData` is `NotLoaded`, set it to `Loading` and spawn a worker that calls the appropriate `repo::load_tab_N(path, config)` function and sends the result via `tab_tx`.

In the `run` loop, drain `tab_rx` and merge results into the current `RepoInfo`.

In `ui_detail.rs`, each tab's rendering code checks the `TabData` state:
- `NotLoaded` / `Loading` → draw a centered "⟳  Loading…" paragraph.
- `Loaded(data)` → draw normal content.
- `Error(msg)` → draw an error paragraph.

**Benefits:**
- Opening a large repo becomes near-instant (only Workspace/Commits loads).
- Graph tab — the most expensive — is only computed when explicitly visited.
- Branches with 500 entries, 2000 tags: never loaded unless the user navigates there.

#### B2. Paginate the Commits panel

**File:** `src/repo.rs:847–901`

- On initial load, fetch only the first `page_size × initial_pages` commits (e.g. 200).
- Store a `commit_walk_cursor: Option<git2::Oid>` in `App` to resume the walk.
- Add a keybinding (e.g. pressing `End` or a new `G` / `Load more`) that fetches the next page asynchronously via the same worker thread.
- Show "─── Showing first 200 commits — press G to load more ───" at the bottom of the list.

**Impact:** Commits panel appears instantly regardless of history depth.

#### B3. Stream `collect_graph_lines` progressively

- Start the `git log --graph ...` child process in a worker thread.
- Read stdout line-by-line, parsing and accumulating `Vec<GraphLine>` as they arrive.
- Periodically send partial batches via `tab_tx` (e.g. every 200 lines).
- The UI renders whatever lines have arrived per frame.
- The user sees the graph growing as it loads rather than waiting for all of it.

---

### Phase C — Caching (1–2 days)

#### C1. In-memory `RepoInfo` cache with TTL

Store a cache entry alongside `current_detail`:
```rust
struct DetailCache {
    path: String,
    detail: ItemDetail,
    loaded_at: std::time::Instant,
}
```
When the user presses Enter on a repo that is already cached and `loaded_at` is within `detail_cache_ttl_secs` (configurable, default 30 s):
- Show the cached data immediately.
- Trigger a silent background refresh; replace the cache when it completes (stale-while-revalidate).

**Impact:** Returning to a large repo after visiting another one is instant.

#### C2. Lazy `build_ref_map`

- Only build `build_ref_map` when `collect_commits` will actually annotate commits with ref labels.
- If `max_commits == 0` (unlimited): build the map lazily inside `collect_commits` once, not before it.
- Consider caching the ref map separately with a TTL so it is not rebuilt on every resync.

#### C3. Per-tab TTL — stale-while-revalidate

Each `TabData::Loaded(data)` stores a `loaded_at: Instant`. On tab re-entry after `tab_ttl_secs` (configurable, default 60 s), trigger a silent background refresh without clearing the current display.

---

### Phase D — OS-level watching (future)

#### D1. FSEvents / inotify on `.git`

Use OS file-system notifications to detect when `.git/` changes (commits, index updates, ref changes). On change, re-run `collect_summary` in the background and update the home-page card indicator. Removes the need for polling `inspect_summary` repeatedly.

#### D2. Persistent OID → commit metadata cache

For frequently visited repos, persist an on-disk mapping of `OID → (summary, author, date)` using a lightweight embedded store (`sled` or a flat file). `collect_commits` becomes a map lookup per OID rather than a libgit2 object parse.

---

## 3. Implementation Order

| # | Item | Effort | Unblocks |
|---|---|---|---|
| **1** | A1 — Default `max_commits = 500` | 5 min | Immediate: commit walk bounded |
| **2** | A2 — Cap graph log `--max-count=1000` | 30 min | Immediate: biggest bottleneck gone |
| **3** | A3 — Single status walk | 2 h | Halves disk I/O per open |
| **4** | A4 — Gate committer stats by `max_commits` | 30 min | Quick win for overview tab |
| **5** | A5 — Async `resync_detail` + all post-action refreshes | 3 h | UI never freezes |
| **6** | B1 — Tab-lazy loading (Core + `TabData` channels) | 3–4 days | Near-instant open |
| **7** | B2 — Commit pagination | 1 day | Unbounded history handled |
| **8** | C1 — In-memory `RepoInfo` cache | 1 day | Re-visits instant |
| **9** | B3 — Streaming graph | 2 days | Progressive graph display |
| **10** | C2 — Conditional `build_ref_map` | 2 h | Minor speedup for ref-heavy repos |
| **11** | C3 — Per-tab TTL refresh | 1 day | Smart background refresh |
| **12** | D1 — FSEvents / inotify | 3+ days | Reactive card updates |
| **13** | D2 — Persistent OID cache | 3+ days | Very large repo browsing speed |

---

## 4. Loading UX While Data Arrives

For Phase A (quick wins), the existing `loading_repo_path` spinner suffices.

For Phase B (lazy tabs), add per-tab loading indicators in `ui_detail.rs`:
- When `TabData` is `Loading`, draw a centered `  ⟳  Loading branches…` paragraph with `muted_style()`.
- Status bar shows `[ Loading Graph tab… ]` in muted style while that tab loads.
- Users can freely interact with already-loaded tabs — the UI is never blocked.

---

## 5. New Config Fields

Add to `Config` in `src/config.rs`:

```toml
# Maximum commits loaded in the Commits panel (0 = unlimited; **default changed to 500**)
max_commits = 500

# Maximum commits visualised in the Graph tab (0 = unlimited; default 1000)
graph_max_commits = 1000

# Seconds a detail view stays cached before a silent background refresh (0 = always refresh)
detail_cache_ttl_secs = 30

# Load each tab's data only when the tab is first visited (true = fast open)
lazy_tab_loading = true
```

---

## 6. Testing Strategy

- **Benchmark:** Add a timed integration test that opens `360yield` and measures seconds-to-first-frame.
- **Unit tests:** For each `TabData` state machine transition; for paginated commit walk; for single-pass status walk.
- **Regression:** All 50 existing tests must continue to pass after every phase.
- **Manual smoke test:** Open `360yield`, confirm the detail view appears in < 1 second after Phase A; individual tabs load independently after Phase B.

---

## 7. Files Affected

| File | Changes |
|---|---|
| `src/config.rs` | New config fields; update defaults for `max_commits`, add `graph_max_commits`, `detail_cache_ttl_secs`, `lazy_tab_loading` |
| `src/repo.rs` | Cap graph `--max-count`; merge status walks; add per-tab load functions; commit pagination cursor; gate committer stats |
| `src/app.rs` | Async `resync_detail` + all post-action refresh sites; `TabData` state; `detail_cache`; `tab_tx`/`tab_rx` channels; lazy-load dispatch on tab switch |
| `src/ui.rs` | Home-page spinner: async `inspect_summary` for card indicators |
| `src/ui_detail.rs` | Render `TabData::NotLoaded`/`Loading`/`Error` states per tab |
| `src/input.rs` | "Load more commits" keybind; tab switch triggers lazy load |
| `.agent/CODEMAP.md` | Update data-flow diagram to show `CoreInfo` + lazy `TabData` |
| `.agent/INSTRUCTIONS.md` | Document `TabData<T>`, lazy loading pattern, async refresh rule |
| `.agent/ROADMAP.md` | Check off sub-items as phases ship |
| `GEMINI.md` | Update architectural patterns section |
