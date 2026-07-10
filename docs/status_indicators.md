# 📂 Item status indicators

Each repository card shows icons and badges reflecting its state:

### General Icons
- `📌` (or `[P]`) — Pinned repository.
- `★` (or `*`) — Starred / favorite repository.
- `● git` (or `G  clean`) — Clean Git repository.
- `○ dir` (or `o dir`) — Directory exists but is not a git repository.
- `✕ missing` (or `x missing`) — Path does not exist or is not a directory.

### Compact Status Suffixes
For git repositories, the status indicator shows compact counts for any non-zero values:

| Suffix | Meaning | Colour |
| ------ | ------- | ------ |
| `N+`   | N files staged for commit | Cyan |
| `N!`   | N files modified but not staged | Yellow |
| `N?`   | N untracked files | Muted |
| `N✕`   | N conflicted files | Red / Danger |
| `N↑`   | N commits ahead of upstream (needs push) | Bold Green |
| `N↓`   | N commits behind upstream (needs pull/fetch) | Bold Yellow |

When all counts are zero, the indicator shows `● clean`. Press `?` or `h` at any time to see the legend inside the app.

### ⚠ Staging Divergence (`⚠ PARTIAL`)
When a repository has **both** staged changes and unstaged changes (modified or untracked) coexisting simultaneously, Gitwig will display a yellow `⚠ PARTIAL` warning badge next to the repository name on its card.

### Active Repository State Badges
When a repository has an active Git operation or special state, Gitwig displays a colored status badge:
- `✓ CLEAN` — No active Git state/operation.
- `⚠ MERGE` — Active Merge session (contains conflicts).
- `🚧 REBASE` — Active Interactive/Normal Rebase.
- `⚡ CHERRY` — Active Cherry-pick operation.
- `⚡ REVERT` — Active Revert operation.
- `🔍 BISECT` — Active Bisect session.
- `📬 APPLY` — Applying patches (mailbox).

### Git LFS Badges (`[LFS]`)
Files tracked by Git LFS will display a blue `[LFS]` badge next to their names in:
- Staged / Unstaged / Conflicts file panels (Workspace tab).
- Changed files lists (Commit Details / Inspect window).
- Repository Files Tree list (Files tab).
- Stashed Files list (Stashes tab).

### Global Summary Header Bar
The high-level dashboard stats at the top of the homepage show:
- **repos**: Total number of configured repositories. If stale projects are hidden via configuration (`show_stale_projects`), the dashboard shows `<visible>/<total>` repositories, indicating how many are hidden due to being stale.
- **dirty**: Repositories with uncommitted/unstaged changes.
- **ahead**: Repositories with local commits ahead of their remote tracking branch.
- **stale**: Repositories where the last commit is older than the configured threshold (default is 1 month; configurable via `stale_threshold_months` in settings).

### Auto-Refresh & Manual Refresh
Items support `~` and `~/...` expansion, so `~/code/gitwig` resolves to your home directory. 

Gitwig automatically refreshes all repository statuses in the background every **10 seconds** using non-blocking background threads, ensuring the home dashboard is always live and up-to-date. You can also press **`R`** to manually refresh the selected item's status immediately (e.g. after running a git command externally); the status bar briefly flashes `Refreshed` to confirm.
